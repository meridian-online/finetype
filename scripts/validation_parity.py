#!/usr/bin/env python3
"""ac-04 — Runtime/eval validation parity test.

The Rust runtime validator (`CompiledValidator`, reached here via the
hidden `finetype validate-value` subcommand) and the Python eval gate
(`apply_ydf_validation_gate.load_validations()[label].passes(value)`)
are two independent implementations of the same JSON-Schema validation.
They MUST agree per value, or the corpus false-veto sweep (ac-05) and
the profile veto (ac-06) measure something the runtime won't reproduce.

Historically they disagreed on exactly the ac-01 rescued values: the
eval gate skips length checks while the Rust validator enforces them, so
a value rescued by a widened pattern was still vetoed by a stale
minLength/maxLength on the Rust side only. ac-01 reconciled by widening
the co-located bounds; ac-04 also bounded hms_24h's fractional part to
`\\d{1,6}` so its pattern can't admit a string longer than maxLength
(the one residual length divergence).

This test pins a fixed fixture — the rescued values plus length/enum
edge cases that would expose a divergence — and asserts the two
implementations return IDENTICAL pass/fail, and that each matches the
expected outcome.

Run:
    uv run --with regex,pyyaml python3 scripts/validation_parity.py

Exit 0 = parity holds. Exit 1 = a divergence or a wrong expected value.
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BIN = REPO / "target/release/finetype"
LABELS = REPO / "labels"

# (label, value, expected_pass)
#
# Rescued values (ac-01/02/03) — must PASS on both sides:
#   the length/pattern widenings and the enum case-fold land here.
# Edge cases — must FAIL on both sides:
#   long-fractional hms_24h (length+pattern bound), 7-digit time.iso
#   fraction (pattern bound), enum non-members, and a non-year float.
FIXTURE: list[tuple[str, str, bool]] = [
    # ── rescued: PASS both ──────────────────────────────────────────
    ("datetime.time.iso", "00:07:13.580", True),
    ("datetime.time.hms_24h", "01:03:29.000", True),
    ("datetime.component.year", "1998.0", True),
    ("datetime.timestamp.rfc_3339", "2020-03-19 15:34:31 -0800", True),
    ("datetime.component.day_of_week", "thursday", True),
    ("identity.person.gender", "male", True),
    # enum case-fold also accepts upper/title forms
    ("datetime.component.day_of_week", "THURSDAY", True),
    ("identity.person.gender", "Female", True),
    ("identity.person.gender_code", "x", True),
    # ── edges: FAIL both (parity under length/pattern/enum) ─────────
    # length+pattern: 9 fractional digits exceeds maxLength 15 AND the
    # \d{1,6} bound — the residual divergence ac-04 closed.
    ("datetime.time.hms_24h", "01:03:29.123456789", False),
    # time.iso fraction capped at 9 digits (nanoseconds) by the pattern;
    # 10 digits exceeds both the pattern and maxLength 18.
    ("datetime.time.iso", "00:07:13.5800000000", False),
    # 7-digit fraction (.NET 100ns ticks) is now admitted.
    ("datetime.time.iso", "00:00:11.1525400", True),
    # single-digit hour is admitted for hms_24h.
    ("datetime.time.hms_24h", "8:00:00", True),
    # enum non-members (case-fold must not admit these).
    ("datetime.component.day_of_week", "thursdays", False),
    ("identity.person.gender", "masculine", False),
    # year out of the 1000–2100 range.
    ("datetime.component.year", "1998.5", False),
]


def rust_pass(label: str, value: str) -> bool:
    out = subprocess.run(
        [str(BIN), "validate-value", "-l", label, "-t", str(LABELS), value],
        capture_output=True, text=True,
    )
    if out.returncode != 0:
        raise SystemExit(
            f"finetype validate-value failed for {label!r} {value!r}:\n"
            f"{out.stderr.strip()}")
    verdict = out.stdout.strip()
    if verdict not in ("PASS", "FAIL"):
        raise SystemExit(
            f"unexpected validate-value output for {label!r}: {verdict!r}")
    return verdict == "PASS"


def gate_pass(specs, label: str, value: str) -> bool:
    spec = specs.get(label)
    if spec is None:
        raise SystemExit(f"eval gate has no spec for label {label!r}")
    return spec.passes(value)


def main() -> int:
    if not BIN.exists():
        print(f"error: build the CLI first — {BIN} missing "
              "(cargo build --release -p finetype-cli)", file=sys.stderr)
        return 2

    sys.path.insert(0, str(REPO / "scripts"))
    from apply_ydf_validation_gate import load_validations  # type: ignore

    specs = load_validations()

    failures: list[str] = []
    for label, value, expected in FIXTURE:
        r = rust_pass(label, value)
        g = gate_pass(specs, label, value)
        status = "ok" if (r == g == expected) else "MISMATCH"
        print(f"  [{status}] {label:34} {value!r:24} "
              f"rust={'PASS' if r else 'FAIL'} "
              f"gate={'PASS' if g else 'FAIL'} "
              f"expected={'PASS' if expected else 'FAIL'}")
        if r != g:
            failures.append(
                f"{label} {value!r}: runtime/gate DIVERGE "
                f"(rust={'PASS' if r else 'FAIL'}, "
                f"gate={'PASS' if g else 'FAIL'})")
        elif r != expected:
            failures.append(
                f"{label} {value!r}: both implementations returned "
                f"{'PASS' if r else 'FAIL'}, expected "
                f"{'PASS' if expected else 'FAIL'}")

    print()
    if failures:
        print(f"PARITY FAILED — {len(failures)} issue(s):", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print(f"PARITY OK — {len(FIXTURE)} fixture values agree across "
          "runtime validator and eval gate.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
