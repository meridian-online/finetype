#!/usr/bin/env python3
"""emission_recheck — live emission count for a roadmap candidate label.

The certainty roadmap prioritises by emission count, but those counts come from
`eval/gittables/corpus_pass/columns.parquet`, a STALE Sense-stage snapshot that
predates current url/windows_path/text detection. Re-check a label's true live
emission before investing in a guard (the stale-scan trap that nearly built a
dead XML guard — see memory jwt-guard-shipped-and-stale-emission-lesson).

What it does: takes the (file, column) pairs the snapshot labelled <label> in
`sense_prediction`, re-profiles a deterministic sample of those files with the
CURRENT binary (full pipeline, post-Sharpen), and reports how many the binary
still calls <label> — plus where the rest went and a projected full-corpus live
count. Writes a provenance-stamped JSON (binary version, git rev, date) so a
cached result carries the binary it was measured against.

Usage:
  scripts/emission_recheck.py <label> [--sample N] [--binary PATH]
      [--model DIR] [--parquet PATH] [--out JSON] [--seed-order sha|path]

<label> is the full taxonomy label, e.g. representation.file.mime_type.
"""
import argparse
import csv
import io
import os
import subprocess
import sys
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_PARQUET = REPO / "eval/gittables/corpus_pass/columns.parquet"
DEFAULT_BINARY = REPO / "target/release/finetype"
DEFAULT_MODEL = REPO / "models/default"


def duckdb_csv(sql):
    """Run a query through the duckdb CLI, return rows as list[dict]."""
    out = subprocess.run(
        ["duckdb", "-csv", "-c", sql],
        capture_output=True, text=True, check=True,
    ).stdout
    # duckdb may prepend a ".duckdbrc loading" notice on stderr only; stdout is clean CSV
    return list(csv.DictReader(out.splitlines()))


def stale_rows(parquet, label):
    sql = (
        "SELECT file_path, file_content_sha256, column_name "
        f"FROM read_parquet('{parquet}') "
        f"WHERE sense_prediction = '{label}'"
    )
    return duckdb_csv(sql)


def provenance(binary, model):
    ver = subprocess.run([str(binary), "--version"], capture_output=True, text=True).stdout.strip()
    rev = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=REPO,
                         capture_output=True, text=True).stdout.strip()
    branch = subprocess.run(["git", "rev-parse", "--abbrev-ref", "HEAD"], cwd=REPO,
                            capture_output=True, text=True).stdout.strip()
    dirty = bool(subprocess.run(["git", "status", "--porcelain", "--", "crates", "labels"], cwd=REPO,
                                capture_output=True, text=True).stdout.strip())
    return {
        "binary_version": ver,
        "git_rev": rev,
        "git_branch": branch,
        "engine_tree_dirty": dirty,  # uncommitted changes under crates/ or labels/
        "model_dir": str(model),
        "checked_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("label", help="full taxonomy label, e.g. representation.file.mime_type")
    ap.add_argument("--sample", type=int, default=300, help="max files to re-profile (default 300)")
    ap.add_argument("--binary", default=str(DEFAULT_BINARY))
    ap.add_argument("--model", default=str(DEFAULT_MODEL))
    ap.add_argument("--parquet", default=str(DEFAULT_PARQUET))
    ap.add_argument("--seed-order", choices=["sha", "path"], default="sha",
                    help="deterministic sample ordering key (default sha)")
    ap.add_argument("--jobs", type=int, default=8, help="parallel profile workers (default 8)")
    ap.add_argument("--out", default=None, help="write provenance JSON here "
                    "(default output/certainty-roadmap/emission_recheck/<label>.json)")
    args = ap.parse_args()

    binary, model = Path(args.binary), Path(args.model)
    if not binary.exists():
        sys.exit(f"binary not found: {binary} (cargo build --release?)")

    rows = stale_rows(args.parquet, args.label)
    if not rows:
        sys.exit(f"no columns.parquet rows with sense_prediction = {args.label!r}")

    stale_cols = len(rows)
    # group stale columns by file; sample files deterministically (no RNG — reproducible)
    by_file = {}
    for r in rows:
        by_file.setdefault(r["file_path"], {"sha": r["file_content_sha256"], "cols": []})
        by_file[r["file_path"]]["cols"].append(r["column_name"])
    stale_files = len(by_file)
    key = (lambda fp: by_file[fp]["sha"]) if args.seed_order == "sha" else (lambda fp: fp)
    ordered = sorted(by_file, key=lambda fp: (key(fp), fp))
    sampled = [fp for fp in ordered if Path(fp).exists()][: args.sample]
    missing_on_disk = sum(1 for fp in ordered if not Path(fp).exists())
    if not sampled:
        sys.exit("no sampled source files exist on disk — cannot re-profile")

    # batch mode only wires json-schema/datapackage (both lossy — they drop the fine
    # taxonomy label), so re-profile each file singly with -o csv, which emits the real
    # post-Sharpen `type`. Model load is ~0.3s/file, so a small thread pool keeps it quick.
    env = {**os.environ, "FINETYPE_MODEL_DIR": str(model)}

    def profile_one(fp):
        """Return {column_name: live_label} for one file, or None on failure."""
        proc = subprocess.run(
            [str(binary), "profile", "-f", fp, "-o", "csv"],
            capture_output=True, text=True, env=env, cwd=str(REPO),
        )
        if proc.returncode != 0:
            return None
        # skip the leading log preamble; parse from the CSV header line onward
        lines = proc.stdout.splitlines()
        start = next((i for i, ln in enumerate(lines) if ln.startswith("column,type,")), None)
        if start is None:
            return None
        live = {}
        for rec in csv.DictReader(io.StringIO("\n".join(lines[start:]))):
            live[rec["column"]] = rec["type"]
        return live

    kept = 0
    dests = Counter()
    profiled_files = 0
    done = 0
    with ThreadPoolExecutor(max_workers=max(1, args.jobs)) as pool:
        for fp, live in zip(sampled, pool.map(profile_one, sampled)):
            done += 1
            if done % 50 == 0:
                sys.stderr.write(f"\r  profiled {done}/{len(sampled)} files")
                sys.stderr.flush()
            if live is None:
                dests["<profile-failed>"] += len(by_file[fp]["cols"])
                continue
            profiled_files += 1
            for col in by_file[fp]["cols"]:
                dest = live.get(col)
                if dest is None:
                    dests["<column-not-found>"] += 1
                elif dest == args.label:
                    kept += 1
                else:
                    dests[dest] += 1
    if done >= 50:
        sys.stderr.write("\r" + " " * 40 + "\r")

    sampled_cols = sum(len(by_file[fp]["cols"]) for fp in sampled)
    keep_rate = kept / sampled_cols if sampled_cols else 0.0
    projected_live = round(stale_cols * keep_rate)

    result = {
        "label": args.label,
        "stale_columns": stale_cols,
        "stale_files": stale_files,
        "sampled_files": len(sampled),
        "profiled_files": profiled_files,
        "missing_on_disk_files": missing_on_disk,
        "sampled_columns": sampled_cols,
        "live_kept_columns": kept,
        "keep_rate": round(keep_rate, 4),
        "projected_live_columns": projected_live,
        "top_destinations": dict(dests.most_common(12)),
        **provenance(binary, model),
    }

    out = Path(args.out) if args.out else (
        REPO / "output/certainty-roadmap/emission_recheck" / f"{args.label}.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    import json
    out.write_text(json.dumps(result, indent=2) + "\n")

    # human summary
    print(f"\n  emission_recheck: {args.label}")
    print(f"  binary {result['binary_version']} @ {result['git_rev']} ({result['git_branch']})"
          + ("  [ENGINE TREE DIRTY]" if result["engine_tree_dirty"] else ""))
    print(f"  stale snapshot : {stale_cols} columns across {stale_files} files")
    print(f"  re-profiled    : {sampled_cols} stale columns in {len(sampled)} sampled files"
          + (f"  ({missing_on_disk} files missing on disk)" if missing_on_disk else ""))
    print(f"  still {args.label.split('.')[-1]:<16}: {kept}  (keep rate {keep_rate:.1%})")
    print(f"  → projected live corpus emission: ~{projected_live}  (was {stale_cols} stale)")
    if dests:
        print("  demoted/relocated to:")
        for dst, n in dests.most_common(8):
            print(f"      {n:>5}  {dst}")
    try:
        shown = out.relative_to(REPO)
    except ValueError:
        shown = out
    print(f"  provenance JSON: {shown}\n")


if __name__ == "__main__":
    main()
