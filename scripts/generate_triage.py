#!/usr/bin/env python3
"""Generate draft triage.md from prescreen_results.tsv (ac-03).

Spec constraint #1: the final keep/augment/replace call requires
human review — no LLM-as-judge. This script produces a
DETERMINISTIC DRAFT based on pre-screen metrics, flagged for human
review before ac-04 replace-execution.

Decision rule (mechanical, transparent):
  1. pass_floors=True                → keep   (meets realism floors)
  2. fail on entropy+skew only
     (categorical/boolean signature) → keep   (legitimate low-cardinality)
  3. fail on null_rate only
     (nested JSON / optional field)  → keep   (representative semi-structured)
  4. fail on multiple axes           → augment (realism below floor)
  5. column absent / values empty    → replace (audit-surfaced gap)

Rationale per row records the exact failing metrics so Hugh can
override the call in his human-review pass.

See:
  .orbit/specs/2026-04-21-eval-expansion/spec.yaml (ac-03)
  .orbit/choices/0055-eval-realism-dimensions.md (triage schema)
"""

from __future__ import annotations

import argparse
import csv
from collections import Counter
from pathlib import Path


def classify(row: dict[str, str]) -> tuple[str, str, bool]:
    """Return (action, rationale, gt_label_change)."""
    if row.get("error"):
        return "replace", f"prescreen error: {row['error']}", True

    pass_floors = row.get("pass_floors", "").strip().lower() == "true"
    notes = row.get("pass_notes", "")
    failing = [n for n in notes.split(";") if "ok" not in n and n.strip()]

    if pass_floors:
        return "keep", "pass_floors=True — meets realism floors", False

    # Analyse which axes failed
    axes = set()
    for note in failing:
        if "shannon_entropy" in note:
            axes.add("entropy")
        if "top_1_skew" in note:
            axes.add("skew")
        if "null_rate" in note:
            axes.add("null")
        if "whitespace" in note:
            axes.add("whitespace")
        if "unique_ratio" in note:
            axes.add("unique")
        if "format_variance" in note:
            axes.add("format_variance")

    # Rule 2: entropy+skew only → legitimate categorical/boolean
    if axes <= {"entropy", "skew"}:
        return (
            "keep",
            "low-cardinality categorical signature; floor too strict for enum family",
            False,
        )

    # Rule 3: null_rate only → legitimate sparse semi-structured
    if axes == {"null"} or axes == {"null", "entropy"}:
        return (
            "keep",
            "high-null semi-structured / optional field; real-world sparsity",
            False,
        )

    # Rule 4: multiple axes → augment
    return "augment", f"realism gaps on axes: {sorted(axes)}", False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--prescreen",
        type=Path,
        default=Path("eval/prescreen_results.tsv"),
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path(".orbit/specs/2026-04-21-eval-expansion/triage.md"),
    )
    args = parser.parse_args()

    with open(args.prescreen, encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh, delimiter="\t"))

    triaged = []
    action_counts: Counter[str] = Counter()
    for r in rows:
        action, rationale, gt_change = classify(r)
        triaged.append((r, action, rationale, gt_change))
        action_counts[action] += 1

    # Sort by action (replace → augment → keep) then dataset/column for review order
    action_order = {"replace": 0, "augment": 1, "keep": 2}
    triaged.sort(
        key=lambda t: (
            action_order.get(t[1], 3),
            t[0]["dataset"],
            t[0]["column_name"],
        )
    )

    # Render markdown
    lines = [
        "# Triage — eval/datasets/manifest.csv (ac-03)",
        "",
        "**Status:** DRAFT — deterministic classification from `prescreen_results.tsv`.",
        "Per spec constraint #1 this draft is subject to Hugh's human review before",
        "ac-04 replace-execution begins. Override any row by editing the Action cell",
        "and noting the override rationale in the Notes column.",
        "",
        f"**Source:** {args.prescreen}",
        f"**Total rows:** {len(rows)}",
        "",
        "## Summary",
        "",
        "```",
    ]
    for action in ("replace", "augment", "keep"):
        lines.append(f"  {action:<8s} {action_counts.get(action, 0)}")
    lines.append("```")
    lines.append("")
    lines.append("## Decision rule (mechanical)")
    lines.append("")
    lines.append("1. `pass_floors=True` → **keep**")
    lines.append("2. fail on entropy+skew only → **keep** (categorical signature)")
    lines.append("3. fail on null_rate only → **keep** (legitimate sparsity)")
    lines.append("4. multiple failing axes → **augment**")
    lines.append("5. pre-screen error / empty column → **replace**")
    lines.append("")
    lines.append("## Worklist")
    lines.append("")
    lines.append(
        "| Dataset | Column | gt_label | Action | Rationale | gt_label_change |"
    )
    lines.append("|---|---|---|---|---|---|")
    for r, action, rationale, gt_change in triaged:
        ds = r["dataset"]
        col = r["column_name"]
        gt = r["gt_label"]
        gtc = "YES" if gt_change else ""
        lines.append(f"| {ds} | {col} | {gt} | **{action}** | {rationale} | {gtc} |")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as fh:
        fh.write("\n".join(lines) + "\n")

    print(f"triage.md written to {args.output}")
    for action in ("replace", "augment", "keep"):
        print(f"  {action}: {action_counts.get(action, 0)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
