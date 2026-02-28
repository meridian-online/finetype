#!/usr/bin/env python3
"""Compare Sense model predictions vs FineType on identical SOTAB validation columns.

Maps FineType's 163 type predictions to the 6 broad categories used by Sense,
enabling a direct apples-to-apples comparison.

Usage:
    python3 scripts/compare_sense_vs_finetype.py [--sample N]
"""

import argparse
import json
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

# FineType label → broad category mapping
def map_finetype_to_broad(label: str) -> str:
    """Map a FineType label to a broad category."""
    if label.startswith("datetime."):
        return "temporal"
    if label.startswith("geography."):
        return "geographic"
    if label.startswith("representation.numeric."):
        return "numeric"
    if label.startswith("representation.boolean."):
        return "text"
    if label.startswith("representation.text."):
        # entity_name is entity in SOTAB context
        if label == "representation.text.entity_name":
            return "entity"
        return "text"
    if label.startswith("representation.discrete."):
        return "text"
    if label.startswith("representation.alphanumeric."):
        return "format"
    if label.startswith("identity.person."):
        # email → format, full_name/first_name/last_name → entity
        if label == "identity.person.email":
            return "format"
        if label == "identity.person.username":
            return "format"
        return "entity"
    if label.startswith("identity.contact."):
        return "format"
    if label.startswith("identity.financial."):
        return "format"
    if label.startswith("identity.code."):
        return "format"
    if label.startswith("identity.organization."):
        return "entity"
    if label.startswith("identity.medical."):
        return "format"
    if label.startswith("technology."):
        return "format"
    if label.startswith("container."):
        return "format"

    return "text"  # fallback


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--sample", type=int, default=0, help="Sample N columns (0=all)")
    args = parser.parse_args()

    val_path = Path("data/sense_spike/val.jsonl")
    if not val_path.exists():
        print("ERROR: val.jsonl not found", file=sys.stderr)
        sys.exit(1)

    # Load validation data
    print("Loading validation data...")
    val_data = []
    with open(val_path) as f:
        for line in f:
            val_data.append(json.loads(line))
    print(f"  {len(val_data)} columns loaded")

    # Optional: stratified sample
    if args.sample > 0 and args.sample < len(val_data):
        import random
        random.seed(42)
        # Group by broad_category for stratified sampling
        by_cat = {}
        for item in val_data:
            cat = item["broad_category"]
            by_cat.setdefault(cat, []).append(item)

        sampled = []
        per_cat = max(1, args.sample // len(by_cat))
        for cat, items in sorted(by_cat.items()):
            n = min(per_cat, len(items))
            sampled.extend(random.sample(items, n))

        # Fill remainder
        remaining = args.sample - len(sampled)
        if remaining > 0:
            used = set(id(x) for x in sampled)
            pool = [x for x in val_data if id(x) not in used]
            sampled.extend(random.sample(pool, min(remaining, len(pool))))

        val_data = sampled
        print(f"  Sampled to {len(val_data)} columns (stratified)")

    # Show category distribution
    cat_dist = Counter(item["broad_category"] for item in val_data)
    for cat in ["entity", "format", "temporal", "numeric", "geographic", "text"]:
        print(f"    {cat}: {cat_dist.get(cat, 0)}")

    # Prepare batch input for FineType
    print("\nPreparing FineType batch input...")
    batch_lines = []
    for item in val_data:
        batch_entry = {"values": item["values"]}
        # SOTAB doesn't have meaningful headers (numeric col indices)
        # So we run FineType without headers — same as Sense without headers
        batch_lines.append(json.dumps(batch_entry))

    batch_input = "\n".join(batch_lines)

    # Run FineType in batch column mode
    print(f"Running FineType on {len(val_data)} columns (no headers)...")
    t0 = time.time()
    try:
        result = subprocess.run(
            ["cargo", "run", "--release", "--", "infer", "--mode", "column", "--batch"],
            input=batch_input,
            capture_output=True,
            text=True,
            cwd=str(Path.cwd()),
            timeout=1800,  # 30 min
        )
    except subprocess.TimeoutExpired:
        print("ERROR: FineType timed out after 30 minutes", file=sys.stderr)
        sys.exit(1)
    elapsed = time.time() - t0

    if result.returncode != 0:
        print(f"ERROR: FineType failed:\n{result.stderr}", file=sys.stderr)
        sys.exit(1)

    # Parse FineType predictions
    ft_predictions = []
    for line in result.stdout.strip().split("\n"):
        if line.strip():
            ft_predictions.append(json.loads(line))

    print(f"  {len(ft_predictions)} predictions in {elapsed:.1f}s ({elapsed/len(ft_predictions)*1000:.1f}ms/col)")

    if len(ft_predictions) != len(val_data):
        print(f"WARNING: prediction count ({len(ft_predictions)}) != val count ({len(val_data)})")
        n = min(len(ft_predictions), len(val_data))
        val_data = val_data[:n]
        ft_predictions = ft_predictions[:n]

    # ===== BROAD CATEGORY COMPARISON =====
    print(f"\n{'=' * 60}")
    print(f"  FineType → Broad Category Comparison")
    print(f"{'=' * 60}")

    broad_categories = ["entity", "format", "temporal", "numeric", "geographic", "text"]
    tp = Counter()
    fp = Counter()
    fn = Counter()
    confusion = {gt: Counter() for gt in broad_categories}
    ft_broad_correct = 0

    for item, pred in zip(val_data, ft_predictions):
        gt_broad = item["broad_category"]
        ft_label = pred.get("label", "unknown")
        ft_broad = map_finetype_to_broad(ft_label)

        if ft_broad == gt_broad:
            ft_broad_correct += 1
            tp[gt_broad] += 1
        else:
            fp[ft_broad] += 1
            fn[gt_broad] += 1
            confusion[gt_broad][ft_broad] += 1

    ft_accuracy = ft_broad_correct / len(val_data)

    print(f"\nFineType broad category accuracy: {ft_accuracy:.4f} ({ft_accuracy*100:.1f}%)")
    print(f"Sense A broad category accuracy:  0.8849 (88.5%)")
    print(f"Delta: {(ft_accuracy - 0.8849)*100:+.1f}pp")

    # Per-category breakdown
    sense_f1 = {
        "entity": 0.845, "format": 0.826, "temporal": 0.972,
        "numeric": 0.954, "geographic": 0.823, "text": 0.874,
    }

    print(f"\n{'Category':<12} {'Support':>8} {'FT correct':>11} {'FT prec':>8} {'FT recall':>10} {'FT F1':>7} {'Sense F1':>9}")
    print("-" * 68)

    for cat in broad_categories:
        support = tp[cat] + fn[cat]
        correct = tp[cat]
        recall = correct / support if support > 0 else 0
        precision = tp[cat] / (tp[cat] + fp[cat]) if (tp[cat] + fp[cat]) > 0 else 0
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
        s_f1 = sense_f1.get(cat, 0)
        winner = "←" if f1 > s_f1 else ("→" if s_f1 > f1 else "=")
        print(f"{cat:<12} {support:>8} {correct:>11} {precision:>7.1%} {recall:>9.1%} {f1:>7.3f} {s_f1:>8.3f} {winner}")

    # Confusion matrix for FineType
    print(f"\nFineType broad category confusion matrix:")
    print(f"{'GT \\ Pred':<12}", end="")
    for cat in broad_categories:
        print(f" {cat[:8]:>8}", end="")
    print()

    for gt in broad_categories:
        print(f"{gt:<12}", end="")
        for pred_cat in broad_categories:
            if gt == pred_cat:
                count = tp[gt]
            else:
                count = confusion[gt].get(pred_cat, 0)
            print(f" {count:>8}", end="")
        print()

    # ===== ENTITY SUBTYPE COMPARISON =====
    print(f"\n{'=' * 60}")
    print(f"  Entity Subtype Analysis")
    print(f"{'=' * 60}")

    FINETYPE_TO_ENTITY = {
        "identity.person.full_name": "person",
        "identity.person.first_name": "person",
        "identity.person.last_name": "person",
        "representation.text.entity_name": None,  # ambiguous
        "geography.location.city": "place",
        "geography.location.country": "place",
        "geography.location.state": "place",
        "geography.location.region": "place",
        "geography.address.city": "place",
        "geography.address.state": "place",
        "geography.address.country": "place",
        "geography.address.full_address": "place",
        "identity.organization.company_name": "organization",
    }

    entity_items = [(item, pred) for item, pred in zip(val_data, ft_predictions)
                    if item.get("entity_subtype") is not None]

    entity_total = len(entity_items)
    entity_mapped = 0
    entity_correct = 0
    subtype_confusion = Counter()

    for item, pred in entity_items:
        gt_subtype = item["entity_subtype"]
        ft_label = pred.get("label", "unknown")
        ft_subtype = FINETYPE_TO_ENTITY.get(ft_label)

        if ft_subtype is not None:
            entity_mapped += 1
            if ft_subtype == gt_subtype:
                entity_correct += 1
            else:
                subtype_confusion[(gt_subtype, ft_subtype)] += 1

    print(f"\nEntity columns: {entity_total}")
    print(f"FineType mapped to subtype: {entity_mapped} ({entity_mapped/entity_total*100:.1f}%)")
    if entity_mapped > 0:
        print(f"FineType entity subtype accuracy (on mapped): {entity_correct/entity_mapped*100:.1f}%")
    print(f"FineType entity subtype accuracy (on all): {entity_correct/entity_total*100:.1f}%")
    print(f"Sense A entity subtype accuracy: 78.0%")
    print(f"\nNote: FineType lacks explicit entity subtyping. Most entities → entity_name")
    print(f"(unmapped). Only person (full_name), place (geography.*), org (company_name)")
    print(f"have natural mappings.")

    if subtype_confusion:
        print(f"\nEntity subtype mismatches (FineType mapped):")
        for (gt, pred), count in subtype_confusion.most_common(10):
            print(f"  {gt} → {pred}: {count}")

    # ===== SPEED COMPARISON =====
    print(f"\n{'=' * 60}")
    print(f"  Speed Comparison")
    print(f"{'=' * 60}")
    ft_ms_per_col = elapsed / len(val_data) * 1000
    print(f"\nFineType: {ft_ms_per_col:.1f}ms/column ({len(val_data)} columns, {elapsed:.1f}s total)")
    print(f"Sense A:  3.6ms/column (50 values)")
    print(f"Speedup:  {ft_ms_per_col/3.6:.0f}x faster with Sense A")

    # ===== SUMMARY =====
    print(f"\n{'=' * 60}")
    print(f"  Summary Comparison")
    print(f"{'=' * 60}")
    print(f"\n{'Metric':<35} {'FineType':>12} {'Sense A':>12} {'Winner':>8}")
    print("-" * 70)
    ft_pct = f"{ft_accuracy*100:.1f}%"
    sense_pct = "88.5%"
    print(f"{'Broad category accuracy':<35} {ft_pct:>12} {sense_pct:>12} {'FT' if ft_accuracy > 0.885 else 'Sense':>8}")
    print(f"{'Entity subtype accuracy':<35} {'N/A':>12} {'78.0%':>12} {'Sense':>8}")
    print(f"{'Speed (ms/column)':<35} {f'{ft_ms_per_col:.0f}ms':>12} {'3.6ms':>12} {'Sense':>8}")
    print(f"{'Model count':<35} {'34+1':>12} {'1':>12} {'Sense':>8}")
    print(f"{'Header required?':<35} {'Yes':>12} {'No*':>12} {'Sense':>8}")
    print(f"\n* Sense A trained with 50% header dropout — works without headers")

    # Save results
    results = {
        "finetype_broad_accuracy": ft_accuracy,
        "sense_a_broad_accuracy": 0.8849,
        "finetype_ms_per_column": ft_ms_per_col,
        "sense_a_ms_per_column": 3.6,
        "n_columns": len(val_data),
        "finetype_per_category": {},
        "sense_a_per_category_f1": sense_f1,
        "entity_total": entity_total,
        "entity_mapped": entity_mapped,
        "entity_correct": entity_correct,
    }
    for cat in broad_categories:
        support = tp[cat] + fn[cat]
        correct = tp[cat]
        recall = correct / support if support > 0 else 0
        precision = tp[cat] / (tp[cat] + fp[cat]) if (tp[cat] + fp[cat]) > 0 else 0
        f1 = 2 * precision * recall / (precision + recall) if (precision + recall) > 0 else 0
        results["finetype_per_category"][cat] = {
            "support": support, "correct": correct,
            "precision": precision, "recall": recall, "f1": f1,
        }

    out_path = Path("models/sense_spike/comparison_results.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(results, f, indent=2)
    print(f"\nResults saved to {out_path}")


if __name__ == "__main__":
    main()
