# Spec Review

**Date:** 2026-05-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-05-20-gittables-multi-lens-diagnostic
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | Pass 1 surfaced training-data, eval-dataset, cross-system, reproducibility content signals + 1 HIGH structural finding | 4 |
| 3 — Adversarial | not triggered (Pass 2 findings are localised, no cascading-failure pattern) | — |

The v4 review's HIGH (ac-03/ac-11 coupling) and all five v4 MEDIUMs are materially addressed in the current spec — ac-11 now grades on `mechanism_correct` only, the ac-12 attestor is named, ac-09 reframes Sense as subject-not-voter, ac-02 adds a Wilson CI gate, ac-13 moves to the calibrate half, ac-08 makes per-column attribution explicit. The findings below are fresh — issues I see on a cold read of the current spec, not v4 carry-overs.

## Findings

### [HIGH] ac-03 references a script that does not exist — `scripts/load_training_corpus.py`
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-03's description names `scripts/load_training_corpus.py` as the loader for "any other rows reachable" beyond synthetic generators and the 200-row labelled_eval. `find scripts/ -name 'load_training_corpus*'` returns nothing — the script does not exist. An implementer reading ac-03 verbatim cannot determine whether (a) the script is meant to already exist and is missing, (b) it is an implicit deliverable of ac-03, or (c) "any other rows" is an optional/empty bucket and the named script is aspirational. The verification clause asserts `top1_accuracy >= 0.70` but says nothing about what the training corpus must contain — so the YDF model could be trained on the 200-row labelled_eval alone (a clear train/eval leakage red flag against the spec's own hard constraint 6) and still close the AC.
**Evidence:** spec.yaml ac-03 description; `find /Users/hugh/github/meridian-online/finetype/scripts -name "load_training_corpus*"` returns empty; `ls scripts/` shows no such file.
**Recommendation:** Pick one. Either (a) mark `scripts/load_training_corpus.py` as a deliverable of ac-03 with a verification clause that asserts it exists and emits a row-count, or (b) drop the reference and constrain the training corpus to "synthetic generators per MADR 0022 plus labelled_eval rows whose `truth_mechanism != misclassification`" (or similar — the point is to name the corpus precisely). Whichever way, add a leakage guard: the YDF model MUST NOT train on labelled_eval rows that will later appear in ac-11's grading — partition labelled_eval into train/grade halves and capture the split file SHA in `ydf_labelled_accuracy.json`.

### [MEDIUM] ac-11 vocabulary-audit precondition is defensive against a condition that does not exist
**Category:** test-gap
**Pass:** 2
**Description:** ac-11 says the labelled_eval was labelled before the closed 10-set was finalised, and that `scripts/audit_labelled_eval_vocabulary.py` must enumerate `truth_mechanism` values and commit a renormalisation table if any value is outside the closed set. Empirically, the labelled_eval's distinct `truth_mechanism` values are `{fallthrough, format_diversity_path_b, misclassification, prediction_confirmed, validator_widening}` — every one is in the closed-10 set (verified by `awk -F'\t' 'NR>1 {print $11}' labelled_eval.tsv | sort -u`). So the audit script runs, finds zero values to renormalise, emits an empty mapping table or skips writing it, and the precondition closes vacuously. The spec is paying real complexity (a new script, a possibly-empty mapping table, a `n_rows_excluded` accounting branch) for a condition that does not arise. If labelled_eval acquires new rows under a different labeller before ac-11 runs, the audit becomes load-bearing — but the spec does not name that scenario.
**Evidence:** labelled_eval.tsv distinct `truth_mechanism` values; spec.yaml ac-11 description; MADR 0081 closed-10 set; labelling_protocol.md confirms labellers were instructed to map judgements to the closed-10 set.
**Recommendation:** Either (a) drop the audit step and add a one-line verification clause "all `truth_mechanism` values in labelled_eval.tsv are members of the MADR 0081 closed-10 set" (deterministic, no new script needed), or (b) keep the audit but add a precondition note: "if the audit emits zero renormalisations, the mapping table file MAY be absent; `n_rows_excluded = 0` is the expected default." The current shape suggests an exclusion branch will likely trigger when it almost certainly won't.

### [MEDIUM] `corpus_pass_id` composite hash function is underspecified — reproducibility (ac-13) locks in whatever the implementer picks
**Category:** test-gap
**Pass:** 2
**Description:** ac-10 says `corpus_pass_id` is "composite hash of the preceding five fields, byte-stable across re-runs". The five fields are `model_sha`, `ydf_sha`, `dbpedia_mapping_sha`, `cascade_version`, `corpus_index_sha`. The composition rule is not specified — is it `SHA256(field_1 || field_2 || ... || field_5)`? `SHA256(json.dumps({...}, sort_keys=True))`? Some canonical TLV encoding? Concatenation with separators? ac-13 then tests that the value is "stable across re-runs" — which it will be, regardless of which choice the implementer makes, because the implementer hashes the same way both runs. So ac-13 cannot fail on this. The risk is downstream: when this spec's outputs are consumed by a follow-up spec (or when a third party tries to verify `corpus_pass_id` independently), the function shape is implicit-by-code rather than declared.
**Evidence:** spec.yaml ac-10 description "composite hash of the preceding five fields"; ac-13 verification clause checks frontmatter value matches between runs only.
**Recommendation:** Specify the function in ac-10. Suggested: "`corpus_pass_id = SHA256(model_sha + ':' + ydf_sha + ':' + dbpedia_mapping_sha + ':' + cascade_version + ':' + corpus_index_sha)` — strings UTF-8, no trailing whitespace, fields concatenated in the listed order with a single `:` delimiter." Compare to the explicit specification of `value_shape_signature` in ac-09 ("SHA256 of the sorted set of distinct character-class patterns extracted from the column's `sample_values` (each value mapped via `[A-Z]→A, [a-z]→a, [0-9]→9, other→.` then deduplicated and sorted lexicographically)") — that level of precision should apply here too.

### [MEDIUM] ac-09's `value_shape_signature` uses sample_values (≤8 per column) — cluster key is coarser than the description suggests
**Category:** failure-mode
**Pass:** 2
**Description:** ac-09 defines candidate-gap clusters by `(mechanism_token, taxonomy_signature, value_shape_signature)`. The `value_shape_signature` is "SHA256 of the sorted set of distinct character-class patterns extracted from the column's `sample_values`". Hard constraint 10 caps `sample_values` at "≤8 values per column" (matches `OBSERVED_SAMPLE_LIMIT` in `cron_cycle_work.py`). With at most 8 values per column, the distinct-character-class-pattern set has ≤8 elements (often fewer after deduplication). Two columns with very different underlying domains can collide on this signature when their 8-sample slice happens to share character-class shapes (e.g. all integer-looking samples become `{"9", "99", "999"}` regardless of whether the column is a year, count, currency value, or sequence ID). This makes clusters bigger and gap candidates more aggregated than the description suggests — affecting `affected_column_count` (the ranking key) and `sample_evidence` selection.
**Evidence:** spec.yaml ac-09 description and hard constraint 10; `cron_cycle_work.py` `OBSERVED_SAMPLE_LIMIT = 8`.
**Recommendation:** Either (a) acknowledge the coarseness explicitly in ac-09 ("the signature is intentionally coarse — clusters aggregate columns whose 8-sample character-class shapes are identical; over-clustering is acceptable as it produces fewer, larger gaps") and add a `progress.md` note showing the empirical cluster size distribution after ac-09 runs, or (b) widen the value-shape signature to use a column-level statistic (e.g. histogram of character classes across all observed rows, not just the 8-sample slice) — but that adds work to the corpus pass. Option (a) is cheaper and is honest about the limitation.

### [MEDIUM] ac-12 per-cell pass rate threshold has no defined consequence for cells that fail
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-12 says "Per-cell pass rate ≥ 90%. Attestation captured in `spot_check.md`" and the verification clause says "if any cell falls below threshold, `progress.md` names the root cause and the remediation action taken before spec close." But the spec does not say what remediation is acceptable. If the attestor finds cell X has 60% pass rate, what closes the AC? Re-running the lens analysis? Demoting cell X to "needs work"? Excluding cell X from `report.md`? Re-labelling the mechanism cascade for the failing gaps? The current language permits "remediation: noted, moving on" which would close the AC despite a known-bad cell. For an `ac_type: ops` criterion this is especially load-bearing — the human signoff is the only gate.
**Evidence:** spec.yaml ac-12 description and verification; `ac_type: ops`.
**Recommendation:** Add a closed enumeration of remediation actions to ac-12's verification clause. Suggested set: (a) re-run the lens analysis with a corrected mapping (requires updating `dbpedia_finetype_mapping.tsv` or re-training YDF and re-running from ac-05 onwards), (b) demote failing-cell gaps from `report.md` to `single_lens_signals.tsv` with a `demoted_by_spot_check = true` flag, (c) exclude the cell entirely and record the exclusion in `report.md`'s frontmatter as `excluded_cells: [...]`. Spec close requires one of these per failing cell.

### [LOW] ac-04 reference to `holdout_paths.txt` is correct but the enumeration delta from the existing partition is unstated
**Category:** content-signal
**Pass:** 1
**Description:** ac-04 says the new corpus index `corpus_paths.txt` "enumerates every parquet under `/Users/hugh/datasets/gittables/`" and the runner "enumerates from this index, not from `holdout_paths.txt`." Memory `autonomy-contract-activation-infrastructure-shipped-2026-05` records that `holdout_paths.txt` was frozen with 1,018,257 deduped entries (29 collisions removed). The new `corpus_paths.txt` enumerates "every parquet" — which is ~1,018,286 raw entries. The two are different sets: holdout was deduplicated by content-hash; corpus is the raw parquet listing. The spec does not address whether `corpus_paths.txt` should be dedup'd, and the existing `gittables_gate.py` partition logic uses `file_content_sha256 MOD 2` — applied to a non-deduplicated list, this means duplicate-content files land in deterministic halves (good) but the same content is processed twice (wasteful). Not a blocker — but the implementer should know.
**Evidence:** spec.yaml ac-04 description; memory `autonomy-contract-activation-infrastructure-shipped-2026-05` (holdout dedup count); memory `gittables-corpus-shape-2026-05-03-1-018` (1,018,286 raw count).
**Recommendation:** Add a sentence to ac-04: "`corpus_paths.txt` is the raw lexicographic listing — no content-hash dedup. Files with duplicate `file_content_sha256` are processed multiple times by the corpus pass; this is acceptable because the partition (MOD 2) is deterministic and `files.parquet` row count matches `corpus_paths.txt` measure-half count by construction." Alternatively, dedup at index time and update the row-count assertion accordingly. Pick one explicitly.

### [LOW] ac-08's `contributing_columns_count_or_reject_count` field name overloads two semantics into one column
**Category:** test-gap
**Pass:** 1
**Description:** The output schema of `mechanism_decomposition.parquet` includes `contributing_columns_count_or_reject_count` — a single column whose interpretation depends on the row's `criterion` value. For criterion-a rows it's a column count; for criterion-b rows it's a reject count. Downstream consumers (the report generator at ac-10, the spot-check at ac-12) must branch on `criterion` to interpret the value. Cleaner schemas use two columns and leave one null per row, or two distinct row types. The current shape is workable but bug-prone — any aggregation query that forgets to branch on `criterion` produces silently-wrong totals.
**Evidence:** spec.yaml ac-08 description (output schema for `mechanism_decomposition.parquet`).
**Recommendation:** Split into `contributing_column_count` (criterion-a only, NULL otherwise) and `reject_count` (criterion-b only, NULL otherwise). Or rename to `criterion_specific_count` with a clear caveat. Minor — does not block implementation, but the v1 shape leaks into downstream queries.

### [LOW] Pass-1 deterministic gate-AC description check
**Category:** content-signal
**Pass:** 1
**Description:** All 13 ACs report `is_gate=0` per the parser output — no ACs are marked as gates. The deterministic gate-AC description rules are vacuously satisfied; no findings emitted. Recorded for the contract.
**Evidence:** `orbit-acceptance.sh acs` output column 4 = `0` for every AC.
**Recommendation:** None. A spec without gate-marked ACs is permitted.

---

## Honest Assessment

The spec has materially absorbed the v4 review's findings — ac-11 decoupled from YDF accuracy (the v4 HIGH), ac-12 attestor named, ac-09 Sense framing fixed, ac-02 CI gate added, ac-13 calibrate-half move, ac-08 column-level attribution. The major design decisions look correct: FineType taxonomy canonical, DBpedia as navigation aid, ≥2-lens corroboration, reproducibility via deterministic signatures, leakage discipline preserved. This is a well-iterated spec.

**Biggest residual risk:** ac-03's `scripts/load_training_corpus.py` reference is to a non-existent file, with no train/eval leakage guard for YDF's training data. An implementer who reads ac-03 literally will either invent a corpus, train on labelled_eval (which then becomes the grading set in ac-11 — direct leakage), or abandon the optional rows clause silently. This is a HIGH because it intersects with hard constraint 6 (train/eval leakage prevention) — the spec's own non-negotiable rule. Naming the YDF training corpus precisely and adding a labelled_eval train/grade split is mechanical to fix.

**Secondary risk:** ac-09's `value_shape_signature` is coarser than its description implies (≤8 samples → ≤8-element pattern set), affecting cluster sizes and ranking. This is acknowledgement-shaped, not redesign-shaped — one sentence noting the empirical coarseness suffices.

The remaining MEDIUMs (vocabulary-audit precondition, corpus_pass_id hash function, spot-check failure remediation) are spec-clarity issues, not design defects. Worth folding into the same revision cycle. Once ac-03's leakage path is closed, the spec is implementation-ready.
