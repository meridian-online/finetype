#!/usr/bin/env python3
"""Verify a registered dataset's integrity against its snapshot.

Per spec 2026-05-24-dataset-provenance-registry ac-03.

Reads eval/datasets/sources.yaml to locate the snapshot for each named
dataset, then re-hashes every file under local_path (or the index-file
sample for index-only snapshots), and reports any drift.

Usage
-----
  # Verify one or more datasets — exits non-zero on any drift
  scripts/dataset_verify.py sherlock cldr gittables

  # Verify everything in sources.yaml that has a snapshot field
  scripts/dataset_verify.py --all

  # Allow extra files on disk that aren't in the snapshot
  # (useful when iterating; default treats extras as drift)
  scripts/dataset_verify.py sherlock --ignore-extra

Exit codes
----------
  0  every named dataset matches its snapshot
  2  drift detected — at least one missing, extra, or hash-mismatched file
  3  configuration error — snapshot/dataset not found, sources.yaml malformed
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCES_YAML = REPO / "eval" / "datasets" / "sources.yaml"

IGNORE_NAMES = {".DS_Store", "Thumbs.db", "._.DS_Store", ".gitkeep"}
IGNORE_SUFFIXES = (".tmp", ".swp", ".swo", ".pyc")


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("names", nargs="*", help="Dataset names to verify (sources.yaml `datasets` list match).")
    p.add_argument("--all", action="store_true", help="Verify every entry in sources.yaml that has a `snapshot` field.")
    p.add_argument("--ignore-extra", action="store_true", help="Don't treat on-disk files missing from the snapshot as drift.")
    return p.parse_args()


def sha256_file(path: Path) -> tuple[int, str]:
    h = hashlib.sha256()
    size = 0
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1048576), b""):
            h.update(chunk)
            size += len(chunk)
    return size, h.hexdigest()


def resolve_local_path(record: str) -> Path:
    if record.startswith("repo://"):
        return REPO / record[len("repo://"):]
    return Path(record)


def load_entries(names: list[str], all_flag: bool) -> list[dict]:
    try:
        import yaml
    except ImportError as exc:
        print(f"error: pyyaml required ({exc})", file=sys.stderr)
        sys.exit(3)
    if not SOURCES_YAML.exists():
        print(f"error: sources.yaml not found at {SOURCES_YAML}", file=sys.stderr)
        sys.exit(3)
    with SOURCES_YAML.open() as f:
        data = yaml.safe_load(f) or {}
    sources = data.get("sources") or []

    out: list[dict] = []
    if all_flag:
        for entry in sources:
            if entry.get("snapshot"):
                out.append(entry)
        if not out:
            print("warning: --all but no entries have a snapshot field yet", file=sys.stderr)
        return out

    for name in names:
        matches = [e for e in sources if name in (e.get("datasets") or []) and e.get("snapshot")]
        if not matches:
            print(f"error: no sources.yaml entry for dataset {name!r} with a snapshot field", file=sys.stderr)
            sys.exit(3)
        out.extend(matches)
    return out


def should_skip(path: Path) -> bool:
    if path.name in IGNORE_NAMES:
        return True
    if any(path.name.endswith(s) for s in IGNORE_SUFFIXES):
        return True
    return False


def verify_full(snapshot: dict, ignore_extra: bool) -> tuple[int, list[str]]:
    """Returns (drift_count, lines)."""
    lines: list[str] = []
    local_path = resolve_local_path(snapshot["local_path"])
    if not local_path.exists():
        return 1, [f"  MISSING_ROOT {local_path}"]

    expected = snapshot.get("files") or {}
    seen: set[str] = set()
    drift = 0

    if local_path.is_file():
        # Single-file snapshots store the file under its basename.
        key = local_path.name
        if key not in expected:
            return 1, [f"  EXTRA file on disk: {key}"]
        size, sha = sha256_file(local_path)
        rec = expected[key]
        if rec["sha256"] != sha or rec["size_bytes"] != size:
            drift += 1
            lines.append(f"  DRIFT {key} (size {rec['size_bytes']}→{size}, sha {rec['sha256'][:12]}…→{sha[:12]}…)")
        seen.add(key)
    else:
        for sub in sorted(local_path.rglob("*")):
            if not sub.is_file() or should_skip(sub):
                continue
            rel = str(sub.relative_to(local_path))
            if rel not in expected:
                if not ignore_extra:
                    drift += 1
                    lines.append(f"  EXTRA {rel}")
                continue
            size, sha = sha256_file(sub)
            rec = expected[rel]
            if rec["sha256"] != sha or rec["size_bytes"] != size:
                drift += 1
                lines.append(f"  DRIFT {rel} (size {rec['size_bytes']}→{size}, sha {rec['sha256'][:12]}…→{sha[:12]}…)")
            seen.add(rel)

    missing = set(expected.keys()) - seen
    for m in sorted(missing):
        drift += 1
        lines.append(f"  MISSING {m}")
    return drift, lines


def verify_index_only(snapshot: dict) -> tuple[int, list[str]]:
    lines: list[str] = []
    drift = 0

    # Index file integrity
    index_path = resolve_local_path(snapshot["index_file"])
    if not index_path.exists():
        return 1, [f"  MISSING_INDEX {index_path}"]
    idx_size, idx_sha = sha256_file(index_path)
    if idx_sha != snapshot["index_file_sha256"] or idx_size != snapshot["index_file_size_bytes"]:
        drift += 1
        lines.append(
            f"  DRIFT index file {index_path.name} "
            f"(size {snapshot['index_file_size_bytes']}→{idx_size}, "
            f"sha {snapshot['index_file_sha256'][:12]}…→{idx_sha[:12]}…)"
        )

    # Re-hash sampled files
    local_path = resolve_local_path(snapshot["local_path"])
    sample = snapshot.get("sample_files") or {}
    for rel, rec in sample.items():
        full = Path(rel) if Path(rel).is_absolute() else (local_path / rel)
        if not full.exists():
            drift += 1
            lines.append(f"  MISSING sample file {rel}")
            continue
        if "error" in rec:
            # Recorded as missing at register time; consistent
            continue
        size, sha = sha256_file(full)
        if rec["sha256"] != sha or rec["size_bytes"] != size:
            drift += 1
            lines.append(
                f"  DRIFT sample {rel} "
                f"(size {rec['size_bytes']}→{size}, sha {rec['sha256'][:12]}…→{sha[:12]}…)"
            )
    return drift, lines


def main() -> int:
    args = parse_args()
    if not args.names and not args.all:
        print("error: pass one or more dataset names or --all", file=sys.stderr)
        return 3

    entries = load_entries(args.names, args.all)
    total_drift = 0
    for entry in entries:
        snapshot_path = REPO / entry["snapshot"]
        if not snapshot_path.exists():
            print(f"error: snapshot file missing: {snapshot_path}", file=sys.stderr)
            return 3
        with snapshot_path.open() as f:
            snapshot = json.load(f)

        name = ", ".join(entry.get("datasets") or ["?"])
        mode = snapshot.get("snapshot_mode", "full")
        print(f"verify {name} (mode={mode}, snapshot={entry['snapshot']})", file=sys.stderr)

        if mode == "index-only":
            drift, lines = verify_index_only(snapshot)
        else:
            drift, lines = verify_full(snapshot, args.ignore_extra)
        for ln in lines:
            print(ln)
        if drift == 0:
            print(f"  OK ({snapshot.get('file_count') or snapshot.get('sample_size', 0)} files)", file=sys.stderr)
        else:
            print(f"  DRIFT: {drift} issue(s)", file=sys.stderr)
        total_drift += drift

    return 0 if total_drift == 0 else 2


if __name__ == "__main__":
    sys.exit(main())
