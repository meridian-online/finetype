# Overcoming the "fixed attention budget" — research memo (2026-06-13)

Deep-research pass (5 angles, 10 findings, each adversarially verified 3-0) on how a
SMALL CPU-bound classifier can acquire new class distinctions via incremental retrain
WITHOUT catastrophic forgetting of untargeted classes and WITHOUT inference-cost
increase. Commissioned after the mining-factory campaign hit its 4th additive-blend
NO-GO (see [[mfg-localefmt-identifier-collapse]]).

## Diagnosis (confirmed by the literature)

FineType's untargeted-class collapse is textbook **class-incremental interference**: a
single shared softmax with no task identity at inference, forced to discriminate
between classes never optimised together — the hardest continual-learning regime
(van de Ven et al., *Nature Machine Intelligence* 2022). Our retrain is technically
imbalanced *joint* retraining with negative transfer, but the shared-head interference
mechanism is the same.

**The actionable mechanism — the "tragic triad"** (Yu et al., PCGrad, NeurIPS 2020,
2000+ cites): negative transfer needs THREE things together — conflicting gradients
**+ large gradient-magnitude difference + high positive curvature**. Conflict alone is
harmless. The magnitude-difference leg is FineType's failure: flooding manufactured
volume for one starved class makes its gradient dominate the shared update and stamp
over untargeted classes. **That leg is cheap to attack** — it's a data-mix/loss dial.

## Ranked bets (all zero inference cost unless noted)

1. **Balanced replay (INVEST FIRST).** Keep prior real columns balanced against new
   manufactured data; don't dump volume. "The only strategy among the top performers
   in all three [incremental] scenarios is replay" — beats costlier A-GEM/FROMP at
   strictly lower cost (van de Ven 2022, PMC9771807). Proven on small CNNs/MLPs.
   Zero inference cost (training-time data/memory only). For FineType this is the
   `--ratio-distilled` / `--distilled-cap` / `--max-cols-per-type` mix we already pass.
2. **Class-balanced / logit-adjusted loss (PAIR WITH replay).** Strengthen rare
   classes by REWEIGHTING, not by manufacturing thousands of columns — directly
   attacks the dominating-gradient leg. Zero inference cost, low effort.
3. **PCGrad gradient surgery (second-order add-on).** Projects conflicting gradients
   apart in the optimiser step only; model-agnostic, any shared-parameter head, zero
   inference cost. Proven on supervised CNN multi-task (CIFAR-100 71%, CelebA).
   Medium effort — needs gradient-group definition.
4. **Mergeable LoRA / RepAdapter (longer shot).** Low-rank delta folds into weights
   (W = W0+BA) → zero added FLOPs, structurally identical to full fine-tune. BUT
   proven only on large Transformers/ViTs; the merge property is architecture-agnostic
   and transfers, the forgetting-cure efficacy at small-classifier scale does NOT.
   Would be a genuine research bet, not a safe one.

## Explicitly NOT recommended

- **EWC / Synaptic Intelligence / MAS.** Fail almost completely in class-IL — near the
  no-protection baseline (EWC 20.6% vs 19.9% on Split-MNIST class-IL, PMC9771807).
  All three approximate the same Fisher information (Benzing, AISTATS 2022); cheapness
  buys nothing for cross-class interference. These are the tempting cheap option and
  the evidence says skip them.
- **PackNet / HAT (expert masks).** Zero inference cost but REQUIRE a per-task ID at
  inference to select the mask — a flat-softmax column classifier has none. Ruled out.

## Connection to what we observed

This session's locale-format run HELD the numerics by spreading mass across 34 types
(more balanced) where the geography-heavy run COLLAPSED them (concentrated/dominant) —
we stumbled onto the balance effect. The research says do it deliberately: reweight
for rare-class lift, balance the mix so no class dominates, skip the volume flood.

## Open questions (not answered by research — need experiments)

1. Can a logit-adjusted loss strengthen latitude with NO manufactured volume? Cleanest
   test of "reweight, don't flood".
2. Does mergeable LoRA mitigate forgetting on a small from-scratch multi-branch head?
   Unproven below Transformer scale.

## Sources

- van de Ven et al., *Three types of incremental learning*, Nature Machine Intelligence 2022 — https://www.nature.com/articles/s42256-022-00568-3 / https://pmc.ncbi.nlm.nih.gov/articles/PMC9771807/
- Yu et al., *Gradient Surgery for Multi-Task Learning (PCGrad)*, NeurIPS 2020 — https://proceedings.neurips.cc/paper/2020/file/3fe78a8acf5fda99de95303940a2420c-Paper.pdf
- Benzing, *Unifying importance-based regularisation (EWC/SI/MAS)*, AISTATS 2022 — https://proceedings.mlr.press/v151/benzing22a/benzing22a.pdf
- Hu et al., *LoRA*, 2021 — https://arxiv.org/html/2106.09685v2 ; RepAdapter — https://arxiv.org/pdf/2302.08106 ; C-LoRA — https://arxiv.org/html/2502.17920v1
- Mallya & Lazebnik, *PackNet*, CVPR 2018 — https://openaccess.thecvf.com/content_cvpr_2018/papers/Mallya_PackNet_Adding_Multiple_PackNet_CVPR_2018_paper.pdf
