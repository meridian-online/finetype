#!/usr/bin/env python3
"""Build the standing representative-data eval fixture from the 2026-06-17 study.

Card 0020 scenario 1 / spec 2026-06-18-representative-accuracy-gate (ac-01).

The representative baseline (output/representative-baseline/finding.md) was a
one-shot study: a deterministic uniform-random draw of non-trivial columns from
the gittables corpus, blind-Sonnet-panel labelled. The draw script was ephemeral
and not committed, so — exactly like the gold corpus — the FIXTURE itself is the
canonical record of membership. This script reconstructs that fixture from the two
persisted study artifacts:

  - output/representative-baseline/panel_labels.json   (idx -> label, confidence, runner_up)
  - output/representative-baseline/repr_predictions.tsv (idx -> sha, column_name)

Output mirrors the gold corpus schema (eval/gold/gold_corpus.tsv) so the EXISTING
score_gold_anchor.py predict/score reads it unchanged. Sample values are NOT stored
(sanitised, like gold); the scorer pulls sample_values_truncated from columns.parquet
by (file_content_sha256, column_name) at predict time.

TRUTH TIER (ac-00): panel tier (`repr-panel-sonnet`), no author tier. The cited
adjudicated 0.68 is not reproducible per-column (the 8 panel-wrong flips were never
localised), so the fixture freezes the raw panel labels and says so in provenance.
"""
import argparse
import csv
import json
from pathlib import Path

REPR = Path("output/representative-baseline")
TAXONOMY_VERSION = "v0.6.33"  # the study's binary; reframe-era taxonomy
LABELLER = "repr-panel-sonnet"

GOLD_FIELDS = [
    "family", "file_path", "file_content_sha256", "column_name",
    "curated_label", "ydf_context", "sense_context", "labeller", "provenance",
]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default="eval/repr/representative_corpus.tsv")
    args = ap.parse_args()

    panel = {x["idx"]: x for x in json.loads((REPR / "panel_labels.json").read_text())}
    coords = {}
    with open(REPR / "repr_predictions.tsv") as f:
        for r in csv.DictReader(f, delimiter="\t"):
            coords[int(r["idx"])] = (r["sha"], r["column_name"])

    idxs = sorted(set(panel) & set(coords))
    rows = []
    for i in idxs:
        sha, col = coords[i]
        p = panel[i]
        conf = p.get("confidence", "")
        runner = p.get("runner_up", "")
        prov = (
            f"{LABELLER} 2026-06-17 (blind Sonnet panel from values+header; "
            f"confidence={conf}"
            + (f"; runner_up={runner}" if runner else "")
            + "; uniform-random non-trivial corpus draw; "
            f"panel tier — adjudicated flips not persisted, see "
            f"output/representative-accuracy-gate/ac00_tier_and_band_policy.md); "
            f"taxonomy {TAXONOMY_VERSION}"
        )
        rows.append({
            "family": "representative",
            "file_path": "",  # sanitised in the study; sha is the join key
            "file_content_sha256": sha,
            "column_name": col,
            "curated_label": p["label"],
            "ydf_context": "",
            "sense_context": "",
            "labeller": LABELLER,
            "provenance": prov,
        })

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=GOLD_FIELDS, delimiter="\t")
        w.writeheader()
        w.writerows(rows)
    print(f"wrote {len(rows)} columns -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
