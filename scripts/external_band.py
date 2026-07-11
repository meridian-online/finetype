#!/usr/bin/env python3
"""Rotating external-data advisory band.

WHY THIS EXISTS
---------------
Gold is curated-*hard* and drawn from the same GitTables corpus the model
trains on. The representative band measures one random *GitTables* column.
Neither profiles a whole real-world external table end-to-end — and it was
exactly whole external tables (GLEIF, SEC EDGAR, NYC DOB, Chicago crimes) that
exposed the ticker / NAICS / org-name / WKT failures in the 2026-07
company-reference audit, at ~10x the yield of gittables gold-polishing. This
band is the standing instrument for that class of miss: profile a real external
table with the shipped binary, score the columns we have adjudicated labels
for, triage the rest.

STATUS: ADVISORY, never blocking (same as the representative band). It sits
ALONGSIDE the representative band in the promotion order, between the gold
corpus accuracy read and the blocking corpus-honest gate. No headline it prints
may override a blocking corpus-honest NO-GO. Because the held labels overlap
gold, the ABSOLUTE headline is common-mode across candidates — the trustworthy
signal is the candidate-vs-baseline delta (identical to the representative
band's rule).

LABEL SOURCE (the v1 design change over the frozen scaffold snapshot)
---------------------------------------------------------------------
Held labels are LIVE-DERIVED from the canonical gold corpus
(`eval/gold/gold_corpus.tsv`): every gold row whose `file_path` resolves to a
CSV under the external datasets dir becomes a held label, keyed by
(table basename, column). This auto-includes the `external:*` rows (openflights
etc.) AND the `compref:*` rows (gleif, sec_edgar, naics, ...) AND anything added
to gold later — the frozen `held_labels_v0.tsv` snapshot went stale
(nyc_dob had 7 labels frozen vs 16 live). No separate label file to maintain,
and no fresh sign-off needed to REUSE labels that are already canonical gold.

The `provenance` / `labeller` (tier) of every scored label is reported so the
overlap with the gold headline is explicit and never silently double-counted.

USAGE
-----
  python3 scripts/external_band.py \
      --binary target/release/finetype \
      --datasets-dir eval/datasets/gold_external \
      --gold eval/gold/gold_corpus.tsv \
      --out output/external-band/report.md

  # a rotated 3-table round (record the seed in the report):
  python3 scripts/external_band.py ... --rotate 3 --seed 20260711

  # score only named tables (the company-reference trio):
  python3 scripts/external_band.py ... --tables gleif_entities.csv,sec_edgar_companies.csv,nyc_dob_permits_sample.csv
"""
from __future__ import annotations

import argparse
import csv
import json
import random
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


def load_held_labels(gold: Path, datasets_dir: Path) -> dict[tuple[str, str], dict]:
    """Live-derive held labels: any gold row whose file_path is an on-disk
    external table. Returns (table_basename, column) -> {label, tier, family}."""
    out: dict[tuple[str, str], dict] = {}
    ds_resolved = datasets_dir.resolve()
    with gold.open(newline="") as fh:
        for r in csv.DictReader(fh, delimiter="\t"):
            fp = r.get("file_path", "")
            if not fp:
                continue
            p = Path(fp)
            try:
                is_ext = p.resolve().parent == ds_resolved
            except OSError:
                is_ext = False
            # tolerate relative-vs-absolute: also match on parent basename
            if not is_ext and p.parent.name != datasets_dir.name:
                continue
            label = r.get("curated_label", "")
            if not label:
                continue
            out[(p.name, r["column_name"])] = {
                "label": label,
                "tier": r.get("labeller", ""),
                "family": r.get("family", ""),
            }
    return out


def profile_table(binary: Path, csv_path: Path) -> dict[str, str]:
    """Profile a whole CSV -> {column: x-finetype-label}. Full table so
    sibling-context attention sees the real neighbours."""
    cmd = [str(binary), "profile", "-f", str(csv_path), "-o", "json-schema"]
    proc = subprocess.run(cmd, capture_output=True, text=True, timeout=900)
    if proc.returncode != 0:
        raise RuntimeError(
            f"profile failed for {csv_path.name}: {proc.stderr.strip()[-300:]}")
    props = json.loads(proc.stdout).get("properties", {})
    return {
        col: (defn.get("x-finetype-label", "") if isinstance(defn, dict) else "")
        for col, defn in props.items()
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--binary", required=True, type=Path,
                    help="finetype release binary under test")
    ap.add_argument("--datasets-dir", type=Path,
                    default=Path("eval/datasets/gold_external"))
    ap.add_argument("--gold", type=Path, default=Path("eval/gold/gold_corpus.tsv"),
                    help="canonical gold corpus (held labels are derived from it)")
    ap.add_argument("--tables", default="",
                    help="comma-separated table basenames to score (default: all "
                         "labelled external tables)")
    ap.add_argument("--rotate", type=int, default=0,
                    help="draw K pool tables this round (0 = all)")
    ap.add_argument("--seed", type=int, default=0,
                    help="rotation seed (record it in the report)")
    ap.add_argument("--out", type=Path, default=Path("output/external-band/report.md"))
    args = ap.parse_args()

    held = load_held_labels(args.gold, args.datasets_dir)
    if not held:
        print(f"ERROR: no held labels derived from {args.gold} for tables under "
              f"{args.datasets_dir}", file=sys.stderr)
        return 2
    labelled_tables = sorted({t for (t, _) in held})

    if args.tables:
        want = {s.strip() for s in args.tables.split(",") if s.strip()}
        tables = sorted(t for t in labelled_tables if t in want)
        missing = want - set(labelled_tables)
        if missing:
            print(f"WARN: requested tables with no held labels: {sorted(missing)}",
                  file=sys.stderr)
    elif args.rotate and args.rotate < len(labelled_tables):
        tables = sorted(random.Random(args.seed).sample(labelled_tables, args.rotate))
    else:
        tables = labelled_tables

    correct = scored = 0
    per_type_hit: dict[str, int] = defaultdict(int)
    per_type_tot: dict[str, int] = defaultdict(int)
    tier_seen: dict[str, int] = defaultdict(int)
    misses: list[tuple] = []       # table,col,gold,pred,tier
    unlabelled: list[tuple] = []   # table,col,pred
    per_table: dict[str, tuple[int, int]] = {}

    for t in tables:
        csv_path = args.datasets_dir / t
        if not csv_path.exists():
            print(f"WARN: missing {csv_path}", file=sys.stderr)
            continue
        preds = profile_table(args.binary, csv_path)
        t_cor = t_tot = 0
        for col, pred in preds.items():
            rec = held.get((t, col))
            if rec:
                gold = rec["label"]
                per_type_tot[gold] += 1
                tier_seen[rec["tier"]] += 1
                scored += 1
                t_tot += 1
                if pred == gold:
                    correct += 1
                    t_cor += 1
                    per_type_hit[gold] += 1
                else:
                    misses.append((t, col, gold, pred, rec["tier"]))
            else:
                unlabelled.append((t, col, pred))
        per_table[t] = (t_cor, t_tot)

    headline = (correct / scored) if scored else 0.0

    L: list[str] = []
    L.append("# External-data advisory band — report")
    L.append("")
    L.append("**Status:** ADVISORY (never blocking). Read the candidate-vs-baseline "
             "delta, not the absolute — held labels overlap gold, so the absolute is "
             "common-mode across candidates. No headline here overrides a blocking "
             "corpus-honest NO-GO.")
    L.append(f"**Binary:** `{args.binary}`")
    L.append(f"**Rotation:** rotate={args.rotate or 'all'} seed={args.seed} "
             f"tables={len(tables)} ({', '.join(tables)})")
    L.append(f"**Label source:** live-derived from `{args.gold}` "
             f"(gold rows pointing at `{args.datasets_dir}/`).")
    L.append("")
    L.append(f"## Headline: {correct}/{scored} = {headline:.3f} "
             f"({len(unlabelled)} unlabelled emissions triaged)")
    L.append("")
    L.append("### Per-table")
    L.append("")
    L.append("| table | correct | scored | headline |")
    L.append("|---|---|---|---|")
    for t in tables:
        c, n = per_table.get(t, (0, 0))
        L.append(f"| {t} | {c} | {n} | {(c/n if n else 0):.3f} |")
    L.append("")
    L.append("### Tier mix of scored labels (gold-overlap disclosure)")
    L.append("")
    L.append("| tier (labeller) | count |")
    L.append("|---|---|")
    for tier in sorted(tier_seen, key=lambda k: -tier_seen[k]):
        L.append(f"| {tier or '(blank)'} | {tier_seen[tier]} |")
    L.append("")
    L.append("## Per-type (adjudicated columns only)")
    L.append("")
    L.append("| label | correct | total | recall |")
    L.append("|---|---|---|---|")
    for lab in sorted(per_type_tot):
        h, tot = per_type_hit[lab], per_type_tot[lab]
        L.append(f"| {lab} | {h} | {tot} | {h/tot:.3f} |")
    L.append("")
    L.append("## Misses (adjudicated gold != predicted)")
    L.append("")
    L.append("| table | column | gold | predicted | tier |")
    L.append("|---|---|---|---|---|")
    for t, c, g, p, tier in misses:
        L.append(f"| {t} | {c} | {g} | {p} | {tier} |")
    L.append("")
    L.append("## Unlabelled emissions (triage — NOT in the headline)")
    L.append("")
    L.append("Profiled columns with no adjudicated label yet. This is the "
             "candidate-expansion queue: an over-emission here (e.g. a ticker read "
             "as a state code) is the failure class this band exists to surface. "
             "Adjudicate + assign a truth tier before any of these counts toward a "
             "headline.")
    L.append("")
    L.append("| table | column | predicted |")
    L.append("|---|---|---|")
    for t, c, p in unlabelled:
        L.append(f"| {t} | {c} | {p} |")
    L.append("")

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text("\n".join(L))
    print(f"headline {correct}/{scored} = {headline:.3f}; {len(misses)} misses, "
          f"{len(unlabelled)} unlabelled -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
