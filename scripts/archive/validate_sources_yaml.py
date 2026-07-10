#!/usr/bin/env python3
"""Validate eval/datasets/sources.yaml — structural check on every entry.

Per spec 2026-05-24-dataset-provenance-registry ac-01 close evidence.

Exits 0 on clean, 1 on schema violations, 2 on missing snapshot files.
Designed for CI / pre-commit use.
"""
from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SOURCES_YAML = REPO / "eval" / "datasets" / "sources.yaml"

REQUIRED_FIELDS = {"source_url", "role"}
RECOMMENDED_FIELDS = {"licence", "fetched_date", "attribution", "datasets"}
INTEGRITY_FIELDS = {"local_path", "snapshot", "dataset_version"}
ALLOWED_ROLES = {"train", "eval", "validate", "both-forbidden"}


def main() -> int:
    try:
        import yaml
    except ImportError as exc:
        print(f"error: pyyaml required ({exc})", file=sys.stderr)
        return 1
    if not SOURCES_YAML.exists():
        print(f"error: sources.yaml not found at {SOURCES_YAML}", file=sys.stderr)
        return 1
    with SOURCES_YAML.open() as f:
        try:
            data = yaml.safe_load(f) or {}
        except yaml.YAMLError as exc:
            print(f"error: YAML parse failed: {exc}", file=sys.stderr)
            return 1

    if not isinstance(data, dict) or "sources" not in data:
        print("error: top-level must be a mapping with a 'sources' key", file=sys.stderr)
        return 1
    sources = data["sources"]
    if not isinstance(sources, list):
        print("error: 'sources' must be a list", file=sys.stderr)
        return 1

    errors: list[str] = []
    warnings: list[str] = []
    missing_snapshots: list[str] = []
    seen_urls: set[str] = set()

    for i, entry in enumerate(sources):
        prefix = f"sources[{i}]"
        if not isinstance(entry, dict):
            errors.append(f"{prefix}: must be a mapping, got {type(entry).__name__}")
            continue
        # Required
        for field in REQUIRED_FIELDS:
            if field not in entry:
                errors.append(f"{prefix}: missing required field {field!r}")
        # Role value
        role = entry.get("role")
        if role is not None and role not in ALLOWED_ROLES:
            errors.append(f"{prefix}: role={role!r} not in {sorted(ALLOWED_ROLES)}")
        # Unique source_url
        url = entry.get("source_url")
        if url:
            if url in seen_urls:
                errors.append(f"{prefix}: duplicate source_url {url!r}")
            seen_urls.add(url)
        # Recommended (warnings only)
        for field in RECOMMENDED_FIELDS:
            if field not in entry:
                warnings.append(f"{prefix} ({url}): missing recommended field {field!r}")
        # Integrity — if any of the three are present, the snapshot file must exist
        integrity_present = INTEGRITY_FIELDS & set(entry.keys())
        if integrity_present:
            snap_rel = entry.get("snapshot")
            if not snap_rel:
                errors.append(
                    f"{prefix} ({url}): has integrity field(s) {sorted(integrity_present)} "
                    "but missing 'snapshot' pointer"
                )
            else:
                snap_path = REPO / snap_rel
                if not snap_path.exists():
                    missing_snapshots.append(f"{prefix} ({url}): snapshot file missing: {snap_rel}")

    # Report
    for w in warnings:
        print(f"WARN  {w}", file=sys.stderr)
    for e in errors:
        print(f"ERROR {e}", file=sys.stderr)
    for m in missing_snapshots:
        print(f"ERROR {m}", file=sys.stderr)

    n_with_integrity = sum(
        1 for e in sources if isinstance(e, dict) and (INTEGRITY_FIELDS & set(e.keys()))
    )
    print(
        f"sources.yaml: {len(sources)} entries, {n_with_integrity} with integrity fields, "
        f"{len(warnings)} warnings, {len(errors)} errors, "
        f"{len(missing_snapshots)} missing snapshot files",
        file=sys.stderr,
    )

    if errors:
        return 1
    if missing_snapshots:
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
