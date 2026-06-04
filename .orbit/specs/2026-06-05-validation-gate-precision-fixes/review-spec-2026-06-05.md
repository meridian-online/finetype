# Spec Review

**Date:** 2026-06-05
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-06-05-validation-gate-precision-fixes
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 2 |
| 2 — Assumption & failure | content signals (taxonomy/schema change, eval gate, profile path) + Pass-1 findings | 3 |
| 3 — Adversarial | not triggered (no cascading/untestable ACs; findings are local and fixable) | — |

## Summary of verification

The spec's substrate checks out against the repo. Every pattern it names is as described:

- `datetime.timestamp.mdy_12h` pattern `^\d{2}/\d{2}/\d{4} \d{2}:\d{2} [AP]M$` — no `:SS` (labels/definitions_datetime.yaml:249).
- `datetime.timestamp.rfc_3339` offset `([+-]\d{2}:\d{2}|Z)` — colon-only (line 187).
- `datetime.time.iso` pattern `^\d{2}:\d{2}:\d{2}\.\d{6}$` with `minLength: 15 / maxLength: 15` (lines 893–895).
- `datetime.time.hms_24h` pattern `^\d{2}:\d{2}:\d{2}$` with `minLength: 8 / maxLength: 8` (lines 919–921).
- `datetime.component.year` pattern `^\d{4}$` (line 1252).
- `identity.person.gender` enum is `Male|Female|Non-binary|Other|...`, self-labelled "illustrative", `references: null` (lines 578–610).
- `identity.person.gender_code` pattern `^[MFX]$` with a description and notes that both claim `O` (lines 612–637) — the spec's "wrongly claims O" is correct.
- Eval gate enum match is `value not in self.enum_set` — case-sensitive (scripts/apply_ydf_validation_gate.py:107).
- Rust enum match is delegated to the `jsonschema` crate (exact, case-sensitive) via `CompiledValidator` (crates/finetype-core/src/validator.rs:49,97,137).
- The asymmetry direction is backed by memory `validation-gate-asymmetry` (NO reliable / YES unreliable; year `^\d{4}$` mis-veto called out explicitly), and the sibling spec 2026-06-04-value-level-ydf-labelling is `closed`.

The direction is sound and well-evidenced. The changes requested below are correctness gaps in two ACs, not a challenge to the plan.

## Findings

### [HIGH] ac-01 misses the length constraints on two of the five validations
**Category:** missing-requirement
**Pass:** 2
**Description:** `datetime.time.iso` carries `minLength: 15 / maxLength: 15` and `datetime.time.hms_24h` carries `minLength: 8 / maxLength: 8` (labels/definitions_datetime.yaml:894–895, 920–921). ac-01 widens only the `pattern`. The Rust validator enforces minLength/maxLength via the `jsonschema` crate, so after the pattern fix `00:07:13.580` (13 chars) still fails `time.iso`'s `maxLength: 15`/`minLength: 15`, and `01:03:29.000` (12 chars) still fails `hms_24h`'s `maxLength: 8`. The pattern widen is necessary but not sufficient — the length bounds must be widened (or dropped) in the same edit or the target value is still vetoed.
**Evidence:** validator.rs delegates pattern + length to jsonschema (`schema_validator.is_valid`, line 97). `time.iso` minLength/maxLength = 15; `hms_24h` = 8.
**Recommendation:** In ac-01, name the length-constraint edits explicitly: relax `time.iso` to `minLength: 9` (`HH:MM:SS.f`) `maxLength: 15`, and `hms_24h` to `maxLength: 12` (drop or widen minLength). Or state the rule generally: "widen pattern AND any minLength/maxLength that the new variants violate."

### [HIGH] ac-04 parity test will surface a length divergence ac-01 doesn't fix
**Category:** failure-mode
**Pass:** 2
**Description:** The eval gate deliberately ignores `minLength`/`maxLength` (scripts/apply_ydf_validation_gate.py:33 — "Length-only validations ... intentionally [skipped]"; `_compile_spec` reads only pattern/enum/minimum/maximum). The Rust validator enforces them. So even before this spec, `time.iso` and `hms_24h` are a latent parity mismatch on length. Once ac-01 widens the pattern but leaves `maxLength: 15`/`maxLength: 8` in place, the divergence becomes live and discriminating: `00:07:13.580` PASSES the eval gate (no length check) and FAILS the Rust validator (maxLength). ac-04's parity test ("a fixed value set through both, identical pass/fail") would then fail on exactly the values ac-01 is trying to rescue. ac-01 and ac-04 are in tension unless the length constraints are also fixed (see prior finding) or the eval gate is taught to honour length.
**Evidence:** eval gate ignores length (line 33, `_compile_spec` lines 157–208); Rust enforces it (validator.rs:97).
**Recommendation:** Resolve the length edit in ac-01, then state in ac-04 that the parity fixture must include the four datetime time/timestamp variants and the `1998.0` year case so the parity test actually exercises the divergence rather than passing vacuously.

### [MEDIUM] ac-02 case-folding enums is non-trivial in the jsonschema-backed validator
**Category:** failure-mode
**Pass:** 2
**Description:** The Rust enum check is the `jsonschema` crate's exact `enum` keyword — there is no case-insensitive flag, and lower-casing the input value before validation would corrupt any co-attached `pattern` or length check (and there are labels with both pattern and enum). The spec frames case-folding as "only ever ADDS matches, so no clean validation regresses", which is true semantically but understates the implementation: it likely requires either compiling word-enums to a case-insensitive regex (`(?i)^(male|female|...)$`) instead of a JSON-Schema enum, or a bespoke enum path that case-folds only the enum comparison. Either choice has a JSON-Schema-export consequence that ac-04 (parity) and ac-07 (MADR) should record. The "no clean validation regresses" claim also needs a guard: case-folding a short word-enum can newly ACCEPT a value that a different correct label would own — e.g. a single-letter or mixed-case token — so "adds matches" is only safe-by-construction for the veto direction, not for selection. Within the veto-only scope of this spec that is fine; it should be stated so the MADR doesn't over-claim.
**Evidence:** validator.rs:49,97,137 (jsonschema delegation, no case flag); labels carry pattern+enum jointly (eval-gate `passes()` applies both, scripts line 100–109).
**Recommendation:** ac-02 should name the mechanism (case-insensitive regex compile vs case-folded comparison) and add an assertion that labels carrying BOTH pattern and enum still apply the pattern case-sensitively. Keep the "adds matches" framing scoped to veto, not selection, in ac-07.

### [MEDIUM] ac-06 hard-veto in live profile is the only behaviour-changing AC and lacks a stated threshold/rollback
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-01..05 are taxonomy/test/measurement changes; ac-06 is the only one that changes what live `finetype profile` returns to a user (NULL/flag a predicted type when pass-rate < threshold). The pass-rate machinery exists (crates/finetype-model/src/validation_features.rs computes per-type pass-rate; validator.rs:345 `validate_column`), so feasibility is sound — but ac-06 leaves the veto **threshold** unspecified ("< threshold") where the canonical `ydf_prediction_gated` uses a fixed 50% (CLAUDE.md, scripts/gittables_corpus_pass.py `--fill-ydf`). It also doesn't say how a vetoed column surfaces to the user (NULL the type? emit a flag + keep the type? what does `profile` JSON show?), nor whether the veto is on by default or opt-in. A hard veto in the default profile path is a user-visible regression risk for any of the 152 unmeasured types if one slips into the "audited-safe" set by mistake.
**Evidence:** ydf_prediction_gated is 50% (CLAUDE.md "fewer than 50% of the column's sample values pass"); ac-06 says "< threshold" without pinning it.
**Recommendation:** Pin the threshold to the canonical 50% (or justify a different one), specify the profile output shape for a vetoed column, and state the default (recommend: advisory pass-rate surfaced for all types; hard NULL gated behind a flag or restricted to the audited-safe set — which ac-06 already half-says). Make "no measurable type silently becomes a hard veto" a testable assertion.

### [LOW] ac-01 mdy_12h `:SS` widening has no seconds in its format_string
**Category:** test-gap
**Pass:** 2
**Description:** ac-01 adds optional `:SS` to `mdy_12h`'s pattern so `06/30/2013 12:00:00 AM` validates, but the definition's `format_string` is `%m/%d/%Y %I:%M %p` (no `%S`) and `transform` strptimes with that format (labels/definitions_datetime.yaml:243–244). A value the pattern now accepts will fail the downstream `strptime` transform in `validate`/materialise. The veto fix is correct for the gate, but the round-trip (does the typed column actually cast?) is left inconsistent.
**Evidence:** format_string `%m/%d/%Y %I:%M %p`, line 243.
**Recommendation:** Either add a seconds-bearing `format_string_alt` (like rfc_3339 already does, line 181) or note in ac-01 that the pattern-vs-transform consistency is out of scope for this veto-only spec. State the choice; don't leave it implicit.

## Honest Assessment

The plan is ready in direction and well-grounded in substrate — the asymmetry framing, the per-validation triage, and the case-fold wins are all corroborated in the repo and the `validation-gate-asymmetry` memory. The biggest risk is mechanical, not strategic: ac-01 widens patterns but not the `minLength`/`maxLength` bounds that sit on two of the same definitions, and because the eval gate ignores length while the Rust validator enforces it, that omission turns ac-04's parity test into either a failure or a vacuous pass on exactly the values the spec exists to rescue. Fix the length constraints inside ac-01, pin ac-04's parity fixture to the rescued values, and pin ac-06's veto threshold and output shape. None of these is a rework — they are precise edits to four AC texts — hence REQUEST_CHANGES rather than BLOCK.
