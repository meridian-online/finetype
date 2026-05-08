# Memo: `--model-type` exposes legacy architectures to users

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, pipeline, legacy

## What the flag claims

```
--model-type <MODEL_TYPE>
    Model type (char-cnn, tiered, transformer)
    [default: multi-branch]
    [possible values: transformer, char-cnn, tiered, multi-branch]
```

Exposed on **three user-facing subcommands**: `infer` (`main.rs:64`),
`load` (`main.rs:251`), `profile` (`main.rs:361`). Already hidden on
`train` (`#[command(hide = true)]`, line 121) and `eval` (`main.rs:498`).

## What the values do

```
| Value         | Loader                       | Status                 |
|---------------|------------------------------|------------------------|
| multi-branch  | load_multi_branch_classifier | DEFAULT, shipped       |
| char-cnn      | load_char_classifier         | legacy                 |
| tiered        | load_tiered_classifier       | legacy (34 CharCNNs)   |
| transformer   | Classifier::load             | legacy spike           |
```

Per decision 0041, **multi-branch is the only shipped pipeline** since
sherlock-v4-sibling. The other three are pre-transition architectures
kept compiling for internal regression testing during promotion.

## Why three of four values are unusable for end users

`models/default` ships a multi-branch checkpoint
(`sherlock-v19-relu-s42`). HuggingFace hosts only multi-branch and
auxiliary models (model2vec, sibling-context, entity-classifier). To
use `--model-type char-cnn | tiered | transformer`, a user needs:

1. A CharCNN / tiered / transformer checkpoint (none on HF)
2. To train one themselves with `finetype train` (hidden subcommand)
3. To pass `--model <local-checkpoint>` (also slated for hiding per
   the prior memo)

End users have access to **one** of the four values. The other three
are advertised in `--help` but unreachable.

## The flag's doc comment is also stale

```rust
/// Model type (transformer, char_cnn)
#[arg(long, default_value = "multi-branch")]
model_type: ModelType,
```

(`main.rs:147-149` and copies on infer/load/profile/eval). The doc
comment lists 2 of 4 values and omits the actual default. Clap's
auto-generated `--help` is correct because it reads from the value
enum, but anyone reading the source sees a misleading comment.

## Composition with `--sharp-only`

The prior memo (`cli-sharp-only-flag`) noted `--sharp-only` is a no-op
on the multi-branch pipeline because of the gate:

```rust
if !sharp_only && !column_classifier.has_multi_branch() {
    wire_sense(&mut column_classifier);
}
```

`--sharp-only` only has effect when **also** `--model-type !=
multi-branch`. The two flags are co-dependent dev-loop machinery.
Hiding one without the other leaves a half-broken composition.

## Three options

**A. Remove three legacy values; keep `--model-type` as a single-value
flag with `multi-branch` only — then remove the flag entirely.**
Aggressive but coherent: the legacy code paths come out of the public
binary. They can stay in `finetype-model` behind a `legacy-classifiers`
feature flag for internal regression testing. Eliminates the
co-dependency with `--sharp-only` (also removed). Smallest user surface.

**B. Hide `--model-type` (`#[arg(long, hide = true)]`).** Stays
functional for the rare maintainer who wants to ablate against a
CharCNN/tiered/transformer checkpoint. Same smell as the prior hides.

**C. Keep public; document the four values honestly.** Update help
text to mark three as "legacy, requires self-trained checkpoint."
Adds clarity but doesn't reduce surface — and most users don't read
caveats in `--help`.

Recommendation when we get to deciding: **A**, in two stages.

1. **Stage 1 (low risk):** Hide `--model-type` and `--sharp-only`
   together. Fix the stale doc comment. Confirm via grep that no
   internal script uses either (already confirmed for `--sharp-only`;
   grep for `--model-type` next).
2. **Stage 2 (after a release cycle):** If nothing depends on the
   hidden flags, gate `ModelType::CharCnn | Tiered | Transformer`
   behind a `legacy-classifiers` cargo feature. The default build
   ships only `MultiBranch`. The flag becomes unconstructible in
   release builds — cleanly removes ~3 loader functions plus their
   error paths from the user-facing binary.

This mirrors how the prior `--model` memo handled the env var split:
keep dev-loop capability for maintainers, narrow the public surface
to what users can actually use.

## The five-memo pattern

Five CLI ergonomics observations from today, all the same shape:

```
| Memo                            | Underlying pattern                              |
|---------------------------------|-------------------------------------------------|
| schema-cli-flag-collision       | --file means taxonomy dir, not input file       |
| schema-export-verbosity         | 7 x-finetype-* fields, only 2 non-derivable     |
| validate-required-flags         | --db/--table mandatory even for read-only       |
| cli-model-flag                  | --model exposes dev-loop concern to end users   |
| cli-sharp-only-flag             | flag is a no-op on the shipped pipeline         |
| cli-model-type-flag (this memo) | 3 of 4 values are unreachable for end users     |
```

Three legacy-transition flags — `--model`, `--sharp-only`, `--model-type`
— form a tightly-coupled dev-loop subsystem. They should hide / remove
together, in that order:

1. `--sharp-only` — fully dead on shipped pipeline (no-op gate).
2. `--model-type` — only one value is reachable for end users.
3. `--model` — load-bearing for eval scripts; replace with `FINETYPE_MODEL`
   env var.

The schema/validate flag memos are independent of this group.

## Not action yet

Observation memo. Ready to graduate to a card or spec when ready.
Strongly recommend bundling with `cli-sharp-only-flag` and
`cli-model-flag` as one coherent "retire dev-loop CLI surface" change
— they share state, doc footprint, and rationale.
