#!/usr/bin/env python3
"""What the corpus says about the day-first compact-date leaf.

Answers the three questions a `datetime.date.compact_dmy` validator change has
to answer before it can claim to be safe, all from a corpus profile pass:

  1. SIZE       — how many columns type `datetime.date.compact_dmy`, and what
                  are they really (header-name census).
  2. BITE       — under candidate DAY/MONTH windows, what fraction of each
                  column's sampled values still validate, and how many columns
                  fall below the 0.5 hard-veto threshold. Three patterns are
                  scored side by side so the YEAR POLICY is decided on measured
                  cost, not on taste.
  3. LEGITIMACY — how much of the corpus is genuinely day-first. DD-MM-YYYY is
                  the ordering most of the world writes, so an over-tight rule
                  on this leaf could be worse than the disease. Two populations
                  are counted: compact day-first columns (this leaf's own real
                  family) and SEPARATOR-BEARING day-first columns (dmy_slash,
                  dmy_dot, …). The second is the control: if the corpus were
                  US-only, both would be empty, and any claim about the first
                  would mean nothing.

Input is a corpus-pass `columns.parquet` (from `gittables_corpus_pass.py
--execute`), whose `sample_values_truncated` field holds up to 8 sampled values
per column joined by U+2502. That 8-value window is a LOWER-FIDELITY proxy for
what the profile path sees (it samples 100), so the pass rates here estimate the
veto's input rather than reproduce its arithmetic. `compact_dmy_blast_radius.sh`
is the authoritative label-delta instrument — a real two-sided profile pass;
this script explains the mechanism behind its numbers.

Emits counts, rates and HEADER NAMES only — never raw corpus cell values
(secret-scanner risk on gittables sample strings).

The report lands in `docs/`, not `output/`: `output/` is blanket-gitignored as
derived experiment scratch, and a number nobody else can open is not evidence.

Run:
    ./eval/gittables/.venv/bin/python scripts/compact_dmy_corpus_family.py \
        --pass output/compact-ymd-gate/full/cand_pass/corpus_pass/columns.parquet \
        --out docs/compact-dmy-corpus-family.json
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

SEP = "│"

# Candidate patterns for the day-first leaf, scored side by side.
#
#   shape_only  — what ships today. Confirms every eight-digit token.
#   candidate   — day 01-31, month 01-12, year `\d{4}`. THE ONE THIS PR SHIPS.
#   year_1000_2099 — NOT SHIPPED. Measured only so the year policy is a
#                 decision with a price on it. `compact_ymd`'s first revision
#                 used `(19|20)\d{2}` and a reviewer refuted it with genuine
#                 1865-1872 dates; this variant is the widest year window that
#                 would still reject the residual (a surrogate key whose low
#                 digits read as an implausible year), and its cost is reported
#                 here rather than assumed.
PATTERNS: dict[str, re.Pattern[str]] = {
    "shape_only": re.compile(r"^\d{8}$"),
    "candidate": re.compile(r"^(0[1-9]|[12]\d|3[01])(0[1-9]|1[0-2])\d{4}$"),
    "year_1000_2099": re.compile(
        r"^(0[1-9]|[12]\d|3[01])(0[1-9]|1[0-2])(1\d|20)\d{2}$"),
}

# The canonical pass-rate veto threshold (finetype_core::VETO_THRESHOLD).
VETO_THRESHOLD = 0.5

DMY = "datetime.date.compact_dmy"

# Day-first leaves that carry a SEPARATOR. The control population for the
# legitimacy question: these are day-first dates the corpus demonstrably holds.
DAY_FIRST_SEPARATOR_PREFIX = "datetime.date.dmy"


def split_samples(raw: str | None) -> list[str]:
    if not raw:
        return []
    return [v for v in raw.split(SEP) if v]


def pass_rate(values: list[str], pattern: re.Pattern[str]) -> float | None:
    """Fraction of values matching `pattern`. None when there is nothing to
    check — mirrors `evaluate_validation_veto`'s inert case."""
    if not values:
        return None
    hits = sum(1 for v in values if pattern.fullmatch(v) is not None)
    return hits / len(values)


def reads_as_day_first(values: list[str]) -> bool:
    """True when every sampled value parses as DD-MM-YYYY and at least one
    leading pair is 13-31 — a pair no month can be, so the month-first reading
    is excluded.

    This is the widest machine-checkable net for "genuine day-first" available
    without hand labels, and it is deliberately WIDE: `20061209` is a
    YYYY-MM-DD date that also reads as 20/06/1209, and it is counted here. A
    column passing this test is therefore a CANDIDATE day-first column, not a
    confirmed one — read the accompanying header names before calling any of
    them real.
    """
    if not values:
        return False
    saw_discriminating = False
    for v in values:
        if PATTERNS["candidate"].fullmatch(v) is None:
            return False
        if 13 <= int(v[:2]) <= 31:
            saw_discriminating = True
    return saw_discriminating


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--pass", dest="pass_parquet", required=True,
                    help="corpus-pass columns.parquet")
    ap.add_argument("--out", required=True, help="JSON report path")
    ap.add_argument("--top-headers", type=int, default=25)
    args = ap.parse_args(argv)

    try:
        import duckdb
    except ImportError:
        print("FAIL: duckdb not importable — run under "
              "./eval/gittables/.venv/bin/python", file=sys.stderr)
        return 2

    con = duckdb.connect()
    rows = con.execute(
        "SELECT column_name, sense_prediction, sample_values_truncated "
        "FROM read_parquet(?)", [args.pass_parquet]
    ).fetchall()

    total_columns = len(rows)
    family: Counter[str] = Counter()
    day_first_separator: Counter[str] = Counter()
    dmy_headers: Counter[str] = Counter()
    dmy_unmeasurable = 0

    # Per-pattern accumulators over the compact_dmy columns.
    rates: dict[str, list[float]] = {k: [] for k in PATTERNS}
    below: dict[str, int] = {k: 0 for k in PATTERNS}
    survivors: dict[str, Counter[str]] = {k: Counter() for k in PATTERNS}

    # Corpus-wide, label-independent day-first candidates.
    day_first_candidates: Counter[str] = Counter()
    day_first_candidate_headers: Counter[str] = Counter()
    day_first_candidate_total = 0
    # …and what each pattern would cost them. UPPER BOUND, not a cost: the
    # candidate population is wide and mostly financial figures whose leading
    # digits coincide with a legal day and month, so a "loss" here is usually a
    # correct rejection. The header census beside it is what tells them apart.
    day_first_cost: dict[str, int] = {k: 0 for k in PATTERNS}
    day_first_cost_headers: dict[str, Counter[str]] = {
        k: Counter() for k in PATTERNS}

    for column_name, label, raw in rows:
        values = split_samples(raw)
        header = (column_name or "").lower()

        if label and label.startswith("datetime.date.compact"):
            family[label] += 1
        if label and label.startswith(DAY_FIRST_SEPARATOR_PREFIX):
            day_first_separator[label] += 1

        if reads_as_day_first(values):
            day_first_candidate_total += 1
            day_first_candidates[label or "(none)"] += 1
            day_first_candidate_headers[header] += 1
            for name, pat in PATTERNS.items():
                r = pass_rate(values, pat)
                if r is not None and r < VETO_THRESHOLD:
                    day_first_cost[name] += 1
                    day_first_cost_headers[name][header] += 1

        if label != DMY:
            continue

        dmy_headers[header] += 1
        if not values:
            dmy_unmeasurable += 1
            continue
        for name, pat in PATTERNS.items():
            r = pass_rate(values, pat)
            assert r is not None  # values is non-empty
            rates[name].append(r)
            if r < VETO_THRESHOLD:
                below[name] += 1
            else:
                survivors[name][header] += 1

    measured = len(rates["candidate"])

    def per_pattern(name: str) -> dict[str, object]:
        rs = rates[name]
        return {
            "pattern": PATTERNS[name].pattern,
            "mean_pass_rate": (sum(rs) / len(rs)) if rs else None,
            "columns_at_pass_rate_1_0": sum(1 for r in rs if r == 1.0),
            "columns_below_veto_threshold": below[name],
            "fraction_below_veto_threshold": (
                below[name] / len(rs)) if rs else None,
            "top_headers_still_passing": survivors[name].most_common(
                args.top_headers),
            "day_first_candidate_columns_lost_upper_bound":
                day_first_cost[name],
            "day_first_candidate_headers_lost":
                day_first_cost_headers[name].most_common(args.top_headers),
        }

    report = {
        "pass_parquet": args.pass_parquet,
        "total_columns_profiled": total_columns,
        "veto_threshold": VETO_THRESHOLD,
        "compact_family_counts": dict(family.most_common()),
        "compact_dmy": {
            "columns": family.get(DMY, 0),
            "measurable": measured,
            "unmeasurable_no_samples": dmy_unmeasurable,
            "top_headers": dmy_headers.most_common(args.top_headers),
            "by_pattern": {k: per_pattern(k) for k in PATTERNS},
        },
        "day_first_legitimacy": {
            "compact_day_first_candidates": {
                "definition": (
                    "every sampled value is 8 digits with day 01-31 and month "
                    "01-12, and at least one leading pair is 13-31 — a pair no "
                    "month can be. WIDE on purpose: a YYYYMMDD date that also "
                    "reads day-first is counted, so this is an upper bound"),
                "columns": day_first_candidate_total,
                "fraction_of_corpus": (
                    day_first_candidate_total / total_columns
                    if total_columns else None),
                "by_current_label": dict(day_first_candidates.most_common()),
                "top_headers": day_first_candidate_headers.most_common(
                    args.top_headers),
            },
            "separator_bearing_day_first_control": {
                "why": ("the control for corpus skew: if this is empty the "
                        "corpus holds no day-first dates at all and nothing "
                        "can be concluded about the compact leaf"),
                "by_label": dict(day_first_separator.most_common()),
                "columns": sum(day_first_separator.values()),
            },
        },
    }

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    json.dump(report, sys.stdout, indent=2, sort_keys=True)
    print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
