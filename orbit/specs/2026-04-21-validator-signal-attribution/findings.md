# Validator-signal attribution — findings

**Date:** 2026-04-21
**Branch:** `v17-re-eval-on-expanded-corpus` (continuing — discovery committed
on the same branch as PR #43's successor, TBD)
**Model under test:** sherlock-v16 via `models/default`
**Instrument:** `crates/finetype-model/examples/validator_signal_trace.rs`
(run output captured at `trace_output.txt` in this directory)

---

## TL;DR

The validation branch is **architecturally correct and alive at inference**.
The silent-zeros hypothesis from the interview is FALSE. But the branch
doesn't rescue http_method or excel_format for two independent reasons:

1. **Validator precision is polluted.** On `http_method` values
   ("GET", "POST", ...), the 240-dim pass-rate vector shows **25+ types
   with pass_rate = 1.000** — including `comma_separated`,
   `postal_code`, `longitude`, `street_name`, `country`, `city`. The
   validation branch learned to use this signal where it's informative
   (email, country_code: ✅) and ignore it where it isn't. Types whose
   validators permit arbitrary short strings cannot be rescued by this
   branch, no matter what its training weights say.
2. **Post-processing is actively demoting correct predictions.** Raw
   `MultiBranchClassifier.classify_column` returns
   `technology.internet.http_method` (conf 0.595) on the http_method
   eval column. The full `finetype profile` pipeline returns
   `representation.discrete.categorical` (conf 0.373). Something
   between raw multi-branch and final output strips the correct
   answer.

Neither root cause is "wire a validator-authoritative promotion Sharpen
rule" as originally framed in MADR 0058. The right next spec is
narrower and different.

---

## Q1 — Silent-zeros hypothesis: FALSE

The CLI's `cmd_profile` at `crates/finetype-cli/src/main.rs:3855` loads
the taxonomy and calls `column_classifier.set_taxonomy(taxonomy)` before
profiling. `ColumnClassifier.classify_multi_branch` at `column.rs:1984`
passes `self.taxonomy.as_ref()` into `mb.classify_column(...)`, which
propagates it to `compute_validation_tensor(...)` at
`multi_branch.rs:543`. No `None` leak. Validation branch receives real
features at eval-time inference.

---

## Q2 — 240-dim pass-rate vector contents (instrument output)

Captured via `cargo run -p finetype-model --example
validator_signal_trace --release`. Top-10 validation pass rates per
column, followed by the expected type's rank out of 240:

### http_method (expected `technology.internet.http_method`)
Values: GET, POST, PUT, DELETE, PATCH, HEAD (6 items)

```
| rank | pass_rate | type                                   |
|------|-----------|----------------------------------------|
| 1    | 1.000     | container.array.comma_separated        |
| 2    | 1.000     | container.array.pipe_separated         |
| 3    | 1.000     | container.array.semicolon_separated    |
| 4    | 1.000     | container.array.whitespace_separated   |
| 5    | 1.000     | container.object.csv                   |
| 6    | 1.000     | geography.address.postal_code          |
| 7    | 1.000     | geography.address.street_name          |
| 8    | 1.000     | geography.coordinate.longitude         |
| 9    | 1.000     | geography.location.city                |
| 10   | 1.000     | geography.location.country             |
| …    | 1.000     | (25 total types at 1.000)              |
| 25   | 1.000     | technology.internet.http_method        |
```

**The expected type is tied at rank 25-of-240 with 24 other types, all
at pass_rate = 1.000.** The branch has no feature-level way to prefer
http_method over, say, postal_code — the pass-rate vector is flat.

### excel_format (expected `representation.file.excel_format`)
Values: xlsx, xls, csv, ods, xlsm, xlsb (6 items)

Same ten-way tie at rank 1–10, expected type at rank 19 of 240 with
pass_rate = 1.000.

### country_code (expected `geography.location.country_code`) — control
Values: DE, IN, CA, DE, AE, IN, AU, GB, JP, US

```
| rank | pass_rate | type                                   |
|------|-----------|----------------------------------------|
| 1–9  | 1.000     | (9 other types — permissive validators) |
| 10   | 1.000     | geography.location.country_code <--    |
```

Expected type rank 10-of-240. **The ISO 3166-1 alpha-2 enum added in
v12's ac-01 makes country_code's validator precise enough that only
valid 2-letter country codes pass.** The branch CAN discriminate
country_code from postal_code etc. — but only because the validator
is enum-constrained.

### email (expected `identity.person.email`) — control
Values: william.jones34@outlook.com … (10 items)

Expected type rank 12-of-240 with pass_rate = 1.000. Email's regex
pattern is precise enough that it's one of a small set of types
passing all values.

---

## Q3 — Per-branch contribution (ablation: `taxonomy = None`)

Running `mb.classify_column(values, header, Some(&tax))` vs
`mb.classify_column(values, header, None)` — the second call takes the
silent-zeros path in `compute_validation_tensor`, feeding the
validation branch zeros:

```
| column        | normal prediction                          | ablated prediction                  | verdict                                         |
|---------------|--------------------------------------------|-------------------------------------|-------------------------------------------------|
| http_method   | technology.internet.http_method (0.595)    | technology.internet.http_method (0.512) | BOTH CORRECT — branch adds 0.083 confidence only |
| excel_format  | representation.text.word (0.335)           | representation.text.entity_name (0.988) | BOTH WRONG — different labels, branch shifts but doesn't fix |
| country_code  | geography.location.country_code (0.994)    | geography.location.country_code (0.328) | BOTH CORRECT — branch adds 0.666 confidence      |
| email         | identity.person.email (0.995)              | identity.person.username (0.777)       | BRANCH RESCUES — validation is the deciding signal |
```

**Key observation:** the validation branch **does earn its keep** for
email and country_code — both cases where the validator is precise
(regex / enum). It's effectively floor-level for http_method
(+0.083 confidence contribution) and actively wrong-direction for
excel_format (normal = text.word 0.335, ablated = entity_name 0.988;
the branch is pulling it toward one wrong answer instead of another).

---

## Q4 — Final per-class logits: not instrumented

The debug binary did not capture raw pre-softmax logits; the argmax
label + confidence were sufficient to reveal the structure above.
Adding a top-5 logit dump is a follow-up if the next spec needs it.

---

## The second finding: post-processing demotes correct http_method

Comparing raw multi-branch (`example` binary) to full CLI profile:

```
| column        | raw multi-branch                        | finetype profile (CLI full pipeline)  |
|---------------|-----------------------------------------|----------------------------------------|
| http_method   | technology.internet.http_method (0.595) | representation.discrete.categorical (0.373) |
| excel_format  | representation.text.word (0.335)        | representation.discrete.categorical (0.318) |
```

For **http_method, the raw multi-branch IS correct** and Sharpen demotes
it. For **excel_format, the raw multi-branch is already wrong** (text.word);
Sharpen's `disambiguate_categorical` then demotes text.word → categorical
because the rule's guard "current top is a generic type → categorical
when 3-20 unique short non-numeric values" matches exactly.

The path for http_method is different from excel_format and not yet
fully traced. Best current hypothesis: the CLI uses
`classify_columns_with_context` which runs **sibling-context attention**
over all 110 coverage_closure_phase_ab headers before the multi-branch
forward pass. The enriched header for "http_method" is surrounded by
categorical-ish siblings (`comma_separated`, `pipe_separated`,
`periodicity`, `excel_format`, `extension`, `color_rgb`, …), which may
shift the raw label enough that `disambiguate_categorical` then
demotes to categorical. **Not instrumented yet — this is the single
remaining open question from this discovery.**

Relevant code:
- `classify_columns_with_context` — `column.rs:830–883`
- `disambiguate_categorical` — `column.rs:3881–3942` (guard at L3913–3924,
  generic types are `text.word`, `text.plain_text`, `text.abbreviation`,
  `integer_number`, `day_of_month` + BOOLEAN_LABELS)
- `sharpen_attractor_demotion` — `column.rs:2919–3031`; http_method is
  NOT in TEXT_ATTRACTORS / NUMERIC_ATTRACTORS / CODE_ATTRACTORS, so this
  rule is not the culprit for http_method.

---

## Decisions

### D1 — Root cause is two-factor, not one
1. **Validator precision pollution** limits the validation branch's
   ceiling for types whose validators accept common short strings.
   This is the Precision Principle from CLAUDE.md made concrete and
   observable.
2. **Sharpen post-processing** demotes the model's output via
   generic-fallback rules that don't consult the validation branch
   signal — specifically `disambiguate_categorical` and
   (probably, for http_method) sibling-context enrichment upstream.

### D2 — Reject the originally-proposed "validator-authoritative promotion" Sharpen rule
That rule was a `categorical → named_type` promotion. The evidence
shows the actual pattern is `named_type → categorical` demotion
happening AFTER the model correctly predicts the named type. The
correct intervention is a **demotion guard, not a promotion step**:
prevent Sharpen from demoting a named-type prediction when the
validation branch's pass-rate for the named type is 1.0 AND the
validator is precise.

### D3 — Patch policy: do NOT patch in this session
The interview reserved the patch decision for evidence. The evidence
is that (a) the fix is not a one-liner — it needs a new Sharpen guard
with "validator precision" as a concept, and (b) there's still one
open mechanism question (sibling-context's contribution to the
http_method demotion path). A dedicated `/orb:spec` will produce a
cleaner, reviewable change than sneaking a fix into this discovery PR.

---

## Recommendations

### Next spec: "Sharpen demotion guard — validator-confirmed named types are not demoted to generic categorical"

Scoped, concrete, no retrain required:

- **Rule**: in `disambiguate_categorical` (and, if confirmed,
  similar generic-fallback rules in the Sharpen stack), add a
  pre-check: if `taxonomy.get_validator(current_label).is_some()` AND
  every non-empty sampled value passes that validator, skip demotion.
- **Constraint**: must not break existing passing tests (the function
  has coverage at `column.rs:5944–6002`).
- **Acceptance**: eval deltas — excel_format should move
  `categorical → excel_format`; http_method needs the
  enrichment/raw-prediction question answered first, which may fold
  in or be carried as a second AC.
- **Evidence to collect in spec discovery**: run
  `classify_columns_with_context` on the coverage_closure_phase_ab.csv
  and capture the raw multi-branch label BEFORE Sharpen applies, for
  http_method specifically. Confirms whether enrichment flips the raw
  label or Sharpen is the sole actor.

### Deferred: validator precision audit

Materially larger lever. The 240-dim pass-rate vector has 25+
simultaneous 1.000 entries on plausible columns because many type
validators are functionally no-ops for short strings (minLength/
maxLength with no pattern, patterns that match `^.+$`, enum lists
that don't cover the real semantic, etc.). Fixing them would
substantially raise the validation branch's ceiling. This is probably
a v18 retrain-accompanying spec, not a Sharpen-layer fix.

Park this until after the demotion-guard spec ships and measurement
shows whether residual errors are in the Sharpen layer or the branch
itself.

### Follow-up artefact: promote the debug binary

If the next spec's discovery needs more of this evidence, promote
`validator_signal_trace.rs` to a shipped `finetype profile
--debug-validation` flag. Not in scope for this card.

---

## Artefacts

- `interview.md` — the discovery interview record.
- `trace_output.txt` — raw stdout from the debug binary.
- `crates/finetype-model/examples/validator_signal_trace.rs` — the
  debug binary (checked in, reproducible).
- This document.

## Reproduction

```
cargo run -p finetype-model --example validator_signal_trace --release
FINETYPE_MODEL=models/default ./target/release/finetype profile \
  --file eval/datasets/csv/coverage_closure_phase_ab.csv -o json \
  | jq '.columns[] | select(.column=="http_method" or .column=="excel_format")'
```

## One thing to preserve across compaction

**The root cause is NOT "wire a promotion step". It is "don't demote a
named-type prediction when the validator confirms it."** The next
spec is a Sharpen demotion *guard*, narrower and more defensible than
the originally-framed promotion rule. The validator precision
question is the larger, parked, probably-retrain-adjacent lever.
