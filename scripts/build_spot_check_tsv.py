#!/usr/bin/env python3
"""ac-12 spot-check as a TSV — one row per sample-evidence record.

The markdown spot_check.md is structured for narrative review;
this TSV is structured for visual scanning in a spreadsheet. Each
row carries the actual sample values inline so you can sort by cell,
filter by mechanism, and form a verdict at a glance.

Columns:
  - cell                    criterion × mechanism (concatenated)
  - criterion
  - mechanism
  - gap_id_short            first 12 chars of gap_id
  - file_short              basename of file_path
  - column_name             gittables column
  - sense_prediction        Sense's call
  - ydf_prediction          YDF lens's call
  - ydf_confidence          YDF confidence
  - recommended_action      cascade → action mapping
  - dbpedia                 DBpedia annotation if present
  - sample_values           up to 5 sample values joined with " | "
  - n_samples_total         how many samples were truncated to
  - prescreen_verdict       my pre-screen call (PASS/FAIL/—)
  - prescreen_reason        one-line reason

Each gap can have up to 5 sample evidence rows (ac-09 cap), so 22 gaps
yield up to 110 rows. The pre-screen verdicts are denormalised: the
verdict applies to the gap; every row of a gap carries the same
verdict for filtering convenience.

USAGE
    python3 scripts/build_spot_check_tsv.py
"""
from __future__ import annotations

import argparse
import csv
import json
import random
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SAMPLING_SEED = 20260520
SAMPLES_PER_CELL = 3
MAX_VALUES_PER_ROW = 5
VALUE_TRUNC = 80

# Pre-screen verdicts and reasons, keyed by gap_id (full).
# Source: eval/gittables/corpus_pass/spot_check_prescreen.md (committed
# 2026-05-23). The attestor remains authoritative — these are loaded
# as defaults; override in the TSV directly.
PRESCREEN: dict[str, tuple[str, str]] = {
    # non_trivial_floor × format_diversity_path_b
    "a57a3a01a7a680485f4206d68bf7cf063f51f17099b5ba77effc9059ecb93365":
        ("PASS", "sentence-shaped text (fill-in-the-blank prompts); format_diversity_path_b defensible"),
    "0c2f31b51309d0faa998691926d5b1afdc489b805d0dcc38ced3aff744e97667":
        ("FAIL", "YDF entity_name doesn't fit email body content; cascade token format_diversity_path_b also off — should be misclassification or fallthrough"),
    "e0eb6890ae08060426de5965621dba73eab998509593650a807923c4a6e1e36a":
        ("PASS", "MTG card rules text; sentence-shaped, format_diversity_path_b OK"),
    # non_trivial_floor × misclassification
    "63db51c7e35b3164a95e9dfcf50d24bfec071efb9d9a07be167c1f0a491d96c7":
        ("PASS", "textbook misclassification: column literally Title, content is title, DBpedia confirms"),
    "08593c1937d3089dfbdee4630ede36eba262520fc6d34c9896b92f1cf293a914":
        ("PASS", "CS-term lists, not sentences; misclassification claim holds even if YDF's sentence read is off"),
    "b4128af448881d541d374e6c280fcfcc9417e9d1e67e55adeef107f495ab3a82":
        ("PASS", "English narrative excerpts; sentence-shaped; misclassification defensible"),
    # reject_rate_ceil × code_vs_canonical_path_a
    "839ed8e4221b6057d2f59457ba0b7ce4888e3c3596a1e378f0f7c6536a10d47a":
        ("FAIL", "atom labels (C1', C2, ...) are a fixed enum; no code/canonical duality — token should be enum_overfit"),
    # reject_rate_ceil × enum_overfit
    "cdf4571a30e4eb2c4592080e1cdebe6bfe432e9def25d081debbc68b12cb3712":
        ("PASS", "mass units (kg, g, mg) — fixed enum, Sense's measurement_unit overfit"),
    "211c7d391798b5e26ce6f36cc384fbdfcc00a39239fff8353aacabe4e1e19ee7":
        ("PASS", "literal value 'boolean' isn't a boolean term; enum_overfit fits"),
    "ecd83415422201be3008b87a241eff9c68a39b09f9182aff05a39b43c29cac06":
        ("PASS", "heterogeneous unit categorical (mass + time); enum_overfit fits"),
    # reject_rate_ceil × format_diversity_path_a
    "7925dc9e13e9691e16cdfbbb09edcf7c8301c54f0ccd34c3c6c5a617a0e595e6":
        ("PASS", "Stripe IDs (cus_*) — alphanumeric_id validator likely fails on the underscore; widening fits"),
    "2453b24e2e5eb6a34ac8a9cfba1df2193747c9fe9a68be9abb4b60b939e051c1":
        ("PASS", "date column with mixed 'TRUE' value; format_diversity_path_a defensible"),
    "cd8ec93a478017266f0d29f30e92cfd6534e265bb31d2c2b57f12b50b230220c":
        ("FAIL", "WEIGHT(%) column with -90, 100; Sense's person.weight is fundamentally wrong, not format-diverse — token should be misclassification"),
    # reject_rate_ceil × format_diversity_path_b
    "7555f3869e52c6b59d5b95c3eec56746e21d575b4aa1c075f951a0fd09e1328a":
        ("PASS", "World region (e.g. OECD90) — Sense's continent is too narrow; format_diversity_path_b fits"),
    "0a86df7d326343afbe16bc794090bd4e6864f2b7178585a7be7ec1e1619b35f5":
        ("PASS", "ADDRESS ZIP column actually contains full addresses; postal_code → full_address is textbook"),
    "ddb54a6cdf58d424e65e0b46d0f7431c2d034b586caea4c9b71df13392bbaefb":
        ("PASS", "same shape as 0a86df7d — ADDRESS ZIP with full addresses"),
    # reject_rate_ceil × misclassification
    "66010133df53e20aab2e686688433a64024f78277ce33e8c7445309277233483":
        ("PASS", "data is corrupt/misaligned (loses column with Reuters/Inc); misclassification is closest available token"),
    "0860c99acb10938deada953a9445f98f7a3db1153073e81a491f0ff2b250ec86":
        ("PASS", "Namespace column with .NET namespaces — code identifiers, no taxonomy fit; misclassification → training_data_addition"),
    "b863f8172da1150c0b797f11ac00514549d596cc9d4c5c3ced966b6a7bde96aa":
        ("PASS", "biology paper author abbreviations; misclassification defensible (could also be format_diversity_path_a)"),
    # reject_rate_ceil × validator_widening
    "75550339ad4baf715cca5ef40c15f35718a823a6c55592e980e7af8fe2ca1f40":
        ("FAIL", "EMAIL column containing full addresses — widening email validator can't make addresses valid emails; token should be misclassification or unknown_no_fit"),
    "099beb68241d4b9b0d90c44b8cf6cebb25554633f78248669f8ccf6501d376c7":
        ("FAIL", "identical shape to 75550339 — EMAIL with full addresses; same token mismatch"),
    "dda85e6d07f2661e833d53e9126e6e9aba582736e500d55847f44bc403460f33":
        ("FAIL", "URL column with bare integers; URL validator can't be widened to accept integers — token should be misclassification"),
}

SAMPLE_SEPARATOR = "│"  # U+2502 — corpus pass's sample join


def trunc(s: str, n: int = VALUE_TRUNC) -> str:
    s = (s or "").replace("\n", " ").replace("\r", " ").replace("\t", " ").strip()
    return s if len(s) <= n else s[: n - 1] + "…"


def split_samples(s: str | None) -> list[str]:
    if not s:
        return []
    return [p for p in s.split(SAMPLE_SEPARATOR) if p]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--corroborated", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet")
    p.add_argument("--dbpedia-annotations", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/dbpedia_annotations.parquet")
    p.add_argument("--out", type=Path,
                   default=REPO / "eval/gittables/corpus_pass/spot_check.tsv")
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
    except ImportError as exc:  # noqa: BLE001
        print(f"error: duckdb missing ({exc})", file=sys.stderr)
        return 2

    con = duckdb.connect()
    gaps = con.execute(f"""
        SELECT *
        FROM read_parquet('{args.corroborated}')
        ORDER BY criterion, mechanism, gap_id
    """).fetch_arrow_table().to_pylist()

    dbp_rows = con.execute(f"""
        SELECT file_path, column_name, dbpedia_semantic_class
        FROM read_parquet('{args.dbpedia_annotations}')
        WHERE dbpedia_semantic_class IS NOT NULL
              AND dbpedia_semantic_class != ''
    """).fetch_arrow_table().to_pylist()
    dbp_lookup = {
        (r["file_path"], r["column_name"]): r["dbpedia_semantic_class"]
        for r in dbp_rows
    }

    # Same sampling as build_spot_check.py
    from collections import defaultdict
    cells: dict[tuple, list[dict]] = defaultdict(list)
    for g in gaps:
        cells[(g["criterion"], g["mechanism"])].append(g)
    sampled: list[dict] = []
    for cell_key, cell_gaps in sorted(cells.items()):
        rng = random.Random(SAMPLING_SEED)
        if len(cell_gaps) <= SAMPLES_PER_CELL:
            sampled.extend(cell_gaps)
        else:
            sampled.extend(rng.sample(cell_gaps, SAMPLES_PER_CELL))

    fields = [
        "cell", "criterion", "mechanism",
        "gap_id_short", "gap_id_full",
        "file_short", "column_name",
        "sense_prediction", "ydf_prediction", "ydf_confidence",
        "recommended_action", "dbpedia",
        "sample_values", "n_samples_total",
        "prescreen_verdict", "prescreen_reason",
        "your_verdict", "your_reason",
    ]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=fields, delimiter="\t")
        w.writeheader()

        for g in sampled:
            gid_full = g["gap_id"]
            gid_short = gid_full[:12]
            criterion = g["criterion"]
            mechanism = g["mechanism"]
            cell = f"{criterion} × {mechanism}"
            action = g["recommended_action_class"]
            lenses = {l["lens_name"]: l for l in (g["corroborating_lenses"] or [])}
            ydf = lenses.get("ydf") or {}
            ydf_pred = ydf.get("prediction_or_annotation") or ""
            ydf_conf = ydf.get("confidence")
            ydf_conf_str = f"{ydf_conf:.2f}" if ydf_conf is not None else ""

            pre_v, pre_r = PRESCREEN.get(gid_full, ("", ""))

            for s in (g["sample_evidence"] or []):
                fp = s["file_path"]
                cn = s["column_name"]
                file_short = Path(fp).name
                sense = s.get("sense_prediction") or ""
                samples = s.get("sample_values") or []
                n_total = len(samples)
                shown = samples[:MAX_VALUES_PER_ROW]
                samples_joined = " | ".join(trunc(v) for v in shown)
                dbp = dbp_lookup.get((fp, cn), "")
                w.writerow({
                    "cell": cell,
                    "criterion": criterion,
                    "mechanism": mechanism,
                    "gap_id_short": gid_short,
                    "gap_id_full": gid_full,
                    "file_short": file_short,
                    "column_name": cn,
                    "sense_prediction": sense,
                    "ydf_prediction": ydf_pred,
                    "ydf_confidence": ydf_conf_str,
                    "recommended_action": action,
                    "dbpedia": dbp,
                    "sample_values": samples_joined,
                    "n_samples_total": str(n_total),
                    "prescreen_verdict": pre_v,
                    "prescreen_reason": pre_r,
                    "your_verdict": "",
                    "your_reason": "",
                })

    # Summary
    n_rows = sum(1 for _ in args.out.open()) - 1
    print(json.dumps({
        "n_gaps_sampled": len(sampled),
        "n_tsv_rows": n_rows,
        "sampling_seed": SAMPLING_SEED,
        "output": str(args.out),
        "open_with": "Numbers, Excel, LibreOffice, or `column -t -s$'\\t'`",
    }, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
