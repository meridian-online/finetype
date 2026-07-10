#!/usr/bin/env python3
"""
Training data audit — model-as-critic.

Uses the current v14 model to classify training examples and flags where
the model's prediction disagrees with the training label. Catches mislabeled
training data that degrades model quality.

Usage:
    python3 scripts/training_data_audit.py [--samples N] [--output PATH]

Requires:
    - output/distillation-v3/sherlock_distilled.csv.gz (training data)
    - data/label_remap.json (label mappings)
    - models/sherlock-v14/label_map.json (taxonomy labels)
    - target/release/finetype (built binary)
"""

import argparse
import csv
import gzip
import json
import os
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DISTILLED = ROOT / "output" / "distillation-v3" / "sherlock_distilled.csv.gz"
LABEL_REMAP = ROOT / "data" / "label_remap.json"
LABEL_MAP = ROOT / "models" / "sherlock-v14" / "label_map.json"
FINETYPE = ROOT / "target" / "release" / "finetype"


def load_remap():
    """Load label_remap.json, removing the _comment key."""
    with open(LABEL_REMAP) as f:
        remap = json.load(f)
    remap.pop("_comment", None)
    return remap


def load_taxonomy():
    """Load taxonomy labels from the model's label_map.json."""
    with open(LABEL_MAP) as f:
        return set(json.load(f))


def load_training_data():
    """Load all training data rows from the distilled CSV."""
    rows = []
    with gzip.open(DISTILLED, "rt") as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)
    return rows


def apply_remap(label, remap):
    """Apply label remapping."""
    return remap.get(label, label)


def sample_rows_by_label(rows, remap, max_samples=20):
    """
    Group rows by remapped label, sample up to max_samples per label.
    Returns dict: remapped_label -> list of (original_label, sample_values_json)
    """
    by_label = defaultdict(list)
    for row in rows:
        orig_label = row["final_label"]
        mapped = apply_remap(orig_label, remap)
        by_label[mapped].append((orig_label, row["sample_values"]))

    sampled = {}
    for label, entries in by_label.items():
        # Take evenly spaced samples if more than max_samples
        if len(entries) <= max_samples:
            sampled[label] = entries
        else:
            step = len(entries) / max_samples
            sampled[label] = [entries[int(i * step)] for i in range(max_samples)]

    return sampled, {k: len(v) for k, v in by_label.items()}


def run_batch_inference(sampled, taxonomy):
    """
    Run batch inference using finetype infer --mode column --batch.

    For each label, we combine values from all sampled training examples
    into one column and classify it. This gives us one prediction per label.

    Returns dict: remapped_label -> model_prediction
    """
    # Build JSONL input: one line per label
    jsonl_lines = []
    label_order = []

    for label in sorted(sampled.keys()):
        # Skip empty labels — they can't be meaningfully classified
        if not label:
            continue

        # Combine values from all sampled examples into one list
        all_values = []
        for _orig, sample_json in sampled[label]:
            try:
                values = json.loads(sample_json)
                if isinstance(values, list):
                    all_values.extend([str(v) for v in values if v])
            except (json.JSONDecodeError, TypeError):
                continue

        if not all_values:
            continue

        # Cap at 200 values for performance
        all_values = all_values[:200]

        entry = {"header": "", "values": all_values}
        jsonl_lines.append(json.dumps(entry))
        label_order.append(label)

    print(f"Running batch inference on {len(jsonl_lines)} labels...", file=sys.stderr)

    # Run finetype in batch mode (single model load)
    input_text = "\n".join(jsonl_lines)
    result = subprocess.run(
        [str(FINETYPE), "infer", "--mode", "column", "--batch"],
        input=input_text,
        capture_output=True,
        text=True,
        timeout=600,
    )

    if result.returncode != 0:
        print(f"ERROR: finetype batch failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    # Parse output — one JSON line per input
    predictions = {}
    output_lines = [l for l in result.stdout.strip().split("\n") if l.startswith("{")]

    if len(output_lines) != len(label_order):
        print(
            f"WARNING: Expected {len(label_order)} predictions, got {len(output_lines)}",
            file=sys.stderr,
        )

    for i, line in enumerate(output_lines):
        if i >= len(label_order):
            break
        try:
            pred = json.loads(line)
            predictions[label_order[i]] = pred
        except json.JSONDecodeError:
            print(f"WARNING: Failed to parse prediction line: {line}", file=sys.stderr)

    return predictions


def run_per_example_inference(sampled, taxonomy):
    """
    Run per-example inference for labels that disagreed in the combined test.
    This gives us a per-example disagreement rate.

    For each disagreeing label, run each of the 20 examples individually.
    Returns dict: label -> list of (model_prediction, confidence, values_sample)
    """
    # Build JSONL for all examples from all labels at once
    jsonl_lines = []
    entry_index = []  # (label, example_idx, orig_label)

    for label in sorted(sampled.keys()):
        if not label:
            continue
        for ex_idx, (orig_label, sample_json) in enumerate(sampled[label]):
            try:
                values = json.loads(sample_json)
                if not isinstance(values, list) or not values:
                    continue
                clean_values = [str(v) for v in values if v]
                if not clean_values:
                    continue
            except (json.JSONDecodeError, TypeError):
                continue

            entry = {"header": "", "values": clean_values}
            jsonl_lines.append(json.dumps(entry))
            entry_index.append((label, ex_idx, orig_label))

    print(
        f"Running per-example inference on {len(jsonl_lines)} examples...",
        file=sys.stderr,
    )

    input_text = "\n".join(jsonl_lines)
    result = subprocess.run(
        [str(FINETYPE), "infer", "--mode", "column", "--batch"],
        input=input_text,
        capture_output=True,
        text=True,
        timeout=600,
    )

    if result.returncode != 0:
        print(f"ERROR: finetype batch failed: {result.stderr}", file=sys.stderr)
        sys.exit(1)

    output_lines = [l for l in result.stdout.strip().split("\n") if l.startswith("{")]

    results = defaultdict(list)
    for i, line in enumerate(output_lines):
        if i >= len(entry_index):
            break
        label, ex_idx, orig_label = entry_index[i]
        try:
            pred = json.loads(line)
            results[label].append(
                {
                    "predicted": pred.get("label", ""),
                    "confidence": pred.get("confidence", 0),
                    "rule": pred.get("disambiguation_rule", ""),
                    "orig_label": orig_label,
                    "example_idx": ex_idx,
                }
            )
        except json.JSONDecodeError:
            pass

    return results


def analyze_unmapped(rows, remap, taxonomy):
    """Analyze the 6 unmapped labels."""
    unmapped = defaultdict(list)
    for row in rows:
        orig = row["final_label"]
        mapped = apply_remap(orig, remap)
        if mapped not in taxonomy and mapped:
            unmapped[mapped].append(row)

    results = {}
    for label, label_rows in unmapped.items():
        # Sample some values
        sample_values = []
        for r in label_rows[:3]:
            try:
                vals = json.loads(r["sample_values"])
                sample_values.append(vals[:5] if isinstance(vals, list) else vals)
            except (json.JSONDecodeError, TypeError):
                pass

        # Check what original labels map here
        orig_labels = Counter(r["final_label"] for r in label_rows)

        results[label] = {
            "count": len(label_rows),
            "original_labels": dict(orig_labels),
            "sample_values": sample_values,
        }

    return results


def domain_match(a, b):
    """Check if two labels share the same domain (first segment)."""
    if not a or not b:
        return False
    return a.split(".")[0] == b.split(".")[0]


def main():
    parser = argparse.ArgumentParser(description="Training data model-as-critic audit")
    parser.add_argument(
        "--samples", type=int, default=20, help="Max samples per label (default: 20)"
    )
    parser.add_argument(
        "--output",
        type=str,
        default=str(
            ROOT / "orbit" / "specs" / "2026-04-18-v16-data-audit-retrain" / "training-audit.md"
        ),
        help="Output report path",
    )
    args = parser.parse_args()

    # Validate prerequisites
    if not FINETYPE.exists():
        print("ERROR: Build finetype first: cargo build --release --bin finetype")
        sys.exit(1)
    if not DISTILLED.exists():
        print(f"ERROR: Training data not found: {DISTILLED}")
        sys.exit(1)

    print("Loading training data...", file=sys.stderr)
    rows = load_training_data()
    remap = load_remap()
    taxonomy = load_taxonomy()

    print(f"Loaded {len(rows)} training rows", file=sys.stderr)
    print(f"Label remap: {len(remap)} mappings", file=sys.stderr)
    print(f"Taxonomy: {len(taxonomy)} types", file=sys.stderr)

    # Sample rows per remapped label
    sampled, label_counts = sample_rows_by_label(rows, remap, max_samples=args.samples)
    total_labels = len([l for l in sampled if l])  # exclude empty
    print(f"Sampled {total_labels} labels (up to {args.samples} examples each)", file=sys.stderr)

    # Phase 1: Combined inference (one prediction per label)
    predictions = run_batch_inference(sampled, taxonomy)

    # Identify disagreements
    agreements = []
    disagreements = []
    not_in_taxonomy = []

    for label in sorted(sampled.keys()):
        if not label:
            continue
        if label not in predictions:
            continue

        pred = predictions[label]
        pred_label = pred.get("label", "")
        pred_conf = pred.get("confidence", 0)
        pred_rule = pred.get("disambiguation_rule", "")

        if label not in taxonomy:
            not_in_taxonomy.append(
                {
                    "label": label,
                    "predicted": pred_label,
                    "confidence": pred_conf,
                    "count": label_counts.get(label, 0),
                }
            )
        elif pred_label == label:
            agreements.append(label)
        else:
            disagreements.append(
                {
                    "label": label,
                    "predicted": pred_label,
                    "confidence": pred_conf,
                    "rule": pred_rule,
                    "count": label_counts.get(label, 0),
                    "same_domain": domain_match(label, pred_label),
                }
            )

    # Phase 2: Per-example inference for ALL labels
    print("\nPhase 2: Per-example inference for all labels...", file=sys.stderr)
    per_example = run_per_example_inference(sampled, taxonomy)

    # Compute per-example disagreement rates
    per_example_stats = {}
    for label, examples in per_example.items():
        total = len(examples)
        disagree_count = sum(1 for e in examples if e["predicted"] != label)
        rate = disagree_count / total if total > 0 else 0
        per_example_stats[label] = {
            "total": total,
            "disagreements": disagree_count,
            "rate": rate,
            "examples": examples,
        }

    # Analyze unmapped labels
    unmapped = analyze_unmapped(rows, remap, taxonomy)

    # Generate report
    print("\nGenerating report...", file=sys.stderr)
    generate_report(
        args.output,
        rows,
        remap,
        taxonomy,
        sampled,
        label_counts,
        predictions,
        agreements,
        disagreements,
        not_in_taxonomy,
        per_example_stats,
        unmapped,
        args.samples,
    )

    print(f"\nReport written to {args.output}", file=sys.stderr)

    # Print summary
    total = len(agreements) + len(disagreements)
    print(f"\n=== AUDIT SUMMARY ===", file=sys.stderr)
    print(f"Labels tested: {total}", file=sys.stderr)
    print(f"Agreements: {len(agreements)} ({100*len(agreements)/total:.1f}%)", file=sys.stderr)
    print(f"Disagreements: {len(disagreements)} ({100*len(disagreements)/total:.1f}%)", file=sys.stderr)
    print(f"Not in taxonomy: {len(not_in_taxonomy)}", file=sys.stderr)

    # High per-example disagreement
    high_disagree = [
        (label, s)
        for label, s in per_example_stats.items()
        if s["rate"] > 0.2 and label in taxonomy and s["total"] >= 5
    ]
    high_disagree.sort(key=lambda x: x[1]["rate"], reverse=True)
    if high_disagree:
        print(f"\nLabels with >20% per-example disagreement: {len(high_disagree)}", file=sys.stderr)
        for label, s in high_disagree[:10]:
            print(
                f"  {label}: {s['disagreements']}/{s['total']} ({100*s['rate']:.0f}%) -> commonly predicted as {Counter(e['predicted'] for e in s['examples'] if e['predicted'] != label).most_common(1)}",
                file=sys.stderr,
            )


def generate_report(
    output_path,
    rows,
    remap,
    taxonomy,
    sampled,
    label_counts,
    predictions,
    agreements,
    disagreements,
    not_in_taxonomy,
    per_example_stats,
    unmapped,
    max_samples,
):
    """Generate the markdown audit report."""
    total_in_taxonomy = len(agreements) + len(disagreements)

    # Compute overall per-example stats
    total_examples = sum(s["total"] for s in per_example_stats.values())
    total_example_disagree = sum(s["disagreements"] for s in per_example_stats.values())

    lines = []
    lines.append("# Training Data Audit — Model-as-Critic (ac-02)")
    lines.append("")
    lines.append("## Methodology")
    lines.append("")
    lines.append(f"- **Training data:** `output/distillation-v3/sherlock_distilled.csv.gz` ({len(rows):,} rows, 176 unique labels)")
    lines.append(f"- **Label remapping:** Applied `data/label_remap.json` ({len(remap)} mappings) before comparison")
    lines.append(f"- **After remap:** {len(set(label_counts.keys()))} unique labels ({total_in_taxonomy} in taxonomy, {len(not_in_taxonomy)} not in taxonomy)")
    lines.append(f"- **Model:** sherlock-v14 (240 taxonomy types)")
    lines.append(f"- **Sampling:** Up to {max_samples} training examples per label")
    lines.append(f"- **Phase 1:** Combined values from all examples per label into one column, classified once per label (tests aggregate signal)")
    lines.append(f"- **Phase 2:** Classified each training example individually (tests per-example agreement)")
    lines.append(f"- **Header hints disabled:** Used empty header to avoid header bias (tests pure value classification)")
    lines.append("")

    lines.append("## Summary Statistics")
    lines.append("")
    lines.append("### Phase 1: Combined column classification (1 prediction per label)")
    lines.append("")
    lines.append(f"```")
    lines.append(f"Labels in taxonomy:       {total_in_taxonomy}")
    lines.append(f"  Agreements:             {len(agreements)} ({100*len(agreements)/total_in_taxonomy:.1f}%)")
    lines.append(f"  Disagreements:          {len(disagreements)} ({100*len(disagreements)/total_in_taxonomy:.1f}%)")
    lines.append(f"Labels not in taxonomy:   {len(not_in_taxonomy)}")
    lines.append(f"```")
    lines.append("")
    lines.append("### Phase 2: Per-example classification")
    lines.append("")
    lines.append(f"```")
    lines.append(f"Total examples tested:    {total_examples}")
    lines.append(f"  Agreements:             {total_examples - total_example_disagree} ({100*(total_examples - total_example_disagree)/total_examples:.1f}%)")
    lines.append(f"  Disagreements:          {total_example_disagree} ({100*total_example_disagree/total_examples:.1f}%)")
    lines.append(f"```")
    lines.append("")

    # Phase 1 disagreements table
    lines.append("## Phase 1: Combined Column Disagreements")
    lines.append("")
    lines.append("Labels where the model predicts a different type when all training examples for that label are combined into one column.")
    lines.append("")

    if disagreements:
        lines.append("```")
        lines.append(f"{'Training Label':<50} {'Model Prediction':<50} {'Conf':>5} {'Rows':>6} {'Domain?':>7}")
        lines.append(f"{'-'*50} {'-'*50} {'-'*5} {'-'*6} {'-'*7}")
        for d in sorted(disagreements, key=lambda x: x["count"], reverse=True):
            domain_flag = "yes" if d["same_domain"] else "NO"
            lines.append(
                f"{d['label']:<50} {d['predicted']:<50} {d['confidence']:>5.2f} {d['count']:>6} {domain_flag:>7}"
            )
        lines.append("```")
    else:
        lines.append("No disagreements found.")
    lines.append("")

    # Phase 2: Per-example high disagreement table
    high_disagree = [
        (label, s)
        for label, s in per_example_stats.items()
        if s["rate"] > 0.2 and label in taxonomy and s["total"] >= 5
    ]
    high_disagree.sort(key=lambda x: x[1]["rate"], reverse=True)

    lines.append("## Phase 2: Labels with >20% Per-Example Disagreement")
    lines.append("")
    lines.append("These labels have significant per-example disagreement, suggesting potential mislabeling or ambiguous training data.")
    lines.append("")

    if high_disagree:
        lines.append("```")
        lines.append(f"{'Training Label':<50} {'Disagree':>8} {'Total':>6} {'Rate':>6} {'Most Common Alternative':<40}")
        lines.append(f"{'-'*50} {'-'*8} {'-'*6} {'-'*6} {'-'*40}")
        for label, s in high_disagree:
            alt_preds = Counter(
                e["predicted"] for e in s["examples"] if e["predicted"] != label
            )
            most_common = alt_preds.most_common(1)
            alt_str = f"{most_common[0][0]} ({most_common[0][1]}x)" if most_common else "-"
            lines.append(
                f"{label:<50} {s['disagreements']:>8} {s['total']:>6} {100*s['rate']:>5.0f}% {alt_str:<40}"
            )
        lines.append("```")
    else:
        lines.append("No labels with >20% per-example disagreement found.")
    lines.append("")

    # Moderate disagreement (10-20%)
    mod_disagree = [
        (label, s)
        for label, s in per_example_stats.items()
        if 0.1 <= s["rate"] <= 0.2 and label in taxonomy and s["total"] >= 5
    ]
    mod_disagree.sort(key=lambda x: x[1]["rate"], reverse=True)

    if mod_disagree:
        lines.append("### Labels with 10-20% Per-Example Disagreement")
        lines.append("")
        lines.append("```")
        lines.append(f"{'Training Label':<50} {'Disagree':>8} {'Total':>6} {'Rate':>6} {'Most Common Alternative':<40}")
        lines.append(f"{'-'*50} {'-'*8} {'-'*6} {'-'*6} {'-'*40}")
        for label, s in mod_disagree:
            alt_preds = Counter(
                e["predicted"] for e in s["examples"] if e["predicted"] != label
            )
            most_common = alt_preds.most_common(1)
            alt_str = f"{most_common[0][0]} ({most_common[0][1]}x)" if most_common else "-"
            lines.append(
                f"{label:<50} {s['disagreements']:>8} {s['total']:>6} {100*s['rate']:>5.0f}% {alt_str:<40}"
            )
        lines.append("```")
        lines.append("")

    # Unmapped labels analysis
    lines.append("## Unmapped Labels Analysis")
    lines.append("")
    lines.append("Labels that exist in training data but have no corresponding taxonomy type (even after remap).")
    lines.append("")

    # Also include empty and 'yes' labels
    empty_count = sum(1 for row in rows if row["final_label"] == "")
    yes_count = sum(1 for row in rows if row["final_label"] == "yes")

    lines.append(f"### Empty label (`''`)")
    lines.append(f"- **Count:** {empty_count}")
    lines.append(f"- **Resolution:** Drop from training data (unlabeled rows)")
    lines.append("")

    lines.append(f"### `yes`")
    lines.append(f"- **Count:** {yes_count}")
    lines.append(f"- **Resolution:** Drop from training data (not a type label)")
    lines.append("")

    for label, info in sorted(unmapped.items()):
        if label in ("", "yes"):
            continue
        lines.append(f"### `{label}`")
        lines.append(f"- **Count:** {info['count']}")
        lines.append(f"- **Original labels:** {info['original_labels']}")
        if info["sample_values"]:
            lines.append(f"- **Sample values:** `{json.dumps(info['sample_values'][0][:5])}`")

        # Recommend resolution
        if label == "representation.text.sentence":
            lines.append(f"- **Resolution:** Remap to `representation.text.plain_text` (sentence is not in taxonomy, plain_text is the closest match)")
        elif label == "representation.text.paragraph":
            lines.append(f"- **Resolution:** Remap to `representation.text.plain_text` (paragraph is not in taxonomy)")
        elif label == "finance.currency.amount_minor_int":
            lines.append(f"- **Resolution:** Drop (only 1 row, type not in taxonomy)")
        elif label == "representation.scientific.metric_prefix":
            lines.append(f"- **Resolution:** Drop or remap to `representation.discrete.categorical` (8 rows, metric prefixes are categorical values)")
        else:
            lines.append(f"- **Resolution:** Investigate")
        lines.append("")

    # Detailed disagreement patterns
    lines.append("## Mislabeling Patterns")
    lines.append("")

    # Group disagreements by predicted label to find systematic patterns
    pred_to_labels = defaultdict(list)
    for label, s in per_example_stats.items():
        if label not in taxonomy or s["total"] < 5:
            continue
        for ex in s["examples"]:
            if ex["predicted"] != label:
                pred_to_labels[(label, ex["predicted"])].append(ex)

    # Find systematic patterns (>3 examples)
    patterns = [
        (pair, exs) for pair, exs in pred_to_labels.items() if len(exs) >= 3
    ]
    patterns.sort(key=lambda x: len(x[1]), reverse=True)

    if patterns:
        lines.append("Systematic confusion patterns (3+ examples mislabeled the same way):")
        lines.append("")
        for (label, pred), exs in patterns:
            lines.append(f"### {label} -> model predicts {pred} ({len(exs)} examples)")
            lines.append("")
            # Show confidence stats
            confs = [e["confidence"] for e in exs]
            avg_conf = sum(confs) / len(confs) if confs else 0
            lines.append(f"- Model confidence: avg {avg_conf:.2f}, range [{min(confs):.2f}, {max(confs):.2f}]")
            rules_used = Counter(e["rule"] for e in exs if e.get("rule"))
            if rules_used:
                lines.append(f"- Disambiguation rules applied: {dict(rules_used)}")

            # Interpretation
            if label.split(".")[0] == pred.split(".")[0]:
                lines.append(f"- **Same domain confusion** — likely ambiguous training data rather than mislabeling")
            else:
                lines.append(f"- **Cross-domain confusion** — potential mislabeling or very ambiguous values")
            lines.append("")
    else:
        lines.append("No systematic mislabeling patterns found (3+ examples).")
    lines.append("")

    # Recommendations
    lines.append("## Recommendations for v16 Training")
    lines.append("")

    # Count total affected rows
    total_affected = 0
    remap_candidates = []

    for label, s in per_example_stats.items():
        if s["rate"] > 0.5 and label in taxonomy and s["total"] >= 5:
            total_affected += label_counts.get(label, 0)
            remap_candidates.append((label, s))

    lines.append(f"### 1. Label remapping additions")
    lines.append("")
    lines.append("Add these to `data/label_remap.json`:")
    lines.append("")
    lines.append("```json")
    new_remaps = {}
    # sentence/paragraph/title -> plain_text
    if "representation.text.sentence" in unmapped or "representation.text.sentence" not in taxonomy:
        new_remaps["representation.text.sentence"] = "representation.text.plain_text"
    if "representation.text.paragraph" not in taxonomy:
        new_remaps["representation.text.paragraph"] = "representation.text.plain_text"
    # Note: title and description already remap to sentence via label_remap.json,
    # but sentence itself isn't in taxonomy. Need to update the remap chain.
    lines.append(json.dumps(new_remaps, indent=2))
    lines.append("```")
    lines.append("")

    lines.append(f"### 2. Drop unmappable rows")
    lines.append("")
    lines.append(f"Drop rows with these labels (total ~{empty_count + yes_count + 1 + 8} rows):")
    lines.append(f"- `''` (empty): {empty_count} rows")
    lines.append(f"- `yes`: {yes_count} rows")
    lines.append(f"- `finance.currency.amount_minor_int`: 1 row")
    lines.append(f"- `representation.scientific.metric_prefix`: 8 rows")
    lines.append("")

    lines.append(f"### 3. Investigate high-disagreement labels")
    lines.append("")
    lines.append("Labels with >50% per-example disagreement should be manually reviewed for mislabeling:")
    lines.append("")
    for label, s in sorted(remap_candidates, key=lambda x: x[1]["rate"], reverse=True):
        lines.append(f"- `{label}`: {s['disagreements']}/{s['total']} disagree ({100*s['rate']:.0f}%), {label_counts.get(label, 0)} total training rows")
    if not remap_candidates:
        lines.append("- None found.")
    lines.append("")

    # Write report
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w") as f:
        f.write("\n".join(lines))


if __name__ == "__main__":
    main()
