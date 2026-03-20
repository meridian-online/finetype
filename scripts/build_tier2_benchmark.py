#!/usr/bin/env python3
"""Build Tier 2 evaluation benchmark from distilled + synthetic data.

Produces a CSV with ~2,500 columns (10 per taxonomy type) for measuring
FineType accuracy. Distilled types use adjudicated rows from
sherlock_distilled.csv.gz; missing types use finetype generate output.

Usage:
    python3 scripts/build_tier2_benchmark.py [OPTIONS]

Options:
    --seed N            Random seed (default: 42)
    --per-type N        Columns per type (default: 10)
    --min-values N      Minimum values per distilled column (default: 5)
    --output PATH       Output CSV path (default: eval/tier2_benchmark.csv)
    --distilled PATH    Distilled data path (default: output/distillation-v3/sherlock_distilled.csv.gz)
    --dry-run           Show plan without writing output
    -h, --help          Show this help message

Output CSV schema:
    type_key           - Taxonomy type key (e.g., identity.person.email)
    source             - "distilled" or "synthetic"
    values             - JSON array of sample values
    header             - Column header (empty string for headerless)
    expected_label     - Expected classification label (same as type_key)
    source_agreement   - "yes", "no", or "synthetic"
"""

import csv
import gzip
import json
import os
import random
import subprocess
import sys
from collections import defaultdict


def get_taxonomy_types(finetype_bin="finetype"):
    """Get all taxonomy type keys from finetype taxonomy command."""
    result = subprocess.run(
        [finetype_bin, "taxonomy"],
        capture_output=True, text=True
    )
    types = set()
    for line in result.stdout.strip().split("\n"):
        if "→" in line:
            key = line.strip().split(" →")[0]
            if "." in key:
                types.add(key)
    return sorted(types)


def load_distilled_rows(path, min_values=5):
    """Load qualifying rows from distilled data, grouped by final_label.

    Qualifying: has final_label, parseable sample_values with >= min_values elements.
    Returns dict of type_key -> list of (values, header, agreement) tuples.
    """
    rows_by_type = defaultdict(list)
    stats = {"total": 0, "no_label": 0, "parse_error": 0, "too_few": 0, "qualifying": 0}

    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt") as f:
        reader = csv.DictReader(f)
        for row in reader:
            stats["total"] += 1
            final = row.get("final_label", "").strip()
            if not final or "." not in final:
                stats["no_label"] += 1
                continue

            try:
                vals = json.loads(row.get("sample_values", "[]"))
            except (json.JSONDecodeError, TypeError):
                stats["parse_error"] += 1
                continue

            if not isinstance(vals, list) or len(vals) < min_values:
                stats["too_few"] += 1
                continue

            stats["qualifying"] += 1
            agreement = row.get("agreement", "").strip()
            header = row.get("column_name", "").strip()
            rows_by_type[final].append({
                "values": vals,
                "header": header,
                "agreement": agreement,
            })

    return rows_by_type, stats


def generate_synthetic_columns(types, values_per_column, seed, finetype_bin="finetype",
                                cache_path=None):
    """Generate synthetic columns for types that need them.

    Args:
        types: dict of type_key -> number of synthetic columns needed
        values_per_column: number of values per synthetic column
        seed: random seed
        finetype_bin: path to finetype binary
        cache_path: if set, reuse cached NDJSON for reproducibility

    Returns:
        dict of type_key -> list of {"values": [...], "header": "", "agreement": "synthetic"}
    """
    if not types:
        return {}

    # Generate enough samples for all types
    max_needed = max(types.values()) * values_per_column
    samples_per_label = max(max_needed, values_per_column * 10)

    # Use cached synthetic data if available (finetype generate isn't fully deterministic)
    ndjson_path = cache_path or "/tmp/tier2_synthetic.ndjson"
    if cache_path and os.path.exists(cache_path):
        print(f"  Using cached synthetic data: {cache_path}", file=sys.stderr)
    else:
        subprocess.run(
            [finetype_bin, "generate",
             "--samples", str(samples_per_label),
             "--output", ndjson_path,
             "--seed", str(seed)],
            capture_output=True, text=True, check=True
        )
        if cache_path:
            print(f"  Generated and cached synthetic data: {cache_path}", file=sys.stderr)

    # Parse generated values grouped by type (sorted for determinism)
    generated = defaultdict(list)
    with open(ndjson_path) as f:
        for line in f:
            obj = json.loads(line)
            key = obj["classification"]
            if key in types:
                generated[key].append(obj["text"])

    # Build columns from generated values (deterministic via seeded RNG)
    rng = random.Random(seed + 1)  # offset to avoid correlation with distilled sampling
    result = defaultdict(list)

    for type_key in sorted(types.keys()):  # sorted for determinism
        num_columns = types[type_key]
        available = generated.get(type_key, [])
        if not available:
            print(f"  WARNING: No generated values for {type_key}", file=sys.stderr)
            continue

        for _ in range(num_columns):
            # Sample values_per_column values (with replacement if needed)
            if len(available) >= values_per_column:
                col_values = rng.sample(available, values_per_column)
            else:
                col_values = [rng.choice(available) for _ in range(values_per_column)]

            result[type_key].append({
                "values": col_values,
                "header": "",
                "agreement": "synthetic",
            })

    if not cache_path:
        os.unlink(ndjson_path)
    return result


def build_benchmark(distilled_path, seed, per_type, min_values, finetype_bin="finetype"):
    """Build the complete Tier 2 benchmark.

    Returns (rows, plan) where rows is a list of benchmark CSV rows and
    plan is a dict summarising what was built.
    """
    rng = random.Random(seed)

    # Step 1: Get all taxonomy types
    all_types = get_taxonomy_types(finetype_bin)
    print(f"Taxonomy types: {len(all_types)}", file=sys.stderr)

    # Step 2: Load distilled data
    distilled, stats = load_distilled_rows(distilled_path, min_values)
    print(f"Distilled data: {stats['qualifying']:,} qualifying rows, "
          f"{len(distilled)} types", file=sys.stderr)
    print(f"  Excluded: {stats['no_label']} no label, {stats['parse_error']} parse error, "
          f"{stats['too_few']} <{min_values} values", file=sys.stderr)

    # Step 3: Plan — how many distilled vs synthetic columns per type
    synthetic_needs = {}  # type -> num synthetic columns needed
    plan = {"full": 0, "partial": 0, "synthetic": 0}

    for type_key in all_types:
        available = len(distilled.get(type_key, []))
        if available >= per_type:
            plan["full"] += 1
        elif available > 0:
            plan["partial"] += 1
            synthetic_needs[type_key] = per_type - available
        else:
            plan["synthetic"] += 1
            synthetic_needs[type_key] = per_type

    print(f"Plan: {plan['full']} fully distilled, {plan['partial']} partial, "
          f"{plan['synthetic']} fully synthetic", file=sys.stderr)

    # Step 4: Generate synthetic columns
    if synthetic_needs:
        # Use median distilled column size for synthetic columns
        all_sizes = [len(r["values"]) for rows in distilled.values() for r in rows]
        median_size = sorted(all_sizes)[len(all_sizes) // 2] if all_sizes else 10
        print(f"Generating synthetic columns ({len(synthetic_needs)} types, "
              f"{median_size} values/column)...", file=sys.stderr)
        cache = os.path.join(os.path.dirname(distilled_path), "tier2_synthetic.ndjson")
        synthetic = generate_synthetic_columns(
            synthetic_needs, median_size, seed, finetype_bin, cache_path=cache
        )
    else:
        synthetic = {}

    # Step 5: Build benchmark rows
    benchmark = []
    missing_types = []

    for type_key in all_types:
        rows_for_type = []

        # Add distilled rows (sample if >per_type)
        available = distilled.get(type_key, [])
        if len(available) > per_type:
            sampled = rng.sample(available, per_type)
        else:
            sampled = list(available)

        for r in sampled:
            rows_for_type.append({
                "type_key": type_key,
                "source": "distilled",
                "values": json.dumps(r["values"]),
                "header": r["header"],
                "expected_label": type_key,
                "source_agreement": r["agreement"],
            })

        # Fill remainder from synthetic
        remaining = per_type - len(rows_for_type)
        if remaining > 0:
            syn_rows = synthetic.get(type_key, [])
            for r in syn_rows[:remaining]:
                rows_for_type.append({
                    "type_key": type_key,
                    "source": "synthetic",
                    "values": json.dumps(r["values"]),
                    "header": r["header"],
                    "expected_label": type_key,
                    "source_agreement": "synthetic",
                })

        if len(rows_for_type) < per_type:
            missing_types.append((type_key, len(rows_for_type)))

        benchmark.extend(rows_for_type)

    if missing_types:
        print(f"WARNING: {len(missing_types)} types with <{per_type} columns:",
              file=sys.stderr)
        for t, n in missing_types:
            print(f"  {t}: {n}/{per_type}", file=sys.stderr)

    return benchmark, plan


def main():
    # Parse arguments
    args = sys.argv[1:]
    seed = 42
    per_type = 10
    min_values = 5
    output = "eval/tier2_benchmark.csv"
    distilled = "output/distillation-v3/sherlock_distilled.csv.gz"
    finetype_bin = "finetype"
    dry_run = False

    i = 0
    while i < len(args):
        if args[i] in ("-h", "--help", "help"):
            print(__doc__)
            return
        elif args[i] == "--seed":
            seed = int(args[i + 1]); i += 2
        elif args[i] == "--per-type":
            per_type = int(args[i + 1]); i += 2
        elif args[i] == "--min-values":
            min_values = int(args[i + 1]); i += 2
        elif args[i] == "--output":
            output = args[i + 1]; i += 2
        elif args[i] == "--distilled":
            distilled = args[i + 1]; i += 2
        elif args[i] == "--finetype":
            finetype_bin = args[i + 1]; i += 2
        elif args[i] == "--dry-run":
            dry_run = True; i += 1
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    # Build
    benchmark, _plan = build_benchmark(distilled, seed, per_type, min_values, finetype_bin)

    if dry_run:
        print(f"\nDry run — would write {len(benchmark)} rows to {output}")
        return

    # Write output
    os.makedirs(os.path.dirname(output) or ".", exist_ok=True)
    fieldnames = ["type_key", "source", "values", "header", "expected_label", "source_agreement"]
    with open(output, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(benchmark)

    print(f"\nWrote {len(benchmark)} rows to {output}", file=sys.stderr)

    # Summary
    sources = {"distilled": 0, "synthetic": 0}
    agreements = {"yes": 0, "no": 0, "synthetic": 0}
    for row in benchmark:
        sources[row["source"]] += 1
        agreements[row["source_agreement"]] += 1

    print(f"  Distilled: {sources['distilled']}, Synthetic: {sources['synthetic']}", file=sys.stderr)
    print(f"  Agreement: {agreements.get('yes', 0)}, "
          f"Disagreement: {agreements.get('no', 0)}, "
          f"Synthetic: {agreements.get('synthetic', 0)}", file=sys.stderr)


if __name__ == "__main__":
    main()
