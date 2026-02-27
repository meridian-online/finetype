#!/usr/bin/env python3
"""FineType Actionability Evaluation (NNFT-147)

Tests whether FineType's format_string predictions actually work on real data.
For each column where FineType predicts a datetime type with a format_string,
runs TRY_STRPTIME on the actual data and measures the success rate.

This answers the analyst's question: "If FineType says this column is
ISO 8601 dates, can I safely TRY_CAST it?"

Usage:
    python3 eval/eval_actionability.py                    # default manifest
    python3 eval/eval_actionability.py --manifest path    # custom manifest

Prerequisites:
    1. Profile results: eval/eval_output/profile_results.csv
    2. Taxonomy YAML files in labels/
    3. Source CSV files referenced in the manifest
"""

import argparse
import csv
import sys
from pathlib import Path

import yaml

try:
    import duckdb
except ImportError:
    print("ERROR: duckdb Python package required. Install with: pip install duckdb", file=sys.stderr)
    sys.exit(1)


def load_format_strings(labels_dir: Path) -> dict[str, str]:
    """Load format_string for all types from taxonomy YAML files."""
    format_strings = {}
    for yaml_file in sorted(labels_dir.glob("definitions_*.yaml")):
        data = yaml.safe_load(yaml_file.read_text())
        for key, val in data.items():
            if isinstance(val, dict) and val.get("format_string") and val["format_string"] != "null":
                format_strings[key] = val["format_string"]
    return format_strings


def load_predictions(results_csv: Path) -> list[dict]:
    """Load profile eval predictions."""
    with open(results_csv) as f:
        return list(csv.DictReader(f))


def load_manifest(manifest_csv: Path) -> dict[tuple[str, str], str]:
    """Load manifest to get file_path for each (dataset, column_name)."""
    mapping = {}
    with open(manifest_csv) as f:
        for row in csv.DictReader(f):
            mapping[(row["dataset"], row["column_name"])] = row["file_path"]
    return mapping


def test_actionability(
    predictions: list[dict],
    manifest: dict[tuple[str, str], str],
    format_strings: dict[str, str],
) -> list[dict]:
    """Test TRY_STRPTIME on each predicted datetime column."""
    results = []
    conn = duckdb.connect()

    for pred in predictions:
        dataset = pred["dataset"]
        column_name = pred["column_name"]
        predicted_type = pred["predicted_type"]
        confidence = float(pred["confidence"])

        # Only test types that have a format_string
        fmt = format_strings.get(predicted_type)
        if not fmt:
            continue

        # Get the file path from the manifest
        file_path = manifest.get((dataset, column_name))
        if not file_path or not Path(file_path).exists():
            continue

        # Test TRY_STRPTIME on the actual data
        try:
            # Escape single quotes in format string
            fmt_escaped = fmt.replace("'", "''")
            col_escaped = column_name.replace('"', '""')

            result = conn.execute(f"""
                SELECT
                    count(*) AS total_non_null,
                    count(TRY_STRPTIME("{col_escaped}", '{fmt_escaped}')) AS parse_success,
                    count(*) - count(TRY_STRPTIME("{col_escaped}", '{fmt_escaped}')) AS parse_fail
                FROM read_csv('{file_path}', auto_detect=true, all_varchar=true)
                WHERE "{col_escaped}" IS NOT NULL AND TRIM("{col_escaped}") != ''
            """).fetchone()

            total, success, fail = result
            if total > 0:
                results.append({
                    "dataset": dataset,
                    "column_name": column_name,
                    "predicted_type": predicted_type,
                    "format_string": fmt,
                    "confidence": confidence,
                    "total_values": total,
                    "parse_success": success,
                    "parse_fail": fail,
                    "success_rate": round(success / total * 100, 1),
                })
        except Exception as e:
            results.append({
                "dataset": dataset,
                "column_name": column_name,
                "predicted_type": predicted_type,
                "format_string": fmt,
                "confidence": confidence,
                "total_values": 0,
                "parse_success": 0,
                "parse_fail": 0,
                "success_rate": 0.0,
                "error": str(e),
            })

    conn.close()
    return results


def print_report(results: list[dict]) -> None:
    """Print actionability report to stdout."""
    print()
    print("═" * 70)
    print("          ACTIONABILITY EVALUATION (NNFT-147)")
    print("═" * 70)
    print()
    print("Can analysts safely TRY_CAST using FineType's format_string?")
    print("Target: >95% success rate for datetime types")
    print()

    if not results:
        print("No datetime predictions with format_strings found in profile results.")
        return

    # Per-column results
    print("─" * 70)
    print(f"{'Dataset':<20} {'Column':<20} {'Type':<35} {'Success':>8}")
    print("─" * 70)
    for r in sorted(results, key=lambda x: x["success_rate"]):
        status = "🟢" if r["success_rate"] >= 95 else ("🟡" if r["success_rate"] >= 80 else "🔴")
        short_type = r["predicted_type"].split(".")[-1]
        print(
            f"{r['dataset']:<20} {r['column_name']:<20} "
            f"{short_type:<35} {r['success_rate']:>6.1f}% {status}"
        )

    # Summary by type
    print()
    print("─" * 70)
    print("Summary by predicted type:")
    print("─" * 70)
    type_stats: dict[str, dict] = {}
    for r in results:
        t = r["predicted_type"]
        if t not in type_stats:
            type_stats[t] = {"total": 0, "success": 0, "columns": 0}
        type_stats[t]["total"] += r["total_values"]
        type_stats[t]["success"] += r["parse_success"]
        type_stats[t]["columns"] += 1

    print(f"{'Predicted Type':<45} {'Cols':>5} {'Values':>8} {'Success':>8}")
    print("─" * 70)
    for t in sorted(type_stats, key=lambda x: type_stats[x]["columns"], reverse=True):
        s = type_stats[t]
        rate = round(s["success"] / s["total"] * 100, 1) if s["total"] > 0 else 0
        status = "🟢" if rate >= 95 else ("🟡" if rate >= 80 else "🔴")
        print(f"{t:<45} {s['columns']:>5} {s['total']:>8} {rate:>6.1f}% {status}")

    # Overall
    total_values = sum(r["total_values"] for r in results)
    total_success = sum(r["parse_success"] for r in results)
    overall_rate = round(total_success / total_values * 100, 1) if total_values > 0 else 0
    overall_status = "🟢" if overall_rate >= 95 else ("🟡" if overall_rate >= 80 else "🔴")

    print()
    print(f"Overall: {total_success}/{total_values} values parsed successfully "
          f"({overall_rate}%) {overall_status}")
    print(f"Columns tested: {len(results)}")
    print(f"Types tested: {len(type_stats)}")


def write_csv(results: list[dict], output_path: Path) -> None:
    """Write results to CSV for downstream analysis."""
    if not results:
        return
    fieldnames = [
        "dataset", "column_name", "predicted_type", "format_string",
        "confidence", "total_values", "parse_success", "parse_fail", "success_rate",
    ]
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(results)
    print(f"\nResults written to: {output_path}")


def main():
    parser = argparse.ArgumentParser(description="FineType Actionability Evaluation")
    parser.add_argument(
        "--manifest",
        default="eval/datasets/manifest.csv",
        help="Path to profile eval manifest CSV",
    )
    parser.add_argument(
        "--predictions",
        default="eval/eval_output/profile_results.csv",
        help="Path to profile results CSV",
    )
    parser.add_argument(
        "--labels-dir",
        default="labels",
        help="Path to taxonomy YAML directory",
    )
    parser.add_argument(
        "--output",
        default="eval/eval_output/actionability_results.csv",
        help="Path for output CSV",
    )
    args = parser.parse_args()

    labels_dir = Path(args.labels_dir)
    predictions_path = Path(args.predictions)
    manifest_path = Path(args.manifest)
    output_path = Path(args.output)

    # Load data
    format_strings = load_format_strings(labels_dir)
    print(f"Loaded {len(format_strings)} types with format_strings")

    predictions = load_predictions(predictions_path)
    print(f"Loaded {len(predictions)} profile predictions")

    manifest = load_manifest(manifest_path)
    print(f"Loaded {len(manifest)} manifest entries")

    # Count how many predictions have testable format_strings
    testable = [p for p in predictions if p["predicted_type"] in format_strings]
    print(f"Testable predictions (have format_string): {len(testable)}")

    # Run actionability tests
    results = test_actionability(predictions, manifest, format_strings)

    # Report
    print_report(results)
    write_csv(results, output_path)


if __name__ == "__main__":
    main()
