#!/usr/bin/env python3
"""Three-lens pre-labelling for the gold corpus (ac-03).

Spec 2026-06-10-human-verified-gold-corpus. Lenses are independent of YDF,
Sense, and mining-factory sources (independence contract):

  value     — predicate pass-rates over the column's sample values
  header    — vocabulary mapping from the column name
  reference — set membership (ISO 3166, IANA TLDs) + format parsing.
              Decisive-lens rule: may corroborate, never decide alone.

Consensus (signed memo): all non-abstaining lenses agree, at least two of
them, and the agreement does not rest on the reference lens alone. Everything
else goes to the author's adjudication queue.

Inputs:  eval/gold/gold_corpus_candidates.tsv (+ _external.tsv)
         output/ydf-validation-gate/v19_gated.parquet  (GitTables sample values)
         eval/datasets/gold_external/*.csv             (external sample values)
         eval/gold/lens_reference/                     (lens-only sets)
Outputs: eval/gold/gold_lens_labels.tsv   (every candidate, lens votes + verdict)
         eval/gold/gold_review_queue.tsv  (the author's queue)

Usage: eval/gittables/.venv/bin/python scripts/gold_lens_prelabel.py
"""
from __future__ import annotations
import csv
import re
from pathlib import Path

import duckdb

PARQUET = "output/ydf-validation-gate/v19_gated.parquet"
CAND = [Path("eval/gold/gold_corpus_candidates.tsv"),
        Path("eval/gold/gold_corpus_candidates_external.tsv")]
OUT_ALL = Path("eval/gold/gold_lens_labels.tsv")
OUT_QUEUE = Path("eval/gold/gold_review_queue.tsv")
REF = Path("eval/gold/lens_reference")
SEP = chr(9474)  # sample-value delimiter in the corpus parquet

L = {k: f"{v}" for k, v in {
    "lat": "geography.coordinate.latitude", "lon": "geography.coordinate.longitude",
    "url": "technology.internet.url", "utc": "datetime.offset.utc",
    "cc": "geography.location.country_code", "city": "geography.location.city",
    "region": "geography.location.region", "cat": "representation.discrete.categorical",
    "id": "representation.identifier.alphanumeric_id", "year": "datetime.component.year",
    "unix": "datetime.epoch.unix_seconds", "isbn": "identity.commerce.isbn",
    "postal": "geography.address.postal_code", "amount": "finance.currency.amount",
    "duri": "technology.internet.data_uri", "tld": "technology.internet.top_level_domain",
    "int": "representation.numeric.integer_number", "dec": "representation.numeric.decimal_number",
    "text": "representation.text.plain_text", "iso": "datetime.date.iso",
}.items()}

NUM_RE = re.compile(r"^-?\d{1,3}(,\d{3})*(\.\d+)?$|^-?\d+(\.\d+)?([eE][+-]?\d+)?$")
ISO_RE = re.compile(r"^\d{4}-\d{2}-\d{2}([ T]\d{2}:\d{2}(:\d{2})?(\.\d+)?(Z|[+-]\d{2}:?\d{2})?)?$")
MDY_RE = re.compile(r"^(\d{1,2})/(\d{1,2})/(\d{2,4})( \d{1,2}:\d{2}(:\d{2})? ?([AP]M)?)?$", re.I)
OFFSET_RE = re.compile(r"^(utc|gmt)?\s?[+-]?\d{1,2}(:[0-5]\d)?$", re.I)
URL_RE = re.compile(r"^(https?://|www\.)", re.I)


def to_num(v: str):
    try:
        return float(v.replace(",", ""))
    except ValueError:
        return None


def frac(vals, pred):
    if not vals:
        return 0.0
    return sum(1 for v in vals if pred(v)) / len(vals)


def value_lens(vals: list[str], sibling_headers: list[str]) -> tuple[str | None, float]:
    """Best-scoring value predicate; abstain below 0.7."""
    vals = [v.strip() for v in vals if v and v.strip()][:200]
    if len(vals) < 4:
        return None, 0.0
    nums = [to_num(v) for v in vals]
    nums = [n for n in nums if n is not None]
    f_num = len(nums) / len(vals)
    # representation types are about WRITTEN format: "0.0" is decimal, not integer
    f_int = frac(vals, lambda v: bool(re.fullmatch(r"-?\d{1,3}(,\d{3})+|-?\d+", v)))
    nonint = frac(vals, lambda v: bool(re.fullmatch(r"-?\d{1,3}(,\d{3})*\.\d+|-?\d+\.\d+", v)))
    distinct = len({v.lower() for v in vals})
    card = distinct / len(vals)
    has_lon_sib = any(re.search(r"(^|[^a-z])(lon|lng|long)", h.lower()) for h in sibling_headers)
    has_lat_sib = any(re.search(r"(^|[^a-z])lat", h.lower()) for h in sibling_headers)

    scores: dict[str, float] = {}
    if f_num >= 0.9 and nums:
        in90 = sum(1 for n in nums if -90 <= n <= 90) / len(nums)
        in180 = sum(1 for n in nums if -180 <= n <= 180) / len(nums)
        if in90 >= 0.95 and nonint >= 0.3:
            scores[L["lat"]] = 0.9 if has_lon_sib else 0.6   # range alone can't beat feature floats
        if in180 >= 0.95 and nonint >= 0.3 and any(abs(n) > 90 for n in nums):
            scores[L["lon"]] = 0.9 if has_lat_sib else 0.65
        if frac(vals, lambda v: bool(re.fullmatch(r"-?(1[5-9]|20)\d{2}", v))) >= 0.95:
            scores[L["year"]] = 0.85
        if frac(vals, lambda v: bool(re.fullmatch(r"\d{9,10}", v)) and 5e8 <= int(v) <= 2.5e9) >= 0.9:
            scores[L["unix"]] = 0.85
        if f_int >= 0.98:
            scores[L["int"]] = 0.7
        elif f_num >= 0.98 and nonint >= 0.2:
            scores[L["dec"]] = 0.7
    if frac(vals, lambda v: bool(URL_RE.match(v))) >= 0.8:
        scores[L["url"]] = 0.95
    if frac(vals, lambda v: v.lower().startswith("data:")) >= 0.8:
        scores[L["duri"]] = 0.97
    if frac(vals, _isbn_ok) >= 0.7:
        scores[L["isbn"]] = 0.97
    if frac(vals, lambda v: bool(OFFSET_RE.fullmatch(v)) and abs((to_num(re.sub(r"[^\d+-]", "", v.split(":")[0])) or 99)) <= 14) >= 0.8:
        # bare small ints are ambiguous with plain integers
        scores[L["utc"]] = 0.9 if frac(vals, lambda v: (":" in v) or v.lower().startswith(("utc", "gmt")) or v.startswith(("+", "-"))) >= 0.5 else 0.65
    if frac(vals, lambda v: bool(re.fullmatch(r"\d{5}(-\d{4})?", v))) >= 0.85:
        scores[L["postal"]] = 0.7   # 5-digit is ambiguous with integers
    if frac(vals, lambda v: bool(re.fullmatch(r"[A-Za-z]{1,2}\d[A-Za-z\d]? ?\d[A-Za-z]{2}", v))) >= 0.8:
        scores[L["postal"]] = 0.92  # UK-style outward+inward
    if frac(vals, lambda v: bool(re.match(r"^[$£€]", v)) and to_num(v[1:]) is not None) >= 0.8:
        scores[L["amount"]] = 0.85
    elif f_num >= 0.9 and frac(vals, lambda v: bool(re.search(r"\.\d{2}$", v))) >= 0.7:
        scores[L["amount"]] = 0.6
    if frac(vals, lambda v: bool(ISO_RE.fullmatch(v))) >= 0.9:
        scores[L["iso"]] = 0.92
    elif frac(vals, lambda v: bool(MDY_RE.fullmatch(v))) >= 0.9:
        days_over_12 = any(int(m.group(1)) > 12 for v in vals if (m := MDY_RE.fullmatch(v)))
        scores["datetime.date.dmy_slash" if days_over_12 else "datetime.date.mdy_slash"] = 0.75
    if f_num < 0.1:
        if frac(vals, lambda v: bool(re.fullmatch(r"[A-Za-z]{2}", v))) >= 0.9:
            scores[L["cc"]] = 0.6   # shape only; reference lens confirms membership
        if frac(vals, lambda v: bool(re.fullmatch(r"\.?[a-z0-9-]{2,24}", v.lower()))) >= 0.9 and card <= 0.6:
            scores[L["tld"]] = 0.55
        if distinct <= 25 and card <= 0.3 and len(vals) >= 10:
            scores[L["cat"]] = 0.75
        if card >= 0.9 and frac(vals, lambda v: bool(re.fullmatch(r"[A-Za-z0-9_-]+", v)) and any(c.isdigit() for c in v)) >= 0.7:
            scores[L["id"]] = 0.8
        if frac(vals, lambda v: bool(re.search(r"[A-Za-z]{3,}", v))) >= 0.9 and card >= 0.5:
            scores.setdefault(L["text"], 0.65)
    # Definitional backbone labels: a column that is purely integers (and
    # matched no more-specific numeric label) IS integer_number — value
    # evidence alone is sufficient for these two labels.
    specific = {L["year"], L["unix"], L["postal"], L["amount"], L["lat"], L["lon"], L["utc"], L["isbn"]}
    if L["int"] in scores and not (specific & set(scores)):
        scores[L["int"]] = 0.95
    if L["dec"] in scores and not (specific & set(scores)):
        scores[L["dec"]] = 0.95
    if not scores:
        return None, 0.0
    label = max(scores, key=lambda k: scores[k])
    return (label, scores[label]) if scores[label] >= 0.5 else (None, scores[label])


def _isbn_ok(v: str) -> bool:
    s = re.sub(r"[- ]", "", v)
    if re.fullmatch(r"\d{9}[\dXx]", s):
        t = sum((10 - i) * (10 if c in "Xx" else int(c)) for i, c in enumerate(s))
        return t % 11 == 0
    if re.fullmatch(r"\d{13}", s):
        t = sum(int(c) * (1 if i % 2 == 0 else 3) for i, c in enumerate(s))
        return t % 10 == 0
    return False


HEADER_MAP: list[tuple[str, str, float]] = [
    (r"(^|[^a-z])lat(itude)?($|[^a-z])", L["lat"], 0.85),
    (r"(^|[^a-z])(lon|lng|long)(itude)?($|[^a-z])", L["lon"], 0.85),
    (r"(^|[^a-z])(url|uri|link|href|website|homepage|permalink)($|[^a-z])", L["url"], 0.85),
    (r"(utc|gmt|hours?)[ _-]?offset|offset[ _-]?(utc|gmt)", L["utc"], 0.9),
    (r"(^|[^a-z])(utc|gmt|tz|timezone|time_zone)($|[^a-z])", L["utc"], 0.6),
    (r"country", L["cc"], 0.6),
    (r"(^|[^a-z])(city|town|municipality)($|[^a-z])", L["city"], 0.8),
    (r"(region|province|county|prefecture|borough|district)", L["region"], 0.7),
    (r"(^|[^a-z])state($|[^a-z])", L["region"], 0.6),
    (r"(status|category|gender|segment)|(^|[^a-z])(type|class|level|grade|tier|group|kind)($|[^a-z])", L["cat"], 0.7),
    (r"isbn", L["isbn"], 0.9),
    (r"(zip|postal|postcode|post_code)", L["postal"], 0.85),
    (r"(price|salary|amount|revenue|gross|paid|cost|fee)", L["amount"], 0.75),
    (r"(^|[^a-z])year($|[^a-z])|(^|[^a-z])yr($|[^a-z])", L["year"], 0.8),
    (r"(timestamp|epoch|unix)", L["unix"], 0.7),
    (r"data_?uri|base64", L["duri"], 0.85),
    (r"(^|[^a-z])tld($|[^a-z])|top_?level", L["tld"], 0.85),
    (r"(^id$|_id$|case_number|certificate|serial|(^|[^a-z])(code|key|ref|sku)($|[^a-z]))", L["id"], 0.7),
    (r"(date|_at$|_on$)", L["iso"], 0.5),   # format unknown from header alone
    (r"(title|name|description|street|address)", L["text"], 0.6),
]


def header_lens(header: str) -> tuple[str | None, float]:
    h = header.lower()
    for rx, label, conf in HEADER_MAP:
        if re.search(rx, h):
            return (label, conf) if conf >= 0.5 else (None, conf)
    return None, 0.0


def load_reference():
    tlds = set()
    for line in (REF / "iana_tlds_alpha.txt").read_text().splitlines():
        if line and not line.startswith("#"):
            tlds.add(line.strip().lower())
    a2, a3, names = set(), set(), set()
    with (REF / "iso3166.csv").open() as fh:
        for row in csv.DictReader(fh):
            a2.add(row["alpha-2"].upper())
            a3.add(row["alpha-3"].upper())
            names.add(row["name"].lower())
    # GeoNames sets — LENS-ONLY corroboration (decisive-lens rule): city names
    # (incl. ascii + alternate spellings) and admin-1 region names.
    cities, regions = set(), set()
    for line in (REF / "cities15000.txt").read_text().splitlines():
        f = line.split("\t")
        if len(f) > 3:
            cities.add(f[1].lower())
            cities.add(f[2].lower())
            for alt in f[3].split(","):
                if alt:
                    cities.add(alt.lower())
    for line in (REF / "admin1CodesASCII.txt").read_text().splitlines():
        f = line.split("\t")
        if len(f) > 2:
            regions.add(f[1].lower())
            regions.add(f[2].lower())
    return tlds, a2, a3, names, cities, regions


def reference_lens(vals: list[str], ref) -> tuple[str | None, float]:
    tlds, a2, a3, names, cities, regions = ref
    vals = [v.strip() for v in vals if v and v.strip()][:200]
    if len(vals) < 4:
        return None, 0.0
    up = [v.upper() for v in vals]
    if frac(up, lambda v: v in a2) >= 0.9 or frac(up, lambda v: v in a3) >= 0.9:
        return L["cc"], 0.95
    if frac(vals, lambda v: v.lower().lstrip(".") in tlds) >= 0.9:
        return L["tld"], 0.95
    if frac(vals, lambda v: v.lower() in names) >= 0.8:
        return "geography.location.country", 0.9   # country NAMES, not codes
    low = [v.lower() for v in vals]
    f_city = frac(low, lambda v: v in cities)
    f_region = frac(low, lambda v: v in regions)
    if f_city >= 0.7 and f_city > f_region:
        return L["city"], 0.85
    if f_region >= 0.7:
        return L["region"], 0.85
    if frac(vals, lambda v: bool(ISO_RE.fullmatch(v))) >= 0.9:
        return L["iso"], 0.85
    return None, 0.0


def main() -> int:
    con = duckdb.connect()
    ref = load_reference()
    # GitTables sample values + per-file sibling headers, one pass each.
    rows = list(csv.DictReader(CAND[0].open(), delimiter="\t"))
    rows_ext = list(csv.DictReader(CAND[1].open(), delimiter="\t"))
    shas = sorted({r["file_content_sha256"] for r in rows})
    con.execute("CREATE TEMP TABLE want(sha VARCHAR)")
    con.executemany("INSERT INTO want VALUES (?)", [(s,) for s in shas])
    got = con.execute(f"""
        SELECT file_content_sha256, column_name, sample_values_truncated
        FROM '{PARQUET}' p JOIN want w ON p.file_content_sha256 = w.sha
    """).fetchall()
    values: dict[tuple[str, str], list[str]] = {}
    siblings: dict[str, list[str]] = {}
    for sha, col, sv in got:
        siblings.setdefault(sha, []).append(col)
        values[(sha, col)] = (sv or "").split(SEP)
    # External sample values from the vendored CSVs.
    for path in {r["file_path"] for r in rows_ext}:
        with open(path) as fh:
            rd = csv.DictReader(fh)
            cols = {c: [] for c in rd.fieldnames}
            for i, row in enumerate(rd):
                if i >= 2000:
                    break
                for c in rd.fieldnames:
                    if row[c] and len(cols[c]) < 200:
                        cols[c].append(row[c])
        sha = next(r["file_content_sha256"] for r in rows_ext if r["file_path"] == path)
        siblings[sha] = list(cols)
        for c, vs in cols.items():
            values[(sha, c)] = vs

    out, queue = [], []
    for r in rows + rows_ext:
        key = (r["file_content_sha256"], r["column_name"])
        vals = values.get(key, [])
        sibs = [h for h in siblings.get(key[0], []) if h != key[1]]
        v_lab, v_conf = value_lens(vals, sibs)
        h_lab, h_conf = header_lens(r["column_name"])
        f_lab, f_conf = reference_lens(vals, ref)
        votes = [x for x in ((v_lab, "value", v_conf), (h_lab, "header", h_conf),
                             (f_lab, "reference", f_conf)) if x[0]]
        labels = {x[0] for x in votes}
        lenses = {x[1] for x in votes}
        combined = 1.0
        for _, _, c in votes:
            combined *= (1 - c)
        combined = 1 - combined
        if not votes:
            verdict, why = "queue", "all_abstain"
        elif len(labels) > 1:
            verdict, why = "queue", "disagreement"
        elif lenses == {"reference"}:
            verdict, why = "queue", "reference_decisive"
        elif len(votes) == 1:
            # value-alone consensus only for the definitional backbone labels
            lab, lens, c = votes[0]
            if lens == "value" and c >= 0.95 and lab in (L["int"], L["dec"]):
                verdict, why = "consensus", "value_definitional"
            else:
                verdict, why = "queue", "single_lens"
        elif combined >= 0.85:
            verdict, why = "consensus", "+".join(sorted(lenses))
        else:
            verdict, why = "queue", "low_combined_confidence"''
        rec = dict(r)
        rec.update(value_label=v_lab or "", value_conf=f"{v_conf:.2f}",
                   header_label=h_lab or "", header_conf=f"{h_conf:.2f}",
                   reference_label=f_lab or "", reference_conf=f"{f_conf:.2f}",
                   verdict=verdict, verdict_basis=why,
                   consensus_label=(labels.pop() if verdict == "consensus" else ""))
        out.append(rec)
        if verdict == "queue":
            queue.append(rec)

    fields = list(out[0].keys())
    for path, data in ((OUT_ALL, out), (OUT_QUEUE, queue)):
        with path.open("w", newline="") as fh:
            w = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
            w.writeheader()
            w.writerows(data)
    n = len(out)
    nc = sum(1 for r in out if r["verdict"] == "consensus")
    print(f"candidates {n}: consensus {nc} ({nc/n:.0%}), queue {len(queue)} ({len(queue)/n:.0%})")
    by = {}
    for r in queue:
        by[r["verdict_basis"]] = by.get(r["verdict_basis"], 0) + 1
    for k, v in sorted(by.items(), key=lambda kv: -kv[1]):
        print(f"  queue/{k}: {v}")
    by = {}
    for r in queue:
        by[r["stratum_label"]] = by.get(r["stratum_label"], 0) + 1
    print("  queue by stratum (top 8):")
    for k, v in sorted(by.items(), key=lambda kv: -kv[1])[:8]:
        print(f"    {v:4d}  {k}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
