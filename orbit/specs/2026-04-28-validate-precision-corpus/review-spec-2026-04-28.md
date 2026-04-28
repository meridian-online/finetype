# Review: 2026-04-28-validate-precision-corpus

**Reviewer:** Nightingale
**Date:** 2026-04-28
**Spec:** `orbit/specs/2026-04-28-validate-precision-corpus/spec.yaml`
**Card:** `orbit/cards/0014-profile-validate-precision.yaml`
**Interview:** `orbit/specs/2026-04-28-validate-precision-corpus/interview.md`
**ambiguity_score (claimed):** 0.08
**Verdict:** REQUEST_CHANGES

---

## Summary

The spec captures a meaningful, well-scoped iteration of card 0014: ship the
harness mechanics + a 7-CSV iter-1 corpus + two bounded fixes, and defer
sourcing-to-30/50 to a follow-up card. The mechanism ontology is honest about
what gets fixed vs surfaced. Movement-based success (ac-14) is appropriate
for a baseline-establishing iteration. The spec is internally well-organised
and ACs are mostly verifiable.

That said, I found **five concrete blockers** that will cause the implementer
to either get stuck or ship something contradictory to the rest of the
codebase, plus a handful of softer concerns. The fixes are surgical — none
require re-shaping the iteration. After the items in §Blockers land,
this is a strong APPROVE.

---

## Blockers

### B1. The `representation.enumeration.*` family does not exist in the taxonomy

**Where:** ac-09 description (lines 269, 282, 283), ac-09 verification
(line 296), constraint at lines 25.

The spec's enum-overfit fix predicates on
`representation.enumeration.*` family membership and uses
`representation.enumeration.boolean_terms` and
`representation.enumeration.categorical` as test fixtures. Neither exists.
The actual taxonomy has:

- `representation.discrete.categorical`
- `representation.discrete.ordinal`

(See `labels/definitions_representation.yaml`. `boolean_terms` is not in the
taxonomy at all — `git grep -n boolean_terms labels/` returns zero hits.)

The current emission code already gates on `representation.discrete.categorical`
(see `crates/finetype-cli/src/main.rs:3345` —
`if label != "representation.discrete.categorical" || enum_threshold == 0`).
**This means ac-09's first half (label-family gating) is already shipped.**
The actual NEW work in ac-09 is the cardinality cap.

**Fix:** Rewrite ac-09 to:
1. Reference the real label family (`representation.discrete.categorical`,
   plus `.ordinal` if intended);
2. Acknowledge the existing gating is already in place and name the
   delta as the cardinality cap and any boolean-detection narrowing
   actually needed;
3. Replace `boolean_terms` and `enumeration.categorical` test fixtures
   with real labels.

### B2. `--enum-threshold` (default 50) collides with the spec's hard 32-cap

**Where:** Constraint at line 25 ("32-cardinality cap is the literal
threshold — implementer does not tune this without a constraint
amendment"), ac-09 third unit test (line 282-285), implementation_notes
line 429 ("32-cardinality cap is a literal value — implementer should NOT
make it a flag in iteration 1").

`finetype profile` already exposes `--enum-threshold N` (default 50)
documented at `crates/finetype-cli/src/main.rs:297-299`. The flag controls
both the categorical-rendering surface AND (per the in-flight
`2026-04-28-profile-json-schema-output/spec.yaml`) the JSON Schema `enum`
keyword under `--stats`.

The spec proposes hard-coding 32 inside the JSON Schema emitter while the
flag continues to advertise 50. This produces a confusing UX: a user
who passes `--enum-threshold 50` gets enum arrays in some output formats
but not in `-o json-schema`.

**Fix:** Pick one of:
- (a) Use `--enum-threshold` (don't introduce a hidden 32). The fix
  becomes "default behaviour", and the unit-test fixture moves to assert
  threshold-driven omission.
- (b) Introduce a separate `--json-schema-enum-cardinality-cap` flag with
  default 32 — explicit second knob, document it in
  `2026-04-28-profile-json-schema-output/` as well as here.
- (c) Lower `--enum-threshold` default to 32 globally (probably wrong:
  cross-format regression risk).

(a) is the smallest delta. The spec must pick one and own the reasoning;
"32 is literal" without a flag is not viable when the user-facing flag
defaults to 50.

### B3. `sources.yaml` schema described in ac-02 does not match the live schema

**Where:** ac-02 description (lines 67-73) and implementation_notes line
433.

The spec describes `id, name, url, licence, role, fetched_date,
provenance_status, datasets`. The current `eval/datasets/sources.yaml`
header documents and uses `source_url, role, licence, fetched_date,
attribution, datasets` with `role ∈ {eval, train, both-forbidden}`.
Crucially: **there is no `provenance_status` field in the live
`sources.yaml`**, and `id`/`name`/`url` are not the field names in use
(they are `source_url`/`attribution`).

This may be a vestige of how m-19 was originally drafted vs how it
shipped. Either way, ac-02 will fail verification on a clean checkout
because the prescribed fields don't exist.

**Fix:** Rewrite ac-02's per-source field list to match the actual
shipped `sources.yaml` schema, OR explicitly add a "schema migration"
sub-AC that names every existing row that gains the new field. The
former is far cheaper.

### B4. ac-01 licence enum conflicts with `eval/licence_allowlist.txt`

**Where:** ac-01 (lines 44-46).

The spec's iter-1 allowlist is
`{public-domain, mit, cc0, cc-by-4.0, odc-pddl, odc-by-1.0}` (lowercase,
non-canonical). The shipped allowlist at
`eval/licence_allowlist.txt` is SPDX-canonical: `MIT`, `CC0-1.0`,
`CC-BY-4.0`, `PDDL-1.0`, `ODC-By-1.0`, plus `internal` /
`public-domain` / `unknown-investigating` as free-form. `odc-pddl` is
not a member; `mit` (lowercase) is not a member; `cc0` (without `-1.0`)
is not a member.

The sidecar `eval/datasets/manifest.csv` already carries values like
`internal` (literal). The iter-1 sources will fail any future
`scripts/check_licence_allowlist` or equivalent.

**Fix:** Either (a) align ac-01's enum with the SPDX values used in
`licence_allowlist.txt`, or (b) explicitly extend
`eval/licence_allowlist.txt` in this iteration (add an AC for it) and
choose one casing convention.

### B5. ontology_schema `no_gt` description contradicts the iter-1 corpus shape

**Where:** ontology_schema, lines 491-497.

The `no_gt` description says: "The 10 ac-04 CSVs have full GT; the
remaining ~25 CSVs contribute `no_gt`-only counts." This is a leftover
from a 30-50-CSV draft. Iter-1 ships **7** CSVs with full GT (constraint
at line 19, ac-04 names exactly 7 sidecars). There are no remaining
~25 CSVs in iter-1.

This contradiction will mislead the implementer about which mechanism
tally is expected to be non-zero in the iter-1 baseline.

**Fix:** Rewrite `no_gt` to: "Iter-1 ships 7 CSVs all with full GT;
`no_gt` is structurally 0 this iteration. The mechanism exists for
forward-compatibility with the corpus expansion follow-up
(card 0015)."

---

## Concerns (non-blocking but worth raising)

### C1. ac-09's monotonicity claim is too strong

ac-09 verification (line 286-288) states: "the corpus-level count of
failing columns attributed to `enum_overfit` is ≤ baseline's count
(monotonic — it can only suppress over-eager enums, never add them)".

This is true at the predicate level — but the harness re-classifies
failures via the attribution rules in ac-07. If suppressing an enum
causes the column to *pass* validation entirely, `enum_overfit`
count drops AND the dataset row count of valid rows rises (good). If
suppressing causes the column to fall through to `format_diversity`
or `unknown` (because the broader `pattern` constraint now bites), the
mechanism count for those *can rise*, and the spec's monotonicity
claim only covers `enum_overfit` itself. That's fine semantically but
the verification language could mislead an implementer into expecting
no other mechanism count to move. Spell out that other mechanism counts
are unconstrained by the fix.

### C2. ac-10's monotonicity claim has the same shape

ac-10 verification (line 333-337): "the corpus-level count of failing
columns attributed to `format_diversity` is ≤ the baseline's count
(monotonic — widenings can only convert format_diversity failures
into passes, never the reverse)". Same caveat as C1: a widening could,
in principle, cause a column that was `format_diversity` to become a
`misclassification` (the now-accepted format triggers a different
inference path on a re-run? No — the harness profiles once. So this is
actually safe in this iter's design). Still, name the assumption
explicitly: "the harness profiles each CSV exactly once per run; the
schema is the artefact under widening, not the model".

### C3. ac-04 column-count discrepancy — 50 vs 58

ac-04 description (line 131) says "approximately 50 columns" then in
the next sentence sums per-dataset to 58. The same number (58) appears
in implementation_notes line 437 ("Total: 58 columns under GT.").
Just say "approximately 58 columns".

### C4. Card 0015 stub creation has no AC

The constraint at lines 16 says a stub card is created at
`orbit/cards/0015-validate-corpus-curation.yaml` "in this iteration". No
AC verifies its creation. Either add a `ac_type: doc` AC for it ("the
file exists; status: draft; references this spec") or drop the
constraint and accept that card 0015 lands separately when curation
sprint kicks off.

### C5. ac-13 `make ci` must include the new harness binary

ac-13 says "The new harness binary is included in the workspace test
matrix". `make ci` runs `cargo test --workspace`, which DOES include
all binaries' unit tests by default — but `cargo test --workspace`
does NOT run binaries themselves (i.e. it runs `#[test]` blocks inside
the binary's source, not the binary). The ac-07 test verification
(line 220) runs `cargo test -p finetype-eval validate_corpus --
attribute_`. Confirm this is the unit-test-block invocation pattern,
not an integration test, otherwise `make ci` won't actually exercise
it. Recommend an `#[cfg(test)] mod attribute_tests` adjacent to the
binary's `main` function.

### C6. Profile binary path assumption

implementation_notes line 435: "the shelled binary should be the
workspace's release build (`target/release/finetype`)". When the
harness runs in CI or a fresh checkout, `target/release/finetype` may
not exist. The Makefile target should `cargo build --release -p
finetype-cli` as a prerequisite, or the harness should resolve via
`cargo run -p finetype-cli --release --` (which transparently
ensures-built). Spec should pin which.

### C7. ac-05's profile invocation flag may not exist

ac-05 step 2 (line 154): `finetype profile -f <csv> -o json-schema
--stdout`. The profile output formats already write to stdout by
default (see `2026-04-28-profile-json-schema-output/spec.yaml`
constraint: "Stdout-by-default for `-o json-schema` — no `--stdout`
flag, no sidecar"). So `--stdout` is wrong here — it'll trip clap.
Drop `--stdout` from ac-05.

### C8. ac-07's attribution rule 2 could trigger on intentional enums

Rule 2 says: `predicted_label == expected_label AND any reject row has
constraint_failed='enum'` → `enum_overfit`. After ac-09 ships, the
only enums that survive are categorical-family with cardinality ≤
threshold — these CAN legitimately reject. A real value not in the
`['M', 'F']` enum is a valid reject. ac-07 rule 2 will still
attribute it as `enum_overfit`, conflating "enum was over-fit" with
"data has unenumerable variants in a legitimately-low-cardinality
column". Mechanism naming is then slightly misleading. Either rename
to `enum_constraint_failure` (mechanism-neutral) or add a tie-break
("if cardinality > threshold-after-fix, it's not a fix candidate").
Probably acceptable for iter-1, but flag it in the post-fix delta
prose.

### C9. Reused infra implies fields missing in `manifest.csv`

The current `eval/datasets/manifest.csv` schema is
`dataset, file_path, column_name, gt_label, source_url, licence,
fetched_date` (7 cols, m-19 schema). The spec's
`validate_manifest.csv` adds `provenance_status, gt_sidecar_path,
row_count, column_count` (9 cols). Two different shapes is fine —
they serve different units (column-keyed vs file-keyed) — but ac-03
says `compute_row_hashes.py` will iterate BOTH manifests. The
iteration code must handle two different column orderings; spec
should explicitly note: "the row-hash extension reads
`validate_manifest.csv` columns by name (csv.DictReader), not
positionally".

### C10. Rio2016 athletes licence check

implementation_notes line 425: rio2016_athletes from
`flother/rio2016` is described as "CC0/public-domain". A quick
check at the source (https://github.com/flother/rio2016) shows the
code is MIT and the data licence is "this data is in the public
domain" — so "public-domain" is closer than CC0. Pin the licence
exactly per the source's LICENSE/README to avoid an audit
follow-up. (Also: ac-01's allowlist excludes `MIT` for licences,
but the rio2016_athletes provenance has MIT-licensed code with
public-domain DATA, so the data licence is what manifests in
`validate_manifest.csv` — fine, just be precise.)

---

## Strengths

- **Decision discipline.** Three MADRs scoped tightly; each has a
  clear question (round-trip metric / infra reuse / fix partition).
  ac-11 names titles, dates, and minimum-Considered-Options counts.
- **Bounded fix surface.** `≤5 widenings` (ac-10) and a hard
  cardinality cap (ac-09) prevent scope creep into "let me also fix
  a few other validators while I'm here".
- **Movement-based success (ac-14)** is the right shape for a
  baseline iteration — the structural deliverable is the harness +
  baseline, not a number that may not move.
- **Mechanism ontology is well-named** and maps cleanly to MADR 0071
  reject ontology. The five-rule deterministic attribution table
  (ac-07) is auditable and unit-testable.
- **Disjointness check** (implementation_notes line 424) is
  explicit and lists every existing eval CSV — no name collision
  with `eval/datasets/csv/`.
- **Iter-1 corpus selection** stresses each named mechanism: pokemon
  for enum-overfit, un_locode for format-diversity, rio2016 for
  code-vs-canonical, world_population as control. Good test design.

---

## Verdict reasoning

The spec is well-structured and the iteration is correctly scoped.
The blockers are factual mismatches with the live codebase
(taxonomy labels, sources.yaml schema, licence allowlist) and one
flag-collision (B2) that would cause real UX confusion. Fixing
them is a surgical edit pass, not a redesign. After the five
blockers land, I'd APPROVE.

**Verdict:** REQUEST_CHANGES
