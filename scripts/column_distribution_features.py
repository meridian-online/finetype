#!/usr/bin/env python3
"""Column-distribution features the 27-dim stats branch GENUINELY lacks
(spec 2026-06-21-column-distribution-features, ac-01).

The existing extract_column_stats(27) already has cardinality/entropy, length stats, and
char composition (ac-00 audit). So this adds ONLY the genuinely-missing signal — and not
length/charset (would duplicate the 27):
  - parsed numeric VALUE & range (the 27 never parses the value; length_min/max are string length)
  - column-level PRECISION (decimal_places is per-value only, not aggregated into the 27)
  - SEQUENTIALITY / monotonicity (increment/serial)

This is the prototype for the Rust port into extract_column_stats. Order is stable.
"""
import math

COLDIST_DIM = 20


def column_distribution_features(values):
    vals = [str(v).strip() for v in values if v is not None and str(v).strip() != ""]
    n = max(len(vals), 1)
    nums = []
    for v in vals:
        try:
            nums.append(float(v))
        except ValueError:
            pass

    def slog(x):
        return math.copysign(math.log10(abs(x) + 1.0), x)

    f = []
    # ── parsed numeric value & range ──────────────────────────────────────────
    f.append(len(nums) / n)  # frac parseable
    if nums:
        s = sorted(nums)

        def pct(p):
            return s[min(len(s) - 1, int(p * len(s)))]

        f += [slog(s[0]), slog(pct(.5)), slog(s[-1])]        # signed-log min/median/max (range)
        abss = sorted(abs(x) for x in nums)
        f += [math.log10(abss[0] + 1), math.log10(abss[-1] + 1)]  # abs magnitude min/max
        f.append(sum(1 for x in nums if x < 0) / len(nums))   # frac negative
        f.append(sum(1 for x in nums if x == int(x)) / len(nums))  # frac integer-valued
        m = sum(slog(x) for x in nums) / len(nums)
        f.append((sum((slog(x) - m) ** 2 for x in nums) / len(nums)) ** 0.5)  # signed-log spread
        f.append(sum(1 for x in nums if abs(x) <= 90.0) / len(nums))   # bounded-90 (lat hint)
        f.append(sum(1 for x in nums if abs(x) <= 180.0) / len(nums))  # bounded-180 (coord hint)
    else:
        f += [0.0] * 10

    # ── column-level precision ────────────────────────────────────────────────
    dps = [len(v.split(".", 1)[1]) if "." in v and v.split(".", 1)[1].isdigit() else 0
           for v in vals]
    f.append(sum(dps) / n)                                  # mean decimal places
    f.append(max(dps) if dps else 0)                        # max decimal places
    f.append(sum(1 for d in dps if d >= 4) / n)             # frac high-precision (coord-ish)

    # ── sequentiality / monotonicity ─────────────────────────────────────────
    if len(nums) >= 2:
        asc = sum(1 for a, b in zip(nums, nums[1:]) if b >= a) / (len(nums) - 1)
        f.append(max(asc, 1.0 - asc))                       # monotonic fraction (either direction)
        ints = [x for x in nums if x == int(x)]
        if len(ints) >= 2:
            span = max(ints) - min(ints) + 1
            f.append(len(set(ints)) / span if span > 0 else 0.0)  # contiguous-run ratio (increment)
        else:
            f.append(0.0)
        f.append(1.0 if nums == sorted(nums) else 0.0)      # is-sorted
    else:
        f += [0.0, 0.0, 0.0]

    f = [(x if math.isfinite(x) else 0.0) for x in f]
    if len(f) < COLDIST_DIM:
        f += [0.0] * (COLDIST_DIM - len(f))
    return f[:COLDIST_DIM]
