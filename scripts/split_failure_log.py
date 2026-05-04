#!/usr/bin/env python3
"""Partition `eval/gittables/failure_log.tsv` deterministically by SHA mod 2.

Bucket 0 = calibrate (descriptive sweep, curve in progress.md).
Bucket 1 = measure (ac-02 floor, ac-13 labelled-eval pool).

Per spec ac-02 / constraint "Calibration partition":
    file_content_sha256 (hex string) interpreted as int -> MOD 2.

The split is a leak-free partition: every row goes to exactly one
bucket, deterministically derived from its content hash. Calling this
script a second time produces byte-identical output (modulo system
clock — there is none; the script is content-only).

USAGE
    python scripts/split_failure_log.py \
        --input eval/gittables/failure_log.tsv \
        --calibrate eval/gittables/failure_log.calibrate.tsv \
        --measure   eval/gittables/failure_log.measure.tsv

The output files share the same 9-column header as the input
(failure_log.tsv schema is unchanged per ac-12).
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path


def sha_bucket(sha: str) -> int:
    """Map a hex SHA-256 string to {0, 1} via int % 2.

    `sha` must be a hex string. Returns 0 (calibrate) or 1 (measure).
    """
    return int(sha, 16) % 2


def split(
    input_path: Path,
    calibrate_path: Path,
    measure_path: Path,
) -> tuple[int, int]:
    """Split input TSV into two output TSVs by SHA-bucket. Returns (n_cal, n_meas)."""
    if not input_path.exists():
        raise FileNotFoundError(f"input not found: {input_path}")

    n_cal = 0
    n_meas = 0
    with open(input_path, encoding="utf-8") as fin, \
         open(calibrate_path, "w", encoding="utf-8") as fcal, \
         open(measure_path, "w", encoding="utf-8") as fmeas:
        header = fin.readline()
        if not header:
            raise ValueError(f"input is empty: {input_path}")
        fcal.write(header)
        fmeas.write(header)
        # Locate the file_content_sha256 column index from the header
        col_names = header.rstrip("\n").split("\t")
        try:
            sha_idx = col_names.index("file_content_sha256")
        except ValueError as exc:
            raise ValueError(
                f"input missing file_content_sha256 column: {col_names}"
            ) from exc
        for line in fin:
            if not line.strip():
                continue
            cells = line.rstrip("\n").split("\t")
            if len(cells) <= sha_idx:
                continue
            sha = cells[sha_idx]
            try:
                bucket = sha_bucket(sha)
            except ValueError:
                # Skip rows with malformed SHA
                continue
            if bucket == 0:
                fcal.write(line)
                n_cal += 1
            else:
                fmeas.write(line)
                n_meas += 1
    return n_cal, n_meas


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument(
        "--input",
        type=Path,
        default=Path("eval/gittables/failure_log.tsv"),
    )
    p.add_argument(
        "--calibrate",
        type=Path,
        default=Path("eval/gittables/failure_log.calibrate.tsv"),
    )
    p.add_argument(
        "--measure",
        type=Path,
        default=Path("eval/gittables/failure_log.measure.tsv"),
    )
    args = p.parse_args()

    n_cal, n_meas = split(args.input, args.calibrate, args.measure)
    total = n_cal + n_meas
    print(
        f"split: {total} rows -> calibrate={n_cal} measure={n_meas}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
