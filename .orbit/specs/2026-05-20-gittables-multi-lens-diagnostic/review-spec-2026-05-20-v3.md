# Spec Review

**Date:** 2026-05-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-05-20-gittables-multi-lens-diagnostic
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 5 |
| 2 — Assumption & failure | content signals (training data, eval datasets, cross-system DBpedia/YDF, leakage firewall) + MEDIUM Pass 1 findings | 3 |
| 3 — Adversarial | not triggered — Pass 2 findings are localised, not structurally unsound | — |

## Findings

### [HIGH] ac-02 strata claim contradicts the on-disk topic count by an order of magnitude
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-02 says the 100-table sample is stratified by directory name under `/Users/hugh/datasets/gittables/` and parenthetically claims "~1.1k topic directories." Empirically the corpus has 94 top-level directories, not ~1.1k. The arithmetic of the cap (`≤2 tables per topic`) on a 100-table proportional-to-size sample then breaks down: with 94 topics and a 2-cap, the maximum sample size is 188 (above 100, so the cap is not binding for the sample-size target itself), but "proportional to topic size" against ~1.02M parquets unevenly distributed across 94 topics will hit the 2-cap for every large topic, collapsing the sample toward uniform-over-topics. The threshold gate `overlap_fraction ≥ 0.20` (which decides whether ac-09 enters downgrade mode) is sensitive to which topics get sampled — so an implementing agent who follows the spec's "~1.1k" claim and finds 94 will either pause, invent a different strata field, or proceed and produce a spike result that is no longer comparable to the spec's framing.
**Evidence:** `find /Users/hugh/datasets/gittables -mindepth 1 -maxdepth 1 -type d | wc -l` returns 94. `find /Users/hugh/datasets/gittables -name '*.parquet' | wc -l` returns 1,018,286. spec.yaml ac-02 description says "~1.1k topic directories".
**Recommendation:** Update ac-02's parenthetical to "~94 topic directories under `/Users/hugh/datasets/gittables/`". Then re-examine whether "proportional to topic size, capped at ≤2 per topic" still produces a useful spike — with 94 topics and a 2-cap, the cap-binding will make most large-topic samples uniform, which may or may not be desired. If the intent was a diverse sample, raise the per-topic cap (e.g. ≤3 per topic, allowing ~282 from large topics if proportional weighting selects them); if the intent was uniform-over-topics for spike economy, drop the proportional language and say "sample 100 tables, one or two per topic, weighted uniformly across the 94 topic directories."

### [HIGH] ac-09 cluster-key components `taxonomy_signature` and `value_shape_signature` are undefined
**Category:** test-gap
**Pass:** 2
**Description:** ac-09 defines candidate gaps as "clusters of columns sharing the same `(mechanism_token, taxonomy_signature, value_shape_signature)` triple." Neither `taxonomy_signature` nor `value_shape_signature` is defined anywhere in the spec. `mechanism_token` is well-defined (the closed 10-set from MADRs 0075/0081). `taxonomy_signature` could be `predicted_type` from `finetype profile`, the `x-finetype-label`, or something derived from sibling columns. `value_shape_signature` could be a regex, a character-class histogram, a length distribution, a token shape (e.g. `\d{4}-\d{2}`), or something else. Different definitions produce different cluster boundaries — fine-grained clusters mean more candidate gaps with lower `affected_column_count`; coarse-grained clusters mean fewer gaps with higher counts. The top-10 ranking inside each (criterion × mechanism) cell is dominated by `affected_column_count`, so the cluster definition is load-bearing for `report.md`'s final shape.
**Evidence:** spec.yaml ac-09 description names the triple but doesn't define the second and third components. They appear nowhere else in the goal, constraints, or other ACs. The goal's GapEntry definition names `gap_id` as "SHA256 of (criterion, mechanism, sorted affected-column signature)" — note "affected-column signature" (the SHA's content), not the same thing as the cluster-key signatures.
**Recommendation:** Add concrete definitions to ac-09 (or to the goal's domain section). Recommended phrasing: "`taxonomy_signature` is the column's `x-finetype-label` (Sense's prediction). `value_shape_signature` is the SHA256 of the sorted set of distinct character-class patterns extracted from the column's `sample_values` (each value mapped via `[A-Z]→A, [a-z]→a, [0-9]→9, other→.` then deduplicated)." Either commit to those, or pick alternatives and write them down. The signature definitions must be deterministic so ac-13's byte-identical-output check passes.

### [MEDIUM] ac-09 downgrade-mode verification predicate contradicts the description
**Category:** constraint-conflict
**Pass:** 1
**Description:** ac-09's downgrade-mode description says "the threshold rises to ≥2 over those two non-Sense lenses — i.e. both YDF and cascade must flag." The verification's downgrade query asserts `SELECT COUNT(*) FROM corroborated_gaps WHERE dbpedia_role = 'validation_only' AND corroborating_lens_count < 3 returns 0`. Two readings of `corroborating_lens_count` are possible: (a) it counts only the lenses that vote toward the threshold (i.e. excludes DBpedia in downgrade mode), in which case the verification should be `< 2` (matching the description) — not `< 3`; (b) it counts all flagging lenses including DBpedia's validation-only role, in which case the verification's `< 3` reads "two non-Sense lenses must flag AND DBpedia must agree" — strictly stricter than the description's "both YDF and cascade must flag." The two readings yield different gap admittance rates.
**Evidence:** spec.yaml ac-09 description: "threshold rises to ≥2 over those two non-Sense lenses — i.e. both YDF and cascade must flag." spec.yaml ac-09 verification: `corroborating_lens_count < 3 returns 0`. These are not consistent unless the metric definition changes between modes — but the metric is named the same in both.
**Recommendation:** Pick reading (a). Rewrite the verification's downgrade clause as: "`SELECT COUNT(*) FROM corroborated_gaps WHERE corroborating_lens_count < 2 returns 0` AND `SELECT COUNT(*) FROM corroborated_gaps WHERE dbpedia_role IS NOT NULL AND dbpedia_role != 'validation_only' returns 0` (i.e. when downgrade mode is active, DBpedia never increments the lens count)." This is consistent with the description and removes the namespace ambiguity.

### [MEDIUM] ac-11's `truth_mechanism` vocabulary may not align with the locked 10-token set
**Category:** test-gap
**Pass:** 1
**Description:** ac-11 grades correctness using `recommended_action_class == mapping_table[truth_mechanism]`. The `mapping_table` is the goal's locked 10-token mapping. But `labelled_eval.tsv`'s `truth_mechanism` column (from `2026-05-04-autonomous-type-inference/`) was labelled before the closed 10-set was finalised in MADR 0075/0081. A spot-check on the file's head row shows `truth_mechanism = "prediction_confirmed"` (in the set) and the prior `mechanism` column showing `"value-shape"` (not in the closed set). If labelled_eval contains any `truth_mechanism` value outside the closed 10-set, `mapping_table[truth_mechanism]` is undefined and ac-11's correctness check raises a KeyError or silently drops rows. The ≥0.80 precision threshold becomes ungradable without a re-labelling pass.
**Evidence:** `head -2 /Users/hugh/github/meridian-online/finetype/.orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.tsv` shows `truth_mechanism` column. MADRs 0075 and 0081 (post-dating the labelled_eval) define the closed 10-set. No verification clause in ac-11 checks the vocabulary intersection.
**Recommendation:** Add a precondition step to ac-11: "`scripts/audit_labelled_eval_vocabulary.py` enumerates distinct `truth_mechanism` values in `labelled_eval.tsv` and asserts each is in the closed 10-set; any value outside the set is renamed (with a mapping table committed alongside) before ac-11's grading runs." Alternatively, make the renormalisation explicit by writing the mapping into the spec as a sub-table. Either way, ac-11 cannot grade rows it cannot map.

### [MEDIUM] ac-12 spot-check sampling seed is logged in progress.md but not pre-declared in the spec
**Category:** test-gap
**Pass:** 1
**Description:** ac-12 says "3 random gaps are sampled (sampling seed logged in `progress.md`)." Logging after the fact means the spot-checker chooses the seed, runs the sample, and records what was used — fine for audit, but the spec doesn't fix the seed up-front, so the spot-checker could re-roll until a favourable sample appears. This is a small but real reproducibility hole that ac-13 (corpus-pass byte-identicality) does not cover — spot-check sampling is not part of the corpus pass.
**Evidence:** spec.yaml ac-12 description. ac-13 verification covers `columns.parquet`, `mechanism_decomposition.parquet`, and `report.md` byte-identicality — not `spot_check.md`.
**Recommendation:** Either declare the seed in the spec (e.g. "sampling seed = 20260520, matching ac-02's sample_seed") so it's fixed before spot-checking begins, or accept the audit-after-the-fact pattern and add a hash commitment: "the spot-checker emits `spot_check.md.commitment` (SHA256 of the seed + per-cell sampled gap_ids) before running the assessment; commitment is timestamped." First option is simpler.

### [MEDIUM] YDF confidence threshold (≥0.5) for "vote" is picked without justification
**Category:** assumption
**Pass:** 2
**Description:** ac-09 says "YDF votes its top-1 prediction when confidence ≥0.5 (lower-confidence rows do not vote)." 0.5 is a default-feeling number but it controls how many YDF votes enter corroboration and therefore how many gaps enter `report.md`. Lowering to 0.3 produces more YDF votes, more corroborated gaps, and possibly different top-10 rankings. Raising to 0.7 produces fewer. The spec does not say where 0.5 came from — there's no calibration step, no sensitivity check, no MADR. Constraint 6 (the leakage firewall) prohibits tuning thresholds against the measure half, so the threshold must be picked from first principles or from the calibrate half — but the spec doesn't say which.
**Evidence:** spec.yaml ac-09 description. No calibration AC. Constraint 6 forbids tuning against measure-half. Goal does not reference YDF confidence threshold.
**Recommendation:** Either (a) cite a defensible source for 0.5 (e.g. "matches the cascade's default confidence floor, MADR XXXX") in ac-09's description, (b) calibrate against the calibrate half in a new ac-03.5 ("YDF confidence threshold selected on calibrate half by sweeping {0.3, 0.5, 0.7} and picking the value that maximises gap precision against held-out labelled_eval rows"), or (c) accept that 0.5 is arbitrary and add a sensitivity check as an additional AC (rerun ac-09 with {0.3, 0.5, 0.7} and report how many gaps move in/out of `report.md`). Option (a) is cheapest if a precedent exists.

### [LOW] ac-04 has a soft-fail band between target (48h) and ceiling (72h)
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-04 says "target ≤ 48 h" and "if projection exceeds 72 h, the spec is amended." Between 48h and 72h there's no defined action — the run proceeds, the target is missed, and the spec does not say whether that's acceptable, requires escalation, or triggers a soft revision. In practice a 48–72h projection probably proceeds without amendment, but the rule is unstated.
**Evidence:** spec.yaml ac-04 description.
**Recommendation:** Clarify: "Target ≤ 48 h; soft-fail acceptable up to 72 h (run proceeds, target miss noted in `progress.md`); >72 h triggers spec amendment via `revision_note` before ac-06 begins." One sentence resolves the ambiguity.

### [LOW] ac-01 ordering relative to ac-02 is implicit
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-01 says "No code work begins on ac-03+ until this AC closes" — ac-02 is observation-type and not mentioned. ac-02's spike result determines whether ac-09's downgrade clause applies, so ac-02 logically runs before ac-09 starts but could in principle run before, after, or in parallel with ac-01. The spec doesn't say.
**Evidence:** spec.yaml ac-01 verification ("No code work begins on ac-03+ until this AC closes").
**Recommendation:** State the order explicitly. Recommended: "ac-01 (disposition MADR) → ac-02 (DBpedia overlap spike) → ac-03..ac-13 (in implementation order). ac-02 may run in parallel with ac-01 only if the spike does not consume gittables resources reserved for the full corpus pass." Clarification, not blocker.

---

## Honest Assessment

The spec has absorbed the v2 review's findings well — ac-05 now correctly describes KV-metadata extraction (no more sidecar-file mistake), `partition_seed` is gone, the action-class enum is declared, ac-11 has a fixed flag predicate, and the runtime budget has been re-scoped. The structural backbone — Sense + YDF + DBpedia + cascade as four independent lenses, ≥2-lens corroboration, mechanism × criterion decomposition, reproducible-by-construction `corpus_pass_id` — is sound. What blocks approval are concrete fixable problems concentrated in three places. First, ac-09's cluster key names two undefined signatures (`taxonomy_signature`, `value_shape_signature`); without their definitions, two reasonable implementing agents will produce different gap counts and different top-10 rankings. Second, ac-02's "~1.1k topic directories" claim is wrong by an order of magnitude (actual: 94), which changes what "stratified" means and what the per-topic cap does. Third, the downgrade-mode verification predicate in ac-09 (`< 3`) does not match its description ("≥2 over two lenses"), which silently changes downgrade behaviour. Fix the cluster-signature definitions first — it is the only finding that propagates into ac-13's byte-identicality check, so getting it wrong invalidates reproducibility too.
