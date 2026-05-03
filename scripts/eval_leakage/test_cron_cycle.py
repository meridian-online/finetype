#!/usr/bin/env python3
"""End-to-end cron cycle wiring test — bead finetype-53r.

Covers:
  - ac-02: dry-run invocation completes a full cycle (preamble +
    measurement + postamble) without an active REPL
  - ac-03: lockfile interaction prevents concurrent firing — pre-acquire
    the lock, verify the cycle script halts and logs H04, lock is left
    in place for the human to inspect

Run standalone:
    python3 scripts/eval_leakage/test_cron_cycle.py
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
CRON_CYCLE = REPO_ROOT / "scripts" / "cron_cycle.sh"


def test_ac02_dry_run_completes_end_to_end() -> None:
    """No active REPL — just shells out to the script."""
    # Use a private lockfile so we don't touch /tmp/finetype-cron.lock.
    with TemporaryDirectory() as td_:
        td = Path(td_)
        env = {**os.environ, "FINETYPE_LOCKFILE": str(td / "lock")}
        # Pre-clean.
        lock = td / "lock"
        if lock.exists():
            lock.unlink()

        res = subprocess.run(
            [str(CRON_CYCLE), "--dry-run"],
            env=env,
            cwd=str(REPO_ROOT),
            capture_output=True, text=True,
            timeout=120,
        )
        assert res.returncode == 0, (
            f"dry-run failed (rc={res.returncode})\n"
            f"stdout:\n{res.stdout}\nstderr:\n{res.stderr}"
        )
        # Lock removed by postamble.
        assert not lock.exists(), "postamble did not release lock"


def test_ac03_pre_acquired_lock_halts_with_h04() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        lock = td / "lock"
        # Pre-acquire the lock with a foreign cycle_id.
        lock.write_text(
            '{"cycle_id":"prior-foreign-cycle","started":"2026-05-03T01:00:00Z","pid":1}\n'
        )
        env = {**os.environ, "FINETYPE_LOCKFILE": str(lock)}

        res = subprocess.run(
            [str(CRON_CYCLE), "--dry-run"],
            env=env,
            cwd=str(REPO_ROOT),
            capture_output=True, text=True,
            timeout=30,
        )
        assert res.returncode == 1, (
            f"expected halt (rc=1), got rc={res.returncode}\n"
            f"stdout:\n{res.stdout}\nstderr:\n{res.stderr}"
        )
        assert "H04" in res.stderr, (
            f"expected H04 in stderr, got:\n{res.stderr}"
        )
        # Lock still present — preamble does not clobber a foreign lock.
        assert lock.exists(), "preamble removed a foreign cycle's lock"
        # Lock content unchanged — postamble did not run.
        assert "prior-foreign-cycle" in lock.read_text()


def main() -> int:
    tests = [
        ("ac-02 dry-run end-to-end", test_ac02_dry_run_completes_end_to_end),
        ("ac-03 pre-acquired lock halts H04", test_ac03_pre_acquired_lock_halts_with_h04),
    ]
    failed: list[str] = []
    for name, fn in tests:
        try:
            fn()
            print(f"  PASS  {name}")
        except AssertionError as exc:
            failed.append(f"{name}: {exc}")
            print(f"  FAIL  {name}: {exc}")
        except Exception as exc:  # noqa: BLE001
            failed.append(f"{name}: {type(exc).__name__}: {exc}")
            print(f"  FAIL  {name}: {type(exc).__name__}: {exc}")
    print()
    print(f"{len(tests) - len(failed)}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
