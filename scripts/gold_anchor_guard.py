#!/usr/bin/env python3
"""Leakage guard for the gold eval anchor — keyed on COLUMN IDENTITY.

Spec 2026-06-05-gold-eval-anchor ac-06.

The existing train_ydf.py exclusion (`_value_hash` over a column's sampled
value tuple, scripts/train_ydf.py) is WINDOW-SENSITIVE: re-sample the same
column with a different window and the hash changes, so the column slips past
the filter. A gold column must be excluded from any training/mining corpus
regardless of how it is later sampled — so this guard keys on the durable
(file_content_sha256, column_name) identity the fixture carries (ac-03), not on
the value tuple.

This is the mechanical half of the independence contract (ac-01): a gold column
can never become a training label for the lens it is meant to judge. ac-07 is
the deferred counterpart — auditing the same identity set against the B2
harvested corpus once that corpus exists.

Used by:
  - scripts/train_ydf.py        (exclusion path, alongside labelled_eval)
  - scripts/audit_gold_anchor_leakage.py  (the standing audit)
  - scripts/test_gold_anchor_guard.py     (the ac-06 test)
"""
from __future__ import annotations

import csv
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_GOLD = REPO / "eval" / "gold" / "gold_eval_anchor.tsv"
# Gold-corpus fixtures (spec 2026-06-10-human-verified-gold-corpus ac-05):
# every CANDIDATE column is excluded from training — not just adjudicated
# gold — so a column can never be both a training example and a column the
# author later verifies. Missing fixtures are skipped (pre-corpus checkouts).
GOLD_CORPUS_FIXTURES = (
    REPO / "eval" / "gold" / "gold_corpus_candidates.tsv",
    REPO / "eval" / "gold" / "gold_corpus_candidates_external.tsv",
)

Identity = tuple[str, str]  # (file_content_sha256, column_name)


def load_gold_identities(path: Path = DEFAULT_GOLD,
                         include_corpus: bool = True) -> set[Identity]:
    """The (file_content_sha256, column_name) identity of every gold column.

    These are the columns excluded from training/mining so the gold anchor
    stays independent of the lens it scores. Rows missing either identity
    component are dropped (the fixture should carry both for every row).
    By default the gold-corpus candidate fixtures are included alongside
    the anchor, so every consumer (train_ydf exclusion, the standing audit)
    covers the full corpus without per-caller changes."""
    ids: set[Identity] = set()
    paths = [path]
    if include_corpus:
        paths += [p for p in GOLD_CORPUS_FIXTURES if p != path]
    for p in paths:
        if not p.exists():
            continue
        with p.open() as fh:
            for r in csv.DictReader(fh, delimiter="\t"):
                sha = (r.get("file_content_sha256") or "").strip()
                col = (r.get("column_name") or "").strip()
                if sha and col:
                    ids.add((sha, col))
    return ids


def is_gold_column(sha: str | None, col: str | None, gold: set[Identity]) -> bool:
    """True iff (sha, col) is a gold-anchor column and must be excluded.

    A training row with no (file, column) provenance (None/empty) can never be
    a gold column, so it is never excluded — the guard only fires on rows that
    actually carry the GitTables identity the fixture keys on."""
    if not sha or not col:
        return False
    return (sha, col) in gold


def partition_gold(
    rows: list[dict], gold: set[Identity],
    sha_key: str = "file_content_sha256", col_key: str = "column_name",
) -> tuple[list[dict], list[dict]]:
    """Split rows into (kept, excluded) by gold identity. Each row is inspected
    for its (sha_key, col_key) fields; rows lacking them are kept."""
    kept: list[dict] = []
    excluded: list[dict] = []
    for row in rows:
        if is_gold_column(row.get(sha_key), row.get(col_key), gold):
            excluded.append(row)
        else:
            kept.append(row)
    return kept, excluded
