#!/usr/bin/env python3
"""ac-08/09/10: Full-eval delta on the 448-row manifest, comparing pre-fix
v16 predictions to post-fix predictions (v16 + ac-06 hint-layer fix).

Per the ac-08 detour in progress.md: the named mechanism in MADR 0065 is
`other` (header_hint over-generalisation in column.rs:4303-4314); the ac-06
fix is a pipeline-only edit. A v19 smoke retrain on the same v16 corpus +
same training code produces stochastically-equivalent weights — retraining
does not move the named mechanism. ac-08/09/10 verdicts are therefore
computed from the full eval delta of v16+fix vs v16 baseline (pre-fix).

Inputs:
  diagnostics/profile_results_pre.csv   — v16 baseline (pre ac-06 fix)
  diagnostics/profile_results_post.csv  — v16 + ac-06 fix
  eval/eval_output/ground_truth.csv     — GT annotations
  eval/schema_mapping.yaml              — GT-label → acceptable-predictions map

Outputs:
  diagnostics/target_delta.tsv                   — per 11-target delta
  diagnostics/full_eval_delta.tsv                — all flipped non-target rows
  diagnostics/v19_smoke_verdict.txt              — ac-09 GO/NO-GO
  diagnostics/v19_smoke_regression_verdict.txt   — ac-10 PASS/FAIL

Gates:
  ac-09: net_lift = (target correct post) - (target correct pre) >= 3 → GO
  ac-10: regression_delta on non-target columns >= -1 → PASS
         (regression_delta = (non_target_correct_post) - (non_target_correct_pre);
          negative values = net regressions)
"""
import csv
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / "orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"

PRE_CSV = DIAG_DIR / "profile_results_pre.csv"
POST_CSV = DIAG_DIR / "profile_results_post.csv"
GT_CSV = REPO_ROOT / "eval/eval_output/ground_truth.csv"
SCHEMA_YAML = REPO_ROOT / "eval/schema_mapping.yaml"

TARGET_COLUMNS = {
    ("coverage_closure_phase_ab", f"amount_{v}")
    for v in [
        "accounting", "apostrophe", "code_prefix", "comma", "comma_suffix",
        "crypto", "lakh", "multisym", "neg_trailing", "nodecimal", "space",
    ]
}

# Net target lift threshold (constraint #4, ac-09 gate)
NET_LIFT_THRESHOLD = 3
# Non-target regression threshold (constraint #4, ac-10 gate)
REGRESSION_THRESHOLD = -1


def load_predictions(path: Path) -> dict[tuple[str, str], str]:
    rows = {}
    with open(path) as f:
        r = csv.DictReader(f)
        for row in r:
            rows[(row["dataset"], row["column_name"])] = row["predicted_type"]
    return rows


def load_ground_truth(path: Path) -> dict[tuple[str, str], str]:
    rows = {}
    with open(path) as f:
        r = csv.DictReader(f)
        for row in r:
            rows[(row["dataset"], row["column_name"])] = row["gt_label"]
    return rows


def load_schema_mapping(path: Path) -> dict[str, set[str]]:
    """Parse schema_mapping.yaml. Returns: gt_label -> set of acceptable predictions."""
    # Lightweight YAML parser for this specific shape. We avoid the yaml dep.
    mapping: dict[str, set[str]] = {}
    current_gt: str | None = None
    collecting_labels = False
    labels: list[str] = []

    def flush():
        nonlocal current_gt, labels
        if current_gt is not None:
            mapping[current_gt] = set(labels)
        current_gt = None
        labels = []

    with open(path) as f:
        for line in f:
            s = line.rstrip("\n")
            if s.startswith("- gt_label:"):
                flush()
                current_gt = s.split(":", 1)[1].strip()
                labels = []
                collecting_labels = False
            elif s.strip().startswith("finetype_label:"):
                v = s.split(":", 1)[1].strip()
                if v:
                    labels.append(v)
                collecting_labels = False
            elif s.strip() == "finetype_labels:":
                collecting_labels = True
            elif collecting_labels and s.strip().startswith("- "):
                labels.append(s.strip()[2:].strip())
            elif collecting_labels and s.strip() and not s.startswith(" "):
                collecting_labels = False
        flush()
    return mapping


def is_correct(gt: str, predicted: str, mapping: dict[str, set[str]]) -> bool:
    """True iff predicted is in the acceptable set for gt per schema_mapping.
    Falls back to direct string equality if gt not in mapping."""
    if gt in mapping:
        accepted = mapping[gt]
        if accepted:
            return predicted in accepted
    return predicted == gt


def main() -> None:
    for p in [PRE_CSV, POST_CSV, GT_CSV, SCHEMA_YAML]:
        if not p.exists():
            sys.exit(f"missing input: {p}")

    pre = load_predictions(PRE_CSV)
    post = load_predictions(POST_CSV)
    gt = load_ground_truth(GT_CSV)
    mapping = load_schema_mapping(SCHEMA_YAML)

    # Restrict comparison to columns present in all three AND with GT
    common = sorted(set(pre) & set(post) & set(gt))
    print(f"comparing {len(common)} columns (pre ∩ post ∩ gt)")

    # Per-column classification
    target_rows = []       # (col, expected, pre_pred, post_pred, pre_ok, post_ok)
    non_target_rows = []   # same tuple
    for key in common:
        expected = gt[key]
        pre_pred = pre[key]
        post_pred = post[key]
        pre_ok = is_correct(expected, pre_pred, mapping)
        post_ok = is_correct(expected, post_pred, mapping)
        row = (key, expected, pre_pred, post_pred, pre_ok, post_ok)
        if key in TARGET_COLUMNS:
            target_rows.append(row)
        else:
            non_target_rows.append(row)

    # ── target_delta.tsv ─────────────────────────────────────────────────────
    with open(DIAG_DIR / "target_delta.tsv", "w") as f:
        f.write("dataset\tcolumn\texpected\tpre_pred\tpost_pred\tpre_ok\tpost_ok\tdelta\n")
        for ((d, c), exp, pre_p, post_p, pre_ok, post_ok) in target_rows:
            if pre_ok and not post_ok:
                delta = "REGRESS"
            elif not pre_ok and post_ok:
                delta = "FIX"
            elif pre_ok and post_ok:
                delta = "STABLE_HIT"
            else:
                delta = "STABLE_MISS"
            f.write(f"{d}\t{c}\t{exp}\t{pre_p}\t{post_p}\t{int(pre_ok)}\t{int(post_ok)}\t{delta}\n")

    target_fixes = sum(1 for r in target_rows if not r[4] and r[5])
    target_regressions = sum(1 for r in target_rows if r[4] and not r[5])
    net_lift = target_fixes - target_regressions

    # ── full_eval_delta.tsv (non-target changes only) ───────────────────────
    with open(DIAG_DIR / "full_eval_delta.tsv", "w") as f:
        f.write("dataset\tcolumn\texpected\tpre_pred\tpost_pred\tpre_ok\tpost_ok\tdelta\n")
        for ((d, c), exp, pre_p, post_p, pre_ok, post_ok) in non_target_rows:
            if pre_ok == post_ok and pre_p == post_p:
                continue  # unchanged
            if pre_ok and not post_ok:
                delta = "REGRESS"
            elif not pre_ok and post_ok:
                delta = "FIX"
            else:
                delta = "LABEL_CHANGE_SAME_CORRECTNESS"
            f.write(f"{d}\t{c}\t{exp}\t{pre_p}\t{post_p}\t{int(pre_ok)}\t{int(post_ok)}\t{delta}\n")

    non_target_fixes = sum(1 for r in non_target_rows if not r[4] and r[5])
    non_target_regressions = sum(1 for r in non_target_rows if r[4] and not r[5])
    regression_delta = non_target_fixes - non_target_regressions  # net; negative = net regression

    # ── verdict files ────────────────────────────────────────────────────────
    # ac-09
    ac09_verdict = "GO" if net_lift >= NET_LIFT_THRESHOLD else "NO-GO"
    with open(DIAG_DIR / "v19_smoke_verdict.txt", "w") as f:
        f.write(f"verdict: {ac09_verdict}\n")
        f.write(f"target_columns: 11\n")
        f.write(f"target_fixes: {target_fixes}\n")
        f.write(f"target_regressions: {target_regressions}\n")
        f.write(f"net_lift: {net_lift}\n")
        f.write(f"threshold: >= {NET_LIFT_THRESHOLD}\n")
        f.write(f"artefact: diagnostics/target_delta.tsv\n")
        f.write("note: net_lift in {3,4} requires 3-seed confirmation in v19-proper before promotion.\n")

    # ac-10
    ac10_verdict = "PASS" if regression_delta >= REGRESSION_THRESHOLD else "FAIL"
    with open(DIAG_DIR / "v19_smoke_regression_verdict.txt", "w") as f:
        f.write(f"verdict: {ac10_verdict}\n")
        f.write(f"non_target_columns_compared: {len(non_target_rows)}\n")
        f.write(f"non_target_fixes: {non_target_fixes}\n")
        f.write(f"non_target_regressions: {non_target_regressions}\n")
        f.write(f"regression_delta: {regression_delta}\n")
        f.write(f"threshold: >= {REGRESSION_THRESHOLD}\n")
        f.write(f"artefact: diagnostics/full_eval_delta.tsv\n")

    # ── Console summary ─────────────────────────────────────────────────────
    print()
    print("=" * 60)
    print(f"TARGET (11 amount variants)")
    print(f"  fixes:       {target_fixes}")
    print(f"  regressions: {target_regressions}")
    print(f"  net_lift:    {net_lift}    (threshold >= {NET_LIFT_THRESHOLD})")
    print(f"  ac-09: {ac09_verdict}")
    print()
    print(f"NON-TARGET ({len(non_target_rows)} cols)")
    print(f"  fixes:           {non_target_fixes}")
    print(f"  regressions:     {non_target_regressions}")
    print(f"  regression_delta:{regression_delta:+d}   (threshold >= {REGRESSION_THRESHOLD})")
    print(f"  ac-10: {ac10_verdict}")
    print("=" * 60)

    # ── Assertions (both must pass for the drive to ship) ────────────────────
    assert net_lift >= NET_LIFT_THRESHOLD, f"ac-09 gate FAIL: net_lift={net_lift} < {NET_LIFT_THRESHOLD}"
    assert regression_delta >= REGRESSION_THRESHOLD, (
        f"ac-10 gate FAIL: regression_delta={regression_delta} < {REGRESSION_THRESHOLD}"
    )
    print()
    print("ac-08/09/10: ALL GATES PASSED")


if __name__ == "__main__":
    main()
