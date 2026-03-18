#!/usr/bin/env python3
"""Download GitTables and SOTAB datasets to ~/datasets/ with resume support.

Uses aria2c for resilient downloads with automatic resume. Verifies MD5
checksums after download. Fully idempotent — skips already-verified files.

Usage:
    python3 scripts/download_datasets.py                    # Download all
    python3 scripts/download_datasets.py --dataset gittables  # GitTables only
    python3 scripts/download_datasets.py --dataset sotab       # SOTAB only
    python3 scripts/download_datasets.py --topic steerageway   # Single GitTables topic
    python3 scripts/download_datasets.py --list-topics         # List available topics
    python3 scripts/download_datasets.py --no-extract          # Download only, skip unzip
    python3 scripts/download_datasets.py --dry-run             # Show what would be downloaded

Requires: aria2c, python3 (no pip dependencies)

Directory layout after download:
    ~/datasets/
        gittables/
            downloads/          # Raw ZIP archives
            parquet/            # Extracted parquet files (by topic)
        sotab/
            downloads/          # Raw ZIP archive
            cta/                # Extracted SOTAB V2 data
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path

# --- Constants ---

DATASETS_DIR = Path.home() / "datasets"

ZENODO_API = "https://zenodo.org/api/records"
ZENODO_FILES = "https://zenodo.org/records"  # Direct file download (no API auth needed)
GITTABLES_RECORD = "6517052"  # Parquet tables with annotations
SOTAB_RECORD = "8422037"  # SOTAB V2 for SemTab 2023

GITTABLES_DIR = DATASETS_DIR / "gittables"
GITTABLES_DOWNLOADS = GITTABLES_DIR / "downloads"
GITTABLES_PARQUET = GITTABLES_DIR / "parquet"

SOTAB_DIR = DATASETS_DIR / "sotab"
SOTAB_DOWNLOADS = SOTAB_DIR / "downloads"
SOTAB_EXTRACT = SOTAB_DIR / "cta"

# aria2c settings for resilience
ARIA2C_OPTS = [
    "--continue=true",  # Resume partial downloads
    "--max-connection-per-server=4",  # Parallel connections per file
    "--split=4",  # Split file into N parts
    "--min-split-size=10M",  # Minimum chunk size
    "--retry-wait=5",  # Wait 5s between retries
    "--max-tries=10",  # Retry up to 10 times
    "--connect-timeout=30",
    "--timeout=120",
    "--auto-file-renaming=false",
    "--allow-overwrite=false",
    "--console-log-level=warn",
    "--summary-interval=30",
    "--user-agent=finetype-dataset-downloader/1.0",
]


def fetch_manifest(record_id: str) -> list[dict]:
    """Fetch file manifest from Zenodo API.

    Uses the API endpoint for metadata but constructs direct download URLs
    (https://zenodo.org/records/{id}/files/{name}) which don't require
    authentication and work reliably with aria2c.
    """
    url = f"{ZENODO_API}/{record_id}"
    print(f"  Fetching manifest from {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "finetype-dataset-downloader/1.0"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        data = json.loads(resp.read())
    files = []
    for f in data["files"]:
        checksum = f["checksum"]
        algo, digest = checksum.split(":", 1) if ":" in checksum else ("md5", checksum)
        # Use direct download URL — the API /content endpoint returns 403 with aria2c
        download_url = f"{ZENODO_FILES}/{record_id}/files/{urllib.parse.quote(f['key'])}"
        files.append(
            {
                "key": f["key"],
                "size": f["size"],
                "url": download_url,
                "checksum_algo": algo,
                "checksum": digest,
            }
        )
    return files


def verify_checksum(path: Path, expected: str, algo: str = "md5") -> bool:
    """Verify file checksum. Returns True if match."""
    if not path.exists():
        return False
    h = hashlib.new(algo)
    with open(path, "rb") as f:
        while True:
            chunk = f.read(1 << 20)  # 1MB chunks
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest() == expected


def human_size(size_bytes: int) -> str:
    """Format bytes as human-readable string."""
    size = float(size_bytes)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if abs(size) < 1024:
            return f"{size:.1f} {unit}"
        size /= 1024
    return f"{size:.1f} PB"


def download_file(url: str, dest: Path) -> bool:
    """Download a file using aria2c with resume support. Returns True on success."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    cmd = ["aria2c"] + ARIA2C_OPTS + [f"--dir={dest.parent}", f"--out={dest.name}", url]
    result = subprocess.run(cmd, capture_output=False)
    return result.returncode == 0


def extract_zip(zip_path: Path, dest_dir: Path) -> bool:
    """Extract a ZIP archive. Skips if dest_dir already has files."""
    if not zip_path.exists():
        print(f"  ✗ ZIP not found: {zip_path}")
        return False

    # Check for empty/tiny zips (some GitTables topics are empty)
    if zip_path.stat().st_size < 100:
        print(f"  ⊘ Skipping empty archive: {zip_path.name}")
        return True

    dest_dir.mkdir(parents=True, exist_ok=True)

    try:
        with zipfile.ZipFile(zip_path, "r") as zf:
            members = zf.namelist()
            if not members:
                print(f"  ⊘ Empty archive: {zip_path.name}")
                return True

            # Check if already extracted (at least some files present)
            existing = set(os.listdir(dest_dir)) if dest_dir.exists() else set()
            # For GitTables: files inside the zip are parquet files
            # For SOTAB: nested directory structure
            top_level = {m.split("/")[0] for m in members if m and not m.endswith("/")}
            if top_level and top_level.issubset(existing):
                print(f"  ✓ Already extracted: {zip_path.name}")
                return True

            print(f"  Extracting {zip_path.name} ({len(members)} entries)...")
            zf.extractall(dest_dir)
            print(f"  ✓ Extracted to {dest_dir}")
            return True
    except zipfile.BadZipFile:
        print(f"  ✗ Bad ZIP file: {zip_path.name} — delete and re-download")
        return False


def process_file(
    file_info: dict, download_dir: Path, extract_dir: Path | None, dry_run: bool
) -> dict:
    """Download and optionally extract a single file. Returns status dict."""
    key = file_info["key"]
    dest = download_dir / key
    status = {"key": key, "size": file_info["size"], "action": "unknown"}

    # Check if already downloaded and verified
    if dest.exists() and dest.stat().st_size == file_info["size"]:
        if verify_checksum(dest, file_info["checksum"], file_info["checksum_algo"]):
            status["action"] = "verified"
            if not dry_run:
                print(f"  ✓ {key} ({human_size(file_info['size'])}) — verified")
            # Still try extraction if needed
            if extract_dir and not dry_run:
                extract_zip(dest, extract_dir)
            return status

    status["action"] = "download"
    if dry_run:
        return status

    # Download
    print(f"  ↓ Downloading {key} ({human_size(file_info['size'])})...")
    if not download_file(file_info["url"], dest):
        print(f"  ✗ Download failed: {key}")
        status["action"] = "failed"
        return status

    # Verify
    if not verify_checksum(dest, file_info["checksum"], file_info["checksum_algo"]):
        print(f"  ✗ Checksum mismatch: {key} — deleting corrupt file")
        dest.unlink(missing_ok=True)
        status["action"] = "checksum_failed"
        return status

    print(f"  ✓ {key} — verified")
    status["action"] = "downloaded"

    # Extract if requested
    if extract_dir:
        extract_zip(dest, extract_dir)

    return status


def download_gittables(
    topics: list[str] | None = None, extract: bool = True, dry_run: bool = False
) -> list[dict]:
    """Download GitTables parquet archives from Zenodo."""
    print("\n═══ GitTables 1M (Parquet) ═══")
    manifest = fetch_manifest(GITTABLES_RECORD)

    # Filter by topic if specified
    if topics:
        topic_keys = {f"{t}_tables_licensed.zip" for t in topics}
        manifest = [f for f in manifest if f["key"] in topic_keys]
        missing = topic_keys - {f["key"] for f in manifest}
        if missing:
            print(f"  ⚠ Topics not found: {', '.join(t.replace('_tables_licensed.zip', '') for t in missing)}")

    # Sort by size (download smaller files first for quick progress)
    manifest.sort(key=lambda f: f["size"])

    total_size = sum(f["size"] for f in manifest)
    print(f"  {len(manifest)} files, {human_size(total_size)} total")

    if dry_run:
        needs_download = []
        for f in manifest:
            dest = GITTABLES_DOWNLOADS / f["key"]
            if not (dest.exists() and dest.stat().st_size == f["size"]):
                needs_download.append(f)
        dl_size = sum(f["size"] for f in needs_download)
        print(f"  → Would download {len(needs_download)} files ({human_size(dl_size)})")
        print(f"  → Would skip {len(manifest) - len(needs_download)} already-verified files")
        return [{"key": f["key"], "action": "would_download"} for f in needs_download]

    results = []
    downloaded = 0
    skipped = 0
    failed = 0

    for i, f in enumerate(manifest, 1):
        topic = f["key"].replace("_tables_licensed.zip", "")
        extract_dest = (GITTABLES_PARQUET / topic) if extract else None
        print(f"\n[{i}/{len(manifest)}] {topic}")
        result = process_file(f, GITTABLES_DOWNLOADS, extract_dest, dry_run)
        results.append(result)

        if result["action"] in ("downloaded",):
            downloaded += 1
        elif result["action"] == "verified":
            skipped += 1
        elif result["action"] in ("failed", "checksum_failed"):
            failed += 1

    print(f"\n  Summary: {downloaded} downloaded, {skipped} verified, {failed} failed")
    return results


def download_sotab(extract: bool = True, dry_run: bool = False) -> list[dict]:
    """Download SOTAB V2 from Zenodo."""
    print("\n═══ SOTAB V2 for SemTab 2023 ═══")
    manifest = fetch_manifest(SOTAB_RECORD)

    total_size = sum(f["size"] for f in manifest)
    print(f"  {len(manifest)} files, {human_size(total_size)} total")

    if dry_run:
        for f in manifest:
            dest = SOTAB_DOWNLOADS / f["key"]
            exists = dest.exists() and dest.stat().st_size == f["size"]
            print(f"  {'✓ Have' if exists else '→ Need'}: {f['key']} ({human_size(f['size'])})")
        return [{"key": f["key"], "action": "dry_run"} for f in manifest]

    results = []
    for f in manifest:
        extract_dest = SOTAB_EXTRACT if extract else None
        result = process_file(f, SOTAB_DOWNLOADS, extract_dest, dry_run)
        results.append(result)

    return results


def list_topics():
    """Fetch and print all available GitTables topics."""
    manifest = fetch_manifest(GITTABLES_RECORD)
    topics = []
    for f in manifest:
        topic = f["key"].replace("_tables_licensed.zip", "")
        topics.append((topic, f["size"]))
    topics.sort(key=lambda t: t[0])
    print(f"\nGitTables topics ({len(topics)}):\n")
    for topic, size in topics:
        print(f"  {topic:40s} {human_size(size):>10s}")
    total = sum(s for _, s in topics)
    print(f"\n  {'Total':40s} {human_size(total):>10s}")


def main():
    parser = argparse.ArgumentParser(
        description="Download GitTables and SOTAB datasets with resume support.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__,
    )
    parser.add_argument(
        "--dataset",
        choices=["gittables", "sotab", "all"],
        default="all",
        help="Which dataset to download (default: all)",
    )
    parser.add_argument(
        "--topic",
        action="append",
        help="Download specific GitTables topic(s). Can be repeated.",
    )
    parser.add_argument(
        "--list-topics",
        action="store_true",
        help="List all available GitTables topics and exit.",
    )
    parser.add_argument(
        "--no-extract",
        action="store_true",
        help="Download only, don't extract ZIP archives.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be downloaded without downloading.",
    )
    parser.add_argument(
        "--dest",
        type=Path,
        default=DATASETS_DIR,
        help=f"Base directory for datasets (default: {DATASETS_DIR})",
    )
    args = parser.parse_args()

    # Update paths if custom dest
    global GITTABLES_DIR, GITTABLES_DOWNLOADS, GITTABLES_PARQUET
    global SOTAB_DIR, SOTAB_DOWNLOADS, SOTAB_EXTRACT
    if args.dest != DATASETS_DIR:
        base = args.dest
        GITTABLES_DIR = base / "gittables"
        GITTABLES_DOWNLOADS = GITTABLES_DIR / "downloads"
        GITTABLES_PARQUET = GITTABLES_DIR / "parquet"
        SOTAB_DIR = base / "sotab"
        SOTAB_DOWNLOADS = SOTAB_DIR / "downloads"
        SOTAB_EXTRACT = SOTAB_DIR / "cta"

    # Check aria2c
    if not shutil.which("aria2c"):
        print("Error: aria2c not found. Install with: brew install aria2", file=sys.stderr)
        sys.exit(1)

    if args.list_topics:
        list_topics()
        return

    extract = not args.no_extract

    print(f"Datasets directory: {args.dest}")
    if args.dry_run:
        print("DRY RUN — no files will be downloaded\n")

    if args.dataset in ("gittables", "all"):
        download_gittables(topics=args.topic, extract=extract, dry_run=args.dry_run)

    if args.dataset in ("sotab", "all") and not args.topic:
        download_sotab(extract=extract, dry_run=args.dry_run)

    print("\n✓ Done.")


if __name__ == "__main__":
    main()
