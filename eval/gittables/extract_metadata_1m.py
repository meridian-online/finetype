#!/usr/bin/env python3
"""Extract GitTables metadata from parquet files for 1M evaluation.

Samples tables per topic, extracts gittables metadata, writes CSV for DuckDB analysis.
"""
import json
import os
import random
import csv
import sys
from pathlib import Path

GITTABLES_DIR = Path(os.environ.get("GITTABLES_DIR", os.path.expanduser("~/datasets/gittables")))
TOPICS_DIR = GITTABLES_DIR / "topics"
OUTPUT_DIR = Path(os.environ.get("EVAL_OUTPUT", str(GITTABLES_DIR / "eval_output")))
DEFAULT_SAMPLE_PER_TOPIC = 50
random.seed(42)

def parse_args():
    import argparse
    parser = argparse.ArgumentParser(description="Extract GitTables metadata from parquet files.")
    parser.add_argument("--samples-per-topic", type=int, default=DEFAULT_SAMPLE_PER_TOPIC,
                        help=f"Number of tables to sample per topic (default: {DEFAULT_SAMPLE_PER_TOPIC})")
    parser.add_argument("--output-dir", type=str, default=str(OUTPUT_DIR),
                        help=f"Output directory (default: {OUTPUT_DIR})")
    return parser.parse_args()

def extract_parquet_metadata(filepath):
    """Extract gittables metadata from parquet file using pyarrow."""
    try:
        import pyarrow.parquet as pq
        pf = pq.ParquetFile(filepath)
        meta = pf.schema_arrow.pandas_metadata
        # Try gittables key from file metadata
        file_meta = pf.schema_arrow.metadata or {}
        gt_raw = file_meta.get(b'gittables', None)
        if gt_raw:
            return json.loads(gt_raw)
    except Exception:
        pass
    return None

def main():
    args = parse_args()
    sample_per_topic = args.samples_per_topic
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)

    # Catalog all topics
    topics = sorted([d for d in TOPICS_DIR.iterdir() if d.is_dir()])
    print(f"Found {len(topics)} topics (sampling {sample_per_topic}/topic)")

    catalog_rows = []
    metadata_rows = []
    total_files = 0
    total_sampled = 0
    total_annotated = 0

    for topic_dir in topics:
        topic = topic_dir.name
        parquet_files = list(topic_dir.glob("*.parquet"))
        total_files += len(parquet_files)

        # Sample
        sample = random.sample(parquet_files, min(sample_per_topic, len(parquet_files)))
        total_sampled += len(sample)

        # Catalog entry
        catalog_rows.append({
            'topic': topic,
            'total_tables': len(parquet_files),
            'sampled_tables': len(sample),
        })

        # Extract metadata from sample
        annotated_count = 0
        for fp in sample:
            meta = extract_parquet_metadata(fp)
            if meta is None:
                continue

            has_schema = bool(meta.get('schema_semantic_column_types', {}))
            has_dbpedia = bool(meta.get('dbpedia_semantic_column_types', {}))
            nrows = meta.get('number_rows', 0)
            ncols = meta.get('number_columns', 0)

            # Extract column annotations
            schema_types = meta.get('schema_semantic_column_types', {})
            dbpedia_types = meta.get('dbpedia_semantic_column_types', {})

            # Prefer schema.org, fall back to dbpedia
            annotations = {}
            for col, info in dbpedia_types.items():
                if isinstance(info, dict):
                    annotations[col] = info.get('cleaned_label', info.get('id', 'unknown'))
                else:
                    annotations[col] = str(info)
            for col, info in schema_types.items():
                if isinstance(info, dict):
                    annotations[col] = info.get('cleaned_label', info.get('id', 'unknown'))
                else:
                    annotations[col] = str(info)

            if annotations:
                annotated_count += 1

            metadata_rows.append({
                'topic': topic,
                'table_name': fp.stem,
                'file_path': str(fp),
                'nrows': nrows,
                'ncols': ncols,
                'has_schema': has_schema,
                'has_dbpedia': has_dbpedia,
                'n_annotated_cols': len(annotations),
                'annotations_json': json.dumps(annotations) if annotations else '',
            })

        total_annotated += annotated_count
        ann_pct = (annotated_count / len(sample) * 100) if sample else 0
        print(f"  {topic}: {len(parquet_files)} tables, sampled {len(sample)}, "
              f"{annotated_count} annotated ({ann_pct:.0f}%)")

    # Write catalog
    with open(output_dir / 'catalog.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, fieldnames=['topic', 'total_tables', 'sampled_tables'])
        w.writeheader()
        w.writerows(catalog_rows)

    # Write metadata
    with open(output_dir / 'metadata.csv', 'w', newline='') as f:
        fields = ['topic', 'table_name', 'file_path', 'nrows', 'ncols',
                  'has_schema', 'has_dbpedia', 'n_annotated_cols', 'annotations_json']
        w = csv.DictWriter(f, fieldnames=fields)
        w.writeheader()
        w.writerows(metadata_rows)

    # Write file list (for DuckDB read_parquet)
    with open(output_dir / 'sampled_files.txt', 'w') as f:
        for row in metadata_rows:
            f.write(row['file_path'] + '\n')

    print(f"\n=== Summary ===")
    print(f"Topics: {len(topics)}")
    print(f"Total tables: {total_files}")
    print(f"Sampled: {total_sampled}")
    print(f"With annotations: {total_annotated} ({total_annotated/total_sampled*100:.1f}%)")
    print(f"\nOutput: {output_dir}/")
    print(f"  catalog.csv: {len(catalog_rows)} rows")
    print(f"  metadata.csv: {len(metadata_rows)} rows")
    print(f"  sampled_files.txt: {len(metadata_rows)} paths")

if __name__ == '__main__':
    main()
