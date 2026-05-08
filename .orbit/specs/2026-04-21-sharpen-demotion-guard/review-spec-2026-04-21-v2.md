# Spec Review

**Date:** 2026-04-21
**Reviewer:** Context-separated agent (fresh session)
**Spec:** /Users/hugh/github/meridian-online/finetype/.orbit/specs/2026-04-21-sharpen-demotion-guard/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 (LOW) |
| 2 — Assumption & failure | content signals (model/eval pipeline, shared-crate API, Sharpen layer ordering) | 2 (LOW) |
| 3 — Adversarial | not triggered — Pass 2 surfaced no structural concerns, only minor operational notes | — |

## Findings

### [LOW] ac-01b TSV row-count tolerance is tighter than taxonomy accounting warrants
**Category:** test-gap
**Pass:** 1
**Description:** ac-01b's verification requires the precise-audit TSV to contain "240 rows, +/- 1". CLAUDE.md declares 240 definitions across 7 domains, which is the canonical total, but the audit's actual row count depends on which iteration path the integration test takes (per-definition-file, per-type, per-concrete-leaf). If the test walks `labels/definitions_*.yaml` as loaded structures, base-type aliases or synthesised types (e.g. locale variants, `decompose`-generated subtypes) may push the row count a few rows above or below 240 without signalling a real problem. A `±1` tolerance is brittle for a number the spec does not itself control.
**Evidence:** spec lines 56–65 (ac-01b description and verification); CLAUDE.md "240 definitions across 7 domains" line — matches the declared total, but does not constrain audit iteration semantics.
**Recommendation:** Soften the verification to "row count matches `finetype taxonomy` JSON total (ground truth) within ±5" or "row count equals the count of `Validation` objects constructable from `labels/definitions_*.yaml`". The exact number is not load-bearing; what matters is the audit covers every type. This is a one-line tweak and does not block implementation — filing as LOW.

### [LOW] ac-05 baseline commit identification assumes linear history
**Category:** assumption
**Pass:** 2
**Description:** ac-05's verification says "Run `git merge-base HEAD main` to identify the baseline commit". This is correct for a topic branch that branched cleanly from `main` and never rebased. If the implementation branch has been rebased onto a newer `main` during the sprint (which is likely given eval-expansion shipped the same day and further changes are plausible), `git merge-base HEAD main` will return a commit that may already include eval-infra changes not present when the implementer started work. The spec's intent — "re-baseline on the same commit the implementation branched from" — is correct; the mechanism (`merge-base`) may not faithfully execute that intent on a rebased branch.
**Evidence:** spec lines 122–124 (constraint that baseline MUST be re-run on the implementation branch's merge base with main); spec line 143 (verification command).
**Recommendation:** Add a parenthetical: "(If the branch has been rebased, use the pre-patch parent commit directly — e.g., `git log --oneline` to identify the first commit introduced by this spec, then `git rev-parse <first-commit>~1`.)" One-sentence clarification; no structural change.

### [LOW] ac-04 relies on `jq -e` returning a truthy non-empty match, but does not pin the output shape
**Category:** test-gap
**Pass:** 2
**Description:** ac-04's verification uses `jq -e '.columns[] | select(.column=="excel_format") | select(.type=="representation.file.excel_format")'`. This assumes `finetype profile -o json` emits a top-level `columns` array with objects keyed `column` and `type`. Given CLI JSON output has changed across versions (and `finetype-cli` exposes multiple output formats), the exact field names are worth pinning. If the current output uses `name` instead of `column`, or `label` instead of `type`, the `jq` expression will produce an empty result, `jq -e` will exit non-zero, and the AC will appear to fail for a cosmetic reason.
**Evidence:** spec line 112 (verification command); CLAUDE.md lists `finetype profile` output modes (`plain|json|csv|markdown|arrow`) but does not specify JSON schema in-file.
**Recommendation:** Before ac-04 runs, the implementer should run `finetype profile --file <path> -o json | jq '.columns[0]'` once and confirm the field names. If they differ, update ac-04's `jq` expression before starting implementation. Not a blocker — a 30-second check at implementation time — but worth flagging so it isn't surprising.

---

## Honest Assessment

v1.1 addresses every finding from v1 cleanly. The HIGH finding — `is_precise()` being hung off the wrong struct — is resolved by relocating to `Validation`, which is the right layer and matches where the raw `pattern`/`enum_values` actually live (verified against `crates/finetype-core/src/validator.rs`). The MEDIUM findings are all addressed: the rejected-pattern set is expanded with the specific loose patterns called out (`^[A-Za-z0-9 ]+$`, `^[\w\s]+$`, `^.{1,N}$`, `^[A-Za-z0-9_\-\.]+$`); the real-taxonomy-audit gap is closed by ac-01b (TSV + 3 real-pattern assertions); the delta-script contract is pinned with an exact `jq '.regressions'` expression and an explicit "schema drift blocks the AC" clause; the rollback/tightening branch is named in constraints and exit_conditions; ac-04 is explicitly end-to-end; ac-06 now gates on observed content, not section existence; and baseline re-run is required on the same merge-base commit.

Remaining findings are all LOW and operational — brittle row-count tolerance, `merge-base` assuming linear history, `jq` field-name assumptions in ac-04. None block implementation; each is a one-line tweak the implementer can address inline. Gate-AC verification check passes: ac-04 verification is 338 chars with explicit command; ac-05 verification is 408 chars with pinned delta command. Neither is a placeholder.

Biggest residual risk: the `is_precise()` predicate is the load-bearing heuristic, and while the rejected-pattern list is now substantially more thorough, regex taxonomies evolve. The ac-01b audit-on-CI mechanism is the correct structural answer — any future pattern addition that trips the predicate will surface as a PR-visible TSV diff. That is a durable safety net. Ship it.