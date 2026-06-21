#!/usr/bin/env python3
"""Numeric-range column features (spec 2026-06-21-numeric-range-and-robust-gating, ac-01).

The format view the two-view overnight was missing: a STATIC, distribution-aware summary
of a column's values that separates numeric types differing only in MAGNITUDE/RANGE —
latitude (decimals in [-90,90]) vs longitude ([-180,180]) vs year (integers ~1900-2100)
vs postcode (fixed-width digit strings, often leading-zero) vs an arbitrary decimal_number.

ac-00 confirmed the Sharpen layer cannot recover these (its coordinate rules only fire
downstream of a coordinate prediction; value range is used to reject, never to promote).
So the model must learn the range from features. This is NOT a semantic encoder — it's
pure value arithmetic (fast, static by construction, no transformer), giving the model the
distribution shape (percentiles, sign, decimals, length, charset) so it can LEARN that
"bounded ~[-90,90] decimals, ~half negative" is latitude where "unbounded decimals" is not.

NUMERIC_RANGE_DIM is fixed; the vector order is stable (do not reorder — it is an FTMB slot).
"""
import math

NUMERIC_RANGE_DIM = 32


def _signed_log(x):
    """Signed log10(|x|+1): compresses huge scale spans while keeping sign + magnitude."""
    return math.copysign(math.log10(abs(x) + 1.0), x)


def numeric_range_features(values):
    """Return NUMERIC_RANGE_DIM static features for a column of string values."""
    vals = [str(v).strip() for v in values if v is not None and str(v).strip() != ""]
    n = max(len(vals), 1)

    nums = []
    for v in vals:
        try:
            nums.append(float(v))
        except ValueError:
            pass

    f = []
    # ── numeric distribution (range / magnitude / sign / spread) ──────────────
    f.append(len(nums) / n)  # frac parseable as number
    if nums:
        s = sorted(nums)

        def pct(p):
            return s[min(len(s) - 1, int(p * len(s)))]

        # signed-log percentiles — the RANGE signal (lat min/max ~[-90,90], year ~[1900,2100])
        f += [_signed_log(s[0]), _signed_log(pct(.25)), _signed_log(pct(.5)),
              _signed_log(pct(.75)), _signed_log(s[-1])]
        absn = sorted(abs(x) for x in nums)
        f += [math.log10(absn[0] + 1), math.log10(absn[len(absn) // 2] + 1),
              math.log10(absn[-1] + 1)]  # abs-magnitude percentiles
        f.append(sum(1 for x in nums if x < 0) / len(nums))     # frac negative
        f.append(sum(1 for x in nums if x == 0) / len(nums))    # frac zero
        f.append(sum(1 for x in nums if x == int(x)) / len(nums))  # frac integer-valued
        m = sum(_signed_log(x) for x in nums) / len(nums)
        f.append((sum((_signed_log(x) - m) ** 2 for x in nums) / len(nums)) ** 0.5)  # spread
    else:
        f += [0.0] * 12

    # ── decimal structure (string-based) ──────────────────────────────────────
    dps = []
    for v in vals:
        if "." in v:
            frac = v.split(".", 1)[1]
            dps.append(len(frac) if frac.isdigit() else 0)
        else:
            dps.append(0)
    f.append(sum(1 for v in vals if "." in v) / n)   # frac with decimal point
    f.append(sum(dps) / n)                            # mean decimal places
    f.append(max(dps) if dps else 0)                 # max decimal places

    # ── length / digit-count ──────────────────────────────────────────────────
    lens = [len(v) for v in vals]
    f += [min(lens), max(lens), sum(lens) / n]
    dc = [sum(c.isdigit() for c in v) for v in vals]
    f.append(sum(dc) / n)                             # mean digit count
    f.append(sum(1 for x in lens if x == lens[0]) / n)  # frac same length as first (fixed-width)
    f.append(sum(1 for v in vals if v[:1] == "0" and len(v) > 1) / n)  # frac leading-zero (postcode!)
    f.append(len(set(vals)) / n)                     # distinct ratio (cardinality)

    # ── charset fractions ─────────────────────────────────────────────────────
    allc = "".join(vals)
    tc = max(len(allc), 1)
    f.append(sum(c.isdigit() for c in allc) / tc)
    f.append(sum(c.isalpha() for c in allc) / tc)
    f.append(sum(c == "." for c in allc) / tc)
    f.append(sum(c == "-" for c in allc) / tc)
    f.append(sum(c in " ,;:/" for c in allc) / tc)
    f.append(sum(1 for v in vals if v.isdigit()) / n)  # frac purely-digit values

    # pad / truncate to the fixed slot width, clamp non-finite
    f = [(x if math.isfinite(x) else 0.0) for x in f]
    if len(f) < NUMERIC_RANGE_DIM:
        f += [0.0] * (NUMERIC_RANGE_DIM - len(f))
    return f[:NUMERIC_RANGE_DIM]


if __name__ == "__main__":
    # Self-test: the four confusable types should land in visibly different feature space.
    cases = {
        "latitude": ["40.7128", "-74.0060", "51.5074", "-33.8688", "35.6895", "-22.9", "48.85"],
        "longitude": ["-122.4194", "151.2093", "139.6917", "-0.1278", "-118.24", "174.76"],
        "year": ["1995", "2001", "2018", "1987", "2024", "1999", "2010"],
        "postcode": ["01234", "90210", "10001", "60601", "02134", "07030", "00501"],
        "decimal": ["3.14159", "1024.5", "0.0033", "9999.9", "42.0", "-7.5", "318000.25"],
    }
    import json
    vecs = {k: numeric_range_features(v) for k, v in cases.items()}
    print(f"NUMERIC_RANGE_DIM={NUMERIC_RANGE_DIM}, all len OK:",
          all(len(v) == NUMERIC_RANGE_DIM for v in vecs.values()))

    def cos(a, b):
        d = sum(x * y for x, y in zip(a, b))
        na = math.sqrt(sum(x * x for x in a)); nb = math.sqrt(sum(y * y for y in b))
        return d / (na * nb + 1e-9)
    keys = list(cases)
    print("\npairwise cosine (lower = better separation):")
    for i in range(len(keys)):
        for j in range(i + 1, len(keys)):
            print(f"  {keys[i]:9} vs {keys[j]:9} = {cos(vecs[keys[i]], vecs[keys[j]]):+.3f}")
