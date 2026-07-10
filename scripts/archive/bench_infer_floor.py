#!/usr/bin/env python3
"""Measure ac-02 floor: ≥60% non-unknown @ confidence ≥0.7 on the MEASURE half.

Per spec ac-02:
    Inference module produces non-`unknown` `inferred_correct_type` on
    ≥60% of B01/B04 entries from the MEASURE half of failure_log.tsv
    (rows where file_content_sha256 SHA-bucket MOD 2 == 1) at confidence
    ≥ ac02_threshold (= 0.7), with the locked weights w_v=0.4, w_h=0.6.

This script:
  1. Reads failure_log.measure.tsv (produced by scripts/split_failure_log.py).
  2. For each row, calls `finetype infer --mode column --batch --explain`
     over its (column_name, predicted_type, observed_values_sample).
  3. Counts rows with `inferred_correct_type != "unknown"` AND
     `confidence >= 0.7`.
  4. Asserts the ratio ≥ 0.60. Prints summary numbers to stdout for
     progress.md.

The script also computes the calibrate-half rate for the curve in
progress.md (curve table: threshold × {calibrate, measure} non-unknown rate).

USAGE
    python scripts/bench_infer_floor.py \
        --measure eval/gittables/failure_log.measure.tsv \
        --calibrate eval/gittables/failure_log.calibrate.tsv \
        --finetype-bin ./target/debug/finetype \
        --max-rows 500   # optional smoke run
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

THRESHOLDS = [0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]
AC02_THRESHOLD = 0.7
AC02_FLOOR = 0.60


def parse_observed_sample(s: str) -> list[str]:
    """Inverse of cron_cycle_work._truncate_sample — split on │."""
    if not s:
        return []
    return [v for v in s.split("│") if v]


def run_inference(
    finetype_bin: str,
    column_name: str,
    predicted_type: str,
    samples: list[str],
) -> dict:
    """Invoke `finetype infer --mode column --batch --explain` once and
    return parsed JSON output. The explain cascade was historically
    `finetype infer-type`; it now lives as the `--explain` flag on
    `infer`. Wire shape unchanged."""
    payload = json.dumps(
        {
            "column_name": column_name,
            "predicted_type": predicted_type,
            "samples": samples,
        }
    )
    res = subprocess.run(
        [finetype_bin, "infer", "--mode", "column", "--batch", "--explain"],
        input=payload,
        capture_output=True,
        text=True,
        timeout=30,
    )
    if res.returncode != 0:
        raise RuntimeError(
            f"finetype infer --explain failed: rc={res.returncode} stderr={res.stderr[:300]}"
        )
    return json.loads(res.stdout.strip())


def measure_half(
    path: Path, finetype_bin: str, max_rows: int | None = None
) -> tuple[int, int, dict[float, int]]:
    """Return (total, non_unknown_at_0.7, threshold_counts)."""
    total = 0
    threshold_hits = {t: 0 for t in THRESHOLDS}
    non_unknown_at_ac02 = 0
    if not path.exists():
        raise FileNotFoundError(f"measure half not found: {path}")
    with open(path, encoding="utf-8") as fh:
        header = fh.readline().rstrip("\n").split("\t")
        col_idx = {c: i for i, c in enumerate(header)}
        required = ["column_name", "predicted_type", "observed_values_sample"]
        for c in required:
            if c not in col_idx:
                raise ValueError(f"missing column {c} in {path}")
        for line in fh:
            if max_rows is not None and total >= max_rows:
                break
            cells = line.rstrip("\n").split("\t")
            if len(cells) < len(header):
                continue
            column_name = cells[col_idx["column_name"]]
            predicted_type = cells[col_idx["predicted_type"]]
            observed = parse_observed_sample(cells[col_idx["observed_values_sample"]])
            try:
                out = run_inference(
                    finetype_bin, column_name, predicted_type, observed
                )
            except Exception as exc:
                print(f"  ! row {total}: {exc}", file=sys.stderr)
                total += 1
                continue
            inferred = out.get("inferred_correct_type", "unknown")
            confidence = float(out.get("confidence", 0.0))
            if inferred != "unknown":
                for t in THRESHOLDS:
                    if confidence >= t:
                        threshold_hits[t] += 1
                if confidence >= AC02_THRESHOLD:
                    non_unknown_at_ac02 += 1
            total += 1
            if total % 100 == 0:
                print(f"  ... {total} rows scored", file=sys.stderr)
    return total, non_unknown_at_ac02, threshold_hits


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument(
        "--measure",
        type=Path,
        default=REPO_ROOT / "eval" / "gittables" / "failure_log.measure.tsv",
    )
    p.add_argument(
        "--calibrate",
        type=Path,
        default=REPO_ROOT / "eval" / "gittables" / "failure_log.calibrate.tsv",
    )
    p.add_argument(
        "--finetype-bin",
        default=str(REPO_ROOT / "target" / "release" / "finetype"),
    )
    p.add_argument(
        "--max-rows",
        type=int,
        default=None,
        help="Cap rows per half (None = full eval)",
    )
    p.add_argument(
        "--enforce-floor",
        action="store_true",
        help="Exit non-zero if ac-02 floor not met",
    )
    args = p.parse_args()

    if not Path(args.finetype_bin).exists():
        # Try the debug binary
        alt = REPO_ROOT / "target" / "debug" / "finetype"
        if alt.exists():
            args.finetype_bin = str(alt)
        else:
            print(
                f"finetype binary not found at {args.finetype_bin}; build first",
                file=sys.stderr,
            )
            return 2

    print("=" * 70, file=sys.stderr)
    print(f"AC-02 FLOOR MEASUREMENT (locked weights w_v=0.4, w_h=0.6)", file=sys.stderr)
    print(f"  finetype:  {args.finetype_bin}", file=sys.stderr)
    print(f"  measure:   {args.measure}", file=sys.stderr)
    print(f"  calibrate: {args.calibrate}", file=sys.stderr)
    print(f"  max_rows:  {args.max_rows}", file=sys.stderr)
    print("=" * 70, file=sys.stderr)

    print("\n[measure half — ac-02 floor]", file=sys.stderr)
    t0 = time.perf_counter()
    m_total, m_ac02, m_thr = measure_half(args.measure, args.finetype_bin, args.max_rows)
    m_secs = time.perf_counter() - t0
    print(f"  measure: {m_total} rows in {m_secs:.1f}s", file=sys.stderr)

    print("\n[calibrate half — curve only]", file=sys.stderr)
    t0 = time.perf_counter()
    c_total, _, c_thr = measure_half(
        args.calibrate, args.finetype_bin, args.max_rows
    )
    c_secs = time.perf_counter() - t0
    print(f"  calibrate: {c_total} rows in {c_secs:.1f}s", file=sys.stderr)

    # Summary
    summary = {
        "measure_total": m_total,
        "calibrate_total": c_total,
        "measure_non_unknown_at_0.7": m_ac02,
        "measure_floor_rate": (m_ac02 / m_total) if m_total else 0.0,
        "ac02_threshold": AC02_THRESHOLD,
        "ac02_floor": AC02_FLOOR,
        "thresholds": {
            "measure": {
                str(t): m_thr[t] / m_total if m_total else 0.0 for t in THRESHOLDS
            },
            "calibrate": {
                str(t): c_thr[t] / c_total if c_total else 0.0 for t in THRESHOLDS
            },
        },
    }
    summary["ac02_pass"] = summary["measure_floor_rate"] >= AC02_FLOOR
    print(json.dumps(summary, indent=2))

    if args.enforce_floor and not summary["ac02_pass"]:
        print(
            f"AC-02 FAILED: non-unknown @ ≥0.7 = {summary['measure_floor_rate']:.4f} "
            f"< floor {AC02_FLOOR}",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
