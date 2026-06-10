#!/usr/bin/env python3
"""Rank the gold adjudication queue to the author's 350-column budget (ac-03/04).

The lens pass (gold_lens_prelabel.py) queues every column that lacks unanimous
two-lens consensus — 1,083 of 1,271 on the v1 corpus, which is the expected
shape: the model_emit buckets are adversarial by construction. The signed
sizing memo caps the author's sitting at 350 columns and says Tier-2 trims
first, so this stage selects WHICH 350 the author sees:

  stratum order   tier1 round-robin, then tier2, then backbone
  within stratum  disagreement > reference_decisive > single_lens > all_abstain
                  (disagreements are the most informative verdicts), external
                  rows first within each class (few, high-information), then
                  header_pos/model_emit alternating for bucket balance
  per-stratum cap tier1 30, tier2 14, backbone 6 — then a global fill to 350

Unselected rows go to gold_unreviewed_backlog.tsv: still candidates, NOT gold.
Gold v1 = anchor (240) + lens consensus + adjudicated queue. Backlog rows can
be promoted by future sittings.

Usage: python3 scripts/gold_queue_prioritise.py
"""
from __future__ import annotations
import csv
from pathlib import Path

SRC = Path("eval/gold/gold_review_queue.tsv")
OUT_Q = Path("eval/gold/gold_review_queue_v1.tsv")
OUT_B = Path("eval/gold/gold_unreviewed_backlog.tsv")
BUDGET = 350
STRATUM_CAP = {"tier1": 30, "tier2": 14, "backbone": 6, "external": 30}
BASIS_RANK = {"disagreement": 0, "reference_decisive": 1, "single_lens": 2, "all_abstain": 3}
TIER_RANK = {"tier1": 0, "external": 0, "tier2": 1, "backbone": 2}


def main() -> int:
    rows = list(csv.DictReader(SRC.open(), delimiter="\t"))
    for i, r in enumerate(rows):
        r["_i"] = i

    def sort_key(r):
        return (
            BASIS_RANK.get(r["verdict_basis"], 9),
            0 if r["tier"] == "external" else 1,
            0 if r["bucket"] == "header_pos" else 1,
            r["_i"],
        )

    by_stratum: dict[tuple[str, str], list[dict]] = {}
    for r in rows:
        by_stratum.setdefault((r["tier"], r["stratum_label"]), []).append(r)
    for v in by_stratum.values():
        v.sort(key=sort_key)

    selected, sel_keys = [], set()
    # Pass 1: per-stratum caps, tier order.
    for tier_pass in ("tier1", "external", "tier2", "backbone"):
        for (tier, stratum), v in sorted(by_stratum.items(), key=lambda kv: kv[0][1]):
            if tier != tier_pass:
                continue
            cap = STRATUM_CAP[tier]
            take = [r for r in v[:cap] if len(selected) + 1 <= BUDGET]
            for r in take:
                if len(selected) >= BUDGET:
                    break
                selected.append(r)
                sel_keys.add(r["_i"])
    # Pass 2: global fill by informativeness if budget remains.
    if len(selected) < BUDGET:
        rest = sorted((r for r in rows if r["_i"] not in sel_keys),
                      key=lambda r: (BASIS_RANK.get(r["verdict_basis"], 9),
                                     TIER_RANK.get(r["tier"], 9), r["_i"]))
        for r in rest[:BUDGET - len(selected)]:
            selected.append(r)
            sel_keys.add(r["_i"])

    backlog = [r for r in rows if r["_i"] not in sel_keys]
    fields = [c for c in rows[0] if c != "_i"]
    for path, data in ((OUT_Q, selected), (OUT_B, backlog)):
        with path.open("w", newline="") as fh:
            w = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", extrasaction="ignore")
            w.writeheader()
            w.writerows(data)

    print(f"queue_v1 {len(selected)} (budget {BUDGET}), backlog {len(backlog)}")
    per = {}
    for r in selected:
        per[r["stratum_label"]] = per.get(r["stratum_label"], 0) + 1
    for k in sorted(per, key=lambda k: -per[k]):
        print(f"  {per[k]:3d}  {k}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
