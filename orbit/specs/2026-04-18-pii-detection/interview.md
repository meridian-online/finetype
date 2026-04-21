# Design: PII Detection

**Date:** 2026-04-18
**Interviewer:** Nightingale
**Card:** orbit/cards/0012-pii-detection.yaml

---

## Context

Card: *PII detection* — 3 scenarios, references: schema.mdx docs, definitions_identity.yaml

## Q&A

### Q1: PII scope
**Q:** Which types should carry the PII flag? Strict (direct identifiers only) vs broad (includes quasi-identifiers like names, addresses) vs tiered (direct/quasi levels)?
**A:** Strict — direct identifiers only. High precision, no edge case debates. 6 types after review: email, phone_number, phone_e164, ssn, pan_india, credit_card_number. Excluded: ABN (public business ID), EIN (employer/business ID), IBAN (account ID, not personal), NPI (provider ID, publicly searchable).

### Q2: Storage location
**Q:** Where should the PII flag live? YAML field per definition vs hardcoded list in code?
**A:** YAML field. Add `pii: true` to the ~9 definitions. Taxonomy as source of truth, visible and auditable.

### Q3: Output surfaces
**Q:** Which surfaces should show the PII flag in this iteration? Schema only, schema + profile, or all three (schema + profile + DuckDB)?
**A:** Schema only. `finetype schema` JSON output and MCP schema tool. Profile and DuckDB come later. Matches the card's "emerging" maturity.

### Q4: Absent vs explicit false
**Q:** Should x-finetype-pii be omitted for non-PII types, or always present as true/false?
**A:** Always present. Every property gets `x-finetype-pii: true` or `x-finetype-pii: false`. More explicit, easier to query.

---

## Summary

### Goal
Add `x-finetype-pii` to JSON Schema output based on a `pii` field in taxonomy definitions. Direct identifiers only — no quasi-identifiers, no scanning, no model changes.

### Constraints
- Strict PII scope: only types that uniquely identify a person
- YAML field `pii: true` on ~9 type definitions
- Schema output only (not profile or DuckDB in this iteration)
- Always present in schema output (true or false, never omitted)
- No model changes, no eval changes

### Success Criteria
- `finetype schema <file>` includes `x-finetype-pii: true/false` on every property
- `finetype schema <type-key>` includes `x-finetype-pii` for single-type schemas
- MCP `schema` tool includes the flag in its JSON output
- `finetype check` validates the new `pii` field in taxonomy definitions
- 6 types flagged: email, phone_number, phone_e164, ssn, pan_india, credit_card_number

### Decisions Surfaced
- **Strict PII scope**: chose strict (direct identifiers) over broad (quasi-identifiers) or tiered (direct/quasi levels) because high precision matters more than coverage for compliance use cases. No debates about whether IP addresses or names are PII.
- **Taxonomy YAML field**: chose `pii: true` in YAML over hardcoded list in code because taxonomy is the source of truth for all type metadata.
- **Schema-only surface**: chose to limit to schema output over full rollout (profile + DuckDB) to keep the first ship tight. Profile/DuckDB are future iterations.
- **Always-present flag**: chose explicit `true/false` over omission for non-PII because it makes filtering easier in downstream tools.

### Open Questions
- None — scope is clear and tight.
