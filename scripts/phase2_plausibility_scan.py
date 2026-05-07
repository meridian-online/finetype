#!/usr/bin/env python3
"""Phase 2 plausibility scan — back-of-envelope for ac-01 of finetype-dnf.

For each row in a deterministic sample of failure_log.measure.tsv:
  1. Invoke `finetype infer-type` to capture per-signal scores under
     locked Phase 1 weights (w_v=0.4, w_h=0.6).
  2. Bin the row by Phase-1 outcome (≥0.7 / 0.5–0.7 / <0.5) and by
     signal pattern (header_match strong/weak; validator_pass_rate
     strong/weak).
  3. Classify each <0.7 row as a candidate-recoverable-by-signal:
       - generator-shape addressable: argmax validator_pass_rate ≥ 0.7
         AND header_match < 0.3 (shape-uniform values, neutral header)
       - sibling-context addressable: argmax score < 0.7 AND BOTH
         validator_pass_rate < 0.7 AND header_match < 0.5 (weak both
         axes — column needs disambiguating context)
       - either (overlap is permitted)
  4. Estimate plausible lift under modest assumptions about the new
     signals (cap each signal's contribution at the unallocated weight
     mass; lift is the fraction of failing rows that would clear 0.7
     IF the new signal contributed at full strength on its addressable
     subset).

Output (stdout JSON): per-bin counts, plausibility estimates, and the
tail of rows where neither signal looks decisive.

Determinism: sample is the first N rows in file order from
failure_log.measure.tsv (which is itself partitioned by SHA-bucket so
the ordering is content-addressed). N defaults to 1000 to match Phase 1.

Usage:
    python scripts/phase2_plausibility_scan.py --max-rows 1000 \
        --finetype-bin ./target/release/finetype \
        --output orbit/specs/2026-05-04-autonomous-type-inference/phase2_plausibility.json
"""
from __future__ import annotations

import argparse
import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Phase 1 locked weights
W_V = 0.4
W_H = 0.6

# Phase 1 thresholds
AC02_THRESHOLD = 0.7
FALLBACK_THRESHOLD = 0.4

# Signal-pattern thresholds for plausibility classification
NEUTRAL_HEADER_MAX = 0.3
INFORMATIVE_HEADER_MIN = 0.5
SHAPE_UNIFORM_MIN = 0.7  # argmax validator_pass_rate threshold
WEAK_VALIDATOR_MAX = 0.7  # below this is "weak validator"


def parse_observed_sample(s: str) -> list[str]:
    if not s:
        return []
    return [v for v in s.split("│") if v]


def run_inference(
    finetype_bin: str,
    column_name: str,
    predicted_type: str,
    samples: list[str],
) -> dict:
    payload = json.dumps(
        {
            "column_name": column_name,
            "predicted_type": predicted_type,
            "samples": samples,
        }
    )
    res = subprocess.run(
        [finetype_bin, "infer-type"],
        input=payload,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"infer-type rc={res.returncode} stderr={res.stderr[:200]}"
        )
    return json.loads(res.stdout.strip())


def classify_failure_signal(
    final_score: float,
    validator_pass_rate: float,
    header_match: float,
) -> dict:
    """Tag a Phase-1-failing row (final < 0.7) by which Phase 2 signal
    is most plausibly addressable for it."""
    is_failing = final_score < AC02_THRESHOLD
    if not is_failing:
        return {"failing": False, "addressable": "passes_phase1"}

    shape_uniform = validator_pass_rate >= SHAPE_UNIFORM_MIN
    neutral_header = header_match < NEUTRAL_HEADER_MAX
    weak_validator = validator_pass_rate < WEAK_VALIDATOR_MAX
    moderate_header = header_match < INFORMATIVE_HEADER_MIN

    gen_shape_addressable = shape_uniform and neutral_header
    sib_ctx_addressable = weak_validator and moderate_header

    if gen_shape_addressable and sib_ctx_addressable:
        tag = "both"
    elif gen_shape_addressable:
        tag = "generator_shape_only"
    elif sib_ctx_addressable:
        tag = "sibling_context_only"
    else:
        tag = "neither"

    return {
        "failing": True,
        "addressable": tag,
        "shape_uniform": shape_uniform,
        "neutral_header": neutral_header,
        "weak_validator": weak_validator,
    }


def main() -> int:
    p = argparse.ArgumentParser()
    p.add_argument(
        "--measure",
        type=Path,
        default=REPO_ROOT / "eval" / "gittables" / "failure_log.measure.tsv",
    )
    p.add_argument(
        "--finetype-bin",
        default=str(REPO_ROOT / "target" / "release" / "finetype"),
    )
    p.add_argument("--max-rows", type=int, default=1000)
    p.add_argument("--output", type=Path, default=None)
    args = p.parse_args()

    print(f"[phase2_plausibility_scan] {args.max_rows} rows from {args.measure.name}", file=sys.stderr)

    rows: list[dict] = []
    score_buckets = {"high": 0, "mid": 0, "low": 0}  # ≥0.7, 0.5-0.7, <0.5
    addressability = {
        "passes_phase1": 0,
        "generator_shape_only": 0,
        "sibling_context_only": 0,
        "both": 0,
        "neither": 0,
    }
    header_buckets = {"informative": 0, "moderate": 0, "neutral": 0}
    validator_buckets = {"strong": 0, "moderate": 0, "weak": 0}
    failing_v_distribution: list[float] = []
    failing_h_distribution: list[float] = []

    with open(args.measure, encoding="utf-8") as fh:
        header = fh.readline().rstrip("\n").split("\t")
        col_idx = {c: i for i, c in enumerate(header)}
        t0 = time.perf_counter()
        for line_no, line in enumerate(fh):
            if line_no >= args.max_rows:
                break
            cells = line.rstrip("\n").split("\t")
            if len(cells) < len(header):
                continue
            column_name = cells[col_idx["column_name"]]
            predicted_type = cells[col_idx["predicted_type"]]
            samples = parse_observed_sample(cells[col_idx["observed_values_sample"]])
            try:
                out = run_inference(args.finetype_bin, column_name, predicted_type, samples)
            except Exception as exc:
                print(f"  ! row {line_no}: {exc}", file=sys.stderr)
                continue
            confidence = float(out.get("confidence", 0.0))
            sigs = out.get("signals", {})
            v = float(sigs.get("validator_pass_rate", 0.0))
            h = float(sigs.get("header_match", 0.0))
            inferred = out.get("inferred_correct_type", "unknown")

            # Bucket by Phase 1 score
            if confidence >= AC02_THRESHOLD:
                score_buckets["high"] += 1
            elif confidence >= 0.5:
                score_buckets["mid"] += 1
            else:
                score_buckets["low"] += 1

            # Bucket by header
            if h >= INFORMATIVE_HEADER_MIN:
                header_buckets["informative"] += 1
            elif h >= NEUTRAL_HEADER_MAX:
                header_buckets["moderate"] += 1
            else:
                header_buckets["neutral"] += 1

            # Bucket by validator
            if v >= SHAPE_UNIFORM_MIN:
                validator_buckets["strong"] += 1
            elif v >= 0.4:
                validator_buckets["moderate"] += 1
            else:
                validator_buckets["weak"] += 1

            cls = classify_failure_signal(confidence, v, h)
            addressability[cls["addressable"]] += 1
            if cls["failing"]:
                failing_v_distribution.append(v)
                failing_h_distribution.append(h)
            rows.append(
                {
                    "line": line_no,
                    "column_name": column_name,
                    "predicted_type": predicted_type,
                    "inferred": inferred,
                    "confidence": confidence,
                    "v": v,
                    "h": h,
                    "addressable": cls["addressable"],
                }
            )
            if (line_no + 1) % 100 == 0:
                print(f"  ... {line_no+1} rows", file=sys.stderr)
        elapsed = time.perf_counter() - t0
        print(f"  ... done {len(rows)} rows in {elapsed:.1f}s", file=sys.stderr)

    total = len(rows)
    failing = total - addressability["passes_phase1"]
    summary = {
        "total_rows_scanned": total,
        "phase1_passes_at_0.7": addressability["passes_phase1"],
        "phase1_passes_rate": addressability["passes_phase1"] / total if total else 0.0,
        "phase1_failing": failing,
        "phase1_failing_rate": failing / total if total else 0.0,
        "score_buckets": score_buckets,
        "header_buckets": header_buckets,
        "validator_buckets": validator_buckets,
        "addressability_among_failing": {
            k: v for k, v in addressability.items() if k != "passes_phase1"
        },
        "addressability_rates_among_failing": {
            k: (v / failing if failing else 0.0)
            for k, v in addressability.items()
            if k != "passes_phase1"
        },
        # Plausibility ESTIMATES (back-of-envelope; not a measurement)
        # Assume new signals contribute "as much as" header does today (modest)
        # and address only their tagged subset. Lift = fraction of *all rows*
        # that move from <0.7 to ≥0.7 under those assumptions.
        "plausibility_estimates": {
            "method": (
                "Upper-bound estimate: each new signal lifts its 'addressable' "
                "subset by 0.4 (the unallocated weight mass under a 4-signal "
                "redistribution that halves Phase-1 weights). 'Both' rows can "
                "be lifted by either signal. Real lift will be lower — this "
                "is a ceiling, not a forecast."
            ),
            "generator_shape_addressable_rows": (
                addressability["generator_shape_only"] + addressability["both"]
            ),
            "sibling_context_addressable_rows": (
                addressability["sibling_context_only"] + addressability["both"]
            ),
            "either_addressable_rows": (
                addressability["generator_shape_only"]
                + addressability["sibling_context_only"]
                + addressability["both"]
            ),
            "neither_addressable_rows": addressability["neither"],
            "ceiling_lift_generator_shape_pp": (
                (addressability["generator_shape_only"] + addressability["both"])
                / total * 100
                if total else 0.0
            ),
            "ceiling_lift_sibling_context_pp": (
                (addressability["sibling_context_only"] + addressability["both"])
                / total * 100
                if total else 0.0
            ),
            "ceiling_lift_either_pp": (
                (addressability["generator_shape_only"]
                 + addressability["sibling_context_only"]
                 + addressability["both"])
                / total * 100
                if total else 0.0
            ),
            "structural_floor_neither_pp": (
                addressability["neither"] / total * 100 if total else 0.0
            ),
        },
        "failing_rows_signal_distribution": {
            "v_median": statistics.median(failing_v_distribution) if failing_v_distribution else None,
            "v_mean": statistics.mean(failing_v_distribution) if failing_v_distribution else None,
            "h_median": statistics.median(failing_h_distribution) if failing_h_distribution else None,
            "h_mean": statistics.mean(failing_h_distribution) if failing_h_distribution else None,
        },
    }

    print(json.dumps(summary, indent=2))

    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, "w") as fh:
            json.dump({"summary": summary, "rows": rows}, fh, indent=2)
        print(f"  wrote per-row data to {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
