# Spec Review

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-dnf
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (shared cycle-worker infra, ground-truth eval set, MADR 0083 architectural constraint, post-mortem evidence in MADR 0084) + Pass-1 MEDIUM/HIGH count | 4 |
| 3 — Adversarial | Pass-2 surfaced an architectural-constraint conflict (sibling-context vs MADR 0083) and a falsifiability gap (ac-01 / ac-02 numbers undefined) — both structural | 2 |

## Findings

### [HIGH] ac-02 "cliff lifts" is not falsifiable as written
**Category:** test-gap
**Pass:** 1
**Description:** The card calls ac-02 "Phase 2's load-bearing 'the signals carry information' evidence". Yet the AC text — "the area under the curve between thresholds 0.5 and 0.7 visibly grows" — has no numeric threshold. "Visibly grows" is not a falsifiable statement: 0.1 pp is technically growth; so is 30 pp. Phase 1's progress.md table (`measure non-unknown rate` 0.204 at 0.5 and 0.071 at 0.7, an AUC over [0.5, 0.7] of roughly 0.014 by trapezoid) gives the implementing agent a concrete baseline number — but the spec doesn't pin a target, so any positive delta passes.

This is the **same failure mode** MADR 0084's "Methodology pass-2 lesson" flagged: a numeric AC ("≥X% on dataset Y") that isn't anchored to a measurement survives review because it's internally consistent. ac-02 here repeats the pattern — it's qualitative ("visibly", "shifted upward") rather than tied to either a prior measurement or an explicit "unbacked target" annotation.

**Evidence:**
- Card line 15 ("then: the area under the curve between thresholds 0.5 and 0.7 visibly grows…")
- MADR 0084 lines 119-124 ("review-spec MUST require either (a) a prior measurement citing the source, or (b) an explicit 'unbacked target' annotation")
- progress.md lines 161-172 — Phase 1's per-threshold curve provides the obvious baseline; spec doesn't reference it
- Bead acceptance line for ac-02 echoes the card prose verbatim

**Recommendation:** Bind ac-02 to a measurable target. Concrete proposal — pick one and write into the spec when it's drafted from this card:
1. "AUC over [0.5, 0.7] on the measure half ≥ 2× Phase 1's baseline (Phase 1 baseline = 0.014 by trapezoid from progress.md table)" — strongest version, falsifiable, anchors to existing data.
2. "non_unknown rate at threshold 0.5 on the measure half rises by ≥10 pp absolute over Phase 1's 0.204" — simpler version, easier to compute from a single threshold cell.

Either makes the AC pass-or-fail. Recommend (2) as the primary metric and (1) as a sidecar diagnostic in progress.md.

### [HIGH] ac-01 ≥60% target is the same unbacked aspiration MADR 0084 just retired
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-01 reinstates the original 60% target as the headline "ship Phase 2" criterion, with branch (b) being "ceiling X% < 60% with structural-cause MADR". MADR 0084 explicitly recorded that 60% was an unbacked aspiration — and yet here it returns as a binary gate without any data justifying that 4 signals can plausibly clear 60% (vs Phase 1's 7%, an 8.5× lift required on the same measure half).

The structural math in MADR 0084 lines 27-35 was specifically about the 2-signal architecture (`score = 0.4·v + 0.6·h`, header-only ceiling 0.6). Phase 2 changes the math (4 signals, weights renormalised), but the spec doesn't quantify what each new signal can plausibly add. Without that, ac-01's branch (a) is a wish, branch (b) is the default outcome, and the empirical decision is foregone before measurement.

This isn't a reason to block the bead — branch (b) is a legitimate ship path (Phase 2 MADR documenting structural ceiling). It's a reason to require the spec to either (i) cite a back-of-envelope estimate per new signal, or (ii) annotate ac-01 as "exploratory target — outcome (b) is the modal expectation, outcome (a) would be a positive surprise" so the team knows what success looks like.

**Evidence:**
- Card line 10 ("≥60% non-unknown at confidence ≥0.7 IS achieved")
- MADR 0084 lines 27-42 (60% was aspirational; 7% is structural ceiling under 2-signal locked weights)
- MADR 0084 line 119-124 (the methodology lesson)
- Card goal line 32 ("If yes — ship… If no — document the structural ceiling")

**Recommendation:** Two options:
1. Add a constraint to the spec requiring a "Phase 2 plausibility memo" before implementation begins — back-of-envelope per signal: how many of Phase 1's failing rows could generator-shape plausibly recover, how many sibling-context. If neither alone reaches ~25 pp lift, the implementation effort is mostly to confirm the structural ceiling, which is fine but should be acknowledged.
2. At minimum, annotate ac-01 with "outcome (b) is the modal expectation given Phase 1's empirical 7% ceiling under 2-signal architecture; outcome (a) requires the new signals to recover 53 pp of structural gap, which is currently uncharacterised". Then the AC passes its branch-(b) path without anyone feeling Phase 2 "failed".

### [HIGH] Sibling-context vs MADR 0083 architectural constraint — unreconciled
**Category:** constraint-conflict
**Pass:** 2
**Description:** MADR 0083 (Phase 1 signal scope lock) recorded an architectural constraint in the "Bad" consequences block:

> "sibling-context as a signal is currently computed for free elsewhere in the runtime (the cycle worker's profile call already runs Model2Vec sibling attention). Phase 1 is leaving that signal on the floor. Mitigated: **the inference module's signals are intentionally independent of the model — one of the architectural claims (MADR 0079) is that the inference module triangulates ON the model's prediction, not WITH the model's internals. Sibling-context attention is part of the model's internals. If a future card moves toward 'inference reuses model internals,' that's a different architectural decision.**"

The card now does exactly this: it adds sibling-context as a Phase 2 signal that "draw[s] on already-loaded Model2Vec embeddings" (card scenario 4 / `given:` block). This is the architectural fork MADR 0083 flagged — the inference module would now triangulate WITH the model's internals.

The card references MADR 0083 (line 40) but doesn't address this specific concern. It treats Phase 2 as a quantitative extension of Phase 1 (more signals, same weight invariant). It's actually a qualitative architectural change that needs an explicit decision.

**Evidence:**
- Card line 23 ("sibling-context drawing on already-loaded Model2Vec embeddings")
- MADR 0083 lines 105-115 (the explicit architectural constraint)
- MADR 0079 lines 70-72 (triangulator's load-bearing claim that signals are independent error sources)
- Card line 40 cites MADR 0083 but only for the weight invariant, not for the model-internals constraint
- `crates/finetype-model/src/column.rs:435,546` — sibling_context is a model-owned attribute (`SiblingContextAttention`), confirming it lives inside model internals

**Recommendation:** Require a new MADR (call it 0085) authored as a spec deliverable, specifically addressing: "May the inference module consume the model's sibling-context attention output, given MADR 0083's claim that signals must be independent of the model?" Considered options should include:
- (A) Yes — re-frame independence as "independent error sources, not independent code paths". Sibling-context attention is run on column-graph topology, not on the predicted_type's validator behaviour, so its error mode is uncorrelated with validator pass-rate even if it shares a Rust crate with the model.
- (B) No — implement sibling-context from scratch in the inference module (separate Model2Vec invocation, separate attention computation). Costs latency and code duplication but preserves MADR 0083's architectural boundary.
- (C) Defer Phase 2 until the architectural call is made, and let Phase 2 ship with generator-shape only as the third signal.

The card's current framing assumes (A) without naming it. The spec must either pick (A) explicitly with rationale or pivot to (B)/(C).

### [MEDIUM] Latency budget — generator-shape signal cost is uncharacterised
**Category:** failure-mode
**Pass:** 2
**Description:** ac-04 inherits Phase 1's <100ms median budget. Phase 1's measured p50 is 70ms (progress.md line 64), leaving 30ms headroom. The spec adds two signals; sibling-context "draws on already-loaded Model2Vec embeddings" so its marginal cost may be near-free (depends on call-site), but generator-shape pulls from `crates/finetype-core/src/generator.rs` (306 KB / 6843 lines per `wc -l`, the largest file in the crate).

Generator-shape conceptually means: for each candidate type T, generate K samples from T's generator and compare to observed samples. Phase 1's validator scan is 240 types × 8 samples = 1920 regex evaluations at ~1µs each = ~2ms. Generator scan would be 240 types × K invocations × generator cost — and generator cost is substantially larger than regex (locale tables, weighted choice, format strings). If generator cost averages 100µs and K=8 to match, that's 240 × 8 × 100µs = 192ms, which alone breaches the budget.

The card mentions the breach as a "Phase 2 finding requiring an architectural MADR before ship" (card scenario 4 `then:` clause), but doesn't pre-empt it with a cost model. Phase 1's spec.yaml lines 442-443 carried an explicit cost-model paragraph; the Phase 2 card and bead description don't.

**Evidence:**
- Card line 25 ("if they don't, the breach itself is a Phase 2 finding requiring an architectural MADR before ship")
- Phase 1 spec.yaml line 443 — explicit cost model: 240 × 8 × ~1µs ≈ 2ms
- `crates/finetype-core/src/generator.rs` 6843 lines (vs validator.rs 1090 lines or so per ls)
- progress.md line 64 (p50 70ms; 30ms remaining headroom)

**Recommendation:** Require the spec to include a cost-model paragraph for both new signals before implementation begins. Concrete proposal:
- Generator-shape: define K (target sample count from each generator). If 240 × K generator invocations × estimated cost > 30ms, the signal needs caching (pre-compute generator fingerprints once at startup) or sub-sampling (only run generator-shape on top-N candidates from validator+header pre-filter). The spec should pick one of these strategies up front.
- Sibling-context: confirm in writing that the cycle worker's profile call has already run sibling attention by the time inference fires, and that the inference module reads cached output rather than re-invoking Model2Vec. If it has to re-invoke, model the cost.

Without this, ac-04 has a high probability of late-cycle breach, and the "architectural MADR before ship" escape clause becomes the modal outcome.

### [MEDIUM] ac-03 "predictable subset of regressions" — verification method undefined
**Category:** test-gap
**Pass:** 2
**Description:** ac-03 asserts that disabling either signal in isolation produces "a predictable subset of regressions, and the auditor can answer 'which signal saved this column?'". This requires:
1. An ablation harness (run inference with generator-shape disabled; run again with sibling-context disabled).
2. A "predictability" criterion — what counts as predictable? If 30% of regressions match expectations and 70% don't, is the AC met?
3. A per-row attribution method (the sidecar must record which signal was decisive for each row).

None of these are sketched in the card. The card hand-waves the per-row attribution as "the sidecar's per-signal score breakdown attributes the lift to a specific signal" — but per-signal scores are an *input*, not an attribution. Attribution requires a counterfactual: "if we'd run without signal X, would this row have stayed below 0.7?".

This matters because ac-03 is the AC that distinguishes "Phase 2 added two signals that work independently" from "Phase 2 added two signals that happen to correlate with validator/header signals and contribute mostly redundancy". The latter is a real risk (sibling-context's signal can correlate with header-name match — both derive from textual cues).

**Evidence:**
- Card line 20 ("disabling either signal in isolation produces a predictable subset of regressions")
- Card line 20 ("the auditor can answer 'which signal saved this column?'")
- ac-12 in Phase 1 spec — sidecar has 12 columns; Phase 2 will add 2 columns (ac-05 in this card). No "decisive signal" column proposed.

**Recommendation:** Spec must define:
1. **Ablation harness:** `bench_infer_floor.py --disable-signal {generator_shape,sibling_context}` produces three result tables (full / no-G / no-S).
2. **"Predictable" criterion:** at minimum, no-G regressions concentrate in shape-driven types (UUIDs, ISO dates, code-formatted IDs); no-S regressions concentrate in neutral-header columns (`x`, `field_3`). The spec should pre-name the type-families each signal is hypothesised to recover, and ac-03 passes if the ablation confirms the hypothesis with ≥X% concentration.
3. **Per-row attribution:** add a `decisive_signal` column to the sidecar (string in `{validator, header, generator_shape, sibling_context, none}`), defined as "the signal whose subscore was numerically largest at argmax". This makes the auditor's question answerable directly without re-running ablation per row.

### [LOW] ac-05 "additive" — schema migration story missing
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-05 says "existing 12-column schema preserved by append" — i.e., the sidecar gains two new columns (`generator_shape`, `sibling_context`) appended at the right. But Phase 1's sidecar already has rows from cycles that ran before Phase 2 ships. Those rows have 12 columns. Phase 2 rows will have 14. The spec doesn't say:
- Are old rows back-filled with empty / null values for the two new columns?
- Are old rows left as 12-column and new rows are 14-column (mixed schema)?
- Is the file rewritten (which conflicts with the contract's H08 append-only invariant cited in Phase 1 spec line 30)?

**Evidence:**
- Card line 30 ("sidecar gains `generator_shape` and `sibling_context` score columns")
- Phase 1 spec.yaml line 30 — H08 append-only invariant
- Phase 1 spec.yaml ac-12 — sidecar is "append-only from creation; no existing append-only file is rewritten"

**Recommendation:** Spec must specify the schema migration strategy. Recommend: introduce `inference_signals_v2.tsv` as a new sidecar (Phase 1 sidecar frozen at 12 columns; Phase 2 cycle worker writes to the v2 file in addition to or instead of v1, picked by deployment cutover). Or: leave Phase 1 rows as 12-col and write 14-col rows from Phase 2 onward, accepting that the file has two schema generations (TSV readers must handle missing trailing fields). Either is fine — the choice has to be made in spec, not at implementation time.

### [LOW] Calibrate-vs-measure leakage path — top-level weights
**Category:** failure-mode
**Pass:** 2
**Description:** Phase 1 locked weights at `w_v=0.4, w_h=0.6` based on interview evidence (MADR 0079), explicitly NOT on a sweep. Phase 2 extends to 4 weights summing to 1.0. The card is silent on whether these new weights are similarly locked (and on what evidence) or selected via sweep (with what discipline against measure-half leakage). Phase 1's calibrate/measure split was specifically constructed to keep weight selection pristine; if Phase 2 sweeps weights, the spec must specify that the sweep runs on calibrate half ONLY and the measure-half number reported in ac-01 is computed once at locked weights.

**Evidence:**
- Phase 1 MADR 0079 lines 92-97 (weight-lock rationale)
- Phase 1 spec.yaml line 27 (locked weights constraint), line 31 (calibration partition)
- Card silent on Phase 2 weight selection

**Recommendation:** Spec must specify Phase 2 weight selection. Two reasonable options:
1. Lock all 4 weights based on first-principles reasoning (similar to Phase 1's "header > validator because evidence shows validators are more error-prone"). Concrete proposal: keep w_v + w_h = 0.5 of Phase 1's 1.0 mass (so 0.2 + 0.3), allocate remaining 0.5 between generator-shape and sibling-context based on which is hypothesised to recover the larger fraction of cliff-cases (likely 0.3 + 0.2 = generator-shape > sibling-context, since generator-shape is a per-value signal and sibling-context is per-column).
2. Sweep on calibrate half, report measure-half number once. Spec must explicitly forbid the implementing agent from looking at the measure half during sweep.

### [LOW] Bead `[gate]` markers absent — none of the 5 ACs are gated
**Category:** constraint-conflict
**Pass:** 1
**Description:** `parse-acceptance.sh acs finetype-dnf` returns `is_gate=0` for all five ACs. Phase 1 spec marked ac-04, ac-05, ac-06 as gates (per its exit_conditions line 498). Phase 2 inherits the same latency / determinism / smoke concerns and a new "cliff lift" empirical gate (ac-02 here). At minimum ac-02 (the falsifiable claim test) and ac-04 (latency budget) should carry `[gate]` markers so the implement skill blocks on them.

**Evidence:**
- `parse-acceptance.sh acs finetype-dnf` output — all 5 rows have `is_gate=0` in column 4
- Phase 1 spec.yaml line 498 — `ac-04, ac-05, ac-06` carry gates
- Card scenarios 1, 2, 4 are the most "load-bearing" (ac-01 floor decision, ac-02 cliff lift, ac-04 latency)

**Recommendation:** When the card is promoted to a spec and the bead's acceptance_criteria field is updated, mark at minimum ac-02 (falsifiable cliff-lift claim) and ac-04 (latency budget) with `[gate]`. Consider also ac-01 (the empirical floor decision) if its branch-(b) MADR ship path is treated as the gating completion criterion.

---

## Honest Assessment

The card has a clear shape — extend Phase 1's triangulator from 2 signals to 4, measure whether the cliff lifts, ship if 60% reached or document a structural-ceiling MADR if not. That structure is sound. The biggest risk is a process risk that MADR 0084 just flagged: ac-01 and ac-02 carry numeric / qualitative claims that aren't anchored to measurement, repeating exactly the failure mode the prior bead's post-mortem warned against. The single highest-leverage fix is binding ac-02's "cliff visibly lifts" to a falsifiable threshold (e.g. "non_unknown@0.5 rises ≥10 pp from Phase 1's 0.204 baseline").

Second-biggest risk is the unreconciled architectural conflict with MADR 0083 — sibling-context drawing on Model2Vec embeddings is the exact "inference reuses model internals" boundary 0083 said requires its own decision. Phase 2 needs that decision authored before implementation, not discovered during it.

Latency and ablation methodology are tractable but currently underspecified; addressing them up front will save a sub-bead during implementation.

REQUEST_CHANGES rather than BLOCK because the card's intent is sound and the gaps are addressable in a spec drafting pass. The card itself doesn't need rework — but the spec generated from it must close the falsifiability, architectural-conflict, and cost-model gaps before the implementing agent starts work.
