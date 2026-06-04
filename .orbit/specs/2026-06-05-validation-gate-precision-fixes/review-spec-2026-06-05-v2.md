# Spec Review — Cycle 2

**Date:** 2026-06-05
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-06-05-validation-gate-precision-fixes
**Cycle:** 2 (prior: review-spec-2026-06-05.md → REQUEST_CHANGES, four findings)
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 |
| 2 — Assumption & failure | taxonomy/schema change, eval gate, profile path + cycle-1 findings | 2 (both LOW, non-blocking) |
| 3 — Adversarial | not triggered (no cascading/untestable ACs; revised ACs are local and self-consistent) | — |

## What this cycle checked

Cycle 1 returned REQUEST_CHANGES on four findings plus one LOW. The job here is to judge
whether the revised spec resolves each, against the live repo — not against the prior
review's word. I re-verified every substrate claim the ACs rest on.

### Substrate re-verified (repo, not trust)

- `datetime.time.iso` — pattern `^\d{2}:\d{2}:\d{2}\.\d{6}$`, **`minLength: 15` / `maxLength: 15`** (labels/definitions_datetime.yaml:893–895). The length bounds cycle 1 flagged are real and still present.
- `datetime.time.hms_24h` — pattern `^\d{2}:\d{2}:\d{2}$`, **`minLength: 8` / `maxLength: 8`** (lines 919–921).
- `datetime.timestamp.rfc_3339` — offset `([+-]\d{2}:\d{2}|Z)`, colon-only (line 187); `format_string: %Y-%m-%d %H:%M:%S%z` with a `format_string_alt` for `Z` (lines 180–181).
- `datetime.timestamp.mdy_12h` — pattern has no `:SS`; `format_string: %m/%d/%Y %I:%M %p`, no `%S` (lines 243, 249). Deferral rationale holds.
- `datetime.component.year` — pattern `^\d{4}$`, `minimum: 1000` / `maximum: 2100` (lines 1252–1254).
- `identity.person.gender` — enum `Male|Female|Non-binary|Other|Prefer not to say|Unknown`, `references: null`, notes self-label "illustrative" (lines 578–610).
- `identity.person.gender_code` — pattern `^[MFX]$`; description (line 615) and notes (line 637) both cite `O` as valid — the "wrongly claims O" point is correct (lines 612–637).
- Eval gate **skips minLength/maxLength entirely** — `_compile_spec` reads only pattern/enum/minimum/maximum (scripts/apply_ydf_validation_gate.py:157–208); doc-comment confirms the deliberate skip (lines 33–36). Enum membership is exact: `value not in self.enum_set` (line 107). When pattern+enum co-occur, `passes()` applies **both** (lines 100–109).
- Rust validator **enforces** minLength/maxLength/pattern/enum via one jsonschema blob (`CompiledValidator::is_valid` → `schema_validator.is_valid`, validator.rs:95–97); `to_json_schema` bundles pattern+minLength+maxLength+enum together (taxonomy.rs:76–101). No case-insensitive enum flag exists.
- Profile/infer pass-rate machinery already exists: `validator_pass_rate()` (infer.rs:213) and `VALIDATOR_REJECT_THRESHOLD`-gated demotion (infer.rs:433–465). ac-06's deliverable has reusable building blocks; the missing piece is wiring a `validation_pass_rate` column + 50% veto into the `profile` path specifically.
- Notes substrate (notes.jsonl): the false-veto sweep (40 clean / 23 ≥10% of 78 measurable; 152 unmeasurable) and the deferred seconds-bearing-mdy_12h taxonomy follow-up are both recorded, matching the AC text.

## Cycle-1 findings — disposition

**[HIGH] ac-01 missed the length constraints — RESOLVED.**
ac-01 now opens with a CRITICAL clause requiring the pattern AND co-located length bounds to move together, and names each edit: time.iso "relax minLength/maxLength from fixed 15 ... (min 10, max 15)"; hms_24h "raise maxLength from 8 ... keep minLength 8". I checked the arithmetic against the parity fixture: shortest valid time.iso `HH:MM:SS.f` is `00:00:00.0` = 10 chars, so `min 10` is exactly right; the fixture's `00:07:13.580` (12 chars) lands inside 10–15. `01:03:29.000` (12 chars) requires hms_24h `maxLength ≥ 12`.

**[HIGH] ac-04 parity vacuity / length divergence — RESOLVED.**
ac-04 now (a) names the gate-skips-length vs validator-enforces-length divergence as "the canonical gap this spec must close", (b) pins the fixture to the exact rescued values (`00:07:13.580`, `01:03:29.000`, `1998.0`, `2020-03-19 15:34:31 -0800`, `thursday`/`male`), and (c) instructs "Reconcile whichever side is wrong" so the fixture passes on both engines rather than vacuously. The test now exercises the divergence instead of masking it.

**[MEDIUM] ac-02 case-fold mechanism — RESOLVED.**
ac-02 now states the no-flag reality, scopes the fold to the enum-membership comparison only (folding both enum set and compared value), keeps co-attached `pattern` byte-exact, mirrors the same scoped fold in the eval gate's `evaluate()`, and scopes the "only ADDS matches" property to the enum direction. The repo confirms the constraint is load-bearing: pattern+enum live in one jsonschema blob, so a correct implementation must lift the enum check out of that blob into a bespoke folded comparison. The AC names the correct end-state and the constraint precisely; the mechanism is left to implementation, which is acceptable for a spec.

**[MEDIUM] ac-06 threshold/output-shape/default — RESOLVED.**
ac-06 now pins the veto to the canonical 50% (matching `ydf_prediction_gated`), scopes the HARD veto to audited-safe labels (~40 sweep-clean + the ac-01/02/03 fixed set), keeps pass-rate ADVISORY-only for the 152 unmeasurable types, and requires the spec to state the output-shape change and the default-on decision. The "no unmeasured type silently becomes a hard veto" guard is now structural.

**[LOW] mdy_12h `:SS` round-trip — RESOLVED.**
mdy_12h is explicitly DEFERRED (not widened), with the round-trip rationale (`%I:%M %p` can't absorb seconds; strptime has no optional-seconds), filed as a taxonomy follow-up in notes.jsonl. ac-05 correctly predicts it stays high. This is the right call: widening the pattern without a transform that round-trips would re-create exactly the gate-vs-materialise inconsistency the prior LOW warned about.

All five cycle-1 findings are addressed at the AC-text level. No rework, no re-litigation.

## New findings (this cycle — both LOW, non-blocking)

### [LOW] ac-03 FHIR narrowing can REMOVE matches and re-introduce a veto
**Category:** failure-mode
**Pass:** 2
**Description:** ac-03 re-sources `identity.person.gender` to FHIR AdministrativeGender (`male | female | other | unknown`). The current enum carries `Non-binary` and `Prefer not to say` too. Narrowing to FHIR drops those, so a column legitimately holding `Non-binary` now fails the enum and would be vetoed — a *removal* of matches, opposite to ac-02's "only ADDS" framing. Within this spec's veto-only scope a stricter enum is defensible (FHIR is the standard), but the regression direction is real and unflagged. ac-05's no-regression guard (`NO previously-clean validation regresses`) could catch it empirically if gender was clean pre-change — but gender was at 22% veto pre-change, so the guard won't fire on it.
**Recommendation:** Note in ac-03 (or ac-05's analysis) that the FHIR narrowing is a deliberate enum *restriction*, and confirm against the sweep that dropping `Non-binary`/`Prefer not to say` doesn't push gender's post-fix veto back up. One sentence; no AC restructure.

### [LOW] ac-01 rfc_3339 `%z` round-trip for colon-less offset is asserted, not verified
**Category:** test-gap
**Pass:** 2
**Description:** ac-01 widens rfc_3339 to accept `[+-]\d{4}` and asserts "format_string `%z` already round-trips both forms — no transform change needed." The definition's own notes (line 205) say "DuckDB %z handles +HH:MM but not Z, hence the alt format" — i.e. `%z` behaviour is already known to be partial. The colon-less `-0800` case is distinct from `Z`, but the same class of trap (pattern accepts a form the transform can't cast) is what sank mdy_12h. ac-04's parity fixture tests the *validation* gate, not the *transform* round-trip, so a colon-less offset that validates but won't `strptime` would slip through.
**Recommendation:** Have ac-01 (or ac-04) confirm `strptime('2020-03-19 15:34:31 -0800', '%Y-%m-%d %H:%M:%S%z')` casts in DuckDB before treating the widening as transform-safe. If it doesn't, mdy_12h's deferral logic applies and rfc_3339's colon-less form is a taxonomy follow-up too. Low probability, cheap to check.

## Honest Assessment

The spec is ready. Every cycle-1 finding is resolved with precise AC-text edits, not hand-waving:
the length bounds are now co-edited with the patterns, the parity fixture is pinned to the rescued
values and forced to reconcile the length divergence, the case-fold is scoped to enum-membership
with the no-flag reality stated, ac-06 pins the 50% threshold and the advisory/hard split, and
mdy_12h is correctly deferred rather than half-fixed. I verified each against the live repo — the
substrate holds line-for-line.

The two new findings are both LOW and share a shape with the (correctly handled) mdy_12h deferral:
a widening that's safe for the *veto* may not round-trip through the *transform* (rfc_3339 colon-less
offset), and a standards re-sourcing can *remove* enum matches as well as normalise case (ac-03 FHIR
narrowing dropping Non-binary). Neither blocks implementation; both are one-sentence guards the
implementer should fold in opportunistically, and ac-05's empirical sweep is the natural place to
catch a regression if one surfaces. APPROVE — proceed to implement, carrying the two LOW notes as
implementation guards rather than spec blockers.

**One-line for the author:** Cycle-2 spec is sound and implementable — all four cycle-1 fixes landed; two leftover LOW notes (rfc_3339 colon-less transform round-trip, ac-03 FHIR enum narrowing dropping "Non-binary") are implementation guards, not blockers.
