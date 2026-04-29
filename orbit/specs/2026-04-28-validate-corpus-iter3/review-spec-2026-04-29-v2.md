# Spec Review

**Date:** 2026-04-29
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-iter3/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 (content signal: eval corpus + ground-truth fixture; gate AC verification passes deterministic check) |
| 2 — Assumption & failure | content signal + cycle-1 HIGH-2 partial resolution + new ac-13 anchor-row pin | 4 |
| 3 — Adversarial | structural concern: anchor pin claims to be VERBATIM from iter-2 spec, but at least one anchor row picks a column not named in iter-2's CvC commitment | 1 |

---

## Findings

### [HIGH] FIFA `Nationality` anchor pin contradicts iter-2's actual CvC commitment

**Category:** assumption
**Pass:** 3
**Description:** ac-13 names `fifa_players, Nationality, code_vs_canonical`
as one of the 5 independent ground-truth anchor rows, claimed VERBATIM from
the iter-2 curation spec's expected-vs-actual table. But iter-2's GT sidecar
(`eval/datasets/validate_corpus/fifa_players.gt.yaml`) and its `notes:`
section name a different CvC commitment for FIFA — position ratings (LS, ST,
RS, LW … RB — 26 columns of "88+2" formatted ratings), currency
(Value/Wage/Release Clause with "€110.5M" / "€565K"), and imperial
measurements (Height "5'7", Weight "159lbs"). Nationality (mapped to
`geography.location.country`) is NOT named as a CvC slot in the iter-2 GT
notes. The current report (`eval/eval_output/validate_corpus.md`) confirms
this empirically: FIFA's failing columns are CAM/CB/CDM/CF/CM, Contract
Valid Until, Height, LS/RB/etc., Photo, Preferred Foot, Value, Wage —
Nationality is NOT in the failing-column list at all. Picking Nationality
as the anchor pins the cascade to a column the harness doesn't even flag
as failing in the iter-2 baseline.

The iter-2 spec line 437 (`fifa_players | code_vs_canonical |
code_vs_canonical | ✓ | nationality non-canonical names fire`) appears in
an *illustrative* mismatch-documentation example (lines 432-440) — not in
a delivered authoritative table. The actual delivered iter-2 thesis is in
the GT sidecar's `notes:`, and it doesn't mention Nationality. The
spec's "VERBATIM from iter-2 spec" claim is therefore literally true (the
example table contains that string) but materially misleading — the
illustrative example wasn't the curation outcome, and the GT sidecar is
silent on Nationality.

If the implementer follows ac-13 as written, they pin a fixture row for
`fifa_players, Nationality, code_vs_canonical` whose harness output may be
"no failure detected at all" (Nationality isn't currently in the failing
set). The fixture-iteration test then has nothing to assert against, OR
it asserts code_vs_canonical against a row that the harness never
processes as a failure (depending on how missing-failing-column rows are
modelled). HIGH-2 of the cycle-1 review wanted independent ground-truth;
this row is independent but for the wrong column.
**Evidence:**
- spec.yaml ac-13 lines 545-550: 5-row anchor list including
  `fifa_players, Nationality, code_vs_canonical`.
- `eval/datasets/validate_corpus/fifa_players.gt.yaml` line 7: Nationality
  mapped to `geography.location.country`.
- Same file lines 100-127 (notes): CvC commitment is position ratings,
  currency, imperial measurements — Nationality is mentioned only as a
  geography mapping, not as a CvC seam.
- `eval/eval_output/validate_corpus.md` lines 70-101: FIFA failing columns
  exhaustive list — Nationality absent.
- `orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml` lines 432-440:
  the `nationality non-canonical names fire` reference is in an illustrative
  mismatch-table example, not the delivered curation table.
**Recommendation:** Resolve before implementation. Three options:
1. **Replace the FIFA anchor row** with one that matches iter-2's actual CvC
   commitment — e.g. `fifa_players, Value, code_vs_canonical` (formatted
   currency CvC with €/M/K) or `fifa_players, LS, code_vs_canonical`
   (position rating CvC). Update ac-13 to cite the GT sidecar `notes:`
   section as the source of truth, NOT the illustrative table.
2. **Drop FIFA from the anchor set** and rely on the 4 remaining anchors
   (NYC Taxi, GDELT, OECD, S&P 500). Document in ac-13 that iter-2's FIFA
   thesis was multi-column with no single load-bearing column suitable for
   anchor pinning.
3. **Confirm with Hugh** whether iter-2's actual intent for FIFA was the
   illustrative-table interpretation (Nationality non-canonical) and
   re-curate the GT sidecar accordingly. This is the largest scope change
   and likely outside iter-3.

Option 1 is the cleanest and stays within the "iter-2 GT sidecars
byte-unchanged" constraint.

### [HIGH] ac-13 S&P 500 GICS anchor row is constructively pre-escalated, undermining the "independent ground-truth" framing

**Category:** failure-mode
**Pass:** 2
**Description:** ac-13 anchors `sp500_constituents, GICS Sector,
code_vs_canonical` as a hard pin, and ac-02 says `gics_*` doesn't exist in
the v0.6.19 taxonomy and the GT label is `representation.discrete.categorical`.
Path B fires when "one side is in the allowlist". The GT
`representation.discrete.categorical` is NOT in the allowlist (it's a
generic shape, not a code-typed label). For path B to fire on this row, the
*predicted* label would have to be a code-typed label from the allowlist —
i.e. the model would have to predict something like `geography.location.country_code`
or `finance.banking.swift_bic` for canonical English text like "Information
Technology" / "Health Care" / "Financials". There's no plausible scenario
where the multi-branch model predicts a code-typed label for English-text
sector names. The GICS Sector column has values like "Information
Technology" — the model will predict `representation.text.word` /
`representation.text.entity_name` / `representation.discrete.categorical`,
none of which are in the code-typed allowlist.

The spec acknowledges this in the GICS carve-out (ac-02 lines 197-207, ac-13
lines 555-564): "If allowlist tuning cannot make `sp500_constituents.GICS
Sector` attribute to code_vs_canonical with label-only signals, this row's
`expected_mechanism` stays `code_vs_canonical` AND a `pending_escalation:
true` field is added". The `#[ignore]` on the test means the row is a
fixture comment, not a test assertion. Combined with the no-plausible-path
analysis above, the GICS anchor is *pre-escalated by construction*. Rather
than being one of "5 iter-2 anchor rows pinned independently from harness
output", it's effectively 4 hard anchors + 1 hand-waved row that documents
a known gap.

Cycle-1 HIGH-2's stated goal was "5 rows of independent ground-truth out of
70+ failing columns". The actual independent ground-truth count under v1.1
is 4 hard anchors + 1 known-gap row. That's still better than 0, but the
spec's framing implies stronger evidence than is delivered.
**Evidence:**
- spec.yaml ac-02 lines 197-207: GICS carve-out acknowledging path-B can't
  fire on label-only signals for the GT label
  `representation.discrete.categorical`.
- spec.yaml ac-13 lines 555-564: "pending_escalation: true ... test marks
  it #[ignore]".
- `eval/datasets/validate_corpus/sp500_constituents.gt.yaml` line 4: `GICS
  Sector: representation.discrete.categorical`.
**Recommendation:** Pick one:
1. **Drop GICS Sector from the hard anchor set.** Add a 5th anchor from
   another iter-2 dataset that has a viable label-only path B (e.g. a GDELT
   secondary code column, or a different OECD CODE column not REF_AREA).
   The GICS row can still appear in the fixture as a `pending_escalation:
   true` row, but it's not framed as an "anchor".
2. **Reframe the anchor list as "4 + 1 known gap"** — acknowledge openly
   that GICS Sector is the known taxonomy-gap row, not an attribution-rule
   verification row. Update ac-13 framing accordingly.
3. **Defer GICS to an explicit follow-up taxonomy-widening card** and
   exclude `sp500_constituents` from iter-3's anchor set entirely. iter-3
   ships with 4 anchors instead of 5.

Option 2 is least disruptive to the spec structure; Option 1 strengthens the
anchor independent-ground-truth evidence; Option 3 is cleanest but loses
S&P 500 from the iter-3 thesis.

### [MEDIUM] Phase 2 fixture rows are still harness-derived; v1.1 doesn't fully resolve cycle-1 HIGH-2

**Category:** test-gap
**Pass:** 2
**Description:** Cycle-1 HIGH-2 said "the fixture is populated from harness
output, then validates harness output — the harness is the oracle agreeing
with itself." v1.1 addresses this by carving out 5 anchor rows (ac-13) as
Phase 1 (independent ground-truth). But Phase 2 — every OTHER fixture row,
~67+ rows out of ~72 failing columns — is still "the harness's actual
output is recorded as the expected_mechanism" with "hand-review each row
for plausibility before commit" as the only safeguard.

The cycle-1 finding's Recommendation (a) was "Independent oracle for at
least the 5 iter-2 curated rows" — v1.1 picks (a) but leaves the other
67+ rows on the hand-review-on-trust path. Recommendation (c) was
"Acknowledge scope: rename ac-05 to fixture-iteration regression test
explicitly" — v1.1 doesn't do this rename. ac-05 still reads as a
correctness assertion ("attribute() returns the fixture's
expected_mechanism for every row").

This isn't fatal — 5 independent anchors + 67 hand-reviewed rows is
substantially better than 0 + 72 — but the spec's framing implies a
correctness check that Phase 2 doesn't actually deliver. A buggy
attribute() that mis-attributes 30 Phase-2 columns will generate a fixture
recording those wrong mechanisms as "expected", and the
fixture-iteration test will pass.

The `vci3_fixture_row_count_baseline` test added in v1.1 (constraint 5)
guards against silent fixture *regrowth* but not against silent fixture
*incorrectness* at Phase-2 commit time.
**Evidence:**
- spec.yaml ac-03 lines 251-256: Phase 2 — "harness's actual output" is
  expected_mechanism; "Hand-review each row before commit (catch any
  obvious miscategorisations)".
- spec.yaml ac-05 lines 326-334: test asserts attribute() returns the
  fixture's expected_mechanism for every row.
- spec.yaml ac-13 lines 549-553: the 5 anchors are framed as the SOLE
  independent ground-truth.
**Recommendation:** Strengthen at least one of:
1. **Reframe ac-05** as `vci3_fixture_attribution_regression_match` and
   document in its rustdoc that it pins behaviour against future drift,
   not against initial correctness. The 5 anchor-row test (`vci3_fixture_iter2_anchor_rows_present`)
   is the correctness check.
2. **Require rationale-quality review in the PR description**: add a
   constraint that the PR description must include a "Phase-2 hand-review
   summary" section calling out any Phase-2 rows where rationale-quality
   was uncertain. review-pr stage greps for this section. Soft check, but
   makes the hand-review step visible.
3. **Add a sample-based independent-review obligation**: require the
   implementer to spot-check N≥5 random Phase-2 rows by hand, comparing
   the harness's chosen mechanism against the iter-2 GT sidecar's
   `notes:` section for that dataset. Document the spot-check in the PR.

The framing fix (option 1) is cheapest; option 3 is the strongest
evidence upgrade.

### [MEDIUM] ac-10 sub-criterion 1 (`format_diversity ≥1 AND code_vs_canonical ≥1`) remains a hard count and may conflict with v1.1's GICS carve-out path

**Category:** failure-mode
**Pass:** 2
**Description:** v1.1 softened ac-10 sub-criterion 3 to fixture-driven, but
sub-criterion 1 stayed: "Per-mechanism breakdown table with
`format_diversity` count ≥ 1 AND `code_vs_canonical` count ≥ 1 (lifts
iter-2's AC-08 downgrade)." If GICS Sector is the only S&P 500 path-B
candidate and it's pre-escalated (per HIGH-2 above), `code_vs_canonical`
≥1 must come from FIFA, OECD, or some non-anchor Phase-2 column. FIFA's
"Nationality" anchor is dubious (per HIGH-1). OECD's REF_AREA has GT
`geography.location.country_code` (in the allowlist), so path B can fire
if the model predicts something else (e.g. `representation.text.word`)
— this is the most plausible code_vs_canonical hit. So the count is
likely ≥1 via REF_AREA alone, but the spec doesn't say "REF_AREA is
the load-bearing path-B column"; it asserts the count without
identifying the carrier.

Combined with the constraint that allowlist tuning is in scope (constraint
4), the implementer can engineer toward `code_vs_canonical ≥1` by
expanding the allowlist until at least one Phase-2 column attributes that
way — which makes sub-criterion 1 trivially satisfiable but uninformative.
The "lifts iter-2's AC-08 downgrade" framing also lacks a quality bar:
`code_vs_canonical = 1` from a single OECD column is mechanically
different from `code_vs_canonical = 9` from all 9 OECD CODE/LABEL pairs.
**Evidence:**
- spec.yaml ac-10 sub-criterion 1 (lines 449-452): hard ≥1 counts.
- spec.yaml constraint 4 (lines 60-67): allowlist tuning IS in scope.
- spec.yaml ac-13 GICS carve-out: GICS path-B fires only if the implementer
  can engineer label-only attribution, otherwise pending_escalation.
- `eval/datasets/validate_corpus/oecd_employment.gt.yaml` lines 8-22: 9
  CODE/LABEL pairs (REF_AREA / MEASURE / UNIT_MEASURE / TRANSFORMATION /
  ADJUSTMENT / SEX / AGE / ACTIVITY / FREQ).
**Recommendation:** Add to ac-10 sub-criterion 1: "the `code_vs_canonical`
≥1 count must include at least one row whose `(dataset, column)` pair is
in the iter-3 fixture with `expected_mechanism: code_vs_canonical` AND
`pending_escalation: false`". This rules out the trivial-engineer-toward-1
path and ties sub-criterion 1 to the fixture authority. Optionally,
strengthen further to "≥3 unique code_vs_canonical rows across ≥2
datasets" so the test is meaningful even after fixture-driven softening.

### [MEDIUM] ac-04 negative test list duplicates the cycle-1 cascade-rule-4 regression test under a misleading name

**Category:** test-gap
**Pass:** 2
**Description:** ac-04 includes
`vci3_attribute_format_diversity_path_a_seam_table_guard` (lines 312-316):
"`predicted == expected`, SEMANTIC_TYPE pattern-reject, column name IS in
the 5-seam table. Asserts code_vs_canonical (path A), NOT format_diversity
(path A)". This asserts cascade ordering — path A code_vs_canonical (rule
5) fires before path A format_diversity (rule 4) — but is filed under
"format_diversity negative tests". Naming-wise it would slot more naturally
under code_vs_canonical positive tests OR under cascade-order tests
(ac-06). The current placement implies it's about format_diversity
boundary, but it's really about cascade ordering between rule 4 and rule 5.

ac-06 already has a similar case (input matches both rule 1 enum_overfit
and rule 4 format_diversity path A → asserts enum_overfit). Adding the
seam-table-guard test to ac-06 would be more coherent.

This is a low-impact organisational issue — the test will exist either
way. But it suggests the cascade-ordering coverage isn't fully consolidated
in ac-06, which weakens cascade-order test discoverability.
**Evidence:**
- spec.yaml ac-04 lines 312-316: seam-table-guard test under
  format_diversity negatives.
- spec.yaml ac-06 lines 348-358: cascade-order test cases — does NOT
  include the rule-5-before-rule-4 case.
**Recommendation:** Move
`vci3_attribute_format_diversity_path_a_seam_table_guard` from ac-04 into
ac-06's case list as: "Input matches both Rule 5 (code_vs_canonical path
A) and Rule 4 (format_diversity path A) → asserts code_vs_canonical (rule
5 fires first by cascade order)". Renumber the test or rename to
`vci3_attribute_cascade_order_rule5_before_rule4` for clarity. ac-04
"format_diversity negatives" stays focused on format_diversity boundaries.

---

## Honest Assessment

v1.1 makes substantive progress — all 8 cycle-1 findings are addressed at
intent level. The 36-entry allowlist is verified taxonomy-real (I checked
all 36 against `labels/definitions_*.yaml`); the prior_review_ref typo is
fixed; the cascade rule 4/5 guard is explicit; the trigger label split
into 6 distinct values; the `vci3_fixture_row_count_baseline` test pins
silent regrowth; ac-13 introduces the strict two-pass commit phasing
enforced by git-log assertion. Gate AC ac-14 verification passes the
deterministic Pass-1 check (clear multi-line evidence, no placeholder
tokens, well over 20 chars).

The biggest remaining risk is **anchor row authenticity**. ac-13's "5
iter-2 anchor rows VERBATIM from the iter-2 spec" claim is the load-bearing
piece of evidence for cascade correctness, but two of the five rows have
problems on inspection:

- **FIFA Nationality** is sourced from an *illustrative* example table in
  iter-2's spec, not from iter-2's delivered GT sidecar `notes:` (which
  names position ratings, currency, imperial measurements as the actual
  CvC commitments — Nationality is not in that list, and Nationality
  isn't even in the current report's failing-column list).
- **S&P 500 GICS Sector** is constructively pre-escalated — the v0.6.19
  taxonomy doesn't have a GICS label, the GT is generic
  `representation.discrete.categorical`, and there's no plausible
  predicted label that would put either side in the allowlist. The
  carve-out is correct; the framing as "anchor" is misleading.

The remaining anchors (NYC Taxi tpep_pickup_datetime, GDELT SQLDATE, OECD
REF_AREA) are credible — each has a clear path-B trigger (same broad-type
prefix subtype drift for the datetimes; allowlist hit on REF_AREA's GT).
That's 3 solid anchors, 1 dubious, 1 pre-escalated.

The Phase-2 hand-review safeguard remains thin (MEDIUM-3) — 67+ fixture
rows depend on the implementer's hand-review without enforcement, and
ac-05 is framed as a correctness check when it's structurally a regression
check.

Ship after fixing HIGH-1 (FIFA anchor) and reframing HIGH-2 (GICS as known
gap, not anchor). The MEDIUMs around Phase-2 framing, ac-10 sub-criterion
1 quality bar, and seam-table test placement are resilience polish — not
blocking but worth pinning before drive resumes implementation.
