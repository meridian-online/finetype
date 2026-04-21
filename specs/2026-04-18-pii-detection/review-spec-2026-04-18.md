# Spec Review

**Date:** 2026-04-18
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-04-18-pii-detection/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [HIGH] Spec claims 6 PII types but YAML already has 15 — scope mismatch will break AC-01
**Category:** assumption
**Description:** The spec states "6 PII types: email, phone_number, phone_e164, ssn, pan_india, credit_card_number" and AC-01 expects `grep 'pii: true'` to return exactly 6 matches. However, the taxonomy YAML files already have 15 types with `pii: true`:

- `identity.person.full_name`
- `identity.person.first_name`
- `identity.person.last_name`
- `identity.person.email`
- `identity.person.phone_number`
- `identity.person.email_display`
- `identity.person.phone_e164`
- `identity.person.password`
- `identity.government.ssn`
- `identity.government.ein`
- `identity.government.pan_india`
- `identity.government.abn`
- `finance.banking.aba_routing`
- `finance.banking.bsb`
- `geography.address.full_address`

The spec's list of 6 is a subset of these. Implementing AC-01 literally ("grep returns exactly 6 matches") would require *removing* `pii: true` from 9 existing types (full_name, first_name, last_name, email_display, password, ein, abn, aba_routing, bsb, full_address). The interview explicitly excludes ABN and EIN, which is consistent — but says nothing about full_name, first_name, last_name, email_display, password, aba_routing, bsb, or full_address.

Removing PII flags from names, passwords, and addresses is a substantive decision that the interview doesn't document.

**Evidence:** `awk '/^[a-z].*:$/{key=$0} /pii: true/{print key}' labels/definitions_*.yaml` returns 15 keys. Spec line 9: "6 PII types". Interview Q1 answer lists only 6.
**Recommendation:** Either (a) update the spec to acknowledge the 15 existing PII types and keep them, adjusting AC-01 to expect the correct count, or (b) explicitly document and justify which types lose `pii: true` and why. The interview's "strict — direct identifiers only" rationale would exclude names (quasi-identifiers) and banking routing numbers (not personal), but this should be stated clearly in the spec.

### [HIGH] `credit_card_number` is listed as PII in the spec but does NOT have `pii: true` in the YAML
**Category:** assumption
**Description:** The spec lists `credit_card_number` as one of the 6 PII types (line 9). However, `finance.payment.credit_card_number` does not currently have `pii: true` in `labels/definitions_finance.yaml`. The two finance types that do have `pii: true` are `finance.banking.aba_routing` and `finance.banking.bsb` — neither of which appears in the spec's list.
**Evidence:** `grep 'pii' labels/definitions_finance.yaml` shows `pii: true` at lines 473 (aba_routing) and 511 (bsb) only. No `pii` field on credit_card_number (line 260).
**Recommendation:** Add `pii: true` to `finance.payment.credit_card_number` as part of implementation. This is clearly correct — a credit card number is a direct identifier. But the spec should acknowledge this is a new addition, not a pre-existing field.

### [MEDIUM] CLI type-level schema only emits `x-finetype-pii` when true — spec requires it always present
**Category:** constraint-conflict
**Description:** The spec's constraint (line 6) and AC-05 require `x-finetype-pii` to always be present (true or false, never omitted). The current CLI `build_json_schema()` at line 2735 only inserts the field when `pii == Some(true)`:
```rust
if def.pii == Some(true) {
    schema.insert("x-finetype-pii".into(), json!(true));
}
```
Non-PII types will have no `x-finetype-pii` field at all. This needs to change to unconditionally insert the field. The golden test `golden_schema_iso_date()` does not currently assert on `x-finetype-pii`, so there's no regression guard for this requirement.
**Evidence:** CLI `main.rs` lines 2735-2737. Golden test at line 649-659 has no PII assertion for iso_date.
**Recommendation:** The spec correctly identifies this gap — the implementation must change the conditional insert to unconditional `schema.insert("x-finetype-pii".into(), json!(def.pii.unwrap_or(false)))`. Add a golden test assertion for `x-finetype-pii: false` on `datetime.date.iso` to cover AC-05.

### [MEDIUM] CLI table-level schema and MCP schema tool do not emit `x-finetype-pii` at all
**Category:** assumption
**Description:** The spec's AC-03 and AC-04 require `x-finetype-pii` in table-level schema output from both CLI and MCP. Currently:
- CLI `cmd_schema_table()` (line 2748) builds per-property objects with `x-finetype-label`, `x-finetype-domain`, `x-finetype-confidence`, `x-finetype-broad-type`, `x-finetype-transform`, and `x-finetype-format-string` — but no `x-finetype-pii`.
- MCP `build_json_schema()` (schema.rs line 30) similarly omits `x-finetype-pii` entirely. The MCP table-level handler (`handle_file`, line 169) also omits it.

The spec correctly identifies these as files needing changes. The implementation task is real and straightforward, but the spec should note that this is new code, not a modification of existing PII handling.
**Evidence:** `grep 'x-finetype-pii' crates/finetype-mcp/src/tools/schema.rs` returns no matches. CLI table-level schema loop (lines 2924-3048) has no PII field insertion.
**Recommendation:** No spec change needed — the files listed in AC-03 and AC-04 are correct. Just noting that the implementation involves adding new code in 3 places (CLI type-level fix, CLI table-level addition, MCP type-level + table-level addition).

### [LOW] `finetype check` validation of `pii` field not mentioned in spec
**Category:** missing-requirement
**Description:** The interview success criteria mention "`finetype check` validates the new `pii` field in taxonomy definitions." The spec's exit conditions include "finetype check passes (taxonomy alignment unbroken)" but there's no AC for `finetype check` actually validating the `pii` field's values (e.g., ensuring it's only `true` or absent, never `false` as a YAML value — since the struct uses `Option<bool>`).
**Evidence:** Interview summary bullet 4: "`finetype check` validates the new `pii` field". Spec has no AC for this.
**Recommendation:** This is low severity because `serde` will handle the deserialization correctly, and the existing `finetype check` already validates taxonomy loading. No new AC needed unless the team wants `check` to report PII type counts.

### [LOW] Golden test gap: no table-level schema golden test
**Category:** test-gap
**Description:** The spec's AC-06 references golden integration tests for schema output. The existing golden tests (`golden_schema_email`, `golden_schema_iso_date`) only test type-level schema. There is no golden test for table-level schema output, so AC-03 (table-level `x-finetype-pii`) has no automated regression guard.
**Evidence:** `cli_golden.rs` lines 605-659 — only two schema tests, both type-level.
**Recommendation:** Consider adding a table-level schema golden test that profiles a small CSV with at least one PII column and one non-PII column, asserting `x-finetype-pii` on both. This would strengthen AC-03 coverage. However, table-level schema tests require model loading and are expensive, so the AC-02 verification command (manual CLI check) may be sufficient for this iteration.

---

## Honest Assessment

This is a well-scoped, low-risk spec for a straightforward feature. The core design decisions (taxonomy-driven, schema-only surface, always-present flag) are sound and well-documented in the interview. However, there is a significant factual error: the spec claims 6 PII types and the YAML already has 15. Implementing the spec as written would silently remove PII flags from types like `full_name`, `password`, and `full_address` — which may be intentional (the "strict direct identifiers" rationale supports it) but is not explicitly stated. The spec also claims `credit_card_number` already has `pii: true` when it does not. These discrepancies need to be resolved before implementation to avoid an unpleasant surprise where the implementer has to make undocumented decisions about which types keep or lose their PII flag. Once the count and the list are corrected, this spec is ready to ship.
