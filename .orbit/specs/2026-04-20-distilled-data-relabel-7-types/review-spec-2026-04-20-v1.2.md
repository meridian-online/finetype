# Spec Review (v1.2)

**Date:** 2026-04-20
**Reviewer:** Context-separated agent (fresh session, cold read of v1.2)
**Spec:** `.orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (metadata.version 1.2)
**Verdict:** REQUEST_CHANGES

---

## Summary

v1.2 closes all v1.1 findings in substance. The sourcing_table is now a
first-class field with named candidate URLs; ac-11 defines "winner" with
a 3-tier tiebreaker; a no-promotion branch exists in both ac-09 (train
gate) and ac-11 (eval gate); ac-10 pins the baseline to corpus-freeze +
git SHA; ac-06 explicitly decides the 27-variant enum strategy; former
ac-08 is collapsed into ac-06.

The spec is close to shippable. But there is one concrete factual error
(HIGH) that will make ac-01/ac-04/ac-05 fail on first run, and a few
governance questions worth resolving before implementation. Verdict:
REQUEST_CHANGES — a 10-minute edit pass, then ready.

---

## v1.1 findings — closure assessment

```
| ID  | v1.1 finding                                      | v1.2 closure status |
|-----|---------------------------------------------------|---------------------|
| H1  | ac-01/ac-02 data availability asserted not evidenced | CLOSED — candidate URLs in sourcing_table, fallback clause added |
| H2  | ac-12 winner undefined, no tiebreaker/rollback    | CLOSED — ac-11 defines winner + 3-tier tiebreak + no-promotion |
| M1  | v16 baseline timing ambiguous                     | CLOSED — ac-10 pins to corpus-freeze + git SHA |
| M2  | ac-06 enum strategy deferred to implementer       | CLOSED — 27 variants enumerated in spec, (?i)-alone forbidden |
| M3  | ac-08 is a non-AC                                 | CLOSED — collapsed into ac-06 |
| M4  | sourcing table lives in constraints, no drift guard | CLOSED — sourcing_table is top-level, mutations require spec bump |
| L1  | ac-05 validator scope creep                       | CLOSED — scope bounded to chain traversal + canonical-key resolution |
| L2  | ac-15 MADR-count ambiguity                        | CLOSED (implicitly — ac-14 now enumerates 4 and sourcing policy is MADR (a)) |
| L3  | ac-13 HF-first ordering implicit                  | CLOSED — ac-12 requires curl HEAD 200 BEFORE PR opens |
| L4  | ac-10 seed-naming convention implicit             | CLOSED — `models/sherlock-v17-seed-{42,43,44}` pinned in ac-09 + constraint |
```

All 10 v1.1 findings substantively addressed.

---

## v1.2 Findings (graded fresh)

### [HIGH] H1-v1.2: Canonical taxonomy labels in sourcing_table are wrong

**Category:** assumption (factual error)

**Description:** The sourcing_table (spec.yaml L20, L77) uses
`technology.web.user_agent` and `technology.web.http_method`. These are
not canonical taxonomy keys. Grep of `labels/` confirms the canonical keys
are `technology.internet.user_agent` (definitions_technology.yaml:240) and
`technology.internet.http_method` (definitions_technology.yaml:271).
`_DROP_DISTILLED_TYPES` in `scripts/prepare_multibranch_data.py:867-875`
also uses `technology.internet.*` for both.

The other 5 sourcing_table entries are correct (`finance.banking.swift_bic`,
`identity.medical.cpt`, `representation.file.excel_format`,
`identity.government.ssn`, `identity.medical.loinc`).

If the implementer treats the sourcing_table as authoritative and their
loader emits rows labelled `technology.web.user_agent`, then ac-05
(label_remap validator) will correctly fail ("unresolved label") — but
only after a full loader build. Worse, ac-04 verification's grep
(`assert user_agent + LOINC are NOT in the list`) will look for
`technology.web.user_agent` in `_DROP_DISTILLED_TYPES` (which is
`technology.internet.user_agent`), not find it, and pass spuriously
without the actual code change needed.

**Evidence:**
- spec.yaml:20, 77 (sourcing_table uses `technology.web.*`)
- `labels/definitions_technology.yaml:240,271` (canonical is `technology.internet.*`)
- `scripts/prepare_multibranch_data.py:867-875` (internal constant uses `technology.internet.*`)

**Recommendation:** Replace `technology.web.user_agent` → `technology.internet.user_agent`
and `technology.web.http_method` → `technology.internet.http_method` in the
sourcing_table. Same error likely propagates into SOURCES.md and the loader
file names — flag the type-key convention at the top of the sourcing_table
field ("keys are canonical taxonomy labels from labels/definitions_*.yaml").

### [HIGH] H2-v1.2: Manual-review gate is self-cleared

**Category:** failure-mode / governance

**Description:** ac-11 "Manual-review branch" says: "if winner's val_acc
is in 88–91.2% AND profile eval passes the gate, the implementer adds an
explicit 'manual review cleared' line to progress.md before promotion."

The implementer is the person who just trained the model and wants to
ship. They self-clear their own manual review. This defeats the purpose
of the gate — the band exists because the model *might* have a data
quality problem that profile eval doesn't catch (the profile eval is 242
columns; val_acc is computed against a held-out slice of the training
corpus; the two can diverge). The whole point of a "manual review" is a
second pair of eyes.

There are two plausible intents:
(a) The implementer is expected to do a genuine investigation (look at
    per-column regressions, inspect loss curves, sanity-check predictions
    on specific columns) before writing "cleared". That's fine but the AC
    should require *artefacts* of the review (checklist, investigation
    notes, not just a line).
(b) The band is advisory only — ship and watch. Then call it that, drop
    the "manual review" framing.

As written, it's the weakest possible gate: a one-line self-attestation.

**Evidence:** ac-11 L365–367; constraint L137–141.

**Recommendation:** Pick one:
(i) Require an evidence block in progress.md: "For a winner in the 88–91.2%
    band, progress.md includes: per-column regression table vs v16, a
    per-type confidence histogram, and a plain-English justification."
(ii) Rename "manual review" → "flag for post-ship monitoring" and drop
     the attestation step. The 88% floor already does the safety job.
(iii) Route to Hugh for explicit approval (checkpoint pattern from
      CLAUDE.md "Decision Checkpoints"). Lightest touch that preserves
      the spirit.

Option (iii) matches the project's "Decision Checkpoints" convention and
is cheapest.

### [MEDIUM] M1-v1.2: Spec-bump-for-sourcing-table-mutation is a heavy interrupt

**Category:** constraint-conflict (friction)

**Description:** The spec now says mutations to the sourcing_table
require a spec bump to v1.3+ (header comment L15–17; constraint on
fallback L122–128). This is good for governance — but practically, if
ac-01's primary+secondary user_agent candidates fail on first attempt,
the implementer must:

1. Stop implementation mid-loader.
2. Open a v1.3 spec (edit metadata.version, write changes_from_v1_2,
   update the sourcing_table entry to `path: generator`).
3. Re-run `/orb:review-spec` on v1.3 (per orbit convention).
4. Wait for review.
5. Resume implementation on the now-generator path.

For a license/availability flip — a minor factual correction, not a
design change — this is a heavy interrupt. The spec doesn't say whether
`/orb:review-spec` is mandatory on v1.3 bumps or whether self-review is
acceptable.

**Evidence:** spec.yaml L15–17, L122–128; orbit workflow conventions in
CLAUDE.md.

**Recommendation:** Clarify the bump process. Options:
(a) Explicit: "Sourcing-table mutations require a spec bump. A fresh
    `/orb:review-spec` is NOT required for pure path-swaps (public_dataset
    → generator) that invoke the documented fallback clause — only the
    version bump + changes_from entry."
(b) Lighter: treat path-swaps as spec amendments recorded in
    `progress.md` with a one-line rationale, not as v1.3. Reserve v1.3
    for material scope changes.

Option (a) is safer (keeps the governance) while removing the interrupt
cost.

### [MEDIUM] M2-v1.2: ac-12/ac-13 rollback chain ill-defined

**Category:** failure-mode

**Description:** Inter-AC rollback. ac-12 step 2 ships the workflow bump
+ models/default flip in a single PR. ac-13 then pushes the v0.6.18 tag
and publishes the release. Question: if ac-13 fails (tag push blocked,
release workflow errors, homebrew auto-bump fails), has ac-12 already
shipped? Yes — ac-12 merges a PR that changes `models/default`, so every
CLI/MCP/DuckDB user pulling main now runs v17. But there's no released
`finetype` binary for v17 yet. Inconsistent state: repo says v17,
installed binaries still at v0.6.17.

Two options: (a) ac-13 is atomic with ac-12 (same PR? tag on the same
commit?); (b) ac-13 has a rollback spec (revert the ac-12 PR if ac-13
fails).

Neither is specified. v0.6.17's release spec
(`.orbit/specs/2026-04-20-v16-release/handover.md`) probably has a precedent
but the current spec doesn't reference it.

**Evidence:** ac-12 L376–394; ac-13 L396–406.

**Recommendation:** Add to ac-13: "If release workflow fails, revert the
ac-12 PR (restoring `FINETYPE_CI_MODEL=sherlock-v16` + `models/default
→ sherlock-v16`) before investigating. Do not leave repo state where
main points at v17 but no v0.6.18 binary exists."

### [MEDIUM] M3-v1.2: Candidate URLs are plausible but unverified

**Category:** assumption

**Description:** Verifying the 4 candidate URLs in sourcing_table:

- **ua-parser/uap-core (L22):** real, Apache-2.0, active at
  github.com/ua-parser/uap-core. regexes.yaml is a regex pattern file,
  NOT a UA-string fixtures file. The repo has `test_resources/` directory
  with UA fixtures (tests/test_ua.yaml, test_device.yaml etc.) which
  contain hundreds to thousands of UA strings as test inputs. So ua-parser
  IS a viable source — but the spec cites the wrong file (regexes.yaml ≠
  fixtures). The implementer will waste a round trying regexes.yaml
  before finding test_resources/.

- **Kaggle hari141v/browser-user-agent-strings-for-web-scraping (L23):**
  cannot verify without web access. Kaggle user `hari141v` and that slug
  are plausible. CC-0 is common on Kaggle. Not red-flagged.

- **LOINC primary (L30):** "search for loinc-codes mirror" is not a URL.
  This is a placeholder. LOINC is Regenstrief-licensed; GitHub "mirrors"
  of the full LOINC database would be license violations, which is why
  the spec hedges. For 1000 rows the spec's own fallback ("LOINC top-100
  common codes") is a more honest primary.

- **LOINC secondary (L31):** also placeholder ("verify license at ac-01
  time").

**Evidence:** spec.yaml L22, L23, L30, L31. ua-parser repo structure
known (regexes.yaml is patterns, test_resources contains fixtures).

**Recommendation:**
(a) Fix the ua-parser URL reference: cite `test_resources/*.yaml` (UA
    fixtures), not `regexes.yaml` (patterns).
(b) Demote the LOINC entry: make the fallback (top-100 common codes
    seeded from a published list — e.g. the CDC-curated common lab codes
    list) the primary, with "public dataset if found" as an upside. This
    avoids the "triage licensing at ac-01 time" work that v1.1 review
    complained about.

### [LOW] L1-v1.2: 27-variant enum maintainability

**Category:** assumption

**Description:** v1.2 commits to enumerating all 27 case variants in
both `enum` AND `pattern` (spec L83; constraint L100–107). This is the
right call for correctness (`(?i)` alone wouldn't work because the enum
is exact-string — see validator.rs:95). But it ossifies the schema:
adding a 10th HTTP method (e.g. `LINK`, `UNLINK` from RFC 2068, or the
newer `QUERY` draft) becomes a 3-string change per method. Not a blocker;
flag so future maintainers aren't surprised.

**Recommendation:** One-line note in ac-06 or the http_method YAML
comment: "Future method additions expand both enum and pattern by 3
variants." Optional.

### [LOW] L2-v1.2: 88% floor rationale is a retrofit

**Category:** assumption (minor)

**Description:** Constraint L139–143 says "v14 shipped at 91.2% val_acc
with real eval quality; the 88% floor is ~3pp below that (≈1σ)". Two
issues:
(a) v14's 91.2% was on v14's corpus, which did NOT include real distilled
    user_agent/LOINC data. v17's corpus is materially different. The 1σ
    claim assumes val_acc variance is corpus-invariant, which is not
    obviously true.
(b) "≈1σ" is asserted, not computed. There's no reference to actual
    val_acc variance across v13/v14/v15/v16 checkpoints.

Not a blocker because:
- The gate is soft (manual review band, not reject-below-88 band).
- The profile eval gate (ac-11) is the real quality promotion gate.
- The ordering (88% < 91.2%) is defensible even without rigour.

**Recommendation:** Reword L141–143 to "88% is a catastrophic-floor
heuristic — ~3pp below v14's 91.2%, chosen to catch data-corruption /
label-remap bugs without hard-rejecting a materially different corpus.
Not grounded in a measured sigma." Honesty over rigour.

### [LOW] L3-v1.2: ac-12 curl HEAD verification is after-the-fact

**Category:** test-gap (minor)

**Description:** ac-12 verification requires "HF HEAD request returns
200 for all 3 files BEFORE the promotion PR is opened." A reviewer
reading the merged PR cannot verify the "BEFORE the PR is opened"
timing after the fact — all they see is that the files currently resolve
on HF. Nothing prevents an implementer from uploading AFTER the PR is
merged (drift-check would catch this within the CI run, but only at
workflow-bump merge time, not at publish time).

**Evidence:** ac-12 L389–391.

**Recommendation:** Make the evidence mechanical: "progress.md records a
curl HEAD transcript with timestamps that precede `git log -1 --format=%cI
<promotion-PR-merge-commit>`". Or more simply: record the HF upload
commit SHA on the meridian-online/finetype-model repo, check its
timestamp against the promotion PR merge commit. Low priority.

### [LOW] L4-v1.2: ac-07 unit test coverage asymmetry

**Category:** test-gap

**Description:** ac-07 asserts `CompiledValidator::is_valid` returns
true for all 27 variants and false for `"GOAT"`, `"SAN JOAQUIN"`,
`"PATROL"`. The false list is arbitrary and small. Missing from the
false list: case-mutants of valid methods that should also fail under
the enum (e.g., `"gET"`, `"POSt"`, `"Delete "` with trailing space).
If the implementer accidentally adds a case-insensitive pattern without
the 27-variant enum, `is_valid("gET")` might pass through pattern while
failing enum — the test would still pass (returns false overall) but
for the wrong reason.

**Recommendation:** Add one assertion: "`is_valid` returns false for
`gET`, `POSt`, and `"GET "` (trailing space) — ensures enum-exact-match
is enforced, not just pattern." Small cost, high signal.

---

## Assumption audit

Assumptions still implicit in v1.2:

- ua-parser/uap-core fixtures live at `test_resources/*.yaml` not
  `regexes.yaml` (see M3).
- LOINC public dataset exists on acceptable terms (the spec already
  hedges via fallback, but still lists placeholders — see M3).
- The promotion skill/release workflow enforces ac-13's tag-push
  ordering relative to ac-12 (see M2).
- Self-attestation in progress.md is an adequate manual-review gate
  (see H2).
- `technology.web.*` labels are equivalent to `technology.internet.*`
  — they are not (see H1).

## Failure-mode analysis (per-AC highlights)

- **ac-01:** loader emits `technology.web.*` labels that don't exist in
  the taxonomy → ac-05 validator fails late, wasting a training run. See H1.
- **ac-04:** grep for `technology.web.user_agent` in
  `_DROP_DISTILLED_TYPES` finds nothing (actual constant uses
  `technology.internet.user_agent`) and "passes" vacuously. Same bug
  as H1.
- **ac-09:** all 3 seeds land in the 88–91.2% band. ac-11 "manual
  review cleared" is self-attested. Worst-case: a subtly broken corpus
  ships with a one-liner approval. See H2.
- **ac-10:** corpus-freeze baseline captures eval against a stale v16
  checkpoint — fine if the verifier grabs the SHA correctly. Good.
- **ac-11:** all seeds below max(235, v16_baseline) → no promotion path
  exists AND the "investigation" next step is not structured (just
  "documented in progress.md"). Minor.
- **ac-12 + ac-13:** inter-AC rollback undefined. See M2.

## Test adequacy

Strong:
- ac-07 function-name-prefix convention carries forward (good for
  /orb:review-pr scan).
- ac-05 positive + negative test. Good.
- ac-09 structured output files per seed. Good.
- ac-10 pinned git SHA captured. Excellent — makes eval reproducible.

Weak:
- ac-07 negative list is thin (see L4).
- ac-11 "manual review cleared" has no evidence requirement (see H2).
- ac-12 curl-HEAD verification is after-the-fact (see L3).

## Gap analysis

- **H1 — wrong type-key namespace in sourcing_table.** Load-bearing.
- **H2 — self-cleared manual review gate.**
- **M2 — no inter-AC rollback specified between ac-12 and ac-13.**
- **M3 — candidate URL for ua-parser points at the wrong file
  (regexes.yaml vs test_resources).**
- Golden-tests still not folded into promotion (carryover from v1.1,
  flagged as non-blocking then; still non-blocking but worth noting —
  `cargo test -p finetype-cli --test cli_golden -- --ignored` per
  CLAUDE.md).

## Constraint check

- Sourcing policy is internally consistent.
- Promotion-ordering constraint (L170–178) mirrors the CI-decouple spec
  — matches CLAUDE.md "Promotion flow". Correct.
- SSN synthetic-only constraint honoured.
- Spec-bump-on-sourcing-table-mutation is governance-positive but
  friction-heavy (see M1).

---

## Honest assessment

v1.2 is a meaningful improvement on v1.1. All 10 prior-round findings
are closed in substance, the sourcing_table is first-class, ac-11 is
properly structured, and the inter-spec references (CI-decouple flow,
decision 0049, N=1 email) are now tight.

One HIGH-priority fix is load-bearing:

**H1 — the sourcing_table uses `technology.web.*` where the taxonomy
uses `technology.internet.*`.** This is a 2-line edit but it will cause
ac-04's verification grep to silently pass (finding nothing) while the
actual drop-list constant is never updated. The implementer will
discover this either (a) when ac-05 fails at corpus-prep time, wasting
hours, or (b) worse, not at all — if `_DROP_DISTILLED_TYPES` isn't
updated because the grep said it was fine, v4's user_agent rows get
dropped by the existing `technology.internet.user_agent` entry and the
whole point of ac-01 is erased.

One HIGH-priority governance issue:

**H2 — "manual review cleared" is self-attested.** Either make it a
Hugh-checkpoint (project convention per CLAUDE.md), require an evidence
block, or demote it from "review" to "post-ship monitoring".

Three MEDIUMs worth folding in:
- M1 (spec-bump friction on sourcing mutations — clarify process)
- M2 (ac-12/ac-13 rollback chain)
- M3 (candidate URL for ua-parser points at the wrong file; LOINC
  primary is a placeholder)

Four LOWs are optional polish.

**Recommend REQUEST_CHANGES**: fix H1 (2-line taxonomy key correction)
and H2 (choose a manual-review resolution). Pick up M1/M2/M3 in the
same pass. Once those land, this spec is ready for `/orb:implement`.

The ambiguity_score of 0.04 in metadata is optimistic — with H1 being
a concrete factual error and H2 being a governance hole, real ambiguity
is closer to 0.10. Not a blocker, but recalibrate.
