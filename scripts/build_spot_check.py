#!/usr/bin/env python3
"""ac-12 — build spot_check.md template for attestation.

Samples 3 gaps per non-empty (criterion × mechanism) cell from
corroborated_gaps.parquet using seed 20260520 (matches ac-02's seed,
pre-declared so the spot-checker cannot re-roll until a favourable
sample appears). For each sampled gap, writes the full evidence the
attestor needs:

  - gap_id, affected_column_count, mechanism, recommended_action_class
  - corroborating_lenses (YDF + cascade verdicts)
  - sample_evidence rows enriched with DBpedia annotations

Plus a verdict placeholder the attestor flips PASS/FAIL with optional
reason, and a per-cell summary table to be filled at attestation time.

USAGE
    python3 scripts/build_spot_check.py
"""
from __future__ import annotations

import argparse
import json
import random
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SAMPLING_SEED = 20260520
SAMPLES_PER_CELL = 3
PASS_RATE_THRESHOLD = 0.90


def fmt_samples(values: list[str], max_n: int = 5, max_len: int = 60) -> str:
    show = (values or [])[:max_n]
    out = []
    for s in show:
        s = (s or "").replace("\n", " ").replace("\r", " ").strip()
        if len(s) > max_len:
            s = s[: max_len - 1] + "…"
        s = s.replace("`", "")
        out.append(f"`{s}`")
    return " ".join(out)


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--corroborated", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet")
    p.add_argument("--dbpedia-annotations", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/dbpedia_annotations.parquet")
    p.add_argument("--out", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/spot_check.md")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: duckdb missing ({exc})", file=sys.stderr)
        return 2

    con = duckdb.connect()
    print("loading corroborated_gaps...", file=sys.stderr)
    gaps = con.execute(f"""
        SELECT *
        FROM read_parquet('{args.corroborated}')
        ORDER BY criterion, mechanism, gap_id
    """).fetch_arrow_table().to_pylist()

    # DBpedia lookup for evidence enrichment
    print("loading dbpedia annotations...", file=sys.stderr)
    dbp_rows = con.execute(f"""
        SELECT file_path, column_name, dbpedia_semantic_class
        FROM read_parquet('{args.dbpedia_annotations}')
        WHERE dbpedia_semantic_class IS NOT NULL
              AND dbpedia_semantic_class != ''
    """).fetch_arrow_table().to_pylist()
    dbp_lookup: dict[tuple[str, str], str] = {
        (r["file_path"], r["column_name"]): r["dbpedia_semantic_class"]
        for r in dbp_rows
    }

    # Group by cell
    cells: dict[tuple[str, str], list[dict]] = defaultdict(list)
    for g in gaps:
        cells[(g["criterion"], g["mechanism"])].append(g)

    # Deterministic sampling per cell
    sampled_per_cell: dict[tuple[str, str], list[dict]] = {}
    for cell_key, cell_gaps in sorted(cells.items()):
        # Stable ordering before sampling — gap_id is unique + sorted above
        rng = random.Random(SAMPLING_SEED)
        if len(cell_gaps) <= SAMPLES_PER_CELL:
            sampled_per_cell[cell_key] = list(cell_gaps)
        else:
            sampled_per_cell[cell_key] = rng.sample(cell_gaps, SAMPLES_PER_CELL)

    # ── Compose spot_check.md ─────────────────────────────────────────
    out: list[str] = []
    out.append("---")
    out.append("attestor: @hughcameron")
    out.append("attest_date: TBD")
    out.append(f"sampling_seed: {SAMPLING_SEED}")
    out.append(f"samples_per_cell: {SAMPLES_PER_CELL}")
    out.append(f"pass_rate_threshold: {PASS_RATE_THRESHOLD}")
    out.append("---")
    out.append("")
    out.append("# ac-12 — Per-cell spot-check on Part 1 (corroborated gaps)")
    out.append("")
    out.append(
        "For each gap below, flip the verdict line to **PASS** or "
        "**FAIL**. A gap passes only if **all three** of the following "
        "hold:"
    )
    out.append("")
    out.append(
        "1. **(a) Mechanism fits the evidence.** The sample_evidence "
        "rows exhibit the column failure pattern that the assigned "
        "`mechanism_token` describes per MADRs 0075 / 0081."
    )
    out.append(
        "2. **(b) Lenses genuinely disagree with Sense.** Both YDF and "
        "the cascade independently point at a different answer than "
        "Sense's prediction (not spurious — i.e. not e.g. both lenses "
        "happen to be wrong in the same direction by coincidence)."
    )
    out.append(
        "3. **(c) Token is correct.** The assigned `mechanism_token` "
        "is the closest-fitting one from the closed 10-token set."
    )
    out.append("")
    out.append(
        "Partial failures (any one of 3 conditions failing) count as "
        "full failures. Per-cell threshold: pass_rate ≥ "
        f"{PASS_RATE_THRESHOLD}. With 3 samples per cell, that means "
        "all 3 must pass."
    )
    out.append("")
    out.append(
        "**Failure consequence (per spec):** if a cell's pass rate "
        "falls below threshold, all gaps in that cell are demoted to "
        "`single_lens_signals.tsv` and the demotion is logged in "
        "`progress.md`."
    )
    out.append("")

    # Per-cell sections
    summary_rows: list[tuple[str, str, int, str]] = []
    for cell_key in sorted(sampled_per_cell):
        criterion, mechanism = cell_key
        samples = sampled_per_cell[cell_key]
        out.append(f"## Cell: `{criterion}` × `{mechanism}`")
        out.append("")
        out.append(f"Sampled {len(samples)} of {len(cells[cell_key])} "
                   f"gaps in this cell.")
        out.append("")
        summary_rows.append(
            (criterion, mechanism, len(samples), "TBD")
        )

        for i, g in enumerate(samples, start=1):
            gid = g["gap_id"]
            affected = g["affected_column_count"]
            action = g["recommended_action_class"]
            lenses = g["corroborating_lenses"] or []
            lens_lines = []
            for l in lenses:
                lens_lines.append(
                    f"- **{l['lens_name']}**: "
                    f"`{l['prediction_or_annotation']}` "
                    f"(conf {l['confidence']:.2f})"
                )

            out.append(f"### Sample {i} — `{gid[:12]}…`")
            out.append("")
            out.append(f"- **gap_id**: `{gid}`")
            out.append(f"- **affected_column_count**: {affected}")
            out.append(f"- **recommended_action_class**: `{action}`")
            out.append("- **corroborating_lenses**:")
            for line in lens_lines:
                out.append(f"  {line}")
            out.append("")
            out.append("**Sample evidence**:")
            out.append("")
            out.append("| file | column | sense | ydf | samples | dbpedia |")
            out.append("|---|---|---|---|---|---|")
            for s in (g["sample_evidence"] or [])[:5]:
                fp = s["file_path"]
                cn = s["column_name"]
                sense = s.get("sense_prediction") or ""
                ydf = s.get("ydf_prediction") or ""
                samples_md = fmt_samples(s.get("sample_values") or [])
                dbp = dbp_lookup.get((fp, cn), "")
                parts = fp.split("/")
                short_fp = "/".join(parts[-2:]) if len(parts) >= 2 else fp
                out.append(
                    f"| `{short_fp}` | `{cn}` | `{sense}` | `{ydf}` "
                    f"| {samples_md} | `{dbp}` |"
                )
            out.append("")
            out.append("**Verdict**: ☐ PASS  ☐ FAIL")
            out.append("")
            out.append("**Reason (if FAIL)**: _to be filled by attestor_")
            out.append("")

    # Per-cell summary
    out.append("---")
    out.append("")
    out.append("## Per-cell summary")
    out.append("")
    out.append("| criterion | mechanism | sampled | passed | pass_rate | meets threshold? |")
    out.append("|---|---|---:|---:|---:|---|")
    for criterion, mechanism, sampled, _ in summary_rows:
        out.append(
            f"| `{criterion}` | `{mechanism}` | {sampled} | TBD | TBD | TBD |"
        )
    out.append("")
    out.append(
        f"**Spec close blocks** until every cell either passes "
        f"({PASS_RATE_THRESHOLD} threshold) OR every failing cell is "
        "demoted per the spec's failure-consequence procedure."
    )
    out.append("")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(out))

    n_total_samples = sum(len(s) for s in sampled_per_cell.values())
    print(json.dumps({
        "n_non_empty_cells": len(sampled_per_cell),
        "n_total_samples": n_total_samples,
        "sampling_seed": SAMPLING_SEED,
        "output": str(args.out),
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
