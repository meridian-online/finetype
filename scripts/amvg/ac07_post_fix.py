#!/usr/bin/env python3
"""ac-07: Post-fix diagnostic — verify the ac-06 remediation moves the named
mechanism per MADR 0065's `post_fix_assertion` frontmatter.

Mechanism named in decision 0065: `other` — header-hint over-generalisation
in `header_hint()` at crates/finetype-model/src/column.rs. The ac-06 fix
inserts 11 exact-match arms for the variant headers BEFORE the destructive
`h.contains("amount")` substring fall-through, so each variant header now
returns its specific `finance.currency.amount_<variant>` label instead of
the plain parent `finance.currency.amount`.

Post-fix assertion (verbatim from choice 0065, amount-subtype-collapse-mechanism):

  "In diagnostics/predictions_post.tsv (profile run against models/default on
  eval/datasets/csv/coverage_closure_phase_ab.csv after the ac-06 fix lands),
  at least 3 of the 11 target eval columns' top-1 predicted label flips from
  `finance.currency.amount` to the expected `finance.currency.amount_<variant>`
  label, versus the pre-fix diagnostics/predictions.tsv baseline."

Assertions (machine-checked):
  (a) predictions_post.tsv has 12 rows (11 targets + plain amount control)
  (b) post schema matches pre schema (column, expected, predicted, confidence)
  (c) at least 3 of the 11 target rows flip from `finance.currency.amount` in
      the pre baseline to the expected `finance.currency.amount_<variant>` label
      in the post run
  (d) the plain amount control column does NOT regress — it must still predict
      `finance.currency.amount` (i.e. no non-target regression on the control)
"""
import json
import os
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / ".orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"
EVAL_CSV = REPO_ROOT / "eval/datasets/csv/coverage_closure_phase_ab.csv"
MODEL = "models/default"

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
EXPECTED = {f"amount_{s.split('amount_', 1)[1]}": s for s in TARGET_SUBTYPES}
EXPECTED["amount"] = "finance.currency.amount"

MIN_FLIPS = 3  # MADR 0065 post_fix_assertion


def run_profile() -> dict:
    cmd = [
        "cargo", "run", "--bin", "finetype", "--quiet", "--",
        "profile", "--file", str(EVAL_CSV), "-o", "json",
    ]
    env = {**os.environ, "FINETYPE_MODEL": MODEL}
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, cwd=REPO_ROOT, env=env)
    return json.loads(out.stdout)


def read_pre_predictions(path: Path) -> dict[str, tuple[str, str, float]]:
    """Return map: column -> (expected, predicted, confidence)."""
    rows: dict[str, tuple[str, str, float]] = {}
    with open(path) as f:
        header = f.readline().strip().split("\t")
        assert header == ["column", "expected", "predicted", "confidence"], f"pre schema: {header}"
        for line in f:
            parts = line.rstrip("\n").split("\t")
            assert len(parts) == 4, f"pre row has {len(parts)} cols: {parts}"
            col, exp, pred, conf = parts
            rows[col] = (exp, pred, float(conf))
    return rows


def main() -> None:
    if not EVAL_CSV.exists():
        sys.exit(f"EVAL_CSV missing: {EVAL_CSV}")
    pre_path = DIAG_DIR / "predictions.tsv"
    if not pre_path.exists():
        sys.exit(f"pre-fix baseline missing: {pre_path} (run ac03_confusion.py first)")

    print(f"profiling {EVAL_CSV.name} with {MODEL} (post-fix)...")
    data = run_profile()
    cols = {c["column"]: c for c in data["columns"]}

    post_rows: list[tuple[str, str, str, float]] = []
    for col_name, expected in EXPECTED.items():
        if col_name not in cols:
            sys.exit(f"expected column {col_name} not in profile output")
        c = cols[col_name]
        post_rows.append((col_name, expected, c["type"], c["confidence"]))

    # (a) 12 rows
    assert len(post_rows) == 12, f"ac-07 (a): expected 12 rows, got {len(post_rows)}"

    DIAG_DIR.mkdir(parents=True, exist_ok=True)
    post_path = DIAG_DIR / "predictions_post.tsv"
    with open(post_path, "w") as f:
        f.write("column\texpected\tpredicted\tconfidence\n")
        for col, exp, pred, conf in post_rows:
            f.write(f"{col}\t{exp}\t{pred}\t{conf:.6f}\n")

    # (b) schema match — implicit: we wrote the same 4 headers.

    pre = read_pre_predictions(pre_path)
    # (c) flips from plain amount to the expected variant on target rows
    flips: list[str] = []
    persistent_amount: list[str] = []
    other_changes: list[tuple[str, str, str, str]] = []
    for col, exp, post_pred, _ in post_rows:
        if exp == "finance.currency.amount":
            continue  # control, handled separately
        pre_exp, pre_pred, _ = pre[col]
        assert pre_exp == exp, f"expected mismatch for {col}: pre={pre_exp} vs {exp}"
        if pre_pred == "finance.currency.amount" and post_pred == exp:
            flips.append(col)
        elif pre_pred == "finance.currency.amount" and post_pred == "finance.currency.amount":
            persistent_amount.append(col)
        elif pre_pred != post_pred:
            other_changes.append((col, exp, pre_pred, post_pred))

    # (d) plain amount control must still predict plain amount
    control = [r for r in post_rows if r[1] == "finance.currency.amount"][0]
    control_pred = control[2]
    assert control_pred == "finance.currency.amount", (
        f"ac-07 (d): plain amount control regressed: pre=finance.currency.amount "
        f"post={control_pred}"
    )

    # Summary
    print()
    print(f"flips (pre=plain amount, post=expected variant): {len(flips)}/11")
    for c in flips:
        print(f"  ✓ {c}")
    print(f"persistent plain-amount (no flip): {len(persistent_amount)}/11")
    for c in persistent_amount:
        print(f"  - {c}")
    if other_changes:
        print(f"other label changes: {len(other_changes)}")
        for (col, exp, pre_p, post_p) in other_changes:
            print(f"  ~ {col}: {pre_p} -> {post_p} (expected {exp})")

    # (c) MADR assertion
    assert len(flips) >= MIN_FLIPS, (
        f"ac-07 (c): MADR 0065 post_fix_assertion requires >= {MIN_FLIPS} flips, "
        f"got {len(flips)}. Mechanism not measurably reduced."
    )

    print()
    print(f"ac-07 PASS: {len(flips)}/11 flips >= {MIN_FLIPS} threshold.")
    print(f"ac-07 artefact: diagnostics/predictions_post.tsv")


if __name__ == "__main__":
    main()
