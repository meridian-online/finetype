---
status: accepted
date-created: 2026-04-20
date-modified: 2026-04-20
---
# 0051. HTTP Method via YAML-Schema ENUM Only

## Context and Problem Statement

`technology.internet.http_method` is the only one of the 7 v16
bad-distilled types whose value space is closed, small, and entirely
specifiable in a schema: 9 IETF methods × 3 case conventions (UPPER,
lower, Title) = 27 variants. The v16 generator produced `PATROL`,
`OPERATING`, `IN PROGRESS`, `SAN JOAQUIN` — nouns and sentences that
happened to be uppercase — and the distilled rows were random uppercase
text. Training the model on 1000+ synthetic rows that are wrong is
worse than training on zero rows: the model learns "http_method =
any uppercase token" and misclassifies `country_code`, `currency_code`,
and short codes as http_method.

A JSON Schema `enum` is authoritative for a closed set. If the schema
correctly enumerates every legal value, no distilled or generated row is
needed — the validator catches non-members, and the model learns
http_method from the schema branch.

A complication: `CompiledValidator` (`crates/finetype-core/src/validator.rs`)
applies `pattern` AND `enum` **conjunctively**. A case-insensitive regex
like `(?i)^(GET|POST|...)$` does NOT cover all 27 variants if the enum
lists only `[GET, POST, ...]` — `get` would match the pattern but fail
the enum, and vice versa.

## Considered Options

- **A. Generator path** — write a generator that emits http_method with
  realistic distribution. Training cost. Still wrong if the generator
  drifts from the IETF-registered method set.
- **B. Public dataset path** — harvest `Method` columns from HTTP log
  datasets (Apache/nginx/Common Log Format samples on Kaggle). Requires
  a loader, license review, and ongoing freshness maintenance.
- **C. YAML-schema ENUM-only** — enumerate all 27 case variants in both
  `enum` and `pattern` alternation. No distilled rows, no generator
  change. The schema *is* the training signal.

## Decision Outcome

Chosen option: **C — YAML-schema ENUM-only**. The value space is too
small to justify a dataset, too closed to justify a generator, and the
schema can express the ground truth exactly.

Implementation — **the 3-surface cascade** (a single schema change
propagates through three downstream surfaces):

1. **`labels/definitions_technology.yaml` L283-286** — `pattern` is the
   regex alternation `^(GET|Get|get|POST|Post|post|...)$` with all 27
   variants listed literally. `enum` lists the same 27 variants. No
   `(?i)` flag.
2. **`crates/finetype-core/src/validator.rs`** — `CompiledValidator`
   compiles the YAML into a `jsonschema::Validator`. Because pattern AND
   enum are applied conjunctively, both must enumerate the same 27
   variants. A new `ac07_http_method_case_variants` unit test asserts
   all 27 positives plus rejection of v16-era bad tokens (`PATROL`,
   `SAN JOAQUIN`), mixed-case (`gET`, `POSt`), whitespace variants, and
   adjacent-token strings (`GET /`, `POST /users`).
3. **Training pipeline** — http_method remains in `_DROP_DISTILLED_TYPES`
   in `scripts/prepare_multibranch_data.py`. No distilled rows are
   consumed. The synthetic generator is untouched (per decision 0049).

### Consequences

- Good, because the schema is authoritative: any change to IETF methods
  or case conventions is a one-line YAML edit that cascades automatically
  to validation and training.
- Good, because we avoid the "garbage in, garbage out" failure mode
  where generator/distilled noise teaches the model to accept any
  uppercase token.
- Good, because validator precision is provably complete — every legal
  value is in the enum, every illegal value is rejected.
- Bad, because we are language-scoped: non-English HTTP methods (there
  are none in practice, but theoretically future RFCs could add them)
  would require a schema update, not a training update.
- Bad, because any new IETF method (rare) requires a YAML edit rather
  than being learned automatically from fresh data.
- Neutral, because the 27-variant enumeration is verbose but explicit —
  reviewers see exactly what's legal and what isn't.

## References

- Spec: `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3)
- Prior decision: `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
- Schema: `labels/definitions_technology.yaml` (http_method block)
- Validator + test: `crates/finetype-core/src/validator.rs` (`http_method_schema`, `ac07_http_method_case_variants`)
- Training prep: `scripts/prepare_multibranch_data.py` (`_DROP_DISTILLED_TYPES`)
