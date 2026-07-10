#!/usr/bin/env python3
"""ac-00: enum-predicate calibration study
(spec 2026-06-17-enum-domain-emission).

Question: does character-shape COHESION (plus full-column cardinality) separate
DESIGNED enums from accidental low cardinality and from cohesive-but-open types?
No enum ground truth exists, so we bucket gold columns by label as a proxy and
measure how the predicate's dials distribute per bucket — separation, not an
accuracy number.

Run: eval/gittables/.venv/bin/python scripts/enum_predicate_study.py
"""
import csv
import os
import re
import statistics
import sys
from collections import Counter, defaultdict

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sweep_decisive_stats import resolve, con  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus.tsv")
OUTDIR = os.path.join(REPO, "output/enum-domain-emission")

# Label buckets (proxy for designed-enum vs not)
BUCKETS = {
    "ENUM": {  # designed, closed, finite-domain — the positives
        "representation.discrete.categorical", "representation.discrete.ordinal",
        "representation.boolean.binary", "representation.boolean.initials",
        "representation.boolean.terms",
    },
    "BOUNDED_SEMANTIC": {  # real bounded domains that are ALSO a specific type
        "geography.location.country_code", "geography.location.region",
        "geography.location.city", "geography.transportation.iata_code",
    },
    "OPEN_TEXT": {  # cohesive maybe, but open sets — should NOT read as closed enum
        "representation.text.entity_name", "representation.text.plain_text",
        "representation.text.word", "representation.text.full_name",
    },
    "DENYLIST": {  # numeric/datetime/id — enum is meaningless; should be excluded
        "representation.numeric.decimal_number", "representation.numeric.integer_number",
        "geography.coordinate.latitude", "geography.coordinate.longitude",
        "representation.identifier.alphanumeric_id", "representation.identifier.increment",
        "technology.internet.url",
    },
}
LABEL2BUCKET = {lbl: b for b, s in BUCKETS.items() for lbl in s}


def rel(p):
    return (f"read_parquet('{p}')" if p.endswith(".parquet")
            else f"read_csv_auto('{p}', SAMPLE_SIZE=-1, ignore_errors=true)")


def shape_sig(v: str) -> str:
    """Collapsed character-class signature: runs of upper/lower/digit collapse to
    U/l/d; other chars (space, punctuation) kept literally."""
    s = re.sub(r"[A-Z]+", "U", v)
    s = re.sub(r"[a-z]+", "l", s)
    s = re.sub(r"[0-9]+", "d", s)
    return s


def cohesion(values):
    """(shape-cohesion, length-cohesion) over distinct values, each in [0,1]."""
    vals = [v for v in values if v]
    if len(vals) < 2:
        return 1.0, 1.0
    sigs = [shape_sig(v) for v in vals]
    dominant = Counter(sigs).most_common(1)[0][1]
    coh_shape = dominant / len(vals)
    lens = [len(v) for v in vals]
    m = statistics.mean(lens)
    cv = (statistics.pstdev(lens) / m) if m else 0.0
    coh_len = max(0.0, 1.0 - cv)
    return coh_shape, coh_len


def col_profile(path, col, vcap=200):
    q = col.replace('"', '""')
    src = rel(path)
    try:
        n, nd = con.execute(f'SELECT count(*), count(DISTINCT "{q}") FROM {src}').fetchone()
        if not n:
            return None
        vals = [str(r[0]) for r in con.execute(
            f'SELECT DISTINCT "{q}" FROM {src} WHERE "{q}" IS NOT NULL LIMIT {vcap}'
        ).fetchall()]
        return n, nd, vals
    except Exception:
        return None


def main():
    os.makedirs(OUTDIR, exist_ok=True)
    rows = defaultdict(list)  # bucket -> [(distinct, ratio, coh_shape, coh_len)]
    for r in csv.DictReader(open(GOLD), delimiter="\t"):
        b = LABEL2BUCKET.get(r["curated_label"])
        if not b:
            continue
        p = resolve(r["file_path"])
        prof = col_profile(p, r["column_name"]) if p else None
        if prof is None:
            try:
                schema = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
                m = {str(c).strip().lower(): c for c in schema}
                cn = m.get(r["column_name"].strip().lower())
                prof = col_profile(p, cn) if cn else None
            except Exception:
                prof = None
        if prof is None:
            continue
        n, nd, vals = prof
        cs, cl = cohesion(vals)
        rows[b].append((nd, nd / n, cs, cl))

    out = open(os.path.join(OUTDIR, "predicate_study.md"), "w")

    def emit(*a):
        line = " ".join(str(x) for x in a)
        print(line)
        out.write(line + "\n")

    def med(xs):
        return statistics.median(xs) if xs else float("nan")

    emit("# ac-00 enum-predicate calibration — distributions by bucket\n")
    emit(f"{'bucket':18s} {'n':>4} {'distinct(med)':>14} {'ratio(med)':>11} "
         f"{'coh_shape(med)':>15} {'coh_len(med)':>13}")
    for b in ("ENUM", "BOUNDED_SEMANTIC", "OPEN_TEXT", "DENYLIST"):
        d = rows[b]
        if not d:
            continue
        emit(f"{b:18s} {len(d):>4} {med([x[0] for x in d]):>14.1f} "
             f"{med([x[1] for x in d]):>11.3f} {med([x[2] for x in d]):>15.3f} "
             f"{med([x[3] for x in d]):>13.3f}")

    # sweep the predicate: low-cardinality AND cohesive; report capture per bucket
    emit("\n## predicate sweep (distinct<=cap AND ratio<=rr AND coh_shape>=cc)")
    emit("   want: HIGH capture of ENUM(+BOUNDED), LOW capture of OPEN_TEXT")
    emit(f"{'cap':>4} {'rr':>5} {'cc':>5}  " + "  ".join(f"{b[:10]:>10}" for b in
         ("ENUM", "BOUNDED_SEMANTIC", "OPEN_TEXT", "DENYLIST")))
    for cap in (16, 32, 64):
        for rr in (0.3, 0.5):
            for cc in (0.5, 0.7, 0.9):
                line = f"{cap:>4} {rr:>5} {cc:>5}  "
                cells = []
                for b in ("ENUM", "BOUNDED_SEMANTIC", "OPEN_TEXT", "DENYLIST"):
                    d = rows[b]
                    if not d:
                        cells.append(f"{'-':>10}")
                        continue
                    fired = sum(1 for nd, rt, cs, cl in d if nd <= cap and rt <= rr and cs >= cc)
                    cells.append(f"{fired:>4}/{len(d):<5}")
                emit(line + "  ".join(cells))

    print(f"\n[written to {os.path.join(OUTDIR, 'predicate_study.md')}]")
    out.close()


if __name__ == "__main__":
    main()
