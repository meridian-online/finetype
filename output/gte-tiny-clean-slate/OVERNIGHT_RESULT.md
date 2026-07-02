# Overnight clean-label retrain — started Fri 19 Jun 2026 23:00:44 AEST

Training set: cs_train_clean.tsv (152,639 rows, 205 labels, 25% residual, model-independent).
v19 baseline: gold 0.798, repr 0.691 (--reframe).

| checkpoint | gold standalone | gold composed | repr standalone | repr composed |
|---|---|---|---|---|
| clean_lin_e8 | 0.520 | 0.769 | 0.483 | 0.614 |
| clean_lin_e12 | 0.524 | 0.766 | 0.502 | 0.614 |
| clean_lin_e16 | 0.525 | 0.769 | 0.502 | 0.610 |
| clean_mlp_e16 | 0.529 | 0.770 | 0.502 | 0.618 |

Finished Sat 20 Jun 2026 01:07:16 AEST

## Gated clean full-label model (morning follow-up)
| approach | gold | repr |
|---|---|---|
| v19 baseline | 0.798 | 0.691 |
| clean full-label, wholesale composed | 0.770 | 0.618 |
| clean full-label, GATED (tau0.95) | 0.690 | 0.660 |
| June 8-family, GATED | 0.809 (+1.1) | 0.691 (flat) |

CONCLUSION: a BROAD gte-tiny is a worse escalation witness than a NARROW one. The June
8-family head won BECAUSE it was focused (clean labels, fires only on contested boundaries,
165:4). Spreading to 205 labels dilutes per-override accuracy -> regresses. Across 4
experiments (composed noisy, composed clean, gated full-label, gated narrow) the pattern is
consistent: NARROW gated escalation wins (+1.1/flat); BROAD replacement/escalation regresses.
gte-tiny's value is a focused contested-boundary specialist ON TOP of v19, not a backbone.
The clean slate (drop v19) does not pan out; the narrow escalation does.
