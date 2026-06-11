#!/usr/bin/env python3
"""v27 recall retrain ac-02 — build the hard-positive distilled blend + audit gate.

Spec 2026-06-11-categorical-identifier-recall-retrain. Additive over the v19
base distilled (the SHIPPED default's training corpus), mirroring the latdec
builder but for the RECALL direction: curated positives for the two gold
miss-pools (categorical R 0.386, alphanumeric_id R 0.111), mined and filtered
by scripts/mine_v27_recall_positives.py.

Concatenates, in order:
  1. v19 base distilled (distillation-v3/sherlock_distilled.csv.gz) — pass-through
  2. v27 hard positives (output/v27-recall-retrain/hard_positives.parquet) —
     correct_label ONLY representation.discrete.categorical or
     representation.identifier.alphanumeric_id (asserted)

Only (final_label, sample_values, column_name) are written — drop-in
compatible with prepare_multibranch_data.py, where the per-label cap lift is
applied via --type-cap (NOT here):
  --type-cap representation.discrete.categorical=3400
  --type-cap representation.identifier.alphanumeric_id=2400

Audit gate. Fails (exit 3) on any of:
  - total rows < AUDIT_TOTAL_ROWS
  - base v19 rows < AUDIT_BASE_ROWS   (the blend must not drop the v19 corpus)
  - hard-positive rows < AUDIT_HP_ROWS
  - ANY hard-positive final_label outside the two targets (the spec's hard
    invariant: a stray label would poison an untargeted boundary by hand)
  - mined values column empty on > 1% of hard positives

Output:
  output/v27-recall-retrain/sherlock_distilled_v27.csv.gz
  output/v27-recall-retrain/v27_blend_manifest.json

Run under the gittables venv (needs the python duckdb module):
  eval/gittables/.venv/bin/python scripts/build_v27_recall_distilled.py
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

V19_DISTILLED = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
HARD_POSITIVES = REPO / "output/v27-recall-retrain/hard_positives.parquet"
DEFAULT_OUT = REPO / "output/v27-recall-retrain/sherlock_distilled_v27.csv.gz"
DEFAULT_MANIFEST = REPO / "output/v27-recall-retrain/v27_blend_manifest.json"

TARGETS = {
    "representation.discrete.categorical",
    "representation.identifier.alphanumeric_id",
}

# Round 2 discovery: prepare_multibranch_data.py's per-type caps are COSMETIC
# for the v4 sibling-grouped path — group_distilled_by_proximity() consumes
# ordered_distilled, which is never rebuilt after cap_distilled_columns()
# mutates the capped dict (line ~2429). So the ONLY layer that controls
# per-type training mass is this blend CSV. The re-scope (categorical ~1,700
# effective: mined 1,300 + base 400) is therefore enforced HERE, by
# deterministic downsampling. Base increment is dropped entirely: it only
# became trainable via --include-column-level-types and v19's status quo
# (synthetic-only) must be preserved for gold's worst over-emitter (P=0.056).
CAT = "representation.discrete.categorical"
ALNUM = "representation.identifier.alphanumeric_id"
INCREMENT = "representation.identifier.increment"
BASE_KEEP = {CAT: 400, ALNUM: 300, INCREMENT: 0}
MINED_KEEP = {CAT: 1_300, ALNUM: 2_094}

AUDIT_TOTAL_ROWS = 85_500
AUDIT_BASE_ROWS = 82_000
AUDIT_HP_ROWS = 3_300  # round 2: B2_word dropped + mined categorical trimmed


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v19-distilled", type=Path, default=V19_DISTILLED)
    p.add_argument("--hard-positives", type=Path, default=HARD_POSITIVES)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = p.parse_args()

    for path in (args.v19_distilled, args.hard_positives):
        if not path.exists():
            print(f"error: source missing — {path}", file=sys.stderr)
            return 2
    args.out.parent.mkdir(parents=True, exist_ok=True)

    per_label_rows: dict[str, int] = defaultdict(int)
    per_cluster_rows: dict[str, int] = defaultdict(int)
    hp_stray_labels = 0
    hp_empty_values = 0
    total = 0

    print(f"writing {args.out}...", file=sys.stderr)
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])

        # ── (1) v19 base distilled (pass-through, target labels capped) ──
        # Deterministic downsample for the three controlled labels: rank by
        # md5(label|values|header), keep the first BASE_KEEP[label]. Every
        # other label passes through untouched (v19 status quo).
        import hashlib

        n_base = 0
        base_dropped: dict[str, int] = defaultdict(int)
        controlled: dict[str, list] = {k: [] for k in BASE_KEEP}
        with gzip.open(args.v19_distilled, "rt", newline="",
                       encoding="utf-8") as fin:
            for row in csv.DictReader(fin):
                label = (row.get("final_label") or "").strip()
                if not label:
                    continue
                sv = row.get("sample_values") or ""
                cn = row.get("column_name") or ""
                if label in BASE_KEEP:
                    controlled[label].append((sv, cn))
                    continue
                writer.writerow([label, sv, cn])
                per_label_rows[label] += 1
                n_base += 1
                total += 1
        for label, rows_c in controlled.items():
            keep = BASE_KEEP[label]
            rows_c.sort(key=lambda r: hashlib.md5(
                f"{label}|{r[0]}|{r[1]}".encode()).hexdigest())
            for sv, cn in rows_c[:keep]:
                writer.writerow([label, sv, cn])
                per_label_rows[label] += 1
                n_base += 1
                total += 1
            base_dropped[label] = max(0, len(rows_c) - keep)
            print(f"  base {label}: {len(rows_c)} -> {min(keep, len(rows_c))}",
                  file=sys.stderr)
        print(f"  {'v19_distilled':>20s}: {n_base:>8,} rows", file=sys.stderr)

        # ── (2) v27 hard positives (parquet → 3-col CSV) ──────────────
        try:
            import duckdb  # type: ignore
        except ImportError as exc:
            print(f"error: duckdb missing ({exc}). Run under the venv.",
                  file=sys.stderr)
            return 2
        con = duckdb.connect()
        rows = con.execute(f"""
            SELECT correct_label, sample_values, column_name, cluster_id
              FROM read_parquet('{args.hard_positives.as_posix()}')
        """).fetchall()
        # Trim mined rows per label to MINED_KEEP, deterministically (same
        # md5 ranking as the base downsample), proportionally across buckets
        # by virtue of hash ordering being bucket-blind.
        by_label: dict[str, list] = defaultdict(list)
        for label, sample_values, cname, cluster_id in rows:
            by_label[label].append((sample_values, cname, cluster_id))
        trimmed = []
        for label, lrows in by_label.items():
            keep = MINED_KEEP.get(label, len(lrows))
            lrows.sort(key=lambda r: hashlib.md5(
                f"{label}|{r[0]}|{r[1]}".encode()).hexdigest())
            if len(lrows) > keep:
                print(f"  mined {label}: {len(lrows)} -> {keep}", file=sys.stderr)
            for sv, cn, cid in lrows[:keep]:
                trimmed.append((label, sv, cn, cid))

        n_hp = 0
        for label, sample_values, cname, cluster_id in trimmed:
            if label not in TARGETS:
                hp_stray_labels += 1
                continue
            # The mined parquet carries the corpus's │-separated raw string;
            # the distilled-CSV contract is a JSON list (the FTMB loader
            # json.loads every row and silently drops parse failures — this
            # conversion is what keeps the mined rows in training). The last
            # token may be truncated mid-value, so drop it when there are
            # multiple.
            vals = [v.strip() for v in (sample_values or "").split("│") if v.strip()]
            if len(vals) > 1:
                vals = vals[:-1]
            if not vals:
                hp_empty_values += 1
                continue
            writer.writerow([label, json.dumps(vals), cname or ""])
            per_label_rows[label] += 1
            per_cluster_rows[cluster_id] += 1
            n_hp += 1
            total += 1
        print(f"  {'v27_hard_positives':>20s}: {n_hp:>8,} rows", file=sys.stderr)

    # ── Audit gate ─────────────────────────────────────────────────────
    failures: list[str] = []
    if total < AUDIT_TOTAL_ROWS:
        failures.append(f"total rows {total:,} < {AUDIT_TOTAL_ROWS:,}")
    if n_base < AUDIT_BASE_ROWS:
        failures.append(f"base rows {n_base:,} < {AUDIT_BASE_ROWS:,}")
    if n_hp < AUDIT_HP_ROWS:
        failures.append(f"hard-positive rows {n_hp:,} < {AUDIT_HP_ROWS:,}")
    if hp_stray_labels:
        failures.append(f"{hp_stray_labels} hard positives with stray labels")
    if hp_empty_values > 0.01 * max(n_hp, 1):
        failures.append(f"{hp_empty_values} hard positives with empty values")

    manifest = {
        "spec": "2026-06-11-categorical-identifier-recall-retrain",
        "out": str(args.out),
        "total_rows": total,
        "per_source_rows": {"v19_distilled": n_base, "v27_hard_positives": n_hp},
        "per_cluster_rows": dict(sorted(per_cluster_rows.items())),
        "target_label_rows": {k: per_label_rows[k] for k in sorted(TARGETS)},
        "hp_stray_label_rows": hp_stray_labels,
        "hp_empty_value_rows": hp_empty_values,
        "effective_label_masses": {
            "note": ("enforced HERE — FTMB --type-cap is cosmetic for the v4 "
                     "sibling-grouped path (ordered_distilled never re-capped)"),
            "base_keep": dict(BASE_KEEP),
            "mined_keep": dict(MINED_KEEP),
            "base_dropped": dict(base_dropped),
        },
        "audit": {
            "total_rows_gate": AUDIT_TOTAL_ROWS,
            "base_rows_gate": AUDIT_BASE_ROWS,
            "hp_rows_gate": AUDIT_HP_ROWS,
            "failures": failures,
            "passed": not failures,
        },
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps(manifest["audit"], indent=2), file=sys.stderr)
    if failures:
        print("AUDIT GATE FAILED", file=sys.stderr)
        return 3
    print(f"AUDIT GATE PASSED — {total:,} rows", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
