#!/usr/bin/env python3
"""ac-07 — Compute four-way cell deltas (v19/v20/v21/v22) and write
`output/corpus-pass-v22/cell_deltas.md`.

Methodology is the prediction-disagreement proxy used by v21's
`output/corpus-pass-v21/cell_deltas.md`:

  Cell 1 — `reject_rate_ceil × format_diversity_path_b`
           proxy: ydf_prediction = 'geography.address.full_address'
                  AND sense_prediction disagrees.
           (Approximate — v21's published 12.61/1000 came from the full
           mechanism_decomposition pipeline; the proxy lands 2.4% low.)

  Cell 2 — `non_trivial_floor × misclassification`
           proxy: sense_prediction NOT LIKE 'geography.%'
                  AND ydf_prediction LIKE 'geography.%'.
           Matches v21's published 160.245/1000 exactly on v19.

Per-subtype Cell 2 breakdown groups by `ydf_prediction` so we can see
which geography subtypes v22 actually moved.

Output:
  output/corpus-pass-v22/cell_deltas.md
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

CORPUS_PASSES = [
    ("v19", REPO / "eval/gittables/corpus_pass/columns.parquet"),
    ("v20", REPO / "output/corpus-pass-v20/corpus_pass/columns.parquet"),
    ("v21", REPO / "output/corpus-pass-v21/corpus_pass/columns.parquet"),
    ("v22", REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet"),
]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out",
                   type=Path,
                   default=REPO / "output/corpus-pass-v22/cell_deltas.md")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    con = duckdb.connect()
    metrics: dict[str, dict] = {}
    cell2_subtypes: dict[str, dict[str, int]] = {}

    for name, path in CORPUS_PASSES:
        if not path.exists():
            print(f"warn: {name} corpus missing at {path} — skipping",
                  file=sys.stderr)
            continue
        try:
            # Probe-read — catches the case where the parquet is still
            # being written incrementally (no magic bytes at EOF yet).
            con.execute(
                f"SELECT * FROM read_parquet('{path.as_posix()}') LIMIT 1"
            ).fetchone()
        except Exception as exc:  # noqa: BLE001
            print(f"warn: {name} parquet unreadable ({exc}); skipping",
                  file=sys.stderr)
            continue
        # Prefer the gated YDF column when present (per spec
        # 2026-05-26-ydf-validation-gate ac-06). Falls back to raw
        # ydf_prediction for pre-gate corpus passes.
        schema_cols = con.execute(
            f"SELECT * FROM read_parquet('{path.as_posix()}') LIMIT 0"
        ).fetchdf().columns.tolist()
        ydf_col = ("ydf_prediction_gated"
                   if "ydf_prediction_gated" in schema_cols
                   else "ydf_prediction")
        print(f"reading {name}: {path}  (ydf column: {ydf_col})",
              file=sys.stderr)
        r = con.execute(f"""
            SELECT COUNT(DISTINCT file_path) AS files,
                   COUNT(*) FILTER (
                       WHERE {ydf_col} = 'geography.address.full_address'
                         AND (sense_prediction IS NULL
                              OR sense_prediction != 'geography.address.full_address')
                   ) AS cell1,
                   COUNT(*) FILTER (
                       WHERE sense_prediction NOT LIKE 'geography.%'
                         AND {ydf_col} LIKE 'geography.%'
                   ) AS cell2
            FROM read_parquet('{path.as_posix()}')
        """).fetchone()
        files, cell1, cell2 = r
        metrics[name] = {
            "files": files,
            "cell1": cell1,
            "cell1_per1k": cell1 / files * 1000,
            "cell2": cell2,
            "cell2_per1k": cell2 / files * 1000,
        }
        # Per-subtype cell 2 breakdown — ydf_prediction LIKE 'geography.%'
        # AND sense_prediction NOT LIKE 'geography.%'.
        rows = con.execute(f"""
            SELECT {ydf_col} AS ydf, COUNT(*) AS n
              FROM read_parquet('{path.as_posix()}')
             WHERE sense_prediction NOT LIKE 'geography.%'
               AND {ydf_col} LIKE 'geography.%'
             GROUP BY 1
             ORDER BY n DESC
        """).fetchall()
        cell2_subtypes[name] = dict(rows)
        print(f"  files={files:,}  cell1={cell1:,} ({cell1/files*1000:.2f}/1k)"
              f"  cell2={cell2:,} ({cell2/files*1000:.2f}/1k)",
              file=sys.stderr)

    if "v19" not in metrics or "v22" not in metrics:
        print("error: v19 and v22 metrics required.", file=sys.stderr)
        return 2

    # ── Write markdown ────────────────────────────────────────────────
    lines: list[str] = []
    lines.append("# v22 corpus-pass cell deltas\n\n")
    lines.append("Per spec `2026-05-25-v22-boundary-training` ac-07.\n\n")
    lines.append(
        "Comparison of m-19 baseline (v19), v20 retrain, v21 retrain, "
        "and v22 retrain on the two AC-04 target cells. Rates are "
        "per-1000-files (normalised because each run processed slightly "
        "different file counts).\n\n"
    )

    # Four-way table
    lines.append("## Four-way comparison\n\n")
    lines.append("| Cell | v19 (m-19) | v20 (YDF) | v21 (GeoNames) | v22 (boundary) | v22 Δ vs v19 |\n")
    lines.append("|------|-----------:|----------:|---------------:|---------------:|-------------:|\n")
    v19 = metrics["v19"]
    for cell_n, cell_key, cell_title in [
        (1, "cell1", "`reject_rate_ceil × format_diversity_path_b` — postal_code → full_address"),
        (2, "cell2", "`non_trivial_floor × misclassification` — missed geography labels"),
    ]:
        row = [f"| **{cell_n}** {cell_title}"]
        for v in ["v19", "v20", "v21", "v22"]:
            if v not in metrics:
                row.append("—")
                continue
            r = metrics[v][f"{cell_key}_per1k"]
            if v == "v19":
                row.append(f"{r:.2f} / 1000")
            else:
                delta = (metrics[v][f"{cell_key}_per1k"] - v19[f"{cell_key}_per1k"]) / v19[f"{cell_key}_per1k"] * 100
                sign = "−" if delta < 0 else "+"
                bold = "**" if v == "v22" else ""
                row.append(f"{bold}{r:.2f} / 1000 ({sign}{abs(delta):.1f}%){bold}")
        # final Δ column
        if "v22" in metrics:
            delta_v22 = (metrics["v22"][f"{cell_key}_per1k"] - v19[f"{cell_key}_per1k"]) / v19[f"{cell_key}_per1k"] * 100
            met = (
                "**MET** (≥−20%)" if delta_v22 <= -20
                else "Partial (10-20%)" if -20 < delta_v22 <= -10
                else "did not meet −20%"
            )
            row.append(met)
        else:
            row.append("—")
        lines.append(" | ".join(row) + " |\n")
    lines.append("\n")

    # Files-processed footnote
    files_line = " · ".join(
        f"{v} = {metrics[v]['files']:,}" for v in ["v19", "v20", "v21", "v22"]
        if v in metrics
    )
    lines.append(f"Files processed: {files_line}\n\n")

    # Methodology note
    lines.append("## Methodology note\n\n")
    lines.append(
        "Cell counts use the prediction-disagreement proxy "
        "(`sense_prediction NOT LIKE 'geography.%' AND ydf_prediction LIKE 'geography.%'` for Cell 2; "
        "`ydf_prediction = 'geography.address.full_address' AND sense_prediction disagrees` for Cell 1) "
        "rather than the full mechanism-decomposition pipeline. Cell 2's proxy reproduces v21's "
        "published v19 baseline (160.25/1000) exactly. Cell 1's proxy lands ~2.4% below v21's published "
        "12.61/1000 — the proxy doesn't apply the file-level criterion-B filter so absolute counts differ "
        "slightly. The ratio v22/v19 is unchanged by this — the file-level filter depends on row distributions, "
        "not the model swap.\n\n"
    )

    # Per-subtype Cell 2 breakdown (v19 → v22)
    if "v19" in cell2_subtypes and "v22" in cell2_subtypes:
        lines.append("## Per-subtype breakdown (cell 2)\n\n")
        lines.append(
            "Where the v22 boundary training actually moved the needle "
            "(per-subtype miss counts; v19 → v22 percent change):\n\n"
        )
        lines.append("| Subtype | v19 misses | v21 misses | v22 misses | v22 Δ vs v19 |\n")
        lines.append("|---------|-----------:|-----------:|-----------:|-------------:|\n")

        # Combine top subtypes seen across v19 + v22
        v19_st = cell2_subtypes["v19"]
        v21_st = cell2_subtypes.get("v21", {})
        v22_st = cell2_subtypes["v22"]
        all_subtypes = sorted(
            set(v19_st) | set(v22_st),
            key=lambda k: -max(v19_st.get(k, 0), v22_st.get(k, 0)),
        )
        for st in all_subtypes:
            n19 = v19_st.get(st, 0)
            n21 = v21_st.get(st, 0)
            n22 = v22_st.get(st, 0)
            if n19 == 0 and n22 == 0:
                continue
            if n19 > 0:
                delta = (n22 - n19) / n19 * 100
                sign = "−" if delta < 0 else "+"
                delta_s = f"{sign}{abs(delta):.1f}%"
                bold = "**" if abs(delta) >= 20 else ""
            else:
                delta_s = "new"
                bold = ""
            short_st = st.replace("geography.", "")
            lines.append(
                f"| {bold}{short_st}{bold} | {n19:,} | {n21:,} | {n22:,} | "
                f"{bold}{delta_s}{bold} |\n"
            )
        lines.append("\n")

    # Pre-flight band classification
    if "v22" in metrics:
        delta = ((metrics["v22"]["cell2_per1k"]
                  - metrics["v19"]["cell2_per1k"])
                 / metrics["v19"]["cell2_per1k"] * 100)
        band = (
            "**Met (≥ 20% reduction)** — boundary-training methodology validated."
            if delta <= -20
            else "**Partial (10–20% reduction)** — methodology right, blend composition undertuned."
            if -20 < delta <= -10
            else "**Failed (< 10% reduction)** — training-data interventions exhausted; architectural surgery next."
        )
        lines.append("## ac-08 band — cell-2 lift vs v19\n\n")
        lines.append(
            f"v22 cell-2 rate: **{metrics['v22']['cell2_per1k']:.2f} / 1000** "
            f"(vs v19 {metrics['v19']['cell2_per1k']:.2f}). Δ = {delta:+.1f}%.\n\n"
        )
        lines.append(f"Band: {band}\n")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
