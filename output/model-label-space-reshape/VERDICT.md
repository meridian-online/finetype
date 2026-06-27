# Model label-space reshape — FINAL VERDICT: NO-GO (leaf-drop disproven on gold)

Spec 2026-06-27-model-label-space-reshape. Decided 2026-06-28 on the 3-seed retrain.

## The number

| system | composed gold (reframe) | vs baseline |
|---|---|---|
| s43 baseline (shipped) | 794/931 = **0.853** | — |
| reshaped-111 seed 42 + recovery | 755/931 = 0.811 | −4.2pp |
| reshaped-111 seed 43 + recovery | 775/931 = 0.832 | −2.0pp |
| reshaped-111 seed 44 + recovery | 737/931 = 0.792 | −6.1pp |
| **mean of 3 seeds** | **0.812** | **−4.1pp** |

**All three seeds are below baseline.** The `alphanumeric_id→word` swing was NOT seed noise —
it is a consistent, structural cost. Per choice 0104 the bar is gold parity-or-better; the
reshape misses it on every seed. **NO-GO.**

## Why it fails (from the ac-3 per-column diagnosis)

The recovery rule is gold-clean (s43 rule-on == rule-off, zero over-fire). The loss is the
RESHAPED MODEL getting worse on the KEPT classes:
- **Residual-boundary degradation** — `alphanumeric_id → word` (the biggest bucket). Removing
  134 classes rebalanced the model's residual attractor and it got worse at the open-vocab
  boundary it still owns.
- **Attractor drift to held-back leaves** recovery structurally can't catch (integer→ndc,
  url→docker_ref, word→country_code) — the columns aren't really those types, so recovery
  correctly stays out, but the model error stands.

## Honest confound

The reshaped models trained on 87k records (44k ceded rows removed) vs the baseline's 131k —
a ~33% training-data reduction is baked into the leaf-drop (ceded columns have no valid target
once their label leaves the output space). So the −4pp blends class-removal and data-reduction.
Recovering the data means keeping ceded columns under a residual label — which reintroduces the
residual-attractor trap the reshape was meant to avoid. So the SIMPLE clean leaf-drop is dead;
a data-preserving variant is a different, attractor-risky, unproven bet.

## What the experiment proved (the value, despite NO-GO)

- ac-2: removing the ceded leaves DOES eliminate the targeted over-emitters (isbn/currency_code/
  si_number → 0) — but the FP mass relocates (43% to specific kept leaves), and recovery absorbs
  ~85% of the specific drift. The over-emit problem is real but class removal is the wrong cure.
- The over-emit fix belongs in TRAINING NEGATIVES (task t-000133e418), not label-space surgery.

## Disposition

- SHELVE the leaf-drop reshape. choice 0108 → rejected.
- KEEP (do not ship): `ceded_leaf_recovery` (gold-clean but a no-op on the 244-label model, so
  no standalone value), the cede-filter infra (Rust `--cede-labels` deny-list, Python filter),
  the ac-0 partition. All reusable if the data-preserving variant is ever revisited.
- The fast-path sizing (~8% columns skippable) is independent and could be pursued separately.
