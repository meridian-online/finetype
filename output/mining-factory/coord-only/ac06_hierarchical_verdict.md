# Hierarchical-head experiment — FALSIFIED. Model-side delivery is exhausted for coordinates.

Spec `2026-06-07-reference-data-mining-factory`, research arm of choice 0097. Tested
whether a domain→category→leaf hierarchical head holds the coordinate-vs-numeric
boundary the flat softmax could not. Same coord-only FTMB, `--head hierarchical`,
single seed. Evaluated at the epoch-33 best checkpoint (val_acc 0.906; the run was
killed externally at epoch 36 before full convergence — see caveat).

## Verdict: WORSE than the flat head on every axis

| instrument | flat coord-only | hierarchical |
|---|---|---|
| Gold anchor | 0.718 | **0.685** (−3.3pp) |
| Sense drift | GO | **NO-GO** (`user_agent`) |
| Corpus-honest gate | NO-GO (7) | **NO-GO (8)** |
| `numeric_code` (the load-bearing collapse) | 59k→10.6k (18% kept) | 59k→**1.7k (3% kept)** |

Hierarchical gate triggers: `numeric_code` collapse to 3% (base_correct 6,936→111),
`user_agent` 21k→149k (7.1×, oracle confirms zero), `isbn` 8k→34k (4.3×), `si_number`
2.1×, `file_size` 4.1×, `compact_ymd` collapse, plus NEW cross-domain instability
`calver` 8.2× and `docker_ref` — technology-domain types the flat run never tripped.

## Why it failed (mechanism)

The domain split did not isolate coordinates from numerics. The shared trunk
(char/embed/stats/validation branches) still entangles the representations before the
head, so the coordinate-decimal signal still bleeds into the numeric region — and the
hierarchical head ADDED interference surface (a domain softmax + a category softmax +
leaf softmaxes, each with its own boundaries), which is why technology-domain types
(`user_agent`, `calver`, `docker_ref`, `isbn`) destabilised. Splitting the OUTPUT head
does not fix interference that originates in the SHARED REPRESENTATION.

## Convergence caveat (and why it does not rescue the hypothesis)

This is the epoch-33 best of a run killed at 36, ~1pp short of a likely peak. Fuller
convergence might recover ~1pp of gold and soften `user_agent`. It cannot reverse the
core result: `numeric_code` collapsed HARDER than the fully-converged flat run (3% vs
18%), and the flat run proved at FULL convergence that this collateral is a stable
representational-overlap effect, not an under-training artifact. A few more epochs do
not un-overlap two value-shapes. The hypothesis is falsified.

## Campaign-level conclusion: model-side delivery is exhausted

Every model-retraining route the research pointed at has now been tried for coordinates:

- **Additive blend, flat head** (locale-format, coord-only): NO-GO — relocates collateral.
- **Balanced/minimal replay** (coord-only): improved (gold-clean, Sense-GO) but still gate NO-GO on numerics.
- **Logit-adjusted loss** (choice 0097 lever 2): built + tested, but it corrects class-FREQUENCY imbalance, not the shape-overlap here — correctly NOT run on coordinates.
- **Hierarchical / per-family head**: FALSIFIED (this doc) — worse, not better.
- **Mergeable LoRA**: research flags it unproven below Transformer scale; not pursued.

The coordinate-vs-numeric boundary is a VALUE-SHAPE overlap (coordinates ARE decimals).
No output-head or training-loss change holds it, because the overlap is in the input
representation. This is now confirmed four independent ways (v24-latitude, locale-format,
flat coord-only, hierarchical coord-only).

## Settled recommendation

**Ship coordinates via a Sharpen value-rule** (latitude ∈ [−90,90], longitude ∈
[−180,180], decimal sub-degree precision; value-only per choice 0048/0096). It is the
only delivery that confirms coordinates WITHOUT touching the shared numeric
representation — zero model risk, banks the lat/lon efficacy win manufacturing proved
reachable (perfect on gold). Stop retraining for coordinates. The follow-up task is
already filed. Reserve the logit-adjusted lever for a genuinely frequency-starved (not
shape-overlapping) class.

Substrate: gold `report_mfg-coords-hier-s42_*.md`, gate
`output/corpus-honest-gate/gate_mfg-coords-hier-s42.json`, Sense `drift_report_full.txt`.
Links [[coord-only-blend-numeric-collateral]], [[catastrophic-forgetting-cure-is-train-time]].
