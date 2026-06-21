# Honest Sense-vs-Sense comparison (2026-06-21) — the gte direction was wrongly buried

Every prior gte "NO-GO" compared gte STANDALONE (Sense argmax) to v19 COMPOSED (Sense + Sharpen,
~+0.29). The Sharpen rules are model-agnostic. The fair comparison is Sense vs Sense
(profile --raw-model for v19/cdist; predict_multibranch for gte). On gold:

| model            | Sense-only | composed |
|------------------|-----------|----------|
| v19 (shipped)    | 0.502     | 0.793    |
| gte floor        | 0.532     | —        |
| gte two-view     | 0.571     | —        |
| cdist (col-dist) | 0.316     | 0.685    |

## Findings
1. gte two-view Sense (0.571) BEATS v19 Sense (0.502) by +0.069; even the frozen floor (0.532)
   beats it. The gte representation produces a better Sense than the shipped model. The gte
   NO-GOs were a measurement artefact (Sense vs Sense+Sharpen).
2. cdist (column-distribution) Sense = 0.316 << v19 0.502 — a REAL NO-GO; its composed 0.685 was
   the rules carrying a weak Sense.
3. Reproducibility, quantified: a fresh retrain (cdist) Sense = 0.316 vs v19's tuned 0.502 — a
   ~0.19 fresh-retrain penalty at the Sense level. gte two-view (also fresh) hits 0.571 — the gte
   embed OVERCOMES the penalty and beats v19. gte is the forward path out of the unreproducible hole.

## Next
Compose the two-view predictions (apply the shared Sharpen stack to its 0.571 Sense) and score
composed two-view vs v19 composed 0.793 — the real ship gate, estimated ~0.79-0.86.
Tooling: score_gold_anchor.py predict --raw-model (Sense-only, now exposed).

## Composed honest gate (2026-06-21) — the real ship comparison
Composed two-view's STANDALONE predictions through the real Sharpen stack via the new
FINETYPE_INJECT_LABEL hook (override the Sense label, run profile's Sharpen — works for any
model since the rules are value-based, no candle). On gold:

| model        | Sense-only | composed |
|--------------|-----------|----------|
| v19          | 0.502     | 0.793    |
| two-view gte | 0.571     | 0.787    |

VERDICT: composed two-view (0.787) is statistically TIED with v19 (0.793; CI 0.760-0.812 overlaps)
— NOT a NO-GO. The +0.069 Sense advantage closes to ~0 composed because the shared Sharpen rules
add more to v19's weaker Sense (+0.291) than to two-view's (+0.216) — the rules are partly
redundant with a better Sense. Two-view ties the TUNED shipped v19 as a single-seed FRESH retrain
(overcoming the ~0.19 reproducibility penalty). A tie doesn't justify gte's inference cost over v19
on accuracy alone; the case is the better Sense + headroom (3-seed/tuning) + rule-simplification.
Tooling: scripts/compose_predictions.py + FINETYPE_INJECT_LABEL.
