# Memo: is `finetype generate` more useful than faker libraries?

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, generators, training

## What the command does

```
finetype generate
    Generate synthetic training data

Options:
  -s, --samples <SAMPLES>    Number of samples per label [default: 100]
  -p, --priority <PRIORITY>  Minimum release priority [default: 3]
  -o, --output <OUTPUT>      Output file [default: training.ndjson]
  -t, --taxonomy <TAXONOMY>  [default: labels]
      --seed <SEED>          Random seed [default: 42]
      --localized            Generate 4-level labels with locale suffixes
```

Iterates the taxonomy, calls each type's `generator` (Rust closure
keyed by type name), emits `{text, classification}` NDJSON. Output
feeds the training pipeline.

## What's behind the scenes

The generator infrastructure is substantial — `crates/finetype-core`
defines per-type generators that compose `fakeit` (Rust faker), `fake`
(another Rust faker), `chrono`, `uuid`, `rand`, plus FineType-specific
logic for things like Luhn-valid credit cards, valid SWIFT BICs, IBANs
with check digits, RFC-compliant emails. Each generator is paired with
a validator in the same taxonomy entry — that's what `finetype check`
enforces.

So `finetype generate` is **not** wrapping a faker library. It's
exposing FineType's own taxonomy-aligned generators. Faker libraries
do not know what `geography.address.postal_code` means in FineType's
sense, nor do they emit values that pass FineType's validation block.

## Compared to faker libraries

```
| Capability                                  | finetype generate | Python faker | Rust fake |
|---------------------------------------------|-------------------|--------------|-----------|
| 240 type-specific generators                | yes               | ~50–80       | ~30–40    |
| Output validates against FineType taxonomy  | yes               | no           | no        |
| Locale-aware (locale_specific designations) | yes               | yes          | partial   |
| Cryptographically-valid IBAN/SWIFT/Luhn     | yes               | partial      | partial   |
| Library use (programmatic API)              | no — CLI only     | yes          | yes       |
| Streaming / per-row generation              | no — bulk NDJSON  | yes          | yes       |
| Active maintenance ecosystem                | no                | yes          | yes       |
```

## Who actually uses it

```
| Caller                              | Why                                                |
|-------------------------------------|----------------------------------------------------|
| `scripts/prepare_multibranch_data.py` | training data synthesis                          |
| Sweep scripts (v16, v17, v18, v19)  | regenerate training data per sweep                 |
| `make eval-actionability`           | actionability eval test inputs                     |
| External users                      | …unclear                                           |
```

The internal callers all want bulk NDJSON for training. They use
`finetype generate` because it's already there and produces aligned
data.

For external users, the value proposition is narrow but real:

- **"I'm writing tests for a pipeline that uses FineType."** Generating
  data that I know will pass `finetype validate` is genuinely useful,
  and faker can't do this without me hand-rolling alignment.
- **"I'm building a FineType-aware schema and need fixture data."**
  Same answer.
- **"I want fake names / addresses / emails for general use."** Faker
  is better — has streaming, locale richness, broader ecosystem,
  programmatic API.

The first two are ~real but small. Most users in those situations
write a Python script with `faker` anyway because the FineType CLI
emits bulk NDJSON, not on-demand single values.

## Three options

**A. Hide it (`#[command(hide = true)]`).** Same treatment as `train`,
`eval`, and the proposed `check`. The training pipeline still calls it
via `cargo run -- generate`. External users who genuinely need
"training data for FineType" can find it via docs or `--help-hidden`
(if clap supports it) — but it stops cluttering `finetype --help`.

**B. Keep public, redirect docs.** Add a doc note in `--help` and
`docs/USAGE.md`: "For general fake data, use Python `faker` or Rust
`fake`. This command is for FineType-aligned training/test fixtures."
Honest framing without removal.

**C. Expose as a programmatic API on the MCP server.** The MCP
server already has a `generate` tool (see CLAUDE.md MCP table —
"Generate synthetic sample data for a type"). That's the right shape
for the "I want a few valid IBANs for my test" use case — call it as
a tool from an agent. The CLI's bulk NDJSON output is for the training
pipeline; the MCP tool covers the per-type/per-value use.

If C is already in place (yes, per the MCP tool list), then the CLI's
public-facing value is mostly redundant for the per-value use case
already.

Recommendation: **A + acknowledge C exists.** Hide the CLI subcommand;
keep MCP `generate` for the small set of users who want per-type
sampling through their agent. This matches the pattern: bulk training
artefacts are maintainer-internal; small-batch sampling is a tool call.

## Composition with other memos

```
| Memo                          | Recommendation             |
|-------------------------------|----------------------------|
| cli-check-internal            | hide                       |
| cli-generate-vs-faker (this)  | hide                       |
| cli-model-flag                | hide / env-var             |
| cli-sharp-only-flag           | remove                     |
| cli-model-type-flag           | hide → feature-gate legacy |
```

Pattern: **maintainer-only tooling stops being public.** Five flags
and two subcommands move out of the user surface. The user-facing
binary shrinks to: `infer | profile | validate | schema | load | mcp`
(plus `taxonomy` for browsing).

That's a tight, honest CLI. Each verb is something a user actually does.

## Honest answer to the headline question

Is `finetype generate` more useful than faker libraries? **Yes, for
exactly one job: producing data that passes FineType's own validation.**
For everything else, faker libraries are better. That one job is real
but narrow, and the MCP tool already covers the per-value variant.

Hide the CLI subcommand. Faker is the right tool for general use; MCP
`generate` is the right tool for FineType-aligned sampling.

## Not action yet

Observation memo. Composes with the other "hide" memos as one
coherent change. Promote together.
