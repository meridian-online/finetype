#!/usr/bin/env python3
"""Planted-collision unit test for the eval_leakage normaliser.

Verifies that pairs the spec says should collide (identical after
normalisation) DO collide, and pairs the spec says should NOT collide
stay distinct. This is the test whose existence is required by:

  - spec 2026-04-21-eval-expansion, ac-06 + ac-07
  - MADR 0056 §Consequences (criterion for `proposed → accepted`)

Run standalone:
    python scripts/eval_leakage/test_normaliser.py

Run as pytest:
    pytest scripts/eval_leakage/test_normaliser.py

Exit code 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import sys
from pathlib import Path

# Make the parent `scripts/` directory importable so `from eval_leakage import …`
# works whether the test is invoked via pytest from the repo root, directly,
# or from a subdirectory. This keeps the test self-contained and keeps
# pyright happy without requiring a conftest.py.
_REPO_SCRIPTS = Path(__file__).resolve().parent.parent
if str(_REPO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_REPO_SCRIPTS))

# pyright: reportMissingImports=false
# The import below resolves at runtime via the sys.path manipulation above.
# Pyright can't statically infer dynamic sys.path modifications; silencing is
# the idiomatic fix.
from eval_leakage import (  # noqa: E402
    NORMALISER_VERSION,
    normalise_header,
    normalise_value,
    row_hash,
)


# ---------------------------------------------------------------------------
# Test fixtures — planted collisions
# ---------------------------------------------------------------------------

# Pairs that MUST collide (hash equal). Each is (header_a, value_a, header_b, value_b, rationale).
COLLIDE: list[tuple[str, str, str, str, str]] = [
    ("email", "alice@example.com", "email", "alice@example.com", "identical"),
    ("Email", "alice@example.com", "email", "alice@example.com", "header case-fold"),
    (" email ", "alice@example.com", "email", "alice@example.com", "header whitespace strip"),
    ("email", " alice@example.com ", "email", "alice@example.com", "value whitespace strip"),
    ("email", "alice@example.com\r\n", "email", "alice@example.com", "CRLF strip"),
    ("email", "alice@example.com\n", "email", "alice@example.com", "trailing LF strip"),
    # NFC: pre-composed é == decomposed e + ◌́ after normalisation
    ("name", "café", "name", "cafe\u0301", "NFC decomposed → composed"),
    # Internal CRLF collapses to LF — but stays internal, still part of the value
    ("multiline", "a\r\nb", "multiline", "a\nb", "CRLF → LF internal"),
]

# Pairs that MUST NOT collide (hash different). Each is (header_a, value_a, header_b, value_b, rationale).
DISTINCT: list[tuple[str, str, str, str, str]] = [
    ("email", "alice@example.com", "email", "bob@example.com", "different value"),
    ("email", "alice@example.com", "user_email", "alice@example.com", "different header"),
    ("email", "alice@example.com", "e-mail", "alice@example.com", "header synonym NOT caught (blind spot)"),
    ("phone", "555-1234", "phone", "555 1234", "internal whitespace NOT caught (blind spot)"),
    ("amount", "1,000.00", "amount", "1000.00", "number format drift NOT caught (blind spot)"),
    # Cross-key boundary: a value containing NUL should not collide with a
    # different (header, value) split that yields the same bytes. The 0x00
    # separator makes this a no-op because 0x00 cannot appear in the
    # stripped+NFC value (control chars aren't stripped, but they don't
    # appear in normal data and if they did, they'd carry through on both
    # sides of the comparison). This test just confirms header/value
    # identity isn't confused.
    ("a", "bc", "ab", "c", "header/value boundary"),
]


# ---------------------------------------------------------------------------
# Assertion helpers
# ---------------------------------------------------------------------------


def _check_collide(h1: str, v1: str, h2: str, v2: str, reason: str) -> tuple[bool, str]:
    r1 = row_hash(h1, v1)
    r2 = row_hash(h2, v2)
    if r1 == r2:
        return True, f"PASS  collide: {reason!r}  {r1[:12]}…"
    return (
        False,
        f"FAIL  collide: {reason!r}\n"
        f"      ({h1!r}, {v1!r}) → {r1}\n"
        f"      ({h2!r}, {v2!r}) → {r2}\n"
        f"      normalised: ({normalise_header(h1)!r}, {normalise_value(v1)!r}) vs "
        f"({normalise_header(h2)!r}, {normalise_value(v2)!r})",
    )


def _check_distinct(h1: str, v1: str, h2: str, v2: str, reason: str) -> tuple[bool, str]:
    r1 = row_hash(h1, v1)
    r2 = row_hash(h2, v2)
    if r1 != r2:
        return True, f"PASS  distinct: {reason!r}"
    return (
        False,
        f"FAIL  distinct: {reason!r} — both hashed to {r1}\n"
        f"      ({h1!r}, {v1!r}) vs ({h2!r}, {v2!r})",
    )


# ---------------------------------------------------------------------------
# Pytest-style tests (run via pytest if available)
# ---------------------------------------------------------------------------


def test_evalx_collisions_all_planted_pairs_collide() -> None:
    failures: list[str] = []
    for h1, v1, h2, v2, reason in COLLIDE:
        ok, msg = _check_collide(h1, v1, h2, v2, reason)
        if not ok:
            failures.append(msg)
    assert not failures, "Planted collisions failed:\n" + "\n".join(failures)


def test_evalx_distinctions_all_planted_pairs_stay_distinct() -> None:
    failures: list[str] = []
    for h1, v1, h2, v2, reason in DISTINCT:
        ok, msg = _check_distinct(h1, v1, h2, v2, reason)
        if not ok:
            failures.append(msg)
    assert not failures, "Planted distinctions failed:\n" + "\n".join(failures)


def test_evalx_normaliser_version_is_pinned() -> None:
    assert NORMALISER_VERSION == "1.0.0", (
        f"NORMALISER_VERSION changed to {NORMALISER_VERSION!r}; "
        "bumping the version requires regenerating eval/row_hashes.tsv "
        "(see MADR 0056 §Normalisation spec)."
    )


def test_evalx_row_hash_is_deterministic() -> None:
    # Same inputs → same output across invocations
    a = row_hash("email", "alice@example.com")
    b = row_hash("email", "alice@example.com")
    assert a == b, f"row_hash is non-deterministic: {a} != {b}"
    # Different casing of header → same output (case-fold)
    c = row_hash("EMAIL", "alice@example.com")
    assert a == c, f"header case-fold failed: {a} != {c}"


# ---------------------------------------------------------------------------
# CLI entry point (runnable standalone without pytest)
# ---------------------------------------------------------------------------


def main() -> int:
    print(f"eval_leakage planted-collision test (NORMALISER_VERSION = {NORMALISER_VERSION})")
    print(f"  {len(COLLIDE)} collision pairs, {len(DISTINCT)} distinction pairs")
    print()

    total = 0
    passed = 0
    failures: list[str] = []

    for h1, v1, h2, v2, reason in COLLIDE:
        total += 1
        ok, msg = _check_collide(h1, v1, h2, v2, reason)
        if ok:
            passed += 1
        else:
            failures.append(msg)
        print(f"  {msg.splitlines()[0]}")

    for h1, v1, h2, v2, reason in DISTINCT:
        total += 1
        ok, msg = _check_distinct(h1, v1, h2, v2, reason)
        if ok:
            passed += 1
        else:
            failures.append(msg)
        print(f"  {msg.splitlines()[0]}")

    print()
    print(f"{passed}/{total} passed")
    if failures:
        print()
        print("Failures:")
        for f in failures:
            print(f)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
