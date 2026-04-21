#!/usr/bin/env python3
"""Compute pre→post regressions for the sharpen demotion guard.

A regression = (dataset, column) where pre predicted == gt and post != gt.
Mirrors the label-match logic of scripts/eval_delta_by_coverage.py:
match iff predicted ∈ {full_gt, bare_gt} (schema_mapping expansion).

Emits:
  - regressions.tsv    (dataset, column, gt, pre, post)
  - improvements.tsv   (dataset, column, gt, pre, post) — post correct, pre wrong
  - changes.tsv        (dataset, column, gt, pre, post) — any label change
  - summary.txt        (regressions count, improvements count, total changes)
"""
from __future__ import annotations
import csv
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent.parent.parent  # specs/<name>/eval-artefacts -> repo
MANIFEST = REPO / "eval" / "datasets" / "manifest.csv"
SCHEMA_MAPPING = REPO / "eval" / "schema_mapping.yaml"
PRE = HERE / "profile_results.pre.csv"
POST = HERE / "profile_results.post.csv"


def load_schema_map() -> dict[str, str]:
    import yaml  # type: ignore[import-untyped]
    with open(SCHEMA_MAPPING) as fh:
        data = yaml.safe_load(fh)
    out: dict[str, str] = {}
    for entry in data.get("mappings", []):
        gt = entry.get("gt_label")
        full = entry.get("finetype_label")
        if gt and full:
            out[gt] = full
    return out


def load_predictions(path: Path) -> dict[tuple[str, str], str]:
    out: dict[tuple[str, str], str] = {}
    with open(path, newline="") as fh:
        for row in csv.DictReader(fh):
            out[(row["dataset"], row["column_name"])] = row.get("predicted_type", "")
    return out


def main() -> int:
    schema = load_schema_map()
    pre = load_predictions(PRE)
    post = load_predictions(POST)

    def is_match(predicted: str, gt: str) -> bool:
        full = schema.get(gt, gt)
        return predicted == full or predicted == gt

    regressions: list[dict] = []
    improvements: list[dict] = []
    changes: list[dict] = []

    with open(MANIFEST, newline="") as fh:
        for row in csv.DictReader(fh):
            key = (row["dataset"], row["column_name"])
            gt = row["gt_label"]
            p = pre.get(key)
            q = post.get(key)
            if p is None or q is None:
                continue
            if p != q:
                changes.append({"dataset": key[0], "column": key[1], "gt": gt, "pre": p, "post": q})
                pre_ok = is_match(p, gt)
                post_ok = is_match(q, gt)
                if pre_ok and not post_ok:
                    regressions.append({"dataset": key[0], "column": key[1], "gt": gt, "pre": p, "post": q})
                elif post_ok and not pre_ok:
                    improvements.append({"dataset": key[0], "column": key[1], "gt": gt, "pre": p, "post": q})

    def write_tsv(path: Path, rows: list[dict]) -> None:
        with open(path, "w") as fh:
            fh.write("dataset\tcolumn\tgt\tpre\tpost\n")
            for r in rows:
                fh.write(f"{r['dataset']}\t{r['column']}\t{r['gt']}\t{r['pre']}\t{r['post']}\n")

    write_tsv(HERE / "regressions.tsv", regressions)
    write_tsv(HERE / "improvements.tsv", improvements)
    write_tsv(HERE / "changes.tsv", changes)

    summary = (
        f"regressions: {len(regressions)}\n"
        f"improvements: {len(improvements)}\n"
        f"neutral_changes: {len(changes) - len(regressions) - len(improvements)}\n"
        f"total_changes: {len(changes)}\n"
    )
    (HERE / "summary.txt").write_text(summary)
    print(summary, end="")
    return 0 if len(regressions) == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
