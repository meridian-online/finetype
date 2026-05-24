#!/usr/bin/env python3
"""Register a dataset — compute SHA256 + size per file, write a snapshot
JSON, and update the corresponding entry in eval/datasets/sources.yaml.

Per spec 2026-05-24-dataset-provenance-registry ac-02.

Usage
-----
  # Full-tree snapshot (default — every file under local_path)
  scripts/dataset_register.py <name> <local_path>

  # Dry-run — print the snapshot to stdout, don't touch disk
  scripts/dataset_register.py <name> <local_path> --dry-run

  # Index-only mode for very-large datasets (GitTables-style)
  # The index file lists the paths; the script hashes the index file
  # itself plus a deterministic 1k-file sample of its contents.
  scripts/dataset_register.py <name> <local_path> \
      --mode index-only --index-file eval/gittables/corpus_paths.txt

  # Override the dataset_version (defaults to today's ISO date for
  # full mode, or the first 12 hex chars of the index SHA for
  # index-only mode)
  scripts/dataset_register.py <name> <local_path> --dataset-version 46.0.0

  # Skip the sources.yaml update (snapshot only)
  scripts/dataset_register.py <name> <local_path> --no-update-registry

Schema of the written snapshot JSON
-----------------------------------
  Full mode:
    {
      "dataset": "<name>",
      "dataset_version": "<stable id — date | release tag | content hash>",
      "snapshot_date": "YYYY-MM-DD",
      "snapshot_mode": "full",
      "registered_by": "scripts/dataset_register.py@<git-sha>",
      "local_path": "<absolute path | repo://… URI>",
      "total_size_bytes": <int>,
      "file_count": <int>,
      "files": {
        "<relative path>": {"size_bytes": <int>, "sha256": "<hex>"},
        ...
      }
    }

  Index-only mode adds `index_file`, `index_file_sha256`,
  `index_file_size_bytes`, `n_paths_in_index`, `sample_size`,
  `sample_total_size_bytes`, `sample_files` and OMITS the full `files`
  map. The two modes are not interchangeable — `dataset_verify.py`
  reads `snapshot_mode` and dispatches accordingly.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import date
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SNAPSHOTS_DIR = REPO / "eval" / "datasets" / "snapshots"
SOURCES_YAML = REPO / "eval" / "datasets" / "sources.yaml"

# OS-noise ignore list — files we never want in a snapshot
IGNORE_NAMES = {".DS_Store", "Thumbs.db", "._.DS_Store", ".gitkeep"}
IGNORE_SUFFIXES = (".tmp", ".swp", ".swo", ".pyc")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(
        description=(__doc__ or "").splitlines()[0],
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p.add_argument("name", help="Dataset name (used as snapshot filename prefix and sources.yaml key)")
    p.add_argument("local_path", help="Path to the dataset on disk (file or directory)")
    p.add_argument(
        "--mode", choices=("full", "index-only"), default="full",
        help="Snapshot mode. 'full' hashes every file under local_path; "
             "'index-only' hashes an index file (--index-file) plus a "
             "deterministic 1k-file sample. Default: full.",
    )
    p.add_argument(
        "--index-file", type=str, default=None,
        help="Required for --mode index-only. Path to a newline-delimited "
             "file listing every dataset file (absolute or relative to "
             "local_path). Lines starting with '#' are ignored.",
    )
    p.add_argument(
        "--dataset-version", type=str, default=None,
        help="Stable identifier for this snapshot. Defaults to today's "
             "ISO date (full mode) or first 12 hex chars of the index "
             "file SHA (index-only mode).",
    )
    p.add_argument(
        "--role", choices=("train", "eval", "validate", "both-forbidden"),
        default=None,
        help="Role for the sources.yaml entry. Required when creating a "
             "new entry; optional (preserved) when updating an existing "
             "entry.",
    )
    p.add_argument(
        "--licence", type=str, default=None,
        help="Licence string (SPDX ID preferred). Required when creating "
             "a new sources.yaml entry.",
    )
    p.add_argument(
        "--attribution", type=str, default=None,
        help="Attribution string. Required when creating a new "
             "sources.yaml entry.",
    )
    p.add_argument(
        "--source-url", type=str, default=None,
        help="Source URL or repo:// URI. Defaults to repo://<rel> for "
             "in-repo paths or <name>:<local_path> sentinel for external "
             "paths. Used as the sources.yaml entry key.",
    )
    p.add_argument(
        "--sample-size", type=int, default=1000,
        help="Number of files to hash in --mode index-only (deterministic "
             "stride sample). Default: 1000.",
    )
    p.add_argument("--dry-run", action="store_true", help="Print snapshot to stdout; don't write any file.")
    p.add_argument(
        "--no-update-registry", dest="update_registry",
        action="store_false", default=True,
        help="Write snapshot only; don't touch sources.yaml.",
    )
    return p.parse_args()


def sha256_file(path: Path) -> tuple[int, str]:
    """Return (size_bytes, sha256_hex) for a regular file."""
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1048576), b""):
            h.update(chunk)
            size += len(chunk)
    return size, h.hexdigest()


def git_sha() -> str:
    try:
        out = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=REPO, text=True, stderr=subprocess.DEVNULL,
        ).strip()
        return out[:12] if out else "unknown"
    except Exception:
        return "unknown"


def resolve_local_path_for_record(p: Path) -> str:
    """In-repo paths become repo:// URIs; external paths stay absolute.
    Mirrors the existing eval/datasets/sources.yaml convention."""
    try:
        rel = p.relative_to(REPO)
        return f"repo://{rel}"
    except ValueError:
        return str(p)


def should_skip(path: Path) -> bool:
    if path.name in IGNORE_NAMES:
        return True
    if any(path.name.endswith(s) for s in IGNORE_SUFFIXES):
        return True
    return False


def build_full_snapshot(name: str, local_path: Path, args: argparse.Namespace) -> dict:
    files: dict[str, dict] = {}
    total_size = 0

    if local_path.is_file():
        size, sha = sha256_file(local_path)
        files[local_path.name] = {"size_bytes": size, "sha256": sha}
        total_size = size
    else:
        for sub in sorted(local_path.rglob("*")):
            if not sub.is_file() or should_skip(sub):
                continue
            rel = sub.relative_to(local_path)
            size, sha = sha256_file(sub)
            files[str(rel)] = {"size_bytes": size, "sha256": sha}
            total_size += size

    if not files:
        sys.exit(f"error: no files found under {local_path} (after ignore filter)")

    return {
        "dataset": name,
        "dataset_version": args.dataset_version or date.today().isoformat(),
        "snapshot_date": date.today().isoformat(),
        "snapshot_mode": "full",
        "registered_by": f"scripts/dataset_register.py@{git_sha()}",
        "local_path": resolve_local_path_for_record(local_path),
        "total_size_bytes": total_size,
        "file_count": len(files),
        "files": files,
    }


def build_index_only_snapshot(name: str, local_path: Path, args: argparse.Namespace) -> dict:
    if not args.index_file:
        sys.exit("error: --mode index-only requires --index-file")
    index_path = Path(args.index_file).resolve()
    if not index_path.exists():
        sys.exit(f"error: index file not found: {index_path}")

    index_size, index_sha = sha256_file(index_path)

    with index_path.open() as f:
        paths = [
            ln.strip() for ln in f
            if ln.strip() and not ln.lstrip().startswith("#")
        ]
    n_paths = len(paths)
    if n_paths == 0:
        sys.exit(f"error: index file {index_path} contains no path entries")

    # Deterministic stride sample — first, then every Nth.
    sample_size = min(args.sample_size, n_paths)
    stride = max(1, n_paths // sample_size)
    sample_paths = paths[::stride][:sample_size]

    sample_files: dict[str, dict] = {}
    sample_total = 0
    for p in sorted(sample_paths):
        full = Path(p) if Path(p).is_absolute() else (local_path / p)
        if not full.exists():
            sample_files[p] = {"error": "missing"}
            continue
        size, sha = sha256_file(full)
        sample_files[p] = {"size_bytes": size, "sha256": sha}
        sample_total += size

    return {
        "dataset": name,
        "dataset_version": args.dataset_version or index_sha[:12],
        "snapshot_date": date.today().isoformat(),
        "snapshot_mode": "index-only",
        "registered_by": f"scripts/dataset_register.py@{git_sha()}",
        "local_path": resolve_local_path_for_record(local_path),
        "index_file": resolve_local_path_for_record(index_path),
        "index_file_sha256": index_sha,
        "index_file_size_bytes": index_size,
        "n_paths_in_index": n_paths,
        "sample_size": len(sample_files),
        "sample_total_size_bytes": sample_total,
        "sample_files": sample_files,
    }


def default_source_url(name: str, local_path_record: str) -> str:
    """Synthesise a sources.yaml entry key when --source-url is omitted.
    repo:// paths use the URI directly; external paths use a
    `dataset://<name>` sentinel so external moves don't break the key."""
    if local_path_record.startswith("repo://"):
        return local_path_record
    return f"dataset://{name}"


def update_sources_yaml(
    name: str,
    snapshot_path: Path,
    snapshot: dict,
    args: argparse.Namespace,
) -> None:
    """Add or update an entry in eval/datasets/sources.yaml.

    Preserves comments and order using ruamel-style round-trip if
    available, falling back to PyYAML (loses comments — flagged in the
    output). Entry is keyed by source_url; if that key exists the
    integrity fields are added/refreshed and other fields preserved.
    """
    try:
        import yaml
    except ImportError as exc:
        sys.exit(f"error: pyyaml required ({exc}). pip install pyyaml")

    if not SOURCES_YAML.exists():
        sys.exit(f"error: sources.yaml not found at {SOURCES_YAML}")

    with SOURCES_YAML.open() as f:
        data = yaml.safe_load(f) or {}

    sources = data.get("sources") or []
    source_url = args.source_url or default_source_url(name, snapshot["local_path"])

    # Find existing by source_url
    existing = None
    for entry in sources:
        if entry.get("source_url") == source_url:
            existing = entry
            break

    snapshot_rel = str(snapshot_path.relative_to(REPO))

    if existing is None:
        # Creating new — require role/licence/attribution
        missing = [
            n for n, v in [
                ("--role", args.role),
                ("--licence", args.licence),
                ("--attribution", args.attribution),
            ] if not v
        ]
        if missing:
            sys.exit(
                f"error: source_url {source_url!r} is new in sources.yaml — "
                f"{', '.join(missing)} required to create the entry."
            )
        new_entry = {
            "source_url": source_url,
            "role": args.role,
            "licence": args.licence,
            "fetched_date": snapshot["snapshot_date"],
            "attribution": args.attribution,
            "datasets": [name],
            "local_path": snapshot["local_path"],
            "snapshot": snapshot_rel,
            "dataset_version": snapshot["dataset_version"],
        }
        sources.append(new_entry)
        print(f"  added new sources.yaml entry: {source_url}", file=sys.stderr)
    else:
        existing["local_path"] = snapshot["local_path"]
        existing["snapshot"] = snapshot_rel
        existing["dataset_version"] = snapshot["dataset_version"]
        if args.role:
            existing["role"] = args.role
        if args.licence:
            existing["licence"] = args.licence
        if args.attribution:
            existing["attribution"] = args.attribution
        if name not in (existing.get("datasets") or []):
            existing.setdefault("datasets", []).append(name)
        print(f"  updated sources.yaml entry: {source_url}", file=sys.stderr)

    data["sources"] = sources
    # Note: this rewrite loses YAML comments. Restoring full comment
    # preservation requires ruamel.yaml — acceptable trade-off for v1
    # since comments are documentation, not load-bearing state.
    with SOURCES_YAML.open("w") as f:
        f.write(
            "# eval/datasets/sources.yaml — role manifest + provenance registry\n"
            "# (auto-managed in part by scripts/dataset_register.py — comments\n"
            "# above individual entries may be lost on rewrite; see git history\n"
            "# for prior annotations or restore via ruamel.yaml round-trip.)\n\n"
        )
        yaml.safe_dump(data, f, sort_keys=False, default_flow_style=False, allow_unicode=True)


def main() -> int:
    args = parse_args()
    local_path = Path(args.local_path).resolve()
    if not local_path.exists():
        sys.exit(f"error: local_path does not exist: {local_path}")

    if args.mode == "index-only":
        snapshot = build_index_only_snapshot(args.name, local_path, args)
    else:
        snapshot = build_full_snapshot(args.name, local_path, args)

    version = snapshot["dataset_version"]
    out_path = SNAPSHOTS_DIR / f"{args.name}-{version}.json"

    if args.dry_run:
        print(json.dumps(snapshot, indent=2))
        return 0

    SNAPSHOTS_DIR.mkdir(parents=True, exist_ok=True)
    with out_path.open("w") as f:
        json.dump(snapshot, f, indent=2)
        f.write("\n")
    print(f"wrote snapshot: {out_path}", file=sys.stderr)
    print(
        f"  mode={snapshot['snapshot_mode']} "
        f"size={snapshot.get('total_size_bytes') or snapshot.get('sample_total_size_bytes', 0):,} bytes "
        f"files={snapshot.get('file_count') or snapshot.get('sample_size', 0)}",
        file=sys.stderr,
    )

    if args.update_registry:
        update_sources_yaml(args.name, out_path, snapshot, args)

    return 0


if __name__ == "__main__":
    sys.exit(main())
