#!/usr/bin/env python3
"""Rotating external-data advisory band (v0 — no network) — SCAFFOLD.

WHY THIS EXISTS
---------------
Gold is curated-*hard*. The representative band measures a random *GitTables*
column. Neither profiles a whole real-world external table end-to-end — and it
was exactly whole external tables (GLEIF, SEC EDGAR, NYC DOB, Chicago crimes)
that exposed the ticker / NAICS / org-name / WKT failures in the 2026-07
company-reference audit. This band is the standing instrument for that class of
miss: profile a real external table with the shipped binary, score the columns
we have adjudicated labels for, and triage the rest.

It is **ADVISORY, never blocking** (same status as the representative band). It
sits ALONGSIDE the representative band in the promotion order, between the gold
corpus accuracy read and the corpus-honest gate. No headline it prints may
override a blocking corpus-honest NO-GO.

WHAT IT SCORES (v0)
-------------------
Data: the 10 external tables already vendored on disk under
`eval/datasets/gold_external/` (the audit's own tables — no network needed).

Held labels: `held_labels_v0.tsv` in this directory, mechanically extracted from
the `external:*` rows of the canonical gold corpus (`eval/gold/gold_corpus.tsv`).
Those labels are ALREADY author/panel-signed (they are canonical gold), so v0
scores against ground truth that needs no fresh adjudication. 56 columns / 10
tables.

The band profiles the FULL table (all columns, sibling-context live), then:
  - joins predicted label to held label -> headline correct/total + per-type gaps
  - lists profiled columns with NO held label as an "unlabelled emission" triage
    block (candidate expansion; these DO NOT count toward the headline until
    adjudicated — see sign-off note in README.md).

ROTATION
--------
`--rotate K --seed N` deterministically draws K of the pool tables for the round
(re-draw each promotion round with a fresh seed; the full 10-table pool is the
sampling frame). Default: score all tables. Record the seed in the report so a
round is reproducible.

Output mirrors the representative band: an advisory headline and the
candidate-vs-baseline delta is the trustworthy signal (labels overlap gold, so
the ABSOLUTE is common-mode across candidates — read the delta).

USAGE
-----
  python3 run_external_band.py \
      --binary ../../../../target/release/finetype \
      --datasets-dir ../../../../eval/datasets/gold_external \
      --labels held_labels_v0.tsv \
      --out report.md

  # rotate a 4-table round:
  python3 run_external_band.py ... --rotate 4 --seed 20260711
"""
from __future__ import annotations

import argparse
import csv
import json
import random
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


def load_labels(path: Path) -> dict[tuple[str, str], str]:
    """(table, column) -> label."""
    out: dict[tuple[str, str], str] = {}
    with open(path, newline="") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            out[(r["table"], r["column_name"])] = r["label"]
    return out


def profile_table(binary: Path, csv_path: Path) -> dict[str, str]:
    """Profile a whole CSV -> {column_name: x-finetype-label}.

    Same Sense+Sharpen path score_gold_anchor.py uses, but on the full table so
    sibling-context attention sees the real neighbours (that context is part of
    what a real external table exercises)."""
    cmd = [str(binary), "profile", "-f", str(csv_path), "-o", "json-schema"]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=600)
    if proc.returncode != 0:
        raise RuntimeError(
            f"profile failed for {csv_path.name}: {proc.stderr.strip()[-300:]}")
    schema = json.loads(proc.stdout)
    props = schema.get("properties", {})
    return {
        col: (defn.get("x-finetype-label", "") if isinstance(defn, dict) else "")
        for col, defn in props.items()
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", required=True, type=Path,
                    help="path to the finetype release binary under test")
    ap.add_argument("--datasets-dir", required=True, type=Path,
                    help="dir holding the external CSVs (eval/datasets/gold_external)")
    ap.add_argument("--labels", default="held_labels_v0.tsv", type=Path)
    ap.add_argument("--rotate", type=int, default=0,
                    help="draw K pool tables this round (0 = all)")
    ap.add_argument("--seed", type=int, default=0,
                    help="rotation seed (record it in the report)")
    ap.add_argument("--out", type=Path, default=Path("report.md"))
    args = ap.parse_args()

    labels = load_labels(args.labels)
    tables = sorted({t for (t, _) in labels})

    if args.rotate and args.rotate < len(tables):
        rng = random.Random(args.seed)
        tables = sorted(rng.sample(tables, args.rotate))

    correct = 0
    scored = 0
    per_type_hit: dict[str, int] = defaultdict(int)
    per_type_tot: dict[str, int] = defaultdict(int)
    misses: list[tuple[str, str, str, str]] = []          # table,col,gold,pred
    unlabelled: list[tuple[str, str, str]] = []           # table,col,pred

    for t in tables:
        csv_path = args.datasets_dir / t
        if not csv_path.exists():
            print(f"WARN: missing {csv_path}", file=sys.stderr)
            continue
        preds = profile_table(args.binary, csv_path)
        for col, pred in preds.items():
            key = (t, col)
            if key in labels:
                gold = labels[key]
                per_type_tot[gold] += 1
                scored += 1
                if pred == gold:
                    correct += 1
                    per_type_hit[gold] += 1
                else:
                    misses.append((t, col, gold, pred))
            else:
                unlabelled.append((t, col, pred))

    headline = (correct / scored) if scored else 0.0

    lines: list[str] = []
    lines.append("# External-data advisory band — report")
    lines.append("")
    lines.append(f"**Status:** ADVISORY (never blocking). Read the candidate-vs-baseline "
                 f"delta, not the absolute — labels overlap gold, so the absolute is common-mode.")
    lines.append(f"**Rotation:** rotate={args.rotate or 'all'} seed={args.seed} "
                 f"tables={len(tables)}")
    lines.append(f"**Binary:** `{args.binary}`")
    lines.append("")
    lines.append(f"## Headline: {correct}/{scored} = {headline:.3f} "
                 f"({len(unlabelled)} unlabelled emissions triaged)")
    lines.append("")
    lines.append("## Per-type (adjudicated columns only)")
    lines.append("")
    lines.append("| label | correct | total | recall |")
    lines.append("|---|---|---|---|")
    for lab in sorted(per_type_tot):
        h, tot = per_type_hit[lab], per_type_tot[lab]
        lines.append(f"| {lab} | {h} | {tot} | {h/tot:.3f} |")
    lines.append("")
    lines.append("## Misses (adjudicated gold != predicted)")
    lines.append("")
    lines.append("| table | column | gold | predicted |")
    lines.append("|---|---|---|---|")
    for t, c, g, p in misses:
        lines.append(f"| {t} | {c} | {g} | {p} |")
    lines.append("")
    lines.append("## Unlabelled emissions (triage — NOT in the headline)")
    lines.append("")
    lines.append("These full-table columns have no adjudicated label yet. This is the "
                 "candidate-expansion queue: an over-emission here (e.g. a ticker read as "
                 "an NPI) is the failure class this band exists to surface. Adjudicate + "
                 "sign off before any of these counts toward a headline.")
    lines.append("")
    lines.append("| table | column | predicted |")
    lines.append("|---|---|---|")
    for t, c, p in unlabelled:
        lines.append(f"| {t} | {c} | {p} |")
    lines.append("")

    args.out.write_text("\n".join(lines))
    print(f"headline {correct}/{scored} = {headline:.3f}; "
          f"{len(misses)} misses, {len(unlabelled)} unlabelled -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
