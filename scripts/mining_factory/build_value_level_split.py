#!/usr/bin/env python3
"""Value-level architecture test — build the train/test split.

The multi-branch additive blend collapses its numeric prior on manufactured
reference data (0-for-4, dose-independent; see ac05_proxy_precheck.md / ac07 memo).
The pivot: a VALUE-LEVEL CharCNN classifies one value at a time with no column-level
sibling entanglement, so that collapse mechanism cannot apply by construction. The
prior value-level spec (2026-06-04) closed INCONCLUSIVE for one reason — cleaned real
GitTables starves the confusion family (its capped set carries latitude=10 distinct,
longitude=24). The manufactured corpus removes that blocker.

This builds a controlled two-arm split over a SHARED test set:
  - pool per type = cleaned_capped rows (real GitTables, source=clean)
                  + manufactured firewalled values (source=mfg), de-duped by text.
  - per type, seed-42 shuffle the DISTINCT values and split 85/15 train/test
    (split by value, so zero train/test value leakage).
  - Arm B (manufactured)   train = train split, ALL sources, capped 10k/type.
  - Arm A (starved baseline) train = train split, source=clean ONLY, capped 10k/type.
  - shared test = test split, capped --test-cap/type, carries the gold label + source.

Both arms train with the identical recipe and are scored on the identical test set;
the ONLY difference is whether manufactured diversity is present in training. That
isolates the architecture question: can the value-level model learn the starved
confusion family (latitude) from manufactured diversity WITHOUT collapsing the
plain-number classes — the boundary the multi-branch model could not hold.

Outputs (output/value-level-arch-test/):
  value_train_mfg.ndjson    {classification, text}   Arm B training
  value_train_base.ndjson   {classification, text}   Arm A training (control)
  value_test.ndjson         {classification, text, source}   shared eval
  split_manifest.json       per-type train/test/source counts + confusion-family table
"""
from __future__ import annotations

import argparse
import json
import random
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OUT_DIR = REPO / "output" / "value-level-arch-test"

CONFUSION_FAMILY = [
    "geography.coordinate.latitude", "geography.coordinate.longitude",
    "representation.numeric.decimal_number", "representation.numeric.integer_number",
    "geography.location.city", "geography.location.region",
    "geography.address.postal_code",
]


def load_cleaned(path: Path) -> dict[str, set[str]]:
    by_type: dict[str, set[str]] = defaultdict(set)
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            t = r.get("classification")
            v = r.get("text")
            if t and v is not None:
                by_type[t].add(v)
    return by_type


def load_manufactured(path: Path) -> dict[str, set[str]]:
    by_type: dict[str, set[str]] = defaultdict(set)
    with path.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            t = r.get("type")
            v = r.get("value")
            if t and v is not None:
                by_type[t].add(v)
    return by_type


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--cleaned", type=Path,
                    default=REPO / "output/value-level-labelling/cleaned_capped.ndjson")
    ap.add_argument("--manufactured", type=Path,
                    default=REPO / "output/mining-factory/firewalled_values.ndjson")
    ap.add_argument("--train-cap", type=int, default=10000)
    ap.add_argument("--test-cap", type=int, default=1500)
    ap.add_argument("--test-frac", type=float, default=0.15)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    rng = random.Random(args.seed)

    clean = load_cleaned(args.cleaned)
    mfg = load_manufactured(args.manufactured)
    all_types = sorted(set(clean) | set(mfg))

    f_mfg = (OUT_DIR / "value_train_mfg.ndjson").open("w", encoding="utf-8")
    f_base = (OUT_DIR / "value_train_base.ndjson").open("w", encoding="utf-8")
    f_test = (OUT_DIR / "value_test.ndjson").open("w", encoding="utf-8")

    manifest: dict[str, dict] = {}
    for t in all_types:
        clean_vals = clean.get(t, set())
        mfg_vals = mfg.get(t, set())
        # source map: a value is 'mfg' only if NOT already in clean (clean wins label).
        pool = {}
        for v in clean_vals:
            pool[v] = "clean"
        for v in mfg_vals:
            pool.setdefault(v, "mfg")
        values = sorted(pool)
        rng.shuffle(values)
        n_test = max(1, int(len(values) * args.test_frac)) if len(values) >= 4 else 0
        test_vals = values[:n_test]
        train_vals = values[n_test:]

        # Arm B: all sources, capped.
        b_train = train_vals[:args.train_cap]
        # Arm A: clean-source only, capped.
        a_train = [v for v in train_vals if pool[v] == "clean"][:args.train_cap]
        # Test: capped.
        te = test_vals[:args.test_cap]

        for v in b_train:
            f_mfg.write(json.dumps({"classification": t, "text": v}, ensure_ascii=False) + "\n")
        for v in a_train:
            f_base.write(json.dumps({"classification": t, "text": v}, ensure_ascii=False) + "\n")
        for v in te:
            f_test.write(json.dumps({"classification": t, "text": v, "source": pool[v]},
                                    ensure_ascii=False) + "\n")

        manifest[t] = {
            "pool": len(values), "clean": len(clean_vals), "mfg_extra": len(values) - len(clean_vals),
            "arm_b_train": len(b_train), "arm_a_train": len(a_train), "test": len(te),
            "test_mfg_source": sum(1 for v in te if pool[v] == "mfg"),
        }

    for fh in (f_mfg, f_base, f_test):
        fh.close()

    (OUT_DIR / "split_manifest.json").write_text(
        json.dumps({"seed": args.seed, "train_cap": args.train_cap,
                    "test_cap": args.test_cap, "test_frac": args.test_frac,
                    "n_types": len(all_types), "per_type": manifest}, indent=2) + "\n",
        encoding="utf-8")

    # Console summary — confusion family is the load-bearing read.
    print(f"types: {len(all_types)}")
    print(f"{'type':45s} {'armA(base)':>11s} {'armB(mfg)':>10s} {'test':>6s} {'test_mfg':>9s}")
    for t in CONFUSION_FAMILY:
        m = manifest.get(t)
        if m:
            print(f"{t:45s} {m['arm_a_train']:>11,} {m['arm_b_train']:>10,} "
                  f"{m['test']:>6,} {m['test_mfg_source']:>9,}")
    tot_a = sum(m["arm_a_train"] for m in manifest.values())
    tot_b = sum(m["arm_b_train"] for m in manifest.values())
    tot_t = sum(m["test"] for m in manifest.values())
    print(f"\ntotals  armA={tot_a:,}  armB={tot_b:,}  test={tot_t:,}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
