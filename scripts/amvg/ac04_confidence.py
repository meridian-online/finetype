#!/usr/bin/env python3
"""ac-04: Per-subtype confidence distribution (11 subtypes × top-5 = 55 rows)
from v16, with softmax-consistency checks.

Runs cargo example amvg_topk for each of the 11 target eval columns and writes
diagnostics/confidence_dist.tsv with columns:
  subtype, rank, predicted_label, probability

Spec v1.2 ac-04 assertions:
  (a) exactly 55 rows (11 subtypes × 5 ranks)
  (b) every subtype appears with ranks 1..5
  (c) every probability in strict (0, 1) — strictly positive, strictly < 1
  (d) top-1 probability >= 1/240 (uniform floor; 240 output classes)
  (e) per-subtype, ranks are in non-increasing order
  (f) per-subtype, top-5 probability sum is in (0, 1.0] (sub-portion of
      full softmax; not the full distribution so <1 is expected)
"""
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / "orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"
EVAL_CSV = REPO_ROOT / "eval/datasets/csv/coverage_closure_phase_ab.csv"
MODEL_DIR = REPO_ROOT / "models/default"

TARGET_COLUMNS = [
    "amount_accounting",
    "amount_apostrophe",
    "amount_code_prefix",
    "amount_comma",
    "amount_comma_suffix",
    "amount_crypto",
    "amount_lakh",
    "amount_multisym",
    "amount_neg_trailing",
    "amount_nodecimal",
    "amount_space",
]

K = 5
N_CLASSES = 240
UNIFORM_FLOOR = 1.0 / N_CLASSES  # top-1 must be at least uniform


def run_topk(column: str) -> list[tuple[int, str, float]]:
    cmd = [
        "cargo", "run", "--example", "amvg_topk", "--quiet",
        "-p", "finetype-model", "--",
        str(EVAL_CSV), column, str(MODEL_DIR), str(K),
    ]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=REPO_ROOT)
    rows = []
    for line in out.stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) != 3:
            sys.exit(f"amvg_topk produced unexpected line: {line!r}")
        rank = int(parts[0])
        label = parts[1]
        prob = float(parts[2])
        rows.append((rank, label, prob))
    if len(rows) != K:
        sys.exit(f"{column}: expected {K} rows, got {len(rows)}")
    return rows


def main() -> None:
    print(f"running top-{K} inference on {len(TARGET_COLUMNS)} target columns...")
    all_rows: list[tuple[str, int, str, float]] = []
    for col in TARGET_COLUMNS:
        topk = run_topk(col)
        for (rank, label, prob) in topk:
            all_rows.append((col, rank, label, prob))
        top1 = topk[0]
        print(f"  {col:24s} top-1: {top1[1]:45s}  p={top1[2]:.4f}")

    # Assertions
    # (a) 55 rows
    assert len(all_rows) == 11 * K, f"ac-04 (a): expected {11*K} rows, got {len(all_rows)}"

    for col in TARGET_COLUMNS:
        col_rows = [r for r in all_rows if r[0] == col]
        # (b) ranks 1..5 present
        assert [r[1] for r in col_rows] == [1, 2, 3, 4, 5], f"ac-04 (b): {col} ranks"
        # (e) non-increasing
        probs = [r[3] for r in col_rows]
        for i in range(1, K):
            assert probs[i] <= probs[i - 1] + 1e-6, f"ac-04 (e): {col} rank {i+1} > rank {i}"
        # (d) top-1 >= uniform floor
        assert probs[0] >= UNIFORM_FLOOR - 1e-9, f"ac-04 (d): {col} top-1 < uniform"
        # (f) top-5 sum in (0, 1.0]
        s = sum(probs)
        assert 0.0 < s <= 1.0 + 1e-6, f"ac-04 (f): {col} top-5 sum = {s}"

    # (c) every prob in strict (0, 1)
    for (col, rank, label, prob) in all_rows:
        assert 0.0 < prob < 1.0, f"ac-04 (c): {col} rank {rank} prob = {prob} not in (0,1)"

    # Write TSV
    DIAG_DIR.mkdir(parents=True, exist_ok=True)
    with open(DIAG_DIR / "confidence_dist.tsv", "w") as f:
        f.write("subtype\trank\tpredicted_label\tprobability\n")
        for (col, rank, label, prob) in all_rows:
            f.write(f"{col}\t{rank}\t{label}\t{prob:.6f}\n")

    # Mechanism-probing summary
    expected_in_top5 = 0
    expected_top1 = 0
    for col in TARGET_COLUMNS:
        expected_label = f"finance.currency.{col}"
        col_rows = [r for r in all_rows if r[0] == col]
        labels = [r[2] for r in col_rows]
        if expected_label in labels:
            expected_in_top5 += 1
            if labels[0] == expected_label:
                expected_top1 += 1

    print(f"expected label top-1: {expected_top1}/11")
    print(f"expected label in top-5: {expected_in_top5}/11")
    print("ac-04 artefact: diagnostics/confidence_dist.tsv")


if __name__ == "__main__":
    main()
