# Memo: `--sharp-only` is a no-op on the default pipeline

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, pipeline, legacy

## What the flag claims to do

```
--sharp-only
    Disable Sense classifier (use Sharpen-only pipeline with header hints)
```

Exposed on **three user-facing subcommands**: `infer` (`main.rs:90`),
`load` (`main.rs:255`), `profile` (`main.rs:365`).

## What the flag actually does today

It does nothing on the default pipeline.

The wiring at every call site is the same shape (`main.rs:1482, 1772,
3213, 4199`):

```rust
// Wire up Sense classifier (Sense → Sharpen pipeline)
if !sharp_only && !column_classifier.has_multi_branch() {
    wire_sense(&mut column_classifier);
    wire_sibling_context(&mut column_classifier);
}
```

Two conditions, AND-ed:

1. `!sharp_only` — user didn't pass the flag.
2. `!column_classifier.has_multi_branch()` — we're **not** running the
   multi-branch model.

Multi-branch has been the default and shipped pipeline since decision
0041 (sherlock-v4-sibling onward). `models/default` always points at a
multi-branch model — currently `sherlock-v19-relu-s42`. So in every
default invocation, `has_multi_branch()` returns true, the gate fails,
and `wire_sense` is never called regardless of `--sharp-only`.

`--sharp-only` only has any observable effect when the user **also**
passes `--model-type char-cnn` (or `tiered`) AND points `--model` at
a non-multi-branch checkpoint. Both of those are themselves dev-loop
flags the previous memo flagged for hiding.

The one place `sharp_only` shows up in user-visible output regardless
of pipeline: a comment string in `cmd_load`'s SQL header (`main.rs:3330-3334`):

```rust
let pipeline = if sharp_only {
    "Sharpen-only"
} else {
    "Sense→Sharpen"
};
```

That printout is misleading — it says "Sense→Sharpen" for the default
multi-branch pipeline, which is neither Sense nor strictly Sharpen-led.
Cosmetic bug, separate from the flag question.

## Provenance

The flag dates to the Sense→Sharpen era (CHANGELOG NNFT-173). Then
useful: it let users / maintainers bypass the Sense classifier when
Sense model files were missing or when running the tiered-CharCNN
fallback. `docs/ARCHITECTURE.md:82` and `docs/SENSE_AND_SHARPEN_PIPELINE.md`
still document it as a supported mode for "maximum throughput" or
"Sense-absent" cases.

After decision 0041, the multi-branch model **replaced** Sense (single
forward pass per column does what Sense + CharCNN used to do). The
flag's gate condition (`!has_multi_branch()`) was added as the safe
landing during transition — but the transition is complete. No shipped
configuration ever evaluates `!has_multi_branch()` as true.

## What about scripts / docs?

No internal script uses `--sharp-only`. Greps clean across `scripts/`,
`eval/`, and CI workflow files. Documentation references:

```
| File                                       | Reference                                  |
|--------------------------------------------|--------------------------------------------|
| CHANGELOG.md:376                           | "opt into legacy tiered-only pipeline"     |
| docs/ARCHITECTURE.md:82                    | "available via --sharp-only"               |
| docs/SENSE_AND_SHARPEN_PIPELINE.md:132,211 | usage example, "maximum throughput" claim  |
| .claude/skills/finetype-cli/SKILL.md       | flag table on infer/profile/load           |
```

The "maximum throughput" framing in
`SENSE_AND_SHARPEN_PIPELINE.md:211` is no longer accurate — multi-branch
is **faster** than Sense+CharCNN (single forward pass per column
replaces ~100 CharCNN value-level inferences, per `CLAUDE.md`). The doc
is stale.

## Three options

**A. Remove `--sharp-only` entirely.** Flag, all three CLI sites,
SKILL.md tables, the misleading "Sense→Sharpen" header in `cmd_load`,
and the stale doc paragraphs. Internal callers that genuinely need to
test the legacy CharCNN path can do so directly via the `--model-type`
+ `--model` combination (also slated for hiding/removal).

**B. Hide it (`#[arg(long, hide = true)]`).** Stays for the rare
maintainer who wants to ablate the Sense path on a CharCNN checkpoint.
Same smell as the `--model` hide question — undeclared dev flag.

**C. Keep it.** Lowest churn, but accumulates dead surface.

Recommendation when we get to deciding: **A**. The flag is a no-op on
every shipped configuration. It documents itself as "Disable Sense
classifier" in `--help`, but Sense was retired by decision 0041 — the
flag describes a pipeline that no longer exists in the default. Worse
than dead code: actively misleading help text.

If we keep the legacy CharCNN path at all (decision pending in the
`--model-type` memo), the right place for "skip Sense wiring" is a
hidden flag on a hidden subcommand or an env var — not surfaced to
end users.

## Same shape as the four prior memos

This is the fifth CLI ergonomics observation from today. The pattern
holds: surfaces designed during architectural transitions outlived the
transition.

```
| Memo                            | Underlying pattern                              |
|---------------------------------|-------------------------------------------------|
| schema-cli-flag-collision       | --file means taxonomy dir, not input file       |
| schema-export-verbosity         | 7 x-finetype-* fields, only 2 non-derivable     |
| validate-required-flags         | --db/--table mandatory even for read-only       |
| cli-model-flag                  | --model exposes dev-loop concern to end users   |
| cli-sharp-only-flag (this memo) | flag is a no-op on the shipped pipeline         |
```

The unifying theme: **the CLI surface accumulated debt during the
Sense→multi-branch transition and the validate sidecar→DuckDB
transition.** Both transitions are complete; the flags they introduced
to ease the bridge are still there.

## Not action yet

Observation memo. Stacks with the four prior memos as v0.7.0 CLI
polish material. Promote together when ready.
