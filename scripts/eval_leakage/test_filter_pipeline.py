#!/usr/bin/env python3
"""Pipeline-level filter test for the eval_leakage row-hash firewall.

Plants K collisions in a mock training dict, runs the same filter loop
used by scripts/prepare_multibranch_data.py against a mock eval-hash
set, and asserts exactly K rows are removed and 0 non-colliding rows
are touched.

This is the ac-07 spec-verification equivalent called out by review-pr
cycle 2: the normaliser test in test_normaliser.py verifies the hash
primitive; this test verifies the pipeline's use of the primitive.

Run standalone:
    python3 scripts/eval_leakage/test_filter_pipeline.py

Exit 0 on all-pass, 1 on any failure.
"""

from __future__ import annotations

import sys
from pathlib import Path

_REPO_SCRIPTS = Path(__file__).resolve().parent.parent
if str(_REPO_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_REPO_SCRIPTS))

# pyright: reportMissingImports=false
from eval_leakage import row_hash  # noqa: E402


# ---------------------------------------------------------------------------
# Mock filter — mirrors the per-column loop in prepare_multibranch_data.py
# ---------------------------------------------------------------------------


def filter_column_against_eval(
    header: str,
    values: list[str],
    eval_hashes: set[str],
) -> tuple[list[str], int]:
    """Return (kept_values, removed_count) for one training column.

    Mirrors the per-value filter loop in prepare_multibranch_data.py.
    A value is removed iff row_hash(header, value) is in eval_hashes.
    """
    kept: list[str] = []
    removed = 0
    for v in values:
        if row_hash(header, v) in eval_hashes:
            removed += 1
            continue
        kept.append(v)
    return kept, removed


# ---------------------------------------------------------------------------
# Test scenarios
# ---------------------------------------------------------------------------


def test_evalx_planted_K_collisions_all_removed() -> None:
    """Plant K collisions; expect exactly K removed, rest untouched."""
    eval_rows = [
        ("email", "alice@example.com"),
        ("email", "bob@example.com"),
        ("phone", "555-1234"),
        ("username", "alice_42"),
        ("city", "Berlin"),
    ]
    eval_hashes = {row_hash(h, v) for h, v in eval_rows}
    assert len(eval_hashes) == len(eval_rows), "hash collision in test fixture"

    # Training column with K collisions + 5 non-colliding values
    training_values = [
        "alice@example.com",  # collision
        "bob@example.com",  # collision
        "carol@example.com",  # clean
        "dan@example.com",  # clean
        "eve@example.com",  # clean
        "frank@example.com",  # clean
        "george@example.com",  # clean
    ]
    kept, removed = filter_column_against_eval("email", training_values, eval_hashes)

    assert removed == 2, f"expected 2 email collisions removed, got {removed}"
    assert len(kept) == 5, f"expected 5 clean values kept, got {len(kept)}"
    assert "alice@example.com" not in kept
    assert "bob@example.com" not in kept
    assert "carol@example.com" in kept

    # Unused eval rows (phone/username/city) must not affect the email column
    for v in kept:
        assert row_hash("email", v) not in eval_hashes


def test_evalx_normalisation_collisions_caught_by_filter() -> None:
    """Eval row normalisation must match training row normalisation."""
    # Eval stores the pre-composed form; training has decomposed — must match
    eval_hashes = {row_hash("name", "café")}  # pre-composed é
    training_values = [
        "cafe\u0301",  # decomposed — MUST be filtered
        "cafe",  # different value — MUST be kept
        "CAFÉ",  # same value with different case — the normaliser does
                 #   NOT case-fold values (only headers); kept
    ]
    kept, removed = filter_column_against_eval("name", training_values, eval_hashes)
    assert removed == 1, f"expected 1 NFC collision removed, got {removed}"
    assert "cafe\u0301" not in kept
    assert "cafe" in kept
    assert "CAFÉ" in kept  # value case is preserved by spec


def test_evalx_no_false_positives_on_different_header() -> None:
    """A value that would collide under a different header must not be filtered."""
    eval_hashes = {row_hash("email", "alice@example.com")}
    training_values = ["alice@example.com"]
    kept, removed = filter_column_against_eval(
        "username",  # different header — must not collide
        training_values,
        eval_hashes,
    )
    assert removed == 0, f"expected no cross-header collision, got {removed} removed"
    assert kept == training_values


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> int:
    tests = [
        ("planted K collisions all removed", test_evalx_planted_K_collisions_all_removed),
        ("normalisation collisions caught", test_evalx_normalisation_collisions_caught_by_filter),
        ("no false positives on different header", test_evalx_no_false_positives_on_different_header),
    ]
    print("eval_leakage pipeline-filter test")
    print()
    failed = 0
    for name, fn in tests:
        try:
            fn()
            print(f"  PASS  {name}")
        except AssertionError as e:
            print(f"  FAIL  {name}: {e}")
            failed += 1
    print()
    print(f"{len(tests) - failed}/{len(tests)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
