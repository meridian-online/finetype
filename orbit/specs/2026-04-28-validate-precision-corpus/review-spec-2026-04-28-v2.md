# Review: 2026-04-28-validate-precision-corpus (cycle 2 — v1.1)

**Reviewer:** Nightingale
**Date:** 2026-04-28
**Spec:** `orbit/specs/2026-04-28-validate-precision-corpus/spec.yaml` (v1.1)
**Card:** `orbit/cards/0014-profile-validate-precision.yaml`
**Interview:** `orbit/specs/2026-04-28-validate-precision-corpus/interview.md`
**Prior review:** `orbit/specs/2026-04-28-validate-precision-corpus/review-spec-2026-04-28.md` (REQUEST_CHANGES, 5 blockers + 10 concerns)
**ambiguity_score (claimed):** 0.08
**Verdict:** APPROVE

---

## Summary

Cycle 1 surfaced five factual blockers (taxonomy labels, `--enum-threshold`
collision, sources.yaml schema, licence enum, ontology no_gt) plus ten
concerns. The v1.1 revision history names each one and the spec body
discharges them cleanly. I re-checked the live codebase and every blocker
is resolved against the truth on disk. The remaining concerns I have are
soft (residual ambiguity in measurement language, one cross-cutting
caveat in the attribution rules), not structural.

The iteration is correctly bounded: harness mechanics + 7-CSV iter-1
corpus + two surgical fixes (enum-overfit default 50→32 and ≤5 validator
widenings) + three MADRs + a stub follow-up card. Movement-based success
(ac-14) is the right shape — the structural deliverable is a measurement
capability and a baseline, not a pre-committed numerical movement. The
mechanism ontology maps cleanly to MADR 0071's reject fields and is
auditable via the six unit tests in ac-07.

I'm approving with five non-blocking notes. None require a v1.2.

---

## Verification of cycle-1 blockers (each independently confirmed against the codebase)

### B1 → resolved. Taxonomy labels now match live YAML.

The live `labels/definitions_representation.yaml` carries
`representation.boolean.binary` (line 973), `representation.boolean.initials`
(line 1001), and `representation.boolean.terms` (line 1035), alongside
`representation.discrete.categorical`. ac-09 sub-fix (b) targets the
boolean.* family for enum-emission gate parity — these are real labels.
The unit-test names (`pvc_enum_kept_for_boolean_terms_label` etc.) reference
labels that exist. The dead `representation.enumeration.*` references from
v1.0 are gone.

### B2 → resolved. Default-lowering replaces hidden 32.

ac-09 v1.1 reframes the fix as lowering the existing
`--enum-threshold` default from 50 → 32 (one literal-value change at
`crates/finetype-cli/src/main.rs:299`, currently
`#[arg(long, default_value = "50")]`). Users who pass `--enum-threshold N`
explicitly are unaffected. There is now exactly one knob in the
user-facing surface, not two. Cross-format consistency is preserved
because `enum_threshold` is already plumbed through every emitter
(profile resolves it once at `main.rs:573`, propagates to both plain
output and JSON Schema output via `collect_unique_values_if_categorical`
at `main.rs:3340`).

### B3 → resolved. ac-02 schema matches live sources.yaml.

The live `eval/datasets/sources.yaml` uses
`source_url, role, licence, fetched_date, attribution, datasets`
(role enum: `eval | train | both-forbidden`). ac-02 v1.1 names exactly
these six keys, declares `provenance_status` as a NEW column on the
`validate_manifest.csv` (file-keyed) rather than on sources.yaml, and
explicitly extends the role enum to add `validate`. The header-comment
update obligation is also named.

### B4 → resolved. Licence values align with `eval/licence_allowlist.txt`.

The live allowlist contains `MIT`, `PDDL-1.0`, `public-domain`,
`unknown-investigating`, plus other SPDX values not used by iter-1.
implementation_notes line 518 pins each iter-1 dataset to one of
`{MIT, PDDL-1.0, public-domain}` and explicitly states no allowlist edit
is needed. ac-01 correctly references the SPDX-canonical values.

### B5 → resolved. ontology_schema `no_gt` matches the 7-CSV shape.

The v1.1 description says: "Iteration 1 ships all 7 CSVs with full GT
(every column in every CSV has a sidecar entry), so `no_gt` is
structurally 0 this iteration. The mechanism exists for forward-
compatibility with the corpus-expansion follow-up (stub card 0015)…".
The leftover "10 ac-04 CSVs / remaining ~25" prose from v1.0 is gone.

### Cycle-1 concerns C1–C10 → addressed

- C1/C2 (monotonicity claims too strong): ac-09 verification now
  explicitly says "mechanism-monotonic on `enum_overfit` only — counts of
  OTHER mechanisms may rise as enum-suppressed columns fall through to
  broader pattern checks; this is expected and not a regression". ac-10
  carries the parallel disclaimer plus the "harness profiles each CSV
  exactly once per run" assumption made explicit.
- C3 (column-count discrepancy): "approximately 58 columns" with the
  ±2 tolerance for upstream schema drift is now stated.
- C4 (no AC for stub card 0015): ac-15 verifies its existence,
  maturity, scenario count, and back-reference.
- C5 (`make ci` and binary tests): ac-07 verification names
  `#[cfg(test)] mod attribute_tests` adjacent to `main` so
  `cargo test --workspace` exercises them. Good.
- C6 (binary path resolution): ac-06 verification now requires the
  `validate-corpus` Makefile target depend on a `cargo build --release`
  prerequisite "so a fresh checkout's `make validate-corpus` succeeds
  without a prior `cargo build`". Good.
- C7 (`--stdout` flag does not exist): dropped from ac-05 step 2.
- C8 (rule-2 attribution caveat): the ontology_schema `enum_overfit`
  description now carries a multi-paragraph caveat naming the post-fix
  conflation between "enum was over-fit" and "data has unenumerable
  variants in a legitimately-low-cardinality column", with the
  observation that "the rule's value is in surfacing the bucket; the
  human reads the per-column failure list to discriminate". Acceptable
  for iter-1.
- C9 (DictReader vs positional csv.reader): implementation_notes line
  527 explicitly names the constraint. Good.
- C10 (rio2016 licence pinning): line 518 now distinguishes the gist
  code MIT from the data licence `public-domain`, with the convention
  that the manifest column tracks the data licence. Also called out:
  "no allowlist edit is needed" because all four licence values
  (`MIT`, `PDDL-1.0`, `public-domain`, plus unused entries) already
  ship in `eval/licence_allowlist.txt`.

I checked the v1.1 source URLs live: all 7 return HTTP 200 today
(2026-04-28), so the manifest-time URLs the implementer commits will
match the spec's pinned values without surprise.

---

## New concerns (non-blocking)

### N1. ac-03 dry-run flag may be a net-new addition, not "if it does not exist"

ac-03 verification language: "the dry-run flag is added in this iteration
if it does not exist". I checked `scripts/compute_row_hashes.py` — the
existing argparse surface has `--manifest`, `--output`, `--max-rows`. There
is no `--dry-run`. So the flag is unambiguously NEW for this iteration,
not contingent. Suggest tightening the language to "the dry-run flag is
added in this iteration; the verification asserts it now exists and reports
union counts". Trivial wording fix; not blocking.

### N2. ac-03 verification leans on a specific test fixture from m-19's eval corpus

The verification asserts `airports.csv` row hashes appear in the
regenerated `eval/row_hashes.tsv`. If a future cleanup of the m-19
manifest moves or renames `airports.csv`, this AC's test breaks
incidentally. Suggest "any row from any current eval manifest entry"
parametrised, or just accept the brittle-but-cheap fixture and trust
follow-up rally hygiene to update it. Not blocking — the test author
will see it fail loudly.

### N3. ac-04 column-count delta tolerance (±2) is asymmetric vs implementation_notes

ac-04 description allows ±2 column drift "if upstream sources change
schemas between spec landing and CSV fetch". implementation_notes line
530 pins the per-dataset breakdown summing to exactly 58. If the actual
fetch lands at 60 columns total (allowed by ac-04), the implementation
notes go stale silently. Either drop the per-dataset numbers from notes
once committed, or re-pin notes from manifest after fetch. Cosmetic.

### N4. ac-09 "post-fix harness re-run … ≤ baseline" wording still has a soft edge

The spec correctly says monotonicity holds on `enum_overfit` only and
names the carve-out for "if baseline shows 0 `enum_overfit` attributions,
the three unit tests alone satisfy the fix's behavioural contract; the
harness baseline-vs-post-fix delta is recorded honestly with prose
noting why". This handles the corner case but the language sequencing
("the corpus-level count … is ≤ baseline's count for that mechanism. The
fix is mechanism-monotonic on `enum_overfit` only — …") could be misread
as implying the harness MUST show movement. The reader should infer that
the unit tests are the structural test and the harness movement is the
secondary signal. Already acceptable; spelling out the precedence one
more time in the verification block would make it bulletproof.

### N5. ac-10 widening priority list is non-prescriptive but the priority anchors might bias

implementation_notes line 523 names `un_locode.Date`, `un_locode.Coordinates`,
`rio2016_athletes.height`, and `pokemon.Type 1/Type 2` as "likely
candidates" while ac-10 description says "implementer chooses based on
baseline measurement, NOT the list above". The list is helpful for
priors but a reviewer-implementer might short-circuit the baseline-first
discipline if the named candidates happen to dominate the baseline. The
guard ("implementer measures, not pre-judges") is in the constraints
block at line 26 and the description, so I'm satisfied — but this is a
known governance pattern where named priors become anchors. Worth
naming in the PR description that the implementer ran the baseline
before choosing widenings.

---

## Strengths (re-affirmed)

- **Cycle-1 review fully discharged.** The revision_history block names
  every concern by tag and the spec body matches. This is the right
  way to consume an APPROVE-after-fix review.
- **Live-codebase alignment.** Every label, file path, line number,
  flag default, and licence value I spot-checked matches the truth on
  disk. The risk of an implementer getting stuck on factual mismatches
  is low.
- **Mechanism partition is honest.** ac-09's "may move other mechanism
  counts upward" disclaimer and ontology_schema's `enum_overfit`
  caveat both name the awkward truth — buckets are diagnostics, not
  diagnoses.
- **Bounded fix surface.** ≤5 widenings in ac-10 + literal 32-cap in
  ac-09 + "no retraining, no model changes" in constraint line 29 +
  "no new public CLI surface" in constraint line 23. The iteration
  cannot quietly grow.
- **Movement-based success (ac-14)** is correctly framed: "movement
  may be 0 if the iteration's two fixes happen to not flip any dataset
  across P=99%, which is acceptable given the bound nature of the
  iteration; the spec success criterion is the harness shipping with
  measurement capability, not a numerical movement floor". This
  prevents a perverse incentive to chase a +1 number.
- **Stub card 0015** with maturity `emerging` and back-reference to
  this spec is the right hand-off shape for the curation sprint.
- **Three MADRs with deterministic titles.** ac-11 names each file
  path, status, date, and minimum-Considered-Options count. The
  MADRs map to interview Q1 (round-trip), Q8 (infra reuse), Q6 (fix
  partition).

---

## Verdict reasoning

All five cycle-1 blockers and all ten cycle-1 concerns are addressed
in v1.1. I verified the discharge against the live codebase
(taxonomy labels at lines 973/1001/1035; `--enum-threshold` default
at main.rs:299; sources.yaml schema and role enum at the file head;
licence_allowlist.txt SPDX values; URL liveness via curl). My five
new concerns are wording / governance polish, not structural — none
warrants another revision cycle.

The spec is ready for implementation. The implementer should:

1. Run the baseline harness FIRST (before any of ac-09/ac-10 work),
   commit `validate_corpus.baseline.md`, and read the per-mechanism
   counts to choose ac-10's 1–5 widenings from measurement.
2. Land the 32-default change (ac-09 sub-fix a) and the boolean.*
   gate parity (ac-09 sub-fix b) as separate commits within the PR
   for clean diffs.
3. Be honest in the PR description about whether the iteration's
   fixes flipped any dataset across P=99%, per ac-14 and
   implementation_notes line 525.

**Verdict:** APPROVE
