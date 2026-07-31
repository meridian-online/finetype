#!/usr/bin/env python3
"""Validate data/label_remap.json against the canonical FineType taxonomy.

Purpose
-------
The label_remap is the bridge between distilled (Sherlock / other corpus)
labels and FineType's canonical taxonomy keys. When a remap entry
`A -> B` exists, the training pipeline rewrites `A` as `B`. Chains are
allowed (`A -> B -> C`), but every chain MUST terminate at a canonical
taxonomy key — a key that appears in `labels/definitions_*.yaml`.

A broken chain (terminal value is not a canonical key) means training
drops rows silently, which was a real bug class before v16 (see the m-18
audit — description/title/sentence/paragraph all chained to labels that
didn't exist). This script catches those failures before training starts.

Scope
-----
Chain traversal + canonical-key resolution ONLY. Taxonomy validation
itself (are the YAML files well-formed?) is `finetype check`'s job.

Exit codes
----------
0  all chains resolve to canonical keys; no cycles
1  one or more chains fail (details printed)

Usage
-----
    ./scripts/validate_label_remap.py
    ./scripts/validate_label_remap.py --remap path/to/label_remap.json
    ./scripts/validate_label_remap.py --labels-dir labels/

Spec reference
--------------
AC-05 of spec 2026-04-20-distilled-data-relabel-7-types (v1.3).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_REMAP = REPO_ROOT / "data" / "label_remap.json"
DEFAULT_LABELS_DIR = REPO_ROOT / "labels"

# A canonical key looks like `domain.category.type` — three dotted
# lowercase identifiers. Top-level YAML keys matching this pattern are
# the canonical taxonomy.
KEY_RE = re.compile(r"^[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*\.[a-z][a-z0-9_]*:\s*$", re.MULTILINE)


def load_canonical_keys(labels_dir: Path) -> set[str]:
    """Scan labels/definitions_*.yaml and return the set of canonical keys.

    We intentionally parse the YAML ourselves (regex on top-level keys)
    rather than use PyYAML: (a) no external dependency needed for a
    pre-training validator; (b) the format is stable; (c) faster.
    """
    if not labels_dir.is_dir():
        raise SystemExit(f"ERROR: labels directory not found: {labels_dir}")
    keys: set[str] = set()
    for yaml_file in sorted(labels_dir.glob("definitions_*.yaml")):
        text = yaml_file.read_text(encoding="utf-8")
        for match in KEY_RE.finditer(text):
            # Strip the trailing colon + whitespace.
            key = match.group(0).rstrip(": \n\t")
            keys.add(key)
    if not keys:
        raise SystemExit(
            f"ERROR: no canonical keys found in {labels_dir} — is the "
            f"labels directory path correct?"
        )
    return keys


def load_remap(path: Path) -> dict[str, str]:
    """Load label_remap.json, stripping the `_comment` field if present."""
    if not path.is_file():
        raise SystemExit(f"ERROR: remap file not found: {path}")
    with path.open(encoding="utf-8") as f:
        raw = json.load(f)
    if not isinstance(raw, dict):
        raise SystemExit(f"ERROR: remap root must be an object, got {type(raw).__name__}")
    return {k: v for k, v in raw.items() if not k.startswith("_")}


def resolve_chain(
    start: str, remap: dict[str, str], max_depth: int = 20
) -> tuple[list[str], str | None]:
    """Follow `start -> remap[start] -> remap[remap[start]] -> …` until we
    hit a key not in the remap (the terminal value).

    Returns (chain, error) where:
      - chain is the list of visited keys (including start and terminal)
      - error is None on success, or a descriptive message on failure
        (cycle detected, depth exceeded)
    """
    chain: list[str] = [start]
    seen = {start}
    current = start
    for _ in range(max_depth):
        nxt = remap.get(current)
        if nxt is None:
            return chain, None  # `current` is the terminal (not in remap)
        if nxt in seen:
            chain.append(nxt)
            return chain, f"cycle detected: {' -> '.join(chain)}"
        chain.append(nxt)
        seen.add(nxt)
        current = nxt
    return chain, f"chain exceeds max_depth={max_depth}: {' -> '.join(chain)}"


def validate(
    remap: dict[str, str], canonical: set[str]
) -> tuple[list[str], list[str]]:
    """Walk every chain. Return (errors, warnings)."""
    errors: list[str] = []
    warnings: list[str] = []
    for source in sorted(remap):
        chain, err = resolve_chain(source, remap)
        if err is not None:
            errors.append(f"[{source}] {err}")
            continue
        terminal = chain[-1]
        if terminal not in canonical:
            errors.append(
                f"[{source}] chain terminates at non-canonical label "
                f"`{terminal}` (chain: {' -> '.join(chain)})"
            )
        elif len(chain) > 2:
            # Long chain — valid but worth flagging for human review.
            warnings.append(
                f"[{source}] chain length {len(chain)}: {' -> '.join(chain)}"
            )
    return errors, warnings


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(__doc__ or "").split("\n\n")[0]
    )
    parser.add_argument("--remap", type=Path, default=DEFAULT_REMAP,
                        help=f"Path to label_remap.json (default: {DEFAULT_REMAP})")
    parser.add_argument("--labels-dir", type=Path, default=DEFAULT_LABELS_DIR,
                        help=f"Path to labels/ (default: {DEFAULT_LABELS_DIR})")
    parser.add_argument("--quiet", action="store_true",
                        help="Print only on failure.")
    args = parser.parse_args()

    canonical = load_canonical_keys(args.labels_dir)
    remap = load_remap(args.remap)

    if not args.quiet:
        print(f"Canonical taxonomy keys: {len(canonical)}")
        print(f"Remap entries:           {len(remap)}")

    errors, warnings = validate(remap, canonical)

    if warnings and not args.quiet:
        print(f"\n{len(warnings)} warning(s):", file=sys.stderr)
        for w in warnings:
            print(f"  WARN: {w}", file=sys.stderr)

    if errors:
        print(f"\n{len(errors)} broken chain(s):", file=sys.stderr)
        for e in errors:
            print(f"  ERROR: {e}", file=sys.stderr)
        return 1

    if not args.quiet:
        print("\n✅ all remap chains resolve to canonical taxonomy keys")
    return 0


if __name__ == "__main__":
    sys.exit(main())
