# Spec Review

**Date:** 2026-04-20
**Reviewer:** Context-separated agent (fresh session)
**Spec:** `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml`
**Verdict:** REQUEST_CHANGES

---

## Findings

### [HIGH] Licensing reality vs goal scope
**Category:** constraint-conflict
**Description:** The goal claims "6 of the 7 types" will get real distilled data, but the constraints + ac-02 treat CPT as deferrable if licensing is unclear. Beyond CPT, LOINC is only "open" under a user agreement (Regenstrief terms forbid certain redistribution patterns); SWIFT BIC directory data is semi-restricted (SWIFT owns the registry; public free-tier lookups exist but bulk redistribution typically requires licensing); Excel format tokens are Microsoft copyright material. The only clearly-public-domain sources in this list are user_agent (from e.g. ua-parser fixtures, CC-0 projects) and http_method (any open HTTP log corpus / RFC).
**Evidence:** spec.yaml L14–16, L27–29, L83–85; interview Q15 ("LOINC, HTTP logs, Excel docs, ICAO" — but ICAO is airport codes, already covered, not BIC). No constraint forces the goal to degrade gracefully below 6 types.
**Recommendation:** Either (a) weaken the goal from "6 of 7" to "as many of LOINC / SWIFT BIC / Excel format / user_agent / CPT as licensing permits, with user_agent and http_method guaranteed" or (b) add a pre-flight AC: "Each source's license reviewed and approved by Hugh BEFORE the loader ships. Any source blocked drops out; goal auto-degrades." Also make the 235/242 gate robust to this — see the next finding.

### [HIGH] Eval gate brittleness under type dropout
**Category:** constraint-conflict
**Description:** The promotion gate is a hard number: v17 ≥ 235/242. That number was computed against v16's training corpus (7 types with synthetic-only). If only 2–3 of the 6 target types get real distilled data (because of licensing), v17 is essentially v16 with marginal additions plus N=1 email data-blend. The 235/242 bar could still be unmet for noise-level reasons. Conversely, if all 6 types get data, v17 might overshoot and the floor becomes irrelevant.
**Evidence:** spec.yaml L8, L43–45, ac-09 L161–166. The gate is absolute, not relative.
**Recommendation:** Make the gate conditional on scope: "If ≥ N of the 6 target types got distilled data, require ≥ 235/242. If < N, require ≥ (v16 score on corrected eval at time-of-run) + 0. No silent regressions below v16." Also capture the eval baseline at the moment v17 training starts, because v16's 235 number is evaluated against corrected GT that can continue to drift.

### [HIGH] HTTP method ENUM — unclear which surface it lives on; case variants may collide with pattern
**Category:** assumption
**Description:** The spec conflates two different surfaces when talking about the HTTP-method enum.
- `labels/definitions_technology.yaml` L283–286 already has BOTH `pattern: "^(GET|POST|…|CONNECT)$"` AND `enum: [GET, POST, …, CONNECT]`. These flow into `CompiledValidator` in `crates/finetype-core/src/validator.rs` and are case-sensitive, exact-string matches (confirmed by test at `validator.rs:912` where `"TRUE "` fails the boolean enum).
- The multi-branch "validation branch" (`crates/finetype-model/src/validation_features.rs`) consumes `CompiledValidator::is_valid` as a per-type pass-rate feature — it is a learned branch, not a filter.
- `finetype_validate` (DuckDB) / `finetype schema` (CLI) consume the same schema but with `is_valid` as a hard gate.

The interview (Q11) and spec ac-05 talk as if extending the enum "strengthens the validation branch" and "picks it up." In reality: expanding the enum makes the validation-branch pass-rate for `http_method` go from ~1/3 (case-sensitive uppercase only) to ~1.0 (all case variants), which changes the feature input the multi-branch model sees at training time. This IS useful — but it requires retraining to realise the benefit, and the pattern regex L285 will still fail non-uppercase, so the two validation keys will be inconsistent unless the pattern is also updated (or `i` flag added).
**Evidence:**
- `labels/definitions_technology.yaml:285-286` (pattern + enum both present, both uppercase-only)
- `crates/finetype-core/src/validator.rs:912-920` (enum is exact-string)
- `crates/finetype-model/src/validation_features.rs:1-10` (validation branch = pass-rate vector, learned input)
- interview.md Q11 treats these as equivalent
**Recommendation:** Split ac-05 into two concrete sub-ACs:
1. Schema change: update `labels/definitions_technology.yaml` L283–286 to include case variants in both `enum` AND `pattern` (or add `(?i)` / rework pattern). Assert `CompiledValidator::is_valid("Get")` returns true via a unit test in `crates/finetype-core/src/validator.rs`.
2. Downstream effects: retrain picks up the new validation-branch feature distribution. Assert v17 classifies `"Get"`-only columns as `http_method`, not via a separate enum "gate" (there is no such gate at the classifier level — the classifier is multi-branch, not rule-based).

Also correct the spec's language throughout to stop referring to "the validation branch picks it up" as if it were a filter.

### [HIGH] N=1 email data-blend is under-specified and may be architecturally incoherent
**Category:** missing-requirement
**Description:** The spec proposes fixing N=1 email via "data-blend (more single-value email rows)." But the multi-branch training pipeline operates on **columns**, not rows — it samples 100 values per column to produce the 4-branch features (per CLAUDE.md: "Sample 100 values, extract 4-branch features"). Adding 1-row email columns to the corpus would either:
- Get padded/resampled up to 100 identical values (degenerate), or
- Get dropped by a minimum-size filter (no effect), or
- Change the column-size distribution in ways that bias ALL types' training, not just email.

The interview (Q8, Q14 of Q&A, success criteria item 3) and N=1 email interview never resolve HOW the data-blend integrates with column-mode training. `orbit/specs/2026-04-20-v16-n1-email-regression/interview.md` L35–40 explicitly lists this as an open choice ("rule vs retrain") — the current spec has closed that question without specifying the mechanism.
**Evidence:**
- spec.yaml L46–49 ("N=1 email regression fixed via data-blend (more single-value email rows in training data)")
- spec.yaml ac-06 L125–135 (verification only checks behaviour, not mechanism)
- interview.md L157–158 (recipe listed as an open question, never closed)
- CLAUDE.md "Inference pipeline" section: "Sample 100 values, extract 4-branch features"
**Recommendation:** Either (a) close this in spec by specifying the concrete mechanism (e.g. "generate synthetic email columns of size 1..5 as part of the corpus, add a low-N augmentation pass during multi-branch training, target N new columns") with an AC asserting the augmentation exists in `scripts/prepare_multibranch_data.py` — OR (b) accept that this is unresolved and split the N=1 email fix back out of this spec (its own card), because bundling an unresolved mechanism into a retrain risks the retrain being blocked by the email issue. Given the low severity marked in the N=1 interview ("edge-case, not a user-common path"), (b) may be the cleaner answer.

### [MEDIUM] Decision 0049 treatment — amendment vs supersession
**Category:** test-gap
**Description:** ac-07 L142 says decision 0049 "is marked superseded/updated." But 0049's core thesis — "keep synthetic generators for the 7 types rather than dropping them" — is NOT being reversed by this spec. The spec is adding real distilled data ON TOP of the synthetic generators (which 0049 insisted on keeping). The `_DROP_DISTILLED_TYPES` update is a consequence of new distilled data existing, not a reversal of the synthetic-retention decision. Marking 0049 superseded would misrepresent the lineage; an amendment/date-modified update is more honest.
**Evidence:** spec.yaml ac-07 L137–146; decision 0049 L42–48 (synthetic retention is the core decision, not the distilled-drop list).
**Recommendation:** Replace "superseded" with "amended" in ac-07. Either (a) add a date-modified header + new "Update 2026-04-2X" section to 0049, or (b) write a new MADR that references 0049 and narrows its `_DROP_DISTILLED_TYPES` scope. Either way, do NOT set 0049's status to `superseded` — its core thesis remains accepted.

### [MEDIUM] "No broken chains" verification is manual/ambiguous
**Category:** test-gap
**Description:** ac-04 L101–112 says `data/label_remap.json` must have "no broken chains" with verification "run the label_remap validator (if one exists, else manual grep for unresolved labels)." This is a blind spot: there's no explicit requirement to BUILD a validator if one doesn't exist, and manual grep over a remap file with transitive chains (A → B → C) is an error-prone way to verify no orphans. The v16 sprint (per CLAUDE.md "Recent work") explicitly fixed broken remap chains — this was a real bug class.
**Evidence:** spec.yaml ac-04 L107–111; CLAUDE.md "v16 data audit" section mentions "fixed label_remap.json broken chains (description/title/sentence/paragraph → plain_text)."
**Recommendation:** Add an AC or amend ac-04: "`scripts/validate_label_remap.py` (or equivalent) exists and runs as part of data prep. It traverses transitive chains and fails if any label doesn't resolve to a canonical taxonomy key." Or cite an existing script if one is in-repo and just missing from the spec.

### [MEDIUM] 3-seed sweep compute: implicit, not cost-gated
**Category:** assumption
**Description:** The spec mandates "3-seed sweep, 100 epochs each, fresh." Per `scripts/sweep_v16.sh` L22, the v16 sweep was budgeted at **~2.5h per seed on M1 Pro with Metal** for 100 epochs — so ~7.5h wall-clock for the sweep alone, before profile eval and promotion. This is NOT mentioned in the spec. Nothing is wrong with the budget, but it's an implicit assumption that the trainer hardware and timeline accommodate an overnight run. If it's on a different machine or a slower one, 7.5h can balloon.
**Evidence:** spec.yaml constraint at L35–37 (methodology described, no time/resource budget); `scripts/sweep_v16.sh:22` ("Estimated time: ~2.5 hours per seed on M1 Pro with Metal").
**Recommendation:** Add an explicit note: "Estimated sweep cost: ~7.5h on M1 Pro with Metal (v16 precedent). Run as overnight job via `scripts/sweep_v17.sh` (adapted from `sweep_v16.sh`)." This also forces the question of whether `sweep_v16.sh` is adapted vs parameterised — interview Q (open) mentions this but spec doesn't close it.

### [MEDIUM] Val-acc gate of 91.2% against a new corpus is apples-to-oranges
**Category:** assumption
**Description:** The 91.2% val_acc gate (spec L39, ac-08) is v16's best. But v17's training corpus is DIFFERENT — it includes real distilled data for 6 types where v16 had synthetic-only. Val accuracy is a function of the corpus, not an absolute quality measure. If the new distilled data is harder (more variance, more edge cases) than the synthetic generators were, val_acc could legitimately drop below 91.2% while profile eval improves. The gate might then reject a model that would actually ship better.
**Evidence:** spec.yaml L38–41 ("Training gate: reject any checkpoint with val_acc < 91.2% (v16 best). Automatic guard against catastrophic data-quality regressions"); interview Q14.
**Recommendation:** Either (a) relax the gate — "reject val_acc < 88% (catastrophic floor); flag 88–91.2% for manual review" — or (b) document that a soft failure here triggers investigation, not automatic rejection, and rely on the profile eval gate (ac-09) as the true quality gate. The train-gate's purpose is to catch bugs (data corruption, wrong labels) before expensive eval, not to enforce absolute quality parity.

### [LOW] Per-seed model directory naming assumption
**Category:** failure-mode
**Description:** ac-08 verification says "`models/sherlock-v17-seed-{42,43,44}/results.json` exist." The v16 sweep script uses exactly this convention (per `models/sherlock-v16-seed-42/` etc. in the working tree), so it's a safe assumption — but if `sweep_v17.sh` is parameterised to accept a model-name prefix (interview open question), the name could drift. Not a blocker; flag it so the sweep script's output paths match the verification text.
**Evidence:** spec.yaml ac-08 L155–157; git status shows `models/sherlock-v16-seed-{42,43,44}/`.
**Recommendation:** Add the naming convention as an explicit expectation in ac-08, or make the verification method tolerant to `models/sherlock-v17-*/results.json`.

### [LOW] Per-column eval-diff file path assumes markdown extension
**Category:** test-gap
**Description:** ac-09 L166 refers to `specs/<this-spec>/eval-comparison.md`. This is fine, but the format (per-column diffs for 3 seeds × 242 columns) is not defined. It could be a table, a diff, a JSON report; reviewers won't know what "good" looks like.
**Recommendation:** Either specify the format (suggest: a markdown table per seed with rows for every mismatch + the v16 baseline column) or point to a prior spec's format as the template.

### [LOW] Homebrew tap + 5-platform binaries are CI-automated — ac-11 verification is redundant
**Category:** test-gap (minor)
**Description:** ac-11 L186–190 lists four verification steps (`cargo metadata`, `git tag -l`, GH release page, Homebrew formula). Per `orbit/specs/2026-04-20-v16-release/handover.md` these are all CI-automated on tag push. The AC is fine but the "verification" list reads as if a human were to check each manually; in practice the release skill / CI handles all four.
**Recommendation:** Minor copy edit — "verification: CI release workflow succeeds on tag push; `ls` / `gh release view` confirm outputs." Low priority.

### [LOW] SSN generator "improve if a coverage gap is identified" is vague
**Category:** missing-requirement
**Description:** spec.yaml L18–20 ("Improve the existing generator only if the spec review identifies a specific coverage gap"). Spec review (this document) doesn't own SSN coverage analysis. No AC asks whether the SSN generator is currently good enough. Either the question is being deferred implicitly, or it's silently waived.
**Recommendation:** Add a pre-flight AC: "SSN synthetic generator coverage reviewed against the v16 eval failure (`ssn` column in `people_directory`). Either documented as adequate or a specific improvement is captured as an AC." Or explicitly defer: "SSN generator unchanged; any improvement is a separate card."

---

## Honest Assessment

The spec is coherent at the strategic level — "source real distilled data for the 7 bad-distilled types and retrain as v17" is the correct follow-up to decision 0049, and the promotion flow cleanly reuses the v0.6.17 / CI-decouple precedent. The ontology and 12-AC structure are sound.

The biggest unresolved risks are three:

1. **Licensing attrition.** Only 2 of 6 source types (user_agent, http_method) are unambiguously open. If LOINC / SWIFT / Excel / CPT all fall to license review, v17 becomes "v16 + enum expansion + email fix," and the 235/242 eval gate becomes mechanically uncertain. The spec should plan for graceful degradation instead of quietly assuming full scope.

2. **HTTP-method ENUM confusion.** The spec and interview conflate three distinct surfaces (YAML schema, compiled validator, learned validation branch). Implementation will stumble over this unless ac-05 is split into schema-level and training-level sub-ACs and the language is cleaned up.

3. **N=1 email data-blend is under-specified.** Multi-branch training is column-scoped, not row-scoped; "add more single-value email rows" does not compose cleanly with 100-value column sampling. Either close the mechanism in this spec or split the email fix back out.

The 91.2% val-acc gate and "superseded" language for decision 0049 are secondary but worth fixing before implementation.

Recommend REQUEST_CHANGES: address findings 1–4 (all HIGH) before `/orb:implement`. Findings 5–11 can be cleaned up in the same pass.
