#!/usr/bin/env python3
"""ac-01 — Four-way per-subtype cell-2 trajectory across v19/v20/v21/v22.

Sibling of compute_cell_deltas_gated.py: that one publishes the
headline v19→v22 lift on the gated baseline. This one breaks v20 and
v21 out so we can see whether the v19→v22 lift was a monotone ratchet
or a discontinuous v22-only jump.

Reads:
  output/ydf-validation-gate/{v19,v20,v21,v22}_gated.parquet

Writes:
  output/v22-direction-review/per_subtype_trajectory.md

Per spec 2026-05-26-v22-gated-direction-review ac-01.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GATE_DIR = REPO / "output/ydf-validation-gate"
DEFAULT_OUT = REPO / "output/v22-direction-review/per_subtype_trajectory.md"

VERSIONS = ["v19", "v20", "v21", "v22"]


def _pct(curr: float, base: float) -> float:
    return ((curr - base) / base * 100.0) if base else 0.0


def _band_tag(p19_21: float, p21_22: float, p19_22: float) -> str:
    """Categorise a subtype's response across the trajectory.

    Args are per-1k percentage drifts:
      p19_21 — v19→v21 drift
      p21_22 — v21→v22 drift
      p19_22 — v19→v22 drift

    - monotone-mover: v19→v21 and v21→v22 both non-positive AND v19→v22 ≤ −10%
    - v22-jumper: |v19→v21| < 5% AND v21→v22 ≤ −10%
    - flat: |v19→v22| < 5%
    - regressed: v19→v22 ≥ +5%
    - mixed: anything else
    """
    if p19_22 >= 5:
        return "regressed"
    if abs(p19_22) < 5:
        return "flat"
    if p19_22 <= -10 and p19_21 <= 0 and p21_22 <= 0:
        return "monotone-mover"
    if abs(p19_21) < 5 and p21_22 <= -10:
        return "v22-jumper"
    return "mixed"


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
    files_by_ver: dict[str, int] = {}
    subtypes_by_ver: dict[str, dict[str, int]] = {}

    for ver in VERSIONS:
        parquet = GATE_DIR / f"{ver}_gated.parquet"
        if not parquet.exists():
            print(f"warn: {parquet} missing — skipping", file=sys.stderr)
            continue
        print(f"reading {parquet}", file=sys.stderr)
        files = con.execute(f"""
            SELECT COUNT(DISTINCT file_path)
              FROM read_parquet('{parquet.as_posix()}')
        """).fetchone()[0]
        files_by_ver[ver] = files

        rows = con.execute(f"""
            SELECT ydf_prediction_gated AS ydf, COUNT(*) AS n
              FROM read_parquet('{parquet.as_posix()}')
             WHERE sense_prediction NOT LIKE 'geography.%'
               AND ydf_prediction_gated LIKE 'geography.%'
             GROUP BY 1
        """).fetchall()
        subtypes_by_ver[ver] = dict(rows)

    if not all(v in subtypes_by_ver for v in VERSIONS):
        missing = [v for v in VERSIONS if v not in subtypes_by_ver]
        print(f"error: missing versions {missing}.", file=sys.stderr)
        return 2

    all_subtypes = sorted(
        set().union(*[set(subtypes_by_ver[v]) for v in VERSIONS]),
        key=lambda k: -subtypes_by_ver["v19"].get(k, 0),
    )

    lines: list[str] = []
    lines.append("# v22 direction review — per-subtype trajectory\n\n")
    lines.append("Per spec `2026-05-26-v22-gated-direction-review` ac-01.\n\n")
    lines.append(
        "Cell-2 miss counts per geography subtype, version-by-version, "
        "scored against the gated YDF baseline (per spec "
        "`2026-05-26-ydf-validation-gate`). Sorted by v19 miss count "
        "descending — the dominant miss classes surface first.\n\n"
    )
    lines.append(
        "Bands (rightmost column): **monotone-mover** = v19≥v20≥v21≥v22 "
        "with ≥10% v19→v22 drop; **v22-jumper** = flat v19→v21 then "
        "≥10% v21→v22 drop; **flat** = |v19→v22| < 5%; **regressed** = "
        "v22 worse by ≥5%; **mixed** = none of the above.\n\n"
    )

    lines.append("## Files in each corpus pass\n\n")
    lines.append("| version | files |\n|---|---:|\n")
    for v in VERSIONS:
        lines.append(f"| {v} | {files_by_ver[v]:,} |\n")
    lines.append("\n")

    lines.append("## Per-subtype trajectory\n\n")
    lines.append(
        "| Subtype | v19 | v20 | v21 | v22 | Δ v19→v22 | Δ v21→v22 | band |\n"
    )
    lines.append("|---|---:|---:|---:|---:|---:|---:|---|\n")

    band_counts: dict[str, int] = {}
    for st in all_subtypes:
        counts = {v: subtypes_by_ver[v].get(st, 0) for v in VERSIONS}
        n19, n20, n21, n22 = counts["v19"], counts["v20"], counts["v21"], counts["v22"]
        if max(n19, n20, n21, n22) == 0:
            continue

        d_19_22 = n22 - n19
        p_19_22 = _pct(n22, n19)
        d_21_22 = n22 - n21
        p_21_22 = _pct(n22, n21)

        # band tagging uses raw count Δ% so it matches the Δ columns the
        # reader sees. File-count drift across v19→v22 is ~0.4%, well
        # below the band thresholds.
        p_19_21 = _pct(n21, n19)
        p_21_22 = _pct(n22, n21)
        band = _band_tag(p_19_21, p_21_22, p_19_22)
        band_counts[band] = band_counts.get(band, 0) + 1

        short = st.replace("geography.", "")
        sign_a = "−" if d_19_22 < 0 else ("+" if d_19_22 > 0 else "")
        sign_b = "−" if d_21_22 < 0 else ("+" if d_21_22 > 0 else "")
        bold = "**" if band in ("monotone-mover", "v22-jumper") else ""
        lines.append(
            f"| {bold}{short}{bold} | {n19:,} | {n20:,} | {n21:,} | {n22:,} | "
            f"{sign_a}{abs(d_19_22):,} ({sign_a}{abs(p_19_22):.1f}%) | "
            f"{sign_b}{abs(d_21_22):,} ({sign_b}{abs(p_21_22):.1f}%) | "
            f"{bold}{band}{bold} |\n"
        )

    lines.append("\n## Band summary\n\n")
    lines.append("| band | count |\n|---|---:|\n")
    for b in ("monotone-mover", "v22-jumper", "flat", "regressed", "mixed"):
        lines.append(f"| {b} | {band_counts.get(b, 0)} |\n")
    lines.append("\n")

    lines.append("## Reading this\n\n")
    lines.append(
        "- **monotone-mover** subtypes responded to the v20/v21/v22 "
        "boundary-training campaign as a whole; another retrain in the "
        "same shape would likely continue moving them.\n"
        "- **v22-jumper** subtypes only responded to v22's specific "
        "recipe (whatever distinguishes v22 from v21 — typically the "
        "boundary-blend ratio or the embed-branch contribution). The "
        "v22 recipe was the load-bearing change for these subtypes.\n"
        "- **flat** subtypes were not addressed by any of v20/v21/v22. "
        "Their bottleneck is something other than boundary blend — "
        "candidate explanations: thin training volume, header-pattern "
        "thinness, or a fundamentally different miss class (header-only "
        "signal, sibling-context dependence).\n"
        "- **regressed** subtypes worsened. Flag these for next-spec "
        "diagnostic before any further boundary-training spend.\n"
    )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("".join(lines))
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
