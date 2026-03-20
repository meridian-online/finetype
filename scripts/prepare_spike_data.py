#!/usr/bin/env python3
"""Prepare training data mixes for the retraining spike.

Reads distilled data (sherlock_distilled.csv.gz) and synthetic data
(from finetype generate), then produces NDJSON training files for each
experiment mix.

Usage:
    python3 scripts/prepare_spike_data.py [OPTIONS]

Options:
    --distilled PATH        Distilled data CSV (default: output/distillation-v3/sherlock_distilled.csv.gz)
    --finetype PATH         Path to finetype binary (default: ./target/release/finetype)
    --output-dir DIR        Output directory (default: output/spike-training)
    --samples-per-type N    Target samples per type (default: 1500)
    --min-values N          Minimum values per column to qualify (default: 5)
    --seed N                Random seed (default: 42)
    --dry-run               Show counts without writing files
    -h, --help              Show this help
"""

import csv
import gzip
import json
import os
import random
import subprocess
import sys
from collections import Counter, defaultdict

# Column-level types that may cause negative transfer
COLUMN_LEVEL_TYPES = {
    "representation.discrete.categorical",
    "representation.discrete.ordinal",
    "representation.identifier.increment",
}


def load_taxonomy_types(finetype_bin):
    """Get the full list of 250 taxonomy types from finetype."""
    result = subprocess.run(
        [finetype_bin, "taxonomy", "--output", "csv"],
        capture_output=True, text=True, check=True,
    )
    types = set()
    reader = csv.DictReader(result.stdout.splitlines())
    for row in reader:
        key = row.get("key", "").strip()
        if key:
            types.add(key)
    return types


def load_distilled_values(distilled_path, min_values, exclude_column_types=False):
    """Load distilled data and explode to individual value→label pairs.

    Returns:
        values_by_type: dict[str, list[str]] — values grouped by type key
        stats: dict with counts for logging
    """
    values_by_type = defaultdict(list)
    stats = {
        "total_rows": 0,
        "qualifying_rows": 0,
        "sparse_rows": 0,
        "parse_errors": 0,
        "empty_label": 0,
        "excluded_column_types": 0,
        "total_values": 0,
        "column_type_counts": Counter(),
    }

    opener = gzip.open if distilled_path.endswith(".gz") else open
    with opener(distilled_path, "rt", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            stats["total_rows"] += 1
            label = row.get("final_label", "").strip()

            if not label:
                stats["empty_label"] += 1
                continue

            # Track column-level types
            if label in COLUMN_LEVEL_TYPES:
                stats["column_type_counts"][label] += 1
                if exclude_column_types:
                    stats["excluded_column_types"] += 1
                    continue

            try:
                vals = json.loads(row.get("sample_values", "[]"))
            except (json.JSONDecodeError, TypeError):
                stats["parse_errors"] += 1
                continue

            n = len(vals) if isinstance(vals, list) else 0
            if n < min_values:
                stats["sparse_rows"] += 1
                continue

            stats["qualifying_rows"] += 1

            # Explode: each value becomes a training sample
            for v in vals:
                text = str(v).strip()
                if text:
                    values_by_type[label].append(text)
                    stats["total_values"] += 1

    return dict(values_by_type), stats


def generate_synthetic(finetype_bin, samples_per_type, seed):
    """Generate synthetic training data via finetype generate.

    Returns: dict[str, list[str]] — values grouped by type key
    """
    import tempfile

    with tempfile.NamedTemporaryFile(suffix=".ndjson", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        subprocess.run(
            [finetype_bin, "generate",
             "--samples", str(samples_per_type),
             "--seed", str(seed),
             "--output", tmp_path],
            capture_output=True, text=True, check=True,
        )

        values_by_type = defaultdict(list)
        with open(tmp_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rec = json.loads(line)
                values_by_type[rec["classification"]].append(rec["text"])
        return dict(values_by_type)
    finally:
        os.unlink(tmp_path)


def write_ndjson(path, values_by_type):
    """Write a dict[type, list[str]] as NDJSON training data."""
    n = 0
    with open(path, "w") as f:
        for type_key in sorted(values_by_type.keys()):
            for text in values_by_type[type_key]:
                f.write(json.dumps({"text": text, "classification": type_key}) + "\n")
                n += 1
    return n


def blend_data(distilled, synthetic, ratio_distilled, samples_per_type, rng):
    """Blend distilled and synthetic data per-type with capping (no oversampling).

    ratio_distilled: float 0.0–1.0 (e.g. 0.7 means 70% distilled)
    Returns: dict[str, list[str]]
    """
    all_types = set(distilled.keys()) | set(synthetic.keys())
    blended = {}

    for type_key in sorted(all_types):
        d_vals = distilled.get(type_key, [])
        s_vals = synthetic.get(type_key, [])

        target_d = int(samples_per_type * ratio_distilled)
        target_s = samples_per_type - target_d

        # Cap at available — no oversampling. Fill remainder from other source.
        if len(d_vals) < target_d:
            actual_d = len(d_vals)
            actual_s = min(len(s_vals), samples_per_type - actual_d)
        elif len(s_vals) < target_s:
            actual_s = len(s_vals)
            actual_d = min(len(d_vals), samples_per_type - actual_s)
        else:
            actual_d = target_d
            actual_s = target_s

        # Sample without replacement
        picked_d = rng.sample(d_vals, actual_d) if actual_d <= len(d_vals) else d_vals[:]
        picked_s = rng.sample(s_vals, actual_s) if actual_s <= len(s_vals) else s_vals[:]

        combined = picked_d + picked_s
        rng.shuffle(combined)
        blended[type_key] = combined

    return blended


def main():
    args = sys.argv[1:]

    # Defaults
    distilled_path = "output/distillation-v3/sherlock_distilled.csv.gz"
    finetype_bin = "./target/release/finetype"
    output_dir = "output/spike-training"
    samples_per_type = 1500
    min_values = 5
    seed = 42
    dry_run = False

    i = 0
    while i < len(args):
        if args[i] == "--distilled":
            distilled_path = args[i + 1]; i += 2
        elif args[i] == "--finetype":
            finetype_bin = args[i + 1]; i += 2
        elif args[i] == "--output-dir":
            output_dir = args[i + 1]; i += 2
        elif args[i] == "--samples-per-type":
            samples_per_type = int(args[i + 1]); i += 2
        elif args[i] == "--min-values":
            min_values = int(args[i + 1]); i += 2
        elif args[i] == "--seed":
            seed = int(args[i + 1]); i += 2
        elif args[i] == "--dry-run":
            dry_run = True; i += 1
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    rng = random.Random(seed)
    os.makedirs(output_dir, exist_ok=True)

    # ─── Load taxonomy ─────────────────────────────────────────────
    print("Loading taxonomy...")
    taxonomy_types = load_taxonomy_types(finetype_bin)
    print(f"  {len(taxonomy_types)} taxonomy types")

    # ─── Load distilled data ───────────────────────────────────────
    print(f"\nLoading distilled data (min_values={min_values})...")
    distilled, d_stats = load_distilled_values(
        distilled_path, min_values, exclude_column_types=False
    )
    print(f"  {d_stats['total_rows']} total rows")
    print(f"  {d_stats['qualifying_rows']} qualifying rows")
    print(f"  {d_stats['sparse_rows']} sparse rows (skipped)")
    print(f"  {d_stats['parse_errors']} parse errors (skipped)")
    print(f"  {d_stats['empty_label']} empty labels (skipped)")
    print(f"  {d_stats['total_values']} individual values across {len(distilled)} types")
    if d_stats["column_type_counts"]:
        print(f"  Column-level type counts (rows):")
        for t, c in d_stats["column_type_counts"].most_common():
            print(f"    {t}: {c}")

    # Also load distilled excluding column-level types for experiment 6
    print(f"\nLoading distilled data (excluding column-level types)...")
    distilled_no_col, d_no_col_stats = load_distilled_values(
        distilled_path, min_values, exclude_column_types=True
    )
    print(f"  {d_no_col_stats['excluded_column_types']} column-type rows excluded")
    print(f"  {d_no_col_stats['total_values']} individual values across {len(distilled_no_col)} types")

    # ─── Generate synthetic data ───────────────────────────────────
    print(f"\nGenerating synthetic data ({samples_per_type} samples/type)...")
    synthetic = generate_synthetic(finetype_bin, samples_per_type, seed)
    total_synth = sum(len(v) for v in synthetic.values())
    print(f"  {total_synth} values across {len(synthetic)} types")

    # ─── Validate labels ───────────────────────────────────────────
    bad_distilled = set(distilled.keys()) - taxonomy_types
    if bad_distilled:
        print(f"\n  WARNING: {len(bad_distilled)} distilled types not in taxonomy:")
        for t in sorted(bad_distilled):
            print(f"    {t} ({len(distilled[t])} values)")
        # Remove invalid types
        for t in bad_distilled:
            del distilled[t]
            if t in distilled_no_col:
                del distilled_no_col[t]

    # ─── Summary ───────────────────────────────────────────────────
    distilled_types = set(distilled.keys())
    synthetic_types = set(synthetic.keys())
    missing_types = taxonomy_types - distilled_types - synthetic_types
    backfill_types = taxonomy_types - distilled_types  # types needing synthetic backfill

    print(f"\n{'='*60}")
    print(f"Type coverage:")
    print(f"  Distilled: {len(distilled_types)} types")
    print(f"  Synthetic: {len(synthetic_types)} types")
    print(f"  Backfill needed (distilled→synthetic): {len(backfill_types)} types")
    print(f"  No source at all: {len(missing_types)} types")
    if missing_types:
        for t in sorted(missing_types):
            print(f"    {t}")
    print(f"{'='*60}")

    if dry_run:
        print("\n[DRY RUN] Would generate these files:")
        print(f"  {output_dir}/synthetic.ndjson")
        print(f"  {output_dir}/distilled-backfill.ndjson")
        print(f"  {output_dir}/blend-50-50.ndjson")
        print(f"  {output_dir}/blend-70-30.ndjson")
        print(f"  {output_dir}/blend-30-70.ndjson")
        print(f"  {output_dir}/blend-70-30-no-coltype.ndjson")
        return

    # ─── Experiment 1: Synthetic-only ──────────────────────────────
    print(f"\n[1/6] Generating synthetic.ndjson...")
    n = write_ndjson(os.path.join(output_dir, "synthetic.ndjson"), synthetic)
    print(f"  {n} samples, {len(synthetic)} types")

    # ─── Experiment 2: Distilled + synthetic backfill ──────────────
    print(f"\n[2/6] Generating distilled-backfill.ndjson...")
    distilled_bf = {}
    for type_key in sorted(taxonomy_types):
        if type_key in distilled and distilled[type_key]:
            # Use all distilled values (capped at samples_per_type)
            vals = distilled[type_key]
            if len(vals) > samples_per_type:
                vals = rng.sample(vals, samples_per_type)
            distilled_bf[type_key] = vals
        elif type_key in synthetic:
            # Backfill from synthetic
            distilled_bf[type_key] = synthetic[type_key][:samples_per_type]
        # else: type has neither source (password, plain_text) — skip
    n = write_ndjson(os.path.join(output_dir, "distilled-backfill.ndjson"), distilled_bf)
    d_count = sum(1 for t in distilled_bf if t in distilled and distilled[t])
    s_count = len(distilled_bf) - d_count
    print(f"  {n} samples, {len(distilled_bf)} types ({d_count} distilled, {s_count} synthetic backfill)")

    # ─── Experiment 3: Blend 50/50 ─────────────────────────────────
    print(f"\n[3/6] Generating blend-50-50.ndjson...")
    blend_50 = blend_data(distilled, synthetic, 0.5, samples_per_type, rng)
    n = write_ndjson(os.path.join(output_dir, "blend-50-50.ndjson"), blend_50)
    print(f"  {n} samples, {len(blend_50)} types")

    # ─── Experiment 4: Blend 70/30 (distilled-heavy) ───────────────
    print(f"\n[4/6] Generating blend-70-30.ndjson...")
    blend_70 = blend_data(distilled, synthetic, 0.7, samples_per_type, rng)
    n = write_ndjson(os.path.join(output_dir, "blend-70-30.ndjson"), blend_70)
    print(f"  {n} samples, {len(blend_70)} types")

    # ─── Experiment 5: Blend 30/70 (synthetic-heavy) ───────────────
    print(f"\n[5/6] Generating blend-30-70.ndjson...")
    blend_30 = blend_data(distilled, synthetic, 0.3, samples_per_type, rng)
    n = write_ndjson(os.path.join(output_dir, "blend-30-70.ndjson"), blend_30)
    print(f"  {n} samples, {len(blend_30)} types")

    # ─── Experiment 6: Blend 70/30 excluding column-level types ────
    print(f"\n[6/6] Generating blend-70-30-no-coltype.ndjson...")
    blend_70_nc = blend_data(distilled_no_col, synthetic, 0.7, samples_per_type, rng)
    n = write_ndjson(os.path.join(output_dir, "blend-70-30-no-coltype.ndjson"), blend_70_nc)
    print(f"  {n} samples, {len(blend_70_nc)} types")

    # ─── Column-level type value counts per mix ────────────────────
    print(f"\n{'='*60}")
    print("Column-level type values per mix:")
    mixes = {
        "synthetic": synthetic,
        "distilled-backfill": distilled_bf,
        "blend-50-50": blend_50,
        "blend-70-30": blend_70,
        "blend-30-70": blend_30,
        "blend-70-30-no-coltype": blend_70_nc,
    }
    for mix_name, mix_data in mixes.items():
        col_counts = {}
        for ct in COLUMN_LEVEL_TYPES:
            col_counts[ct.split(".")[-1]] = len(mix_data.get(ct, []))
        total = sum(col_counts.values())
        print(f"  {mix_name}: {total} total — {col_counts}")
    print(f"{'='*60}")

    # ─── Write manifest ────────────────────────────────────────────
    manifest_path = os.path.join(output_dir, "manifest.json")
    manifest = {
        "seed": seed,
        "samples_per_type": samples_per_type,
        "min_values": min_values,
        "distilled_source": distilled_path,
        "taxonomy_types": len(taxonomy_types),
        "distilled_types": len(distilled_types),
        "synthetic_types": len(synthetic_types),
        "experiments": [
            "synthetic.ndjson",
            "distilled-backfill.ndjson",
            "blend-50-50.ndjson",
            "blend-70-30.ndjson",
            "blend-30-70.ndjson",
            "blend-70-30-no-coltype.ndjson",
        ],
    }
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"\nManifest: {manifest_path}")
    print("Done.")


if __name__ == "__main__":
    main()
