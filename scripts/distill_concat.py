#!/usr/bin/env python3
"""Concatenate distillation batch CSVs into one gzipped file per source.

Usage:
    python3 scripts/distill_concat.py [--dest output/distillation-v3/] [--dry-run]

Only includes batches that have a .done marker (bad/incomplete batches are
naturally excluded). Validates each row and logs exclusions.

Output: <dest>/<source>_distilled.csv.gz
Log:    <dest>/distill_concat.log
"""

import csv
import gzip
import glob
import os
import re
import sys
from collections import Counter, defaultdict

DEST = "output/distillation-v3"

EXPECTED_COLUMNS = [
    "source", "source_file", "column_name", "sample_values",
    "blind_label", "blind_confidence", "finetype_label", "finetype_confidence",
    "agreement", "final_label", "reasoning",
    "ground_truth_label", "ground_truth_source",
]

VALID_DOMAINS = {
    "container", "datetime", "finance", "geography",
    "identity", "representation", "technology",
}

VALID_CONFIDENCE = {"high", "medium", "low"}
VALID_AGREEMENT = {"yes", "no", "agree", "disagree"}

MAX_EXCLUDE_PCT = 1.0  # Fail if >1% of rows excluded


def validate_label(label):
    """Check if a label is a valid 3-part taxonomy key or empty."""
    if not label or not label.strip():
        return True  # empty is OK
    parts = label.strip().split(".")
    if len(parts) != 3:
        return False
    return parts[0] in VALID_DOMAINS


def get_done_batches(dest_dir):
    """Return set of batch_ids that have .done markers."""
    done = set()
    for fname in os.listdir(dest_dir):
        if fname.endswith(".done"):
            batch_id = fname[:-5]
            done.add(batch_id)
    return done


def find_batch_csvs(dest_dir, source):
    """Find batch CSVs for a source that have matching .done markers."""
    done = get_done_batches(dest_dir)
    pattern = os.path.join(dest_dir, f"{source}_batch_*.csv")
    csvs = []
    skipped = []
    for path in sorted(glob.glob(pattern)):
        fname = os.path.basename(path)
        batch_id = fname[:-4]  # strip .csv
        # Skip test artifacts
        if "test" in batch_id:
            continue
        if batch_id in done:
            csvs.append(path)
        else:
            skipped.append((path, "no .done marker"))
    return csvs, skipped


def concat_source(dest_dir, source, log_file, dry_run=False):
    """Concatenate all valid batch CSVs for a source into a gzipped file."""
    csvs, skipped_files = find_batch_csvs(dest_dir, source)

    if not csvs:
        print(f"  No batch CSVs with .done markers for source '{source}'")
        return None

    output_path = os.path.join(dest_dir, f"{source}_distilled.csv.gz")

    print(f"  Found {len(csvs)} batch CSVs with .done markers")
    if skipped_files:
        print(f"  Skipped {len(skipped_files)} CSVs without .done markers:")
        for path, reason in skipped_files:
            print(f"    {os.path.basename(path)}: {reason}")
            log_file.write(f"SKIP_FILE\t{path}\t{reason}\n")

    # Track duplicates by (source_file, column_name)
    seen_keys = set()
    total_rows = 0
    valid_rows = 0
    excluded_rows = 0
    exclude_reasons = Counter()

    if dry_run:
        # Just count and validate without writing
        for csv_path in csvs:
            with open(csv_path, newline="") as f:
                reader = csv.DictReader(f)
                # Validate header
                if reader.fieldnames != EXPECTED_COLUMNS:
                    log_file.write(f"SKIP_FILE\t{csv_path}\tschema mismatch: {reader.fieldnames}\n")
                    print(f"  WARNING: Schema mismatch in {os.path.basename(csv_path)}")
                    continue
                for row in reader:
                    total_rows += 1
                    ok, reason = validate_row(row, seen_keys)
                    if ok:
                        valid_rows += 1
                    else:
                        excluded_rows += 1
                        exclude_reasons[reason] += 1

        print(f"  [DRY RUN] Would write {valid_rows} rows, exclude {excluded_rows}")
        return None

    # Write concatenated gzipped CSV
    with gzip.open(output_path, "wt", newline="") as gz:
        writer = csv.DictWriter(gz, fieldnames=EXPECTED_COLUMNS)
        writer.writeheader()

        for csv_path in csvs:
            batch_name = os.path.basename(csv_path)
            batch_valid = 0
            batch_excluded = 0

            with open(csv_path, newline="") as f:
                reader = csv.DictReader(f)

                # Validate header
                if reader.fieldnames != EXPECTED_COLUMNS:
                    log_file.write(f"SKIP_FILE\t{csv_path}\tschema mismatch: {reader.fieldnames}\n")
                    print(f"  WARNING: Schema mismatch in {batch_name}, skipping entire file")
                    continue

                for row_num, row in enumerate(reader, start=2):  # line 2+ (after header)
                    total_rows += 1
                    # Strip any extra fields from malformed CSVs
                    row.pop(None, None)
                    ok, reason = validate_row(row, seen_keys)
                    if ok:
                        # Only write expected columns
                        clean_row = {k: row.get(k, "") for k in EXPECTED_COLUMNS}
                        writer.writerow(clean_row)
                        valid_rows += 1
                        batch_valid += 1
                    else:
                        excluded_rows += 1
                        batch_excluded += 1
                        exclude_reasons[reason] += 1
                        log_file.write(
                            f"EXCLUDE\t{batch_name}\trow {row_num}\t{reason}\t"
                            f"final_label={row.get('final_label', '')}\t"
                            f"blind_confidence={row.get('blind_confidence', '')}\n"
                        )

            if batch_excluded > 0:
                print(f"    {batch_name}: {batch_valid} valid, {batch_excluded} excluded")

    # Summary
    print(f"\n  Total rows: {total_rows}")
    print(f"  Valid rows: {valid_rows}")
    print(f"  Excluded rows: {excluded_rows}")
    if exclude_reasons:
        print(f"  Exclusion reasons:")
        for reason, count in exclude_reasons.most_common():
            print(f"    {reason}: {count}")

    exclude_pct = 100 * excluded_rows / total_rows if total_rows > 0 else 0
    print(f"  Exclusion rate: {exclude_pct:.2f}%")

    if exclude_pct > MAX_EXCLUDE_PCT:
        print(f"\n  FAIL: Exclusion rate {exclude_pct:.2f}% exceeds {MAX_EXCLUDE_PCT}% threshold")
        print(f"  Review {os.path.join(dest_dir, 'distill_concat.log')} and fix or adjust threshold")
        return None

    # Verify output
    with gzip.open(output_path, "rt") as gz:
        line_count = sum(1 for _ in gz) - 1  # minus header
    print(f"\n  Output: {output_path}")
    print(f"  Verified: {line_count} rows (expected {valid_rows})")

    if line_count != valid_rows:
        print(f"  ERROR: Row count mismatch!")
        return None

    # File size
    size_bytes = os.path.getsize(output_path)
    if size_bytes > 1_000_000:
        print(f"  Size: {size_bytes / 1_000_000:.1f} MB")
    else:
        print(f"  Size: {size_bytes / 1_000:.1f} KB")

    return output_path


def validate_row(row, seen_keys):
    """Validate a single row. Returns (ok, reason)."""
    # Check for duplicate — use sample_values as part of key since
    # Sherlock data has empty source_file and column_name
    key = (
        row.get("source_file", ""),
        row.get("column_name", ""),
        row.get("sample_values", "")[:200],  # truncate for memory
    )
    if key[0] or key[1]:  # only dedup when we have meaningful keys
        if key in seen_keys:
            return False, "duplicate"
        seen_keys.add(key)

    # Check blind_confidence
    conf = row.get("blind_confidence", "").strip()
    if conf not in VALID_CONFIDENCE:
        return False, f"invalid_blind_confidence:{conf[:30]}"

    # Check and normalize agreement
    agree = row.get("agreement", "").strip()
    if agree not in VALID_AGREEMENT:
        return False, f"invalid_agreement:{agree[:30]}"
    # Normalize agree/disagree → yes/no
    if agree == "agree":
        row["agreement"] = "yes"
    elif agree == "disagree":
        row["agreement"] = "no"

    # Check final_label
    label = row.get("final_label", "").strip()
    if not validate_label(label):
        return False, f"invalid_final_label:{label[:50]}"

    return True, ""


def main():
    args = sys.argv[1:]
    dest_dir = DEST
    dry_run = False
    sources = ["sherlock"]  # Only sherlock for now; add others as they complete

    i = 0
    while i < len(args):
        if args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] == "--dry-run":
            dry_run = True
            i += 1
        elif args[i] == "--source":
            sources = [args[i + 1]]
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    log_path = os.path.join(dest_dir, "distill_concat.log")

    with open(log_path, "w") as log_file:
        log_file.write(f"# distill_concat.py log\n")

        for source in sources:
            print(f"\nProcessing source: {source}")
            result = concat_source(dest_dir, source, log_file, dry_run)
            if result:
                print(f"  Success: {result}")
            elif not dry_run:
                print(f"  FAILED for source '{source}'")
                sys.exit(1)

    print(f"\nLog written to: {log_path}")


if __name__ == "__main__":
    main()
