#!/usr/bin/env python3
"""Draw the GitTables share of the gold-corpus candidate sample.

Spec 2026-06-10-human-verified-gold-corpus ac-02. Quotas per the signed sizing
memo (output/gold-corpus/sizing_memo.md): 9 battleground types x90, 7 model-gap
types x50 (year 50), 4-type common backbone x60 — 1,400 total, net of the 240
gold-anchor seed rows (eval/gold/gold_eval_anchor.tsv), which fold in per type.

Selection is from the stable v19 corpus parquet
(output/ydf-validation-gate/v19_gated.parquet, 6.59M columns) in two buckets:

  A header_pos  — header matches the type's positive regex (and clears the
                  negative one where defined), ANY model prediction. Supplies
                  recall measurement once labelled.
  B model_emit  — v19 emits the type on a header that does NOT match the
                  positive regex. Supplies precision measurement beyond the
                  easy headers (the FP-hunting bucket).

Backbone types draw a single random bucket of v19 emissions.

Determinism: ORDER BY md5(file_content_sha256 || column_name) — same parquet,
same output. No randomness, no Date.now. Sanitisation: the TSV carries NO raw
sample values (anchor convention) — values are pulled from the parquet on
demand by the lens/review tooling.

Labels (curated_label/labeller/provenance) stay EMPTY here: ac-03 lenses and
the ac-04 human pass fill them. The independence contract is not violated by
using v19 predictions for *selection* — selection bias is acknowledged in the
fixture's bucket column; labels never derive from model output.

Shortfalls (bucket A cannot fill, e.g. utc positives ~absent in GitTables) are
written to eval/gold/gold_corpus_external_needed.tsv — the external-sourcing
mandate for the ~30% external share.

Usage: eval/gittables/.venv/bin/python scripts/build_gold_corpus_candidates.py
"""
from __future__ import annotations
import csv
import math
from pathlib import Path

import duckdb

PARQUET = "output/ydf-validation-gate/v19_gated.parquet"
ANCHOR = Path("eval/gold/gold_eval_anchor.tsv")
OUT = Path("eval/gold/gold_corpus_candidates.tsv")
EXT = Path("eval/gold/gold_corpus_external_needed.tsv")

# Regexes shared with scripts/build_rare_type_gold.py where they exist.
LAT_POS = r'(x|y|geo|dec|decimal|wgs84)?[ _-]?lat(itude)?[ _-]?(dd|deg|degrees|decimal|n|s|wgs84|coord|gms|new|x|y|[0-9])?'
LON_POS = r'(x|y|geo|dec|decimal|wgs84)?[ _-]?(lon|lng|long)(itude)?[ _-]?(dd|deg|degrees|decimal|e|w|wgs84|coord|gms|new|x|y|[0-9])?'
COORD_NEG = (r'(population|score|rating|\brate\b|ratio|error|\berr\b|rms|residual|magnitude|\bmag\b|'
             r'depth|temperature|\btemp\b|price|cost|amount|weight|height|\bage\b|percent|\bpct\b|'
             r'probability|\bprob\b|correlation|coefficient|\bcoef\b|\bmean\b|median|stdev|\bstd\b|'
             r'variance|\bvar\b|salary|income|revenue|\bgdp\b|density|frequency|voltage|velocity|'
             r'pressure|balance|elevation|distance|speed)')
URL_POS = r'((^|[^a-z])(url|uri|link)($|[^a-z])|href|website|web_?site|homepage|web_?page|permalink)'

# label -> (tier, gross_quota, pos_regex or None, neg_regex or None)
# pos=None means backbone: single random model-emission bucket.
STRATA: dict[str, tuple[int, int, str | None, str | None]] = {
    "geography.coordinate.latitude":            (1, 90, LAT_POS, COORD_NEG),
    "geography.coordinate.longitude":           (1, 90, LON_POS, COORD_NEG),
    "technology.internet.url":                  (1, 90, URL_POS, None),
    "datetime.offset.utc":                      (1, 90, r'((^|[^a-z])utc($|[^a-z])|(^|[^a-z])gmt($|[^a-z])|time_?zone|(^|[^a-z])tz($|[^a-z])|(utc|gmt|time|tz|zone|hours?)[ _-]?offset|offset[ _-]?(utc|gmt|hours?|minutes?))', None),
    "geography.location.country_code":          (1, 90, r'(country|nation|nationality)', None),
    "geography.location.city":                  (1, 90, r'((^|[^a-z])(city|town|cities)($|[^a-z])|municipality)', None),
    "geography.location.region":                (1, 90, r'(region|province|county|prefecture|\bstate\b|territory|district)', None),
    "representation.discrete.categorical":      (1, 90, r'(status|category|(^|[^a-z])(type|class|level|grade|tier|group|kind)($|[^a-z])|gender|segment)', None),
    "representation.identifier.alphanumeric_id": (1, 90, r'(^id$|_id$|_id_|^id_|\bcode\b|\bkey\b|\bref\b|\bsku\b|serial|identifier)', None),
    "datetime.component.year":                  (2, 50, r'(year|\byr\b|vintage|\banno\b)', None),
    "datetime.epoch.unix_seconds":              (2, 50, r'(timestamp|epoch|unix|_ts$|time_?stamp|created_?at|updated_?at)', None),
    "identity.commerce.isbn":                   (2, 50, r'isbn', None),
    "geography.address.postal_code":            (2, 50, r'(\bzip\b|zip_?code|postal|post_?code|postcode|\bplz\b|\bcap\b)', None),
    "finance.currency.amount":                  (2, 50, r'(price|cost|amount|\bfee\b|salary|revenue|payment|income|\btotal\b|spend|budget|\bgbp\b|\busd\b|\beur\b)', None),
    "technology.internet.data_uri":             (2, 50, r'(data_?uri|base64|img_?data|image_?data|thumbnail|favicon|icon_?data)', None),
    "technology.internet.top_level_domain":     (2, 50, r'(\btld\b|top_?level|domain_?suffix|domain_?ext|^suffix$)', None),
    "representation.numeric.integer_number":    (0, 60, None, None),
    "representation.numeric.decimal_number":    (0, 60, None, None),
    "representation.text.plain_text":           (0, 60, None, None),
    "datetime.date.iso":                        (0, 60, None, None),
}

BASE_FILTER = ("is_trivial = false AND sample_values_truncated IS NOT NULL "
               "AND length(sample_values_truncated) > 0 AND length(column_name) <= 64 "
               "AND column_name NOT LIKE '__index_level%'")


def anchor_seed_counts() -> tuple[dict[str, int], set[tuple[str, str]]]:
    counts: dict[str, int] = {}
    keys: set[tuple[str, str]] = set()
    with ANCHOR.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            counts[row["curated_label"]] = counts.get(row["curated_label"], 0) + 1
            keys.add((row["file_content_sha256"], row["column_name"]))
    return counts, keys


def main() -> int:
    con = duckdb.connect()
    seed_counts, used = anchor_seed_counts()
    out_rows: list[dict] = []
    shortfalls: list[dict] = []

    def draw(where: str, n: int, exclude: set[tuple[str, str]],
             base: str = BASE_FILTER) -> list[tuple]:
        if n <= 0:
            return []
        # Over-fetch 3x then dedup in python (exclusion set is small).
        q = f"""
            SELECT file_path, file_content_sha256, column_name,
                   sense_prediction, ydf_prediction_gated
            FROM '{PARQUET}'
            WHERE {base} AND ({where})
            ORDER BY md5(file_content_sha256 || column_name)
            LIMIT {n * 12}
        """
        rows = []
        per_header: dict[str, int] = {}
        for r in con.execute(q).fetchall():
            if (r[1], r[2]) in exclude:
                continue
            h = r[2].lower()
            if per_header.get(h, 0) >= 5:  # diversity: one header can't eat a stratum
                continue
            rows.append(r)
            exclude.add((r[1], r[2]))
            per_header[h] = per_header.get(h, 0) + 1
            if len(rows) == n:
                break
        return rows

    for label, (tier, gross, pos, neg) in STRATA.items():
        net = gross - seed_counts.get(label, 0)
        if net <= 0:
            continue
        if pos is None:  # backbone: random emission draw
            # No is_trivial filter here: the flag marks plain numeric/text
            # columns wholesale, which is precisely the backbone's population.
            got = draw(f"sense_prediction = '{label}'", net, used,
                       base=("sample_values_truncated IS NOT NULL "
                             "AND length(sample_values_truncated) > 0 "
                             "AND length(column_name) <= 64 "
               "AND column_name NOT LIKE '__index_level%'"))
            for r in got:
                out_rows.append(dict(zip(
                    ("file_path", "file_content_sha256", "column_name",
                     "sense_context", "ydf_context"), r),
                    stratum_label=label, tier="backbone", bucket="random_emit"))
            if len(got) < net:
                shortfalls.append({"stratum_label": label, "bucket": "random_emit",
                                   "missing": net - len(got)})
            continue

        n_a = math.ceil(net / 2)
        n_b = net - n_a
        neg_clause = f" AND NOT regexp_matches(lower(column_name), '{neg}')" if neg else ""
        a = draw(f"regexp_matches(lower(column_name), '{pos}'){neg_clause}", n_a, used)
        # bucket A shortfall rolls into B before being declared external.
        b_target = n_b + (n_a - len(a))
        b = draw(f"sense_prediction = '{label}' AND NOT regexp_matches(lower(column_name), '{pos}')",
                 b_target, used)
        for bucket, rows in (("header_pos", a), ("model_emit", b)):
            for r in rows:
                out_rows.append(dict(zip(
                    ("file_path", "file_content_sha256", "column_name",
                     "sense_context", "ydf_context"), r),
                    stratum_label=label, tier=f"tier{tier}", bucket=bucket))
        filled = len(a) + len(b)
        if filled < net:
            shortfalls.append({"stratum_label": label, "bucket": "header_pos",
                               "missing": net - filled})
        print(f"{label}: quota {gross} (seed {seed_counts.get(label, 0)}) -> "
              f"A {len(a)}/{n_a}, B {len(b)}, short {max(0, net - filled)}")

    fields = ["stratum_label", "tier", "bucket", "file_path", "file_content_sha256",
              "column_name", "sense_context", "ydf_context", "source", "licence",
              "curated_label", "labeller", "provenance"]
    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields, delimiter="\t")
        w.writeheader()
        for row in out_rows:
            row.update(source="gittables", licence="CC-BY-4.0 (Zenodo 5706316)",
                       curated_label="", labeller="",
                       provenance="build_gold_corpus_candidates.py deterministic md5-order draw")
            w.writerow(row)

    with EXT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=["stratum_label", "bucket", "missing"],
                           delimiter="\t")
        w.writeheader()
        w.writerows(shortfalls)

    print(f"\nwrote {len(out_rows)} candidates -> {OUT}")
    print(f"external mandate: {sum(s['missing'] for s in shortfalls)} columns "
          f"across {len(shortfalls)} strata -> {EXT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
