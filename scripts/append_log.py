"""Atomic append helpers for the autonomy-contract append-only logs.

Implements bead finetype-87j ac-01: log writers use atomic append
(flock + >> redirect) so concurrent cron firings cannot tear writes.
The lockfile mechanism in finetype-nms prevents concurrent cycles in
the first place; this module is the second line of defence — even if
two writers somehow run, neither corrupts the file.

Used by the cycle preamble + branch logic to write rows to:
  - eval/gittables/failure_log.tsv
  - eval/gittables/working_slice_coverage.tsv
  - eval/gittables/harvest_pool.tsv

For cycle_log.jsonl, the same primitives apply via JSON serialisation.

The atomic-append guarantee uses POSIX `O_APPEND`: kernel guarantees
each `write()` is positioned at end-of-file atomically. We additionally
take an `fcntl.flock()` exclusive lock around the entire entry to
serialise multi-line writes — though all current entries are single
lines, this is forward-protection if a future entry needs multiple
lines and is required to be atomic at the entry level.

Per ac-03, the integrity check script `log_integrity_check.sh` looks
for `chflags uchg` (macOS user-immutable flag) but applying that flag
is NOT done from this writer — the writer needs to append. The cycle
postamble drops the flag, the cycle does its writes, and then re-applies
on completion. This module is therefore agnostic to uschg state; if a
write fails because the file is uschg-locked, the failure is loud and
visible (OSError → cycle aborts via H02).

See:
    eval/gittables/SCHEMAS.md (column layouts)
    orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml §2
"""

from __future__ import annotations

import fcntl
import json
import os
from pathlib import Path
from typing import Mapping, Sequence


__all__ = [
    "append_tsv_row",
    "append_jsonl_record",
    "count_lines",
    "FAILURE_LOG_HEADER",
    "COVERAGE_LOG_HEADER",
    "HARVEST_POOL_HEADER",
]


FAILURE_LOG_HEADER: tuple[str, ...] = (
    "cycle_id",
    "timestamp",
    "file_path",
    "file_content_sha256",
    "column_name",
    "predicted_type",
    "observed_values_sample",
    "inferred_correct_type",
    "mechanism",
)

COVERAGE_LOG_HEADER: tuple[str, ...] = (
    "cycle_id",
    "timestamp",
    "file_path",
    "file_content_sha256",
    "outcome",
    "predicted_type_distribution_json",
)

HARVEST_POOL_HEADER: tuple[str, ...] = (
    "cycle_id",
    "timestamp",
    "file_path",
    "file_content_sha256",
    "column_name",
    "target_generator",
    "samples_json",
)


# A row's TSV-encoding rules:
#   - tab → space (cannot survive in a TSV cell)
#   - newline → space (cannot survive without breaking line semantics)
#   - carriage return → space
# We do not URL-encode; the loss of those three control chars is
# acceptable for the contract's purposes (failure observation samples).
_TSV_ESCAPE = str.maketrans({"\t": " ", "\n": " ", "\r": " "})


def _tsv_encode_cell(value: object) -> str:
    if value is None:
        return ""
    return str(value).translate(_TSV_ESCAPE)


def _open_appending(path: Path) -> int:
    """Open `path` with O_APPEND | O_WRONLY | O_CREAT, mode 0644."""
    path.parent.mkdir(parents=True, exist_ok=True)
    flags = os.O_WRONLY | os.O_APPEND | os.O_CREAT
    return os.open(str(path), flags, 0o644)


def _ensure_header(path: Path, header: Sequence[str]) -> None:
    """Write the TSV header once on first creation; no-op otherwise.

    Uses a separate fd from the append fd, so this runs before any
    row writes and only when the file is empty.
    """
    if path.exists() and path.stat().st_size > 0:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    # Race-tolerant: if a parallel writer creates + writes header
    # between our stat and write, our O_APPEND below means we'll just
    # add a duplicate row — but the lock below also covers this.
    flags = os.O_WRONLY | os.O_APPEND | os.O_CREAT
    fd = os.open(str(path), flags, 0o644)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        try:
            # Re-check under lock.
            if os.fstat(fd).st_size == 0:
                os.write(fd, ("\t".join(header) + "\n").encode("utf-8"))
        finally:
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)


def append_tsv_row(
    path: Path,
    row: Mapping[str, object] | Sequence[object],
    *,
    header: Sequence[str],
) -> None:
    """Atomically append a single TSV row.

    `row` may be a dict (fields by name; missing keys → empty cell) or
    a sequence (positional). Either way, `header` defines the column
    order written. The TSV header is written on first call (when the
    file is empty); subsequent calls only append the data row.
    """
    _ensure_header(path, header)

    if isinstance(row, Mapping):
        cells = [_tsv_encode_cell(row.get(col, "")) for col in header]
    else:
        if len(row) != len(header):
            raise ValueError(
                f"row length {len(row)} != header length {len(header)}"
            )
        cells = [_tsv_encode_cell(v) for v in row]
    payload = ("\t".join(cells) + "\n").encode("utf-8")

    fd = _open_appending(path)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        try:
            os.write(fd, payload)
        finally:
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)


def append_jsonl_record(path: Path, record: Mapping[str, object]) -> None:
    """Atomically append a single JSON object as one NDJSON line."""
    payload = (json.dumps(record, sort_keys=True) + "\n").encode("utf-8")
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = _open_appending(path)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX)
        try:
            os.write(fd, payload)
        finally:
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)


def count_lines(path: Path) -> int:
    """Count lines (header + data) under shared lock — safe vs. concurrent appender.

    Returns 0 if the file does not exist. Used by the cycle preamble
    to record `*_lines_before`, and by `log_integrity_check.sh` to
    detect H08/H09 corruption.
    """
    if not path.exists():
        return 0
    fd = os.open(str(path), os.O_RDONLY)
    try:
        fcntl.flock(fd, fcntl.LOCK_SH)
        try:
            n = 0
            with os.fdopen(os.dup(fd), "rb") as fh:
                for _ in fh:
                    n += 1
            return n
        finally:
            fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)
