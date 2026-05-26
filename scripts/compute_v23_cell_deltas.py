#!/usr/bin/env python3
"""ac-07 — Five-way cell-2 deltas including v22+v23 (R26 Sharpen).

Reads:
  - v19/v20/v21/v22 corpus passes (raw sense_prediction)
  - v22 corpus pass overlaid with v23's R26 Sharpen rule
    (output/v23-sharpen-codes/columns_sharpened.parquet, from ac-06)

Writes:
  output/v23-sharpen-codes/cell_deltas.md

Methodology mirrors compute_v22_cell_deltas.py — prediction-disagreement
proxy for cell-2 (`sense NOT LIKE 'geography.%' AND ydf LIKE 'geography.%'`).
v22+v23 uses `sharpen_prediction` instead of `sense_prediction` so R26's
country_code promotions move the count.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

CORPUS_PASSES = [
    ("v19", REPO / "eval/gittables/corpus_pass/columns.parquet", "sense_prediction"),
    ("v20", REPO / "output/corpus-pass-v20/corpus_pass/columns.parquet", "sense_prediction"),
    ("v21", REPO / "output/corpus-pass-v21/corpus_pass/columns.parquet", "sense_prediction"),
    ("v22", REPO / "output/corpus-pass-v22/corpus_pass/columns.parquet", "sense_prediction"),
    ("v22+v23", REPO / "output/v23-sharpen-codes/columns_sharpened.parquet", "sharpen_prediction"),
]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--out",
                   type=Path,
                   default=REPO / "output/v23-sharpen-codes/cell_deltas.md")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:
        print(f"error: duckdb missing ({exc}).", file=sys.stderr)
        return 2

    con = duckdb.connect()
    metrics: dict[str, dict] = {}
    cell2_subtypes: dict[str, dict[str, int]] = {}

    for name, path, sense_col in CORPUS_PASSES:
        if not path.exists():
            print(f"warn: {name} corpus missing at {path} — skipping",
                  file=sys.stderr)
            continue
        try:
            con.execute(
                f"SELECT * FROM read_parquet('{path.as_posix()}') LIMIT 1"
            ).fetchone()
        except Exception as exc:  # noqa: BLE001
            print(f"warn: {name} parquet unreadable ({exc}); skipping",
                  file=sys.stderr)
            continue
        print(f"reading {name}: {path}  (using {sense_col})", file=sys.stderr)
        r = con.execute(f"""
            SELECT COUNT(DISTINCT file_path) AS files,
                   COUNT(*) FILTER (
                       WHERE ydf_prediction = 'geography.address.full_address'
                         AND ({sense_col} IS NULL
                              OR {sense_col} != 'geography.address.full_address')
                   ) AS cell1,
                   COUNT(*) FILTER (
                       WHERE {sense_col} NOT LIKE 'geography.%'
                         AND ydf_prediction LIKE 'geography.%'
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
        rows = con.execute(f"""
            SELECT ydf_prediction, COUNT(*) AS n
              FROM read_parquet('{path.as_posix()}')
             WHERE {sense_col} NOT LIKE 'geography.%'
               AND ydf_prediction LIKE 'geography.%'
             GROUP BY 1
             ORDER BY n DESC
        """).fetchall()
        cell2_subtypes[name] = dict(rows)
        print(f"  files={files:,}  cell2={cell2:,} ({cell2/files*1000:.2f}/1k)",
              file=sys.stderr)

    if "v19" not in metrics or "v22+v23" not in metrics:
        print("error: v19 and v22+v23 metrics required.", file=sys.stderr)
        return 2

    # ── Write markdown ────────────────────────────────────────────────
    lines: list[str] = []
    lines.append("# v22 + v23 (R26 Sharpen) corpus-pass cell deltas\n\n")
    lines.append("Per spec `2026-05-26-v23-sharpen-code-discriminator` ac-07.\n\n")
    lines.append(
        "Five-way comparison: v19 baseline, v20/v21/v22 retrains, and v22+v23 "
        "where v23's R26 country_code Sharpen rule is applied offline to "
        "v22's corpus pass (no fresh corpus pass — pure post-Sense post-processing).\n\n"
    )

    # Five-way table
    lines.append("## Five-way comparison (cell-2)\n\n")
    lines.append("| Variant | files | cell-2 | per-1k | Δ vs v19 |\n")
    lines.append("|---|---:|---:|---:|---:|\n")
    v19 = metrics["v19"]
    for name in ["v19", "v20", "v21", "v22", "v22+v23"]:
        if name not in metrics:
            continue
        m = metrics[name]
        if name == "v19":
            delta = "—"
        else:
            d = (m["cell2_per1k"] - v19["cell2_per1k"]) / v19["cell2_per1k"] * 100
            sign = "−" if d < 0 else "+"
            delta = f"{sign}{abs(d):.1f}%"
        bold = "**" if name == "v22+v23" else ""
        lines.append(
            f"| {bold}{name}{bold} | {m['files']:,} | {m['cell2']:,} | "
            f"{m['cell2_per1k']:.2f} | {bold}{delta}{bold} |\n"
        )
    lines.append("\n")

    # R26 incremental lift
    if "v22" in metrics and "v22+v23" in metrics:
        v22 = metrics["v22"]
        v22v23 = metrics["v22+v23"]
        r26_delta_abs = v22v23["cell2"] - v22["cell2"]
        r26_delta_pct = (v22v23["cell2_per1k"] - v22["cell2_per1k"]) / v22["cell2_per1k"] * 100
        lines.append(f"**R26 incremental lift vs v22-Sense-only:** "
                     f"{r26_delta_abs:+,} columns moved out of cell-2 "
                     f"({r26_delta_pct:+.2f}% relative).\n\n")

    # Per-subtype Cell 2 breakdown for v22 vs v22+v23 — the load-bearing
    # comparison for v23.
    if "v22" in cell2_subtypes and "v22+v23" in cell2_subtypes:
        lines.append("## Per-subtype cell-2 (v22 → v22+v23)\n\n")
        lines.append(
            "R26 only promotes to country_code; every other subtype should "
            "be unchanged unless a column whose Sense label changed happened "
            "to also flip the cell-2 inclusion check.\n\n"
        )
        lines.append("| Subtype | v22 misses | v22+v23 misses | Δ |\n")
        lines.append("|---|---:|---:|---:|\n")
        v22_st = cell2_subtypes["v22"]
        v23_st = cell2_subtypes["v22+v23"]
        all_subtypes = sorted(
            set(v22_st) | set(v23_st),
            key=lambda k: -max(v22_st.get(k, 0), v23_st.get(k, 0)),
        )
        for st in all_subtypes:
            n22 = v22_st.get(st, 0)
            n23 = v23_st.get(st, 0)
            if n22 == 0 and n23 == 0:
                continue
            d = n23 - n22
            short_st = st.replace("geography.", "")
            sign = "−" if d < 0 else ("+" if d > 0 else "")
            d_str = f"{sign}{abs(d)}" if d != 0 else "0"
            bold = "**" if "country_code" in st else ""
            lines.append(f"| {bold}{short_st}{bold} | {n22:,} | {n23:,} | {bold}{d_str}{bold} |\n")
        lines.append("\n")

    # ac-08 band classification
    if "v22+v23" in metrics:
        delta_v19 = ((metrics["v22+v23"]["cell2_per1k"]
                      - metrics["v19"]["cell2_per1k"])
                     / metrics["v19"]["cell2_per1k"] * 100)
        # R26-scoped bands (per the rescoped spec ac-08)
        r26_delta = (metrics["v22+v23"]["cell2_per1k"]
                     - metrics["v22"]["cell2_per1k"]) if "v22" in metrics else 0
        if r26_delta < 0:
            band = ("**Lift ≥ 0 (per AC-08 R26-scoped band)** — R26 fires "
                    "productively; country_code recovery works as designed.")
        elif r26_delta == 0:
            band = ("**Lift 0 pp** — R26 fires but in this corpus its "
                    "promotions don't move the cell-2 metric (all R26 "
                    "promotions were already in cell-2 by another mechanism).")
        else:
            band = ("**Lift < 0 (R26 made things worse)** — implementation "
                    "bug or anti-enum misconfiguration. Investigate.")
        lines.append("## ac-08 band — R26-scoped\n\n")
        lines.append(
            f"v22+v23 cell-2 rate: **{metrics['v22+v23']['cell2_per1k']:.2f} / 1000** "
            f"(vs v19 {metrics['v19']['cell2_per1k']:.2f}, Δ {delta_v19:+.1f}%).\n\n"
        )
        lines.append(f"Band: {band}\n")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
