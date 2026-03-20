#!/usr/bin/env python3
"""Score a FineType model against the Tier 2 benchmark.

Runs finetype infer --mode column --batch on each benchmark column,
compares predicted vs expected labels, and produces an accuracy report.

Usage:
    python3 scripts/score_tier2.py [OPTIONS]

Options:
    --benchmark PATH    Benchmark CSV path (default: eval/tier2_benchmark.csv)
    --finetype PATH     Path to finetype binary (default: finetype)
    --output PATH       Output report path (default: stdout)
    --format FMT        Output format: text, csv, json (default: text)
    -h, --help          Show this help message

Output report includes:
    - Overall accuracy
    - Per-domain accuracy
    - Per-type accuracy (sorted worst-first)
    - Distilled vs synthetic accuracy split
    - Agreement vs disagreement accuracy split
"""

import csv
import json
import subprocess
import sys
from collections import defaultdict


def load_benchmark(path):
    """Load benchmark CSV into list of dicts."""
    rows = []
    with open(path, newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
    return rows


def score_batch(rows, finetype_bin="finetype"):
    """Run finetype infer --mode column --batch on all benchmark rows.

    Returns list of predicted labels (same order as input rows).
    """
    # Build JSONL input
    jsonl_lines = []
    for row in rows:
        values = json.loads(row["values"])
        header = row.get("header", "")
        jsonl_lines.append(json.dumps({"header": header, "values": values}))

    input_data = "\n".join(jsonl_lines) + "\n"

    # Run finetype
    result = subprocess.run(
        [finetype_bin, "infer", "--mode", "column", "--batch"],
        input=input_data,
        capture_output=True,
        text=True,
    )

    if result.returncode != 0:
        print(f"finetype infer failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    # Parse results
    predictions = []
    for line in result.stdout.strip().split("\n"):
        if not line.strip():
            continue
        obj = json.loads(line)
        predictions.append(obj.get("label", ""))

    if len(predictions) != len(rows):
        print(f"ERROR: Got {len(predictions)} predictions for {len(rows)} rows",
              file=sys.stderr)
        sys.exit(1)

    return predictions


def compute_report(rows, predictions):
    """Compute accuracy metrics from predictions vs expected labels."""
    # Overall
    correct = 0
    total = len(rows)

    # Per-type
    type_correct = defaultdict(int)
    type_total = defaultdict(int)

    # Per-domain
    domain_correct = defaultdict(int)
    domain_total = defaultdict(int)

    # By source
    source_correct = defaultdict(int)
    source_total = defaultdict(int)

    # By agreement
    agreement_correct = defaultdict(int)
    agreement_total = defaultdict(int)

    # Misclassifications for reporting
    misclassifications = []

    for row, pred in zip(rows, predictions):
        expected = row["expected_label"]
        source = row["source"]
        agreement = row["source_agreement"]
        domain = expected.split(".")[0]

        is_correct = pred == expected
        if is_correct:
            correct += 1

        type_correct[expected] += int(is_correct)
        type_total[expected] += 1

        domain_correct[domain] += int(is_correct)
        domain_total[domain] += 1

        source_correct[source] += int(is_correct)
        source_total[source] += 1

        agreement_correct[agreement] += int(is_correct)
        agreement_total[agreement] += 1

        if not is_correct:
            misclassifications.append({
                "expected": expected,
                "predicted": pred,
                "source": source,
                "agreement": agreement,
            })

    return {
        "overall": {"correct": correct, "total": total},
        "by_type": {t: {"correct": type_correct[t], "total": type_total[t]}
                    for t in type_total},
        "by_domain": {d: {"correct": domain_correct[d], "total": domain_total[d]}
                      for d in sorted(domain_total)},
        "by_source": {s: {"correct": source_correct[s], "total": source_total[s]}
                      for s in sorted(source_total)},
        "by_agreement": {a: {"correct": agreement_correct[a], "total": agreement_total[a]}
                         for a in sorted(agreement_total)},
        "misclassifications": misclassifications,
    }


def format_text(report):
    """Format report as human-readable text."""
    lines = []
    o = report["overall"]
    acc = 100 * o["correct"] / o["total"] if o["total"] else 0
    lines.append(f"Tier 2 Benchmark Results")
    lines.append(f"{'=' * 70}")
    lines.append(f"Overall: {o['correct']}/{o['total']} ({acc:.1f}%)")
    lines.append("")

    # By source
    lines.append("By Source:")
    for s, v in report["by_source"].items():
        a = 100 * v["correct"] / v["total"] if v["total"] else 0
        lines.append(f"  {s:<12} {v['correct']:>5}/{v['total']:<5} ({a:.1f}%)")
    lines.append("")

    # By agreement
    lines.append("By Agreement Status:")
    for s, v in report["by_agreement"].items():
        a = 100 * v["correct"] / v["total"] if v["total"] else 0
        lines.append(f"  {s:<12} {v['correct']:>5}/{v['total']:<5} ({a:.1f}%)")
    lines.append("")

    # By domain
    lines.append("By Domain:")
    for d, v in report["by_domain"].items():
        a = 100 * v["correct"] / v["total"] if v["total"] else 0
        lines.append(f"  {d:<20} {v['correct']:>5}/{v['total']:<5} ({a:.1f}%)")
    lines.append("")

    # Per-type accuracy (sorted worst-first)
    lines.append("Per-Type Accuracy (worst first):")
    type_accs = []
    for t, v in report["by_type"].items():
        a = v["correct"] / v["total"] if v["total"] else 0
        type_accs.append((a, t, v))
    type_accs.sort()

    for a, t, v in type_accs[:50]:  # Show worst 50
        pct = 100 * a
        lines.append(f"  {pct:>5.1f}%  {v['correct']:>2}/{v['total']:<2}  {t}")

    if len(type_accs) > 50:
        remaining = len(type_accs) - 50
        lines.append(f"  ... ({remaining} more types with higher accuracy)")

    return "\n".join(lines)


def format_json(report):
    """Format report as JSON."""
    # Add computed accuracy percentages
    output = {
        "overall_accuracy": report["overall"]["correct"] / report["overall"]["total"],
        "overall_correct": report["overall"]["correct"],
        "overall_total": report["overall"]["total"],
        "by_domain": {
            d: {"accuracy": v["correct"] / v["total"] if v["total"] else 0, **v}
            for d, v in report["by_domain"].items()
        },
        "by_source": {
            s: {"accuracy": v["correct"] / v["total"] if v["total"] else 0, **v}
            for s, v in report["by_source"].items()
        },
        "by_agreement": {
            a: {"accuracy": v["correct"] / v["total"] if v["total"] else 0, **v}
            for a, v in report["by_agreement"].items()
        },
        "by_type": {
            t: {"accuracy": v["correct"] / v["total"] if v["total"] else 0, **v}
            for t, v in sorted(report["by_type"].items(),
                               key=lambda x: x[1]["correct"] / x[1]["total"] if x[1]["total"] else 0)
        },
    }
    return json.dumps(output, indent=2)


def main():
    args = sys.argv[1:]
    benchmark_path = "eval/tier2_benchmark.csv"
    finetype_bin = "finetype"
    output_path = None
    fmt = "text"

    i = 0
    while i < len(args):
        if args[i] in ("-h", "--help", "help"):
            print(__doc__)
            return
        elif args[i] == "--benchmark":
            benchmark_path = args[i + 1]; i += 2
        elif args[i] == "--finetype":
            finetype_bin = args[i + 1]; i += 2
        elif args[i] == "--output":
            output_path = args[i + 1]; i += 2
        elif args[i] == "--format":
            fmt = args[i + 1]; i += 2
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    # Load benchmark
    print(f"Loading benchmark: {benchmark_path}", file=sys.stderr)
    rows = load_benchmark(benchmark_path)
    print(f"  {len(rows)} columns to score", file=sys.stderr)

    # Score
    print("Running finetype infer --mode column --batch...", file=sys.stderr)
    predictions = score_batch(rows, finetype_bin)
    print("  Done", file=sys.stderr)

    # Report
    report = compute_report(rows, predictions)

    if fmt == "text":
        output = format_text(report)
    elif fmt == "json":
        output = format_json(report)
    else:
        print(f"Unknown format: {fmt}", file=sys.stderr)
        sys.exit(1)

    if output_path:
        with open(output_path, "w") as f:
            f.write(output + "\n")
        print(f"Report written to {output_path}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
