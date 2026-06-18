# ac-00 — is FineType's confidence calibrated? (reliability curve)

**Spec:** 2026-06-18-calibrated-confidence-abstention (GATE)
**Date:** 2026-06-18 · v19 / 0.6.34 · `score … --reframe` lens
**Substrate:** `scripts/probe_confidence_calibration.py`, `calib_gold.tsv`, `calib_repr.tsv`

## Verdict: GO (qualified) — confidence RANKS correctness, but does not equal it.

Surface a **quality band / abstention on the ranking** (robust). Do **not** surface
the raw number as if it were calibrated — it systematically over-states accuracy.

## Reliability curves

**Representative (n=259) — cleanly monotonic, the production-relevant signal:**

| conf bin | n | accuracy | mean conf |
|---|--:|--:|--:|
| [0.00,0.50) | 17 | **0.353** | 0.418 |
| [0.50,0.70) | 64 | 0.578 | 0.587 |
| [0.70,0.85) | 31 | 0.710 | 0.786 |
| [0.85,0.95) | 53 | 0.755 | 0.896 |
| [0.95,1.00) | 94 | **0.851** | 0.988 |

**Gold (n=805) — monotonic with one dip:**

| conf bin | n | accuracy | mean conf |
|---|--:|--:|--:|
| [0.00,0.50) | 45 | 0.644 | 0.411 |
| [0.50,0.70) | 224 | 0.808 | 0.594 |
| [0.70,0.85) | 72 | **0.694** | 0.784 |
| [0.85,0.95) | 136 | 0.919 | 0.904 |
| [0.95,1.00) | 328 | 0.909 | 0.990 |

## The three reads

1. **Monotonic?** Yes on representative (0.35→0.58→0.71→0.76→0.85 — every bin
   better than the last). On gold, monotonic except a dip at [0.70,0.85)=0.694 —
   driven by hardcoded rule/hint confidences (see below). **Net: the ranking is
   trustworthy, especially on production data.**

2. **Calibrated (confidence ≈ accuracy)?** **No.** It is over-confident in the
   high bins: the 0.99 bin is 0.851 accurate on repr (0.909 on gold), and the
   [0.70,0.85) bin is over-stated on both. The [0.50,0.70) bin is *under*-confident
   on gold (0.594 conf, 0.808 acc). So the number ranks well but does not mean
   "right this often" — a surfaced numeric confidence needs an isotonic map; a band
   does not.

3. **Abstention knee.** Abstaining (flag/`unknown`) below a threshold lifts the
   accuracy of what's kept and catches wrong guesses:

   | T | repr kept acc | repr % wrong caught | gold kept acc | gold % wrong caught |
   |--|--:|--:|--:|--:|
   | 0.5 | 0.740 | 15% | 0.861 | 13% |
   | 0.6 | 0.772 | 34% | 0.878 | 32% |
   | **0.7** | **0.798** | **51%** | **0.882** | **48%** |
   | 0.85 | 0.816 | 64% | 0.912 | 66% |

   At **T=0.7** on representative data, kept-accuracy rises 0.714→0.798 and **half
   of all wrong predictions fall into the flagged bucket** (which sits at 0.531
   accuracy — genuinely the coin-flip half). That is a real, useful separation.

## The rule/hint pollution (stated limit)

Rule/veto-set predictions (n=460 gold) carry hardcoded confidences (0.85/0.9) and
are MORE accurate (0.865) at LOWER mean confidence (0.770) than pure-vote
predictions (0.826 acc at 0.874 conf). So the hardcoded confidences are not aligned
with the branch-vote confidences — they are the source of the gold [0.70,0.85) dip.
A calibration map should fit on (or at least not be fooled by) this mixture; a band
cut at 0.7 already absorbs it.

## Hand-off to ac-01

GO. Design a **quality band + abstention** on the ranking (threshold ≈0.7 for the
"scrutinise" cut), optionally an isotonic map if a numeric confidence is surfaced as
honest, and the runner-up on the low band (determinability-probe recommendation).
The precision/recall trade above is the input.
</content>
