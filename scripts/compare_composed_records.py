#!/usr/bin/env python3
"""Diff two `finetype resharpen` passes over the WHOLE composed record.

`resharpen` emits `id<TAB>label<TAB>confidence<TAB>quality_band<TAB>runner_up<TAB>rule`
— every field of the profile record that Sharpen can move. This script compares
two such passes column by column and reports how many columns changed, split by
which field moved.

Why it exists. A Sharpen rule was once certified inert on the grounds that the
composed prediction was "byte-identical across 837,625 columns". The comparison
read the LABEL and nothing else. The rule in question was in fact collapsing
confidence from 0.877 to 0.500, dropping the quality band from `high` to `low`,
surfacing a runner-up, and rewriting the disambiguation rule — all of which
`finetype profile -o json` prints, and none of which a label-only diff can see.
An inert-rule claim has to be made over the record the user is shown.

`detected_locale` is not part of the record `resharpen` emits and so is not
compared: it is not run-to-run stable on the same binary. Any locale claim needs
that fixed first.

Usage:
  compare_composed_records.py BASELINE.tsv CANDIDATE.tsv [--label NAME] [--top N]
Exit status is 0 whatever the result; this reports, it does not gate.
"""
from __future__ import annotations

import argparse
import collections
import sys

FIELDS = ("label", "confidence", "quality_band", "runner_up", "disambiguation_rule")


def load(path: str) -> dict[str, tuple[str, str, str, str, str]]:
    """Read a resharpen TSV into {id: (label, conf, band, runner_up, rule)}.

    Short lines are tolerated (older passes emitted fewer fields) by padding with
    the empty string, so a format change shows up as a diff rather than a crash.
    """
    rows: dict[str, tuple[str, str, str, str, str]] = {}
    with open(path, encoding="utf-8", errors="replace") as fh:
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) < 2:
                continue
            key = parts[0]
            rest = parts[1:6]
            while len(rest) < 5:
                rest.append("")
            rows[key] = (rest[0], rest[1], rest[2], rest[3], rest[4])
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("baseline")
    ap.add_argument("candidate")
    ap.add_argument("--label", default="candidate", help="name for the report header")
    ap.add_argument("--top", type=int, default=10, help="how many transitions to list")
    a = ap.parse_args()

    base = load(a.baseline)
    cand = load(a.candidate)
    shared = base.keys() & cand.keys()

    per_field: collections.Counter[str] = collections.Counter()
    label_moves: collections.Counter[str] = collections.Counter()
    rule_moves: collections.Counter[str] = collections.Counter()
    band_moves: collections.Counter[str] = collections.Counter()
    changed = 0

    for key in shared:
        b = base[key]
        c = cand[key]
        if b == c:
            continue
        changed += 1
        for i, name in enumerate(FIELDS):
            if b[i] != c[i]:
                per_field[name] += 1
        if b[0] != c[0]:
            label_moves[f"{b[0]} -> {c[0]}"] += 1
        if b[4] != c[4]:
            rule_moves[f"{b[4] or '(none)'} -> {c[4] or '(none)'}"] += 1
        if b[2] != c[2]:
            band_moves[f"{b[2]} -> {c[2]}"] += 1

    print(f"── {a.label} ──")
    print(f"  columns compared        {len(shared)}")
    print(f"  baseline-only ids       {len(base.keys() - cand.keys())}")
    print(f"  candidate-only ids      {len(cand.keys() - base.keys())}")
    print(f"  columns with ANY change {changed}")
    for name in FIELDS:
        print(f"    {name:<22} {per_field[name]}")
    if changed == 0:
        print("  INERT over the whole emitted record")
        return 0
    for title, counter in (
        ("label transitions", label_moves),
        ("quality_band transitions", band_moves),
        ("disambiguation_rule transitions", rule_moves),
    ):
        if not counter:
            continue
        print(f"  top {title}:")
        for move, n in counter.most_common(a.top):
            print(f"    {n:>7}  {move}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
