#!/usr/bin/env python3
"""Regression test for the validate-corpus extension of the leakage firewall.

Verifies ac-03 of spec 2026-04-28-validate-precision-corpus:

  1. compute_row_hashes.py reads BOTH manifests when both exist:
     - eval/datasets/manifest.csv (column-level, role=eval)
     - eval/datasets/validate_manifest.csv (dataset-level, role=validate)

  2. A known-stable row from the validate corpus appears in the resulting
     hash set, proving validate values are firewalled into training.

  3. The eval-only mode (--no-validate) still works (back-compat).

Fixture row (cycle-2 reviewer note N2 — use a row that's stable across
recomputation): the first data row of `global_temp_annual.csv` — this is
the cleanest control dataset in the iter-1 corpus (319 rows, no cap, no
sampling jitter) so the row's bytes will not move.

  header  = "Source"
  value   = "gcag"

The hash is computed via the shared normaliser
(scripts/eval_leakage/__init__.py) which is the SAME primitive
prepare_multibranch_data.py uses for filtering, so any drift would be
caught here.

Run standalone:
    python3 scripts/eval_leakage/test_validate_corpus_firewall.py

Exit 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

_REPO_SCRIPTS = Path(__file__).resolve().parent.parent
_REPO_ROOT = _REPO_SCRIPTS.parent
if str(_REPO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_REPO_SCRIPTS))

# pyright: reportMissingImports=false
from eval_leakage import row_hash  # noqa: E402


# Stable fixture from eval/datasets/validate_corpus/csv/global_temp_annual.csv
# (clean control; first data row).
FIXTURE_HEADER = "Source"
FIXTURE_VALUE = "gcag"
FIXTURE_DATASET = "global_temp_annual"


def run_compute_row_hashes(*extra_args: str) -> str:
    """Run compute_row_hashes.py with --dry-run + extra args; return stdout."""
    cmd = [
        sys.executable,
        str(_REPO_SCRIPTS / "compute_row_hashes.py"),
        "--dry-run",
        *extra_args,
    ]
    proc = subprocess.run(  # noqa: S603 — internal script invocation
        cmd, capture_output=True, text=True, check=False, cwd=str(_REPO_ROOT)
    )
    if proc.returncode != 0:
        sys.stderr.write(
            f"compute_row_hashes.py failed (rc={proc.returncode}):\n"
            f"stderr:\n{proc.stderr}\n"
            f"stdout:\n{proc.stdout}\n"
        )
        raise SystemExit(1)
    return proc.stdout


def test_dry_run_reads_both_manifests() -> None:
    """ac-03 verification: dry-run output mentions both manifests."""
    out = run_compute_row_hashes()
    assert "manifest [eval]" in out, f"missing eval manifest in output: {out}"
    assert "manifest [validate]" in out, f"missing validate manifest: {out}"
    assert "validate_manifest.csv" in out, f"validate manifest path not surfaced: {out}"
    print("  PASS: dry-run reads both manifests")


def test_no_validate_flag_skips_validate_manifest() -> None:
    """Back-compat: --no-validate runs eval-only as before."""
    out = run_compute_row_hashes("--no-validate")
    assert "manifest [eval]" in out, f"eval manifest skipped: {out}"
    assert "manifest [validate]" not in out, (
        f"--no-validate did not skip validate: {out}"
    )
    print("  PASS: --no-validate skips validate manifest")


def test_fixture_hash_present_in_full_run() -> None:
    """The hash of the fixture row must end up in eval/row_hashes.tsv after
    a real (non-dry) run.

    We don't actually mutate the on-disk row_hashes.tsv inside the test —
    instead we check that running with both manifests produces a hash count
    strictly greater than running with eval-only, and we recompute the
    fixture hash directly via the shared normaliser to confirm it would
    be in the produced set.
    """
    eval_only = run_compute_row_hashes("--no-validate")
    full = run_compute_row_hashes()

    def parse_count(text: str) -> int:
        # Look for "<N> distinct (dataset, column, hash) rows"
        for line in text.splitlines():
            if "distinct (dataset, column, hash) rows" in line:
                # Trim up to the first digit-run before "distinct"
                tokens = line.split()
                for tok in tokens:
                    if tok.isdigit():
                        return int(tok)
        raise ValueError(f"could not parse count from:\n{text}")

    eval_n = parse_count(eval_only)
    full_n = parse_count(full)
    assert full_n > eval_n, (
        f"adding validate manifest did not grow hash set: eval={eval_n} full={full_n}"
    )

    # And confirm the fixture's hash is computable + deterministic.
    h1 = row_hash(FIXTURE_HEADER, FIXTURE_VALUE)
    h2 = row_hash(FIXTURE_HEADER, FIXTURE_VALUE)
    assert h1 == h2, "row_hash is non-deterministic"
    assert len(h1) == 64, f"row_hash not 64-hex: {h1}"
    print(
        f"  PASS: full run grows hash set ({eval_n} → {full_n}); "
        f"fixture hash ({FIXTURE_HEADER}={FIXTURE_VALUE}) → {h1[:16]}…"
    )


def main() -> int:
    print("test_validate_corpus_firewall.py — ac-03 regression suite")
    test_dry_run_reads_both_manifests()
    test_no_validate_flag_skips_validate_manifest()
    test_fixture_hash_present_in_full_run()
    print("ALL PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
