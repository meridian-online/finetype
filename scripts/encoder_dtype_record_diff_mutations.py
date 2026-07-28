#!/usr/bin/env python3
"""Mutation harness for `encoder_dtype_record_diff.py`.

A self-test that passes on a broken implementation is a structural guard, and
this repository has collected fifteen instances of exactly that.  So the claim
"the cases bite" is not made, it is run: each mutation below rewrites the tool
into a *plausible* wrong implementation — the one a competent author would
actually write — and the harness reports which named cases die.

A mutation that kills no case is a coverage gap, and the run exits non-zero.

    scripts/encoder_dtype_record_diff_mutations.py
    scripts/encoder_dtype_record_diff_mutations.py --only label-only-comparison

Each entry is (name, why it is realistic, old, new).  `old` must appear exactly
once in the source or the mutation is refused — a silently-missed substitution
would otherwise look like a passing mutation with no surviving cases.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

TOOL = Path(__file__).resolve().parent / "encoder_dtype_record_diff.py"

MUTATIONS: list[tuple[str, str, str, str]] = [
    (
        "label-only-comparison",
        "The exact defect that got two pull requests refused: diff the label and "
        "call it a whole-record result.",
        'COMPARED_FIELDS = list(PROFILE_FIELDS)',
        'COMPARED_FIELDS = ["type"]',
    ),
    (
        "positional-row-matching",
        "zip(a, b) is the obvious way to walk two record sets and is correct "
        "whenever nothing was dropped, so it survives every happy-path case.",
        """    a_by = {_key(r): r for r in a_rows}
    b_by = {_key(r): r for r in b_rows}
    only_a = sorted(k for k in a_by if k not in b_by)
    only_b = sorted(k for k in b_by if k not in a_by)
    common = sorted(k for k in a_by if k in b_by)""",
        """    n = min(len(a_rows), len(b_rows))
    a_by = {_key(a_rows[i]): a_rows[i] for i in range(n)}
    b_by = {_key(a_rows[i]): b_rows[i] for i in range(n)}
    only_a = []
    only_b = []
    common = sorted(_key(a_rows[i]) for i in range(n))""",
    ),
    (
        "skip-empty-fields",
        "'Nothing on one side, nothing to compare' reads like defensiveness and "
        "silently excuses a regression that stops emitting a field at all.",
        """            va, vb = ra.get(f, ""), rb.get(f, "")
            if va == vb:
                continue""",
        """            va, vb = ra.get(f, ""), rb.get(f, "")
            if va == vb or not va or not vb:
                continue""",
    ),
    (
        "naive-csv-split",
        "str.split(',') is what you write before remembering that format_string "
        "can be '#,##0.00'.",
        """    reader = csv.reader(io.StringIO(stdout))
    rows = [r for r in reader if r]""",
        """    rows = [
        [c.strip('"') for c in line.split(",")]
        for line in stdout.splitlines()
        if line.strip()
    ]""",
    ),
    (
        "max-delta-is-last-delta",
        "Assigning instead of comparing is a one-character slip and reports a "
        "plausible-looking magnitude on every input.",
        """                if d > max_abs[f]:
                    max_abs[f] = d
                    max_abs_at[f] = f"{k[1]} ({va} -> {vb})\"""",
        """                if True:
                    max_abs[f] = d
                    max_abs_at[f] = f"{k[1]} ({va} -> {vb})\"""",
    ),
    (
        "first-row-not-named-row",
        "A single-column CSV has one profile row, so taking rows[1] works for "
        "every column this tool actually profiles — until it does not.",
        """    for rec in data:
        if rec["column"] == column_name:
            return rec""",
        """    if data:
        return data[0]""",
    ),
    (
        "no-header-validation",
        "Trusting the CLI's column order is the default assumption; the failure "
        "it hides is every field shifted one to the left.",
        """    if header != PROFILE_FIELDS:
        raise ProfileError(
            f"profile CSV header changed: expected {PROFILE_FIELDS}, got {header}"
        )""",
        """    _ = header""",
    ),
    (
        "writer-drops-unknown-fields",
        "Writing a fixed short field list is how the label-only file was born in "
        "the first place.",
        """RECORD_FIELDS = ["file_content_sha256", "column_name"] + PROFILE_FIELDS""",
        """RECORD_FIELDS = ["file_content_sha256", "column_name", "column", "type"]""",
    ),
    (
        "single-row-fallback-always-fires",
        "Dropping the length guard makes the sniffer workaround swallow a "
        "genuinely missing column and return an unrelated row.",
        """    if len(data) == 1:
        return data[0]""",
        """    if data:
        return data[0]""",
    ),
    (
        "unstable-marker-dropped",
        "The [NOT run-to-run stable] marker looks cosmetic; without it a locale "
        "flip reads as an effect of the change under test.",
        """        if f in noise:
            note = (note + "  " if note else "") + "[NOT run-to-run stable]\"""",
        """        if f in noise:
            note = note""",
    ),
    (
        "whole-record-counts-fields-not-rows",
        "Summing the per-field counts is the natural 'total differences' number "
        "and equals the row count whenever exactly one field moves per row.",
        """        if row_moved:
            whole_record_differ += 1""",
        """        whole_record_differ += sum(
            1 for f in COMPARED_FIELDS if ra.get(f, "") != rb.get(f, "")
        )""",
    ),
]


def run_self_test(source: str) -> tuple[int, str]:
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "mutant.py"
        p.write_text(source)
        proc = subprocess.run(
            [sys.executable, str(p), "self-test"], capture_output=True, text=True
        )
    return proc.returncode, proc.stdout + proc.stderr


def failing_cases(out: str) -> set[str]:
    return {
        m.group(1).strip()
        for m in re.finditer(r"^  FAIL (.+)$", out, flags=re.MULTILINE)
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--only", help="run a single mutation by name")
    args = ap.parse_args()

    base = TOOL.read_text()
    rc, out = run_self_test(base)
    if rc != 0:
        print("the UNMUTATED tool does not pass its own self-test; fix that first")
        print(out)
        return 2
    total_cases = out.count("  ok  ")
    print(f"baseline: {total_cases} cases pass\n")

    survivors: list[str] = []
    for name, why, old, new in MUTATIONS:
        if args.only and name != args.only:
            continue
        n = base.count(old)
        if n != 1:
            print(f"{name}: REFUSED — anchor appears {n} times, expected 1")
            survivors.append(name)
            continue
        rc, out = run_self_test(base.replace(old, new))
        killed = failing_cases(out)
        if rc == 0 or not killed:
            print(f"{name}: SURVIVED — no case detected it")
            print(f"    why it is realistic: {why}")
            survivors.append(name)
            continue
        print(f"{name}: killed by {len(killed)} case(s)")
        for c in sorted(killed)[:6]:
            print(f"    - {c}")
        if len(killed) > 6:
            print(f"    ... and {len(killed) - 6} more")

    print()
    if survivors:
        print(f"{len(survivors)} SURVIVING mutation(s): {', '.join(survivors)}")
        return 1
    print("every mutation was killed by at least one named case")
    return 0


if __name__ == "__main__":
    sys.exit(main())
