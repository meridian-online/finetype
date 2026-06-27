# ac-3 — composed gold parity (recovery rule in place)

Offline path: predict_multibranch (raw Sense) → compose_predictions (REAL Sharpen via
binary, incl. ceded_leaf_recovery) → score_gold_anchor --reframe. gold_m2v8m.ftmb, 931 cols.

| system | composed gold | 95% CI |
|---|---|---|
| s43, recovery rule OFF (shipped baseline) | 794/931 = **0.853** | 0.829–0.874 |
| s43, recovery rule ON | 794/931 = **0.853** | 0.829–0.874 |
| reshaped-111 + recovery (candidate) | 775/931 = **0.832** | 0.807–0.855 |

## Two findings

**1. The recovery rule is gold-CLEAN.** s43 rule-on == rule-off, byte-identical (794/931).
Zero over-fire on the 931 gold columns — the rule only corrects, never regresses the
production model. The gold-regression concern about the rule itself is answered: none.

**2. The reshaped model is −2.1pp under baseline** (0.832 vs 0.853; CIs overlap, so within
CI, but NOT parity-or-better). This is UNDER-recovery + model drift, not rule over-fire.

## Where the 19-net-column loss is (36 regressions − 17 gains)

The regressions are dominated by the RESHAPED MODEL's own predictions, not the recovery:
- **9 alphanumeric_id → word** — the reshaped model shifts the residual boundary
  (alphanumeric_id is KEPT open-vocab; removing 134 classes rebalanced its residual
  attractor). Biggest single bucket. Plausibly seed variance — needs 3 seeds to confirm.
- **attractor drift to HELD-BACK leaves recovery can't catch:** integer→ndc, word→country_code,
  url→docker_ref (3). Those targets aren't ceded, and the columns aren't really that type, so
  recovery correctly stays out — but the model error stands.
- isbn→(no header) 1, uuid 1 — recovery gaps (header-gated / <3 values).

Gains (17): country_code +3, region +2, word +6 — the reshape helps some geography/residual.

## Verdict: NOT clean parity at this seed — a within-CI miss

Per choice 0104 the bar is gold parity-or-better; 0.832 < 0.853 point estimate misses it.
The reshape's thesis (simplicity AND accuracy) is NOT confirmed on gold — accuracy is
roughly neutral-to-slightly-negative at this single seed. The recovery rule works; the gap
is (a) a residual-boundary model shift and (b) uncaught attractor drift into held-back leaves.

## Options to close the gap
1. **3-seed retrain** — separate seed variance from a real reshape cost (the alphanumeric_id
   shift is the swing factor). The spec's plan anyway.
2. **Cheap recovery/veto gaps:** url→docker_ref override (3), cede ndc with checksum (drift sink).
3. Accept the reshape's value as simplicity + over-emit removal + ~8% fast-path, at a possible
   ~2pp gold cost — a value call, not an accuracy win.
