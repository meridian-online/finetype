#!/usr/bin/env python3
"""Prepare feature-vector training data for the multi-branch model.

Reads distilled data (sherlock_distilled.csv.gz) and synthetic data
(from finetype generate), blends them per-type, extracts 3 feature
branches via `finetype extract-features`, and writes a binary .ftmb file.

Usage:
    python3 scripts/prepare_multibranch_data.py [OPTIONS]

Options:
    --distilled PATH        Distilled CSV (default: output/distillation-v3/sherlock_distilled.csv.gz)
    --finetype PATH         finetype binary (default: ./target/release/finetype)
    --output PATH           Output binary file (default: output/multibranch-training/blend-30-70.ftmb)
    --samples-per-type N    Target samples per type (default: 1500)
    --ratio-distilled F     Distilled ratio 0.0-1.0 (default: 0.3)
    --min-values N          Min values per column (default: 5)
    --seed N                Random seed (default: 42)
    --workers N             Parallel feature extraction workers (default: 4)
    --dry-run               Show counts without extracting features
    -h, --help              Show help
"""

import csv
import gzip
import json
import os
import random
import struct
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed

# ═══════════════════════════════════════════════════════════════════════════════
# Constants
# ═══════════════════════════════════════════════════════════════════════════════

CHAR_DIM = 960
EMBED_DIM = 512
STATS_DIM = 27
MAGIC = b"FTMB"
VERSION = 1

# Column-level types that may cause negative transfer (same as prepare_spike_data.py)
COLUMN_LEVEL_TYPES = {
    "representation.discrete.categorical",
    "representation.discrete.ordinal",
    "representation.identifier.increment",
}


# ═══════════════════════════════════════════════════════════════════════════════
# Data loading (reused from prepare_spike_data.py)
# ═══════════════════════════════════════════════════════════════════════════════


def load_taxonomy_types(finetype_bin):
    """Get the full list of 250 taxonomy types from finetype."""
    result = subprocess.run(
        [finetype_bin, "taxonomy", "--output", "csv"],
        capture_output=True,
        text=True,
        check=True,
    )
    types = set()
    reader = csv.DictReader(result.stdout.splitlines())
    for row in reader:
        key = row.get("key", "").strip()
        if key:
            types.add(key)
    return types


def load_distilled_columns(distilled_path, min_values):
    """Load distilled data as columns (groups of values per type).

    Returns:
        columns_by_type: dict[str, list[list[str]]] — each type has a list of columns,
                         each column being a list of string values.
        stats: dict with counts for logging
    """
    columns_by_type = defaultdict(list)
    stats = {
        "total_rows": 0,
        "qualifying_rows": 0,
        "sparse_rows": 0,
        "parse_errors": 0,
        "empty_label": 0,
        "excluded_column_types": 0,
        "total_values": 0,
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

            # Skip column-level types
            if label in COLUMN_LEVEL_TYPES:
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
            # Keep as a column (list of values)
            clean_vals = [str(v).strip() for v in vals if str(v).strip()]
            if len(clean_vals) >= min_values:
                columns_by_type[label].append(clean_vals)
                stats["total_values"] += len(clean_vals)

    return dict(columns_by_type), stats


def generate_synthetic_columns(finetype_bin, samples_per_type, seed, min_values):
    """Generate synthetic training data via finetype generate, grouped as columns.

    Returns: dict[str, list[list[str]]] — each type has one "column" of values
    """
    with tempfile.NamedTemporaryFile(suffix=".ndjson", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        subprocess.run(
            [
                finetype_bin,
                "generate",
                "--samples",
                str(samples_per_type),
                "--seed",
                str(seed),
                "--output",
                tmp_path,
            ],
            capture_output=True,
            text=True,
            check=True,
        )

        values_by_type = defaultdict(list)
        with open(tmp_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rec = json.loads(line)
                values_by_type[rec["classification"]].append(rec["text"])

        # Convert to columns: chunk each type's values into column-sized groups
        columns_by_type = {}
        for type_key, values in values_by_type.items():
            if len(values) < min_values:
                continue
            # Create columns of ~100 values each (matching typical column sizes)
            col_size = min(100, len(values))
            columns = []
            for i in range(0, len(values), col_size):
                chunk = values[i : i + col_size]
                if len(chunk) >= min_values:
                    columns.append(chunk)
            columns_by_type[type_key] = columns

        return columns_by_type
    finally:
        os.unlink(tmp_path)


def blend_columns(distilled, synthetic, ratio_distilled, samples_per_type, rng):
    """Blend distilled and synthetic column data per-type with capping.

    samples_per_type: target number of columns per type
    ratio_distilled: float 0.0-1.0 (e.g. 0.3 means 30% distilled)
    Returns: dict[str, list[list[str]]]
    """
    all_types = set(distilled.keys()) | set(synthetic.keys())
    blended = {}

    for type_key in sorted(all_types):
        d_cols = distilled.get(type_key, [])
        s_cols = synthetic.get(type_key, [])

        target_d = int(samples_per_type * ratio_distilled)
        target_s = samples_per_type - target_d

        # Cap at available, no oversampling. Fill remainder from other source.
        if len(d_cols) < target_d:
            actual_d = len(d_cols)
            actual_s = min(len(s_cols), samples_per_type - actual_d)
        elif len(s_cols) < target_s:
            actual_s = len(s_cols)
            actual_d = min(len(d_cols), samples_per_type - actual_s)
        else:
            actual_d = target_d
            actual_s = target_s

        picked_d = rng.sample(d_cols, actual_d) if actual_d <= len(d_cols) else d_cols[:]
        picked_s = rng.sample(s_cols, actual_s) if actual_s <= len(s_cols) else s_cols[:]

        combined = picked_d + picked_s
        rng.shuffle(combined)
        if combined:
            blended[type_key] = combined

    return blended


# ═══════════════════════════════════════════════════════════════════════════════
# Feature extraction
# ═══════════════════════════════════════════════════════════════════════════════


def extract_features(finetype_bin, values, header=None):
    """Call `finetype extract-features` to get feature vectors for a column.

    Returns: dict with 'char', 'embed', 'stats' arrays, or None on failure.
    """
    cmd = [finetype_bin, "extract-features", "--json"]
    if header:
        cmd.extend(["--header", header])

    try:
        result = subprocess.run(
            cmd,
            input=json.dumps(values),
            capture_output=True,
            text=True,
            timeout=60,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout.strip())
    except (subprocess.TimeoutExpired, json.JSONDecodeError, Exception) as e:
        print(f"  Warning: feature extraction failed: {e}", file=sys.stderr)
        return None


# ═══════════════════════════════════════════════════════════════════════════════
# Binary file I/O
# ═══════════════════════════════════════════════════════════════════════════════


def write_ftmb(path, records):
    """Write records to a .ftmb binary file.

    records: list of (label: str, char_features: list[float], embed_features: list[float], stats_features: list[float])

    Header (24 bytes):
        magic: b"FTMB" (4 bytes)
        version: uint32 = 1
        n_records: uint64
        char_dim: uint16 = 960
        embed_dim: uint16 = 512
        stats_dim: uint16 = 27
        padding: 2 bytes of zeros

    Each record:
        label_len: uint16
        label: bytes (UTF-8 type key)
        char_features: 960 x float32 (little-endian)
        embed_features: 512 x float32 (little-endian)
        stats_features: 27 x float32 (little-endian)
    """
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        # Header
        f.write(MAGIC)
        f.write(struct.pack("<I", VERSION))
        f.write(struct.pack("<Q", len(records)))
        f.write(struct.pack("<HHH", CHAR_DIM, EMBED_DIM, STATS_DIM))
        f.write(b"\x00\x00")  # padding

        for label, char_feat, embed_feat, stats_feat in records:
            label_bytes = label.encode("utf-8")
            f.write(struct.pack("<H", len(label_bytes)))
            f.write(label_bytes)
            f.write(struct.pack(f"<{CHAR_DIM}f", *char_feat))
            f.write(struct.pack(f"<{EMBED_DIM}f", *embed_feat))
            f.write(struct.pack(f"<{STATS_DIM}f", *stats_feat))


def read_ftmb(path):
    """Read a .ftmb binary file, yielding (label, char_feat, embed_feat, stats_feat) tuples."""
    with open(path, "rb") as f:
        magic = f.read(4)
        assert magic == MAGIC, f"Bad magic: {magic}"
        (version,) = struct.unpack("<I", f.read(4))
        assert version == VERSION, f"Unknown version: {version}"
        (n_records,) = struct.unpack("<Q", f.read(8))
        char_dim, embed_dim, stats_dim = struct.unpack("<HHH", f.read(6))
        _padding = f.read(2)

        assert char_dim == CHAR_DIM, f"char_dim mismatch: {char_dim}"
        assert embed_dim == EMBED_DIM, f"embed_dim mismatch: {embed_dim}"
        assert stats_dim == STATS_DIM, f"stats_dim mismatch: {stats_dim}"

        records = []
        for _ in range(n_records):
            (label_len,) = struct.unpack("<H", f.read(2))
            label = f.read(label_len).decode("utf-8")
            char_feat = list(struct.unpack(f"<{CHAR_DIM}f", f.read(CHAR_DIM * 4)))
            embed_feat = list(struct.unpack(f"<{EMBED_DIM}f", f.read(EMBED_DIM * 4)))
            stats_feat = list(struct.unpack(f"<{STATS_DIM}f", f.read(STATS_DIM * 4)))
            records.append((label, char_feat, embed_feat, stats_feat))

        return records


# ═══════════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════════


def main():
    args = sys.argv[1:]

    # Defaults
    distilled_path = "output/distillation-v3/sherlock_distilled.csv.gz"
    finetype_bin = "./target/release/finetype"
    output_path = "output/multibranch-training/blend-30-70.ftmb"
    samples_per_type = 1500
    ratio_distilled = 0.3
    min_values = 5
    seed = 42
    workers = 4
    dry_run = False

    i = 0
    while i < len(args):
        if args[i] == "--distilled":
            distilled_path = args[i + 1]
            i += 2
        elif args[i] == "--finetype":
            finetype_bin = args[i + 1]
            i += 2
        elif args[i] == "--output":
            output_path = args[i + 1]
            i += 2
        elif args[i] == "--samples-per-type":
            samples_per_type = int(args[i + 1])
            i += 2
        elif args[i] == "--ratio-distilled":
            ratio_distilled = float(args[i + 1])
            i += 2
        elif args[i] == "--min-values":
            min_values = int(args[i + 1])
            i += 2
        elif args[i] == "--seed":
            seed = int(args[i + 1])
            i += 2
        elif args[i] == "--workers":
            workers = int(args[i + 1])
            i += 2
        elif args[i] == "--dry-run":
            dry_run = True
            i += 1
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    rng = random.Random(seed)

    # ─── Load taxonomy ─────────────────────────────────────────────
    print("Loading taxonomy...")
    taxonomy_types = load_taxonomy_types(finetype_bin)
    print(f"  {len(taxonomy_types)} taxonomy types")

    # ─── Load distilled data ───────────────────────────────────────
    print(f"\nLoading distilled data (min_values={min_values})...")
    distilled, d_stats = load_distilled_columns(distilled_path, min_values)
    print(f"  {d_stats['total_rows']} total rows")
    print(f"  {d_stats['qualifying_rows']} qualifying rows")
    print(f"  {d_stats['sparse_rows']} sparse rows (skipped)")
    print(f"  {d_stats['parse_errors']} parse errors (skipped)")
    print(f"  {d_stats['empty_label']} empty labels (skipped)")
    print(f"  {d_stats['excluded_column_types']} column-level types (excluded)")
    total_d_cols = sum(len(cols) for cols in distilled.values())
    print(f"  {total_d_cols} columns across {len(distilled)} types")
    print(f"  {d_stats['total_values']} individual values")

    # ─── Generate synthetic data ───────────────────────────────────
    print(f"\nGenerating synthetic data ({samples_per_type} samples/type)...")
    synthetic = generate_synthetic_columns(finetype_bin, samples_per_type, seed, min_values)
    total_s_cols = sum(len(cols) for cols in synthetic.values())
    print(f"  {total_s_cols} columns across {len(synthetic)} types")

    # ─── Validate labels ───────────────────────────────────────────
    bad_distilled = set(distilled.keys()) - taxonomy_types
    if bad_distilled:
        print(f"\n  WARNING: {len(bad_distilled)} distilled types not in taxonomy:")
        for t in sorted(bad_distilled):
            print(f"    {t} ({len(distilled[t])} columns)")
        for t in bad_distilled:
            del distilled[t]

    # ─── Blend ────────────────────────────────────────────────────
    print(f"\nBlending data ({ratio_distilled:.0%} distilled, {1-ratio_distilled:.0%} synthetic)...")
    blended = blend_columns(distilled, synthetic, ratio_distilled, samples_per_type, rng)
    total_blended = sum(len(cols) for cols in blended.values())
    print(f"  {total_blended} columns across {len(blended)} types")

    # ─── Type coverage summary ─────────────────────────────────────
    distilled_types = set(distilled.keys())
    synthetic_types = set(synthetic.keys())
    blended_types = set(blended.keys())
    missing_types = taxonomy_types - blended_types

    print(f"\n{'='*60}")
    print(f"Type coverage:")
    print(f"  Taxonomy:   {len(taxonomy_types)} types")
    print(f"  Distilled:  {len(distilled_types)} types ({total_d_cols} columns)")
    print(f"  Synthetic:  {len(synthetic_types)} types ({total_s_cols} columns)")
    print(f"  Blended:    {len(blended_types)} types ({total_blended} columns)")
    print(f"  Missing:    {len(missing_types)} types (no source)")
    if missing_types and len(missing_types) <= 10:
        for t in sorted(missing_types):
            print(f"    {t}")
    print(f"{'='*60}")

    if dry_run:
        print(f"\n[DRY RUN] Would extract features for {total_blended} columns")
        print(f"  Output: {output_path}")
        print(f"  Record size: {2 + 30 + CHAR_DIM*4 + EMBED_DIM*4 + STATS_DIM*4} bytes (avg)")
        est_size_mb = total_blended * (CHAR_DIM + EMBED_DIM + STATS_DIM) * 4 / (1024 * 1024)
        print(f"  Estimated file size: ~{est_size_mb:.0f} MB")
        return

    # ─── Extract features ──────────────────────────────────────────
    print(f"\nExtracting features for {total_blended} columns (workers={workers})...")

    # Build work items: (type_key, column_values)
    work_items = []
    for type_key in sorted(blended.keys()):
        for col_values in blended[type_key]:
            work_items.append((type_key, col_values))

    records = []
    errors = 0
    start_time = time.time()

    def process_item(item):
        type_key, col_values = item
        features = extract_features(finetype_bin, col_values)
        return type_key, features

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(process_item, item): item for item in work_items}
        for i, future in enumerate(as_completed(futures)):
            type_key, features = future.result()

            if features is None:
                errors += 1
                continue

            char_feat = features.get("char", [0.0] * CHAR_DIM)
            embed_feat = features.get("embed", [0.0] * EMBED_DIM)
            stats_feat = features.get("stats", [0.0] * STATS_DIM)

            # Validate dimensions
            if len(char_feat) != CHAR_DIM:
                print(f"  Warning: {type_key} char dim {len(char_feat)} != {CHAR_DIM}", file=sys.stderr)
                errors += 1
                continue
            if len(embed_feat) != EMBED_DIM:
                print(f"  Warning: {type_key} embed dim {len(embed_feat)} != {EMBED_DIM}", file=sys.stderr)
                errors += 1
                continue
            if len(stats_feat) != STATS_DIM:
                print(f"  Warning: {type_key} stats dim {len(stats_feat)} != {STATS_DIM}", file=sys.stderr)
                errors += 1
                continue

            records.append((type_key, char_feat, embed_feat, stats_feat))

            # Progress
            done = i + 1
            if done % 100 == 0 or done == len(work_items):
                elapsed = time.time() - start_time
                rate = done / elapsed if elapsed > 0 else 0
                eta = (len(work_items) - done) / rate if rate > 0 else 0
                print(
                    f"  [{done}/{len(work_items)}] {rate:.1f} cols/sec, "
                    f"ETA {eta/60:.1f}min, {errors} errors",
                    file=sys.stderr,
                )

    # ─── Write binary file ─────────────────────────────────────────
    print(f"\nWriting {len(records)} records to {output_path}...")
    write_ftmb(output_path, records)

    file_size = os.path.getsize(output_path)
    print(f"  File size: {file_size / (1024*1024):.1f} MB")

    # ─── Summary ──────────────────────────────────────────────────
    type_counts = Counter(r[0] for r in records)
    elapsed = time.time() - start_time

    print(f"\n{'='*60}")
    print(f"Summary:")
    print(f"  Records written: {len(records)}")
    print(f"  Types covered:   {len(type_counts)}")
    print(f"  Extraction errors: {errors}")
    print(f"  Time: {elapsed:.1f}s ({elapsed/60:.1f}min)")
    print(f"  Output: {output_path}")
    print(f"{'='*60}")

    # Write manifest
    manifest_path = output_path.replace(".ftmb", ".manifest.json")
    manifest = {
        "seed": seed,
        "samples_per_type": samples_per_type,
        "ratio_distilled": ratio_distilled,
        "min_values": min_values,
        "distilled_source": distilled_path,
        "taxonomy_types": len(taxonomy_types),
        "blended_types": len(blended_types),
        "records_written": len(records),
        "types_covered": len(type_counts),
        "errors": errors,
        "dimensions": {
            "char": CHAR_DIM,
            "embed": EMBED_DIM,
            "stats": STATS_DIM,
        },
        "type_counts": dict(type_counts.most_common()),
    }
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"Manifest: {manifest_path}")
    print("Done.")


if __name__ == "__main__":
    main()
