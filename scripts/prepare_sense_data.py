#!/usr/bin/env python3
"""NNFT-163: Prepare training data for Sense model spike.

Extracts column-level features from SOTAB column_values.parquet:
- Samples up to N values per column (stratified by frequency)
- Maps 91 Schema.org GT labels → 6 broad categories
- Maps entity columns → 4 entity subtypes
- Splits train/val (80/20, stratified)
- Outputs Parquet files ready for training

Usage:
    python3 scripts/prepare_sense_data.py
    python3 scripts/prepare_sense_data.py --max-values 20 --output data/sense_spike/
"""

import argparse
import json
import random
from collections import Counter, defaultdict
from pathlib import Path

import duckdb
import numpy as np

# ── Label mappings ──────────────────────────────────────────────────

# Map SOTAB GT labels → 6 broad categories
BROAD_CATEGORY_MAP = {
    # ENTITY - named things
    "Organization": "entity", "Person": "entity", "Person/name": "entity",
    "Place": "entity", "Place/name": "entity", "MusicArtistAT": "entity",
    "LocalBusiness/name": "entity", "Hotel/name": "entity",
    "Restaurant/name": "entity", "Brand": "entity", "SportsTeam": "entity",
    "EducationalOrganization": "entity", "MusicGroup": "entity",
    "Museum/name": "entity", "MusicAlbum": "entity",
    "MusicRecording/name": "entity", "Event/name": "entity",
    "Book/name": "entity", "Recipe/name": "entity", "Movie/name": "entity",
    "CreativeWork/name": "entity", "SportsEvent/name": "entity",
    "TVEpisode/name": "entity", "MusicAlbum/name": "entity",
    "JobPosting/name": "entity", "CreativeWork": "entity",
    "CreativeWorkSeries": "entity", "Product/name": "entity",
    "ProductModel": "entity",
    # FORMAT - structured identifiers, codes, URLs
    "URL": "format", "email": "format", "telephone": "format",
    "faxNumber": "format", "postalCode": "format", "IdentifierAT": "format",
    "identifierNameAP": "format", "unitCode": "format",
    "CategoryCode": "format",
    # TEMPORAL - dates, times, durations
    "Date": "temporal", "DateTime": "temporal", "Duration": "temporal",
    "Time": "temporal", "DayOfWeek": "temporal", "openingHours": "temporal",
    "workHours": "temporal",
    # NUMERIC - numbers, measurements, quantities
    "Number": "numeric", "Integer": "numeric", "Mass": "numeric",
    "Distance": "numeric", "Energy": "numeric", "weight": "numeric",
    "QuantitativeValue": "numeric", "price": "numeric",
    "priceRange": "numeric", "currency": "numeric",
    "MonetaryAmount": "numeric", "CoordinateAT": "numeric",
    "Rating": "numeric", "typicalAgeRange": "numeric",
    # GEOGRAPHIC - locations, addresses
    "addressLocality": "geographic", "addressRegion": "geographic",
    "Country": "geographic", "streetAddress": "geographic",
    "PostalAddress": "geographic",
    # TEXT - free text, categories, enums, descriptions
    "Text": "text", "category": "text", "ItemAvailability": "text",
    "ItemList": "text", "Review": "text", "EventStatusType": "text",
    "BookFormatType": "text", "Language": "text", "Thing": "text",
    "GenderType": "text", "EventAttendanceModeEnumeration": "text",
    "OccupationalExperienceRequirements": "text", "unitText": "text",
    "OfferItemCondition": "text", "Boolean": "text",
    "paymentAccepted": "text", "Photograph": "text", "Offer": "text",
    "Action": "text", "DeliveryMethod": "text", "RestrictedDiet": "text",
    "Product": "text", "LocationFeatureSpecification": "text",
    "audience": "text", "MusicRecording": "text", "WarrantyPromise": "text",
    "EducationalOccupationalCredential": "text",
}

# Map entity GT labels → 4 subtypes
ENTITY_SUBTYPE_MAP = {
    "Person": "person", "Person/name": "person", "MusicArtistAT": "person",
    "Place": "place", "Place/name": "place", "Hotel/name": "place",
    "Restaurant/name": "place", "Museum/name": "place",
    "Organization": "organization", "LocalBusiness/name": "organization",
    "Brand": "organization", "SportsTeam": "organization",
    "EducationalOrganization": "organization", "MusicGroup": "organization",
    "MusicAlbum": "creative_work", "MusicRecording/name": "creative_work",
    "Event/name": "creative_work", "Book/name": "creative_work",
    "Recipe/name": "creative_work", "Movie/name": "creative_work",
    "CreativeWork/name": "creative_work", "SportsEvent/name": "creative_work",
    "TVEpisode/name": "creative_work", "MusicAlbum/name": "creative_work",
    "JobPosting/name": "creative_work", "CreativeWork": "creative_work",
    "CreativeWorkSeries": "creative_work", "Product/name": "creative_work",
    "ProductModel": "creative_work",
}

BROAD_CATEGORIES = ["entity", "format", "temporal", "numeric", "geographic", "text"]
ENTITY_SUBTYPES = ["person", "place", "organization", "creative_work"]


def sample_values(values: list[str], max_n: int, seed: int = 42) -> list[str]:
    """Sample up to max_n values using stratified frequency-weighted sampling.

    Strategy: take top-K most frequent values to preserve distribution signal,
    then fill remaining slots with random diverse values. This gives the model
    both common patterns and rare variants.
    """
    if len(values) <= max_n:
        return values

    rng = random.Random(seed)

    # Count frequencies
    freq = Counter(values)
    unique_vals = list(freq.keys())

    # Take top half by frequency
    top_k = max_n // 2
    by_freq = sorted(unique_vals, key=lambda v: freq[v], reverse=True)
    selected = set(by_freq[:top_k])

    # Fill remaining with random diverse values (not already selected)
    remaining = [v for v in unique_vals if v not in selected]
    fill_n = max_n - len(selected)
    if fill_n > 0 and remaining:
        rng.shuffle(remaining)
        selected.update(remaining[:fill_n])

    # If still short (fewer unique values than max_n), add duplicates
    result = list(selected)
    if len(result) < max_n:
        extras = [v for v in values if v not in selected]
        rng.shuffle(extras)
        result.extend(extras[: max_n - len(result)])

    return result[:max_n]


def load_sotab_columns(
    parquet_path: Path,
) -> list[dict]:
    """Load SOTAB columns and group values."""
    con = duckdb.connect()
    rows = con.execute(
        f"""SELECT table_name, col_index, gt_label, col_value
            FROM read_parquet('{parquet_path}')"""
    ).fetchall()
    con.close()

    # Group by column
    columns: dict[tuple[str, int], list[str]] = defaultdict(list)
    gt_labels: dict[tuple[str, int], str] = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = (table_name, int(col_index))
        if col_value is not None:
            columns[key].append(str(col_value))
        gt_labels[key] = gt_label

    result = []
    for key, values in columns.items():
        table_name, col_index = key
        gt_label = gt_labels[key]
        broad_cat = BROAD_CATEGORY_MAP.get(gt_label)
        if broad_cat is None:
            continue  # Skip unmapped labels

        entity_subtype = ENTITY_SUBTYPE_MAP.get(gt_label)

        result.append({
            "table_name": table_name,
            "col_index": col_index,
            "gt_label": gt_label,
            "broad_category": broad_cat,
            "entity_subtype": entity_subtype,  # None for non-entity columns
            "values": values,
        })

    return result


def load_profile_columns(
    datasets_dir: Path,
    manifest_path: Path,
    schema_mapping_path: Path,
) -> list[dict]:
    """Load profile eval columns as additional training/test data."""
    import csv
    import yaml

    # Load schema mapping for broad category inference
    with open(schema_mapping_path) as f:
        mappings = yaml.safe_load(f)

    # Build gt_label → finetype_label lookup
    label_to_ft = {}
    for entry in mappings:
        label_to_ft[entry["gt_label"]] = entry["finetype_label"]

    # Map FineType labels to broad categories
    def ft_to_broad(ft_label: str) -> str:
        domain = ft_label.split(".")[0] if "." in ft_label else ""
        if domain == "datetime":
            return "temporal"
        elif domain == "geography":
            return "geographic"
        elif domain == "identity":
            return "entity"  # person names, emails, etc.
        elif domain == "technology":
            return "format"  # URLs, codes, etc.
        elif domain == "representation":
            cat = ft_label.split(".")[1] if ft_label.count(".") >= 1 else ""
            if cat in ("numeric", "scientific"):
                return "numeric"
            elif cat in ("boolean", "discrete"):
                return "text"
            else:
                return "text"
        elif domain == "container":
            return "format"
        return "text"

    # Load manifest
    manifest_rows = []
    with open(manifest_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            manifest_rows.append(row)

    result = []
    for row in manifest_rows:
        dataset = row["dataset"]
        file_path = Path(row["file_path"])
        col_name = row["column_name"]
        gt_label = row["gt_label"]

        # Resolve file path
        if not file_path.is_absolute():
            file_path = datasets_dir / file_path

        if not file_path.exists():
            continue

        # Read column values
        try:
            con = duckdb.connect()
            values = con.execute(
                f"""SELECT CAST("{col_name}" AS VARCHAR)
                    FROM read_csv_auto('{file_path}')
                    WHERE "{col_name}" IS NOT NULL"""
            ).fetchall()
            con.close()
            values = [str(v[0]) for v in values if v[0] is not None]
        except Exception:
            continue

        if not values:
            continue

        ft_label = label_to_ft.get(gt_label, "")
        broad_cat = ft_to_broad(ft_label) if ft_label else "text"

        result.append({
            "table_name": f"profile_{dataset}",
            "col_index": 0,
            "gt_label": gt_label,
            "broad_category": broad_cat,
            "entity_subtype": None,
            "header": col_name,  # Profile eval has real column names!
            "values": values,
        })

    return result


def main():
    parser = argparse.ArgumentParser(description="Prepare Sense model training data")
    parser.add_argument(
        "--max-values", type=int, default=50,
        help="Max values to sample per column (default: 50)",
    )
    parser.add_argument(
        "--output", type=str, default="data/sense_spike",
        help="Output directory (default: data/sense_spike)",
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed (default: 42)",
    )
    parser.add_argument(
        "--val-fraction", type=float, default=0.2,
        help="Validation fraction (default: 0.2)",
    )
    parser.add_argument(
        "--sotab-dir", type=str,
        default=str(Path.home() / "datasets/sotab/cta"),
        help="SOTAB CTA directory",
    )
    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    sotab_dir = Path(args.sotab_dir)
    rng = random.Random(args.seed)

    # ── Load SOTAB validation columns ──
    val_parquet = sotab_dir / "validation" / "column_values.parquet"
    print(f"Loading SOTAB validation from {val_parquet}...")
    columns = load_sotab_columns(val_parquet)
    print(f"  Loaded {len(columns)} columns")

    # Also load SOTAB test set if available
    test_parquet = sotab_dir / "test" / "column_values.parquet"
    if test_parquet.exists():
        print(f"Loading SOTAB test from {test_parquet}...")
        test_cols = load_sotab_columns(test_parquet)
        print(f"  Loaded {len(test_cols)} columns")
        columns.extend(test_cols)

    # ── Category distribution ──
    cat_counts = Counter(c["broad_category"] for c in columns)
    print(f"\nBroad category distribution ({len(columns)} total):")
    for cat in BROAD_CATEGORIES:
        print(f"  {cat:12s}: {cat_counts.get(cat, 0):5d}")

    entity_cols = [c for c in columns if c["broad_category"] == "entity"]
    sub_counts = Counter(c["entity_subtype"] for c in entity_cols)
    print(f"\nEntity subtype distribution ({len(entity_cols)} entity columns):")
    for sub in ENTITY_SUBTYPES:
        print(f"  {sub:15s}: {sub_counts.get(sub, 0):5d}")

    # ── Sample values ──
    print(f"\nSampling up to {args.max_values} values per column...")
    for col in columns:
        col["sampled_values"] = sample_values(
            col["values"], args.max_values, seed=args.seed
        )
        # SOTAB has integer column indices — no meaningful header
        if "header" not in col:
            col["header"] = None

    # ── Train/val split (stratified by broad category) ──
    print(f"Splitting train/val ({1-args.val_fraction:.0%}/{args.val_fraction:.0%})...")

    # Group by category for stratified split
    by_category: dict[str, list[dict]] = defaultdict(list)
    for col in columns:
        by_category[col["broad_category"]].append(col)

    train_cols = []
    val_cols = []
    for cat, cat_cols in by_category.items():
        rng.shuffle(cat_cols)
        split_idx = int(len(cat_cols) * (1 - args.val_fraction))
        train_cols.extend(cat_cols[:split_idx])
        val_cols.extend(cat_cols[split_idx:])

    rng.shuffle(train_cols)
    rng.shuffle(val_cols)

    print(f"  Train: {len(train_cols)}, Val: {len(val_cols)}")

    # ── Write output ──
    def write_jsonl(path: Path, data: list[dict]):
        with open(path, "w") as f:
            for item in data:
                record = {
                    "table_name": item["table_name"],
                    "col_index": item["col_index"],
                    "gt_label": item["gt_label"],
                    "broad_category": item["broad_category"],
                    "broad_category_idx": BROAD_CATEGORIES.index(item["broad_category"]),
                    "entity_subtype": item["entity_subtype"],
                    "entity_subtype_idx": (
                        ENTITY_SUBTYPES.index(item["entity_subtype"])
                        if item["entity_subtype"]
                        else -1
                    ),
                    "header": item.get("header"),
                    "values": item["sampled_values"],
                    "n_original_values": len(item["values"]),
                }
                f.write(json.dumps(record, ensure_ascii=False) + "\n")

    train_path = output_dir / "train.jsonl"
    val_path = output_dir / "val.jsonl"

    write_jsonl(train_path, train_cols)
    write_jsonl(val_path, val_cols)

    # Write label indices
    meta = {
        "broad_categories": BROAD_CATEGORIES,
        "entity_subtypes": ENTITY_SUBTYPES,
        "max_values": args.max_values,
        "seed": args.seed,
        "n_train": len(train_cols),
        "n_val": len(val_cols),
        "broad_category_map": BROAD_CATEGORY_MAP,
        "entity_subtype_map": ENTITY_SUBTYPE_MAP,
    }
    with open(output_dir / "meta.json", "w") as f:
        json.dump(meta, f, indent=2)

    print(f"\nWrote {train_path} ({len(train_cols)} columns)")
    print(f"Wrote {val_path} ({len(val_cols)} columns)")
    print(f"Wrote {output_dir / 'meta.json'}")

    # ── Summary stats ──
    train_cat = Counter(c["broad_category"] for c in train_cols)
    val_cat = Counter(c["broad_category"] for c in val_cols)
    print("\nCategory split:")
    print(f"  {'Category':12s} {'Train':>6s} {'Val':>6s}")
    for cat in BROAD_CATEGORIES:
        print(f"  {cat:12s} {train_cat.get(cat, 0):6d} {val_cat.get(cat, 0):6d}")


if __name__ == "__main__":
    main()
