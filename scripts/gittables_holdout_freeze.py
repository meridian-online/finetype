#!/usr/bin/env python3
"""Freeze the GitTables holdout slice — autonomy-contract gate-metric surface.

Implements:
  - bead finetype-e6d, ac-01 (deterministic holdout, byte-identical re-run)
  - bead finetype-s16, ac-02 (content-hash dedup before stratified sampling)
  - bead finetype-s16, ac-05 (working-slice tracker uses content-hash key)

Algorithm:
  1. Walk <root>/*/*.parquet (one level — topic-cluster directories).
  2. Sort all paths lexicographically (determinism prerequisite).
  3. For each path, compute SHA256 over file bytes.
  4. Dedupe by content-hash: keep the first lex-sorted path per hash.
     This is what prevents the GitTables miles_per_hour/ /
     inflation_rate/ shared-bytes leak (s16 motivation).
  5. Group survivors by topic-cluster (parent directory name).
  6. Stratified random sample with proportional allocation:
       per_cluster_take = round(holdout_size * cluster_pop / dedup_pop)
     using a seeded RNG (random.Random(seed).sample).
  7. Apply integer-rounding correction so the final count equals
     holdout_size exactly (largest-remainders adjustment).
  8. Sort the selected paths lexicographically.
  9. Emit to <output> with a header recording seed, version, counts.

Re-running with identical seed + identical filesystem state produces
a byte-identical output file (the test for ac-01).

Hash computation is the bulk of the cost — ~1M parquet files at
~50 KB each is ~30 min on M1. The `--hash-cache` flag caches
(path, mtime, size, hash) tuples between runs so re-freezes are cheap.
The cache is keyed by (path, mtime, size); a file edit bumps mtime
and forces rehash. The cache file is local-only (not committed); the
freeze output is the source of truth for the contract.

Usage:
    python3 scripts/gittables_holdout_freeze.py
    python3 scripts/gittables_holdout_freeze.py --dry-run
    python3 scripts/gittables_holdout_freeze.py --root /path/to/gittables
    python3 scripts/gittables_holdout_freeze.py --holdout-size 2000 --seed 20260503

See:
    orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml §1
    .orbit/choices/0056-train-eval-leakage-prevention.md
"""

from __future__ import annotations

import argparse
import hashlib
import os
import random
import sys
import time
from pathlib import Path

_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

# pyright: reportMissingImports=false
from eval_leakage.content_hash import (  # noqa: E402
    CONTENT_HASH_VERSION,
    file_content_sha256,
)


REPO_ROOT = _HERE.parent
DEFAULT_ROOT = Path("/Users/hugh/datasets/gittables")
DEFAULT_OUTPUT = REPO_ROOT / "eval" / "gittables" / "holdout_paths.txt"
DEFAULT_HASH_CACHE = REPO_ROOT / "eval" / "gittables" / ".content_hash_cache.tsv"
DEFAULT_HOLDOUT_SIZE = 2000
DEFAULT_SEED = 20260503


def _walk_parquets(root: Path) -> list[Path]:
    """Return all *.parquet files at depth 1 (root/<cluster>/*.parquet).

    Sorted lexicographically. Anything beyond depth 1 is ignored —
    GitTables layout is one cluster per directory and we don't recurse.
    """
    if not root.exists():
        raise FileNotFoundError(f"GitTables root not found: {root}")
    paths: list[Path] = []
    for cluster_dir in sorted(p for p in root.iterdir() if p.is_dir()):
        for parquet in sorted(cluster_dir.glob("*.parquet")):
            paths.append(parquet)
    return paths


def _load_hash_cache(cache_path: Path) -> dict[str, tuple[int, int, str]]:
    """Load (path → (mtime_ns, size, hash)) cache.

    Tolerant of missing or malformed entries — bad lines are skipped.
    """
    if not cache_path.exists():
        return {}
    out: dict[str, tuple[int, int, str]] = {}
    with open(cache_path, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) != 4:
                continue
            path, mtime_s, size_s, h = parts
            try:
                out[path] = (int(mtime_s), int(size_s), h)
            except ValueError:
                continue
    return out


def _save_hash_cache(
    cache_path: Path, cache: dict[str, tuple[int, int, str]]
) -> None:
    cache_path.parent.mkdir(parents=True, exist_ok=True)
    tmp = cache_path.with_suffix(cache_path.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write(f"# content_hash_version: {CONTENT_HASH_VERSION}\n")
        fh.write("# path\tmtime_ns\tsize\tsha256\n")
        for path in sorted(cache):
            mtime_ns, size, h = cache[path]
            fh.write(f"{path}\t{mtime_ns}\t{size}\t{h}\n")
    os.replace(tmp, cache_path)


def _compute_or_load_hashes(
    paths: list[Path],
    cache_path: Path | None,
    progress_every: int = 5000,
) -> dict[Path, str]:
    """Returns path → sha256, using a (mtime, size) cache when available."""
    cache: dict[str, tuple[int, int, str]] = (
        _load_hash_cache(cache_path) if cache_path else {}
    )
    out: dict[Path, str] = {}
    n_cached = 0
    n_computed = 0
    t0 = time.perf_counter()
    for i, p in enumerate(paths, 1):
        try:
            stat = p.stat()
        except FileNotFoundError:
            continue
        key = str(p)
        cached = cache.get(key)
        if cached is not None and cached[0] == stat.st_mtime_ns and cached[1] == stat.st_size:
            out[p] = cached[2]
            n_cached += 1
        else:
            h = file_content_sha256(p)
            out[p] = h
            cache[key] = (stat.st_mtime_ns, stat.st_size, h)
            n_computed += 1
        if i % progress_every == 0:
            elapsed = time.perf_counter() - t0
            rate = i / elapsed if elapsed else 0
            print(
                f"  hashed {i}/{len(paths)} "
                f"({n_cached} cached, {n_computed} computed, "
                f"{rate:.0f} files/s)",
                file=sys.stderr,
            )
    if cache_path is not None and n_computed > 0:
        _save_hash_cache(cache_path, cache)
    print(
        f"  hash totals: {n_cached} cached, {n_computed} computed",
        file=sys.stderr,
    )
    return out


def _dedupe_by_content_hash(
    paths: list[Path], hashes: dict[Path, str]
) -> list[Path]:
    """Keep the first lex-sorted path per content-hash.

    Input must already be lex-sorted; the iteration order determines
    which path wins on a collision. Determinism is the contract.
    """
    seen: set[str] = set()
    survivors: list[Path] = []
    for p in paths:
        h = hashes.get(p)
        if h is None:
            continue
        if h in seen:
            continue
        seen.add(h)
        survivors.append(p)
    return survivors


def _stratified_sample(
    paths: list[Path],
    holdout_size: int,
    seed: int,
) -> list[Path]:
    """Proportional-allocation stratified sample by topic-cluster.

    Each cluster receives a quota proportional to its share of the
    deduped population. Largest-remainders correction ensures the
    sum equals holdout_size exactly. Selection within a cluster uses
    a seeded RNG (random.Random(seed).sample); cluster ordering is
    lexicographic; the global RNG is namespaced per cluster by
    deriving cluster seeds from a single base seed (so re-runs are
    reproducible regardless of cluster-iteration order).
    """
    by_cluster: dict[str, list[Path]] = {}
    for p in paths:
        cluster = p.parent.name
        by_cluster.setdefault(cluster, []).append(p)

    total = sum(len(v) for v in by_cluster.values())
    if total < holdout_size:
        raise ValueError(
            f"deduped population ({total}) < holdout_size ({holdout_size})"
        )

    # Largest-remainders proportional allocation.
    quotas_float = {
        cluster: holdout_size * len(items) / total
        for cluster, items in by_cluster.items()
    }
    quotas: dict[str, int] = {c: int(q) for c, q in quotas_float.items()}
    remainders = sorted(
        (
            (quotas_float[c] - quotas[c], c)
            for c in by_cluster
        ),
        # Larger remainder first; tie-break alphabetically (stable seed).
        key=lambda t: (-t[0], t[1]),
    )
    deficit = holdout_size - sum(quotas.values())
    for _, c in remainders[:deficit]:
        quotas[c] += 1

    # Per-cluster seeded sampling. Cluster seed is derived deterministically
    # from (base_seed, cluster_name) via SHA256 — Python's built-in hash() is
    # randomised per-invocation under PYTHONHASHSEED and would break the
    # byte-identical re-run contract (e6d ac-05).
    selected: list[Path] = []
    for cluster in sorted(by_cluster):
        items = by_cluster[cluster]
        take = quotas[cluster]
        if take == 0:
            continue
        if take >= len(items):
            selected.extend(items)
            continue
        digest = hashlib.sha256(
            f"{seed}\x00{cluster}".encode("utf-8")
        ).digest()
        cluster_seed = int.from_bytes(digest[:8], "big", signed=False)
        rng = random.Random(cluster_seed)
        selected.extend(rng.sample(items, take))
    return sorted(selected)


def _emit(
    output: Path,
    selected: list[Path],
    seed: int,
    holdout_size: int,
    dedup_pop: int,
    raw_pop: int,
    root: Path,
) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    tmp = output.with_suffix(output.suffix + ".tmp")
    with open(tmp, "w", encoding="utf-8") as fh:
        fh.write("# GitTables autonomy-contract gate-metric holdout\n")
        fh.write(f"# content_hash_version: {CONTENT_HASH_VERSION}\n")
        fh.write(f"# seed: {seed}\n")
        fh.write(f"# holdout_size: {holdout_size}\n")
        fh.write(f"# dedup_population: {dedup_pop}\n")
        fh.write(f"# raw_population: {raw_pop}\n")
        fh.write(f"# root: {root}\n")
        fh.write(
            "# regenerate via: python3 scripts/gittables_holdout_freeze.py\n"
        )
        for p in selected:
            fh.write(f"{p}\n")
    os.replace(tmp, output)


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(__doc__ or "").split("\n", maxsplit=1)[0]
    )
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument(
        "--hash-cache",
        type=Path,
        default=DEFAULT_HASH_CACHE,
        help=(
            "Path to (path, mtime, size, sha256) cache TSV. "
            "Default: eval/gittables/.content_hash_cache.tsv (gitignored)."
        ),
    )
    parser.add_argument("--no-hash-cache", action="store_true")
    parser.add_argument("--holdout-size", type=int, default=DEFAULT_HOLDOUT_SIZE)
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print summary; do not write the output file.",
    )
    parser.add_argument(
        "--max-files",
        type=int,
        default=None,
        help="Cap walked-file count (debugging only — breaks reproducibility).",
    )
    args = parser.parse_args()

    print(f"holdout-freeze: root={args.root}", file=sys.stderr)
    print(f"  seed={args.seed} holdout_size={args.holdout_size}", file=sys.stderr)

    paths = _walk_parquets(args.root)
    if args.max_files is not None:
        paths = paths[: args.max_files]
        print(
            f"  WARNING: --max-files capped walk to {len(paths)} (non-reproducible)",
            file=sys.stderr,
        )
    print(f"  raw population: {len(paths)} parquet files", file=sys.stderr)

    cache_path = None if args.no_hash_cache else args.hash_cache
    hashes = _compute_or_load_hashes(paths, cache_path)
    deduped = _dedupe_by_content_hash(paths, hashes)
    print(
        f"  deduped population: {len(deduped)} "
        f"({len(paths) - len(deduped)} duplicates removed)",
        file=sys.stderr,
    )

    selected = _stratified_sample(deduped, args.holdout_size, args.seed)
    print(f"  selected: {len(selected)}", file=sys.stderr)

    if args.dry_run:
        for p in selected[:10]:
            print(p)
        print(f"... ({len(selected)} total) [DRY-RUN — no file written]")
        return 0

    _emit(
        args.output,
        selected,
        seed=args.seed,
        holdout_size=args.holdout_size,
        dedup_pop=len(deduped),
        raw_pop=len(paths),
        root=args.root,
    )
    print(f"  wrote {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
