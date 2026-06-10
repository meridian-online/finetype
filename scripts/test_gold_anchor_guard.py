#!/usr/bin/env python3
"""Tests for the gold-anchor leakage guard (spec 2026-06-05-gold-eval-anchor ac-06).

The load-bearing assertion is `test_zero_overlap_with_current_corpus`: zero
(file_content_sha256, column_name) overlap between the gold fixture and the
corpus train_ydf.py actually trains on. The guard is keyed on column IDENTITY,
not value-hash, so it holds regardless of how a column is later sampled — which
is the whole point of ac-06 (the value-hash filter is window-sensitive).

`test_guard_fires_on_injected_gold_row` proves the wiring is mechanical, not
decorative: a distilled-shaped row carrying a real gold identity is dropped by
train_ydf._load_distilled_columns even though it would otherwise be valid.

Run with:
    eval/gittables/.venv/bin/python -m unittest scripts.test_gold_anchor_guard
    # or, from repo root:
    python3 -m unittest scripts.test_gold_anchor_guard
"""
from __future__ import annotations

import gzip
import json
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))

from gold_anchor_guard import (  # noqa: E402
    DEFAULT_GOLD,
    is_gold_column,
    load_gold_identities,
    partition_gold,
)

DISTILLED = REPO / "output" / "distillation-v3" / "sherlock_distilled.csv.gz"


class LoadIdentitiesTests(unittest.TestCase):
    def test_fixture_loads_with_complete_identities(self):
        gold = load_gold_identities(DEFAULT_GOLD)
        self.assertGreater(len(gold), 0, "gold fixture loaded no identities")
        for sha, col in gold:
            self.assertTrue(sha, "empty file_content_sha256 in gold identity")
            self.assertTrue(col, "empty column_name in gold identity")

    def test_missing_fixture_returns_empty(self):
        self.assertEqual(
            load_gold_identities(REPO / "no" / "such.tsv", include_corpus=False),
            set())

    def test_corpus_fixtures_included_by_default(self):
        # spec 2026-06-10-human-verified-gold-corpus ac-05: the candidate
        # fixtures join the exclusion set without per-caller changes, even
        # when the explicit path is missing.
        anchor_only = load_gold_identities(include_corpus=False)
        full = load_gold_identities()
        self.assertGreater(len(full), len(anchor_only),
                           "gold-corpus candidate fixtures not folded in")
        bogus = load_gold_identities(REPO / "no" / "such.tsv")
        self.assertEqual(bogus, full - anchor_only)


class IsGoldColumnTests(unittest.TestCase):
    def setUp(self):
        self.gold = {("sha_abc", "msg_id"), ("sha_def", "TEAM_ABBREVIATION")}

    def test_positive(self):
        self.assertTrue(is_gold_column("sha_abc", "msg_id", self.gold))

    def test_negative_unknown_identity(self):
        self.assertFalse(is_gold_column("sha_abc", "other_col", self.gold))
        self.assertFalse(is_gold_column("sha_xyz", "msg_id", self.gold))

    def test_none_or_empty_never_fires(self):
        # A training row with no (file, column) provenance can never be gold.
        self.assertFalse(is_gold_column(None, "msg_id", self.gold))
        self.assertFalse(is_gold_column("sha_abc", None, self.gold))
        self.assertFalse(is_gold_column("", "", self.gold))

    def test_partition_splits_on_identity(self):
        rows = [
            {"file_content_sha256": "sha_abc", "column_name": "msg_id"},
            {"file_content_sha256": "sha_zzz", "column_name": "keep_me"},
            {"sherlock_index": "17"},  # no identity → kept
        ]
        kept, excluded = partition_gold(rows, self.gold)
        self.assertEqual(len(excluded), 1)
        self.assertEqual(excluded[0]["column_name"], "msg_id")
        self.assertEqual(len(kept), 2)


class CorpusOverlapTests(unittest.TestCase):
    """The ac-06 assertion: zero (file, column) overlap between the gold
    fixture and the current training corpus."""

    def _distilled_identities(self) -> set[tuple[str, str]]:
        ids: set[tuple[str, str]] = set()
        if not DISTILLED.exists():
            return ids
        import csv

        with gzip.open(DISTILLED, "rt") as fh:
            for row in csv.DictReader(fh):
                sha = (row.get("file_content_sha256") or "").strip()
                col = (row.get("column_name") or "").strip()
                if sha and col:
                    ids.add((sha, col))
        return ids

    def test_zero_overlap_with_current_corpus(self):
        gold = load_gold_identities(DEFAULT_GOLD)
        self.assertGreater(len(gold), 0)
        training = self._distilled_identities()
        overlap = gold & training
        self.assertEqual(
            overlap, set(),
            f"gold columns leaked into the training corpus: {sorted(overlap)}",
        )


class GuardWiringTests(unittest.TestCase):
    """Prove the train_ydf.py exclusion path actually drops a gold column."""

    def test_guard_fires_on_injected_gold_row(self):
        import train_ydf

        gold = load_gold_identities(DEFAULT_GOLD)
        self.assertGreater(len(gold), 0)
        victim_sha, victim_col = sorted(gold)[0]

        # Two distilled-shaped rows: one carries a gold identity (must be
        # dropped by identity), one is clean (must survive).
        rows = [
            {
                "final_label": "representation.identifier.alphanumeric_id",
                "sample_values": json.dumps(["a1", "b2", "c3"]),
                "file_content_sha256": victim_sha,
                "column_name": victim_col,
            },
            {
                "final_label": "representation.numeric.integer_number",
                "sample_values": json.dumps(["1", "2", "3"]),
                "file_content_sha256": "not_a_gold_sha",
                "column_name": "not_a_gold_col",
            },
        ]
        with tempfile.TemporaryDirectory() as td:
            csv_gz = Path(td) / "mini_distilled.csv.gz"
            import csv as _csv

            with gzip.open(csv_gz, "wt", newline="") as fh:
                w = _csv.DictWriter(
                    fh,
                    fieldnames=[
                        "final_label", "sample_values",
                        "file_content_sha256", "column_name",
                    ],
                )
                w.writeheader()
                for r in rows:
                    w.writerow(r)

            cols, _n_hash_excluded, n_gold_excluded = (
                train_ydf._load_distilled_columns(csv_gz, gold_ids=gold)
            )

        self.assertEqual(n_gold_excluded, 1, "gold row was not excluded")
        labels = [c[0] for c in cols]
        self.assertIn("representation.numeric.integer_number", labels)
        self.assertNotIn(
            "representation.identifier.alphanumeric_id", labels,
            "gold-identity row survived the exclusion path",
        )


if __name__ == "__main__":
    unittest.main()
