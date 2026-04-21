# Spec Review

**Date:** 2026-04-21
**Reviewer:** Context-separated agent (fresh session)
**Spec:** /Users/hugh/github/meridian-online/finetype/orbit/specs/2026-04-21-sharpen-demotion-guard/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | content signals (model/eval pipeline, shared crate API) + 1 HIGH finding in Pass 1 | 3 |
| 3 — Adversarial | Pass 2 exposed structural ambiguity around how `is_precise()` can access the regex/enum it must inspect | 2 |

## Findings

### [HIGH] `CompiledValidator` does not currently expose pattern or enum fields — ac-01 is not implementable as written
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-01 requires `CompiledValidator::is_precise(&self) -> bool` to inspect the regex pattern string against a rejected-pattern list (`^.+$`, `^.*$`, `^\S+$`, `.+`, `.*`) AND to check whether an enum list is non-empty. But the actual struct at `crates/finetype-core/src/validator.rs:47-54` has only three fields: `schema_validator: jsonschema::Validator`, `minimum: Option<f64>`, `maximum: Option<f64>`. The raw `pattern` string and `enum_values` list are consumed by `Validation::to_json_schema()` during `new()` and then discarded — there is no `self.pattern` or `self.enum_values` to read. The spec writes ac-01 as if these fields exist (`the validator's regex pattern contains…`, `enum-constrained (non-empty enum list)`), but the implementer will discover on first contact that `is_precise()` cannot be written without one of three additional changes the spec does not call out:
  1. Retain the original `Validation` fragment (or the pattern/enum fields) on the struct, OR
  2. Move `is_precise()` to `Validation` (not `CompiledValidator`) and plumb it through, OR
  3. Reconstruct patterns/enums from the compiled `jsonschema::Validator` (non-trivial and fragile).
**Evidence:** `crates/finetype-core/src/validator.rs` lines 47–54 (struct definition) and 61–74 (`new()` drops pattern/enum after compilation); spec lines 24–34 (ac-01 presumes access to those fields). The spec's parenthetical "(or wherever CompiledValidator is defined — inspect during implementation)" hints the author did not verify struct internals; the location is already known (the finding above cites it), but the shape is not what ac-01 assumes.
**Recommendation:** Update ac-01 to either (a) explicitly require adding the needed fields to `CompiledValidator` (document which — `pattern: Option<String>`, `enum_values: Option<Vec<String>>`), or (b) relocate `is_precise()` to `Validation` and change ac-02 to call `taxonomy.get_validation(label).map(|v| v.is_precise())` instead. Option (b) is cleaner — `Validation` already holds the raw pattern/enum and is the right layer for a structural-precision predicate. This is a 5-minute spec edit but a blocker if caught only at implementation.

### [MEDIUM] `is_precise()` predicate is permissive — many nominally "anchored" patterns do not actually constrain
**Category:** test-gap
**Pass:** 2
**Description:** The spec defines "precise" as: pattern is not in the rejected set `{^.+$, ^.*$, ^\S+$, .+, .*}` AND the pattern does not consist solely of `.`, `\w`, `\S`, `\S+`, `\w+` within its anchors. Several real taxonomy patterns that *are* functionally permissive would still pass this check:
  - `^[A-Za-z0-9 ]+$` (matches almost every short identifier)
  - `^[\w\s]+$` (word chars + whitespace — matches virtually all short English strings)
  - `^.{1,100}$` (anchored, has char class `.`, length bounded, but matches anything printable)
  - `^[A-Za-z0-9_\-\.]+$` (matches slugs, filenames, many codes simultaneously)
These patterns will cause the guard to fire for "named types with loose validators" that the spec's own rationale (Q2, discovery finding #2) explicitly warns against. The rejected-pattern allowlist is small enough that a real pattern set audit will find more bypasses.
**Evidence:** spec lines 27–33 define the predicate; CLAUDE.md Precision Principle ("A validation that confirms 90% of random input is not a validation") and discovery finding #2 ("25+ types return pass_rate=1.000 on short strings") both warn against exactly this category. `labels/definitions_*.yaml` contains many regex patterns — the spec does not require auditing them.
**Recommendation:** Either (a) add an AC requiring a one-time audit that enumerates every `^…$` regex in the taxonomy and classifies each as precise/imprecise under the proposed predicate, with imprecise cases either relaxed-further or excluded by explicit regex-text matching; or (b) strengthen `is_precise()` with a stricter rule — e.g., reject any pattern whose character class after anchor-stripping contains `.`, or whose implied max-length exceeds some threshold. Minimal version: add one AC "unit tests include 3 real-taxonomy borderline patterns sampled from `labels/definitions_*.yaml`" so the predicate is tested against reality, not just synthetic examples.

### [MEDIUM] ac-05's "zero regressions" gate is measured by a script whose semantics are not pinned in-spec
**Category:** test-gap
**Pass:** 2
**Description:** ac-05 requires `regressions == 0` as measured by `scripts/eval_delta_by_coverage.py`. The spec refers to this as "the delta script used in the v17 re-eval" but does not quote or reference its exact output schema, does not state which field name = "regression", and does not define how the script behaves when the pre-patch baseline (`profile_results.csv`) is not attached or is stale. If the script's default semantics change (e.g., it gains a "soft regression" category or changes column names), the AC's verification command silently becomes unreliable. For a no-retrain Sharpen-layer patch, eval delta is the primary safety gate — its machine contract should be pinned.
**Evidence:** spec lines 81–87 reference the script but do not freeze its output schema; the spec lives or dies on whether this script correctly attributes regressions. Adjacent MADRs 0055–0058 treat similar measurement contracts as first-class (row-hash tables, coverage gate, realism floors).
**Recommendation:** In ac-05, either (a) quote the exact `jq`/`awk` expression over the script's CSV output that the implementer must run to extract the "regression count" (making the contract explicit), or (b) add a one-line "script-output schema" constraint listing the three fields the AC depends on (`label_correct`, `domain_correct`, `regressions`), with a note that the AC is void if the script's schema has drifted.

### [MEDIUM] No rollback plan if full eval shows unexpected regression mid-landing
**Category:** missing-requirement
**Pass:** 2
**Description:** The spec asserts `zero regressions` but says nothing about what happens if ac-05 fails. Given the guard runs for *every* column that reaches `disambiguate_categorical` across the 448-row eval, a collateral regression is the realistic risk mode, not an implementation bug. The spec has no explicit "if ac-05 regresses, revert the commit and widen the precise-validator definition" clause, and MADR 0059's status ladder (`proposed` → `accepted` after ac-05) implicitly assumes ac-05 passes on the first attempt.
**Evidence:** exit_conditions (lines 155–159) list "no regression" as a requirement but not a contingency; no AC covers the "eval shows N regressions, now what?" branch. Precedent: the v17 hold (MADR 0054) documents exactly this style of mid-landing pivot.
**Recommendation:** Add an operational note (not necessarily a new AC) either to `progress.md`'s required sections or to constraints: "If ac-05 shows regressions > 0, the guard predicate is tightened (additional patterns blacklisted in `is_precise()`) and the eval is re-run; if no tightening reaches regressions == 0 without losing the `excel_format` fix, the spec is paused and a follow-up discovery card is opened — MADR 0059 remains `proposed`." This makes the decision branch explicit instead of implicit.

### [LOW] Sampling scope for "every non-empty sampled value passes" is not pinned
**Category:** assumption
**Pass:** 2
**Description:** ac-02 says "every non-empty sampled value passes the validator via `is_valid()`". The sample set at the `disambiguate_categorical` callsite is whatever the caller provides — the inference pipeline currently samples up to 100 values per column (per CLAUDE.md). If the sample size or the "non-empty" filter semantics shift (e.g., future change to sampling strategy), the guard's behaviour changes silently. The spec should pin whether "non-empty" means `!s.is_empty()` or `!s.trim().is_empty()`, and whether the guard operates on the already-sampled slice passed into the function.
**Evidence:** spec line 42–44 (ac-02); column.rs:3881 signature `fn disambiguate_categorical(values: &[String], top_labels: &[&str])` — the function receives a pre-sampled slice, so the guard inherits whatever sampling + filtering the caller did.
**Recommendation:** Tighten ac-02: "non-empty" means `!s.trim().is_empty()` (or whichever convention matches existing Sharpen rules in column.rs — inspect and match). Also clarify that the guard operates on the `values` slice as-passed, not on the raw column.

### [LOW] ac-06 verification is unfalsifiable by design — rename to signal intent
**Category:** test-gap
**Pass:** 2
**Description:** ac-06 explicitly says "This AC never fails — its purpose is to preserve evidence, not to gate shipping." Having a `doc` AC that cannot fail is defensible, but the verification wording ("Inspect progress.md — confirm a section named `http_method outcome` exists…") *is* falsifiable if the section is missing. The description and verification contradict each other mildly.
**Evidence:** spec lines 89–101.
**Recommendation:** Either strengthen the description to "AC passes iff the `http_method outcome` section exists in `progress.md` and records the observed label, confidence, and next-step decision" (making it genuinely gating on documentation quality) — or downgrade to a spec-body "required deliverable" note and remove from acceptance_criteria. Preference: strengthen; the evidence-preservation principle (weight 0.10) is worth a real gate.

### [LOW] Pass-3 cascade: Fix could succeed in isolation but be masked by a concurrent eval shift
**Category:** failure-mode
**Pass:** 3
**Description:** The ac-05 baseline (297/352 label, 326/352 domain) is stated as "v16 baseline on the same corpus". If between spec-freeze and ac-05 execution any *other* change lands on main that alters the eval (e.g., a manifest row edit, a row-hash firewall adjustment, a taxonomy tweak), the delta script will attribute unrelated drift to this patch. The expanded-eval infra (MADRs 0055–0057) shipped days ago and is still settling; concurrent changes are plausible.
**Evidence:** CLAUDE.md "Recent work" shows eval-expansion shipped 2026-04-21, same day as this spec; row_hashes.tsv and sources.yaml are fresh artefacts; no freeze is declared.
**Recommendation:** Require ac-05 to re-baseline on the *current* HEAD of the pre-patch branch (i.e., run `profile_eval.sh` twice — once pre-patch, once post-patch — on the same commit's eval infra), and attach both `profile_results.csv` snapshots (already specified at line 87 — good). Add: "baseline re-run must be within same commit as implementation branch point; stale baselines invalidate the AC."

### [LOW] Adversarial: guard interaction with other demoters not bounded
**Category:** failure-mode
**Pass:** 3
**Description:** Constraint 1 says "Other Sharpen rules (attractor demotion, header_sharpen, value_sharpen's other branches) are untouched in this spec." But the guard preserving a label in `disambiguate_categorical` does not prevent a *downstream* Sharpen rule from demoting it via a different mechanism. If `excel_format` is rescued here but then demoted by `apply_header_sharpen` or a value_sharpen branch, ac-04's targeted CLI check will catch it, but ac-05's delta script will see "net zero" and the team will think the guard works. Hidden dependency between ACs: ac-04 is the only gate that actually proves the guard's end-to-end effect on the canonical failure case.
**Evidence:** CLAUDE.md pipeline description (Sharpen post-processing: feature_sharpen → value_sharpen → apply_header_sharpen) implies ordering; spec lines 11, 17 claim other rules are untouched but do not assert non-interference.
**Recommendation:** ac-04 already covers this — but make it explicit in the AC: "This AC specifically tests end-to-end CLI output (post all Sharpen layers), not just `disambiguate_categorical`'s return value. If `excel_format` is rescued in `disambiguate_categorical` but demoted by a downstream Sharpen rule, ac-04 fails and the spec does not ship." One-sentence addition; makes the cascade explicit.

---

## Honest Assessment

The spec is carefully written, traceable to discovery evidence, and appropriately narrow in scope — the pivot from "promotion rule" to "demotion guard" is well-justified and the constraints are tight. But there is one HIGH finding that will block implementation on day one: `CompiledValidator` does not hold the fields `is_precise()` needs to inspect, so ac-01 as written is not directly implementable. That is a 5-minute spec edit (relocate `is_precise()` to `Validation`, or add pattern/enum fields to `CompiledValidator`), but it must happen before implementation starts. The MEDIUM findings cluster around a single deeper risk: the "precise validator" predicate is the load-bearing heuristic for the whole spec, and it is defined by a short rejected-pattern allowlist that has not been cross-checked against the actual taxonomy's regex set. If the predicate turns out to accept a few loose patterns (plausible on inspection of typical `^[A-Za-z0-9_]+$`-style regexes), the guard will fire on them and introduce exactly the regressions ac-05 is designed to catch — but without a pre-audit, the team will discover this only after running the full eval. Biggest risk: ac-05 comes back red, and the spec has no explicit rollback/tightening path. Request changes rather than block — the structure is sound, the fixes are small, and the evidence trail is strong.

