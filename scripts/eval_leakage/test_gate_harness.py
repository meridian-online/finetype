#!/usr/bin/env python3
"""Reproducibility test for gittables_gate.py — bead finetype-e6d ac-05.

Runs the harness twice on the same small holdout and asserts that:
  - gate_score is identical
  - per-file (path, passed, n_cols, non_trivial_cols, total_rows,
    rejects_non_trivial) is identical

The gate is reproducible by construction (frozen holdout + fixed model
symlink + deterministic profile/validate). This test pins the
construction — H06 implausible-improvement detection depends on it.

Run standalone:
    python3 scripts/eval_leakage/test_gate_harness.py
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
GATE_SCRIPT = REPO_ROOT / "scripts" / "gittables_gate.py"
GITTABLES = Path("/Users/hugh/datasets/gittables")


def _build_holdout(td: Path, n: int = 3) -> Path:
    holdout = td / "holdout.txt"
    cluster = GITTABLES / "abstraction"
    if not cluster.exists():
        # Skip on environments without the dataset rather than fail.
        raise SystemExit(0)
    paths = sorted(cluster.glob("*.parquet"))[:n]
    if len(paths) < n:
        raise SystemExit(0)
    holdout.write_text("\n".join(str(p) for p in paths) + "\n")
    return holdout


def _run(holdout: Path) -> dict:
    res = subprocess.run(
        [
            sys.executable, str(GATE_SCRIPT),
            "--holdout", str(holdout),
            "--quiet",
        ],
        capture_output=True, text=True, timeout=600,
    )
    if res.returncode != 0:
        raise AssertionError(
            f"gate harness rc={res.returncode}\nstderr:\n{res.stderr}"
        )
    return json.loads(res.stdout)


def _stable_view(summary: dict) -> dict:
    """Project summary down to fields that MUST be reproducible.

    Excludes elapsed_s (timing varies) and harness_dirty (depends on
    on-disk state during the test run).
    """
    keep_top = ("gate_score", "files_passed", "files_total", "files_errored",
                "model_sha", "model_tag")
    keep_per_file = ("path", "passed", "n_cols", "non_trivial_cols",
                     "non_trivial_pct", "total_rows", "rejects_non_trivial",
                     "reject_rate_non_trivial", "error")
    out = {k: summary[k] for k in keep_top if k in summary}
    if "per_file" in summary:
        out["per_file"] = [
            {k: row.get(k) for k in keep_per_file}
            for row in summary["per_file"]
        ]
    return out


def test_e6d_ac05_byte_identical_score() -> None:
    with TemporaryDirectory() as td:
        holdout = _build_holdout(Path(td))
        a = _run(holdout)
        b = _run(holdout)
        va = _stable_view(a)
        vb = _stable_view(b)
        assert va == vb, (
            "gate harness not reproducible:\n"
            f"first:  {json.dumps(va, indent=2)}\n"
            f"second: {json.dumps(vb, indent=2)}"
        )


def main() -> int:
    try:
        test_e6d_ac05_byte_identical_score()
        print("PASS  e6d ac-05 reproducibility")
        return 0
    except AssertionError as exc:
        print(f"FAIL  e6d ac-05 reproducibility: {exc}")
        return 1


if __name__ == "__main__":
    sys.exit(main())
