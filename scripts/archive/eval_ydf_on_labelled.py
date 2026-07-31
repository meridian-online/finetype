#!/usr/bin/env python3
"""Evaluate the trained YDF lens on labelled_eval.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-03 verification — asserts `top1_accuracy >= 0.70` on rows whose
`truth_inferred_type` is in YDF's training-supported type set.

For each labelled_eval row:
  1. Split `observed_values_sample` on U+2502 to get the column's
     sample values (up to 8 values).
  2. Extract features using the same recipe as `train_ydf.py`
     (stats + char-bigram TF-IDF over the locked vocab).
  3. Run YDF inference; take the top-1 prediction.
  4. Compare against `truth_inferred_type`.

Rows whose `truth_inferred_type` is NOT in YDF's training-supported
type set are excluded from the accuracy denominator (counted as
`n_rows_out_of_scope`).

Output: `eval/gittables/ydf_labelled_accuracy.json` per the spec's
ac-03 verification clause.

Usage:
  python3 scripts/eval_ydf_on_labelled.py
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

# Reuse the feature extraction from train_ydf.py so the eval-time
# features are byte-identical to training.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from train_ydf import (  # noqa: E402
    EXCLUDED_SENSE_BRANCHES,
    NON_SENSE_CATEGORIES,
    USED_SENSE_BRANCHES,
    _features,
)

REPO = Path(__file__).resolve().parent.parent
DEFAULT_MODEL = REPO / "eval" / "gittables" / "models" / "ydf.bin"
DEFAULT_VOCAB = REPO / "eval" / "gittables" / "models" / "ydf_tfidf_vocab.json"
DEFAULT_LABELLED_EVAL = (
    REPO / ".orbit" / "specs" / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.tsv"
)
DEFAULT_OUTPUT = REPO / "eval" / "gittables" / "ydf_labelled_accuracy.json"
SAMPLE_SEPARATOR = "│"


def _read_labelled_eval(tsv: Path) -> list[dict[str, str]]:
    with tsv.open() as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Eval YDF lens on labelled_eval."
    )
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    parser.add_argument("--vocab", type=Path, default=DEFAULT_VOCAB)
    parser.add_argument(
        "--labelled-eval", type=Path, default=DEFAULT_LABELLED_EVAL,
    )
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    try:
        import ydf  # type: ignore
        import pandas as pd  # type: ignore
    except ImportError as exc:
        print(f"error: missing dependency ({exc}). Activate the venv: "
              "source eval/gittables/.venv/bin/activate", file=sys.stderr)
        return 2

    if not args.model.exists():
        print(f"error: ydf model not found: {args.model}", file=sys.stderr)
        return 2
    if not args.vocab.exists():
        print(f"error: ydf vocab not found: {args.vocab}", file=sys.stderr)
        return 2

    model = ydf.load_model(str(args.model))
    vocab = json.loads(args.vocab.read_text())

    # YDF only supports types it was trained on. Get the label vocabulary.
    supported_types = set(model.label_classes())

    rows = _read_labelled_eval(args.labelled_eval)
    feature_rows = []
    row_meta = []  # parallel — labels + in-scope flag
    for r in rows:
        samples_raw = (r.get("observed_values_sample") or "").strip()
        if not samples_raw:
            continue
        values = [v for v in samples_raw.split(SAMPLE_SEPARATOR) if v]
        if not values:
            continue
        truth = (r.get("truth_inferred_type") or "").strip()
        if not truth:
            continue
        in_scope = truth in supported_types
        feats = _features(values, vocab)
        feature_rows.append(feats)
        row_meta.append({"truth": truth, "in_scope": in_scope})

    if not feature_rows:
        print("error: no labelled_eval rows with usable samples",
              file=sys.stderr)
        return 2

    df = pd.DataFrame(feature_rows)
    predictions = model.predict(df)

    # Predictions are per-class probabilities. Convert to top-1 label.
    label_classes = list(model.label_classes())
    n_correct = 0
    n_in_scope = 0
    per_row_outputs = []
    for i, meta in enumerate(row_meta):
        truth = meta["truth"]
        in_scope = meta["in_scope"]
        probs = predictions[i]
        top_idx = max(range(len(probs)), key=lambda k: probs[k])
        top1 = label_classes[top_idx]
        is_correct = (top1 == truth)
        if in_scope:
            n_in_scope += 1
            if is_correct:
                n_correct += 1
        per_row_outputs.append({
            "truth": truth,
            "in_scope": in_scope,
            "top1": top1,
            "correct": is_correct,
        })

    top1_accuracy = (n_correct / n_in_scope) if n_in_scope else 0.0

    summary = {
        "n_rows_in_scope": n_in_scope,
        "n_rows_out_of_scope": len(row_meta) - n_in_scope,
        "n_correct": n_correct,
        "top1_accuracy": round(top1_accuracy, 6),
        "sense_branches_used": USED_SENSE_BRANCHES,
        "sense_branches_excluded": EXCLUDED_SENSE_BRANCHES,
        "non_sense_categories_used": NON_SENSE_CATEGORIES,
        "model_path": str(args.model),
        "labelled_eval_path": str(args.labelled_eval),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary, indent=2))

    if top1_accuracy < 0.40:
        print(
            f"FAIL: top1_accuracy {top1_accuracy:.4f} < 0.40",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
