#!/usr/bin/env python3
"""Run the B3 late-fusion model over a keyed set of columns and emit predictions.

Spec 2026-06-08-late-fusion-sense-classifier, ac-04 (pre-port kill switch). The fusion
head lives in Rust/Candle but is NOT yet wired into `finetype profile` (that is the
ac-05 port, deliberately gated behind this kill switch). So to score the head through
the corpus-honest gate and the gold anchor BEFORE committing to the port, we reproduce
the fusion forward pass out-of-band:

    finetype dump-fusion-features --key-col key   ->  <stem>.f32 + <stem>.keys.tsv
    train-fusion-head predict --head <dir>        ->  row_idx \t pred_label
    join on row_idx                               ->  key \t predicted_label

The input CSV must carry: key, final_label, sample_values (JSON array), column_name.
`final_label` is unused for prediction (kept only so the dump's row filter passes);
`key` is an opaque per-row identifier the caller splits downstream.

This is the SAME value substrate v19 saw — sample_values_truncated from the baseline
pass — so the comparison is apples-to-apples on identical inputs.
"""
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]


def run(cmd: list[str], env: dict | None = None) -> None:
    print("+", " ".join(str(c) for c in cmd), file=sys.stderr)
    r = subprocess.run(cmd, env=env)
    if r.returncode != 0:
        raise SystemExit(f"command failed (exit {r.returncode}): {cmd[0]}")


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--input", type=Path, required=True,
                    help="CSV with columns key,final_label,sample_values,column_name")
    ap.add_argument("--value-model", type=Path, default=REPO / "models/value-charcnn-v25")
    ap.add_argument("--mb-model", type=Path, default=REPO / "models/sherlock-v19-relu-s42")
    ap.add_argument("--head", type=Path, default=REPO / "models/fusion-head-v25")
    ap.add_argument("--out", type=Path, required=True,
                    help="output TSV: key\\tpredicted_label")
    ap.add_argument("--workdir", type=Path, default=REPO / "output/late-fusion/predict_tmp")
    ap.add_argument("--binary", type=Path, default=REPO / "target/release/finetype")
    ap.add_argument("--train-bin", type=Path,
                    default=REPO / "target/release/train-fusion-head")
    ap.add_argument("--sample-n", type=int, default=32)
    args = ap.parse_args()

    args.workdir.mkdir(parents=True, exist_ok=True)
    stem = args.workdir / "feats"

    run([str(args.binary), "dump-fusion-features",
         "--input", str(args.input),
         "--value-model", str(args.value_model),
         "--mb-model", str(args.mb_model),
         "--output", str(stem),
         "--sample-n", str(args.sample_n),
         "--key-col", "key"],
        env={"FINETYPE_MODEL": str(args.mb_model), "PATH": "/usr/bin:/bin"})

    preds_tsv = args.workdir / "preds.tsv"
    run([str(args.train_bin), "predict",
         "--head", str(args.head),
         "--features", str(stem),
         "--out", str(preds_tsv)])

    # join keys.tsv (row_idx\tkey) with preds.tsv (row_idx\tpred) on row_idx
    keys_path = args.workdir / "feats.keys.tsv"
    keys: dict[str, str] = {}
    with keys_path.open(encoding="utf-8") as f:
        next(f, None)  # header
        for line in f:
            ri, key = line.rstrip("\n").split("\t", 1)
            keys[ri] = key
    n = 0
    with preds_tsv.open(encoding="utf-8") as f, args.out.open("w", encoding="utf-8") as out:
        for line in f:
            ri, pred = line.rstrip("\n").split("\t", 1)
            key = keys.get(ri)
            if key is None:
                continue
            out.write(f"{key}\t{pred}\n")
            n += 1
    print(f"wrote {n} predictions -> {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
