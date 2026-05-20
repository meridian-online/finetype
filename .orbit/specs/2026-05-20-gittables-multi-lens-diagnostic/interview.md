# Discovery: Gittables multi-lens corpus diagnostic

**Date:** 2026-05-20
**Interviewer:** Claude / `orb:discovery`
**Memo:** `.orbit/memos/2026-05-20-gittables-ydf-research.md`
**Cards (candidate parents):** `0014-profile-validate-precision`, `0002-semantic-type-detection`
**Mode:** discovery

---

## Context

Author proposal: train a YDF (Yggdrasil Decision Forests) model on the FineType taxonomy and run it across the gittables corpus, as a path toward measuring accuracy and possibly a supervised-learning approach.

Initial interviewer pushback in the memo: model agreement is not ground truth. Two models trained on the same labels disagreeing tells us which columns are ambiguous, not which prediction is correct. Cheaper first move: use gittables' DBpedia / Schema.org annotations as weak ground truth where they overlap the FineType taxonomy.

**Author reframe during discovery:** YDF is **toolset expansion**, not architecture replacement. DBpedia / Schema.org are **navigation aids**, not competing taxonomies — they help locate columns of known semantic class so we can ask "does FineType cover this and does Sense predict it correctly?" FineType's taxonomy stays canonical. "Iron sharpens iron": independent lenses (Sense, YDF, mechanism cascade, DBpedia annotations) pointed at the same corpus surface gaps in the FineType pipeline.

**Strategic posture surfaced in Q7:** this work **supersedes all other in-flight FineType work** and becomes the eval-expansion milestone — potentially absorbing m-19 (Phase A+B).

---

## Q&A

### Q1: Primary outcome

**Q:** What's the primary outcome you want from this research?

**A:** Diagnostic findings + training data. Significant prior work already exists evaluating performance against the gittables corpus — the autonomous-type-inference spec at `2026-05-04-autonomous-type-inference/`, its 200-row hand-labelled `labelled_eval.tsv`, and the mechanism-attribution cascade in MADR 0075. New work must engage with the existing scoring methodology, particularly the cascade vocabulary used to improve the `validate` function — not build a parallel track.

### Q2: Integration with existing scoring stack — reframed

**Q:** Where does this work plug into the existing scoring stack (offered: scale labelled eval, parallel diagnostic track, Phase 3 triangulator signal, training pipeline only)?

**A:** Pushed back on the framing. "DBpedia / Schema.org annotations" as ground truth read like a competing taxonomy import — not the author's intent. Intent: expand toolset to map the gittables corpus, in an iron-sharpens-iron pattern. End product is a finely tuned FineType inference pipeline. Expects this work to find gaps and issues in JSON schema (taxonomy + validators) and model structure along the way. YDF is a tool; DBpedia is a navigation aid; FineType taxonomy is canonical.

### Q3: Scope

**Q:** How far does this work go before it stops — surface gaps, or also act on them?

**A:** **Surface + rank gaps.** Produce a ranked list of gaps (taxonomy-missing types, validator-too-narrow cases, model-error patterns) with prevalence and evidence from gittables. Each ranked gap becomes a candidate spec or card for downstream work. Fixes happen in subsequent specs; this research is the diagnostic.

### Q4: Corpus slice

**Q:** Which slice of gittables does the research operate on?

**A:** **Full corpus pass.** Lenses pointed at every gittables table. Maximal coverage; finds long-tail gaps. Aggressive ranking required to filter signal from noise at scale.

### Q5: Output shape

**Q:** How are gaps filtered and organised in the diagnostic output?

**A:** **Mechanism categories + corroboration filter.** A gap must be flagged by ≥2 independent lenses (Sense+YDF, Sense+DBpedia, YDF+cascade, etc.) to enter the report. Corroborated gaps grouped by MADR 0075 mechanism category (taxonomy-missing, validator-too-narrow, code-vs-canonical, misclassification, etc.); ranked by affected-column-count within category.

### Q6: Done criterion

**Q:** What's the done-criterion for the research itself (independent of downstream fixes)?

**A:** Top-N per mechanism category is fair, but the **two-criterion gate from `scripts/gittables_gate.py`** is more analyst-useful:

- **(a) ≥ 80% of columns predicted as non-trivial type** — where "trivial" = `representation.text.plain_text` or `representation.numeric.decimal_number`
- **(b) ≤ 1% of rows rejected** on non-trivial columns

Criterion (a) failures map to taxonomy gaps / fallback overuse / under-confidence. Criterion (b) failures map to validator-too-narrow or model-error. The deliverable: mechanism-cascade decomposition of failures under each criterion, at full corpus scale; top-N corroborated gaps within each (criterion × mechanism) cell.

### Q7: Priority relative to in-flight work

**Q:** How does this research relate to m-19 eval-expansion (Phase A+B), v19 retrain gate, autonomous inference module Phase 2?

**A:** **This work supersedes all other work in FineType.** It becomes the basis for all future design improvements. Likely becomes the eval-expansion milestone itself, absorbing m-19. Should be the only active spec.

---

## Summary

### Goal

Build a **multi-lens corpus diagnostic** that runs the FineType pipeline (Sense, validators, mechanism cascade) plus independent lenses (YDF, DBpedia / Schema.org annotations) across the full gittables corpus, and surfaces ranked, corroborated gaps in FineType's taxonomy, validators, and model. Diagnostic outputs feed downstream specs that fix the surfaced gaps. The work supersedes m-19 and becomes the eval-expansion milestone.

### Constraints

- **FineType taxonomy is canonical.** DBpedia / Schema.org annotations are navigation aids — they identify columns of known semantic class so we can measure whether FineType covers and correctly predicts the column. They are NOT imported as type definitions.
- **YDF is tooling, not architecture.** It runs alongside Sense, not as a candidate replacement. Decision 0041 (multi-branch as Sense implementation) stays.
- **Corroboration filter against noise.** A gap must be flagged by ≥2 independent lenses to enter the report. Single-lens artefacts are filtered out.
- **Mechanism-cascade vocabulary preserved.** Outputs use MADR 0075's 4-bucket / 6-trigger taxonomy plus the extended inference-module tokens from MADR 0081 (`validator_widening`, `prediction_confirmed`, `unknown_no_fit`, `fallthrough`, etc.).
- **Python is acceptable for offline research and tooling.** YDF runs offline; any result that bleeds into runtime must respect FineType's zero-Python-at-runtime policy.
- **This is the only active spec.** v19 retrain gate stays enforced; m-19 deliverables either fold into this work or wait; Phase 2 inference signal work pauses or rolls in.

### Success Criteria

- Full gittables corpus measured under the two-criterion gate from `scripts/gittables_gate.py`:
  - (a) `non_trivial_pct ≥ 0.80` per file
  - (b) `reject_rate_non_trivial ≤ 0.01` per file
- For each criterion, mechanism-cascade decomposition of failures (per MADR 0075 + 0081 vocabulary).
- For each (criterion × mechanism category) cell, top-N corroborated gaps surfaced with: affected-column-count, sample evidence, recommended action class, candidate spec/card link.
- Multi-lens corroboration: each surfaced gap is flagged by ≥2 independent lenses (Sense, YDF, DBpedia annotation, mechanism cascade).
- Reproducibility: same corpus + same lens versions → same ranked gap list (mod per-file errors).

### Decisions Surfaced

These need MADR records during or after the spec phase:

- **D1 — Multi-lens corpus diagnostic supersedes m-19 eval-expansion.** This work absorbs and replaces m-19 (Phase A+B) as the eval-expansion milestone. m-19's three deliverables (realism standard + pre-screen, coverage floor, train/eval leakage firewall) fold into the diagnostic's design — leakage prevention especially, since full-corpus measurement requires train/eval separation. (Author intent: "this should be the only active spec we pursue.")
- **D2 — YDF as toolkit member, not architectural alternative.** YDF joins Sense + mechanism cascade + DBpedia annotations as one of several independent lenses. Not a candidate Sense replacement. Decision 0041 stands.
- **D3 — DBpedia / Schema.org as navigation aids, not taxonomy imports.** Used to locate columns of known semantic class for measurement; never imported as type definitions. FineType's taxonomy remains canonical.
- **D4 — Multi-lens corroboration filter for noise control at corpus scale.** A gap requires ≥2 independent lens agreement to enter the report. Single-lens signals are noted but don't surface as ranked gaps.
- **D5 — Two-criterion gate as analyst-useful done-metric.** Top-N per mechanism category is fair, but the (`non_trivial_pct ≥ 0.80`, `reject_rate_non_trivial ≤ 0.01`) decomposition is what analysts can act on. Both reported; the gate criteria are headline.
- **D6 — Full corpus pass, not sampled.** Maximal lens coverage; aggressive corroboration filter handles noise. Computational cost accepted.
- **D7 — Research-only scope; fixes downstream.** This spec produces the diagnostic. Validator widening, taxonomy additions, training-corpus contributions live in subsequent specs.

### Implementation Notes

Means-level observations — starting context for the implementing agent:

- **Existing gate harness:** `scripts/gittables_gate.py` already implements the two-criterion measurement against a 2,000-file holdout. Extending it from holdout to full corpus is a scale + storage problem, not a logic problem. The script's `FileOutcome` dataclass + JSON summary shape is the natural per-file primitive.
- **Trivial type set:** `representation.text.plain_text`, `representation.numeric.decimal_number` (frozenset in the gate script). Lock in as the canonical "Sense was generic" indicator.
- **Mechanism vocabulary:** MADR 0075 (4 buckets, 6 trigger paths) extended per MADR 0081 (10 closed tokens: `format_diversity_path_a/b`, `code_vs_canonical_path_a/b`, `enum_overfit`, `misclassification`, `prediction_confirmed`, `validator_widening`, `unknown_no_fit`, `fallthrough`). Use these as-is; do not invent new tokens without a MADR.
- **Labelled-eval anchor:** `.orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.tsv` (200 rows, hand-labelled per the labelling_protocol rubric) is the precision-on-labelled anchor. Multi-lens corroboration validates against this set.
- **DBpedia annotation source:** gittables tables ship with DBpedia and Schema.org column annotations as part of the corpus. No external API call required. Mapping FineType taxonomy ↔ DBpedia / Schema.org class is a deliverable of the spec.
- **YDF feature design:** YDF needs tabular features per column. Reuse the existing multi-branch Sense feature primitives where possible (char n-grams, embed, stats, header, validation) — different model, same feature shape. Decision 0041 architecture serves as feature reference. **Independence caveat below in open question 4.**
- **`failure_log.tsv` integration:** the existing 21,789-row `eval/gittables/failure_log.tsv` is a subset of what the full corpus pass will produce. Schema-compatible output (`cycle_id`, `file_path`, `file_content_sha256`, `column_name`, `predicted_type`, `observed_values_sample`, `inferred_correct_type`, `mechanism`) keeps existing tooling working.
- **Storage shape:** at full corpus scale, per-column outputs across millions of tables run to tens of millions of rows. Parquet on disk, DuckDB for aggregation queries, is the path of least resistance. The existing gate script already emits per-file JSON; converting to Parquet sidecar is mechanical.
- **Leakage prevention** (folding in MADR 0056): train/eval split via `file_content_sha256 MOD 2` (already in `scripts/split_failure_log.py`) extends to the full corpus. Lenses run on the measure half; calibration on the calibrate half.

### Open Questions

Intent-level only — implementation questions resolved in spec phase.

1. **m-19 deliverable disposition.** Author said "perhaps this work becomes the eval-expansion milestone." Three m-19 deliverables (realism standard + pre-screen, coverage floor 240/240, train/eval leakage firewall) need explicit fold-in / wait / drop status in the spec. Recommended: fold leakage firewall in (D7 implies it), absorb coverage floor (full corpus pass naturally covers it), realism standard becomes the corroboration filter design — but spec phase should confirm.

2. **v19 retrain gate disposition.** Decision 0066 holds v19 as a hard retrain gate. The supersedes-all-other-work framing implies v20 promotion waits for this diagnostic, but doesn't explicitly say so. Spec phase should pin whether v19 stays enforced indefinitely or until top-N model-error gaps are addressed.

3. **Phase 2 inference module relationship.** Decisions 0083–0086 (Phase 1 signal scope lock, signal addition, weight lock) describe the autonomous-inference module's progression. If this diagnostic supersedes all other work, Phase 2's empirical sweep may pause or fold in. Spec phase decides.

4. **YDF feature scope and lens independence.** Reusing all five Sense branches' features (char / embed / stats / header / validation) makes YDF a near-replica of multi-branch with a different head — collapsing lens independence and weakening the corroboration filter (D4). Using only a subset (e.g. stats + header) makes YDF a genuinely independent lens. The spec phase prescribes the feature set; the corroboration filter assumes independence.

5. **DBpedia / Schema.org overlap fraction.** Unknown a priori. If the overlap is small (<20% of columns have a mappable annotation), DBpedia's diagnostic value is limited and the multi-lens design leans on YDF + mechanism cascade. A small spike (measure overlap on a sample) precedes full design lock.

---

**Next step:** `/orb:spec` to generate a structured specification from this discovery.
