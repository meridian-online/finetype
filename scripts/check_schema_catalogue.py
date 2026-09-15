#!/usr/bin/env python3
"""Gate the generated taxonomy schema catalogue against the shape its one
consumer actually reads.

`brightfield_engine::semantic::parse_schema_catalogue` (in the sibling
`brightfield` repository) reads the catalogue as a JSON array. For each object
it takes `x-finetype-label` as the label, and treats an entry as *checkable*
when the object carries, at its top level, any of exactly six keywords:
`pattern`, `enum`, `minLength`, `maxLength`, `minimum`, `maximum`. It refuses a
catalogue where no entry is both labelled and checkable.

This reads the GENERATED FILE, not the generator's exit code. `finetype
taxonomy -o json-schema` can exit 0 and still write something the consumer
refuses — an empty array, or an array whose every entry lost its validation
keywords — and checking the exit code alone would not catch that. This does,
because it opens the file `finetype taxonomy` wrote and applies the same
predicate the consumer applies.

WHAT IS NOT ASSERTED
    That every entry is labelled and checkable, or any count beyond "at least
    one" — that is the consumer's own bar (`brightfield_engine::semantic::
    parse_schema_catalogue`), reproduced here so a release-time defect is
    caught before the asset ships rather than after a downstream install.
    Coverage of the shipped model's label set is a SEPARATE property, checked
    by `scripts/check_model_coverage.py`.

USAGE
    scripts/check_schema_catalogue.py <path/to/catalogue.json>
    scripts/check_schema_catalogue.py --self-test

EXIT CODES
    0  at least one entry carries both `x-finetype-label` and a checkable
       keyword
    1  the file parses as JSON but the catalogue does not qualify — not an
       array, or zero qualifying entries
    2  the check could not run: bad usage, missing file, or invalid JSON

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

CHECKABLE_KEYWORDS = ("pattern", "enum", "minLength", "maxLength", "minimum", "maximum")


def check(catalogue: object) -> list[str]:
    """Return failure strings for a parsed catalogue value (empty = pass)."""
    if not isinstance(catalogue, list):
        return [f"top-level value is a {type(catalogue).__name__}, not a JSON array"]

    total = len(catalogue)
    labelled = 0
    checkable = 0
    qualifying = 0
    for entry in catalogue:
        if not isinstance(entry, dict):
            continue
        label = entry.get("x-finetype-label")
        has_label = isinstance(label, str) and label != ""
        has_checkable = any(k in entry for k in CHECKABLE_KEYWORDS)
        labelled += has_label
        checkable += has_checkable
        qualifying += has_label and has_checkable

    if qualifying == 0:
        return [
            "no entry carries both `x-finetype-label` and one of "
            f"{list(CHECKABLE_KEYWORDS)} "
            f"({total} entries, {labelled} labelled, {checkable} checkable, 0 both)"
        ]
    return []


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def _run_cli(argv: list[str]) -> tuple[int, str]:
    """Run THIS file as a real subprocess and return (exit code, combined output).

    The in-process cases below prove `check` returns the right verdict. They
    say nothing about whether the PROGRAM acts on it: replacing `main`'s
    `failures = check(catalogue)` with `failures = []` leaves every one of them
    green while the gate exits 0 over an emptied catalogue -- the exact defect
    this file exists to catch, shipped past a green CI job. The verdict and the
    exit code are two different altitudes and only the second is what CI reads,
    so these cases go through argv, `main` and `sys.exit`.
    """
    proc = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *argv],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, proc.stdout + proc.stderr


def _exit_code_cases() -> int:
    """Return the number of exit-code cases that did not behave as stated."""
    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        qualifying = root / "qualifying.json"
        qualifying.write_text(
            json.dumps([{"x-finetype-label": "identity.person.email", "pattern": "^.+@.+$"}]),
            encoding="utf-8",
        )
        emptied = root / "emptied.json"
        emptied.write_text("[]", encoding="utf-8")
        as_object = root / "object.json"
        as_object.write_text('{"identity.person.email": {"pattern": "^.+@.+$"}}', encoding="utf-8")
        unlabelled = root / "unlabelled.json"
        unlabelled.write_text('[{"pattern": "^.+@.+$"}]', encoding="utf-8")
        not_json = root / "not-json.json"
        not_json.write_text("this is not JSON", encoding="utf-8")

        # Exact codes, never "non-zero": exit 1 means the catalogue was read
        # and refused, exit 2 means the check could not run. A defect that
        # swaps one for the other refuses the right input for the wrong reason,
        # and "non-zero" cannot tell them apart.
        cases: list[tuple[str, list[str], int]] = [
            ("a qualifying catalogue exits 0", [str(qualifying)], 0),
            ("a deliberately emptied catalogue exits 1", [str(emptied)], 1),
            ("a top-level object rather than an array exits 1", [str(as_object)], 1),
            ("entries with a checkable keyword but no label exit 1", [str(unlabelled)], 1),
            ("a file that is not JSON exits 2, not 1", [str(not_json)], 2),
            ("a catalogue path that does not exist exits 2, not 1", [str(root / "absent.json")], 2),
            ("no catalogue path at all exits 2, not 0", [], 2),
        ]
        for name, argv, expected in cases:
            code, output = _run_cli(argv)
            if code != expected:
                print(f"  MISS {name}: exited {code}")
                last = output.strip().splitlines()
                print(f"      last line of output: {last[-1] if last else '<none>'}")
                failed += 1
            else:
                print(f"  ok   {name}")
    return failed


def self_test() -> int:
    good = [
        {"x-finetype-label": "identity.person.email", "pattern": "^.+@.+$"},
        {"x-finetype-label": "container.array.comma_separated"},
    ]
    control = check(good)
    if control:
        print("  CONTROL FAILED — a plausible good catalogue does not pass:")
        for failure in control:
            print(f"      {failure}")
        return 1
    print("  ok   control: a labelled, checkable catalogue passes")

    cases: list[tuple[str, object, str]] = [
        (
            "a deliberately emptied catalogue",
            [],
            "no entry carries both",
        ),
        (
            "every entry loses its checkable keyword (label survives)",
            [
                {"x-finetype-label": "identity.person.email"},
                {"x-finetype-label": "container.array.comma_separated"},
            ],
            "no entry carries both",
        ),
        (
            "every entry loses its label (checkable keyword survives)",
            [
                {"pattern": "^.+@.+$"},
                {"enum": ["a", "b"]},
            ],
            "no entry carries both",
        ),
        (
            "the top-level value is an object, not an array",
            {"identity.person.email": {"pattern": "^.+@.+$"}},
            "not a JSON array",
        ),
        (
            "the top-level value is a bare string",
            "not a catalogue",
            "not a JSON array",
        ),
    ]

    failed = 0
    for name, mutated, expected in cases:
        found = check(mutated)
        text = "\n".join(found)
        if not found:
            print(f"  MISS {name}: mutation survived")
            failed += 1
        elif expected not in text:
            print(f"  WRONG {name}: caught, but not for the stated reason")
            print(f"      expected to see: {expected}")
            print(f"      got: {text}")
            failed += 1
        else:
            print(f"  ok   {name}")

    # The verdict cases above end here; what follows drives the same mutations
    # through the process boundary CI actually reads.
    failed += _exit_code_cases()

    if failed:
        print(f"\nself-test FAILED: {failed} case(s) not detected correctly")
        return 1
    print(f"\nself-test passed: {len(cases)} verdict mutations detected, control clean, exit codes pinned")
    return 0


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("catalogue", nargs="?", type=Path, help="path to the generated catalogue JSON")
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    if args.catalogue is None:
        parser.error("catalogue path is required unless --self-test is given")

    try:
        text = args.catalogue.read_text(encoding="utf-8")
    except OSError as exc:
        print(f"error: could not read {args.catalogue}: {exc}", file=sys.stderr)
        return 2

    try:
        catalogue = json.loads(text)
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON in {args.catalogue}: {exc}", file=sys.stderr)
        return 2

    failures = check(catalogue)
    if failures:
        print(f"schema catalogue {args.catalogue} does not qualify:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(f"schema catalogue {args.catalogue} qualifies (>= 1 labelled, checkable entry)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
