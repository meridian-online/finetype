#!/usr/bin/env python3
"""Leakage firewall over the labelled distilled corpus, ahead of the fusion dump.

Spec 2026-06-08-late-fusion-sense-classifier, ac-02. The fusion head trains on
features dumped from a LABELLED corpus (Sherlock distilled), while every eval
instrument (corpus-honest gate, gold anchor) is GitTables-sourced — so the two
are provenance-disjoint and leakage is structurally near-impossible. This script
adds the canonical value-level belt-and-braces: it drops any distilled
(header, value) whose row_hash collides with eval/row_hashes.tsv, using the
SAME normaliser the rest of the train<->eval firewall uses
(scripts/eval_leakage, MADR 0056). It emits a PLAIN, firewalled CSV that
`finetype dump-fusion-features` reads (the Rust dump has no gz/decompress path).

Note on the header-less v3 corpus: distillation-v3 carries no column header, so
the (header, value) hash degenerates to row_hash("", value); the eval hashes are
header-keyed, so overlap is expected to be ~0. That is correct — the binding
separation here is provenance, and this pass proves zero value-level overlap on
top of it.

Usage:
    python3 scripts/fusion/firewall_distilled.py \
        --distilled output/distillation-v3/sherlock_distilled.csv.gz \
        --out output/late-fusion/distilled_firewalled.csv
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
from eval_leakage import row_hash  # noqa: E402

csv.field_size_limit(1 << 24)


def load_eval_hashes(path: Path) -> set[str]:
    hashes: set[str] = set()
    with path.open(encoding="utf-8") as f:
        for i, line in enumerate(f):
            if line.startswith("#"):
                continue
            parts = line.rstrip("\n").split("\t")
            if i == 0 or len(parts) < 4 or parts[-1] == "row_hash":
                continue
            hashes.add(parts[-1])
    return hashes


def open_maybe_gz(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8", newline="")
    return path.open(encoding="utf-8", newline="")


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--distilled", type=Path,
                    default=REPO / "output/distillation-v3/sherlock_distilled.csv.gz")
    ap.add_argument("--row-hashes", type=Path, default=REPO / "eval/row_hashes.tsv")
    ap.add_argument("--out", type=Path,
                    default=REPO / "output/late-fusion/distilled_firewalled.csv")
    ap.add_argument("--label-col", default="final_label")
    ap.add_argument("--min-values", type=int, default=3)
    args = ap.parse_args()

    args.out.parent.mkdir(parents=True, exist_ok=True)
    eval_hashes = load_eval_hashes(args.row_hashes)
    print(f"eval row_hashes loaded: {len(eval_hashes):,}")

    n_cols = n_kept = n_dropped_few = 0
    vals_total = vals_dropped = 0
    cols_touched = 0  # columns that lost >=1 value to the firewall

    with open_maybe_gz(args.distilled) as fin, \
            args.out.open("w", encoding="utf-8", newline="") as fout:
        reader = csv.DictReader(fin)
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])
        for row in reader:
            n_cols += 1
            label = (row.get(args.label_col) or "").strip()
            if not label:
                continue
            header = (row.get("column_name") or "").strip()
            try:
                vals = json.loads(row.get("sample_values") or "[]")
            except json.JSONDecodeError:
                continue
            vals = [str(v) for v in vals]
            vals_total += len(vals)
            kept = [v for v in vals if row_hash(header, v) not in eval_hashes]
            d = len(vals) - len(kept)
            if d:
                cols_touched += 1
                vals_dropped += d
            if len(kept) < args.min_values:
                n_dropped_few += 1
                continue
            writer.writerow([label, json.dumps(kept, ensure_ascii=False), header])
            n_kept += 1

    print(f"columns read:           {n_cols:,}")
    print(f"columns kept:           {n_kept:,}")
    print(f"columns dropped (<{args.min_values} vals left): {n_dropped_few:,}")
    print(f"values examined:        {vals_total:,}")
    print(f"values dropped (eval overlap): {vals_dropped:,}")
    print(f"columns touched by firewall:   {cols_touched:,}")
    print(f"wrote {args.out}")
    if vals_dropped == 0:
        print("FIREWALL: zero value-level overlap (provenance-disjoint, as expected).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
