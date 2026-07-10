#!/usr/bin/env python3
"""ac-01 — Pattern-tightness audit for v23 Sharpen-stage code rules.

For each of the six target geography-code patterns (iso6346, mgrs,
plus_code, country_code-alpha-2, iata, icao), compute a per-
(ydf_prediction) breakdown of columns in v22's corpus pass where ≥80%
of the sampled values match the pattern. Output is a TSV that lets us
see, for each pattern, which other ydf_prediction buckets it would
sweep up if used as a Sharpen-stage promote trigger without an enum
gate.

Read:   output/corpus-pass-v22/corpus_pass/columns.parquet
Write:  output/v23-sharpen-codes/pattern_fp_audit.tsv

Methodology:
  - sample_values_truncated is │-delimited (up to N values per row).
  - For each row + pattern, count fraction of values matching the
    pattern; a column "hits" the pattern when ≥80% match AND there
    are ≥5 non-empty values (matches the runtime rule's min_values
    floor).
  - Group hitting columns by ydf_prediction; report n_columns.

The runtime promote rules (ac-03, ac-04) intentionally guard against
firing on v22's strong classes (city, region, country, etc.); the
audit also flags those collisions so we can size their precision
risk.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_IN = REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/v23-sharpen-codes/pattern_fp_audit.tsv"

# Patterns mirror the regexes documented in the v23 spec goal block.
# plus_code uses Open Location Code's 20-char alphabet (CFGHJMPQRVWX23456789);
# the spec's `[OLC-alphabet]` was shorthand.
PATTERNS: list[tuple[str, str, str]] = [
    ("iso6346", r"^[A-Z]{3}[UJZ][0-9]{7}$",
     "geography.transportation.iso6346"),
    ("mgrs", r"^[0-9]{1,2}[C-HJ-NP-X][A-HJ-NP-Z]{2}[0-9]{2,10}$",
     "geography.coordinate.mgrs"),
    ("plus_code", r"^[CFGHJMPQRVWX23456789]{8}\+[CFGHJMPQRVWX23456789]{2,}$",
     "geography.coordinate.plus_code"),
    ("country_code_alpha2", r"^[A-Z]{2}$",
     "geography.location.country_code"),
    ("iata", r"^[A-Z]{3}$",
     "geography.transportation.iata_code"),
    ("icao", r"^[A-Z]{4}$",
     "geography.transportation.icao_code"),
]

MIN_VALUES = 5
MATCH_THRESHOLD = 0.80


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=DEFAULT_IN)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}); run with "
              "`uv run --with duckdb python3 scripts/audit_geography_code_patterns.py`",
              file=sys.stderr)
        return 2

    con = duckdb.connect()
    return _run(con, args.columns, args.out)


def _run(con, columns_path: Path, out_path: Path) -> int:
    if not columns_path.exists():
        print(f"error: columns parquet missing at {columns_path}",
              file=sys.stderr)
        return 2

    print(f"reading {columns_path}", file=sys.stderr)

    out_path.parent.mkdir(parents=True, exist_ok=True)
    rows: list[tuple[str, str, str, int]] = []  # pattern, target, ydf, n_cols

    for name, regex, target in PATTERNS:
        print(f"  pattern {name}: {regex}", file=sys.stderr)
        # DuckDB list_filter + regexp_matches lets us count matches per
        # row without exploding the table.
        sql = f"""
        WITH split AS (
            SELECT
                ydf_prediction,
                sense_prediction,
                list_filter(
                    string_split(sample_values_truncated, '│'),
                    v -> v IS NOT NULL AND length(trim(v)) > 0
                ) AS values
            FROM read_parquet('{columns_path.as_posix()}')
            WHERE sample_values_truncated IS NOT NULL
        ),
        scored AS (
            SELECT
                ydf_prediction,
                sense_prediction,
                length(values) AS n,
                length(
                    list_filter(values,
                                v -> regexp_matches(v, '{regex}'))
                ) AS k
            FROM split
            WHERE length(values) >= {MIN_VALUES}
        ),
        hits AS (
            SELECT ydf_prediction
              FROM scored
             WHERE n > 0
               AND (k::DOUBLE / n) >= {MATCH_THRESHOLD}
        )
        SELECT COALESCE(ydf_prediction, '(null)') AS ydf,
               COUNT(*) AS n_cols
          FROM hits
         GROUP BY 1
         ORDER BY n_cols DESC
        """
        for ydf, n_cols in con.execute(sql).fetchall():
            rows.append((name, target, ydf, n_cols))

    # Write TSV
    with out_path.open("w") as fh:
        fh.write("pattern\ttarget_ydf\tobserved_ydf\tn_columns\tis_target\n")
        for pattern, target, ydf, n_cols in rows:
            is_target = "true" if ydf == target else "false"
            fh.write(f"{pattern}\t{target}\t{ydf}\t{n_cols}\t{is_target}\n")
    print(f"wrote {out_path}  ({len(rows)} rows)", file=sys.stderr)

    # Summary banner — surface the headline so a reviewer can sanity-
    # check ac-01's close criteria without opening the TSV.
    print("\nfalse-positive summary (cols outside target_ydf):", file=sys.stderr)
    print(f"  {'pattern':<22} {'target_hits':>11}  {'fp_hits':>8}  "
          f"{'top_fp_buckets'}", file=sys.stderr)
    for name, _, target in PATTERNS:
        matching = [(r[2], r[3]) for r in rows if r[0] == name]
        target_hits = sum(n for ydf, n in matching if ydf == target)
        fp_rows = [(ydf, n) for ydf, n in matching if ydf != target]
        fp_total = sum(n for _, n in fp_rows)
        top = ", ".join(
            f"{ydf.replace('geography.', 'g.').replace('representation.', 'r.')}"
            f"={n}"
            for ydf, n in sorted(fp_rows, key=lambda x: -x[1])[:3]
        )
        print(f"  {name:<22} {target_hits:>11}  {fp_total:>8}  {top}",
              file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
