#!/usr/bin/env python3
"""ac-06 — Re-apply v23's R26 country_code Sharpen rule to v22's corpus pass.

Reads v22's per-column predictions, applies R26 logic against the sample
values, writes a new parquet with the Sharpen-overridden label so ac-07
can compute the v22+v23 cell-2 delta without a fresh 6h corpus pass.

R26 is the only Sharpen rule v23 actually ships (see spec ac-04 + the
ac-01 finding that invalidated the other five planned rules). This script
mirrors the Rust implementation at
`crates/finetype-model/src/column.rs::sharpen_country_code_promote`
and the enums at `crates/finetype-model/src/country_code_enums.rs`. A
sanity test at the bottom of the script asserts the enum sizes match
the Rust constants so drift is caught at runtime.

Input:  output/corpus-pass-v22/corpus_pass/columns.parquet
Output: output/v23-sharpen-codes/columns_sharpened.parquet
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_IN = REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/v23-sharpen-codes/columns_sharpened.parquet"

# Mirrors crates/finetype-model/src/country_code_enums.rs::ISO_3166_1_ALPHA_2.
# Sanity-checked below — len must be 249.
ISO_3166_1_ALPHA_2 = frozenset([
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT",
    "AU", "AW", "AX", "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI",
    "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS", "BT", "BV", "BW", "BY",
    "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
    "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK",
    "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL",
    "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR",
    "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
    "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS",
    "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW",
    "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP",
    "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
    "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM",
    "SN", "SO", "SR", "SS", "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF",
    "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR", "TT", "TV", "TW",
    "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
])

# Mirrors crates/finetype-model/src/country_code_enums.rs::US_STATE_ALPHA_2.
# Sanity-checked below — len must be 56.
US_STATE_ALPHA_2 = frozenset([
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID",
    "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS",
    "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK",
    "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC", "AS", "GU", "MP", "PR", "VI",
])

assert len(ISO_3166_1_ALPHA_2) == 249, (
    f"ISO enum drift: got {len(ISO_3166_1_ALPHA_2)}, expected 249. "
    "Sync with crates/finetype-model/src/country_code_enums.rs.")
assert len(US_STATE_ALPHA_2) == 56, (
    f"US-state enum drift: got {len(US_STATE_ALPHA_2)}, expected 56. "
    "Sync with crates/finetype-model/src/country_code_enums.rs.")

MIN_VALUES = 5
ISO_RATE_THRESHOLD = 0.80
STATE_ONLY_RATE_CAP = 0.05
ISO_ONLY_DISTINCT_RATE_FLOOR = 0.30
MIN_ENUM_DISTINCT = 3
COUNTRY_CODE_LABEL = "geography.location.country_code"


def r26_promote(sense_label: str, values: list[str]) -> str | None:
    """Mirror of `sharpen_country_code_promote` in column.rs.

    Returns the promoted label ("geography.location.country_code") when
    the rule fires; None when it doesn't.
    """
    if sense_label is None or sense_label.startswith("geography."):
        return None

    non_empty = [v.strip() for v in values if v is not None and v.strip()]
    if len(non_empty) < MIN_VALUES:
        return None

    iso_match = 0
    state_only_match = 0
    iso_only_distinct: set[str] = set()
    overlap_distinct: set[str] = set()

    for v in non_empty:
        if len(v) != 2 or not v.isascii() or not v.isupper():
            continue
        in_iso = v in ISO_3166_1_ALPHA_2
        in_states = v in US_STATE_ALPHA_2
        if in_iso:
            iso_match += 1
            if in_states:
                overlap_distinct.add(v)
            else:
                iso_only_distinct.add(v)
        elif in_states:
            state_only_match += 1

    iso_rate = iso_match / len(non_empty)
    state_only_rate = state_only_match / len(non_empty)
    total_enum_distinct = len(iso_only_distinct) + len(overlap_distinct)
    if total_enum_distinct == 0:
        return None
    iso_only_distinct_rate = len(iso_only_distinct) / total_enum_distinct

    if (iso_rate >= ISO_RATE_THRESHOLD
            and state_only_rate <= STATE_ONLY_RATE_CAP
            and iso_only_distinct_rate >= ISO_ONLY_DISTINCT_RATE_FLOOR
            and total_enum_distinct >= MIN_ENUM_DISTINCT):
        return COUNTRY_CODE_LABEL
    return None


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--columns", type=Path, default=DEFAULT_IN)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
        import pyarrow as pa  # type: ignore
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb/pyarrow missing ({exc}); run with "
              "`uv run --with duckdb,pyarrow python3 scripts/apply_v23_sharpen_offline.py`",
              file=sys.stderr)
        return 2

    if not args.columns.exists():
        print(f"error: columns parquet missing at {args.columns}",
              file=sys.stderr)
        return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)
    print(f"reading {args.columns}", file=sys.stderr)
    con = duckdb.connect()

    # Read directly into an Arrow table — keeps the columnar layout end-to-end
    # so the only Python-side work is the per-row Sharpen decision.
    table = con.execute(f"""
        SELECT file_path, file_content_sha256, column_name,
               sense_prediction, sense_confidence,
               ydf_prediction, ydf_confidence,
               sample_values_truncated, is_trivial
          FROM read_parquet('{args.columns.as_posix()}')
    """).fetch_arrow_table()
    n = table.num_rows
    print(f"  {n:,} columns", file=sys.stderr)

    sense_preds = table["sense_prediction"].to_pylist()
    samples_col = table["sample_values_truncated"].to_pylist()

    sharpen_preds: list[str | None] = [None] * n
    sharpen_rules: list[str | None] = [None] * n
    fired = 0
    fired_by_prior: dict[str, int] = {}

    for i in range(n):
        sense_pred = sense_preds[i]
        samples = samples_col[i]
        if samples is None:
            sharpen_preds[i] = sense_pred
            continue
        values = samples.split('│')
        promoted = r26_promote(sense_pred or "", values)
        if promoted is not None:
            sharpen_preds[i] = promoted
            sharpen_rules[i] = "country_code_promote"
            fired += 1
            key = sense_pred or "<null>"
            fired_by_prior[key] = fired_by_prior.get(key, 0) + 1
        else:
            sharpen_preds[i] = sense_pred

    print(f"  R26 fired on {fired:,} columns "
          f"({fired/n*100:.3f}% of corpus)", file=sys.stderr)
    if fired_by_prior:
        print("  Promoted FROM these Sense labels (top 10):", file=sys.stderr)
        for label, k in sorted(fired_by_prior.items(), key=lambda x: -x[1])[:10]:
            print(f"    {label}: {k:,}", file=sys.stderr)

    # Append the two new columns to the Arrow table, write as parquet.
    out_table = table.append_column("sharpen_prediction", pa.array(sharpen_preds))
    out_table = out_table.append_column("sharpen_rule", pa.array(sharpen_rules))
    pq.write_table(out_table, args.out)
    print(f"wrote {args.out}  ({out_table.num_rows:,} rows)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
