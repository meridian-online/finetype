# Spec Review

**Date:** 2026-04-29
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-iter3/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 (gate AC ac-14 passes deterministic check; content signal present) |
| 2 — Assumption & failure | content signal: eval corpus + ground-truth fixture | 0 |
| 3 — Adversarial | not triggered | — |

---

## Findings

None.

---

## Honest Assessment

v1.2 substantively resolves all 5 cycle-2 findings, and a fresh structural +
assumption-failure pass surfaces no new blockers. Verifying the cycle-2
fixes individually:

- **HIGH-1 (FIFA Nationality anchor):** ac-13 swaps the FIFA anchor to
  `Value, code_vs_canonical` and cites `eval/datasets/validate_corpus/fifa_players.gt.yaml`
  notes item (2) verbatim. I cross-checked the GT sidecar — lines 109-113
  name `Value/Wage/Release Clause` with €/M/K formatted-currency CvC
  exactly as the spec quotes. Source-of-truth precedence is now pinned
  explicitly: GT sidecar wins over iter-2's illustrative mismatch table.
  Anchor is now traceable to a delivered curation artefact.

- **HIGH-2 (GICS pre-escalated, undermines anchor framing):** ac-13 reframes
  openly as "4 hard anchors + 1 known taxonomy-gap row". The numerical
  test `vci3_fixture_anchor_count_4_hard_1_gap` enforces the framing —
  it asserts exactly 4 rows with `pending_escalation: false` matching
  the hard anchors. The GICS row carries `pending_escalation: true` and
  is correctly excluded from the count. Framing now matches the
  delivered evidence.

- **MEDIUM-1 (Phase-2 fixture rows still harness-derived):** ac-05 renamed
  to `vci3_fixture_attribution_regression_match`; rustdoc requirement
  in the verification field mandates the test contain "regression
  check" AND a reference to "ac-13" as the correctness anchor. The
  regression-vs-correctness distinction is now explicit in the test
  itself — buggy attribute() output recorded as "expected" in Phase 2
  is still possible, but the framing no longer over-claims.

- **MEDIUM-2 (ac-10 sub-criterion 1 trivially engineerable):** Sub-criterion
  1 now requires the `code_vs_canonical` ≥1 count to include at least
  one row matching a fixture entry with `expected_mechanism:
  code_vs_canonical` AND `pending_escalation: false`. Symmetric
  constraint on `format_diversity ≥1`. Allowlist tuning still in scope
  but cannot satisfy the count via pending-escalation rows or
  unblessed-by-fixture rows. Quality bar restored.

- **MEDIUM-3 (seam-table-guard misplaced):** ac-04 parenthetical at lines
  313-315 explicitly points the seam-table-guard test to ac-06
  cascade-order. ac-06 lines 376-386 add the Rule 5 vs Rule 4 case
  with implementer choice on whether it lives as a sub-test inside
  `vci3_attribute_cascade_order` or as a separately-named test.
  Cascade-ordering coverage now consolidated.

The gate AC verification field (ac-14) passes Pass-1's deterministic check
— well over 20 chars, no placeholder tokens, names ≥19 specific tests with
identifiers and counts. The two-pass commit phasing is enforced by ac-13's
git-log assertion (Phase 1 commit's diff contains exactly the 5 anchor
rows). The fixture-baseline pin (constraint 5 +
`vci3_fixture_row_count_baseline`) guards against silent shrinking; the
PR-description "Fixture diff rationale" requirement makes any change
visible at review-pr stage.

The remaining residual risk — that Phase-2 rationale-quality depends on
implementer hand-review without enforcement — is acknowledged in v1.2
metadata as the trade-off taken for MEDIUM-1, and ac-13's 4-hard-anchor
correctness check is the structurally-bounded compensating control. That
trade-off was Hugh's call (the cycle-2 review explicitly framed option
1 as the cheapest fix), and the v1.2 spec implements it cleanly.

The spec is ready for implementation. The biggest residual uncertainty is
empirical, not structural: whether allowlist tuning during Phase 2 can
yield ≥1 fixture-blessed `code_vs_canonical` and ≥1 `format_diversity`
hit on the iter-2 curated datasets without crossing into value-shape
escalation. The spec handles that uncertainty explicitly via constraint
4's escalation path and ac-13's `pending_escalation` mechanism — drive
won't re-litigate the design if escalation fires, it will document and
file a follow-up.
