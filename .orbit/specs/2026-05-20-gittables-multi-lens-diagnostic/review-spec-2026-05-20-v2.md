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
| 2 — Assumption & failure | MEDIUM+ findings in Pass 1 + content signals (training data, leakage firewall, cross-system boundaries, large-scale compute) | 4 |
| 3 — Adversarial | not triggered — Pass 2 reveals localised fixes, not structural unsoundness | — |

## Findings

### [HIGH] ac-05 mis-states the source format of DBpedia / Schema.org annotations
**Category:** failure-mode
**Pass:** 1
**Description:** ac-05 says "Gittables ships per-table annotation files alongside the parquet" and instructs the implementing agent to extract them into `dbpedia_annotations.parquet`. Empirically, gittables does NOT ship sidecar annotation files. Annotations live inside each parquet's Arrow KV metadata under the key `gittables`, as a JSON blob carrying four distinct annotation namespaces — `dbpedia_syntactic_column_types`, `dbpedia_semantic_column_types`, `schema_syntactic_column_types`, `schema_semantic_column_types` — plus per-column similarity scores and a `table_domain` block. An implementing agent following the spec verbatim will search for missing sidecar files and either give up, invent a file format, or silently drop the lens entirely.
**Evidence:** `ls /Users/hugh/datasets/gittables/abstraction/` shows only `*.parquet` — no sidecars. `duckdb -c "SELECT decode(key), decode(value) FROM parquet_kv_metadata('/Users/hugh/datasets/gittables/abstraction/Designite_Ninject.Web.Mvc_DesignSmells.parquet') WHERE decode(key)='gittables'"` returns the full annotation JSON inline. spec.yaml lines 277–288 describe sidecar files; spec.yaml line 280 names `dbpedia_annotations.parquet` as the extraction target without specifying the source-shape transformation.
**Recommendation:** Rewrite ac-05 description to read approximately: "DBpedia / Schema.org annotations are extracted from each parquet's KV metadata under the `gittables` key — a JSON blob carrying `dbpedia_syntactic_column_types`, `dbpedia_semantic_column_types`, `schema_syntactic_column_types`, `schema_semantic_column_types`, per-column similarity scores, and a `table_domain` block." Then commit to which of the four annotation namespaces (or which union) becomes "the DBpedia lens" — likely `dbpedia_semantic_column_types` since it gives the per-column semantic type. Without this decision, ac-09's "DBpedia annotation" lens-vote is undefined.

### [HIGH] `partition_seed` is named in ac-10's frontmatter and ac-13's reproducibility check but never defined
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-10 says `corpus_pass_id` is the composite hash of `model_sha, ydf_sha, dbpedia_mapping_sha, cascade_version, corpus_index_sha` plus `partition_seed`. ac-13 says "same corpus + same lens versions + **same partition seed** produces byte-identical outputs." The partition scheme defined in the goal (constraint 6) is `file_content_sha256 MOD 2 == 1` — a deterministic function with no seed. So either (a) `partition_seed` is a redundant input that does nothing because the partition function takes no seed, or (b) the intent was a salted partition (`HASH(file_content_sha256, partition_seed) MOD 2 == 1`) and the goal's partition formula is wrong. Either way, `corpus_pass_id` cannot be computed because one of its declared inputs is undefined.
**Evidence:** spec.yaml line 404 (`corpus_pass_id` frontmatter field listing partition_seed), spec.yaml line 458 ("same partition seed"), spec.yaml line 70 (`file_content_sha256 MOD 2 == 1` — no seed parameter). No `partition_seed` value or definition appears anywhere in the spec.
**Recommendation:** Pick one. Option A: drop `partition_seed` from the corpus_pass_id inputs and from ac-13's reproducibility predicate; the partition is fully determined by `file_content_sha256 MOD 2 == 1` and needs no seed. Option B: change the partition to `HASH(file_content_sha256 || partition_seed) MOD 2 == 1`, declare `partition_seed` (a fixed constant — recommend reusing `sample_seed = 20260520` from ac-02), and update constraint 6 and ac-06 verification accordingly. Option A is the principled choice — the partition is already deterministic; introducing a seed adds a coordination point without buying anything.

### [HIGH] ac-11 hinges on the predicate "YDF flagged the row" but the predicate is never defined
**Category:** test-gap
**Pass:** 1
**Description:** ac-11's correctness clause has two conjuncts: (a) `recommended_action_class` matches `truth_mechanism`'s expected action, AND (b) "YDF prediction agrees with `truth_inferred_type` **when YDF flagged the row**." Nothing in the spec defines what makes YDF "flag" a row. Candidate readings: (i) YDF made any prediction (i.e. all rows in YDF's training-supported type set); (ii) YDF's top-1 confidence exceeded some threshold (unspecified); (iii) YDF disagreed with Sense (a "candidate gap" reading); (iv) YDF entered the corroboration set for some gap. Each reading produces a different `n_flagged_by_diagnostic` and a different `precision_on_flagged`, and the ≥0.80 threshold is meaningless without a fixed predicate.
**Evidence:** spec.yaml lines 416–426 (ac-11 description), spec.yaml lines 427–431 (verification). The verification output `{n_rows_total, n_flagged_by_diagnostic, n_correctly_diagnosed, precision_on_flagged}` names `n_flagged_by_diagnostic` as the denominator without saying what triggers a flag.
**Recommendation:** Pin the flag predicate explicitly. Recommended phrasing: "A row is **flagged by the diagnostic** if it appears as `sample_evidence` in any GapEntry that enters `report.md` (i.e. it is part of a corroborated gap cluster). For flagged rows, ac-11 measures (a) `recommended_action_class == mapping_table[truth_mechanism]` AND (b) `ydf_prediction == truth_inferred_type`. Precision = correct / flagged ≥ 0.80." Also clarify: are unflagged rows counted (true negatives) or ignored? The current "precision on flagged" phrasing implies ignored — confirm that explicitly.

### [MEDIUM] ac-04 runtime budget is tight against realistic single-file cost
**Category:** failure-mode
**Pass:** 2
**Description:** Corpus enumerates ~1.02M parquet files (`find /Users/hugh/datasets/gittables -name '*.parquet' | wc -l` = 1,018,286). ac-04 targets ≤24 h at `--jobs 16` with a 72 h ceiling that triggers spec amendment. `scripts/gittables_gate.py` reports "Target <60min on M1 for 2000 files" single-process — i.e. ~1.8 s/file. Measure half is ~509k files. At `--jobs 16` with linear scaling (optimistic — DuckDB COPY contention and finetype binary startup amortise poorly): 509,000 × 1.8 s / 16 ≈ 16 h. With realistic 60% parallel efficiency: ~26 h. The 24 h target is achievable on a good day; the 72 h ceiling is comfortable. But: (i) `finetype profile` + `finetype validate` per file is two subprocess invocations — startup cost dominates on small files, and gittables files are mostly small (`Designite_*` files are 36–66 KB). (ii) The corpus runner adds YDF inference + DBpedia metadata extraction per file, neither of which is in the baseline gittables_gate benchmark. The realistic single-file cost is likely 2.5–4× the gate's per-file cost.
**Evidence:** `find /Users/hugh/datasets/gittables -name '*.parquet' | wc -l` returns 1,018,286. `scripts/gittables_gate.py` line 43: "Target <60min on M1 for 2000 files." Spec lines 250–263 set the budget. Spec does not name the additional YDF + DBpedia overhead.
**Recommendation:** Re-state ac-04's runtime budget with the additional lens overhead in mind. Recommend: "Target ≤ 48 h wall-clock at `--jobs 16` on a workstation-class machine **including YDF inference and DBpedia metadata extraction**; the 72 h ceiling triggers spec amendment." Alternatively, instruct the dry-run estimator (the `--dry-run` invocation already in ac-04's verification) to run the per-file pipeline on a 1000-file sample and project linearly — this gives the implementing agent an empirical number rather than two competing estimates.

### [MEDIUM] ac-02 "stratified by DBpedia table topic" leaves the strata source unstated
**Category:** test-gap
**Pass:** 1
**Description:** ac-02 says the 100-table sample is "stratified by DBpedia table topic, sample seed = 20260520." Gittables exposes multiple candidate topic fields: the parent directory name (`abstraction`, `attrition_rate`, etc. — ~1,100 directories), the `table_domain.dbpedia_semantic` value in KV metadata (e.g. `http://dbpedia.org/ontology/File`), or the `table_domain.dbpedia_syntactic` field. These three give different stratifications. An implementing agent will pick one and the spike's `overlap_fraction` will be a function of that pick — the ≥0.20 threshold gate (which controls whether ac-09 enters downgrade mode) is sensitive to it.
**Evidence:** spec.yaml lines 206–214. Empirical KV metadata sample shows three distinct topic-class fields in `table_domain`. No spec text picks one.
**Recommendation:** Name the field. Recommended: "stratified by the directory name (gittables' canonical 'topic' partition — ~1.1k topic directories under `/Users/hugh/datasets/gittables/`); sample 100 tables proportional to topic size, capped at ≤2 tables per topic to avoid heavy-tail dominance." Or pick `table_domain.dbpedia_semantic` and say so. Either is fine; the load-bearing requirement is that the spec names it.

### [MEDIUM] `validator_widening` is simultaneously a mechanism token AND an action class — the locked mapping is self-referential
**Category:** constraint-conflict
**Pass:** 2
**Description:** The mechanism-token → action-class table (goal §Mechanism token → recommended action class) maps `validator_widening` (mechanism token, from the closed 10-set per MADRs 0075 + 0081) to `validator_widening` (action class). This is structurally fine — a mechanism token can name its own action — but it makes ac-11's correctness check ambiguous in one edge case: if `truth_mechanism = "validator_widening"` and the diagnostic emits `recommended_action_class = "validator_widening"`, the row is "correct" mechanically, but the diagnostic learned nothing the labeller didn't already encode. More importantly, an implementing agent reading the table cold may confuse the two namespaces. Action classes appear nowhere else as a closed set in the spec (no `action_class` enum is declared) — the reader must infer their closure from the right-hand column of the mapping table.
**Evidence:** spec.yaml lines 109–120. Action classes appearing in the table: `validator_widening`, `model_retrain`, `training_data_addition`, `taxonomy_addition`, `fallback_adjustment`, `N/A (no gap surfaced)` — six values, of which one collides with a mechanism token name.
**Recommendation:** Add a one-line declaration above the mapping table: "**Recommended-action-class enum (closed 5-set):** `validator_widening`, `model_retrain`, `training_data_addition`, `taxonomy_addition`, `fallback_adjustment`. `prediction_confirmed` maps to `N/A` (no gap surfaced and no row emitted)." Then the self-reference is documented and the closure is explicit. Optionally rename the action class to `validator_widening_action` to remove the namespace collision — but that ripples through downstream specs, so prefer the declaration approach.

### [MEDIUM] DBpedia lens disagreement semantics are undefined
**Category:** test-gap
**Pass:** 2
**Description:** ac-12 says a gap passes spot-check if "the `corroborating_lenses` genuinely disagree with each other" — but the corroborating lenses are by construction the ones that *agree* on flagging the gap, by ac-09's filter design. The intended reading is probably "the lenses disagree with the column's current Sense prediction" (i.e. they jointly identify a problem), but the prose says the lenses disagree with each other. Without a fixed definition of "agreement" for the DBpedia lens (which produces semantic classes, not taxonomy IDs), the spot-check verification is grader-dependent.
**Evidence:** spec.yaml lines 439–441 (ac-12 description). ac-09 (spec.yaml lines 367–379) defines lens corroboration as agreement on flagging, not on type. There is no explicit "DBpedia lens vote" function — does `dbpedia_semantic_column_types.<col>.id = "http://dbpedia.org/ontology/odor"` map to a FineType taxonomy ID via the hand-curated table, and that mapped ID is the DBpedia vote? Implied but not stated.
**Recommendation:** In ac-09 or the goal's domain object, state the DBpedia lens-vote function explicitly: "The DBpedia lens votes `mapping_table[dbpedia_semantic_column_types.<col>.id]` if that mapping has `mapping_status ∈ {direct, partial}`; otherwise it does not vote." Then in ac-12, rephrase the disagreement clause as: "the `corroborating_lenses` jointly disagree with the column's `sense_prediction` (or each pair of voters returns predictions that genuinely point at the same gap)." This makes spot-check graders' judgements reproducible.

### [MEDIUM] `OBSERVED_SAMPLE_LIMIT` line reference drift
**Category:** missing-requirement
**Pass:** 1
**Description:** Goal constraint 10 references `OBSERVED_SAMPLE_LIMIT` at `scripts/cron_cycle_work.py:80`. The constant lives at line 97 of that file (`grep -n 'OBSERVED_SAMPLE_LIMIT = 8' scripts/cron_cycle_work.py` returns line 97). The value (8) matches. The reference does not block — the load-bearing claim is the value, not the line — but it suggests the spec was not re-verified against the file after edits, and other line/file references in the spec may have drifted similarly.
**Evidence:** spec.yaml line 101 (`scripts/cron_cycle_work.py:80`). Actual location: line 97.
**Recommendation:** Either fix the line number or strip it (write `OBSERVED_SAMPLE_LIMIT in scripts/cron_cycle_work.py` without a line number). Line numbers drift; symbol references don't. Recommend dropping the line number across the spec to remove the maintenance tax.

### [LOW] ac-08 verification's "2 × N table" assertion is mechanically odd
**Category:** test-gap
**Pass:** 2
**Description:** ac-08's verification says the `GROUP BY criterion, mechanism_token` query returns "a 2 × N table whose mechanism_token column is a subset of the closed 10-token set." Read literally, this asserts the result has exactly 2 rows (one per criterion), each with an N-element mechanism_token list — but the actual SQL emits one row per (criterion, mechanism_token) pair, so the result is up to 2 × 10 = 20 rows, not "2 × N." The intended assertion is probably "the result has 2 distinct criterion values and ≤10 distinct mechanism_token values, all drawn from the closed set." Currently the verification is grader-dependent.
**Evidence:** spec.yaml lines 354–362 (ac-08 verification).
**Recommendation:** Rewrite as: "DuckDB query `SELECT DISTINCT mechanism_token FROM 'mechanism_decomposition.parquet'` returns a subset of the closed 10-token set (assertion: every emitted token is in the set). A second query `SELECT DISTINCT criterion FROM 'mechanism_decomposition.parquet'` returns exactly `{non_trivial_floor, reject_rate_ceil}` (assertion: the criterion enum is closed)." The "2 × N table" phrasing is confusing and load-bearing only as evidence; the assertions are what matters.

### [LOW] ac-11 declares a per-row lens-vote tuple is emitted but the verification does not check it appears anywhere on disk
**Category:** test-gap
**Pass:** 2
**Description:** ac-11's description says "the diagnostic emits its per-row lens-vote tuple `(sense_prediction, ydf_prediction, dbpedia_annotation, mechanism_token, recommended_action_class)`" for each row of `labelled_eval.tsv`. The verification names only `labelled_eval_validation.json` with the aggregate `{n_rows_total, n_flagged_by_diagnostic, n_correctly_diagnosed, precision_on_flagged}` — there's no artefact that holds the per-row tuples for audit. The spot-check (ac-12) refers to the corpus-level `report.md`, not labelled_eval. So the per-row claim is unverifiable.
**Evidence:** spec.yaml lines 416–431.
**Recommendation:** Add a sidecar artefact: "Per-row lens votes land in `eval/gittables/corpus_pass/labelled_eval_per_row.tsv` with columns `(row_id, sense_prediction, ydf_prediction, dbpedia_annotation, mechanism_token, recommended_action_class, truth_inferred_type, truth_mechanism, flagged, correctly_diagnosed)`. The validation JSON aggregates over this TSV." Without it, ac-11 is unauditable and reviewers cannot reproduce the precision number.

---

## Honest Assessment

The spec has improved substantially since the v1 review — the schema-parse failure is fixed, the timestamp/byte-identical contradiction is resolved via `corpus_pass_id`, the corpus-index path is named, the mechanism → action-class mapping is locked into the goal, the `finetype infer-type` subcommand name is correct, and ac-12's per-cell stratified spot-check addresses the prior sample-size concern. What remains are concrete fixable problems concentrated in three areas: (1) the DBpedia lens is under-specified — its source format is wrong (ac-05), its strata field is unnamed (ac-02), and its vote function is implicit (ac-09/ac-12); (2) the `partition_seed` input to `corpus_pass_id` is named but never defined, which silently breaks ac-13's reproducibility check; (3) ac-11's "flagged" predicate is undefined, which makes the ≥0.80 precision threshold unauditable. Each fix is local and doesn't touch the design's structural backbone. The lens-independence work in ac-03 and the mechanism-decomposition work in ac-08 are solid. Fix the DBpedia source-format misstatement first — it is the only finding likely to send the implementing agent down a multi-hour wrong path before they realise the sidecar files don't exist.
