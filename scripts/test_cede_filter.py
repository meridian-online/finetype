#!/usr/bin/env python3
"""Fixture test for the cede-leaf training filter (spec 2026-06-27-model-label-space-reshape ac-1).

Proves filter_ceded_groups removes ceded leaves as TARGETS while preserving sibling
context, and that the committed labels/ceded_leaves.txt is well-formed and guardrail-clean.
Run: python3 scripts/test_cede_filter.py
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from prepare_multibranch_data import filter_ceded_groups, load_cede_labels  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def _rec(label, idx):
    return {"label": label, "column_index": idx, "char": [], "embed": [],
            "stats": [], "header": [], "validation": []}


def test_filter_drops_targets_keeps_context():
    cede = {"technology.internet.uuid_CEDE", "datetime.date.iso_CEDE"}
    groups = [
        # mixed group: one keep + two cede → keep survives, siblings preserved
        {"sibling_headers": ["city", "id", "created"],
         "records": [_rec("geography.location.city", 0),
                     _rec("technology.internet.uuid_CEDE", 1),
                     _rec("datetime.date.iso_CEDE", 2)]},
        # all-cede group → dropped entirely
        {"sibling_headers": ["a", "b"],
         "records": [_rec("technology.internet.uuid_CEDE", 0),
                     _rec("datetime.date.iso_CEDE", 1)]},
        # no-cede group → untouched
        {"sibling_headers": ["x", "y"],
         "records": [_rec("representation.numeric.integer_number", 0),
                     _rec("representation.text.word", 1)]},
    ]
    out, stats = filter_ceded_groups(groups, cede)

    assert stats["ceded_records"] == 4, stats
    assert stats["dropped_groups"] == 1, stats          # the all-cede group
    assert stats["labels_hit"] == cede, stats
    assert len(out) == 2, "all-cede group must be dropped"

    # surviving group keeps the city target AND the full sibling_headers (ceded cols as context)
    g0 = out[0]
    assert [r["label"] for r in g0["records"]] == ["geography.location.city"]
    assert g0["sibling_headers"] == ["city", "id", "created"], "sibling context must survive"
    assert g0["records"][0]["column_index"] == 0, "column_index must be preserved"

    # round-trip: no ceded leaf survives anywhere
    surviving = {r["label"] for g in out for r in g["records"]} & cede
    assert not surviving, surviving
    print("ok: filter drops targets, preserves sibling context, drops empty groups")


def test_empty_cede_is_noop():
    groups = [{"sibling_headers": ["a"], "records": [_rec("x", 0)]}]
    out, stats = filter_ceded_groups(groups, set())
    assert out is groups and stats["ceded_records"] == 0
    print("ok: empty cede set is a no-op (every existing retrain unchanged)")


def test_committed_cede_file():
    cede = load_cede_labels(REPO / "labels" / "ceded_leaves.txt")
    assert len(cede) == 134, f"expected 134 ceded leaves, got {len(cede)}"
    # guardrail 1: no open-vocab leaf may be ceded
    open_vocab = {"city", "entity_name", "username", "numeric_code", "full_name",
                  "first_name", "last_name", "region", "country", "street_name",
                  "password", "hostname", "measurement_unit"}
    leaked = {leaf for leaf in cede if leaf.rsplit(".", 1)[-1] in open_vocab}
    assert not leaked, f"guardrail-1 breach — open-vocab leaves ceded: {leaked}"
    # the 4 thesis over-emitters MUST be ceded
    for leaf in ("identity.commerce.isbn", "finance.currency.currency_code",
                 "technology.internet.user_agent", "representation.numeric.si_number"):
        assert leaf in cede, f"over-emitter not ceded: {leaf}"
    print(f"ok: committed cede file = {len(cede)} leaves, guardrail-1 clean, over-emitters ceded")


if __name__ == "__main__":
    test_filter_drops_targets_keeps_context()
    test_empty_cede_is_noop()
    test_committed_cede_file()
    print("\nALL PASS")
