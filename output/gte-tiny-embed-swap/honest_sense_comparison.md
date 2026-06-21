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
