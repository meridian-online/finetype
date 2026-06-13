# Representation-separability research — closing the framing gap (2026-06-14)

Second deep-research pass (5 angles, ~21 claims, each adversarially verified; findings
below are the 3-0 survivors). Framed as a feature-SEPARABILITY problem (the angle the
first forgetting-framed pass missed). Commissioned after every model-side delivery —
flat blend, balanced replay, logit-adjust, hierarchical head — failed the
coordinate-vs-numeric boundary the same way ([[hierarchical-head-falsified-for-shape-overlap]]).

## The diagnosis is confirmed correct — and it's class OVERLAP, not imbalance or forgetting

Class overlap is formally low class-separability: "an overlapped domain is one where
the class separability is low," and "class overlap is undeniably problematic, even in
balanced domains" (Santos et al., *Information Fusion* 2022). When coordinates and
numeric_code occupy the same region of the shared representation, **no clean boundary
exists there** — which is exactly why output-head-only and hierarchical-head changes
could not fix it. Our mechanism call was right.

## Decoupling (the arm we flagged as "missed") — answered: it would NOT have helped

- cRT / LWS / tau-normalization (Kang et al., ICLR 2020, ~3000 cites) recover a class
  that is merely IMBALANCED — under-sampled but already SEEN and separated by the
  representation. "Data imbalance might not be an issue in learning high-quality
  representations." Freezing the backbone + retraining the head fixes classifier BIAS.
- BUT for a genuine representation GAP — a starved class the trunk never learned to
  separate (our coordinates) — frozen-feature classifier-only adaptation BREAKS;
  backbone updates are required (Luo et al., ICML 2023). "Backbone adaptation is
  preferred when the domain shift [is] so large that the learned feature space
  deforms."

So we did not miss a fix — we missed *confirming a non-fix*. Decoupling addresses bias,
not gaps; coordinates are a gap.

## The vise, now with citations (why every retrain failed)

- **Freeze the backbone** → can't learn the starved class (no separating direction exists to reweight).
- **Update the backbone** to create the direction → DISTORTS the shared representation and harms neighbours. Full fine-tuning beats linear probing ~2% in-distribution but loses **~7% out-of-distribution** across 10 shift datasets (Kumar et al., ICLR 2022 spotlight). This is precisely the mechanism of our `numeric_code` collapse (59k→1.7k): retraining to fit coordinates deformed the lower-layer features the overlapping neighbours depend on.

This is a structural dilemma, not a tuning failure. It is why dose, replay balance, and head architecture all failed identically.

## The genuinely new lever (zero inference cost) — for the NEXT hard class

Training-time **feature-geometry-shaping losses** target the exact entanglement layer
the plain softmax ignores (softmax penalises only classification error; it does not
shape feature geometry):

- **Supervised contrastive (SupCon)** (Khosla et al., NeurIPS 2020): pulls same-class points together, pushes different-class clusters apart.
- **ArcFace additive angular margin** (Deng et al., CVPR 2019): an angular margin in the softmax to maximise class separability.
- **Center / contrastive-center / prototype losses**: pull features to class centroids.

Inference cost: **ZERO** — the plain softmax/cosine head is unchanged at inference; the
loss only acts during training. Caveats: training stability, and small-classifier-scale
evidence is thinner than the large-model results.

## For coordinates specifically — the rule wins, confirmed

When the distinction is a cleanly checkable property (bounded range lat[−90,90] /
lon[−180,180] + a precision signature), a deterministic post-hoc RULE or an engineered
input feature beats representation learning on cost: a rule adds no loss term and no
constrained optimisation, whereas encoding a range as a loss constraint is "a difficult
optimisation problem" (Dash et al. 2021). And at the small data scale a starved class
lives in, engineered features ≈ deep representation learning (deep nets only +0.006 A',
−0.056 kappa across constructs; Jiang et al., AIED 2018) — no decisive separability gain
from learning. Output-range bounds are a recognised, distinct class of domain-knowledge
constraint (von Rueden / Borghesi surveys).

## Decision-grade takeaways

1. **Coordinates → Sharpen value-rule.** Settled, now triply confirmed (empirically by
   5 NO-GOs, and by this evidence: rule beats learning for cleanly-checkable distinctions
   at small scale).
2. **The next starved class whose distinction is NOT cleanly rule-expressible → add a
   feature-geometry-shaping loss** (SupCon / ArcFace / center) to the training objective.
   Zero inference cost; reshapes the shared trunk so the new class separates without the
   backbone-distortion neighbour-collapse. This is the next sanctioned engine investment
   when a rule won't express the boundary — the analogue of the logit-adjust lever for
   shape-overlap rather than frequency.
3. **Do NOT pursue decoupling/frozen-backbone for starved classes** — it only fixes
   classifier bias, not representation gaps.

## Sources
- Santos et al., class-overlap survey, *Information Fusion* 2022 — https://sci2s.ugr.es/sites/default/files/ficherosPublicaciones/2997_2022-INFFUS-Unifying%20view%20overlap.pdf
- Kang et al., *Decoupling representation and classifier*, ICLR 2020 — https://arxiv.org/abs/1910.09217
- Luo et al., ICML 2023 — https://proceedings.mlr.press/v202/luo23e/luo23e.pdf
- Kumar et al., *Fine-tuning can distort pretrained features*, ICLR 2022 — (linear-probe-then-finetune)
- Khosla et al., *Supervised Contrastive Learning*, NeurIPS 2020; Deng et al., *ArcFace*, CVPR 2019
- Dash et al. 2021 (knowledge-constraint survey); von Rueden / Borghesi (informed-ML surveys); Jiang et al., AIED 2018 (engineered-vs-learned at small scale)
