#!/usr/bin/env python3
"""ac-01: bootstrap the value-level YDF prior on synthetic per-type values.

Pipeline:
  1. `finetype generate` -> synthetic {classification, text} per type.
  2. Export the 277-dim per-value feature contract (ac-00 bin).
  3. Leakage firewall: drop synthetic values colliding with the eval holdout.
  4. Train a YDF RandomForest (label = classification, features = the 277 dims).
  5. Save model + feature manifest under eval/gittables/models/ydf_value*, and a
     report with per-type support and held-out synthetic accuracy.

Run from the eval venv:
  eval/gittables/.venv/bin/python scripts/value_ydf/ac01_train_prior.py
"""
from __future__ import annotations

import argparse
import json
import subprocess

import pandas as pd
import ydf

import common as C
from eval_leakage import normalise_value


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--samples", type=int, default=200, help="synthetic values per type")
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--num-trees", type=int, default=300)
    ap.add_argument("--holdout", type=float, default=0.2)
    args = ap.parse_args()

    C.OUT_DIR.mkdir(parents=True, exist_ok=True)
    C.MODELS_DIR.mkdir(parents=True, exist_ok=True)

    # 1. Generate synthetic per-type values.
    gen_ndjson = C.OUT_DIR / "synthetic_values.ndjson"
    print(f"[1/5] generating synthetic values (samples={args.samples}, seed={args.seed})")
    subprocess.run(
        [str(C.FINETYPE_BIN), "generate", "--samples", str(args.samples),
         "--seed", str(args.seed), "--output", str(gen_ndjson)],
        check=True, cwd=str(C.REPO),
    )

    # 2. Export the 277-dim feature contract.
    feat_csv = C.OUT_DIR / "synthetic_features.csv"
    manifest = C.MODELS_DIR / "ydf_value_manifest.txt"
    print("[2/5] exporting per-value features (277-dim contract)")
    C.export_features(gen_ndjson, feat_csv, manifest)

    df = pd.read_csv(feat_csv, dtype={"classification": str, "text": str}, keep_default_na=False)
    n_total = len(df)
    feat_cols = C.feature_columns(df.columns)
    assert len(feat_cols) == 277, f"expected 277 feature cols, got {len(feat_cols)}"

    # 3. Leakage firewall against the eval holdout (conservative, value-level).
    print("[3/5] leakage firewall vs eval holdout")
    eval_vals = C.eval_value_set()
    keep_mask = df["text"].map(lambda v: normalise_value(v) not in eval_vals)
    n_dropped = int((~keep_mask).sum())
    df = df[keep_mask].reset_index(drop=True)
    print(f"      eval holdout values: {len(eval_vals)}; dropped {n_dropped} colliding rows")

    # 4. Train the YDF RandomForest prior.
    print(f"[4/5] training YDF RandomForest (num_trees={args.num_trees})")
    train_df = df.sample(frac=1.0 - args.holdout, random_state=args.seed)
    test_df = df.drop(train_df.index)
    cols = ["classification"] + feat_cols
    learner = ydf.RandomForestLearner(
        label="classification", num_trees=args.num_trees, winner_take_all=False
    )
    model = learner.train(train_df[cols])
    evaluation = model.evaluate(test_df[cols])
    accuracy = float(evaluation.accuracy)
    print(f"      held-out synthetic accuracy: {accuracy:.4f}")

    # 5. Save model + report.
    model_dir = C.MODELS_DIR / "ydf_value"
    print(f"[5/5] saving model -> {model_dir}")
    model.save(str(model_dir))

    support = df["classification"].value_counts().to_dict()
    report = {
        "n_synthetic_total": n_total,
        "n_after_firewall": len(df),
        "n_dropped_firewall": n_dropped,
        "n_train": len(train_df),
        "n_test": len(test_df),
        "n_types": int(df["classification"].nunique()),
        "feature_dim": len(feat_cols),
        "num_trees": args.num_trees,
        "holdout_accuracy": accuracy,
        "min_per_type_support": int(min(support.values())),
        "max_per_type_support": int(max(support.values())),
        "per_type_support": support,
    }
    (C.OUT_DIR / "ac01_prior_report.json").write_text(json.dumps(report, indent=2))

    md = [
        "# ac-01 — value-level YDF prior (synthetic bootstrap)",
        "",
        f"- synthetic values generated: **{n_total}** ({report['n_types']} types)",
        f"- leakage firewall dropped: **{n_dropped}** (eval-holdout collisions)",
        f"- trained on: **{len(train_df)}**, held out: **{len(test_df)}**",
        f"- held-out synthetic accuracy: **{accuracy:.4f}**",
        f"- per-type support: min **{report['min_per_type_support']}**, "
        f"max **{report['max_per_type_support']}**",
        f"- feature contract: **{len(feat_cols)}** dims (37 deterministic + 240 schema)",
        f"- model: `{model_dir.relative_to(C.REPO)}`",
        "",
        "Firewall note: synthetic values are headerless, so the firewall is a "
        "conservative value-level exclusion against the eval holdout values "
        "(cannot under-exclude vs the header-bearing row_hash). The load-bearing "
        "row_hash(header,value) firewall runs on the gittables side (ac-02/ac-08).",
    ]
    (C.OUT_DIR / "ac01_prior_report.md").write_text("\n".join(md) + "\n")
    print("done.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
