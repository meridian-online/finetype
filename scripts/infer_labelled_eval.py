#!/usr/bin/env python3
"""
infer_labelled_eval.py — run `finetype infer-type` on each row of
labelled_eval.unlabelled.tsv and write predictions to a separate file
(NOT joined onto the labelled subset, per labelling_protocol.md
anti-pattern: "Do not consult the inference module's output for any
row before assigning truth_inferred_type").

Output:
  orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.module_predictions.tsv

After hand-labelling produces labelled_eval.tsv, run
compute_precision_on_labelled.py to join + report.
"""

from __future__ import annotations

import argparse
import csv
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_INPUT = (
    REPO_ROOT
    / "orbit"
    / "specs"
    / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.unlabelled.tsv"
)
DEFAULT_OUTPUT = (
    REPO_ROOT
    / "orbit"
    / "specs"
    / "2026-05-04-autonomous-type-inference"
    / "labelled_eval.module_predictions.tsv"
)
DEFAULT_BIN = REPO_ROOT / "target" / "release" / "finetype"

OUT_COLS = [
    "cycle_id",
    "file_path",
    "file_content_sha256",
    "column_name",
    "predicted_type",
    "module_inferred_type",
    "module_confidence",
    "module_mechanism",
    "module_validator_pass_rate",
    "module_header_match",
]


def parse_observed_sample(s: str) -> list[str]:
    if not s:
        return []
    try:
        v = json.loads(s)
        if isinstance(v, list):
            return [str(x) for x in v]
    except json.JSONDecodeError:
        pass
    return [s]


def run_one(binary: Path, column_name: str, predicted_type: str, samples: list[str]) -> dict:
    payload = json.dumps(
        {
            "column_name": column_name,
            "predicted_type": predicted_type,
            "samples": samples,
        }
    )
    r = subprocess.run(
        [str(binary), "infer-type"],
        input=payload,
        capture_output=True,
        text=True,
        check=False,
    )
    if r.returncode != 0:
        raise RuntimeError(f"infer-type failed: {r.stderr.strip()}")
    return json.loads(r.stdout)


def main(input_path: Path, output_path: Path, binary: Path) -> int:
    with input_path.open("r", encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f, delimiter="\t"))

    print(f"running inference on {len(rows)} rows...")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8", newline="") as f:
        w = csv.DictWriter(f, fieldnames=OUT_COLS, delimiter="\t", lineterminator="\n")
        w.writeheader()

        for i, r in enumerate(rows, 1):
            samples = parse_observed_sample(r.get("observed_values_sample", ""))
            try:
                out = run_one(
                    binary,
                    r.get("column_name", ""),
                    r.get("predicted_type", ""),
                    samples,
                )
            except Exception as e:
                print(f"  row {i}: error {e}", file=sys.stderr)
                continue

            sig = out.get("signals", {})
            w.writerow(
                {
                    "cycle_id": r["cycle_id"],
                    "file_path": r["file_path"],
                    "file_content_sha256": r["file_content_sha256"],
                    "column_name": r["column_name"],
                    "predicted_type": r["predicted_type"],
                    "module_inferred_type": out.get("inferred_correct_type", ""),
                    "module_confidence": f'{out.get("confidence", 0.0):.4f}',
                    "module_mechanism": out.get("mechanism", ""),
                    "module_validator_pass_rate": f'{sig.get("validator_pass_rate", 0.0):.4f}',
                    "module_header_match": f'{sig.get("header_match", 0.0):.4f}',
                }
            )
            if i % 25 == 0:
                print(f"  ... {i}/{len(rows)}")

    print(f"wrote {output_path}")
    return 0


if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    p.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    p.add_argument("--finetype-bin", type=Path, default=DEFAULT_BIN)
    args = p.parse_args()
    sys.exit(main(args.input, args.output, args.finetype_bin))
