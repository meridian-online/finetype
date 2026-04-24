#!/usr/bin/env python3
"""ac-03: Confusion matrix on the 11 eval columns using v16 (models/default).

Runs `finetype profile` on the eval corpus CSV (coverage_closure_phase_ab.csv),
extracts the 11 target columns' predictions, and writes a confusion matrix
alongside a per-column prediction trace.

Artefacts:
  diagnostics/confusion.tsv       — rows=expected subtype, cols=predicted label
  diagnostics/predictions.tsv     — per-column: expected, predicted, confidence

Spec v1.2 ac-03 assertions:
  (a) 11 rows in confusion.tsv (one per target subtype, excluding plain amount)
  (b) all rows tabulate the same set of predicted labels (columns)
  (c) row sums == 1 (one prediction per eval column)
"""
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / "orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"
EVAL_CSV = REPO_ROOT / "eval/datasets/csv/coverage_closure_phase_ab.csv"
MODEL = "models/default"

# 11 target subtypes (plain amount is control, tracked separately)
TARGET_SUBTYPES = [
    "finance.currency.amount_accounting",
    "finance.currency.amount_apostrophe",
    "finance.currency.amount_code_prefix",
    "finance.currency.amount_comma",
    "finance.currency.amount_comma_suffix",
    "finance.currency.amount_crypto",
    "finance.currency.amount_lakh",
    "finance.currency.amount_multisym",
    "finance.currency.amount_neg_trailing",
    "finance.currency.amount_nodecimal",
    "finance.currency.amount_space",
]
# Column name → ground truth label
# (the CSV column names match the subtype suffix after `finance.currency.`)
EXPECTED = {f"amount_{s.split('amount_', 1)[1]}": s for s in TARGET_SUBTYPES}
EXPECTED["amount"] = "finance.currency.amount"  # control row in predictions.tsv


def run_profile() -> dict:
    cmd = [
        "cargo", "run", "--bin", "finetype", "--quiet", "--",
        "profile", "--file", str(EVAL_CSV), "-o", "json", "--model", MODEL,
    ]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=REPO_ROOT)
    return json.loads(out.stdout)


def main() -> None:
    if not EVAL_CSV.exists():
        sys.exit(f"EVAL_CSV missing: {EVAL_CSV}")

    print(f"profiling {EVAL_CSV.name} with {MODEL}...")
    data = run_profile()
    cols = {c["column"]: c for c in data["columns"]}

    # Per-column predictions (include plain amount as 12th control row)
    pred_rows = []
    for col_name, expected in EXPECTED.items():
        if col_name not in cols:
            sys.exit(f"expected column {col_name} not in profile output")
        c = cols[col_name]
        pred_rows.append((col_name, expected, c["type"], c["confidence"]))

    # Write predictions.tsv
    DIAG_DIR.mkdir(parents=True, exist_ok=True)
    with open(DIAG_DIR / "predictions.tsv", "w") as f:
        f.write("column\texpected\tpredicted\tconfidence\n")
        for col, exp, pred, conf in pred_rows:
            f.write(f"{col}\t{exp}\t{pred}\t{conf:.6f}\n")

    # Build confusion matrix over the 11 targets only (exclude plain amount)
    target_rows = [(c, e, p) for (c, e, p, _) in pred_rows if e != "finance.currency.amount"]
    predicted_labels = sorted({p for (_, _, p) in target_rows})
    assert len(target_rows) == 11, f"expected 11 target rows, got {len(target_rows)}"

    with open(DIAG_DIR / "confusion.tsv", "w") as f:
        f.write("expected\t" + "\t".join(predicted_labels) + "\n")
        for sub in TARGET_SUBTYPES:
            row = {p: 0 for p in predicted_labels}
            for (col, exp, pred) in target_rows:
                if exp == sub:
                    row[pred] += 1
            assert sum(row.values()) == 1, f"ac-03 (c) row sum != 1 for {sub}"
            f.write(sub + "\t" + "\t".join(str(row[p]) for p in predicted_labels) + "\n")

    # Summary
    correct = sum(1 for (_, e, p) in target_rows if e == p)
    flat_500 = sum(1 for (_, _, _, conf) in pred_rows if abs(conf - 0.5) < 1e-6)
    print(f"ac-03: correct={correct}/11, flat-0.500-confidences={flat_500}/12")
    print(f"predicted label set: {predicted_labels}")
    print("ac-03 artefacts: diagnostics/confusion.tsv, diagnostics/predictions.tsv")


if __name__ == "__main__":
    main()
