# Spec Review

**Date:** 2026-04-29
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-iter3/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 (content signal: eval corpus + ground-truth fixture) |
| 2 — Assumption & failure | content signal + cascade-rule complexity + new fixture artefact | 5 |
| 3 — Adversarial | structural concerns: label-only attribution may under-fire on iter-2 datasets, fixture-population workflow is recursively self-referential | 2 |

---

## Findings

### [HIGH] code_vs_canonical path B is label-only but iter-2 evidence shows the failing columns route via predicted == expected (path A territory)

**Category:** failure-mode
**Pass:** 2
**Description:** Path B for `code_vs_canonical` (ac-01 rule 3) fires only when
`predicted != expected` AND one side is in the code-typed allowlist. The iter-2
expected-vs-actual table (curation spec lines 435-439) names FIFA's `nationality`
and OECD's `LOCATION` as the target columns. Looking at the current iter-2
report (`eval/eval_output/validate_corpus.md` lines 12-17), `code_vs_canonical`
already shows 0 — meaning these columns currently attribute to misclassification,
which means **`predicted != expected` is already happening for them**. That's
consistent with path B. But: the spec gives no evidence that the model's
*current* predicted labels for these columns put either side in the proposed
allowlist. If FIFA `nationality` predicts as `representation.text.word` and GT
is `geography.location.country`, neither is in the allowlist as drafted (allowlist
contains `country_code`, not `country`). Path B then doesn't fire, and ac-10
sub-criterion 3 ("FIFA, OECD, S&P 500 show `code_vs_canonical ≥ 1`") fails.

The constraint explicitly forbids escalation to value-shape signals in iter-3
("If the rule under-fires on iter-2's curated datasets at first run, escalate
... NOT in this iteration"). So the spec ships a rule and a fixture that *may*
record the actual outcome (rule under-fires), but ac-10 sub-criterion 3 is
written as a hard requirement. These two cannot both be true.
**Evidence:**
- spec.yaml ac-01 rule 3 (lines 103-104): code_vs_canonical path B requires
  `predicted != expected` AND `one side is in the code-typed taxonomy allowlist`.
- spec.yaml ac-02 (lines 137-148): allowlist enumerated, includes `country_code`
  but not `country`; includes `gics_*` with hedge that they may not exist.
- spec.yaml ac-10 sub-criterion 3 (lines 354-356): "FIFA, OECD, S&P 500 show
  `code_vs_canonical ≥ 1`" — hard assertion.
- spec.yaml constraint 4 (lines 60-62): "If the rule under-fires on iter-2's
  curated datasets at first run, escalate to value-shape signals as a follow-up
  spec, NOT in this iteration."
- eval/eval_output/validate_corpus.md lines 12-17: current `code_vs_canonical`
  count = 0 (iter-2 baseline, all 5 curated datasets attribute to
  misclassification including FIFA / OECD / S&P 500).
- labels grep confirms `gics_*` and `country_subdivision_code` do not exist in
  the taxonomy; `country_code` does (`labels/definitions_geography.yaml:59`).
**Recommendation:** Resolve one of three ways before implementation:
(1) Soften ac-10 sub-criterion 3 to a *fixture-driven* assertion: "for every
row in the fixture where `expected_mechanism: code_vs_canonical` and
`dataset in {fifa_players, oecd_employment, sp500_constituents}`, the actual
attribution matches" — implementation is then free to expand the allowlist or
take other label-only steps to make the fixture pass; or
(2) Pre-empt the path-B-under-fires risk by extending the allowlist criterion in
ac-02 to include the *canonical* sides (e.g. `geography.location.country`,
`identity.person.full_name` if FIFA `nationality` GT is one of those) — make
explicit which canonical labels are paired with which code labels; or
(3) Acknowledge in constraint 4 that *expanding the allowlist within iter-3 is
permitted* (it's still label-only) and only *adding new value-shape plumbing*
is deferred — distinguish between "tuning the allowlist" and "adding new
artefacts."

The spec as-written sets up implementer for a forced choice between violating
constraint 4 (escalate to value-shape) and failing ac-10.

### [HIGH] ac-05 verification is circular — fixture is populated *from* the harness output it then validates

**Category:** test-gap
**Pass:** 3
**Description:** Implementation note 4 (lines 460-466) describes the fixture
population workflow: phase 2 is "run iter-3 harness against the full 12-dataset
manifest, capture per-dataset failing columns, add a row per column with
rationale = first sentence of the rule's rustdoc." Then ac-05's
`vci3_fixture_attribution_match` test loads the fixture and asserts attribute()
returns the expected mechanism for every row. **The harness is the oracle; the
test asserts the oracle agrees with itself.** A buggy attribute function that
mis-attributes 30 columns would generate a fixture that records those wrong
mechanisms as "expected," and the fixture-iteration test would still pass.

This isn't theoretical — implementation note 4 phase 3 says "hand-review each
row for accuracy" but doesn't pin a constraint that hand-review HAPPENED. There's
no signal in the PR that distinguishes a hand-reviewed fixture from a
machine-generated one.

The iter-2 expected-vs-actual table (rows for nyc_taxi, gdelt_events, fifa_players,
oecd_employment, sp500_constituents) is the only **independent** ground-truth.
Five rows out of (let's estimate) 70+ failing columns. The other 65+ rows are
"trust the implementer's hand-review."
**Evidence:**
- spec.yaml ac-05 (lines 230-248): test loads fixture, asserts attribute()
  returns expected_mechanism for every row.
- spec.yaml implementation_notes phase 2 (lines 461-463): fixture rows added
  from harness output.
- spec.yaml implementation_notes phase 3 (line 464): "hand-review each row
  for accuracy" — no enforcement.
- eval/eval_output/validate_corpus.md: misclassification = 72 columns. Iter-2
  authoritative table covers 5; the other 67+ rows go through the
  hand-review-on-trust path.
**Recommendation:** Add at least one of:
(a) **Independent oracle for at least the 5 iter-2 curated rows**: an explicit
constraint that the 5 iter-2 expected-vs-actual rows (nyc_taxi/tpep_pickup_datetime,
gdelt_events/SQLDATE, fifa_players/nationality, oecd_employment/LOCATION,
sp500_constituents/GICS Sector) MUST be in the fixture with the EXACT
`expected_mechanism` values from the curation spec table — not whatever the
harness happened to produce. Add a verification step that greps the fixture for
those 5 (dataset, column) pairs and asserts the expected_mechanism matches the
curation spec. This makes at least 5 rows independent ground-truth.
(b) **Two-pass fixture construction in implementation_notes**: explicit phase
ordering — first commit the 5 iter-2 rows from the curation spec table by
hand; then run the harness against the rest; then human-review the rest with
the rationale field documenting each judgement. PR review can then check
rationale quality on the non-iter-2 rows.
(c) **Acknowledge scope**: rename ac-05 to "fixture-iteration regression test"
explicitly framing it as a *regression* test (catches future drift) rather
than a *correctness* test (which would require independent oracle). This
weakens the spec's claim but makes it honest.

### [MEDIUM] Cascade rule 4 (path A format_diversity) requires `not in 5-seam table` per ac-01 line 106 — but the iter-2 source code has the rules in opposite order

**Category:** assumption
**Pass:** 2
**Description:** ac-01 lists rule 4 (path A format_diversity, iter-2 preserved)
as: `predicted == expected` AND `SEMANTIC_TYPE pattern-reject` AND `not in
the 5-seam table`. Looking at validate_corpus.rs:325-335, the iter-2 cascade
runs **code_vs_canonical first** (with seam-table check) then **format_diversity
falls through** when seam-table doesn't match — meaning iter-2's
format_diversity only fires when seam-table is false. So the rule 4 condition
"not in 5-seam table" is currently *implicit* (cascade ordering), not *explicit*
in the rule body.

If iter-3's refactored rule 4 keeps the implicit ordering (rule 5 runs before
rule 4 because they're listed 5-then-4 OR rule 4 explicitly checks
`!column_in_seam_table()`), behaviour is preserved. The spec's rule numbering
(format_diversity path A is rule 4, code_vs_canonical path A is rule 5)
**reverses** iter-2's cascade ordering of these two rules. Iter-2: code_vs_canonical
seam-check runs **before** format_diversity. Iter-3: format_diversity path A
fires first (rule 4 < rule 5).

If rule 4 has an explicit `!column_in_seam_table(column_name)` guard, the
behaviour is equivalent. If rule 4 lacks that guard and relies on cascade
ordering (rule 5 first), then iter-3 inverts iter-2 behaviour: a seam-table
column with `predicted == expected` + pattern-reject would attribute to
**format_diversity** in iter-3 instead of `code_vs_canonical`.

ac-01 verification only checks "rule functions exist" and "rustdoc present" —
it doesn't catch this regression.
**Evidence:**
- validate_corpus.rs:325-335: iter-2 `code_vs_canonical` (rule 3 in iter-2
  numbering, with seam check) runs before iter-2 `format_diversity` (rule 4 in
  iter-2 numbering).
- spec.yaml ac-01 (lines 105-108): iter-3 reverses this — `format_diversity
  path A` is rule 4, `code_vs_canonical path A` is rule 5; format_diversity
  description says "AND not in the 5-seam table" but it's unclear whether
  this is intended as an explicit guard or an implicit "rule 5 hasn't fired."
**Recommendation:** Add to ac-01 (or as a new constraint): "Path A
format_diversity rule must include an explicit `!column_in_seam_table(column)`
guard in its body. The cascade ordering between rules 4 and 5 is preserved as
documented (4 before 5), but the seam-table guard is what disambiguates them
— not cascade order." Alternatively, if cascade-order disambiguation is the
intent, swap the rule numbering (path A code_vs_canonical = rule 4, path A
format_diversity = rule 5) to mirror iter-2's behaviour exactly. Either choice
is valid; the spec must pick one.

Add a regression test (under ac-04 or ac-06) that pins this: input
`column_name="Country"`, `predicted == expected ==
geography.location.country`, single SEMANTIC_TYPE pattern reject — must
attribute to `code_vs_canonical`, not `format_diversity`.

### [MEDIUM] `metadata.prior_review_ref` points to a nonexistent file

**Category:** assumption
**Pass:** 1
**Description:** spec.yaml line 547 declares `prior_review_ref:
orbit/specs/2026-04-28-validate-corpus-iter3/review-spec-2026-04-29.md`. That
file does not exist on disk; the prior review is named
`review-spec-2026-04-29-stub.md` (per the directory listing). This is
factually wrong. It also creates an ordering paradox if the convention is for
this very review (which is generating now) to take that path — the spec
references a future artefact as a past one. Drive.yaml notes "Pre-drive
review-spec on the stub spec ... preserved at review-spec-2026-04-29-stub.md
for audit trail" so the intent of the metadata field was to point at the stub
review. Mistake is the missing `-stub` suffix.
**Evidence:**
- spec.yaml line 547: `prior_review_ref: orbit/specs/.../review-spec-2026-04-29.md`
- ls of dir: `review-spec-2026-04-29-stub.md` is the file that exists.
- drive.yaml lines 22-25: confirms the stub review at the `-stub.md` path.
**Recommendation:** Update `prior_review_ref` to
`orbit/specs/2026-04-28-validate-corpus-iter3/review-spec-2026-04-29-stub.md`.

### [MEDIUM] ac-02 allowlist contains `gics_*` and `country_subdivision_code` which don't exist in the taxonomy — hedge is too soft for verification

**Category:** test-gap
**Pass:** 2
**Description:** ac-02 enumerates the initial code-typed allowlist with
entries like `geography.location.country_subdivision_code`,
`finance.classification.gics_sector`, `gics_industry_group`, `gics_industry`,
`gics_sub_industry`. Grep across `labels/definitions_*.yaml` confirms none of
these 5 labels exist. The spec hedges with "(if these exist; otherwise
document the closest available)" but that hedge is on `gics_*` only — not on
`country_subdivision_code`. The verification at ac-02
(`vci3_code_typed_allowlist_nonempty`) only asserts "non-empty" and "matches
`^[a-z_]+\\.[a-z_]+\\.[a-z_]+$`" — it does NOT assert that each entry is a
**real** taxonomy label.

A literal implementation following the spec ships an allowlist with 5 dead
labels — they pass the regex test but cannot ever match a real predicted /
expected label, so the rule under-fires by design on those mechanisms.
sp500_constituents `GICS Sector` is named in the iter-2 expected-vs-actual
table as the canonical example for `code_vs_canonical` → if `gics_sector`
isn't in the taxonomy, the model can never predict it, and path B can never
fire on the column GT side (where GT is the canonical column name's type).
Risk compounds with HIGH-1.
**Evidence:**
- spec.yaml ac-02 lines 137-148: enumerated allowlist entries.
- Grep across labels/definitions_*.yaml: `gics`, `country_subdivision` →
  zero matches.
- spec.yaml ac-02 verification (lines 152-156): "valid taxonomy label string
  (matches regex)" — checks shape, not existence.
**Recommendation:** Strengthen ac-02 verification to: "every entry in
`CODE_TYPED_LABELS` corresponds to a real taxonomy key in
`labels/definitions_*.yaml` (or equivalent canonical taxonomy source). Test
loads the taxonomy and asserts each allowlist entry is a known label." Then
explicitly remove non-existent entries from the spec's enumerated list, or
add a footnote naming the closest existing taxonomy label for each removed
one (e.g. `geography.location.gics_*` → swap for `finance.classification.<actual_key>`).

This also surfaces a question: if no existing taxonomy label captures GICS
Sector, the iter-3 work may surface a *taxonomy gap* rather than an
attribution-rule gap. That's important context for whether ac-10
sub-criterion 3 is achievable in iter-3.

### [MEDIUM] No constraint or AC pins fixture against silent regrowth

**Category:** missing-requirement
**Pass:** 2
**Description:** Constraint 5 says "Per-(dataset, column) fixture ... is
authoritative ground-truth. ... Silent re-attribution is impossible — drift
requires a fixture diff in the PR." The mechanism that enforces this is
`vci3_fixture_attribution_match` (ac-05). But there's no constraint that says
*the fixture itself cannot grow without human review*. An implementer mid-PR
can re-run the harness, find new failing columns (because some upstream
change shifted predictions), and **append** rows to the fixture — making the
test pass. The "fixture diff in the PR" part of the constraint becomes
trivially true (every PR will have a diff that adds rows) and the constraint
loses its bite.

This is a future-PR risk, not a launch-PR risk. But the spec ships the
contract, and the contract has a hole.
**Evidence:**
- spec.yaml constraint 5 (lines 64-71): "drift requires a fixture diff in the
  PR" — but doesn't say the diff must be reviewed for legitimacy or limited
  to expected mechanism shifts.
- Implementation_notes phase 2 (lines 461-463): describes population by harness
  re-run; no constraint on subsequent re-runs.
**Recommendation:** Add a CI check (or extend ac-05) that asserts: "if the
fixture file changes in a PR, the PR description must contain a `## Fixture
diff rationale` section documenting each added/removed/modified row." This
is a soft check (PR-text-grep, not test-failure) but it makes silent regrowth
visible. Alternative: tighten ac-05's verification to count fixture rows and
emit a warning in the test output if row count differs from a baseline (hardcoded
in the test as `EXPECTED_FIXTURE_ROW_COUNT`). Drift then requires updating
the constant, which forces a code review.

### [LOW] ac-09 trigger label `cascade-default` overlaps semantically with `unknown` mechanism

**Category:** assumption
**Pass:** 2
**Description:** ac-09 lists 4 trigger label values: `path-a-pattern`,
`path-b-prefix`, `path-b-codetype`, `cascade-default`. The "cascade-default"
trigger is used for "catch-all rules: enum_overfit, misclassification,
unknown" per the spec. But that lumps together three operationally-distinct
rules — enum_overfit (a specific rule with a specific condition),
misclassification (the broad catch-all), and unknown (the absolute fallback
when no rule fires). Analysts reading the per-column table can't distinguish
"this hit the misclassification catch-all" from "this hit the unknown
fallback at the bottom" from "this hit enum_overfit." Three trigger labels
collapse to one.
**Evidence:**
- spec.yaml ac-09 lines 326-330: trigger label enumeration.
**Recommendation:** Split `cascade-default` into three labels matching the
underlying rule: `enum-constraint`, `prediction-error` (for misclassification),
`fallthrough` (for unknown). Total trigger label vocabulary: 6, not 4. Or
keep 4 if the analyst-facing doc (ac-08) explicitly explains the lumping —
in which case add a clause to ac-08 calling that out.

### [LOW] ac-12 doesn't pin which CHANGELOG section format is required

**Category:** test-gap
**Pass:** 1
**Description:** ac-12 says "CHANGELOG.md [Unreleased] section gains entries"
but doesn't specify keep-a-changelog format vs free-form vs prefix
conventions. The verification only greps for "new [Unreleased] entries" via
`git diff main`. A one-word entry passes. This is low-impact (cosmetic) but
worth pinning if the project has a CHANGELOG style.
**Evidence:**
- spec.yaml ac-12 lines 401-407 + 412-416.
- No grep performed against existing CHANGELOG.md to check style.
**Recommendation:** Either soften: "an entry is added to [Unreleased] —
content shape per existing CHANGELOG style" — or strengthen with a concrete
example of the entry text expected.

---

## Honest Assessment

The full spec is a meaningful upgrade over the stub. The three HIGH findings
from the prior stub review have been addressed at intent level: cascade
ordering is explicit (constraint 3), the per-(dataset, column) fixture is the
anti-regression lock (constraint 5), test naming converges to `vci3_*`
(constraint 6), and the iter-2 rules are preserved as paths-A inside the
4-bucket coalesce (ac-01). Implementation_notes are detailed enough that an
implementer can ship without re-design.

The biggest remaining risk is **rule-effectiveness vs spec assertions**. The
spec assumes the proposed rules will fire correctly on iter-2's curated 5
datasets — ac-10 pins this as a hard outcome — but the rule shape (label-only,
narrow allowlist with non-existent labels in it) and the explicit ban on
escalating to value-shape signals (constraint 4) create a corner where the
implementer is forced to either fail ac-10 or violate constraint 4. HIGH-1
captures this tension. HIGH-2 names a related risk: the fixture is
populated by the harness it then validates, so a buggy implementation
doesn't get caught — only the 5 iter-2 rows are independent ground-truth, and
the spec doesn't pin them as such.

The label-only escalation gate is the right call for scope discipline, but
needs a clearer fork: either *the rule MUST be expressible label-only, or
ac-10 is a soft outcome (fixture-driven, not hard sub-counts)*. Implementer
needs to know which way the wind blows when path B under-fires.

Resolve HIGH-1 and HIGH-2, fix the prior_review_ref typo, tighten the
allowlist verification (MEDIUM about gics/country_subdivision), and the spec
is implementable. The MEDIUMs around cascade-order seam-table behaviour and
fixture regrowth are about resilience, not blocking.
