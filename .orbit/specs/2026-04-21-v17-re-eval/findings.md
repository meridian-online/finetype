# v17 re-eval on expanded corpus — findings

**Date:** 2026-04-21
**Branch:** `v17-re-eval-on-expanded-corpus`
**Trigger:** Decision 0054 held v17 from promotion pending expanded-eval
measurement. PR #42 (eval-expansion Phase A+B) merged to main at commit
`4631b3c`. This re-score is the first measurement of v17 against the 448-row
corpus decision 0054 called for.

---

## Headline numbers

Format-detectable scored subset (352 columns):

```
|                     | label correct | label %  | domain correct | domain %  |
|---------------------|---------------|----------|----------------|-----------|
| v16 (default)       | 297 / 352     | 84.4%    | 323 / 352      | 91.8%     |
| v17-seed-44         | 295 / 352     | 83.8%    | 326 / 352      | 92.6%     |
| delta               | −2            | −0.6pp   | +3             | +0.8pp    |
```

Full 448-row manifest (combined):

```
|                     | matches       | accuracy |
|---------------------|---------------|----------|
| v16 (default)       | 308 / 448     | 68.8%    |
| v17-seed-44         | 302 / 448     | 67.4%    |
| delta               | −6            | −1.4pp   |
```

By coverage origin:

```
|                     | previously_covered | newly_covered |
|---------------------|--------------------|---------------|
| v16                 | 250 / 338 = 74.0%  | 58 / 110 = 52.7% |
| v17-seed-44         | 248 / 338 = 73.4%  | 54 / 110 = 49.1% |
| delta               | −2                 | −4               |
```

**v17 does not outperform v16 on the expanded eval.** Losses are small, gains
are small, net is negative-or-flat depending on the slice. Decision 0054's
original read ("identical eval vs v16 + 3 non-target regressions") holds after
expansion.

---

## Relabel-target head-to-head (the decisive slice)

PR #40 shipped relabel work for 7 types: swift_bic, cpt, loinc, excel_format,
user_agent, ssn, http_method. The eval manifest has 10 rows touching these
types. Comparing v16 and v17-seed-44 row-by-row:

```
| Dataset                   | Column       | gt       | v16                 | v17                            | delta            |
|---------------------------|--------------|----------|---------------------|--------------------------------|------------------|
| tech_systems              | user_agent   | user agent | ❌ jwt              | ❌ jwt                         | stable-miss      |
| network_logs              | user_agent   | user agent | ❌ docker_ref        | ❌ whitespace_separated        | stable-miss*     |
| server_logs_json          | user_agent   | user agent | ✅ user_agent        | ✅ user_agent                  | stable-hit       |
| people_directory          | ssn          | ssn      | ✅ ssn              | ✅ ssn                         | stable-hit       |
| new_identity              | ssn          | ssn      | ✅ ssn              | ✅ ssn                         | stable-hit       |
| new_identity              | loinc        | loinc    | ✅ loinc            | ✅ loinc                       | stable-hit       |
| new_identity              | cpt          | cpt      | ✅ cpt              | ✅ cpt                         | stable-hit       |
| codes_and_ids             | issn         | issn     | ✅ issn             | ✅ issn                        | stable-hit       |
| coverage_closure_phase_ab | excel_format | repr.file.excel_format | ❌ categorical | ❌ categorical          | stable-miss      |
| coverage_closure_phase_ab | http_method  | tech.internet.http_method | ❌ categorical | ❌ categorical      | eval-gt-stale**  |
```

\* network_logs/user_agent failure mode shifted from `docker_ref` to
`whitespace_separated`; still wrong, different prediction.

\*\* http_method is a **named** categorical — decision 0051 shipped
http_method as ENUM-only and dropped it from distilled training
(`_DROP_DISTILLED_TYPES`), with the intent that the schema/validator layer
would promote the generic enum-shape prediction to the named type
`technology.internet.http_method`. In practice the pipeline is stopping at
`representation.discrete.categorical` without promoting — the 6-row column's
values all match the http_method enum, but the inference pipeline never runs
that check. This is a pipeline-integration gap, not a model gap. Same
shape of failure for excel_format (format-string validator knows the
pattern; pipeline returns categorical).

**Score: 6 stable-hit / 4 stable-miss / 0 fix / 0 regression.** The relabel
work is **invisible** on the 10 rows that measure it.

### A surfaced pipeline gap: validator → named-type promotion

Two of the 4 stable-misses (`http_method` and `excel_format`) share a
structural shape: the model's multi-branch prediction is `categorical`,
the column's values all match a specific type's validator/enum, and no
post-processing step promotes the prediction to the specific named type.

Decision 0051 assumed the schema/validator signal would flow through the
pipeline to produce `technology.internet.http_method`. That flow isn't
wired. This is probably a **Sharpen rule or validator-post-processing gap**,
not a training problem — meaning it's tractable without a retrain, and
v17's identical score to v16 on these rows is consistent with both models
hitting the same pipeline ceiling rather than the same training-data gap.

This promotion gap is a **separate, smaller, concrete card** from the
feature-discrimination discovery:

- **Small card (concrete):** Wire validator-authoritative promotion.
  When model predicts `categorical` and every value passes a named-type
  validator (http_method enum, excel_format pattern, any other schema-carrying
  type), promote the prediction. `/orb:spec`-able; existing Sharpen rule
  pattern (R1–R31, F1–F6).
- **Larger card (discovery):** Feature discrimination for user_agent-like
  types where no validator exists and the pattern is ambiguous.
  `/orb:discovery`.

`swift_bic` does not appear here because it has zero eval coverage (MADR 0055
restricted-registry carve-out under `synthetic-necessary`). The carve-out
protects the coverage gate but also means swift_bic relabel is structurally
unmeasurable in the current eval.

---

## Diagnosis

**The relabel PR (#40) did not move the eval.** Three candidate explanations,
in order of plausibility:

1. **Signal-to-noise argument.** PR #40 fixed ~45 distilled rows across 7
   types. Each type's training dataset is ~1000 synthetic + distilled rows.
   45 / (7 × 1000+) ≈ 0.6% of the training signal. A fix at that scale falls
   below the noise floor even if the relabel was entirely correct. **This is
   the most likely explanation.**

2. **Feature-discrimination ceiling.** The stable-miss types show a specific
   pattern: `excel_format → categorical`, `http_method → categorical`,
   `user_agent → jwt|whitespace_separated`. These are all "tiny-vocabulary
   type mistaken for a generic enum / generic token pattern." The multi-branch
   model's 36-dim stats branch has no feature that says "this column's values
   look like format codes, not free-text category labels." No amount of
   training-label cleaning fixes a discrimination gap that lives in the
   feature representation.

3. **Displaced capacity.** v17's regressions on unrelated types (array
   separators, datetime specificity, protein_sequence) suggest some training
   capacity got redirected at the expense of previously-learned types. A
   second-order effect, but a real one.

**Rollup:** the relabel hypothesis doesn't have the wrong sign — it has the
wrong magnitude. Fixing 45 rows is too small an intervention to show up
against the 7000+ training rows per type. The next move is not more relabel;
it's a question about **what we'd need to change for the numbers to move at
all**.

---

## Regressions & fixes on newly-covered bucket (reference)

v17 flipped 7 previously-correct rows to wrong and 3 previously-wrong rows to
correct, both relative to v16, on the newly-covered bucket:

**Regressions (v16 ✅ → v17 ❌):**
- container.array.comma_separated → representation.text.entity_name
- container.array.pipe_separated → representation.discrete.categorical
- datetime.date.mdy_short_slash → datetime.date.dmy_short_slash
- datetime.date.month_year_slash → datetime.component.year
- datetime.date.weekday_full_month → datetime.date.weekday_abbreviated_month
- representation.scientific.protein_sequence → representation.identifier.alphanumeric_id
- technology.cryptographic.token_urlsafe → technology.internet.url

**Fixes (v16 ❌ → v17 ✅):**
- datetime.timestamp.dot_dmy_24h
- datetime.timestamp.iso_8601_compact
- representation.numeric.si_number

Net: −4 on this bucket. None of the regressions or fixes involve relabel
target types.

---

## Recommendation

1. **Do not promote v17.** Ratify decision 0054 under expanded-eval evidence
   as MADR 0058. v17-seed-{42,43,44} can be deleted from disk or archived.

2. **Treat "training-data quality" as a completed-but-insufficient lever.**
   The CLAUDE.md "What's next" item suggesting more distilled-data fixes for
   the 7 target types is not the right next move — the signal-to-noise
   argument says 45 more fixes won't move the needle either.

3. **The open question is feature discrimination.** `excel_format → categorical`
   and `http_method → categorical` can't be fixed by data alone. The stats
   branch needs a feature (or the embed branch needs context) that separates
   "vocabulary-constrained format token" from "open-domain enum label". This
   is a `/orb:discovery` entry, not `/orb:spec`.

4. **Stop rehearsing the relabel question.** The next retrain (v18) doesn't
   get to start against the same uncorrectable discrimination. The pre-v18
   work is measurement-and-design, not training.

---

## Artefacts

- `v16-baseline-profile_results.csv` — v16's predictions on expanded eval
- `v16-baseline-report.md` — full v16 eval report (as of PR #42 merge)
- `v16-baseline-delta_by_coverage.md` — v16 split by coverage origin
- `v17-seed-44-profile_results.csv` — v17 seed-44's predictions
- `v17-seed-44-report.md` — full v17 eval report
- `v17-seed-44-delta_by_coverage.md` — v17 split by coverage origin

Run reproduction:

```
FINETYPE_MODEL=models/sherlock-v17-seed-44 ./eval/profile_eval.sh eval/datasets/manifest.csv
uv run --with pyyaml python3 scripts/eval_delta_by_coverage.py
```

Both v17-seed-{42,43} are on disk and available for additional seeds of
this comparison if the seed-44 result is doubted. Seed-44 was the winner
in PR #40's 3-seed sweep; this re-score uses that choice.
