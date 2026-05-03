#!/usr/bin/env python3
"""Cron preamble + postamble tests — bead finetype-nms.

Covers:
  - ac-01: preamble exits 1 on lock-present (H04)
  - ac-02: preamble exits 1 on disk < 20GB (mocked df)
  - ac-03: postamble removes lockfile; subsequent preamble succeeds
  - ac-04: cycle_id propagated through preamble JSON
  - ac-05: preamble + postamble combined < 2s

Run standalone:
    python3 scripts/eval_leakage/test_cron_preamble.py
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from tempfile import TemporaryDirectory

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PREAMBLE = REPO_ROOT / "scripts" / "cron_preamble.sh"
POSTAMBLE = REPO_ROOT / "scripts" / "cron_postamble.sh"


def _env(td: Path, *, lockfile: Path | None = None, df_cmd: str | None = None) -> dict:
    env = {**os.environ}
    env["FINETYPE_REPO_ROOT"] = str(td)
    env["FINETYPE_LOCKFILE"] = str(lockfile) if lockfile else str(td / "lock")
    if df_cmd is not None:
        env["FINETYPE_DISK_CMD"] = df_cmd
    return env


def _ensure_contract(td: Path) -> None:
    """Symlink the real contract into the test repo root so preamble finds it."""
    contracts = td / "orbit" / "contracts"
    contracts.mkdir(parents=True, exist_ok=True)
    src = REPO_ROOT / "orbit" / "contracts" / "2026-05-03-gittables-90-percent-roundtrip.yaml"
    dst = contracts / "2026-05-03-gittables-90-percent-roundtrip.yaml"
    if not dst.exists():
        dst.symlink_to(src)


def test_ac01_lock_present_halts() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        _ensure_contract(td)
        lockfile = td / "lock"
        lockfile.write_text('{"cycle_id":"prior","started":"2026-05-03T10:00:00Z","pid":1}\n')

        res = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile),
            capture_output=True, text=True,
        )
        assert res.returncode == 1, f"expected 1 (halt), got {res.returncode}: {res.stderr}"
        payload = json.loads(res.stdout)
        assert payload["status"] == "halt"
        assert payload["halt_id"] == "H04"
        # Lock not removed (preamble doesn't clobber another cycle's lock).
        assert lockfile.exists()


def test_ac02_low_disk_halts() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        _ensure_contract(td)
        lockfile = td / "lock"

        # Mock df: produce two-line output with 5G available (< 20G floor).
        df_mock = td / "df_mock.sh"
        df_mock.write_text(
            "#!/bin/sh\n"
            "echo 'Filesystem 1G-blocks Used Available Capacity'\n"
            "echo '/dev/disk1   500       100  5         95%'\n"
        )
        df_mock.chmod(0o755)

        res = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile, df_cmd=str(df_mock)),
            capture_output=True, text=True,
        )
        assert res.returncode == 1, f"expected 1 (halt), got {res.returncode}: {res.stderr}"
        payload = json.loads(res.stdout)
        assert payload["status"] == "halt"
        assert payload["halt_id"] == "H01"
        # Lock IS present (preamble created it before disk check).
        assert lockfile.exists()


def test_ac01_ac03_ac04_happy_path_then_postamble_releases() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        _ensure_contract(td)
        lockfile = td / "lock"

        df_mock = td / "df_mock.sh"
        df_mock.write_text(
            "#!/bin/sh\n"
            "echo 'Filesystem 1G-blocks Used Available Capacity'\n"
            "echo '/dev/disk1   500       100  400       25%'\n"
        )
        df_mock.chmod(0o755)

        # First run — succeeds, creates lock, returns cycle_id.
        res1 = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile, df_cmd=str(df_mock)),
            capture_output=True, text=True,
        )
        assert res1.returncode == 0, f"unexpected halt: {res1.stdout}\n{res1.stderr}"
        p1 = json.loads(res1.stdout)
        assert p1["status"] == "ok"
        assert p1["cycle_id"]  # ac-04: present and non-empty
        assert p1["free_disk_gb_start"] == 400
        assert lockfile.exists()

        # Second run with same lockfile — H04.
        res_h04 = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile, df_cmd=str(df_mock)),
            capture_output=True, text=True,
        )
        assert res_h04.returncode == 1
        assert json.loads(res_h04.stdout)["halt_id"] == "H04"

        # Postamble releases.
        res_post = subprocess.run(
            [str(POSTAMBLE), p1["cycle_id"]],
            env=_env(td, lockfile=lockfile),
            capture_output=True, text=True,
        )
        assert res_post.returncode == 0, f"postamble failed: {res_post.stderr}"
        assert not lockfile.exists()

        # Third run — succeeds again (ac-03: postamble unblocks).
        res2 = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile, df_cmd=str(df_mock)),
            capture_output=True, text=True,
        )
        assert res2.returncode == 0, f"second cycle blocked: {res2.stdout}\n{res2.stderr}"
        p2 = json.loads(res2.stdout)
        assert p2["cycle_id"] != p1["cycle_id"], "cycle_id should be fresh per cycle"


def test_postamble_refuses_mismatched_cycle_id() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        lockfile = td / "lock"
        lockfile.write_text('{"cycle_id":"abc","started":"2026-05-03T10:00:00Z","pid":1}\n')

        res = subprocess.run(
            [str(POSTAMBLE), "different-cycle-id"],
            env={**os.environ, "FINETYPE_LOCKFILE": str(lockfile)},
            capture_output=True, text=True,
        )
        assert res.returncode == 1, f"expected 1, got {res.returncode}"
        assert lockfile.exists(), "postamble should NOT remove a foreign cycle's lock"


def test_ac05_preamble_postamble_under_2s() -> None:
    with TemporaryDirectory() as td_:
        td = Path(td_)
        _ensure_contract(td)
        lockfile = td / "lock"
        df_mock = td / "df_mock.sh"
        df_mock.write_text(
            "#!/bin/sh\n"
            "echo 'h'\n"
            "echo 'd 500 100 400 25%'\n"
        )
        df_mock.chmod(0o755)

        t0 = time.perf_counter()
        res = subprocess.run(
            [str(PREAMBLE)],
            env=_env(td, lockfile=lockfile, df_cmd=str(df_mock)),
            capture_output=True, text=True,
        )
        assert res.returncode == 0
        cycle_id = json.loads(res.stdout)["cycle_id"]

        res2 = subprocess.run(
            [str(POSTAMBLE), cycle_id],
            env=_env(td, lockfile=lockfile),
            capture_output=True, text=True,
        )
        assert res2.returncode == 0
        elapsed = time.perf_counter() - t0
        assert elapsed < 2.0, f"preamble+postamble took {elapsed:.2f}s (>2s budget)"


def main() -> int:
    tests = [
        ("ac-01 lock-present halts (H04)", test_ac01_lock_present_halts),
        ("ac-02 low-disk halts (H01)", test_ac02_low_disk_halts),
        ("ac-01/03/04 happy path + postamble release",
         test_ac01_ac03_ac04_happy_path_then_postamble_releases),
        ("postamble refuses foreign cycle_id", test_postamble_refuses_mismatched_cycle_id),
        ("ac-05 preamble+postamble <2s", test_ac05_preamble_postamble_under_2s),
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
