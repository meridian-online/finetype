# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-28-profile-json-schema-output/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 2 (both LOW) |
| 2 — Assumption & failure | content signal: cross-system boundary (CLI ↔ MCP lockstep), shared config (helper module shape), backwards compatibility (README + sibling cards 0005/0006) | 2 (both LOW/INFO) |
| 3 — Adversarial | not triggered (no structural concerns from Pass 2) | — |

---

## Findings

### [LOW] ac-04 — clap option (a) vs (b) ambiguity is allowed but underspecified
**Category:** test-gap
**Pass:** 1
**Description:** ac-04 verifies behaviour ("`finetype profile -f f.csv -o json --stats` exits non-zero and prints a message containing `--stats requires -o json-schema`"). Implementation note 3 names two acceptable mechanisms — clap `conflicts_with`/`requires` (preferred) versus an explicit dispatch-time check. Both options can satisfy the verification string, but they yield different exit semantics — clap argument-validation errors typically exit code 2 (or whatever `clap::Error.exit()` returns) while a hand-rolled dispatch error yields whatever `Result<()>` lowering converts to (commonly 1). The spec doesn't pin an exit code; the test only requires "non-zero".
**Evidence:** spec.yaml ac-04 verification (line 70–73); implementation_notes line 187–191.
**Recommendation:** Acceptable as-is — "non-zero" is sufficient since the message string is the load-bearing assertion. No change needed unless the implementer wants determinism in the golden test exit code (in which case pin `assert_exit_code(2)` in the golden, which presumes option (a)).

### [LOW] ac-08 third test references `titanic` — verify fixture has the right shape for `--enum-threshold` toggling
**Category:** test-gap
**Pass:** 1
**Description:** `golden_profile_json_schema_enum_threshold_titanic` is named for the fixture, but the AC doesn't pin the threshold value at which `enum` flips on/off, nor the column it targets. The Titanic dataset has both low-cardinality columns (`Sex`: 2, `Embarked`: 3, `Pclass`: 3) and high-cardinality columns (`Name`, `Ticket`, `Fare`). Two candidate threshold values exist — one that lets all small categoricals through, one that excludes some. The test name promises "toggling" but the AC verification only says "the `enum` keyword toggling on `--enum-threshold`" without a concrete pivot.
**Evidence:** spec.yaml ac-08 lines 122–130; ac-08 verification line 131–134.
**Recommendation:** Implementer should pin a specific column + two threshold values (e.g., assert `Embarked` column has `enum` keyword at `--enum-threshold 5` but not at `--enum-threshold 2`) when authoring the golden. No spec change required — this is implementer-discretion territory.

### [LOW/INFO] Pass 2 — Round-trip degradation path is well-named but creates a silent-pass risk
**Category:** failure-mode
**Pass:** 2
**Description:** ac-10 names a degraded path: if card 0005's `--schema -` (stdin) hasn't shipped, the AC degrades to "helper output parses as valid JSON Schema (Draft 2020-12) via a dependency-free shape check." Per implementation_notes line 200–206, the rally serial order means card 0005 lands AFTER this card, so the degraded path is the realistic outcome at PR-time. The risk: a "shape check via `serde_json::from_str` + structural assertions named in ac-08" is exactly the same set of assertions ac-08 already makes — so ac-10 in degraded mode collapses to a duplicate of ac-08, not an independent gate. The round-trip property — that profile output is *consumable by validate* — won't actually be exercised before merge.
**Evidence:** ac-10 verification line 159–164; implementation_notes line 200–206.
**Recommendation:** Acknowledged risk, not a change request. The spec is honest about it ("note this in `progress.md` when the implementation enters review-pr"). The follow-up is structural: card 0005's spec/PR must exercise the round-trip from the consumer side and reference back to this card's output as its input. If that doesn't happen, surface in card 0005 review-spec.

### [LOW/INFO] Pass 2 — `OutputFormat` enum location consistency across CLI and MCP
**Category:** assumption
**Pass:** 2
**Description:** Constraint 4 says `OutputFormat::JsonSchema` is the enum variant; ac-01 places it in `crates/finetype-cli/src/main.rs`. Constraint 5 / ac-06 says the MCP `profile` tool gains a `format: json | json-schema` parameter — implementation_notes line 192–199 calls this `Option<ProfileFormat>`. So there are *two* enums: `OutputFormat` in the CLI crate, `ProfileFormat` in the MCP crate. They share the same kebab-cased CLI/JSON surface (`json-schema`) but are distinct types. This is fine — the helper module accepts a `bool stats` and `usize enum_threshold`, not the enum — but a reader scanning the spec might assume a single shared enum (`finetype-core::OutputFormat`?). Constraint 8 explicitly forbids promoting the helper to `finetype-core`, which implicitly forbids a shared enum too.
**Evidence:** spec.yaml constraints 4, 5, 8; ontology_schema lines 217–228; implementation_notes lines 193–195.
**Recommendation:** No change. The split is consistent with constraint 8 ("CLI-internal helper module, NOT promoted to finetype-core"). Just flagging the implicit two-enum design so the implementer doesn't try to share via re-export.

---

## Honest Assessment

This spec is implementation-ready. It traces cleanly: 11 ACs, each verifiable; constraints are non-conflicting; scope sits squarely inside the rally's serial order; the seed file (`cmd_schema_table` at `main.rs:2699+`) and target locus (`OutputFormat` enum at `main.rs:476-483`, dispatch arms at `main.rs:4386-4757`) match what's actually in the codebase (verified). The Pass-1 gate-AC structural check passes for both `ac-10` (165 chars, no placeholder tokens) and `ac-11` (90 chars, no placeholder tokens).

The biggest residual risk is the one the spec already names: ac-10's round-trip property is degraded to a shape check at PR-time because card 0005 lands later in the rally. That makes ac-10 effectively a duplicate of ac-08 in degraded mode. This is acceptable for this card — the round-trip becomes card 0005's responsibility to verify from the consumer side. If card 0005's spec doesn't include a "round-trip with `profile -o json-schema` output as input" AC, surface it in 0005's review-spec.

Surface symmetry is the strongest aspect of this spec: every `-o X` writes to stdout uniformly, `--stats` is the single new flag and is bounded by clap conflict, MCP gains the same capability in lockstep with the same PR. The "no silent shape drift" principle (evaluation_principle 1, weight 0.30) is enforced both by the byte-for-content constraint with PR #51's trimmed extensions and by the existing `golden_profile_*` tests staying green.

No structural concerns. Approve to implement.
