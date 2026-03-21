#!/usr/bin/env python3
"""Read and inspect a .ftmb (FineType Multi-Branch) binary file.

Usage:
    python3 scripts/read_ftmb.py <file.ftmb> [--records N] [--type KEY] [--stats]

Options:
    --records N     Show first N records (default: 10)
    --type KEY      Filter to specific type key
    --stats         Show per-type summary statistics
    --verify        Verify all feature vectors have correct dimensions and no NaN/Inf
"""

import math
import struct
import sys
from collections import Counter

MAGIC = b"FTMB"


def read_header(f):
    """Read and validate the FTMB header. Returns (version, n_records, char_dim, embed_dim, stats_dim)."""
    magic = f.read(4)
    if magic != MAGIC:
        print(f"ERROR: Bad magic bytes: {magic!r} (expected {MAGIC!r})", file=sys.stderr)
        sys.exit(1)

    (version,) = struct.unpack("<I", f.read(4))
    (n_records,) = struct.unpack("<Q", f.read(8))
    char_dim, embed_dim, stats_dim = struct.unpack("<HHH", f.read(6))
    _padding = f.read(2)

    return version, n_records, char_dim, embed_dim, stats_dim


def read_record(f, char_dim, embed_dim, stats_dim):
    """Read a single record. Returns (label, char_feat, embed_feat, stats_feat) or None at EOF."""
    label_len_bytes = f.read(2)
    if len(label_len_bytes) < 2:
        return None
    (label_len,) = struct.unpack("<H", label_len_bytes)
    label = f.read(label_len).decode("utf-8")
    char_feat = list(struct.unpack(f"<{char_dim}f", f.read(char_dim * 4)))
    embed_feat = list(struct.unpack(f"<{embed_dim}f", f.read(embed_dim * 4)))
    stats_feat = list(struct.unpack(f"<{stats_dim}f", f.read(stats_dim * 4)))
    return label, char_feat, embed_feat, stats_feat


def feat_summary(feat):
    """Return a one-line summary of a feature vector."""
    nonzero = sum(1 for v in feat if abs(v) > 1e-10)
    has_nan = any(math.isnan(v) for v in feat)
    has_inf = any(math.isinf(v) for v in feat)
    min_v = min(feat) if feat else 0.0
    max_v = max(feat) if feat else 0.0
    mean_v = sum(feat) / len(feat) if feat else 0.0
    flags = ""
    if has_nan:
        flags += " [NaN!]"
    if has_inf:
        flags += " [Inf!]"
    return f"dim={len(feat)}, nonzero={nonzero}, range=[{min_v:.4f}, {max_v:.4f}], mean={mean_v:.4f}{flags}"


def main():
    args = sys.argv[1:]
    if not args or args[0] in ("-h", "--help"):
        print(__doc__)
        sys.exit(0)

    path = args[0]
    max_records = 10
    filter_type = None
    show_stats = False
    verify = False

    i = 1
    while i < len(args):
        if args[i] == "--records":
            max_records = int(args[i + 1])
            i += 2
        elif args[i] == "--type":
            filter_type = args[i + 1]
            i += 2
        elif args[i] == "--stats":
            show_stats = True
            i += 1
        elif args[i] == "--verify":
            verify = True
            i += 1
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    with open(path, "rb") as f:
        version, n_records, char_dim, embed_dim, stats_dim = read_header(f)

        print(f"FTMB File: {path}")
        print(f"  Version:    {version}")
        print(f"  Records:    {n_records}")
        print(f"  Char dim:   {char_dim}")
        print(f"  Embed dim:  {embed_dim}")
        print(f"  Stats dim:  {stats_dim}")
        print()

        type_counts = Counter()
        shown = 0
        issues = 0

        for idx in range(n_records):
            record = read_record(f, char_dim, embed_dim, stats_dim)
            if record is None:
                print(f"WARNING: EOF at record {idx} (expected {n_records})")
                break

            label, char_feat, embed_feat, stats_feat = record
            type_counts[label] += 1

            if verify:
                for name, feat in [("char", char_feat), ("embed", embed_feat), ("stats", stats_feat)]:
                    for v in feat:
                        if math.isnan(v) or math.isinf(v):
                            print(f"  ISSUE: record {idx} ({label}) has NaN/Inf in {name}")
                            issues += 1
                            break

            if filter_type and label != filter_type:
                continue

            if shown < max_records:
                print(f"Record {idx}: {label}")
                print(f"  char:  {feat_summary(char_feat)}")
                print(f"  embed: {feat_summary(embed_feat)}")
                print(f"  stats: {feat_summary(stats_feat)}")
                shown += 1

        if show_stats:
            print(f"\n{'='*60}")
            print(f"Type distribution ({len(type_counts)} types):")
            for type_key, count in type_counts.most_common():
                print(f"  {type_key}: {count}")
            print(f"{'='*60}")

        if verify:
            if issues == 0:
                print(f"\nVerification PASSED: {n_records} records, no NaN/Inf values")
            else:
                print(f"\nVerification FAILED: {issues} issues found")
                sys.exit(1)


if __name__ == "__main__":
    main()
