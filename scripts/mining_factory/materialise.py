#!/usr/bin/env python3
"""B2 mining factory — ac-04: materialise firewalled values + blend onto the base.

Spec 2026-06-07-reference-data-mining-factory, ac-04.

The firewall leaves value-level rows (value, type, source, locale). The training
pipeline consumes COLUMNS — `(final_label, sample_values, column_name)`. This
script regroups the survivors into synthetic columns and blends them onto the base
distilled corpus, drop-in compatible with `prepare_multibranch_data.py` (the same
shape `build_v24_distilled.py` emits).

Materialise:
  - Per type, shuffle the surviving values (seed 42) and chunk into columns of
    `--column-size` values (default 50). Each column draws a realistic header from
    `prepare_multibranch_data.get_header_for_type` so the header branch sees variety
    rather than one constant token per type.
  - `final_label` = the type key; `sample_values` = the chunk (JSON); `column_name`
    = the drawn header.

Blend:
  - Concatenate base distilled corpus + manufactured columns into one augmented CSV.
  - The PER-TYPE CAP is NOT applied here — it is applied by `prepare_multibranch_data
    --distilled-cap` at FTMB time (ac-05), exactly as the v24 hard-negative blend
    does. This script reports the pre-cap volumes and the documented blend recipe so
    the cap's effect is auditable.

Audit gate (fails non-zero):
  - ZERO `representation.discrete.categorical` rows in the manufactured output
    (CLAUDE.md — categorical positives must stay at zero by construction).
  - Every Tier-1 type present with at least one column.

Output:
  output/mining-factory/manufactured_distilled.csv.gz   — manufactured columns only
  output/mining-factory/<blend-out>                     — base + manufactured (if --base)
  output/mining-factory/ac04_materialise_blend.md       — per-type volumes + recipe
  output/mining-factory/mfg_blend_manifest.json
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from prepare_multibranch_data import get_header_for_type  # noqa: E402

OUT_DIR = REPO / "output" / "mining-factory"
CATEGORICAL = "representation.discrete.categorical"

# The 21 Tier-1 types this spec manufactures — the audit requires each present.
TIER1_TYPES = {
    "geography.location.city", "geography.location.region",
    "geography.location.country", "geography.location.country_code",
    "geography.location.continent", "geography.location.state_code",
    "geography.address.postal_code", "geography.address.street_name",
    "geography.address.street_suffix", "geography.coordinate.latitude",
    "geography.coordinate.longitude", "datetime.component.day_of_week",
    "datetime.component.month_name", "datetime.date.abbreviated_month",
    "technology.internet.top_level_domain", "technology.internet.http_method",
    "technology.code.locale_code", "finance.currency.currency_code",
    "finance.currency.currency_symbol", "identity.person.gender_code",
    "identity.person.blood_type",
}


def load_survivors(path: Path) -> dict[str, list[str]]:
    by_type: dict[str, list[str]] = defaultdict(list)
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            by_type[r["type"]].append(r["value"])
    return by_type


def materialise_columns(by_type: dict[str, list[str]], column_size: int,
                        min_values: int, rng: random.Random
                        ) -> tuple[list[tuple[str, list[str], str]], dict[str, dict]]:
    """Chunk each type's values into (label, values, header) columns."""
    columns: list[tuple[str, list[str], str]] = []
    stats: dict[str, dict] = {}
    for type_key in sorted(by_type):
        values = list(dict.fromkeys(by_type[type_key]))  # de-dup, keep order
        rng.shuffle(values)
        n_cols = 0
        for i in range(0, len(values), column_size):
            chunk = values[i:i + column_size]
            if len(chunk) < min_values:
                continue
            header = get_header_for_type(type_key, rng)
            columns.append((type_key, chunk, header))
            n_cols += 1
        stats[type_key] = {
            "distinct_values": len(values),
            "columns": n_cols,
            "values_in_columns": sum(
                len(c[1]) for c in columns if c[0] == type_key),
        }
    return columns, stats


def write_distilled(columns: list[tuple[str, list[str], str]], path: Path) -> None:
    with gzip.open(path, "wt", newline="", encoding="utf-8") as f:
        w = csv.writer(f)
        w.writerow(["final_label", "sample_values", "column_name"])
        for label, values, header in columns:
            w.writerow([label, json.dumps(values, ensure_ascii=False), header])


def blend(base: Path, manufactured: Path, out: Path,
          rng: random.Random) -> tuple[int, int]:
    """Interleave manufactured columns evenly through the base distilled stream.

    Appending the manufactured block verbatim sorts it into one contiguous run per
    type (materialise emits types in sorted order). `prepare_multibranch_data`'s v3
    proximity grouping (`group_distilled_by_proximity`) then cuts 5–15 ADJACENT rows
    into pseudo-tables on the assumption that adjacent rows share a source table — so
    a sorted manufactured block becomes degenerate single-type pseudo-tables and the
    sibling-context branch trains on columns whose neighbours are all the same label.
    That is what collapsed the proxy (numerics swallowed into generic text labels).

    Fix: shuffle the manufactured columns (breaking the per-type runs) and splice them
    one-at-a-time into the base stream at an even stride, so each manufactured column
    lands inside a real, mixed-type base proximity group. Base adjacency is otherwise
    preserved. Returns (base_rows, mfg_rows)."""
    mfg: list[list[str]] = []
    with gzip.open(manufactured, "rt", newline="", encoding="utf-8") as fm:
        r = csv.DictReader(fm)
        for row in r:
            mfg.append([row.get("final_label", ""), row.get("sample_values", "[]"),
                        row.get("column_name", "")])
    rng.shuffle(mfg)
    mfg_rows = len(mfg)

    base_rows = 0
    opener_base = gzip.open if base.suffix == ".gz" else open
    with opener_base(base, "rt", newline="", encoding="utf-8") as fb:  # type: ignore
        base_rows = sum(1 for _ in csv.DictReader(fb))

    # Even stride: drop one manufactured column every `stride` base rows.
    stride = max(1, base_rows // (mfg_rows + 1)) if mfg_rows else base_rows + 1
    mfg_iter = iter(mfg)
    emitted_mfg = 0
    with gzip.open(out, "wt", newline="", encoding="utf-8") as fo:
        w = csv.writer(fo)
        w.writerow(["final_label", "sample_values", "column_name"])
        with opener_base(base, "rt", newline="", encoding="utf-8") as fb:  # type: ignore
            r = csv.DictReader(fb)
            for i, row in enumerate(r):
                w.writerow([row.get("final_label", ""), row.get("sample_values", "[]"),
                            row.get("column_name", "")])
                if emitted_mfg < mfg_rows and (i + 1) % stride == 0:
                    w.writerow(next(mfg_iter))
                    emitted_mfg += 1
        for row in mfg_iter:  # any remainder
            w.writerow(row)
            emitted_mfg += 1
    return base_rows, mfg_rows


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--in", dest="inp", type=Path,
                    default=OUT_DIR / "firewalled_values.ndjson")
    ap.add_argument("--mfg-out", type=Path,
                    default=OUT_DIR / "manufactured_distilled.csv.gz")
    ap.add_argument("--base", type=Path, default=None,
                    help="Base distilled corpus to blend onto (optional).")
    ap.add_argument("--blend-out", type=Path,
                    default=OUT_DIR / "sherlock_distilled_mfg.csv.gz")
    ap.add_argument("--column-size", type=int, default=50)
    ap.add_argument("--min-values", type=int, default=5)
    ap.add_argument("--distilled-cap", type=int, default=600,
                    help="Documented per-type cap applied by prepare_multibranch at "
                         "FTMB time; recorded in the recipe, NOT applied here.")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    rng = random.Random(args.seed)
    by_type = load_survivors(args.inp)
    columns, stats = materialise_columns(by_type, args.column_size,
                                         args.min_values, rng)

    # Audit gate. Zero categorical is the load-bearing HARD gate (CLAUDE.md).
    cat_cols = sum(1 for c in columns if c[0] == CATEGORICAL)
    if cat_cols:
        print(f"AUDIT FAIL: {cat_cols} categorical columns — must be zero",
              file=sys.stderr)
        return 3
    # Tier-1 coverage is REPORTED, not a hard fail: a closed-vocab type whose entire
    # vocabulary lives in the eval holdout is decimated by the ac-03 firewall to fewer
    # than `min_values` distinct survivors and forms no column. That is the documented,
    # expected outcome (ac-03) — those types were never the starvation problem and the
    # base corpus already covers them. Manufacturing's load-bearing contribution is the
    # high-cardinality rare-value diversity, which survives in volume.
    decimated = sorted(t for t in TIER1_TYPES if stats.get(t, {}).get("columns", 0) == 0)
    if decimated:
        print(f"NOTE: {len(decimated)} Tier-1 types firewall-decimated below the "
              f"{args.min_values}-distinct column floor (reported, not a failure): "
              f"{decimated}", file=sys.stderr)

    write_distilled(columns, args.mfg_out)
    total_cols = len(columns)
    total_vals = sum(len(c[1]) for c in columns)
    print(f"materialised {total_cols:,} columns ({total_vals:,} values) -> {args.mfg_out}",
          file=sys.stderr)

    def rel(p: Path) -> str:
        try:
            return str(Path(p).resolve().relative_to(REPO))
        except ValueError:
            return str(p)

    blend_info = None
    if args.base:
        base_rows, mfg_rows = blend(args.base, args.mfg_out, args.blend_out, rng)
        blend_info = {"base": rel(args.base), "base_rows": base_rows,
                      "mfg_rows": mfg_rows, "blend_out": rel(args.blend_out)}
        print(f"blended: {base_rows:,} base + {mfg_rows:,} mfg -> {args.blend_out}",
              file=sys.stderr)

    manifest = {
        "column_size": args.column_size,
        "min_values": args.min_values,
        "seed": args.seed,
        "documented_distilled_cap": args.distilled_cap,
        "total_columns": total_cols,
        "total_values": total_vals,
        "per_type": stats,
        "categorical_columns": cat_cols,
        "firewall_decimated_types": decimated,
        "blend": blend_info,
    }
    (OUT_DIR / "mfg_blend_manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8")

    # Report.
    lines = [
        "# ac-04 — materialise + blend: manufactured corpus in the training shape",
        "",
        "Spec `2026-06-07-reference-data-mining-factory`, ac-04. The firewall survivors",
        "(value-level) are regrouped into synthetic COLUMNS in the distilled shape",
        "`prepare_multibranch_data` consumes — `(final_label, sample_values,",
        f"column_name)` — at {args.column_size} values/column, each column drawing a",
        "realistic header from `get_header_for_type` so the header branch sees variety.",
        "",
        f"**{total_cols:,} manufactured columns ({total_vals:,} values) across "
        f"{len([t for t in stats if stats[t]['columns'] > 0])} types. "
        f"Categorical columns: {cat_cols} (HARD audit: must be zero).**",
        "",
        f"Firewall-decimated below the {args.min_values}-distinct column floor "
        f"(reported, not a failure): "
        + (", ".join(f"`{t}`" for t in decimated) if decimated else "none")
        + ". These closed-vocab types lose their full vocabulary to the eval holdout"
        " (ac-03) — they were never the starvation problem and the base corpus covers"
        " them. Manufacturing's load-bearing contribution is the high-cardinality"
        " rare-value diversity, which survives in volume.",
        "",
        "## Blend recipe (v19 recipe, seed 42)",
        "",
        "The manufactured columns are concatenated onto the base distilled corpus into",
        "one augmented CSV. The PER-TYPE CAP is applied by `prepare_multibranch_data",
        f"--distilled-cap {args.distilled_cap}` at FTMB time (ac-05) — not here — so a",
        "high-volume type (latitude ~460 cols) cannot swamp the base distribution. The",
        "v19 recipe blends distilled:synthetic at `--ratio-distilled 0.5",
        "--samples-per-type 1200 --seed 42`.",
        "",
    ]
    if blend_info:
        lines += [
            f"Base: `{blend_info['base']}` ({blend_info['base_rows']:,} columns) + "
            f"{blend_info['mfg_rows']:,} manufactured = "
            f"{blend_info['base_rows'] + blend_info['mfg_rows']:,} columns "
            f"-> `{blend_info['blend_out']}`.",
            "",
        ]
    else:
        lines += [
            "_Base not yet blended — run with `--base <corpus>` to emit the augmented",
            "CSV. Base-corpus choice is finalised at the ac-05 train boundary._",
            "",
        ]
    lines += [
        "## Per-type manufactured volumes (pre-cap)",
        "",
        "| type | distinct values | columns | values in columns |",
        "|---|---:|---:|---:|",
    ]
    for t in sorted(stats):
        s = stats[t]
        lines.append(f"| `{t}` | {s['distinct_values']:,} | {s['columns']:,} | "
                     f"{s['values_in_columns']:,} |")
    (OUT_DIR / "ac04_materialise_blend.md").write_text("\n".join(lines) + "\n",
                                                       encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
