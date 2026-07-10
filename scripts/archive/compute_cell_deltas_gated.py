#!/usr/bin/env python3
"""ac-04 — Compute cell deltas against the GATED YDF baseline.

Variant of compute_v22_cell_deltas.py that uses `ydf_prediction_gated`
(from scripts/apply_ydf_validation_gate.py) instead of raw
`ydf_prediction`. The gated column is NULL for any YDF prediction
where fewer than 50% of sampled values pass the label's JSON Schema
validation — those columns drop out of cell-2 because they can't
contribute "missed geography" misses against a label that wasn't
honest to begin with.

Read:
  output/ydf-validation-gate/{v19,v20,v21,v22}_gated.parquet

Write:
  output/ydf-validation-gate/cell_deltas_gated.md

Per spec 2026-05-26-ydf-validation-gate ac-04.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GATE_DIR = REPO / "output/ydf-validation-gate"
DEFAULT_OUT = GATE_DIR / "cell_deltas_gated.md"

VERSIONS = ["v19", "v20", "v21", "v22"]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    con = duckdb.connect()
    metrics: dict[str, dict] = {}
    cell2_subtypes: dict[str, dict[str, int]] = {}

    for ver in VERSIONS:
        parquet = GATE_DIR / f"{ver}_gated.parquet"
        if not parquet.exists():
            print(f"warn: {parquet} missing — skipping", file=sys.stderr)
            continue
        print(f"reading {parquet}", file=sys.stderr)

        # cell-2 against the GATED YDF prediction.
        r = con.execute(f"""
            SELECT COUNT(DISTINCT file_path) AS files,
                   COUNT(*) FILTER (
                       WHERE sense_prediction NOT LIKE 'geography.%'
                         AND ydf_prediction_gated LIKE 'geography.%'
                   ) AS cell2_gated,
                   COUNT(*) FILTER (
                       WHERE sense_prediction NOT LIKE 'geography.%'
                         AND ydf_prediction       LIKE 'geography.%'
                   ) AS cell2_raw
              FROM read_parquet('{parquet.as_posix()}')
        """).fetchone()
        files, cell2_gated, cell2_raw = r
        metrics[ver] = {
            "files": files,
            "cell2_raw": cell2_raw,
            "cell2_gated": cell2_gated,
            "cell2_raw_per1k": cell2_raw / files * 1000,
            "cell2_gated_per1k": cell2_gated / files * 1000,
        }
        rows = con.execute(f"""
            SELECT ydf_prediction_gated AS ydf, COUNT(*) AS n
              FROM read_parquet('{parquet.as_posix()}')
             WHERE sense_prediction NOT LIKE 'geography.%'
               AND ydf_prediction_gated LIKE 'geography.%'
             GROUP BY 1
             ORDER BY n DESC
        """).fetchall()
        cell2_subtypes[ver] = dict(rows)
        print(f"  files={files:,}  cell2_raw={cell2_raw:,}  "
              f"cell2_gated={cell2_gated:,} "
              f"(Δ {cell2_gated - cell2_raw:+,})", file=sys.stderr)

    if "v19" not in metrics or "v22" not in metrics:
        print("error: v19 and v22 metrics required.", file=sys.stderr)
        return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    lines.append("# Gated cell-2 — v19/v20/v21/v22 against the cleaned baseline\n\n")
    lines.append("Per spec `2026-05-26-ydf-validation-gate` ac-04.\n\n")
    lines.append(
        "Cell-2 (`sense NOT LIKE 'geography.%' AND ydf LIKE 'geography.%'`) "
        "recomputed using `ydf_prediction_gated` instead of raw "
        "`ydf_prediction`. Any YDF prediction the gate refused (per "
        "ac-01) drops out — the metric stops penalising Sense for "
        "disagreeing with demonstrably-wrong YDF labels (msg_id as "
        "iso6346, stock_id as mgrs, team-codes as country_code, ...).\n\n"
    )

    # Headline comparison: raw vs gated, per version
    lines.append("## Raw vs gated cell-2\n\n")
    lines.append("| Variant | files | cell-2 raw | cell-2 gated | drop | raw per-1k | gated per-1k |\n")
    lines.append("|---|---:|---:|---:|---:|---:|---:|\n")
    for ver in VERSIONS:
        if ver not in metrics:
            continue
        m = metrics[ver]
        drop = m["cell2_raw"] - m["cell2_gated"]
        drop_pct = drop / m["cell2_raw"] * 100 if m["cell2_raw"] else 0
        lines.append(
            f"| {ver} | {m['files']:,} | {m['cell2_raw']:,} | "
            f"**{m['cell2_gated']:,}** | -{drop:,} (-{drop_pct:.1f}%) | "
            f"{m['cell2_raw_per1k']:.2f} | **{m['cell2_gated_per1k']:.2f}** |\n"
        )
    lines.append("\n")

    # Δ vs v19 on the gated baseline — this is the honest "did Sense improve" measure
    lines.append("## Δ vs v19 (gated baseline)\n\n")
    v19g = metrics["v19"]["cell2_gated_per1k"]
    lines.append("| Variant | gated per-1k | Δ vs v19 gated | band |\n")
    lines.append("|---|---:|---:|---|\n")
    for ver in VERSIONS:
        if ver not in metrics:
            continue
        m = metrics[ver]
        if ver == "v19":
            lines.append(f"| {ver} | {m['cell2_gated_per1k']:.2f} | — | baseline |\n")
            continue
        d = (m["cell2_gated_per1k"] - v19g) / v19g * 100
        sign = "−" if d < 0 else "+"
        if d <= -20:
            band = "**Met (≥ 20%)**"
        elif d <= -10:
            band = "**Partial (10–20%)**"
        else:
            band = "Failed (< 10%)"
        bold = "**" if ver == "v22" else ""
        lines.append(f"| {bold}{ver}{bold} | {m['cell2_gated_per1k']:.2f} | "
                     f"{bold}{sign}{abs(d):.1f}%{bold} | {band} |\n")
    lines.append("\n")

    # Per-subtype breakdown — the headline diagnostic
    if "v19" in cell2_subtypes and "v22" in cell2_subtypes:
        lines.append("## Per-subtype cell-2 — v19 → v22 (gated)\n\n")
        lines.append(
            "Where v22 actually moved the needle once the metric stops "
            "scoring against demonstrably-wrong YDF labels.\n\n"
        )
        lines.append("| Subtype | v19 gated | v22 gated | Δ |\n")
        lines.append("|---|---:|---:|---:|\n")
        v19_st = cell2_subtypes["v19"]
        v22_st = cell2_subtypes["v22"]
        all_subtypes = sorted(
            set(v19_st) | set(v22_st),
            key=lambda k: -max(v19_st.get(k, 0), v22_st.get(k, 0)),
        )
        for st in all_subtypes:
            n19 = v19_st.get(st, 0)
            n22 = v22_st.get(st, 0)
            if n19 == 0 and n22 == 0:
                continue
            d = n22 - n19
            pct = (d / n19 * 100) if n19 else 0
            short_st = st.replace("geography.", "")
            sign = "−" if d < 0 else ("+" if d > 0 else "")
            bold = "**" if abs(pct) >= 20 else ""
            d_str = f"{sign}{abs(d):,} ({sign}{abs(pct):.1f}%)" if n19 else f"+{d:,} (new)"
            lines.append(f"| {bold}{short_st}{bold} | {n19:,} | {n22:,} | {bold}{d_str}{bold} |\n")
        lines.append("\n")

    # Compare to noisy baseline
    lines.append("## Compare to noisy baseline (was v22 in band?)\n\n")
    v19_raw = metrics["v19"]["cell2_raw_per1k"]
    v22_raw = metrics["v22"]["cell2_raw_per1k"]
    d_raw = (v22_raw - v19_raw) / v19_raw * 100
    v19_g = metrics["v19"]["cell2_gated_per1k"]
    v22_g = metrics["v22"]["cell2_gated_per1k"]
    d_g = (v22_g - v19_g) / v19_g * 100
    lines.append(
        f"- v22 vs v19 on the **noisy** baseline: {d_raw:+.1f}% "
        f"(per output/corpus-pass-v22/cell_deltas.md — Failed band).\n"
    )
    lines.append(
        f"- v22 vs v19 on the **gated** baseline: {d_g:+.1f}%.\n"
    )
    if d_g <= -20:
        lines.append("\n**v22 was always in the Met band against an honest baseline.** "
                     "The Failed verdict was a measurement artefact.\n")
    elif d_g <= -10:
        lines.append("\n**v22 reaches the Partial band against an honest baseline.**\n")
    else:
        lines.append("\n**v22 is still Failed even on the gated baseline.** The "
                     "model genuinely under-recovers geography after the "
                     "noise correction.\n")

    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
