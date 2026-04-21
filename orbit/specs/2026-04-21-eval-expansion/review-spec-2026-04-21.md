# Review: eval-expansion spec (Phase A+B)

**Spec:** `orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.0)
**Reviewer:** Nightingale (fork session, context-separated)
**Date:** 2026-04-21
**Passes run:** 1 (Structural) + 2 (Assumption/failure) + 3 (Adversarial)

---

**Verdict:** REQUEST_CHANGES

**Findings by severity:** 3 blocking, 5 high, 4 medium, 3 low.
The spec is well-structured, the MADR plan is right, and phasing is sane.
But there is at least one concrete breakage in the consumer code of
`manifest.csv`, and several ACs are soft enough that "done" is not
mechanically determinable. A focused revision should clear these; no
rewrite required.

---

## Pass 1 — Structural scan

### Gate-AC deterministic verification check

Two `ac_type: gate` ACs (ac-05, ac-12). Verification fields inspected:

```
| AC     | len | placeholder? | concrete?                                            |
|--------|-----|--------------|------------------------------------------------------|
| ac-05  | 210 | no           | yes — names script, inputs, exit-code contract       |
| ac-12  | 197 | no           | partial — names a file + section + "per-type delta"  |
```

Both pass the minimum bar (non-empty, non-placeholder, ≥20 chars).
ac-12 is weaker than ac-05 (see F-H3 below).

### AC testability per-row

```
| AC     | type   | testable | notes                                                  |
|--------|--------|----------|--------------------------------------------------------|
| ac-01  | code   | yes      | file-exists + output-shape check                       |
| ac-02  | config | yes      | header + zero-empty-cells                              |
| ac-03  | doc    | yes      | row-count + field-presence (weak on *quality* of triage)|
| ac-04  | code   | weak     | "realism bar" is asserted but not codified — F-H1      |
| ac-05  | gate   | yes      | set-containment + exit code 0                          |
| ac-06  | code   | yes      | byte-identical regeneration                            |
| ac-07  | code   | weak     | "expected removal count" is undefined — F-B1           |
| ac-08  | doc    | weak     | "every distinct source" is a set, but populating it across all loaders is itself a discovery task |
| ac-09  | doc    | yes      | MADR file-exists + accepted status                     |
| ac-10  | doc    | yes      | MADR file-exists + docstring reference                 |
| ac-11  | doc    | yes      | MADR file-exists + link                                |
| ac-12  | gate   | weak     | narrative-only verification — F-H3                     |
| ac-13  | config | yes      | grep + diff                                            |
| ac-14  | doc    | yes      | file-exists + section presence                         |
```

### Constraint conflicts

None material. The "existing 242 columns may be kept" constraint lines
up with triage-action schema in ac-03. The "no LLM-as-judge" constraint
lines up with ac-01's deterministic pre-screen.

### Scope vs goal alignment

Goal is a three-dimensional realism standard + coverage floor +
leakage prevention. ACs cover all three. Phase C is cleanly excluded.

### Content signals (triggers Pass 2)

- Training data pipeline changes (ac-07 touches `prepare_multibranch_data.py`)
- Ground-truth schema changes (ac-02 mutates `manifest.csv`)
- Cross-system boundaries (`manifest.csv` consumed by 6+ tools)
- Eval correctness dependency (ac-05 coverage gate)

Pass 2 is triggered.

---

## Pass 2 — Assumption & failure analysis

### Assumptions the spec makes, and what breaks when wrong

**A1. Consumers of `manifest.csv` tolerate appended columns.**
*Reality:* false for at least one consumer. `eval/profile_eval.sh:78,148`
reads the CSV with bash `while IFS=, read -r dataset file_path column_name gt_label`.
Bash `read` with 4 variables and an input line of 7 comma-separated
fields **packs the remaining 3 fields into `gt_label`** — so
`gt_label` becomes `"id,https://...,CC-BY-4.0,2026-04-21"`. The
eval pipeline silently produces zero correct predictions against
this corrupted GT. **This is F-B1 below.**

Rust consumers (`eval_mapping.rs:181`, `eval_actionability.rs:116`,
`extract_features.rs:42`, `data.rs:798`) use `csv::Reader` with
positional `record.get(N)` for N∈{0,1,2,3}. These are backward-compatible
provided the new columns are *appended* (as the spec says). The bash
script is the exception and needs an explicit fix-up task.

**A2. Row-hash dedup catches the contamination that matters.**
The spec normalises `(lower(header), value)` with case-preserved value
and trimmed whitespace. Failure modes not covered:
- A value reformatted between train and eval (e.g. `"GET"` vs `" get "`,
  `"2024-01-01"` vs `"2024-01-01T00:00:00Z"`) will **not** collide.
- Header renamed (e.g. `"method"` in eval, `"http_verb"` in training)
  will **not** collide even if value sets are identical.
- This gives a false sense of leakage coverage. The MADR (ac-10)
  must state these limits; otherwise it becomes a floor that is
  later mistaken for a ceiling. See F-H2.

**A3. Every taxonomy type can be covered with a real column.**
240 types include some that resist real sourcing under the
public-dataset-or-generator policy of decision 0050 — e.g.
`identity.medical.cpt` (AMA-licensed), `identity.government.ssn`
(PII). ac-05 asserts 100% coverage but gives no escape hatch
for types whose only realistic source is synthetic-because-we-have-to.
Under decision 0050 (Option A: public or generator), synthetic
generator output ends up in the eval set — contradicting the
"provenance_status ∈ {real, hand-curated}" bar in ac-04.
**This is F-B2.**

**A4. The 242-column human review fits in one sprint.**
Prior art: `orbit/specs/2026-04-18-v16-data-audit-retrain/eval-audit.md`
reviewed 338 rows across 1–2 days (interview line 28 cites this).
The new audit is 242 rows plus **new sourcing** for zero-coverage
types plus ~80 MADR/manifest/sources.yaml fields filled in by hand.
Interview Q9 estimates "1–2 weeks" for Phase A+B. The spec does
not commit to a time-box, surface a daily rate assumption, or
note what falls out if the audit over-runs. See F-M1.

**A5. v16 re-scores can serve as the new baseline (ac-12).**
If the expanded eval adds columns v16 was never trained on (e.g.
swift_bic, cpt, loinc, excel_format, user_agent — exactly the
zero-coverage list), v16's score will **drop** mechanically — not
because v16 got worse, but because the denominator grew on
types it was never expected to know. ac-12 records this as
"the new baseline for the next retrain", which is fine, but the
per-type delta table is not differentiated between "previously-
unmeasured type" and "regression on a previously-measured type".
A future reviewer looking at a 235→225 drop will mis-diagnose.
See F-H3.

**A6. `sources.yaml` role can be determined retroactively.**
ac-08 requires classifying every existing source as
`train | eval | both-forbidden`. For a source that produced both
an eval CSV (under `eval/datasets/`) and training rows (via a
v4 loader), the correct role is `both-forbidden` — and the
remedy is to drop it from *one* side. The spec does not say
which side loses, who decides, or what happens if Nightingale
finds (say) 10 sources in this state. See F-H4.

**A7. The pre-screen "floors" in ac-04 exist.**
ac-01 describes metrics (null_rate, unique_ratio, whitespace_ratio,
format_variance, shannon_entropy, top_k_skew) but does not pin
thresholds. ac-04 then references "the pre-screen script's
messiness + distributional-fidelity floors for the column's
type family". Those floors are neither defined in the spec nor
assigned a deliverable. See F-B3.

**A8. MADRs can be drafted alongside implementation, not before.**
ac-09 (realism dimensions) and ac-10 (leakage) encode decisions
that constrain *how* ac-01, ac-06, ac-07 are built. If the MADRs
ship last, the code may define facts the MADR then rubber-stamps —
inverting the decision register's purpose. No ordering is
specified. See F-M2.

### Failure-mode analysis — key ACs

**ac-02 fails if:** a row exists in the new manifest where `licence`
is a freeform string (e.g. "CC BY 4.0" vs `CC-BY-4.0`). The
verification only checks non-empty; downstream tooling that
expects SPDX identifiers (per ontology_schema lines 178-179) will
silently fail later. See F-M3.

**ac-05 fails if:** a type has coverage in manifest.csv but its
`file_path` points at a missing/empty file. ac-05's verification
checks set-containment only via `schema_mapping.yaml` resolution,
not via actually loading the column. A type can be "covered on
paper" but yield zero rows at profile time. See F-M4.

**ac-07 fails if:** training corpus diverges from eval at
normalisation boundaries (A2) — the filter reports "0 rows removed"
and the verification check ("reports the expected removal count")
degenerates. "Expected count" is undefined. See F-B1.

**ac-13 fails if:** the retrain block is only written in sweep
script comments + CLAUDE.md. Neither is a mechanical enforcement.
A future agent running `./scripts/sweep_v17.sh` (or a v18 successor)
with no block-check will just train. Recommend either a pre-flight
check in the sweep script that reads this spec's status, or
accept this as process-only and say so explicitly. Not blocking.

### Test adequacy per `verification` field

```
| AC     | adequacy   | gap                                                    |
|--------|------------|--------------------------------------------------------|
| ac-01  | adequate   | none                                                   |
| ac-02  | weak       | no licence-format validation (F-M3)                    |
| ac-03  | adequate   | weak on triage *quality* — "one line" not enforced     |
| ac-04  | inadequate | "realism bar" is asserted not tested (F-H1)            |
| ac-05  | inadequate | paper-coverage vs real-coverage (F-M4)                 |
| ac-06  | adequate   | byte-identical is a strong test                        |
| ac-07  | inadequate | "expected removal count" undefined (F-B1)              |
| ac-08  | weak       | union completeness is an afternoon of discovery itself |
| ac-09  | adequate   | MADR template + status check                           |
| ac-10  | adequate   | but constraints matter — see F-H2                      |
| ac-11  | adequate   | script link check is concrete                          |
| ac-12  | weak       | narrative + numeric — F-H3                             |
| ac-13  | adequate   | grep + diff                                            |
| ac-14  | adequate   | section presence                                       |
```

Pass 2 finds structural problems. Pass 3 runs.

---

## Pass 3 — Adversarial

### Simultaneous-failure scenarios

**S1. Manifest rewrite + profile_eval.sh unpatched.**
ac-02 lands first (schema change committed), profile_eval.sh
is patched in a later commit. Between those two commits, every
CI eval run produces garbage (gt_label contains commas from
appended URLs). Because `profile_eval.sh` is used by
`make eval-report` and may be invoked by anyone re-scoring a
model, this produces silently-wrong numbers. *Mitigation:* ac-02's
verification must include "all consumer tools still pass a
known-baseline eval", not just "header has 7 columns".

**S2. Row-hash filter live but mis-normalised.**
ac-07 goes green (filter imports `row_hashes.tsv`, logs
"N rows removed"). If the normalisation on the training side
differs from the eval side by one character (e.g. `lower()` vs
`casefold()`, or trimming `\r\n` vs `\n`), the filter silently
removes the wrong rows. The verification has no cross-check that
a known contaminated row is actually removed. Recommend a unit
test with a planted collision.

**S3. Three MADRs drafted post-hoc.**
If ac-09/10/11 land after the code ACs, the MADRs will describe
what was built rather than what was decided. The spec does not
enforce ordering. See F-M2.

### Cascade analysis

- **Manifest schema change → downstream consumers:** `profile_eval.sh`
  (breaks), `eval_mapping.rs`/`eval_actionability.rs`/`extract_features.rs`/`prepare_sense_data.rs`→`data.rs` (tolerant), `scripts/prepare_multibranch_data.py` (doesn't read manifest — uses distillation output), DuckDB SQL loaders for eval dashboard (untested in this review — worth confirming).
- **Row-hash filter active → training corpus size shrinks.** If the
  filter removes, say, 5k training rows, downstream model behaviour
  shifts. Retrain-block (ac-13) covers this, but only for v18+ —
  the post-expansion v16 re-score in ac-12 uses the **already-trained**
  v16 which was built with contaminated data. So "the new baseline"
  measures v16 against eval rows v16 saw during training. That's
  literally the leakage ac-10 is supposed to prevent. See F-B3.

### Rollback feasibility

- Manifest rewrite: reversible via git; but downstream callers that
  cached the new schema (none identified, but worth a grep before
  merge) would need reverting too. OK.
- Row-hash filter: feature-flag-able; spec says "active by default"
  but does not require a disable switch. Recommend a `--no-dedup`
  flag for emergency rollback + debugging.
- Triage worklist: pure doc, no rollback needed.
- MADRs: can be marked `superseded` if later revised.

### Impact radius

The single biggest-radius change is **ac-07** (train-pipeline row-hash
filter). Every future training run goes through this. A silent bug
here costs full retrains.

---

## Findings

### Blocking (must fix before implementation starts)

**F-B1 — `profile_eval.sh` will mis-parse the extended manifest.**
The bash `read` loop at `eval/profile_eval.sh:78` and `:148` reads
exactly 4 fields; appending `source_url,licence,fetched_date` will
fold those three values into `gt_label`, producing wrong eval
numbers silently. ac-02's verification (header column count + empty
cell check) will pass while the pipeline is broken.
*Fix:* add a sub-AC under ac-02 that runs `make eval-report` on
the v16 model against the extended manifest and asserts the score
is unchanged from the pre-change baseline. Also explicitly patch
`profile_eval.sh` as part of ac-02's scope.

**F-B2 — ac-05 requires 100% coverage that decision 0050 forbids.**
Decision 0050 (Option A) prohibits restricted-registry scraping for
training. If that policy applies to eval too (not explicitly stated
in 0050), types like CPT (AMA-licensed), LOINC (licence-permitting
but ambiguous), SWIFT BIC (ISO 9362 — registry fees) cannot get
real sourced columns under ethical constraints. The spec's
`provenance_status ∈ {real, hand-curated}` (ac-04) then conflicts
with "≥1 column per type" (ac-05) for these types.
*Fix:* either (a) widen provenance_status to include `synthetic-
necessary` with an explicit MADR carve-out, or (b) exempt a named
list of types from the real-data coverage floor and require only
synthetic coverage for them, or (c) clarify that decision 0050
applies to training only, not eval, with an updated attribution
plan for the restricted registries.

**F-B3 — ac-01's "floors" are referenced by ac-04 but never defined.**
ac-04's verification hinges on "the pre-screen script passes its
messiness + distributional-fidelity floors for the column's type
family". Neither ac-01 nor ac-09 (realism MADR) defines what those
floors are. A later session cannot run ac-04's verification without
picking the floors on the spot — which is exactly the kind of
implicit decision the spec process should surface.
*Fix:* add a sub-AC (or expand ac-01) pinning the floor values,
or pin them in the realism MADR (ac-09) and reference them from
ac-04's verification.

### High (should fix)

**F-H1 — "Realism bar" is not codified in ac-04.**
Same cluster as F-B3 but narrower: "Replacement columns meet the
realism bar" has no test. As worded, the verification only cross-
checks dates and source_url — a column could be perfectly synthetic
with a fabricated HuggingFace URL and still pass.
*Fix:* ac-04 verification must assert `provenance_status` is
`real` or `hand-curated` per row AND the pre-screen script returns
pass for that row.

**F-H2 — Leakage MADR (ac-10) must enumerate known filter blind
spots.** The row-hash scheme cannot detect format-drift collisions
(timestamp reformatting, case changes beyond `lower(header)`,
whitespace normalisation that differs from the filter's). Without
these limits in writing, the MADR becomes a load-bearing "solved"
claim that later review treats as a ceiling.
*Fix:* ac-10's description should require the MADR to list
known blind spots under "Consequences / Bad".

**F-H3 — ac-12's per-type delta table will be misread without a
"newly-measured" flag.** Zero-coverage types (swift_bic, cpt, loinc,
excel_format, user_agent plus audit surprises) will appear in the
delta as mechanical losses. Without a `newly_measured: bool` column
or equivalent, the table will look like v16 regressed, when it's
an unbiased estimator finally measuring what was invisible before.
*Fix:* ac-12 verification should require the per-type delta to
tag each row as `previously_covered | newly_covered` and to
report the two subset scores separately.

**F-H4 — ac-08's `both-forbidden` resolution has no workflow.**
When `sources.yaml` audit finds sources currently feeding both
training and eval, the spec does not specify which side loses or
who decides. For types where eval-side removal breaks ac-05
coverage, there is a 3-way deadlock (train vs eval vs coverage).
*Fix:* add an explicit rule — e.g. "eval keeps; train relocates
to an alternative source; if no alternative exists, the type is
re-sourced" — and a sub-AC that counts the `both-forbidden`
resolutions.

**F-H5 — v16 re-score in ac-12 is measured with a contaminated
baseline.** v16 was trained before row-hash dedup existed. Its
training set likely overlaps with the current eval. Running v16
against the *expanded* eval set gives a score that is partly
genuine and partly leakage — and is then declared "the new
baseline for the next retrain". The next retrain is then
measured against this contaminated floor.
*Fix:* either (a) note explicitly in ac-12 that the v16 re-score
is diagnostic-only and will not serve as the promotion floor for
v18, OR (b) rebuild v16's training corpus under the new filter
and re-train (expensive, likely out of scope).

### Medium

**F-M1 — No time-box on the audit itself.** Interview estimates
"1–2 weeks" but the spec has no progress-gate. If the audit over-
runs, it blocks all other model work by ac-13. Recommend a stated
checkpoint (e.g. "if Phase A is not substantially complete by
day 5, escalate to Hugh") or a scope-trim option (e.g. "if
replace worklist > 40 columns, drop lowest-priority and backlog").

**F-M2 — MADR ordering not specified.** ac-09/10/11 should land
before or alongside the code ACs they govern. Recommend
sequencing explicit in the spec: MADR drafts → code → MADR
finalisation.

**F-M3 — Licence field format unvalidated.** ac-02 asserts
non-empty; ontology_schema line 179 claims SPDX identifiers.
Without a validator, "CC BY 4.0" and "CC-BY-4.0" both pass,
but only the latter is SPDX. *Fix:* ac-02 validator must check
licence values against an allowlist or SPDX registry.

**F-M4 — ac-05's coverage check is paper-only.** A row exists
in manifest but file missing → set-containment still green.
*Fix:* extend the coverage script to also assert every
referenced file exists and has ≥N rows for the column in
question.

### Low

**F-L1 — `FINETYPE_CI_MODEL` env var not mentioned.** Recent CI
decoupling work (CLAUDE.md) added a separate env var authoritative
for CI. If ac-13 (sprint goal update) is checked against CI
behaviour, the two env vars may drift. Low because CI is out of
this spec's direct path, but worth a one-liner in ac-13.

**F-L2 — "Existing ground-truth labels are not renegotiated except
as a side-effect of replacement" (constraint line 17) contradicts
ac-03 soft.** `replace` may implicitly change gt_label (different
source file → different header → different mapping). Not blocking,
but recommend calling this out in the triage worklist schema.

**F-L3 — `FINETYPE_MODEL` vs `FINETYPE_MODEL_DIR` confusion risk
in ac-12.** The v16 re-score must use the correct env var (per
decision 0049's post-mortem). Recommend ac-12's verification cite
the env var explicitly to prevent the v16-was-actually-v14 class
of bug.

---

## Summary of recommended changes

1. **Block before implementation:** fix F-B1 (add sub-AC to patch
   `profile_eval.sh` + run-to-completion regression test on
   extended manifest), resolve F-B2 (carve-out for types with no
   public-real source or widen provenance), codify floors (F-B3).
2. **High-priority edits:** strengthen ac-04 verification (F-H1),
   require MADR blind-spot enumeration (F-H2), add newly-measured
   flag to ac-12 (F-H3), give ac-08 a resolution rule (F-H4),
   caveat v16 re-score (F-H5).
3. **Medium:** time-box the audit, pin MADR ordering, validate
   licence format, make coverage gate non-paper.
4. **Low:** note CI env var, call out gt_label side-effects in
   triage, cite env var in ac-12.

Strength through simplification (decision 0038) applies: the spec is
already close to right — the blocking items are specific gaps, not
structural reworks. Expected turnaround on the revision: half a day.

---

## Appendix — files inspected

- `orbit/specs/2026-04-21-eval-expansion/spec.yaml`
- `orbit/specs/2026-04-21-eval-expansion/interview.md`
- `orbit/specs/2026-04-20-distilled-data-relabel-7-types/handover.md`
- `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
- `orbit/decisions/0050-per-type-sourcing-policy.md`
- `orbit/decisions/0052-scope-aware-eval-gate.md`
- `orbit/decisions/0054-hold-v17-no-promotion.md`
- `orbit/decisions/_template.md`
- `eval/datasets/manifest.csv` (first 30 rows; 338 total)
- `eval/profile_eval.sh`
- `eval/schema_mapping.yaml` (structure)
- `scripts/prepare_multibranch_data.py` (drop-set + loader layout)
- `crates/finetype-eval/src/bin/eval_mapping.rs`
- `crates/finetype-eval/src/bin/eval_actionability.rs`
- `crates/finetype-cli/src/bin/extract_features.rs`
- `crates/finetype-train/src/bin/prepare_sense_data.rs`
- `crates/finetype-train/src/data.rs` (manifest loader)
