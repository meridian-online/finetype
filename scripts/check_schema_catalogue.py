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
import sys
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

    if failed:
        print(f"\nself-test FAILED: {failed} of {len(cases)} mutations not detected")
        return 1
    print(f"\nself-test passed: {len(cases)} mutations detected, control clean")
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
