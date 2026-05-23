# Spec Review

**Date:** 2026-05-23
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-05-23-ydf-specialist-geography
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content-signal: training data, eval inputs, model retrain | 3 |
| 3 — Adversarial | structural concern: closure of AC-02 depends on out-of-scope retrain | 1 |

The spec touches training data, eval datasets, and model retrain — Pass 2 triggered on content signals alone. Pass 3 triggered by an AC whose verification depends on a v20 training run not scoped inside this spec.

## Findings

### [HIGH] AC-01 has no leakage firewall against `sources.yaml` or holdout
**Category:** missing-requirement
**Pass:** 2
**Description:** The proposed `eval/gittables/v20_training_candidates/geography.tsv` becomes training data for v20's Sense retrain (AC-02). Two leakage paths are unaddressed:

1. **`sources.yaml` is silent on gittables.** MADR 0056 requires every source feeding training to declare `role` ∈ `train` / `eval` / `both-forbidden`. `eval/datasets/sources.yaml` currently contains zero entries for gittables. Adding 2,300+ training rows from gittables without updating sources.yaml violates the leakage-prevention contract by construction — `prepare_multibranch_data.py`'s row-hash filter will not catch them because gittables-derived rows are not in `eval/row_hashes.tsv`.
2. **Holdout overlap is unfiltered.** 16 of the 6,873 candidate columns (`ydf_prediction LIKE 'geography%' AND ydf_confidence >= 0.7`) sit inside `eval/gittables/holdout_paths.txt` — the 2,000-file autonomy-contract gate corpus (memory `autonomy-contract-activation-infrastructure-shipped-2026-05`). If those rows enter training, the v20 gate score is leaked relative to v16's baseline; AC-02's "measurable reduction" becomes unmeasurable as a gate signal.

**Evidence:**
- `grep -rn "gittables" eval/datasets/sources.yaml` returns no matches.
- DuckDB join of candidate file_paths against `holdout_paths.txt` returns 16 overlapping files.
- MADR 0056 line: "Per unique source_url, records: `role` ∈ `train` | `eval` | `both-forbidden`."

**Recommendation:** Add a sub-AC (or amend AC-01) requiring:
- A `sources.yaml` entry for gittables with `role: train` declared *before* the extract is written.
- The extractor filters out rows where `file_path` is in `holdout_paths.txt`.
- The extractor emits row hashes via the shared `eval_leakage` normaliser so the prepare-multibranch filter catches any cross-corpus collision.

---

### [HIGH] AC-01's "Sense-missed" definition is circular when the column is mislabelled in source data
**Category:** assumption
**Pass:** 2
**Description:** The largest "Sense-non-geography vs YDF-geography" bucket (437 of 684 non-geography Sense rows) is `sense=identity.person.email` + `ydf=geography.address.full_address`. Spot-check: headers literally read `email` / `Email Address` / `What is your UWaterloo email address?`, values are US street addresses with state and zip. **Sense is reading the header correctly; the source column is mislabelled by the dataset author.**

If the extractor accepts these as training rows with `candidate_target_label = geography.address.full_address`, v20 learns "header contains 'email' + values look address-shaped ⇒ classify as full_address". That harms generalisation against the vastly larger population of `email` columns that actually contain emails. This is exactly the precision principle in CLAUDE.md inverted — a learned signal that confirms the wrong thing on ~90% of real-world inputs.

The deeper issue: AC-01 treats `Sense ≠ YDF` as a proxy for "Sense is wrong". The corpus diagnostic (m-19) deliberately frames these as *corroborated gaps requiring author review*, not as ground-truth Sense errors. Lifting the disagreement set into training data wholesale collapses that careful epistemics.

**Evidence:**
- DuckDB sample of 5 rows in the `sense=email, ydf=geography` bucket: every header is an `email` field, every value is a faker-style US address (likely synthetic test data inside the gittables source files).
- Card 0017 `i_want` text: "pulling columns where YDF is high-confidence on a geography subtype". The spec's framing is "YDF is right, Sense is wrong" — but YDF's confidence is the *only* signal, and the spec gives no third lens for adjudication.

**Recommendation:** Tighten AC-01 with one or more of:
- Require dbpedia annotation corroboration (the third lens already in `corpus_pass/dbpedia_annotations.parquet`) for any row crossing into training — at minimum where `Sense ≠ YDF AND dbpedia_semantic_class` agrees with YDF.
- Exclude rows where the column header matches a strong Sense-aligned cue (e.g. `header` is one of the seven Sense-trusted email/phone/id cues). Documented as "Sense's header is load-bearing on these cues; only override when both other lenses agree".
- OR explicitly scope AC-01 to the *header-neutral* subset where Sense's prediction came from values, not headers. That is the population where YDF's value-based signal genuinely outvotes Sense.

A spot-check sample of N=20 rows from each Sense-prediction bucket, with author tick-through, is the minimum evidence needed to defend that "Sense-missed" actually means Sense-missed.

---

### [HIGH] AC-02's "measurable reduction" depends on a v20 retrain that is not scoped inside this spec
**Category:** test-gap
**Pass:** 3
**Description:** AC-02 reads: "v20 retrain consumes the extract — report.md (Part 1) shows a measurable reduction in geography-domain corroborated gaps". Closing AC-02 requires:

1. v20 training run completes (multi-branch retrain, not in this spec's scope).
2. Full corpus diagnostic re-runs (m-19's pipeline; ~hours of compute against 500k+ files).
3. Report comparison of geography mechanism cells across v19 → v20.

Each of those is a meaningful piece of work. The spec carries no `ac_type` field — defaulting to `code`, which blocks `spec.close` per METHOD.md band rules. But AC-02 is structurally an `observation` (post-cutover measurement of an external training run). With `ac_type: code`, the spec cannot close until v20 trains and the corpus pass re-runs — coupling this spec to an unscheduled, multi-day external workload.

Worse, the threshold "measurable reduction" is undefined. Is a 1% reduction in `reject_rate_ceil × format_diversity_path_b` cell count enough? 10%? Statistically significant against what noise floor? Without a number, AC-02 is unfalsifiable.

**Evidence:**
- METHOD.md `ac_type` table: code/config/doc block spec.close; ops/observation defer. AC-02 reads as `observation` (post-cutover empirical measurement).
- `corroborated_gaps.parquet` shows the two geography-relevant cells contain 61,068 rows total; a "measurable reduction" without a numeric floor is open to interpretation.
- AC-03 ("scale to second domain") explicitly depends on AC-02 demonstrating lift — making AC-02's threshold a load-bearing decision for whether the pattern generalises at all.

**Recommendation:**
- Declare `ac_type: observation` on AC-02 (and AC-03 — same dependency chain). This is honest about the close-time band; the spec ships when the extract+sources entry+holdout filter land (AC-01), and AC-02/AC-03 close on the v20 observation window.
- Pin a numeric threshold: e.g. "≥ 20% reduction in count of `reject_rate_ceil × format_diversity_path_b` rows where the affected column predicts a non-geography label and YDF predicts geography with confidence ≥ 0.7". The threshold can be conservative — the point is it must be falsifiable.

---

### [MEDIUM] AC-03's "without code changes" is unverifiable as written
**Category:** test-gap
**Pass:** 1
**Description:** AC-03 reads: "Scale to second domain — same shape of extract — the pattern generalises without code changes per domain". "Without code changes" is a structural claim about the extractor's surface. It can be tested two ways:
- The extractor accepts `--domain` as a free-form argument and the same script handles finance/datetime/identity unchanged.
- A git diff between the geography-only run and the second-domain run shows zero changes to scripts/.

The spec doesn't say which test applies. A reviewer cannot determine whether AC-03 is closed without inferring intent.

**Evidence:** Card 0017 scenario: `when: I run --domain finance (or datetime, or identity)`. The card's wording implies the first test. The spec drops the example.

**Recommendation:** Rewrite AC-03 as: "Running `extract_ydf_specialist_training_data.py --domain finance` produces `eval/gittables/v20_training_candidates/finance.tsv` with the same column schema as `geography.tsv`. The script's git history shows no commits between the two runs."

---

### [MEDIUM] AC-01's output schema mixes file_path types
**Category:** missing-requirement
**Pass:** 1
**Description:** The columns parquet stores absolute paths (`/Users/hugh/datasets/gittables/...`). AC-01's TSV would inherit those. Absolute paths bake the author's local username into the training-data artefact, which then ships into v20's training corpus — a reproducibility and portability regression. The eval-corpus pattern (per `eval/datasets/sources.yaml` convention) is to record paths relative to a known root.

**Evidence:** DuckDB sample shows `/Users/hugh/datasets/gittables/...` in `file_path`. Sources.yaml convention elsewhere uses relative or URL-form provenance.

**Recommendation:** Specify in AC-01 that `file_path` in the output TSV is stored relative to `$GITTABLES_ROOT` (or whatever the canonical env var is in `corpus_pass.log` headers).

---

### [LOW] AC-01's confidence threshold (0.7) is unjustified
**Category:** assumption
**Pass:** 2
**Description:** The 0.7 confidence floor is asserted without reference to a calibration study or precision/recall curve on YDF's geography predictions. The YDF lens was trained on purely synthetic data (`scripts/train_ydf.py`); its confidence calibration against real gittables columns is not characterised in the m-19 outputs that the spec cites.

**Evidence:** `scripts/train_ydf.py` lines 64-80: training corpus is `finetype generate` synthetic output exclusively.

**Recommendation:** Either cite the calibration evidence ("0.7 chosen because YDF's geography precision exceeds 0.9 above this threshold on the spot-check corpus"), or leave the threshold as a tunable parameter with the default at 0.7 documented as "preliminary; revisit after AC-02 observation".

---

### [LOW] No spec-internal `ac_type` declarations
**Category:** missing-requirement
**Pass:** 1
**Description:** All three ACs omit `ac_type`. Per METHOD.md, the default is `code` — which blocks `spec.close` on every AC. AC-02 and AC-03 are structurally `observation` (post-retrain measurement); leaving them implicit forces the spec to stay open until v20 ships, defeating the band's design intent.

**Recommendation:** Add explicit `ac_type` to all three ACs: AC-01 = `code`, AC-02 = `observation`, AC-03 = `code` (the second-domain run is a code-shipped artefact; the lift measurement is what would be `observation`).

---

## Honest Assessment

The card is right in spirit — turning m-19's corroborated gaps into a self-improving training loop is the obvious next move, and the geography-first scoping is sensible. The spec, however, sidesteps the two hardest questions: how to keep the loop honest about leakage, and how to keep it honest about Sense's errors.

The biggest risk is **finding #2 — circular precision degradation**. The largest single bucket of "Sense-missed geography" is mislabelled email columns where Sense reads the header correctly; the spec would train v20 to confidently mis-classify the vastly larger real-world `email` population. This is precisely the failure mode CLAUDE.md's precision principle warns against. AC-01 needs a third-lens corroboration filter (dbpedia) or a header-cue exclusion before this extract is safe to feed v20.

Finding #1 (leakage firewall) is a compliance gap with MADR 0056. Mechanically fixable by adding a sources.yaml entry, a holdout filter, and a row-hash emission — but the spec is silent on all three and would ship a corpus that the existing training pipeline does not protect.

Finding #3 (AC-02 measurability) is the gating defensibility question for the whole spec. Without a numeric threshold and an `ac_type: observation` declaration, "measurable reduction" becomes whatever the agent decides looks like progress — which collapses the spec back into vibes, the exact bar the four-pillars framework exists to raise above.

Address findings #1, #2, and #3 — the extract becomes a defensible, leakage-clean, falsifiable training-data pipeline. Leave them — v20 ships with a precision-degraded geography branch and a leaked gate score.
