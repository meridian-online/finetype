#!/usr/bin/env python3
"""ac-04 — Cell-2 deltas, v23 vs v22 on the gated YDF baseline.

The companion to `compute_v23_per_cluster_fp_rate.py`. Where that
script measures whether the top-6 FP clusters shrank (the
precision-retrain's stated target), this one measures whether the
overall gated cell-2 ratio held — the v22 spec's pre-committed
acceptance band still has to be defended.

Compares v23's gated YDF predictions against v19 (the baseline lens)
and against v22 (the current default). Emits the same per-subtype
breakdown the v22 work used (`output/ydf-validation-gate/
cell_deltas_gated.md`), extended with the v23 column.

Reads:
  output/ydf-validation-gate/v19_gated.parquet
  output/ydf-validation-gate/v22_gated.parquet
  output/ydf-validation-gate/v23_gated.parquet

Writes:
  output/v23-precision-retrain/cell_deltas_v23_vs_v22.md

Per spec 2026-05-27-v23-precision-retrain ac-04.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GATE_DIR = REPO / "output/ydf-validation-gate"
DEFAULT_OUT = REPO / "output/v23-precision-retrain/cell_deltas_v23_vs_v22.md"

# ac-04 band thresholds, pre-committed in the spec — cell-2 component.
# Met:     gated cell-2 vs v19 ≤ −10.4% (v22's level — no regression)
# Partial: cell-2 within ±2pp of v22 (i.e. −8.4% to −12.4%)
# Failed:  cell-2 regresses ≥2pp vs v22 (i.e. > −8.4%)
V22_CELL2_PCT = -10.4
BAND_MET_CEILING = V22_CELL2_PCT          # cell-2 ≤ −10.4% to be Met
BAND_PARTIAL_FLOOR = V22_CELL2_PCT + 2.0  # cell-2 ≥ −8.4% to be worse than Partial
BAND_PARTIAL_CEILING = V22_CELL2_PCT - 2.0  # cell-2 ≤ −12.4% to be better than Partial (but still in band)


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v19", type=Path, default=GATE_DIR / "v19_gated.parquet")
    p.add_argument("--v22", type=Path, default=GATE_DIR / "v22_gated.parquet")
    p.add_argument("--v23", type=Path, default=GATE_DIR / "v23_gated.parquet")
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    for path in (args.v19, args.v22, args.v23):
        if not path.exists():
            print(f"error: missing {path}", file=sys.stderr)
            return 2

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    con = duckdb.connect()
    metrics: dict[str, dict] = {}
    cell2_subtypes: dict[str, dict[str, int]] = {}

    for ver, path in (("v19", args.v19), ("v22", args.v22), ("v23", args.v23)):
        print(f"reading {path}", file=sys.stderr)
        r = con.execute(f"""
            SELECT COUNT(DISTINCT file_path) AS files,
                   COUNT(*) FILTER (
                       WHERE sense_prediction NOT LIKE 'geography.%'
                         AND ydf_prediction_gated LIKE 'geography.%'
                   ) AS cell2_gated
              FROM read_parquet('{path.as_posix()}')
        """).fetchone()
        files, cell2_gated = r
        metrics[ver] = {
            "files": files,
            "cell2_gated": cell2_gated,
            "cell2_gated_per1k": cell2_gated / files * 1000 if files else 0.0,
        }
        rows = con.execute(f"""
            SELECT ydf_prediction_gated AS ydf, COUNT(*) AS n
              FROM read_parquet('{path.as_posix()}')
             WHERE sense_prediction NOT LIKE 'geography.%'
               AND ydf_prediction_gated LIKE 'geography.%'
             GROUP BY 1
             ORDER BY n DESC
        """).fetchall()
        cell2_subtypes[ver] = dict(rows)
        print(f"  files={files:,} cell2_gated={cell2_gated:,}",
              file=sys.stderr)

    v19g = metrics["v19"]["cell2_gated_per1k"]
    v22g = metrics["v22"]["cell2_gated_per1k"]
    v23g = metrics["v23"]["cell2_gated_per1k"]
    d_v22_vs_v19 = (v22g - v19g) / v19g * 100 if v19g else 0.0
    d_v23_vs_v19 = (v23g - v19g) / v19g * 100 if v19g else 0.0
    d_v23_vs_v22 = (v23g - v22g) / v22g * 100 if v22g else 0.0

    # Band verdict (cell-2 component)
    if d_v23_vs_v19 <= BAND_MET_CEILING:
        band_c2 = "Met"
    elif d_v23_vs_v19 <= BAND_PARTIAL_FLOOR:
        band_c2 = "Partial"
    else:
        band_c2 = "Failed"

    lines: list[str] = []
    lines.append("# v23 vs v22 — gated cell-2 deltas\n\n")
    lines.append("Per spec `2026-05-27-v23-precision-retrain` ac-04.\n\n")
    lines.append(
        "Cell-2 is the `sense NOT LIKE 'geography.%' AND ydf LIKE "
        "'geography.%'` miss count under the gated YDF baseline (per "
        "spec `2026-05-26-ydf-validation-gate`). v23 must not regress "
        "vs v22 to satisfy ac-04's cell-2 component.\n\n"
    )

    lines.append("## Δ vs v19 (gated baseline)\n\n")
    lines.append("| Variant | files | cell-2 gated | per-1k | Δ vs v19 | band |\n")
    lines.append("|---|---:|---:|---:|---:|---|\n")
    for ver, d in (("v19", 0.0), ("v22", d_v22_vs_v19), ("v23", d_v23_vs_v19)):
        m = metrics[ver]
        sign = "−" if d < 0 else "+"
        if ver == "v19":
            d_str = "baseline"
            band_str = "baseline"
        else:
            d_str = f"{sign}{abs(d):.1f}%"
            if d <= -20:
                band_str = "**Met (≥ 20%)**"
            elif d <= -10:
                band_str = "**Partial (10–20%)**"
            else:
                band_str = "Failed (< 10%)"
        bold = "**" if ver == "v23" else ""
        lines.append(
            f"| {bold}{ver}{bold} | {m['files']:,} | "
            f"{m['cell2_gated']:,} | {m['cell2_gated_per1k']:.2f} | "
            f"{bold}{d_str}{bold} | {band_str} |\n"
        )
    lines.append("\n")

    sign_22 = "−" if d_v23_vs_v22 < 0 else "+"
    lines.append("## Δ v22 → v23\n\n")
    lines.append(
        f"- v22 gated cell-2 per-1k: {v22g:.2f}\n"
        f"- v23 gated cell-2 per-1k: {v23g:.2f}\n"
        f"- Δ v23 vs v22: **{sign_22}{abs(d_v23_vs_v22):.1f}%**\n\n"
    )

    # Per-subtype breakdown so the v23 geography retention is visible
    all_subtypes = sorted(
        set(cell2_subtypes["v19"]) | set(cell2_subtypes["v22"]) | set(cell2_subtypes["v23"]),
        key=lambda k: -cell2_subtypes["v19"].get(k, 0),
    )
    lines.append("## Per-subtype cell-2 trajectory\n\n")
    lines.append(
        "Where v23's geography accuracy held vs v22, and where it "
        "moved (positive Δ = regression, more geography misses).\n\n"
    )
    lines.append("| Subtype | v19 | v22 | v23 | Δ v22→v23 |\n")
    lines.append("|---|---:|---:|---:|---:|\n")
    for st in all_subtypes:
        n19 = cell2_subtypes["v19"].get(st, 0)
        n22 = cell2_subtypes["v22"].get(st, 0)
        n23 = cell2_subtypes["v23"].get(st, 0)
        if max(n19, n22, n23) == 0:
            continue
        d = n23 - n22
        pct = (d / n22 * 100) if n22 else 0.0
        sign = "−" if d < 0 else ("+" if d > 0 else "")
        short_st = st.replace("geography.", "")
        bold = "**" if abs(pct) >= 10 else ""
        lines.append(
            f"| {bold}{short_st}{bold} | {n19:,} | {n22:,} | {n23:,} | "
            f"{bold}{sign}{abs(d):,} ({sign}{abs(pct):.1f}%){bold} |\n"
        )
    lines.append("\n")

    lines.append("## ac-04 band — cell-2 component\n\n")
    lines.append(
        f"Pre-committed thresholds (relative to v22's −10.4% vs v19):\n"
        f"  - **Met** — v23 cell-2 vs v19 ≤ {V22_CELL2_PCT:+.1f}% "
        "(no regression)\n"
        f"  - **Partial** — within ±2pp of v22 (i.e. "
        f"{BAND_PARTIAL_FLOOR:+.1f}% to {BAND_PARTIAL_CEILING:+.1f}%)\n"
        f"  - **Failed** — > {BAND_PARTIAL_FLOOR:+.1f}% "
        "(regresses ≥ 2pp vs v22)\n\n"
    )
    sign_19 = "−" if d_v23_vs_v19 < 0 else "+"
    lines.append(
        f"v23 reads: **{sign_19}{abs(d_v23_vs_v19):.1f}%** vs v19 gated.\n\n"
    )
    lines.append(f"**Verdict (cell-2 component): {band_c2}**.\n\n")
    lines.append(
        "The final ac-04 band combines this with the FP-rate verdict "
        "(see `per_cluster_fp_rate.md`). The pair must both meet their "
        "threshold for the overall band:\n"
        "  - Met overall: FP drop ≥ 50% AND cell-2 ≤ v22's −10.4%.\n"
        "  - Partial overall: FP drop 20–50% AND cell-2 within ±2pp.\n"
        "  - Failed overall: either component fails.\n"
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
