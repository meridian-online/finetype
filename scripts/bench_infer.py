#!/usr/bin/env python3
"""Latency benchmark for `finetype infer-type` (ac-05).

Per spec ac-05:
    Median latency <100ms per column on M1 over a 1000-column benchmark
    sampled from failure_log.tsv (calibrate half). Benchmark MUST invoke
    the same code path the cycle worker uses in production: one
    subprocess invocation per column for subprocess form, OR one socket
    round-trip per column for server form.

This script measures the SUBPROCESS form — one `finetype infer-type`
invocation per column. p50 and p95 are reported. If p50 ≥ 100ms,
the spec recommends escalating to a long-lived `finetype infer-server`
form (Unix socket) — record the breach + escalation in progress.md.

USAGE
    python scripts/bench_infer.py \
        --calibrate eval/gittables/failure_log.calibrate.tsv \
        --finetype-bin ./target/release/finetype \
        --n 1000
"""
from __future__ import annotations

import argparse
import json
import random
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def parse_observed_sample(s: str) -> list[str]:
    if not s:
        return []
    return [v for v in s.split("│") if v]


def sample_rows(path: Path, n: int, seed: int) -> list[tuple[str, str, list[str]]]:
    rng = random.Random(seed)
    rows: list[tuple[str, str, list[str]]] = []
    with open(path, encoding="utf-8") as fh:
        header = fh.readline().rstrip("\n").split("\t")
        col_idx = {c: i for i, c in enumerate(header)}
        for line in fh:
            cells = line.rstrip("\n").split("\t")
            if len(cells) < len(header):
                continue
            rows.append(
                (
                    cells[col_idx["column_name"]],
                    cells[col_idx["predicted_type"]],
                    parse_observed_sample(cells[col_idx["observed_values_sample"]]),
                )
            )
    rng.shuffle(rows)
    return rows[:n]


def time_one(finetype_bin: str, column_name: str, predicted: str, samples: list[str]) -> float:
    payload = json.dumps(
        {
            "column_name": column_name,
            "predicted_type": predicted,
            "samples": samples,
        }
    )
    t0 = time.perf_counter()
    res = subprocess.run(
        [finetype_bin, "infer-type"],
        input=payload,
        capture_output=True,
        text=True,
        timeout=30,
    )
    elapsed_ms = (time.perf_counter() - t0) * 1000.0
    if res.returncode != 0:
        raise RuntimeError(f"finetype infer-type failed: {res.stderr[:200]}")
    return elapsed_ms


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument(
        "--calibrate",
        type=Path,
        default=REPO_ROOT / "eval" / "gittables" / "failure_log.calibrate.tsv",
    )
    p.add_argument(
        "--finetype-bin",
        default=str(REPO_ROOT / "target" / "release" / "finetype"),
    )
    p.add_argument("--n", type=int, default=1000)
    p.add_argument("--seed", type=int, default=42)
    args = p.parse_args()

    if not Path(args.finetype_bin).exists():
        alt = REPO_ROOT / "target" / "debug" / "finetype"
        if alt.exists():
            args.finetype_bin = str(alt)
            print(
                f"warning: release binary missing; using debug at {alt}",
                file=sys.stderr,
            )
        else:
            print(
                f"finetype binary not found at {args.finetype_bin}; build first",
                file=sys.stderr,
            )
            return 2

    rows = sample_rows(args.calibrate, args.n, args.seed)
    print(f"benchmarking {len(rows)} rows from {args.calibrate}", file=sys.stderr)

    times_ms: list[float] = []
    for i, (col, pred, samples) in enumerate(rows, 1):
        try:
            ms = time_one(args.finetype_bin, col, pred, samples)
        except Exception as exc:
            print(f"  ! row {i}: {exc}", file=sys.stderr)
            continue
        times_ms.append(ms)
        if i % 100 == 0:
            print(
                f"  ... {i}/{len(rows)} (running median ~{sorted(times_ms)[len(times_ms)//2]:.1f}ms)",
                file=sys.stderr,
            )

    if not times_ms:
        print("no successful runs", file=sys.stderr)
        return 1

    times_ms.sort()
    n = len(times_ms)
    p50 = times_ms[n // 2]
    p95 = times_ms[int(n * 0.95)]
    p99 = times_ms[int(n * 0.99)]
    summary = {
        "form": "subprocess-per-column",
        "binary": args.finetype_bin,
        "n_runs": n,
        "p50_ms": round(p50, 2),
        "p95_ms": round(p95, 2),
        "p99_ms": round(p99, 2),
        "min_ms": round(min(times_ms), 2),
        "max_ms": round(max(times_ms), 2),
        "mean_ms": round(sum(times_ms) / n, 2),
        "ac05_target_ms": 100,
        "ac05_pass": p50 < 100,
    }
    print(json.dumps(summary, indent=2))
    return 0 if summary["ac05_pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
