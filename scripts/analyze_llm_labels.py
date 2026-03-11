#!/usr/bin/env python3
"""Analyze LLM labelling results from llm_label.sh output."""

import csv
import sys
from collections import Counter, defaultdict

def main():
    if len(sys.argv) < 2:
        print("Usage: python3 analyze_llm_labels.py <labels.csv>")
        sys.exit(1)

    filepath = sys.argv[1]

    rows = []
    with open(filepath, 'r') as f:
        reader = csv.DictReader(f)
        for row in reader:
            rows.append(row)

    total = len(rows)
    if total == 0:
        print("No data found.")
        return

    # --- Basic stats ---
    valid = sum(1 for r in rows if r.get('llm_valid') == 'yes')
    invalid = total - valid
    valid_rows = [r for r in rows if r.get('llm_valid') == 'yes']

    has_ft = any(r.get('finetype_label') for r in valid_rows)
    agree = sum(1 for r in valid_rows if r.get('agreement') == 'yes')
    disagree_rows = [r for r in valid_rows if r.get('agreement') == 'no']

    print("=" * 60)
    print("  LLM Labelling Analysis")
    print("=" * 60)
    print(f"  Total columns:     {total}")
    print(f"  Valid labels:      {valid} ({valid*100//total}%)")
    print(f"  Invalid labels:    {invalid} ({invalid*100//total}%)")
    if has_ft:
        compared = sum(1 for r in valid_rows if r.get('agreement') in ('yes', 'no'))
        print(f"  Agreement:         {agree}/{compared} ({agree*100//max(compared,1)}%)")
        print(f"  Disagreements:     {len(disagree_rows)}")
    print()

    # --- Type distribution ---
    llm_types = Counter(r['llm_label'] for r in valid_rows)
    print(f"  Types seen:        {len(llm_types)} / 250")
    print()
    print("  Top 20 LLM-assigned types:")
    for typ, count in llm_types.most_common(20):
        print(f"    {typ:50s} {count:5d}")
    print()

    # --- Domain distribution ---
    domain_counts = Counter(r['llm_label'].split('.')[0] for r in valid_rows if '.' in r['llm_label'])
    print("  Domain distribution:")
    for domain, count in sorted(domain_counts.items(), key=lambda x: -x[1]):
        print(f"    {domain:20s} {count:5d} ({count*100//max(valid,1)}%)")
    print()

    # --- Disagreement analysis ---
    if has_ft and disagree_rows:
        print("  Top disagreements (LLM vs FineType):")
        confusion = Counter()
        for r in disagree_rows:
            pair = (r['llm_label'], r.get('finetype_label', ''))
            confusion[pair] += 1

        for (llm, ft), count in confusion.most_common(20):
            print(f"    {llm:40s} vs {ft:40s} ({count}x)")
        print()

        # Domain-level disagreements
        domain_disagree = sum(
            1 for r in disagree_rows
            if r['llm_label'].split('.')[0] != r.get('finetype_label', '').split('.')[0]
        )
        print(f"  Domain-level disagreements: {domain_disagree}/{len(disagree_rows)}")
        print(f"  Same-domain disagreements:  {len(disagree_rows) - domain_disagree}/{len(disagree_rows)}")
        print()

    # --- Invalid label analysis ---
    if invalid > 0:
        invalid_labels = Counter(r['llm_label'] for r in rows if r.get('llm_valid') != 'yes')
        print("  Top invalid labels (LLM hallucinations):")
        for label, count in invalid_labels.most_common(10):
            print(f"    {label:50s} {count:5d}")
        print()

    # --- Coverage gaps ---
    if len(llm_types) < 250:
        print(f"  Types NOT seen in real-world data: {250 - len(llm_types)}")
        print("  (Run on more data sources to improve coverage)")
    print()
    print("=" * 60)


if __name__ == '__main__':
    main()
