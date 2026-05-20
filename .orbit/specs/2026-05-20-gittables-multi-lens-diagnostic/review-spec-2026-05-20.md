# Spec Review

**Date:** 2026-05-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-05-20-gittables-multi-lens-diagnostic
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 7 |
| 2 — Assumption & failure | MEDIUM+ findings in Pass 1 + content signals (training data, leakage firewall, cross-system boundaries) | 4 |
| 3 — Adversarial | not triggered — Pass 2 reveals concrete spec edits rather than structural unsoundness | — |

## Findings

### [HIGH] Spec is malformed under orbit-state schema — top-level `constraints` field rejected
**Category:** constraint-conflict
**Pass:** 1
**Description:** `orbit --json spec show 2026-05-20-gittables-multi-lens-diagnostic` fails with `unknown field 'constraints', expected one of 'id', 'goal', 'cards', 'status', 'labels', 'acceptance_criteria'`. The spec carries 9 load-bearing constraints (taxonomy canonicality, lens independence, mechanism vocabulary closure, two-criterion gate constants, leakage firewall, zero-Python-at-runtime, supersession, reproducibility, storage shape) under a `constraints:` key that the substrate does not accept. Every downstream `orbit` verb against this spec — `spec show`, `task` linkage, the audit harness, and `/orb:drive`'s spec-fetch step — will fail until the schema mismatch is resolved.
**Evidence:** `orbit --json spec show 2026-05-20-gittables-multi-lens-diagnostic` returns `{"error":{"category":"malformed","message":"yaml parse failed: unknown field 'constraints', expected one of 'id', 'goal', 'cards', 'status', 'labels', 'acceptance_criteria' at line 17 column 1"}}`. `plugins/orb/scripts/orbit-acceptance.sh acs` consequently fails to enumerate the AC list. spec.yaml lines 17–28 hold the orphaned block.
**Recommendation:** Move the body of `constraints:` into the spec's `goal` (or a follow-up MADR referenced from `goal`). The constraints are real and load-bearing — they are not removable — but the substrate has no schema slot for them at top level. Re-introduce them in the body of the `goal` block, or split each constraint into a separate MADR under `.orbit/choices/`. Re-run `orbit spec show` to confirm parse succeeds before implementation begins. Add `id: 2026-05-20-gittables-multi-lens-diagnostic` to the spec — it is currently missing.

### [HIGH] ac-13 reproducibility contradicts ac-10 frontmatter requirement
**Category:** constraint-conflict
**Pass:** 1
**Description:** ac-10's verification requires `report.md` frontmatter to include `corpus_pass_timestamp` (a per-run wall-clock value). ac-13's verification requires `report.md` to be **byte-identical** between two runs of `scripts/test_corpus_pass_reproducibility.sh`. Two real runs cannot share a timestamp; the test will always fail on the timestamp line. The same holds for any other run-dependent field (PID, hostname, run duration) that a frontmatter generator might include without thought.
**Evidence:** spec.yaml line 295 (`corpus_pass_timestamp`); spec.yaml line 360 (`(c) report.md is byte-identical between runs`).
**Recommendation:** Either (a) relax ac-13's `report.md` check from "byte-identical" to "byte-identical after stripping the frontmatter timestamp" (and name the strip rule explicitly), or (b) replace `corpus_pass_timestamp` in ac-10's frontmatter with `corpus_pass_id` (a hash of `(model_sha, ydf_sha, dbpedia_mapping_sha, cascade_version, partition_seed, corpus_index_sha)`) which is byte-stable across re-runs and still answers "which corpus snapshot produced this report". Option (b) is the principled choice — reproducibility is the load-bearing property, the wall-clock is decoration.

### [HIGH] "Full corpus" scope is unbounded and no corpus index exists
**Category:** missing-requirement
**Pass:** 1
**Description:** The spec repeatedly says "full corpus" / "full gittables corpus" / "every gittables column", and implementation_notes estimate "~1M tables × ~10 columns avg = ~10M column rows ≈ ~2GB Parquet". ac-04's verification, however, says row count "equals the measure-half file count from `holdout_paths.txt` (or equivalent corpus index)". `holdout_paths.txt` contains 2,000 paths — the holdout, not the corpus. There is no committed corpus-wide index in `eval/gittables/`. The "or equivalent corpus index" hand-waves the most expensive design decision in the spec: what file set the corpus pass enumerates, who owns it, and how its leakage-partition signature is computed.
**Evidence:** `wc -l eval/gittables/holdout_paths.txt` returns 2008. `ls eval/gittables/` shows no full-corpus index (no `all_paths.txt`, `corpus_index.txt`, etc.). The full dataset lives at `/Users/hugh/datasets/gittables/` outside the repo (one directory per gittables "topic"). spec.yaml lines 174–179 (ac-06 verification) reference `holdout_paths.txt`, conflating holdout-half with corpus-half.
**Recommendation:** Add an AC (or fold into ac-04) that produces a committed corpus index file `eval/gittables/corpus_paths.txt` enumerating every parquet under `/Users/hugh/datasets/gittables/`, with a recorded SHA256 of the sorted index. Then `holdout_paths.txt` is no longer the row-count truth set — `corpus_paths.txt` is. Also bound the runtime — at 2,000 files the existing gate takes ~50min on M1 (per `gittables_gate.py` docstring); 1M files extrapolates to ~17 days single-process. The spec needs either a runtime budget (with `--jobs N` calibration) or a "we expect this to take K days, this is acceptable" line in implementation_notes.

### [MEDIUM] `finetype infer` CLI name does not exist — it is `finetype infer-type`
**Category:** failure-mode
**Pass:** 1
**Description:** implementation_notes line 2 says "the diagnostic invokes `finetype infer` (Phase 1 module shipped per 2026-05-04 spec)". The 2026-05-04 progress.md explicitly records that the subcommand is `finetype infer-type`, not `finetype infer` — the rename was forced by a name collision with an existing subcommand (recorded under `finetype-7zi.1` in that spec's progress.md). An implementing agent following the spec verbatim will issue a wrong subprocess call and fail on first invocation.
**Evidence:** `.orbit/specs/2026-05-04-autonomous-type-inference/progress.md` line: "existing `finetype infer` subcommand collides with the spec's `finetype infer` triangulator. Resolved by using `finetype infer-type`." `crates/finetype-cli/src/main.rs` exists and is the canonical source — verify subcommand spelling there before implementation.
**Recommendation:** Replace `finetype infer` with `finetype infer-type` in spec.yaml's implementation_notes (line 365). One-character fix; load-bearing because the spec is the implementing agent's brief.

### [MEDIUM] Mechanism-token → action-class mapping is the load-bearing relation but the spec defers it to "progress.md before this AC is measured"
**Category:** test-gap
**Pass:** 1
**Description:** ac-08 locks `mechanism_token` to a closed 10-token set. ac-10 locks `recommended_action_class` to a closed 5-value set. ac-11 says "the diagnostic's `recommended_action_class` for the row matches the row's `truth_mechanism` family (validator_widening ↔ validator_widening token; taxonomy_addition ↔ unknown_no_fit token; etc.)" — the partial example given is 2 of (at minimum) 10 entries. The "etc." hides the mapping from mechanism token to action class, which is the function that converts cascade signal into downstream-spec recommendation. ac-11 says the mapping is "locked in `progress.md` before this AC is measured" — meaning by the implementing agent, post-hoc, with no spec-level review. The mapping is precisely the kind of artefact that benefits from review-spec scrutiny up front; deferring it pushes the decision into a sidecar markdown the spec-close gate does not enforce.
**Evidence:** spec.yaml lines 302–312 (ac-11 description), lines 313–319 (ac-11 verification). No explicit `mechanism_to_action_class` table appears in the spec or in the linked MADRs (0075, 0081). The example given is `validator_widening ↔ validator_widening` (trivially symmetric) and `unknown_no_fit ↔ taxonomy_addition` (the only non-trivial mapping spelled out).
**Recommendation:** Add a `mechanism_to_action_class:` mapping block to spec.yaml's `ontology_schema` (or as a sibling table). Spell out all 10 mechanism tokens → 5 action classes (with explicit "N/A" for tokens that have no actionable response, e.g. `prediction_confirmed`). Then ac-11's verification compares against the spec-locked table, not a progress.md sidecar. This is a spec-phase decision, not an implementation-phase observation.

### [MEDIUM] ac-09 corroboration logic over-loads when `design_path == "downgrade_dbpedia"`
**Category:** failure-mode
**Pass:** 1
**Description:** ac-09's downgrade branch says: when DBpedia is downgraded, "the corroboration rule for gaps in DBpedia-anchored cells requires Sense + YDF + cascade (DBpedia counts only as validation, not as a corroboration lens)." But ac-09 then requires DuckDB query `SELECT COUNT(*) FROM corroborated_gaps WHERE dbpedia_role = 'validation_only' AND corroborating_lens_count < 3` returns 0. The `corroborating_lens_count` definition is unclear in the downgrade case: does DBpedia count toward the count (and "validation_only" is just a label) or not (and the count is over `{Sense, YDF, cascade}` only with a floor of 3)? The two readings give different answers when DBpedia agrees: under the first reading, DBpedia + 1 lens passes; under the second, DBpedia is invisible and 3 non-DBpedia lenses are required even when DBpedia agrees with them.
**Evidence:** spec.yaml lines 238–264 (ac-09 description + verification). The text alternates between "DBpedia counts only as validation" (DBpedia excluded from count) and "corroborating_lens_count < 3" (which implies 3 lenses including or excluding DBpedia — unspecified).
**Recommendation:** Pin the semantics explicitly. Recommended phrasing: "In the downgrade case, `corroborating_lens_count` is computed over `{Sense, YDF, cascade}` only — DBpedia agreement does not increment the count. The `dbpedia_role` field records `'validation_only'` for downgrade-mode gaps that DBpedia happens to corroborate (informational), but does not enter the corroboration arithmetic." Add this as a one-sentence clarification immediately after the downgrade rule.

### [MEDIUM] Pass 1 gate-AC text check passes for ac-01, ac-02, ac-12 (no `gate: true` ACs in spec)
**Category:** observation
**Pass:** 1
**Description:** Deterministic gate-AC description check (rule 5 of the structural scan) requires every AC where `is_gate=1` to have a non-empty, non-placeholder description ≥ 20 chars. The orbit-state parser would emit `is_gate` from the AC's `gate:` field, but the spec uses `ac_type` (the AC-taxonomy band) and not `gate: true` flags at the AC level. The card 0014 scenarios all carry `gate: false`. No AC in this spec has `gate: true`, so the check is vacuously passed — this is a clean structural result, not a finding.
**Evidence:** spec.yaml acceptance_criteria entries 29–361 — none carry a `gate:` key. Card 0014 scenarios (lines 7–47) all explicitly carry `gate: false`. The parser fails to run (per HIGH finding above), but inspection of the YAML confirms no gate ACs are present.
**Recommendation:** No change required from this check. Logged for traceability.

### [MEDIUM] Lens independence is asserted by ac-03 but `disjoint_from_sense: bool` is a boolean over multi-element feature categories
**Category:** test-gap
**Pass:** 2
**Description:** ac-03's verification asserts `disjoint_from_sense == true` and `feature_categories: [list]`. Sense's five branches are `{char, embed, stats, header, validation}`. The constraint in the spec body says YDF "MUST use ≥1 feature category disjoint from Sense's five branches" — i.e. YDF needs at least one category that is *not* in Sense's set. But the verification's `disjoint_from_sense: bool` reduces this to a single truth value with no audit trail of which categories were chosen. An implementing agent could pick `{char, embed}` (both in Sense's set) plus one new category `{tfidf}` and pass the check on the technicality that `tfidf` is "disjoint", even though the YDF model would be ~80% feature-overlap with Sense — defeating the corroboration filter's independence assumption.
**Evidence:** spec.yaml lines 88–98 (ac-03 description), 99–106 (verification). evaluation_principles line 412 ("YDF's feature scope must be disjoint from ≥1 of Sense's five branches") weight 0.4 — load-bearing. The phrasing "disjoint from ≥1 of Sense's five branches" itself is ambiguous: "disjoint from at least one branch" (trivially true for any non-Sense feature) vs. "uses zero features from at least one Sense branch" (the principled reading).
**Recommendation:** Tighten the verification: emit `sense_branches_used: list[str]`, `sense_branches_excluded: list[str]`, `non_sense_categories_used: list[str]`. Assert `len(non_sense_categories_used) >= 1` AND `len(sense_branches_excluded) >= 1`. State the principled reading explicitly in ac-03's description: "YDF's feature pipeline must (a) exclude at least one of Sense's five branches entirely AND (b) include at least one feature category that is not in any Sense branch." The boolean reduces an audit-able structural property to an opaque assertion; the per-list form lets the spot-check (ac-12) and a future review judge whether independence is real.

### [MEDIUM] ac-12 spot-check sample size of 20 is too small to defend the corroboration filter
**Category:** test-gap
**Pass:** 2
**Description:** ac-12 spot-checks 20 randomly-sampled top-ranked gaps with a ≥18/20 (≥90%) pass threshold. The diagnostic ranks gaps by `(criterion × mechanism)` cell — 2 criteria × 10 mechanisms = up to 20 cells. A 20-sample audit stratified over 20 cells is at best 1 sample per cell; an unstratified random draw could leave 5–10 cells with zero spot-check coverage. The corroboration filter's failure modes are likely cell-specific (e.g. `enum_overfit` corroborated only by Sense + cascade may behave differently from `unknown_no_fit` corroborated by Sense + DBpedia). A 20-sample audit cannot distinguish "filter works in general" from "filter works in 12 cells and silently fails in 8."
**Evidence:** spec.yaml lines 323–342 (ac-12). ontology_schema fields (lines 382–408) describe the per-cell ranking; the spot-check does not stratify across cells.
**Recommendation:** Either (a) increase the sample size to 60 (3 per cell) with stratified sampling — 1 random gap per non-empty cell + 2 random gaps from the global top-N pool — or (b) change ac-12 to "≥3 gaps per non-empty (criterion × mechanism) cell, ≥90% pass per cell." Option (b) is the principled choice — it makes the spot-check a per-cell quality gate rather than a corpus-level smoke test. The 90% threshold should also be defended: at 20 samples, 18/20 = 90% but the binomial 95% CI is [68%, 99%], a band wide enough to hide real filter failures.

### [LOW] Storage estimate (2GB Parquet) under-states YDF + DBpedia overhead
**Category:** missing-requirement
**Pass:** 2
**Description:** implementation_notes estimate 10M rows × ~200 bytes = 2GB Parquet for `columns.parquet`. The schema in ac-04 includes `sample_values (truncated)`, `ydf_prediction`, `ydf_confidence`, `dbpedia_annotation`, `dbpedia_mapping_status`. Realistic per-row size with these columns + Snappy compression is closer to 400–800 bytes; corpus_pass storage is likely 4–8GB, not 2GB. `mechanism_decomposition.parquet` is a separate file. ac-04 also requires the per-column rows to include sample values, which at "≥3 example rows per gap" in ac-10 implies 3+ string samples per column — pushing the row size further.
**Evidence:** spec.yaml line 369 (storage estimate). ac-04 schema (lines 117–124) lists 11 fields including truncated samples and lens predictions.
**Recommendation:** Update the storage estimate to a realistic range (4–10GB). Not a blocking issue but matters for the agent's disk-budget call. Optional: add `sample_values_truncation_length: 64` (or similar) to the spec to bound the per-row sample size deterministically.

### [LOW] ac-01 disposition MADR slug is named but the MADR number is `NNNN`
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-01 says the disposition lands as `NNNN-gittables-diagnostic-absorbs-m19.yaml`. The next sequential MADR number is 0089 (last shipped per `.orbit/choices/` listing is 0086). The implementing agent will need to claim a number; the spec should pre-claim it or instruct the agent to pick the next available number with a documented rule.
**Evidence:** spec.yaml line 42–44 (ac-01 description). `.orbit/choices/` contains 0086 as highest numbered file (0083–0086 the Phase 2 inference module set).
**Recommendation:** Replace `NNNN` with the explicit number (0089 if 0087/0088 are unclaimed at spec start) or add a line: "implementing agent picks the next unused MADR number under `.orbit/choices/` and updates this AC with the claimed number." The first option is preferable — it removes a coordination point.

---

## Honest Assessment

The spec is conceptually sound and the multi-lens design is well-motivated by the discovery interview. The corroboration filter, mechanism-cascade decomposition, and two-criterion gate decomposition give the implementing agent a clear shape to work toward. The biggest risks are mechanical, not conceptual: (1) the spec does not currently parse under orbit-state's schema, so every downstream `orbit` verb fails until the `constraints:` block is moved; (2) the "full corpus" scope is unbounded and the row-count anchor is the holdout file, not a corpus index that does not yet exist; (3) the byte-identical reproducibility check in ac-13 contradicts the timestamp in ac-10's frontmatter. Each is a localised fix, not a re-design — REQUEST_CHANGES rather than BLOCK. The lens-independence verification (ac-03) and spot-check sample size (ac-12) are weaker than the principles they defend; tightening them protects the diagnostic's load-bearing claim that ≥2-lens agreement implies real-gap signal. Fix the schema parse failure first; it gates every other orbit verb the implementing agent will run.
