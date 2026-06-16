#!/usr/bin/env python3
"""ac-04b: decisive-stat sweep study
(spec 2026-06-16-duckdb-shellout-ingestion).

For each candidate neural-short-circuit predicate, sweep its strictness knob and
report, on the gold corpus, the CAPTURED count and SLICE PRECISION — the fraction
of captured columns whose curated_label equals the predicate's asserted label.
The ship setting for a predicate is the LOOSEST knob holding slice precision
>= 0.98 (the corpus-honest relocation gate, scripts/corpus_honest_gate.py, is a
separate blocking co-condition). A predicate that cannot reach 0.98 at any
setting does not get to skip.

Slice precision is a pure function of column values + predicate — no model. Full-
column stats come from DuckDB aggregates over the WHOLE column (what the ingestion
scan reads near-free), mirroring scripts/probe_column_stats.py.

Run: uv run --with duckdb python3 scripts/sweep_decisive_stats.py
"""
import csv
import os
from collections import Counter

import duckdb  # type: ignore[import-not-found]  # provided via `uv run --with duckdb`

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus.tsv")
DATASETS = "/Users/hugh/datasets/"
OUTDIR = os.path.join(REPO, "output/decisive-stat-sweep")

CAT = "representation.discrete.categorical"
ALN = "representation.identifier.alphanumeric_id"
INT = "representation.numeric.integer_number"
INC = "representation.identifier.increment"
BIN = "representation.boolean.binary"

con = duckdb.connect()
con.execute("SET threads=4")


def resolve(fp):
    for c in (os.path.join(DATASETS, fp), os.path.join(REPO, fp)):
        if os.path.exists(c):
            return c
    return None


def rel(path):
    return (f"read_parquet('{path}')" if path.endswith(".parquet")
            else f"read_csv_auto('{path}', SAMPLE_SIZE=-1, ignore_errors=true)")


def colstats(path, col):
    """Full-column stats via DuckDB aggregates. Returns dict or None."""
    q = col.replace('"', '""')
    src = rel(path)
    try:
        row = con.execute(f'''
            SELECT
              count("{q}")                                  AS n,
              count(DISTINCT "{q}")                         AS nd,
              count(TRY_CAST("{q}" AS DOUBLE))              AS n_num,
              min(TRY_CAST("{q}" AS DOUBLE))                AS mn,
              max(TRY_CAST("{q}" AS DOUBLE))                AS mx,
              count(DISTINCT TRY_CAST("{q}" AS DOUBLE))     AS nd_num,
              sum(CASE WHEN TRY_CAST("{q}" AS DOUBLE) = floor(TRY_CAST("{q}" AS DOUBLE))
                       THEN 1 ELSE 0 END)                   AS n_int,
              sum(CASE WHEN regexp_matches(CAST("{q}" AS VARCHAR), '[A-Za-z]')
                       THEN 1 ELSE 0 END)                   AS n_alpha,
              sum(CASE WHEN regexp_matches(CAST("{q}" AS VARCHAR), '[0-9]')
                       THEN 1 ELSE 0 END)                   AS n_digit
            FROM {src}
        ''').fetchone()
    except Exception:
        return None
    n, nd, n_num, mn, mx, nd_num, n_int, n_alpha, n_digit = row
    if not n or n < 5:
        return None
    card = nd / n
    num_frac = (n_num or 0) / n
    int_frac = (n_int or 0) / max(n_num or 0, 1)
    # increment signature: integer, near-unique, near-contiguous range
    contig = uniq = incr = 0.0
    if num_frac >= 0.95 and int_frac >= 0.99 and mn is not None and mx is not None and nd_num:
        span = (mx - mn + 1)
        contig = min(nd_num / span, 1.0) if span > 0 else 0.0
        uniq = nd_num / n
        incr = contig * uniq
    return dict(
        n=n, nd=nd, card=card, num_frac=num_frac, int_frac=int_frac,
        nd_num=nd_num or 0,
        mn=mn if mn is not None else float("nan"),
        mx=mx if mx is not None else float("nan"),
        alpha_frac=(n_alpha or 0) / n, digit_frac=(n_digit or 0) / n,
        contig=contig, uniq=uniq, incr=incr,
    )


# ---- predicates: (stats, knob) -> bool ---------------------------------------
def p_categorical(s, k):
    return s["card"] <= k


def p_alnum(s, k):
    return s["card"] >= k and s["alpha_frac"] >= 0.5 and s["digit_frac"] >= 0.5


def p_increment(s, k):
    return (s["num_frac"] >= 0.95 and s["int_frac"] >= 0.99 and s["nd_num"] >= 5
            and s["mn"] == s["mn"] and s["mn"] >= 0 and s["incr"] >= k)


def p_binary(s, k):
    # k = max fraction of values allowed OUTSIDE the integer {0,1} domain.
    if not (s["mn"] == s["mn"] and s["mx"] == s["mx"]):
        return False
    in_domain = (s["num_frac"] >= (1 - k) and s["int_frac"] >= 0.99
                 and s["mn"] >= 0 and s["mx"] <= 1 and s["nd_num"] <= 2)
    return in_domain


PREDICATES = [
    ("low_card->categorical", CAT, p_categorical,
     [0.01, 0.02, 0.05, 0.10, 0.20, 0.30, 0.50]),
    ("high_card+alnum->alphanumeric_id", ALN, p_alnum,
     [0.50, 0.70, 0.90, 0.95, 0.99, 1.00]),
    ("increment_signature->increment", INC, p_increment,
     [0.50, 0.70, 0.80, 0.90, 0.95]),
    ("exact_binary->binary", BIN, p_binary,
     [0.00, 0.01, 0.05]),
]


def main():
    gold = list(csv.DictReader(open(GOLD), delimiter="\t"))
    cols = []   # (curated_label, stats, column_name, path)
    skipped = 0
    for r in gold:
        p = resolve(r["file_path"])
        if not p:
            skipped += 1
            continue
        cn = r["column_name"]
        s = colstats(p, cn)
        if s is None:
            # resolve exact column name from schema, retry
            try:
                schema = [c[0] for c in con.execute(
                    f"DESCRIBE SELECT * FROM {rel(p)}").fetchall()]
                m = {str(c).strip().lower(): c for c in schema}
                cn2 = m.get(r["column_name"].strip().lower())
                if cn2:
                    cn = cn2
                    s = colstats(p, cn)
            except Exception:
                s = None
        if s is None:
            skipped += 1
            continue
        cols.append((r["curated_label"], s, cn, p))

    support = Counter(lbl for lbl, _, _, _ in cols)
    os.makedirs(OUTDIR, exist_ok=True)
    out = open(os.path.join(OUTDIR, "sweep.md"), "w")

    def emit(*a):
        line = " ".join(str(x) for x in a)
        print(line)
        out.write(line + "\n")

    emit(f"# ac-04b decisive-stat sweep — gold slice precision\n")
    emit(f"resolved {len(cols)}/{len(gold)} gold columns ({skipped} unresolved)\n")
    emit("gold support for asserted labels:")
    for _, lbl, _, _ in PREDICATES:
        emit(f"  {support.get(lbl,0):4d}  {lbl}")
    emit("")

    for name, asserted, pred, grid in PREDICATES:
        emit(f"\n## {name}")
        emit(f"asserted label: {asserted}  (gold support {support.get(asserted,0)})")
        emit(f"{'knob':>6} {'captured':>9} {'correct':>8} {'precision':>10}   top misfire labels")
        for k in grid:
            cap = [(lbl, s) for lbl, s, _, _ in cols if pred(s, k)]
            n_cap = len(cap)
            n_correct = sum(1 for lbl, _ in cap if lbl == asserted)
            prec = n_correct / n_cap if n_cap else float("nan")
            misfire = Counter(lbl.split(".")[-1] for lbl, _ in cap if lbl != asserted)
            top = ", ".join(f"{l}:{c}" for l, c in misfire.most_common(4))
            star = "  <== >=0.98" if (n_cap and prec >= 0.98) else ""
            emit(f"{k:>6} {n_cap:>9} {n_correct:>8} {prec:>10.3f}   {top}{star}")

    # ---- inspection: WHAT the gold-blind predicates fire on -------------------
    # Gold has 0 binary and 1 increment columns, so slice precision can't score
    # them. Dump the columns they DO fire on (name, gold label, sample values) to
    # judge whether asserting the label would relocate a real human label.
    def samples(path, col, lim=8):
        q = col.replace('"', '""')
        try:
            rows = con.execute(
                f'SELECT DISTINCT "{q}" FROM {rel(path)} WHERE "{q}" IS NOT NULL LIMIT {lim}'
            ).fetchall()
            return [str(r[0]) for r in rows]
        except Exception:
            return []

    for name, asserted, pred, k in [
        ("exact_binary->binary", BIN, p_binary, 0.0),
        ("increment_signature->increment", INC, p_increment, 0.80),
    ]:
        emit(f"\n## INSPECT {name} @ knob {k} — what it fires on (gold-blind)")
        fired = [(lbl, s, cn, p) for lbl, s, cn, p in cols if pred(s, k)]
        for lbl, s, cn, p in fired:
            sv = samples(p, cn)
            emit(f"  gold={lbl.split('.')[-1]:14s} card={s['card']:.3f} "
                 f"mn={s['mn']:.0f} mx={s['mx']:.0f} nd={s['nd']:<4d} "
                 f"col={cn!r} vals={sv}")

    out.write("\n[done]\n")
    out.close()
    print(f"\n[written to {os.path.join(OUTDIR, 'sweep.md')}]")


if __name__ == "__main__":
    main()
