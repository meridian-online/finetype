#!/usr/bin/env python3
"""t-000133e418 — build the attractor hard-negative distilled blend + audit gate.

Recipe: output/company-reference-audit/retrain_recipe_draft.md (author-approved 2026-07-05).
Additive over the v19 base (output/distillation-v3/sherlock_distilled.csv.gz), the
latdec/v27/clean-label pattern — the v22 additive chain is not reproducible from disk.

Six mined families, five destination labels:
  npi->integer_number      financial/id-header 10-digit columns the model calls npi
  npi->unix_seconds        market/weather epoch columns (regularMarketTime et al.)
  upc->integer_number      yfinance magnitudes + particleId runs the model calls upc
  user_agent->plain_text   prose columns the raw model calls user_agent (W1-demoted set)
  height/weight->decimal/integer/plain_text/word   instrument heights, statistical
                           weights, text under h/w headers ("N mph" wind cols EXCLUDED)
  locale_code->word        non-ISO 2-letter vocab (veg_type EN, act_tag sd)

Values are fetched FRESH from the source corpus parquet files (up to 32 non-null),
NOT from sample_values_truncated — prior builders shipped rows with 2-4 truncated
values that the loader's min_values>=5 floor silently dropped.

Mined rows carry their REAL corpus headers (identity-fortification precedent).

Audit gate (exit 3) on: per-family floors, leakage vs the gold/representative
identity lists, zero rows in COLUMN_LEVEL_TYPES post-remap, min-values floor.

Run under the gittables venv:
  eval/gittables/.venv/bin/python scripts/build_attractor_negatives_distilled.py
"""
from __future__ import annotations

import csv
import gzip
import hashlib
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import duckdb

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from gold_anchor_guard import load_gold_identities  # noqa: E402

AUDIT_DIR = REPO / "output/company-reference-audit"
BASELINE = AUDIT_DIR / "w3_baseline_with_oracle.parquet"
W2B_CAND = AUDIT_DIR / "eval_w2b_substance/sample_pass/corpus_pass/columns.parquet"
SEXP_PASS = AUDIT_DIR / "eval_sexp/sample_pass/corpus_pass/columns.parquet"
UA_DEMOTED = AUDIT_DIR / "ua_demoted_negatives.parquet"
BASE_BLEND = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
LABEL_REMAP = REPO / "data/label_remap.json"
REPR_FIXTURE = REPO / "eval/repr/representative_corpus.tsv"

OUT_DIR = REPO / "output/distillation-attneg"
OUT_BLEND = OUT_DIR / "sherlock_distilled_attneg.csv.gz"
OUT_MANIFEST = OUT_DIR / "attneg_blend_manifest.json"
OUT_PROVENANCE = OUT_DIR / "attneg_hard_negatives.tsv"

INT = "representation.numeric.integer_number"
DEC = "representation.numeric.decimal_number"
UNIX = "datetime.epoch.unix_seconds"
PLAIN = "representation.text.plain_text"
WORD = "representation.text.word"

COLUMN_LEVEL_TYPES = {
    "representation.discrete.categorical",
    "representation.discrete.ordinal",
    "representation.identifier.increment",
}

MIN_VALUES = 5
MAX_VALUES = 32
MAX_VALUE_CHARS = 400  # base blend fields max ~11.6k total; the csv loader caps fields at 128k
PER_HEADER_CAP = 50

YMD = "datetime.date.compact_ymd"

# Round-2 repair sources (gate NO-GO 2026-07-06): the round-1 candidate's damage sets,
# mined directly from the fresh gate passes. See retrain_recipe_draft.md §round 2.
R1_BASELINE = REPO / "output/attneg-retrain/baseline_with_oracle.parquet"
R1_CAND = REPO / "output/attneg-retrain/cand_pass/corpus_pass/columns.parquet"
YMD_RE = re.compile(r"^(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])$")
FIN_TAIL_RE = re.compile(
    r"(sellinggeneral|totalliab|netreceivable|^nav$|goodwill|retainedearnings|minorityinterest"
    r"|propertyplant|inventory|sharesoutstanding|surplus|stockholder|intangible|^wc$)", re.I)

ENT = "representation.text.entity_name"
ISO_DATE = "datetime.date.iso"

# (family, target) -> keep cap. Round-3 revision (derisk review 2026-07-06): decfix is
# re-mined from the damage transitions constant-first (the verified attractor is
# constant/near-constant numeric columns, NOT financial headers); txtpres preserves the
# text family against the whitespace_separated over-emit; datecw counterweights the
# 'date' header so ymdfix cannot mint a new date->compact_ymd shortcut.
MINED_KEEP = {
    ("decfix", DEC): 1200,
    ("ymdfix", YMD): 200,
    ("txtpres", ENT): 900,
    ("txtpres", PLAIN): 600,
    ("datecw", ISO_DATE): 150,
    ("npi", INT): 610,
    ("npi", UNIX): 150,
    ("upc", INT): 215,
    ("ua", PLAIN): 1500,
    ("height", DEC): 60,
    ("height", INT): 35,
    ("height", PLAIN): 160,
    ("height", WORD): 60,
    ("weight", DEC): 200,
    ("weight", INT): 60,
    ("weight", PLAIN): 120,
    ("weight", WORD): 50,
    ("locale", WORD): 400,
}

# Floors detect corruption (a family silently zeroed), not aspiration: height/locale
# lose ~25-40% of selected columns to the >=5-value fetch floor (tiny source files).
AUDIT_FAMILY_FLOORS = {"npi": 600, "upc": 180, "ua": 1200, "height": 150, "weight": 250, "locale": 200,
                       "decfix": 600, "ymdfix": 80, "txtpres": 600, "datecw": 40}
# unixfix (financial epoch-range 10-digit -> integer, teach-both-ways) is structurally
# EMPTY at the loader's min_values>=5 floor: all 300 corpus candidates live in <5-value
# per-ticker files (measured 2026-07-06). The financial->unix leak stays a documented
# residual; its selection query is kept below for the log.

FIN_RE = re.compile(
    r"(ebit|revenue|profit|asset|equity|marketcap|debt|income|cash|liabilit|invest"
    r"|balance|amount|value|price|cost|expense|sales|fund|capital)", re.I)
ID_RE = re.compile(r"((^|[_ ])id$|identifier|particle|product|item|sku|code|nbr|^no$|number)", re.I)
TIME_RE = re.compile(r"(time|date|epoch|utc|timestamp|unix)", re.I)
HW_GENUINE_RE = re.compile(r"(height|weight|(^|[_ ])ht$|(^|[_ ])wt$|mass|kg|lb|bmi)", re.I)
LOCALE_GENUINE_RE = re.compile(r"(locale|lang|language|culture|lcid)", re.I)
UNIT_SPEED_RE = re.compile(r"^\s*-?\d+(\.\d+)?\s*(mph|km/h|kph|m/s|knots?)\s*$", re.I)
UPC_EXCLUDE_HEADERS = {"_version", "project number", "tracking number",
                       "payee_id", "prov_id", "mbr_pcp_nbr"}
UA_EXCLUDE_HEADERS = {"responsible classes", "fan-in types", "fan-out types", "rates_basechar"}

ISO_639_1 = set("""aa ab ae af ak am an ar as av ay az ba be bg bh bi bm bn bo br bs ca ce ch co cr cs cu cv cy
da de dv dz ee el en eo es et eu fa ff fi fj fo fr fy ga gd gl gn gu gv ha he hi ho hr ht hu hy hz ia id ie ig
ii ik io is it iu ja jv ka kg ki kj kk kl km kn ko kr ks ku kv kw ky la lb lg li ln lo lt lu lv mg mh mi mk ml
mn mr ms mt my na nb nd ne ng nl nn no nr nv ny oc oj om or os pa pi pl ps pt qu rm rn ro ru rw sa sc sd se sg
si sk sl sm sn so sq sr ss st su sv sw ta te tg th ti tk tl tn to tr ts tt tw ty ug uk ur uz ve vi vo wa wo xh
yi yo za zh zu""".split())

EPOCH_LO, EPOCH_HI = 9e8, 2.2e9
PERSON_LO, PERSON_HI = 30.0, 250.0

# Full-corpus supplement: the 33k-sample demoted set is a 6.6% stratified draw of the
# corpus attractor population (npi 42,675 / upc 12,873 per stratified_sample.resolution.json),
# and most of its financial columns live in ~4-row per-ticker dump files that fail the
# loader's min_values>=5 floor. Supplement from the SAME population at corpus scale
# (eval/gittables/corpus_pass/columns.parquet), mirroring the guard's demote logic with a
# checksum-FAIL requirement so only provably-not-npi/upc columns enter.
CORPUS_PASS = REPO / "eval/gittables/corpus_pass/columns.parquet"
SUPPLEMENT_OVERSELECT = 8  # fetch this multiple of the remaining cap (small files drop out)


def luhn_ok(digits: str) -> bool:
    total = 0
    for i, ch in enumerate(reversed(digits)):
        d = int(ch)
        if i % 2 == 1:
            d *= 2
            if d > 9:
                d -= 9
        total += d
    return total % 10 == 0


def npi_check_ok(v: str) -> bool:
    v = v.strip()
    return len(v) == 10 and v.isdigit() and luhn_ok("80840" + v)


def gs1_check_ok(v: str) -> bool:
    v = v.strip()
    if len(v) != 12 or not v.isdigit():
        return False
    body, check = v[:-1], int(v[-1])
    total = sum(int(d) * (3 if i % 2 == 0 else 1) for i, d in enumerate(reversed(body)))
    return (10 - total % 10) % 10 == check


def h(label: str, header: str, sha: str) -> str:
    return hashlib.md5(f"{label}|{header}|{sha}".encode()).hexdigest()


def median(nums):
    s = sorted(nums)
    return s[len(s) // 2] if s else None


def parse_floats(vals):
    out = []
    for v in vals:
        try:
            out.append(float(str(v).replace(",", "").strip()))
        except (ValueError, TypeError):
            pass
    return out


def fetch_values(con, jobs):
    """jobs: list of (file_path, column_name, key). Returns key -> [str values]."""
    by_file = defaultdict(list)
    for fp, col, key in jobs:
        by_file[fp].append((col, key))
    out, failed = {}, 0
    for i, (fp, cols) in enumerate(by_file.items()):
        if i % 500 == 0:
            print(f"  value fetch: file {i}/{len(by_file)}", flush=True)
        for col, key in cols:
            qcol = col.replace('"', '""')
            try:
                rows = con.execute(
                    f'SELECT CAST("{qcol}" AS VARCHAR) FROM read_parquet(?) '
                    f'WHERE "{qcol}" IS NOT NULL LIMIT {MAX_VALUES}', [fp]).fetchall()
                vals = [str(r[0])[:MAX_VALUE_CHARS] for r in rows
                        if r[0] is not None and str(r[0]).strip() != ""]
                if len(vals) >= MIN_VALUES:
                    out[key] = vals[:MAX_VALUES]
                else:
                    failed += 1
            except Exception:
                failed += 1
    print(f"  value fetch done: {len(out)} ok, {failed} dropped (missing/short/err)", flush=True)
    return out


def corpus_supplement(con, gold_ids, per_header, per_bucket, kept_keys):
    """Fill npi/upc/unix bucket deficits from the full corpus pass.

    Same classification rules as the sample round; a >=50% checksum-PASS column is
    excluded (mirrors the guard: only provably-not-npi/upc columns become negatives)."""
    targets = [("npi", INT), ("npi", UNIX), ("upc", INT)]
    deficits = {bk: MINED_KEEP[bk] - per_bucket[bk] for bk in targets
                if per_bucket[bk] < MINED_KEEP[bk]}
    if not deficits or not CORPUS_PASS.exists():
        return []
    rows = con.execute(f"""
        SELECT file_path, column_name, file_content_sha256, sample_values_truncated,
               sense_prediction
        FROM read_parquet('{CORPUS_PASS}')
        WHERE sense_prediction IN ('identity.medical.npi', 'identity.commerce.upc')
          AND NOT is_trivial AND sample_values_truncated IS NOT NULL
    """).fetchall()
    print(f"  corpus supplement: {len(rows)} attractor-population rows, deficits {deficits}",
          flush=True)
    sel = []
    for fp, col, sha, tvals, label in rows:
        hdr = (col or "").strip()
        if (sha, hdr) in gold_ids or (fp, hdr) in kept_keys or not (tvals or "").strip():
            continue
        if label == "identity.medical.npi":
            med = median(parse_floats(tvals.split("│")))
            if TIME_RE.search(hdr) and med is not None and EPOCH_LO <= med <= EPOCH_HI:
                sel.append(dict(family="npi", target=UNIX, fp=fp, col=hdr, sha=sha,
                                check="none", source="corpus"))
            elif FIN_RE.search(hdr) or ID_RE.search(hdr):
                sel.append(dict(family="npi", target=INT, fp=fp, col=hdr, sha=sha,
                                check="npi", source="corpus"))
            elif med is not None and not (EPOCH_LO <= med <= EPOCH_HI):
                sel.append(dict(family="npi", target=INT, fp=fp, col=hdr, sha=sha,
                                check="npi", source="corpus"))
        else:
            if hdr.lower() in UPC_EXCLUDE_HEADERS:
                continue
            sel.append(dict(family="upc", target=INT, fp=fp, col=hdr, sha=sha,
                            check="gs1", source="corpus"))
    sel.sort(key=lambda c: h(c["target"], c["col"], c["sha"] or c["fp"]))
    take, tcount = [], Counter()
    for c in sel:
        bk = (c["family"], c["target"])
        if bk not in deficits or tcount[bk] >= deficits[bk] * SUPPLEMENT_OVERSELECT:
            continue
        tcount[bk] += 1
        take.append(c)
    vals = fetch_values(con, [(c["fp"], c["col"], i) for i, c in enumerate(take)])
    out = []
    for i, c in enumerate(take):
        if i not in vals:
            continue
        bk = (c["family"], c["target"])
        hk = (c["family"], c["target"], c["col"].lower())
        if deficits.get(bk, 0) <= 0 or per_header[hk] >= PER_HEADER_CAP:
            continue
        v = vals[i]
        if c["check"] == "npi":
            digits = [x for x in v if x.strip().isdigit() and len(x.strip()) == 10]
            if digits and sum(npi_check_ok(x) for x in digits) >= len(digits) / 2:
                continue  # majority pass the NPI check: could be real — not a negative
        elif c["check"] == "gs1":
            digits = [x for x in v if x.strip().isdigit() and len(x.strip()) == 12]
            if digits and sum(gs1_check_ok(x) for x in digits) >= len(digits) / 2:
                continue
        per_header[hk] += 1
        deficits[bk] -= 1
        out.append(dict(c, values=v))
    print(f"  corpus supplement kept: {Counter((c['family'], c['target']) for c in out)}",
          flush=True)
    return out


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    con = duckdb.connect()
    candidates = []  # dicts: family, target, file_path, column_name(header), sha

    # ---- npi + upc: guard-demoted sets (baseline sense = npi/upc) --------------------
    demoted = con.execute(f"""
        SELECT b.file_path, b.column_name, b.sense_prediction AS base_label,
               b.ydf_prediction_gated AS oracle, c.sense_prediction AS cand_label,
               c.file_content_sha256 AS sha, c.sample_values_truncated AS tvals
        FROM read_parquet('{BASELINE}') b
        JOIN read_parquet('{W2B_CAND}') c USING (file_path, column_name)
        WHERE b.sense_prediction IN ('identity.medical.npi', 'identity.commerce.upc')
    """).fetchall()
    for fp, col, base, oracle, cand, sha, tvals in demoted:
        hdr = col or ""
        if base == "identity.medical.npi":
            if cand == base or oracle == base:      # retained, or oracle co-signed npi
                continue
            if not (tvals or "").strip():
                continue
            med = median(parse_floats((tvals or "").split("│")))
            if TIME_RE.search(hdr) and med is not None and EPOCH_LO <= med <= EPOCH_HI:
                candidates.append(dict(family="npi", target=UNIX, fp=fp, col=hdr, sha=sha))
            elif FIN_RE.search(hdr) or ID_RE.search(hdr):
                candidates.append(dict(family="npi", target=INT, fp=fp, col=hdr, sha=sha))
            elif med is not None and not (EPOCH_LO <= med <= EPOCH_HI):
                candidates.append(dict(family="npi", target=INT, fp=fp, col=hdr, sha=sha))
            # else ambiguous: epoch-range values under a non-time header — excluded
        else:  # upc
            if hdr.strip().lower() in UPC_EXCLUDE_HEADERS:
                continue
            if cand == base:
                # coincidental checksum pass: include only via header+value rule
                if not FIN_RE.search(hdr):
                    continue
            candidates.append(dict(family="upc", target=INT, fp=fp, col=hdr, sha=sha))

    # ---- user_agent: W1-demoted prose set --------------------------------------------
    ua = con.execute(f"""
        SELECT u.file_path, u.column_name, u.vals, s.file_content_sha256 AS sha
        FROM read_parquet('{UA_DEMOTED}') u
        LEFT JOIN read_parquet('{SEXP_PASS}') s USING (file_path, column_name)
    """).fetchall()
    for fp, col, vals, sha in ua:
        hdr = (col or "").strip()
        if hdr.lower() in UA_EXCLUDE_HEADERS or not (vals or "").strip():
            continue
        first = (vals or "").split("│")[0]
        if len(first.split()) < 2:                   # short-token columns: skip (word noise)
            continue
        candidates.append(dict(family="ua", target=PLAIN, fp=fp, col=hdr, sha=sha or ""))

    # ---- height / weight / locale_code from the newest composed pass ------------------
    hw = con.execute(f"""
        SELECT file_path, column_name, sense_prediction, file_content_sha256,
               sample_values_truncated
        FROM read_parquet('{SEXP_PASS}')
        WHERE sense_prediction IN ('identity.person.height', 'identity.person.weight',
                                   'technology.code.locale_code')
    """).fetchall()
    for fp, col, label, sha, tvals in hw:
        hdr = (col or "").strip()
        vals = [v for v in (tvals or "").split("│") if v.strip()]
        if not vals:
            continue                                  # empty columns: untrainable
        fam = {"identity.person.height": "height", "identity.person.weight": "weight",
               "technology.code.locale_code": "locale"}[label]
        if fam == "locale":
            if LOCALE_GENUINE_RE.search(hdr):
                continue                              # genuine locale/lang headers: leave alone
            toks = {v.strip().lower() for v in vals}
            if all(re.fullmatch(r"[a-zA-Z]{2,3}", t) for t in toks):
                iso = sum(1 for t in toks if t in ISO_639_1)
                if toks and iso / len(toks) >= 0.8:
                    continue                          # real language codes: leave alone
            candidates.append(dict(family=fam, target=WORD, fp=fp, col=hdr, sha=sha))
            continue
        # height / weight
        if fam == "weight" and sum(bool(UNIT_SPEED_RE.match(v)) for v in vals) >= len(vals) / 2:
            continue                                  # "N mph" wind cols: taxonomy gap, excluded
        nums = parse_floats(vals)
        if len(nums) >= len(vals) / 2:                # numeric column
            med = median(nums)
            if med is not None and PERSON_LO <= med <= PERSON_HI and HW_GENUINE_RE.search(hdr):
                continue                              # plausibly a genuine person h/w column
            target = INT if all(float(n).is_integer() for n in nums) else DEC
            candidates.append(dict(family=fam, target=target, fp=fp, col=hdr, sha=sha))
        else:                                         # text under a h/w-emitting column
            multi = sum(1 for v in vals if len(v.split()) >= 2)
            target = PLAIN if multi >= len(vals) / 2 else WORD
            candidates.append(dict(family=fam, target=target, fp=fp, col=hdr, sha=sha))

    # ---- round-2 repairs: mine the round-1 candidate's damage sets ---------------------
    if R1_BASELINE.exists() and R1_CAND.exists():
        r1 = con.execute(f"""
            SELECT b.sense_prediction AS frm, c.sense_prediction AS dst,
                   b.file_path, b.column_name, c.file_content_sha256 AS sha,
                   c.sample_values_truncated AS tvals
            FROM read_parquet('{R1_BASELINE}') b
            JOIN read_parquet('{R1_CAND}') c USING (file_path, column_name)
            WHERE b.sense_prediction != c.sense_prediction
              AND (
                (b.sense_prediction = '{DEC}'
                 AND c.sense_prediction IN ('unknown', '{WORD}'))
                OR (b.sense_prediction = '{YMD}' AND c.sense_prediction IN ('{INT}', 'unknown'))
                OR (c.sense_prediction = '{UNIX}')
                OR (b.sense_prediction IN ('{ENT}', '{PLAIN}')
                    AND c.sense_prediction = 'container.array.whitespace_separated')
              )
        """).fetchall()
        n_r1 = {"decfix": 0, "decfix_const": 0, "unixfix": 0, "ymdfix": 0, "txtpres": 0}

        def near_constant(vs):
            return len(set(v.strip() for v in vs)) <= max(2, int(0.34 * len(vs)))

        for frm, dst, fp, col, sha, tvals in r1:
            hdr = (col or "").strip()
            vals = [v for v in (tvals or "").split("│") if v.strip()]
            if len(vals) < 5:   # below the loader's trainability floor — cannot repair from it
                continue
            if frm == DEC and dst in ("unknown", WORD):
                # the verified attractor: constant/near-constant float columns asserted
                # integer/scientific and vetoed (94% of the damage is constant-like) —
                # mine the damage itself, constants filling the bucket FIRST (prio 0)
                if re.match(r"^-?[0-9]+\.[0-9]+$", vals[0].strip()):
                    const = near_constant(vals)
                    candidates.append(dict(family="decfix", target=DEC, fp=fp, col=hdr,
                                           sha=sha, prio=0 if const else 1))
                    n_r1["decfix"] += 1
                    n_r1["decfix_const"] += int(const)
            elif frm == YMD:
                # genuine YYYYMMDD columns the round-1 model broke — the true-damage route
                # is CONSTANT repeated dates (asserted iso_8601, vetoed), so constants first
                ok = sum(bool(YMD_RE.match(v.strip())) for v in vals)
                if ok >= max(1, int(0.8 * len(vals))):
                    candidates.append(dict(family="ymdfix", target=YMD, fp=fp, col=hdr,
                                           sha=sha, prio=0 if near_constant(vals) else 1))
                    n_r1["ymdfix"] += 1
            elif dst == UNIX:
                # log only — structurally untrainable (tiny files), documented residual
                if (FIN_RE.search(hdr) or FIN_TAIL_RE.search(hdr)) and vals[0].strip().isdigit() \
                        and len(vals[0].strip()) == 10:
                    n_r1["unixfix"] += 1
            elif dst == "container.array.whitespace_separated":
                # text-preservation positives: the ws over-emit inflow is small-vocab
                # multi-word phrase columns — keep them at their baseline text label
                candidates.append(dict(family="txtpres", target=frm, fp=fp, col=hdr, sha=sha))
                n_r1["txtpres"] += 1
        print(f"  round-2 repair candidates (damage sets, >=5 tvals): {n_r1}", flush=True)

        # The damage concentrates in ~4-row per-ticker files that fail the loader's
        # min_values floor, so the trainable repair signal must come from the corpus's
        # LARGER tables: same value+header rules, sourced from the full corpus pass
        # (v19 lens as the source filter; value shape re-verified; fetch enforces >=5).
        # NOTE: the corpus pass marks plain numeric columns is_trivial=true (1.9M decimal
        # rows) — do NOT filter on it here; the value+header rules carry the selection.
        fin_sql = ("(?i)(ebit|revenue|profit|asset|equity|marketcap|debt|income|cash|liabilit"
                   "|invest|balance|amount|value|price|cost|expense|sales|fund|capital"
                   "|sellinggeneral|goodwill|retainedearnings|minorityinterest|propertyplant"
                   "|inventory|sharesoutstanding|surplus|stockholder|intangible|receivable)")
        n_r2c = {"decfix": 0, "unixfix": 0, "ymdfix": 0, "datecw": 0}
        q = {
            "decfix": f"""SELECT file_path, column_name, file_content_sha256
                FROM read_parquet('{CORPUS_PASS}')
                WHERE sense_prediction = '{DEC}'
                  AND regexp_matches(column_name, '{fin_sql}')
                  AND regexp_matches(split_part(sample_values_truncated, '│', 1),
                                     '^-?[0-9]+\\.[0-9]+$')
                  AND len(string_split(sample_values_truncated, '│')) >= 5""",
            "unixfix": f"""SELECT file_path, column_name, file_content_sha256
                FROM read_parquet('{CORPUS_PASS}')
                WHERE sense_prediction IN ('identity.medical.npi',
                                           'representation.numeric.integer_number')
                  AND regexp_matches(column_name, '{fin_sql}')
                  AND regexp_matches(split_part(sample_values_truncated, '│', 1), '^[0-9]{{10}}$')
                  AND TRY_CAST(split_part(sample_values_truncated, '│', 1) AS BIGINT)
                      BETWEEN 900000000 AND 2200000000
                  AND len(string_split(sample_values_truncated, '│')) >= 5""",
        }
        # datecw: counterweight so the 'date' header keeps its dominant corpus label and
        # ymdfix cannot mint a reverse 'date -> compact_ymd' shortcut (balance audit rec)
        q["datecw"] = f"""SELECT file_path, column_name, file_content_sha256
            FROM read_parquet('{CORPUS_PASS}')
            WHERE sense_prediction = '{ISO_DATE}'
              AND lower(trim(column_name)) IN ('date', 'dates', 'game_date', 'gamedate')
              AND len(string_split(sample_values_truncated, '│')) >= 5"""
        tgt = {"decfix": DEC, "unixfix": INT, "datecw": ISO_DATE}
        for fam, sql in q.items():
            for fp, col, sha in con.execute(sql).fetchall():
                candidates.append(dict(family=fam, target=tgt[fam], fp=fp,
                                       col=(col or "").strip(), sha=sha))
                n_r2c[fam] += 1
        for fp, col, sha, tvals in con.execute(f"""
                SELECT file_path, column_name, file_content_sha256, sample_values_truncated
                FROM read_parquet('{CORPUS_PASS}')
                WHERE sense_prediction = '{YMD}'
                  AND len(string_split(sample_values_truncated, '│')) >= 5""").fetchall():
            vals = [v for v in (tvals or "").split("│") if v.strip()]
            ok = sum(bool(YMD_RE.match(v.strip())) for v in vals)
            if vals and ok >= max(1, int(0.8 * len(vals))):
                candidates.append(dict(family="ymdfix", target=YMD, fp=fp,
                                       col=(col or "").strip(), sha=sha))
                n_r2c["ymdfix"] += 1
        print(f"  round-2 repair candidates (corpus, >=5 tvals): {n_r2c}", flush=True)

    # dedupe candidate identities (damage-set and corpus selections can overlap)
    seen_ids = set()
    deduped = []
    for c in candidates:
        k = (c["fp"], c["col"], c["family"])
        if k not in seen_ids:
            seen_ids.add(k)
            deduped.append(c)
    candidates = deduped

    # ---- leakage firewall -------------------------------------------------------------
    gold_ids = load_gold_identities()
    if REPR_FIXTURE.exists():
        with REPR_FIXTURE.open() as fh:
            for r in csv.DictReader(fh, delimiter="\t"):
                sha = (r.get("file_content_sha256") or "").strip()
                colname = (r.get("column_name") or "").strip()
                if sha and colname:
                    gold_ids.add((sha, colname))
    before = len(candidates)
    candidates = [c for c in candidates if (c["sha"], c["col"]) not in gold_ids]
    leaked = before - len(candidates)

    # ---- caps: per-header, then per-(family,target); constants (prio 0) fill first,
    # ---- then deterministic md5 order --------------------------------------------------
    candidates.sort(key=lambda c: (c.get("prio", 1), h(c["target"], c["col"], c["sha"] or c["fp"])))
    per_header: Counter = Counter()
    per_bucket: Counter = Counter()
    kept = []
    for c in candidates:
        hk = (c["family"], c["target"], c["col"].lower())
        bk = (c["family"], c["target"])
        if bk not in MINED_KEEP:
            continue
        if per_header[hk] >= PER_HEADER_CAP or per_bucket[bk] >= MINED_KEEP[bk]:
            continue
        per_header[hk] += 1
        per_bucket[bk] += 1
        kept.append(c)

    # ---- fetch real values from source corpus files ------------------------------------
    jobs = [(c["fp"], c["col"], i) for i, c in enumerate(kept)]
    values = fetch_values(con, jobs)
    kept = [dict(c, values=values[i], source="sample") for i, c in enumerate(kept)
            if i in values]

    # ---- refill npi/upc/unix deficits from the full corpus pass -------------------------
    # (the sample's financial columns live in ~4-row per-ticker files that fail the
    #  loader's min_values floor; the corpus has the same population in bigger files)
    per_bucket_fetched = Counter((c["family"], c["target"]) for c in kept)
    kept_keys = {(c["fp"], c["col"]) for c in kept}
    kept += corpus_supplement(con, gold_ids, per_header, per_bucket_fetched, kept_keys)

    # ---- denoise the integer negatives (hard-negative contamination, lit review) -------
    # An integer negative whose values are really dates re-teaches the compact_ymd drag;
    # EXCEPT under financial headers, where 8-digit amounts the baseline mislabelled as
    # dates are CORRECT negatives (challenge-agent verified: grossProfit=20100500).
    before_dn = len(kept)
    kept = [c for c in kept if not (
        c["target"] == INT
        and not (FIN_RE.search(c["col"]) or FIN_TAIL_RE.search(c["col"]))
        and c["values"]
        and sum(bool(YMD_RE.match(str(v).strip())) for v in c["values"]) >= 0.8 * len(c["values"])
    )]
    print(f"  denoise: dropped {before_dn - len(kept)} date-shaped integer negatives", flush=True)

    # ---- label-remap / column-level-types invariant ------------------------------------
    remap = json.loads(LABEL_REMAP.read_text()) if LABEL_REMAP.exists() else {}
    bad = [c for c in kept if remap.get(c["target"], c["target"]) in COLUMN_LEVEL_TYPES]

    # ---- write blend: base pass-through + mined rows ------------------------------------
    per_label_rows: Counter = Counter()
    fam_counts: Counter = Counter()
    with gzip.open(OUT_BLEND, "wt", newline="") as out:
        w = csv.writer(out)
        w.writerow(["final_label", "sample_values", "column_name"])
        n_base = 0
        n_poison = 0
        with gzip.open(BASE_BLEND, "rt", newline="") as fh:
            for r in csv.DictReader(fh):
                lbl = r.get("final_label", "")
                # base-blend poison: headerless whitespace_separated rows that are really
                # short phrases (artist names, titles) — direct 'phrase = array' supervision.
                # Genuine token lists (median >= 3 tokens/value) are kept.
                if lbl == "container.array.whitespace_separated":
                    try:
                        vs = json.loads(r.get("sample_values", "") or "[]")
                        toks = sorted(len(str(v).split()) for v in vs) if vs else [0]
                        if toks[len(toks) // 2] < 3:
                            n_poison += 1
                            continue
                    except (ValueError, TypeError):
                        pass
                w.writerow([lbl, r.get("sample_values", ""),
                            r.get("column_name", "") or ""])
                n_base += 1
        print(f"  base poison drop: {n_poison} phrase-valued whitespace_separated rows removed",
              flush=True)
        for c in kept:
            w.writerow([c["target"], json.dumps(c["values"]), c["col"]])
            per_label_rows[c["target"]] += 1
            fam_counts[c["family"]] += 1

    with OUT_PROVENANCE.open("w") as fh:
        pw = csv.writer(fh, delimiter="\t")
        pw.writerow(["family", "target", "source", "file_path", "column_name",
                     "file_content_sha256", "n_values"])
        for c in kept:
            pw.writerow([c["family"], c["target"], c.get("source", "sample"), c["fp"],
                         c["col"], c["sha"], len(c["values"])])

    per_bucket_final = Counter((c["family"], c["target"]) for c in kept)

    # per-header label entropy: the one-sided-header report the round-1 blend lacked
    hdr_labels: dict = defaultdict(Counter)
    for c in kept:
        hdr_labels[c["col"].lower()][c["target"]] += 1
    one_sided = []
    for hk, lc in hdr_labels.items():
        tot = sum(lc.values())
        top_label, top_n = lc.most_common(1)[0]
        if tot >= 5 and top_n / tot >= 0.9:
            one_sided.append({"header": hk, "label": top_label, "rows": tot})
    one_sided.sort(key=lambda x: -x["rows"])
    print(f"  one-sided headers (>=5 rows, >=90% single label): {len(one_sided)}; "
          f"top: {[(o['header'], o['rows']) for o in one_sided[:8]]}", flush=True)

    manifest = {
        "task": "t-000133e418", "recipe": "output/company-reference-audit/retrain_recipe_draft.md",
        "base": str(BASE_BLEND), "base_rows": n_base,
        "mined_rows": len(kept), "leaked_excluded": leaked,
        "per_family": dict(fam_counts), "per_label_rows": dict(per_label_rows),
        "per_source": dict(Counter(c.get("source", "sample") for c in kept)),
        "per_bucket": {f"{k[0]}->{k[1]}": v for k, v in per_bucket_final.items()},
        "caps": {"per_header": PER_HEADER_CAP,
                 "mined_keep": {f"{k[0]}->{k[1]}": v for k, v in MINED_KEEP.items()}},
        "min_values": MIN_VALUES, "max_values": MAX_VALUES,
        "base_poison_dropped": n_poison,
        "one_sided_headers": one_sided[:40],
    }
    OUT_MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest, indent=2))

    # ---- audit gate ---------------------------------------------------------------------
    errs = []
    if bad:
        errs.append(f"{len(bad)} mined rows land in COLUMN_LEVEL_TYPES post-remap")
    for fam, floor in AUDIT_FAMILY_FLOORS.items():
        if fam_counts[fam] < floor:
            errs.append(f"family {fam}: {fam_counts[fam]} rows < floor {floor}")
    if n_base < 100_000:
        errs.append(f"base pass-through {n_base} < 100k (corrupted base?)")
    short = [c for c in kept if len(c["values"]) < MIN_VALUES]
    if short:
        errs.append(f"{len(short)} mined rows below min_values={MIN_VALUES}")
    for bk, cap in MINED_KEEP.items():
        if per_bucket_final.get(bk, 0) == 0:
            errs.append(f"bucket {bk[0]}->{bk[1]} delivered ZERO rows (cap {cap}) — "
                        "empty buckets must be explicit, not silent")
    if errs:
        print("AUDIT GATE FAILED:\n  " + "\n  ".join(errs), file=sys.stderr)
        return 3
    print(f"AUDIT GATE OK — {n_base} base + {len(kept)} mined rows -> {OUT_BLEND}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
