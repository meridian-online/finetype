# Spec Review

**Date:** 2026-05-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-05-20-gittables-multi-lens-diagnostic
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | Pass 1 surfaced training-data, eval, cross-system, and reproducibility content signals + 1 HIGH structural finding | 4 |
| 3 — Adversarial | Pass 2 surfaced cascading dependencies between ac-03 (YDF accuracy) and ac-11 (precision floor) plus ac-02→ac-09 path divergence | 2 |

## Findings

### [HIGH] ac-11 precision floor is structurally coupled to ac-03's YDF accuracy bar, and the two thresholds are inconsistent
**Category:** assumption
**Pass:** 2
**Description:** ac-11 requires `precision_on_flagged >= 0.80`. A flagged row's `correctly_diagnosed` is true iff (a) `recommended_action_class == mapping_table[truth_mechanism]` AND (b) `ydf_prediction == truth_inferred_type`. Part (a) is deterministic by construction — the recommended-action-class is derived from the mechanism via the locked table, so `mapping_table[mechanism_token] == mapping_table[truth_mechanism]` reduces to "is the diagnostic's mechanism token correct?" Part (b) is just YDF's top-1 accuracy on the flagged subset. The diagnostic precision is therefore at most `min(mechanism_correctness_on_flagged, ydf_accuracy_on_flagged)`. ac-03 only requires YDF ≥0.70 top-1. If YDF lands at its floor (0.70), ac-11's 0.80 floor is *structurally unreachable* on the YDF-accuracy term alone, and the flagged subset is biased toward Sense-disagreement rows where YDF may do worse than its overall average. Of the 200 labelled_eval rows, 149 (74.5%) are `truth_mechanism = misclassification` — so YDF's accuracy on misclassification rows dominates the precision computation. This is the single biggest hidden risk in the spec: ac-11 can fail through no fault of the diagnostic, purely because YDF is good-enough for ac-03 but not good-enough for ac-11.
**Evidence:** spec.yaml ac-03 verification clause `top1_accuracy >= 0.70`; spec.yaml ac-11 description "A flagged row has correctly_diagnosed = true iff (a) recommended_action_class == mapping_table[truth_mechanism] AND (b) ydf_prediction == truth_inferred_type"; labelled_eval distribution measured at 149/200 `misclassification`, 35 `prediction_confirmed`, 10 `format_diversity_path_b`, 4 `validator_widening`, 2 `fallthrough` (`awk -F'\t' 'NR>1 {print $11}' labelled_eval.tsv | sort | uniq -c`).
**Recommendation:** One of three changes. Pick **option A**: raise ac-03's floor to ≥0.85 top-1 on the labelled_eval subset that maps to the closed taxonomy, so ac-11's 0.80 bar is achievable. If retraining can't clear 0.85, ac-11's bar must drop — and that decision belongs in the spec, not in implementation. Options B and C kept for the author: B — decouple ac-11 from YDF entirely; grade `correctly_diagnosed` on (a) mechanism correctness only (the diagnostic's core claim), with YDF's verdict surfaced but not scored. C — lower ac-11 to ≥0.70 with an explicit note that "precision = YDF accuracy bound × mechanism accuracy."

### [MEDIUM] ac-12 spot-check has no human attestor named and no acceptance criterion for what "sensible" means
**Category:** test-gap
**Pass:** 1
**Description:** ac-12 says a gap passes spot-check if "(c) the `recommended_action_class` is sensible." The recommended-action-class is derived from a locked mapping table — so its "sensibility" is determined upstream by the mapping, not by per-gap judgement. What the spot-check actually needs to judge is whether the mechanism token (which DOES drive the action class) is correctly assigned for the sampled gap. That assessment requires a human reviewer named in the spec (per `ac_type: ops`, the band is "operator action with a captured log line, signoff, or dashboard check"). The spec lists `spot_check.md` will carry "spot-checked-by attestation" but does not name the attestor, the rubric for "sensible," or what counts as a failure beyond per-cell aggregate threshold.
**Evidence:** spec.yaml ac-12 description and verification; `ac_type: ops`.
**Recommendation:** Replace "(c) the `recommended_action_class` is sensible" with "(c) the assigned mechanism_token is correct for the sample evidence (i.e. the column failure pattern matches the closed-set token's MADR-0075/0081 definition)." Name the attestor in the spec body (e.g. "spot-checked by: hescameron@gmail.com" or "human reviewer per session contract"). Add a rubric clause: a gap fails spot-check if any of (a)/(b)/(c) fails; partial failures (1 of 3) count as full failures.

### [MEDIUM] ac-09 lens-vote definition makes Sense vote unconditionally — meaning Sense agreement is invisible to the corroboration count
**Category:** assumption
**Pass:** 2
**Description:** ac-09 says "Sense votes its `x-finetype-label` per column" and "A lens FLAGS a candidate gap iff its vote disagrees with Sense's prediction." If Sense is one of the lenses, it cannot disagree with itself — so Sense never flags. The corroboration threshold ≥2 is then over the pool `{YDF, DBpedia, cascade}` in default mode and `{YDF, cascade}` in downgrade mode. The downgrade mode therefore requires **both** YDF and cascade to flag (a 2-of-2 AND) — exactly as the spec says. This is consistent — but the spec's framing "≥2 of {Sense, YDF, DBpedia, cascade}" reads as if Sense can vote, which obscures the actual pool size. Worse, in downgrade mode there is effectively no corroboration slack — a single noisy YDF prediction or cascade emission becomes a hard signal.
**Evidence:** spec.yaml ac-09 description: lens-vote functions enumerated; "Only candidates with `corroborating_lens_count >= 2` (lenses other than Sense itself that flag) enter `report.md`"; downgrade-mode pool reduced to `{YDF, cascade}`.
**Recommendation:** Two clarifications. (1) Remove Sense from the corroborating-lens enumeration in the goal block — the rule is `corroborating_lens_count` over `{YDF, DBpedia, cascade}` (default) or `{YDF, cascade}` (downgrade). Sense is the **subject** of corroboration, not a voter. (2) Add an explicit AC or progress.md note that downgrade-mode results carry a `corroboration_fragility` flag — single-noisy-lens errors will surface as gaps. This is acceptable but the author should know.

### [MEDIUM] ac-02 spike threshold (0.20) is not justified by anything in the spec
**Category:** assumption
**Pass:** 2
**Description:** ac-02 says: if `overlap_fraction >= 0.20`, full design proceeds; if `< 0.20`, DBpedia degrades to `validation_only`. The 0.20 number has no source — it is not derived from the corpus, not pinned to a memory or MADR, and the spec offers no defence for "20% mappable" as the right line between "lens" and "validator." If the spike returns 0.18, the design path shifts substantially (downgrade-mode is genuinely different — the corroboration pool drops to 2 lenses and effectively becomes an AND). A 2% measurement difference should not flip the whole design.
**Evidence:** spec.yaml ac-02 description and verification; no MADR linked, no historical measurement cited.
**Recommendation:** Either (a) anchor 0.20 to a prior measurement (a memory exists at `gittables-corpus-shape-2026-05-03-1-018` — does it carry a DBpedia annotation rate?), or (b) widen the spike sample to give the threshold statistical headroom (e.g. compute a 95% confidence interval from the 94-table sample, and only downgrade if the *upper bound* of the CI is below 0.20). The current binary threshold on a single point estimate is fragile.

### [MEDIUM] ac-13 reproducibility check uses 100 files from the *measure half* — risk of contaminating measure-half during reproducibility iteration
**Category:** failure-mode
**Pass:** 2
**Description:** ac-13 says: "Tested via a 100-file sub-pass on the measure half." If the reproducibility check is run repeatedly during implementation debugging (until both runs produce byte-identical outputs), the implementer is interacting with the measure half iteratively — *not* to tune thresholds (hard constraint 6 forbids that) but to debug determinism. The boundary between "debugging the runner" and "debugging the corpus pass" is thin. A more disciplined choice would be to run ac-13 on the *calibrate* half (`file_content_sha256 MOD 2 == 0`), which the spec already reserves for descriptive curves.
**Evidence:** spec.yaml ac-13 description; hard constraint 6: "weight tuning, threshold selection, and any parameter influencing ranked-gap selection MUST NOT consult the measure half. Cross-half leakage is a halting condition for spec close."
**Recommendation:** Move the reproducibility sub-pass to the calibrate half. Byte-identical determinism is a property of the runner, not of the corpus partition — calibrate-half files are sufficient. This protects measure-half hygiene from debugging traffic.

### [MEDIUM] ac-08 mechanism-token assignment for ROWS is unclear — mechanisms are per-column in the inference cascade, not per-row
**Category:** failure-mode
**Pass:** 2
**Description:** ac-08 says: "For each criterion-(b) failure (high reject rate), each rejected row is tagged similarly" with a mechanism token. The `finetype infer-type` subcommand operates at the column level — it produces one mechanism per column, not one per row. The cascade in `crates/finetype-core/src/infer.rs` (verified by inspection: `pub fn infer(taxonomy: &Taxonomy, input: &InferInput) -> InferOutput` returns a single `mechanism: String`) takes a column's worth of samples and emits one mechanism. To tag *rows*, the spec implicitly assumes a row-level mechanism attribution that the engine does not natively produce.
**Evidence:** `/Users/hugh/github/meridian-online/finetype/crates/finetype-core/src/infer.rs` lines 86–95 (`InferInput`/`InferOutput` structs) and lines 481–568 (`infer` returns one mechanism per call); spec.yaml ac-08 description "each rejected row is tagged similarly."
**Recommendation:** Clarify ac-08 explicitly: rejected rows inherit the *column's* mechanism token (each rejected row's `mechanism_token` equals the mechanism assigned to its `column_name` in the file). Alternatively, define a row-level mechanism attribution rule (e.g. per-row reject reason → mechanism via a separate mapping) and pin it in this spec. The current language straddles two interpretations.

### [LOW] ac-04 runtime budget is plausible but the cost model omits YDF inference and KV-extraction terms
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-04 sets a 48h target and 72h ceiling at `--jobs 16`. Memory `gittables-corpus-shape-2026-05-03-1-018` records ~1.8s/file profile+validate on M1. Measure half ≈ 509k files. Baseline alone at 16 jobs: 509000 × 1.8 / 16 ≈ 16 hours. Adding YDF inference (Python, model-load + per-column inference) and DBpedia KV-extraction (DuckDB call per file, JSON-parse) will push wall-clock substantially. The 48h target is reasonable but the spec does not bound the *new* cost terms — only the *total*. If the dry-run extrapolates to 70h, the spec says "amend if > 72h" — that leaves a narrow band.
**Evidence:** memory `gittables-corpus-shape-2026-05-03-1-018`; spec.yaml ac-04 description and verification.
**Recommendation:** Add to ac-04 a per-file cost decomposition in the dry-run output: `{baseline_profile_validate_s, ydf_inference_s, dbpedia_kv_extraction_s}`. This makes the dry-run output diagnostic, not just go/no-go.

### [LOW] ac-01 disposition for MADR 0066 (v19 retrain gate) is left open-ended
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-01 says MADR 0066 "stays enforced indefinitely OR until a named milestone in this spec is reached." That OR is unresolved — the spec does not name which of its milestones lifts the retrain gate. A reader cannot determine when v20 promotion becomes possible.
**Evidence:** spec.yaml ac-01 description: "(ii) v19 retrain gate (MADR 0066) — stays enforced indefinitely OR until a named milestone in this spec is reached."
**Recommendation:** In MADR 0087 (the disposition record this AC produces), pick one of: (a) "stays enforced until `report.md` ships per ac-10 and surfaces ≥1 corroborated gap in the `training_data_addition` action class"; (b) "stays enforced for the full duration of this spec; v20 is gated on a separate follow-up spec"; (c) some other named trigger. Don't leave the OR unresolved in 0087.

### [LOW] `candidate_spec_slug` in GapEntry lacks a generation rule
**Category:** test-gap
**Pass:** 3
**Description:** GapEntry's `candidate_spec_slug` field is "string, may be empty." Per "Evaluation principles → Action density," gaps with empty `candidate_spec_slug` rank below those with one. So the slug affects ranking — but the spec gives no rule for *how* the slug is generated. Is it author-supplied? Heuristically derived from `(mechanism, taxonomy_signature)`? Empty by default? The ranking criterion is sensitive to a field with undefined provenance.
**Evidence:** spec.yaml "Domain object: GapEntry" — `candidate_spec_slug` (string, may be empty); "Evaluation principles → Action density" — ranking penalises empty slug.
**Recommendation:** Either (a) define a generation rule (e.g. "empty for the first pass; populated by author during ac-12 spot-check"), or (b) remove the empty-slug ranking penalty so the field is purely advisory.

### [LOW] Pass-1 deterministic gate-AC description check
**Category:** content-signal
**Pass:** 1
**Description:** All 13 ACs have `gate=0` per the parser output — no ACs are marked as gates. The deterministic gate-AC description rules are therefore vacuously satisfied; no findings emitted. Flagged for the record only.
**Evidence:** parser output column 4 = `0` for every AC.
**Recommendation:** None. A spec without gate-marked ACs is permitted.

---

## Honest Assessment

The spec is structurally rigorous — locked vocabularies, deterministic signatures, reproducibility verification, leakage discipline, explicit downgrade path. The big design moves (FineType as canonical taxonomy; DBpedia as navigation aid; corroboration as the noise filter) are sound. What it lacks is internal consistency between two of its quantitative thresholds (HIGH-severity ac-03 vs ac-11) and operational definition of two of its judgement-calls (ac-12 attestor, ac-08 row-vs-column granularity).

**Biggest risk:** the ac-11 precision floor is not under the implementer's direct control — it is bounded above by YDF's accuracy on the flagged subset. ac-03 only guarantees 0.70 top-1 overall; ac-11 demands 0.80 precision. An implementer who clears ac-03 may still fail ac-11 through no fault of the diagnostic design, and have no clear remedy short of retraining YDF. That coupling needs to be made explicit or broken before implementation starts.

**Secondary risk:** ac-02's binary 0.20 threshold flips a substantive design path (downgrade-mode reduces the corroboration pool to 2 lenses, effectively making it an AND) on a single-point estimate from a 94-table sample. Add a confidence interval or pre-measure the rate.

Three changes would clear my objection: (1) align ac-03 / ac-11 thresholds or decouple them; (2) name the ac-12 attestor and tighten the spot-check rubric; (3) clarify ac-08's row-level mechanism rule. The other findings are smaller corrections that could be folded into the same revision cycle.
