# Design: validate-corpus iter-4 — finance.currency.amount bare-decimal widening

**Date:** 2026-04-29
**Interviewer:** Nightingale
**Card:** orbit/cards/0014-profile-validate-precision.yaml

---

## Context

Card 0014 — *Profile-validate precision corpus* — 9 scenarios, goal: round-trip
pass at P=99% on real-world CSVs; in-scope fixes are enum-overfit (profile-side)
and validator-format-diversity (taxonomy-side); misclassification + code-vs-canonical
defer to follow-up cards.

**Prior specs (3, all shipped):**

- **iter-1** (`2026-04-28-validate-precision-corpus/`) — round-trip harness
  (`make validate-corpus`) over 7 real-world CSVs; 3-of-7 datasets pass at P=99%.
  Two fixes shipped: ac-09 enum threshold 50→32 + boolean gate parity (enum-overfit
  bucket); ac-10 scientific notation widening on `representation.numeric.decimal_number`
  (validator-format-diversity bucket).
- **iter-2** (`2026-04-28-validate-corpus-curation/`) — 5 mechanism-coverage
  datasets added (NYC Taxi, GDELT, FIFA, S&P 500, OECD employment); 12 datasets
  / 46622 rows total, 3-of-12 pass.
- **iter-3** (`2026-04-28-validate-corpus-iter3/`) — cascade refactor: 7 explicit
  rules with 6 trigger labels; 80-row fixture as anti-regression lock with
  `pending_escalation` flag; analyst-facing doc; 21 `vci3_*` tests. Six MADRs
  total across the arc (0072–0077).

**Discovery vehicle:** `eval/datasets/csv/ecommerce_orders.csv` — synthetic test
data Hugh ran `finetype profile -f data.csv -o json-schema | finetype validate`
against. Three failures surfaced:

| Column | GT label | Model predicted | Mechanism | Why |
|---|---|---|---|---|
| `total_price` (63/100) | price | `finance.currency.amount` | format_diversity | Regex requires `[0-9]{1,3}(,[0-9]{3})*` — comma-grouped thousands. `1914.96` (4-digit, no comma) fails. |
| `status` (100/100) | status | `datetime.component.periodicity` | misclassification | Model picked periodicity (enum: Once/Daily/Weekly/...). Values shipped/pending/delivered/returned/cancelled all reject. |
| `order_id` (100/100) | code | `finance.securities.sedol` | misclassification | SEDOL regex `^[B-DF-HJ-NP-TV-Z0-9]{6}[0-9]$` rejects everything; model latched onto code-shape. |

**Card-fit analysis:**

- `total_price` is **format_diversity** — in scope by card mandate.
- `status` and `order_id` are **misclassification** — explicitly deferred per
  card goal ("misclassification and code-vs-canonical defer to follow-up cards").

iter-4's mandate is therefore narrow by card design: ship the validator widening
for `total_price`, document the two misclassifications as discovery findings,
file a follow-up card for the model-classification work.

## Q&A

### Q1: Scope
**Q:** What's the scope of iter-4 — narrow (regex only), standard
(regex + record misclassifications), or broad (regex + curate real-world
e-commerce CSV into the corpus)?

**A:** Standard — regex + record misclassifications. Adds a paragraph
in `progress.md` naming the two misclassifications (status→periodicity,
order_id→SEDOL) as deferred-to-retrain findings. No fixture changes
(no real-world dataset to anchor against). One follow-up card created
at end of spec.

### Q2: Precision floor for finance.currency.amount
**Q:** Given regex header hints are off the table (MADR 0042/0048),
what's the precision call for `finance.currency.amount` widening?

**A:** Hugh corrected the framing — *"is it possible there's a canonical
source of regex definitions? I'd prefer not to invent various regex
definitions when there's good prior art."*

**Resolution:** External canonical sources for bare-decimal money formats
don't exist (CLDR is locale-aware DSL not regex; there's no IETF/ISO
standard for the unsymbolled form). Internal canonical source DOES
exist: `representation.numeric.decimal_number` at
`labels/definitions_representation.yaml:79` carries the pattern
`^-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$` — exactly the bare-decimal shape
we need. iter-1 ac-10 already widened this pattern to add scientific
notation; iter-4 borrows the same pattern verbatim as a 4th alternation
in `finance.currency.amount`.

### Q3: Reuse shape
**Q:** Reuse `decimal_number`'s pattern verbatim (including sci-notation
tail), reuse non-sci portion only, or tighter (require fractional part,
1–4 decimal digits)?

**A:** Reuse verbatim, sci-notation included. No invention, no fork.
`1e6` validating as amount when the model predicted amount is acceptable
edge case — the model is the precision gate; the validator's job is
shape confirmation.

### Q4: Locality
**Q:** Single-file change in `labels/definitions_finance.yaml` or
refactor amount + decimal_number to share a bare-decimal fragment?

**A:** Single-file change. YAML doesn't support imports natively; FineType's
taxonomy treats each entry as self-contained. Add a comment referencing
`decimal_number` as the source. Single-LOC change in the union.

### Q5: Discovery dataset
**Q:** How should iter-4 treat `ecommerce_orders.csv` — don't add to corpus,
add as synthetic carve-out, or replace with real-world equivalent?

**A:** Don't add. Synthetic per MADR 0055 (sequential `ORD-` IDs,
`alice26@outlook.com`, `example.com` URLs); doesn't meet realism floor.
Stays as Hugh's workflow-test scaffolding. iter-4 demonstrates the fix
on it (before/after 63→0 rejects) but corpus-level evidence comes from a
future real-world dataset.

---

## Summary

### Goal

Widen `finance.currency.amount` validation to accept plain bare-decimal
values via reuse of `representation.numeric.decimal_number`'s canonical
regex. Demonstrate fix on the discovery vehicle (ecommerce_orders.csv:
total_price 63 rejects → 0 rejects). Record the two misclassification
findings (status→periodicity, order_id→SEDOL) as deferred-to-retrain;
file follow-up card.

### Constraints

- Single-file regex change in `labels/definitions_finance.yaml` at line 131.
- Reuse `decimal_number`'s pattern verbatim (incl. sci-notation tail) — no
  invention, no taxonomy-shared snippet refactor.
- No regex header hints (MADR 0042). No new value-based rules (MADR 0048
  scope — value-based rules are a later expansion, out of iter-4 scope).
- `ecommerce_orders.csv` is NOT added to `eval/datasets/validate_manifest.csv`
  (synthetic, fails MADR 0055 realism floor).
- iter-3 fixture lock (MADR 0076) must continue to pass — no row-level
  attribution drift on iter-3's 12 datasets allowed.
- `make ci` exit 0; `cargo clippy -- -D warnings` clean; `finetype check`
  taxonomy alignment clean.

### Success Criteria

- The amount pattern at `definitions_finance.yaml:131` gains a 4th
  alternation: `^-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$` (verbatim from
  `decimal_number`).
- A regression test (`vci4_*` prefix) verifies bare-decimal values
  (`1914.96`, `19.95`, `100`, `-50.5`) match `finance.currency.amount`
  AND that the existing US-format/EU-format/accounting-paren values
  continue to match.
- Demonstration: running `finetype validate eval/datasets/csv/ecommerce_orders.csv
  schema.json --db tmp.db --table orders` shows total_price reject count
  drop from 63 → 0.
- iter-3's `vci3_fixture_attribution_regression_match` test continues
  to pass — no row-level attribution drift on the 12 corpus datasets.
- Two misclassification findings recorded in `progress.md` as deferred,
  with diagnostic detail (predicted label, actual values, why the model
  failed, what fix path applies).
- Follow-up card filed at `orbit/cards/NNNN-status-and-orderid-misclassification.yaml`
  (or similar) capturing the two retrain-required cases. Card slots under
  the model-improvement umbrella, references this spec.

### Decisions Surfaced

- **Reuse `decimal_number`'s canonical bare-decimal regex in `finance.currency.amount`**:
  chose verbatim reuse over invented regex variants because (a) prior art
  exists and (b) decimal_number already governs this shape across the
  taxonomy. → MADR candidate (single decision: "validator alternations
  may compose from sibling-type canonical patterns; comment annotation
  required for traceability").
- **Don't widen amount via header hints or value-based rules**: respects
  MADRs 0042 and 0048. The validator widening is the right level for this
  fix; the model is the precision gate.
- **Misclassifications defer to a follow-up card**: respects card 0014's
  goal ("misclassification and code-vs-canonical defer to follow-up cards").

### Implementation Notes

- **Implementation order:**
  1. Update `labels/definitions_finance.yaml:131` — append 4th alternation;
     add comment block referencing `definitions_representation.yaml:79`.
  2. Add `vci4_*` regression tests in `crates/finetype-eval/src/bin/validate_corpus.rs`
     `mod attribute_tests` (or new `mod amount_widening_tests`).
  3. Run `cargo run -p finetype-cli -- validate eval/datasets/csv/ecommerce_orders.csv
     schema.json --db /tmp/iter4-demo.db --table orders` to capture
     before/after reject counts.
  4. Run `make validate-corpus` against iter-3's 12-dataset manifest,
     verify `vci3_fixture_attribution_regression_match` passes (no row-level
     drift). If any FIFA Value/Wage rows silently pass validation
     (would have been `code_vs_canonical / path-b-codetype`), per MADR 0076
     this is silent-preservation behavior — fixture row stays as
     forward-looking anchor; test is unaffected.
  5. Update `eval/eval_output/validate_corpus.md` (regenerated by `make
     validate-corpus`).
  6. File follow-up card for status/order_id misclassifications. Card
     captures: predicted label, GT label, sample values, why the model
     misclassifies (`status` header + finite enum values mapped to
     periodicity; ORD- prefix mapped to SEDOL prefix-shape), and the fix
     path (training data — generator widening for `representation.discrete.status`,
     SEDOL generator tightening on the prefix shape).
  7. CHANGELOG `[Unreleased]` entry; CLAUDE.md "Recent work" / "What's next"
     refresh.
  8. `make ci` gate.

- **Test prefix:** `vci4_` — matches the iter-1/2/3 convention.
- **Risk on the regex change:** the iter-3 fixture lock catches any
  silent attribution drift on the 12-dataset corpus. If FIFA Value/Wage
  (currently `code_vs_canonical / path-b-codetype`) silently start passing
  validation under widened amount, the fixture rows are forward-looking
  anchors per MADR 0076 — no test failure, no required action.
- **Single MADR likely sufficient:** 0078 (or whatever the next index is) —
  "Validator alternations may compose canonical sibling-type patterns".
  Records the precedent that future widening can borrow from sibling
  validator patterns when the shape is canonical to that sibling type.

### Open Questions

None — all intent-level questions resolved.
