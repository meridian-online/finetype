#!/usr/bin/env python3
"""Identity fortification — extract username/full_name training columns from the
corpus (spec 2026-06-18-identity-header-hint-fortification ac-01/ac-02).

Mines author/login-headed columns from the corpus pass and labels them by VALUE
SHAPE (training-time labelling; inference stays value-based per decision 0048):

  * HANDLE   (space-frac <= 0.15 AND distinct-frac >= 0.9)  -> identity.person.username
  * MULTITOKEN (space-frac >= 0.5)                          -> identity.person.full_name
  * ambiguous/mixed                                         -> excluded (8-sample noise)

The point is to pair the author/login HEADER with handle VALUES so the learned
multi-branch model carries author+handle -> username (today the hardcoded
author->full_name hint overrides it; ac-00 confirmed the learned model already
predicts username on handle VALUES alone, then it is suppressed).

MINIMAL FIRST COMPOSITION (ac-02 rationale): username positives + full_name only,
NO hand-labelled hard negatives. username is a generic-catch-all attractor, so the
drift proxy is the gate that decides whether it over-emits; if NO-GO, add targeted
hard negatives and re-proxy rather than guess noisy labels up front.

Leakage: MADR 0056 dedup — a column is dropped if ANY sample value, hashed with the
shared normaliser row_hash(column_name, value), collides with eval/row_hashes.tsv.

Output: <out-dir>/identity_hard_negatives.parquet
  file_path, column_name, sample_values (list<varchar>), correct_label, source_shape

No python duckdb/pyarrow needed — parquet I/O via the duckdb CLI.
Usage: scripts/extract_identity_hard_negatives.py [--columns PARQUET] [--out-dir DIR]
                                                   [--cap-username N] [--cap-fullname N]
"""
from __future__ import annotations

import argparse
import csv
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
_SCRIPTS = REPO / "scripts"
if str(_SCRIPTS) not in sys.path:
    sys.path.insert(0, str(_SCRIPTS))

from eval_leakage import row_hash  # noqa: E402  # pyright: ignore[reportMissingImports]

EVAL_ROW_HASHES = REPO / "eval" / "row_hashes.tsv"
DEFAULT_COLUMNS = REPO / "eval" / "gittables" / "corpus_pass" / "columns.parquet"
DEFAULT_OUT = REPO / "output" / "identity-fortification"
SAMPLE_DELIM = "│"  # the corpus sample_values_truncated separator
AUTHOR_HEADERS = (
    "author", "login", "username", "user", "screen_name", "created_by",
    "user_name", "handle", "nick", "nickname", "authors", "submitter", "owner",
)


def duckdb_csv(sql: str) -> str:
    r = subprocess.run(
        ["duckdb", "-noheader", "-csv", "-c", sql],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        print(f"duckdb error:\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    return r.stdout


def duckdb_exec(sql: str) -> None:
    r = subprocess.run(["duckdb", "-c", sql], capture_output=True, text=True)
    if r.returncode != 0:
        print(f"duckdb error:\n{r.stderr}", file=sys.stderr)
        sys.exit(1)


def load_eval_hashes() -> set[str]:
    hashes: set[str] = set()
    with EVAL_ROW_HASHES.open(encoding="utf-8") as fh:
        for line in fh:
            parts = line.rstrip("\n").split("\t")
            if len(parts) == 4 and parts[3] != "row_hash":
                hashes.add(parts[3])
    return hashes


def collides(column_name: str, values: list[str], eval_hashes: set[str]) -> bool:
    return any(v and row_hash(column_name, v) in eval_hashes for v in values)


def shape_of(values: list[str]) -> str | None:
    """HANDLE / FULLNAME / None(exclude) — same discriminator as the username veto."""
    ne = [v.strip() for v in values if v and v.strip()]
    if len(ne) < 4:
        return None
    with_space = sum(1 for v in ne if any(c.isspace() for c in v))
    space_frac = with_space / len(ne)
    distinct_frac = len(set(ne)) / len(ne)
    if space_frac <= 0.15 and distinct_frac >= 0.9:
        # also require handle charset on most values
        handle = sum(
            1 for v in ne
            if not any(c.isspace() for c in v)
            and any(c.isalpha() for c in v)
            and all(c.isalnum() or c in "._-" for c in v)
        )
        if handle / len(ne) >= 0.8:
            return "HANDLE"
        return None
    if space_frac >= 0.5:
        return "FULLNAME"
    return None


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--columns", type=Path, default=DEFAULT_COLUMNS)
    ap.add_argument("--out-dir", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--cap-username", type=int, default=1800)
    ap.add_argument("--cap-fullname", type=int, default=900)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    eval_hashes = load_eval_hashes()
    header_list = ", ".join(f"'{h}'" for h in AUTHOR_HEADERS)

    # Pull author/login-headed columns with their samples. Deterministic order
    # (file_path, column_name) so --seed-free reservoir is reproducible via LIMIT
    # after a stable sort + hash bucketing.
    rows = duckdb_csv(
        f"SELECT file_path, column_name, sample_values_truncated "
        f"FROM read_parquet('{args.columns.as_posix()}') "
        f"WHERE lower(column_name) IN ({header_list}) "
        f"  AND is_trivial = false AND sample_values_truncated IS NOT NULL "
        f"ORDER BY file_path, column_name"
    )

    kept: dict[str, list[tuple[str, str, list[str]]]] = {"HANDLE": [], "FULLNAME": []}
    n_total = n_collided = n_excluded = 0
    reader = csv.reader(rows.splitlines())
    for row in reader:
        if len(row) < 3:
            continue
        fp, col, raw = row[0], row[1], row[2]
        n_total += 1
        vals = [s for s in raw.split(SAMPLE_DELIM) if s.strip()]
        if not vals:
            n_excluded += 1
            continue
        shape = shape_of(vals)
        if shape is None:
            n_excluded += 1
            continue
        if collides(col, vals, eval_hashes):
            n_collided += 1
            continue
        kept[shape].append((fp, col, vals))

    label_for = {"HANDLE": "identity.person.username", "FULLNAME": "identity.person.full_name"}
    caps = {"HANDLE": args.cap_username, "FULLNAME": args.cap_fullname}

    # Cap by even stride (deterministic) to spread across the corpus, not first-N.
    out_csv = args.out_dir / "identity_hard_negatives.csv"
    n_emitted = {"HANDLE": 0, "FULLNAME": 0}
    with out_csv.open("w", newline="", encoding="utf-8") as fh:
        w = csv.writer(fh)
        w.writerow(["file_path", "column_name", "sample_values", "correct_label", "source_shape"])
        for shape, items in kept.items():
            cap = caps[shape]
            stride = max(1, len(items) // cap) if cap else 1
            for i, (fp, col, vals) in enumerate(items):
                if i % stride != 0 or n_emitted[shape] >= cap:
                    continue
                w.writerow([fp, col, SAMPLE_DELIM.join(vals), label_for[shape], shape])
                n_emitted[shape] += 1

    out_parquet = args.out_dir / "identity_hard_negatives.parquet"
    duckdb_exec(
        f"COPY (SELECT file_path, column_name, "
        f"  string_split(sample_values, '{SAMPLE_DELIM}') AS sample_values, "
        f"  correct_label, source_shape "
        f"  FROM read_csv('{out_csv.as_posix()}', header=true, all_varchar=true)) "
        f"TO '{out_parquet.as_posix()}' (FORMAT parquet)"
    )

    print(f"author-headed columns scanned: {n_total:,}")
    print(f"  excluded (ambiguous/scant): {n_excluded:,}")
    print(f"  leakage collisions dropped: {n_collided:,}")
    print(f"  available HANDLE->username: {len(kept['HANDLE']):,}  (emitted {n_emitted['HANDLE']:,})")
    print(f"  available MULTITOKEN->full_name: {len(kept['FULLNAME']):,}  (emitted {n_emitted['FULLNAME']:,})")
    print(f"wrote {out_parquet}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
