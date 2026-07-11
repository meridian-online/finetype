# Pilot spec — two-stage / abstaining head on the static potion encoder

**Status:** scaffolded (no training run this session). Single-seed pilot BEFORE any 3-seed burn.
**Source of the bet:** `output/next-train-research/RESEARCH.md` — "the reachable win" (§ lines 65–70):
the only lever touching the 56% over-tighten mass, latency-neutral, projection **0.60–0.66** raw Sense.
**Precursor (run this first, cheaper):** move-3a `--logit-adjust` A/B (predict-time, no retrain) —
see the cross-reference at the end. If logit-adjust alone recovers a slice of the over-tighten mass,
the retrain bar drops.

> **UPDATE 2026-07-11 — the precursor ran, and it is a RED/YELLOW flag against this pilot.**
> Move-3a is done (`output/logit-adjust-ab/RESULT.md`). The cheap predict-time reweighting
> **net-degrades at every tau, raw AND composed** (raw 0.535→0.416, composed **0.870→0.824**),
> breaks 3–4× more columns than it fixes, and its one real win (lat/lon recall) is **already fully
> recovered by the deterministic Sharpen layer** (composed latitude identical with/without it). Per
> §8 below, a logit-adjust *relocation* is "early evidence the mass is a **representational** overlap
> the softmax can't be re-weighted out of — the hierarchical-head failure mechanism (§0) — a yellow
> flag that the head will relocate too." That is exactly what happened. Also load-bearing: **Sharpen
> already lifts raw 0.535 → composed 0.870 (+0.335)**, so the real addressable headroom at the product
> level is **~121 columns, not the 56% raw over-tighten mass**. **Recommendation: DEMOTE this pilot.**
> Only run it if a redesign conditions the commit gate on column **values** (not class frequency),
> and even then the bar is "beat a Sharpen layer that is already at composed 0.870" — a much thinner
> prize than RESEARCH.md's raw-Sense framing implied. The §5 recover-vs-relocate instrumentation is
> now mandatory, not optional.

---

## 0. The one thing this spec exists to prevent a mistake about

**The proposed head is NOT `HeadType::Hierarchical`. That is a different, already-falsified thing.**

- `HierarchicalHead` (`crates/finetype-model/src/char_cnn.rs:331`) is a **tree-softmax**: it computes
  `p(leaf) = softmax(domain)·softmax(cat)·softmax(leaf|cat)` and `argmax` **always resolves to a leaf**.
  It has **no abstain path** — it cannot emit a coarse/loose node. It is fully implemented, wired
  through `MultiBranchModel::new_hierarchical` (`multi_branch.rs:364`) and `--head hierarchical`
  (`crates/finetype-cli/src/cmd_train.rs:39`), and it was **TRIED and FALSIFIED** on the coordinate
  boundary: gold 0.718→0.685 (−3.3pp), `numeric_code` collapsed 18%→3%, gate NO-GO(8) vs NO-GO(7).
  Verdict + mechanism: `output/mining-factory/coord-only/ac06_hierarchical_verdict.md`. The mechanism
  is decisive and **applies to any output-head change**: *"Splitting the OUTPUT head does not fix
  interference that originates in the SHARED REPRESENTATION."*

- The proposed **two-stage / abstaining head** is architecturally different in exactly the way that
  matters: it adds a **calibrated commit-to-leaf-vs-stay-loose gate** that can output a **non-leaf
  (coarse / "loose") label**. That is the whole point — the 249 over-tighten columns are ones where
  gold is a loose "no tighter type fits" label and the flat softmax picks a tighter attractor
  (`RESEARCH.md` § 47–56). An abstaining gate lets the model **decline the tight leaf and keep the
  loose node**, removing that residual from flat-softmax competition. A tree-softmax cannot; it must
  pick a leaf.

**Why the falsified hierarchical result does not pre-kill this pilot, but does set the bar:** the
hierarchical failure was interference in the shared trunk leaking through a still-forced leaf choice.
The abstaining gate's escape valve is *not choosing a leaf at all*. But the same trunk-interference
risk means the load-bearing check below (recover-vs-relocate) is mandatory, not optional.

---

## 1. Current architecture (verified against the live tree, 2026-07-11)

- **Head is Flat.** `MultiBranchConfig::default()` → `head_type: HeadType::Flat`
  (`crates/finetype-train/src/multi_branch.rs:133`); `HeadType` has only `Flat` (default) and
  `Hierarchical` (`multi_branch.rs:54–60`). `MultiBranchModel::new()` builds a single
  `Linear(merge_hidden[1] → n_classes)` head (`multi_branch.rs:343`), stored as
  `head: Option<Linear>` (`multi_branch.rs:322`).
- **Trunk output the head reads:** `forward_trunk(...)` returns `hidden` of width `merge_hidden[1]`
  (= **500**, default at `multi_branch.rs:130`). `forward()` (`multi_branch.rs:668`) is where the
  head applies:
  ```
  if let Some(ref head) = self.head { head.forward_t(&hidden, false) }        // Flat: logits
  else if let Some(ref hier) = self.hierarchical { hier.forward(&hidden, n) } // tree-softmax probs
  ```
- **Shipped default** `m2v8m-s43`: dual encoder, 244-label, Flat head, Sense **0.522** / composed
  **0.791–0.812** (`RESEARCH.md` § 27, 33–40).

---

## 2. The architectural change (exactly where it attaches)

Add a third `HeadType` and a head module that sits **at the same seam as the Flat head** — it reads
`hidden` (dim 500) and produces the final class distribution. The trunk (`build_trunk`,
`forward_trunk`) is **unchanged**: one variable, the head, so the pilot is honest.

**New:** `HeadType::TwoStage` (name it `Abstaining` if preferred; `TwoStage` is clearer about the
mechanism). Add to the enum at `multi_branch.rs:54–60` and the `--head` match at
`cmd_train.rs:37–40` (`"two-stage" => HeadType::TwoStage`).

**New module** `TwoStageHead` (co-locate with `HierarchicalHead` in `char_cnn.rs`, or a new
`two_stage.rs` in `finetype-model`), reading `hidden: [B, 500]`, producing:

1. **Coarse head** `Linear(500 → K_coarse)` — K_coarse = the label's family/domain nodes
   (reuse `HierarchyMap` from `char_cnn.rs:141` to derive coarse indices from the sorted
   `domain.category.type` labels; this is the ONE piece worth reusing from the hierarchical code).
2. **Leaf head** `Linear(500 → n_classes)` — the existing flat leaf logits.
3. **Commit gate** `Linear(500 → 1)` (scalar, sigmoid) — the calibrated
   commit-to-leaf-vs-stay-loose decision, trained with a target derived from *whether gold is a leaf
   or a loose node*. At inference: `g = σ(gate)`; if `g ≥ τ_commit` emit `argmax(leaf logits)`,
   else emit the **coarse node** (`argmax(coarse) → its loose representative label`, e.g. the
   family's residual/`unknown`-adjacent node). `τ_commit` is **calibrated on a held-out slice**, not
   hard-coded — this is the "calibrated" in the research line.

**Wiring:** mirror `new_hierarchical` — a `MultiBranchModel::new_two_stage(config, labels, vb)`
constructor (`multi_branch.rs` near :364) that builds the trunk once and attaches the new head;
extend `forward()` (`multi_branch.rs:686`) with the third arm; add a `forward_levels`-style method
for the multi-part training loss (coarse CE + leaf CE + gate BCE). Training-loop plumbing lives in
`train_multi_branch` (called from `cmd_train.rs:357`).

**Scope flag:** this is a real code change to `finetype-train` + `finetype-model` + `finetype-cli`
(not a config-only pilot). It must clear the house bar (`cargo fmt`, `clippy -D warnings`,
`cargo test`, taxonomy `check`) before the training run. Audit-before-edit: run
`codegraph_impact` on `MultiBranchModel::forward` and `HeadType` before touching them.

---

## 3. Single-seed training command

Reuse the `overnight_potion.sh` machinery verbatim except (a) one seed and (b) the new head. The FTMB
and gold FTMB are **already built** by prior potion runs (`output/multibranch-training/m2v8m-244.ftmb`,
`output/embed-frontier/gold_m2v8m.ftmb`) — the pilot reuses them, so no rebuild cost.

```bash
# One-off: the two-stage head must be built into the binary first.
cargo build --bin finetype -p finetype-cli --no-default-features --features model2vec --release
cargo build --bin predict_multibranch -p finetype-train --no-default-features --release

BIN=./target/release/finetype
FTMB=output/multibranch-training/m2v8m-244.ftmb          # reuse (already built)
OUT=models/m2v8m-twostage-s42

"$BIN" train-multi-branch \
  --data "$FTMB" \
  --output "$OUT" \
  --model-config models/m2v8m-244-config.json \
  --seed 42 \
  --head two-stage \            # NEW value (§2)
  --patience 15
```

Or, once `--head two-stage` exists, drive it through the existing overnight harness for a single
seed (it already reads `OVERRIDE_SEEDS` and does the FTMB gate + `type_index_keys` injection + gold
FTMB build + Sense/composed scoring for you):

```bash
OVERRIDE_SEEDS="42" ./scripts/overnight_potion.sh --tag m2v8m-twostage
# (after adding `--head two-stage` to the train invocation at overnight_potion.sh:132-134,
#  which currently pins `--head flat`.)
```

Prefer the harness path — it already does composed scoring, which §4 requires.

---

## 4. Eval protocol — COMPOSED, not just raw Sense

RESEARCH.md is emphatic: **composed is the product number** (0.812) and it is **rule-bound** — a
raw-Sense win on the over-tighten/numeric mass is *largely redundant* because Sharpen already owns
loose-vs-tight (latitude Sense 0.18 → composed 1.00). So a raw-Sense-only read would mis-grade this
pilot in both directions. Score both, decide on composed.

1. **Raw Sense** (diagnostic): `predict_multibranch --model $OUT --data
   output/embed-frontier/gold_m2v8m.ftmb` → score with `scripts/score_gold_anchor.py score` (the
   offline potion path; overnight_potion.sh:170–195 does exactly this). Baseline to beat:
   **0.522** (m2v8m-s43).
2. **Composed** (the decision number): pipe raw Sense preds through
   `scripts/compose_predictions.py` (applies the Sharpen layer), then score. Baseline: **0.791**
   (s43 composed, same instrument). The potion pilot scores from the pre-built gold FTMB, so use the
   `predict_multibranch → compose_predictions → score` chain the overnight script already wires
   (overnight_potion.sh:186–192).
3. **Representative band (advisory):** `score_gold_anchor.py … --reframe` on
   `eval/repr/representative_corpus.tsv` (v19 baseline 0.691) — advisory flag fires if the candidate
   drops > CI (~6pp) below baseline. Not blocking.
4. **Corpus-honest gate (BLOCKING, H05):** `scripts/corpus_honest_gate.py` on the 33k stratified
   sample. This is the ONLY relocation detector and it is where the hierarchical head died. A NO-GO
   here is blocking regardless of the gold headline.
5. **Drift proxy (pre-registered read):** already implicit if run through the overnight harness.

---

## 5. THE load-bearing check — does it RECOVER the over-tighten mass or merely RELOCATE it?

This is the single unknown (`RESEARCH.md` § 108). **Six prior additive retrains all relocated** the
attractor rather than removing it (v22/v23/v24/latdec/coord-blend/coord-hier). The gold headline
alone cannot tell recover from relocate — a candidate can gain on the 249 over-tighten columns while
lighting a fresh attractor on untargeted columns the gold corpus never samples. How to tell:

**Direct measurement on the over-tighten subset (the 249 columns).** Reconstruct the over-tighten
gold subset from `RESEARCH.md` § 47–56 decomposition (loose gold → tighter attractor: word 68,
plain_text 32, alnum 24, integer→binary 52, →numeric_code 32, →year 17, lat/lon→decimal 68). For
each, classify the pilot's prediction:
- **RECOVERED** — the gate abstained and the column resolved to its **loose gold label** (or a coarse
  node that Sharpen then composes to gold). Count these.
- **RELOCATED** — the gate still committed, but to a **different** tight leaf (a new wrong attractor),
  or the same one. Count these.
- Net = RECOVERED − new-relocations. **Recovery requires net positive AND no fresh attractor
  elsewhere.**

**The elsewhere check is the corpus-honest gate (§4.4).** Recover-vs-relocate is not decidable from
gold alone precisely because gold is curated-hard and small: the relocation can land on the untargeted
mass. The gate's `over_emit` / `collapse` / `oracle_fp` bands on the 33k sample are the instrument
that catches a fix that *moves* error rather than *removes* it. **Both must agree**: over-tighten
subset net-positive on gold AND corpus-honest GO. Choice 0104 makes the corpus-honest gate a
**gold-adjudicated relocation review** for model swaps (it is structurally unpassable as a hard GO by
any retrain — 0% pass rate, it measures deviation from v19), so the read is: clean bands, or an
explicit relocation review that shows the abstentions land loose and no untargeted boundary explodes.

**One-line test:** *"On the 249 over-tighten columns the gate should turn `commit→leaf` into
`abstain→loose` and those columns should score gold; and the 33k corpus-honest sample should show NO
new over-emission. Recover = both true. Relocate = gold up but a fresh attractor lights in the gate."*

---

## 6. Expected wall-clock

Single seed, patience-15 early stop, reusing the pre-built FTMB (no data-build cost). The overnight
harness runs 3 seeds × 100 epochs (early-stopped); a single seed is ~1/3 of that plus the one-time
compile of the new head. **Confirm the exact per-seed figure from a prior potion log's epoch
timestamps** before committing to a slot — the archived logs are TUI-rendered (ANSI), so read the
per-epoch `Time` column, not a grep. Working estimate: **~3–5 h wall** for the seed on the training
box, + the compile/verify pass. Gold + composed scoring is minutes. Corpus-honest gate on the 33k
sample is the long pole of the eval (tens of minutes to ~1 h). Budget **half a day** end-to-end for
the pilot verdict.

---

## 7. Go / No-Go read (pre-registered)

- **GO to the 3-seed burn** iff **all** hold:
  1. Composed ≥ 0.791 (no regression vs s43) — this is a Sense-lever pilot, composed must not drop.
  2. Over-tighten subset (§5) **net-positive RECOVERED** with the gate's `abstain→loose` visibly
     doing the work (not a leaf-attractor reshuffle).
  3. Corpus-honest gate **GO** (or explicit gold-adjudicated relocation review passing).
  4. Raw Sense ≥ 0.55 (toward the 0.60–0.66 projection; a raw-Sense lever that doesn't move raw
     Sense has failed on its own terms).
- **NO-GO / stop** if: composed regresses, OR the over-tighten mass RELOCATES (gate NO-GO with gold
  up), OR a fresh cross-domain attractor lights up (the hierarchical-head failure signature —
  `user_agent`/`calver`/`docker_ref` destabilising). A NO-GO here bars the 3-seed burn; it does not
  bar re-scoping the gate target (τ calibration, coarse-node granularity) for one more pilot.
- **Grey (single-seed noise):** a marginal composed within CI is a single-seed artifact — do NOT
  promote on it; the 3-seed burn exists to separate marginal candidates. The pilot's job is to kill
  a relocating design cheaply, not to bank a win.

---

## 8. Cross-reference — move-3a (logit-adjust) is the cheap precursor

`predict_multibranch` already implements `--logit-adjust <tau> --priors <file>`
(`crates/finetype-train/src/bin/predict_multibranch.rs:44,65,95–113`): post-hoc
`logit_c -= tau·log(prior_c)`, down-weighting frequent-class attractors **at predict time, no
retrain**. Move-3a A/Bs `tau ∈ 0.5–1.0`. It is the **same over-tighten mass** this head targets,
attacked with zero training cost. **Run move-3a first.** Reads:
- If logit-adjust alone recovers a slice of the over-tighten mass **and composed holds**, it lowers
  the bar this retrain must clear (the head only needs to beat the free predict-time fix).
- If logit-adjust *relocates* (composed drops, gate NO-GO), that is early evidence the mass is a
  **representational** overlap the softmax can't be re-weighted out of — which is exactly the
  hierarchical-head failure mechanism (§0) and a yellow flag that the head will relocate too. Treat
  a logit-adjust relocation as a reason to sharpen §5's instrumentation before spending the retrain.

Note the RESEARCH.md caveat (§ 100–104): numeric-range stats into the GATE only (never a naive
`COLUMN_STATS_DIM 27→44` bump — that is the cdist 0.316 collapse). If the pilot later feeds
value-range features, feed them to the **commit gate**, not the leaf softmax.

---

## Substrate / provenance

- Head is Flat: `crates/finetype-train/src/multi_branch.rs:54–60,133,322,343,686`.
- HierarchicalHead ≠ this (tree-softmax, no abstain): `crates/finetype-model/src/char_cnn.rs:331`;
  falsified: `output/mining-factory/coord-only/ac06_hierarchical_verdict.md`.
- `--head` CLI: `crates/finetype-cli/src/cmd_train.rs:37–40`; `main.rs:374` (`default_value="flat"`).
- Training harness: `scripts/overnight_potion.sh` (train :132–134, compose+score :170–195).
- logit-adjust: `crates/finetype-train/src/bin/predict_multibranch.rs:44,65,95–113`.
- Error decomposition + projection: `output/next-train-research/RESEARCH.md` § 47–70, 106–113.
