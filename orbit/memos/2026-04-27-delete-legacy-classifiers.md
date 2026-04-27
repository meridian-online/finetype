# Memo: delete legacy classifiers — `--model-type` is the symptom

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a follow-up spec
**Tags:** cli, model-type, legacy, refactor, deferred

## Why this is its own memo

The visibility-cleanup spec (v0.6.19) walked every "hide" candidate
through a *do-we-actually-use-this?* gate. Most items survived as hide
or got promoted to outright removal. One item didn't fit either
bucket: **`--model-type`**.

The flag itself is small. The entanglement behind it isn't. This
memo captures what the cleanup would actually require, so it can
ship as a separate spec when we're ready.

## What `--model-type` exposes today

```rust
// crates/finetype-cli/src/main.rs:520
enum ModelType {
    Sense,         // legacy: per-value sense classifier
    MultiBranch,   // production: 5-branch model + Sharpen
    Sharpen,       // legacy: sense → CharCNN tier classifier
    Flat,          // legacy: flat 239-class CharCNN
}
```

The flag plumbs `model_type: ModelType` through 5 command surfaces
(infer, profile, schema, load, eval) and 12+ call sites. The actual
dispatch happens at one point:

```rust
// crates/finetype-cli/src/main.rs:1449
let mut column_classifier = if matches!(model_type, ModelType::MultiBranch) {
    // production path
} else {
    let classifier: Box<dyn ValueClassifier> = match model_type {
        ModelType::Sense    => /* SenseClassifier */,
        ModelType::Sharpen  => /* TieredClassifier with Sharpen wrapper */,
        ModelType::Flat     => /* CharClassifier (flat 239-class) */,
        ModelType::MultiBranch => unreachable!(),
    };
    // legacy path
};
```

In production, only `MultiBranch` is reachable. The taxonomy,
training pipeline, default model symlink, MCP server, DuckDB
extension, and eval scripts all assume multi-branch. Decision 0041
(2026-03-25) made that explicit:

> **Multi-branch as Sense replacement.** The multi-branch model
> (sherlock-v4-sibling) is the default classifier. It replaces both
> Sense and CharCNN — single forward pass per column.

The other three values — `Sense`, `Sharpen`, `Flat` — are dead
branches kept alive by the flag's existence.

## What's behind the dead branches

```
| Branch     | Backing classifier        | Crate location                                   |
|------------|---------------------------|---------------------------------------------------|
| Sense      | SenseClassifier           | crates/finetype-model/src/sense.rs                 |
| Sharpen    | TieredClassifier (CharCNN)| crates/finetype-model/src/tiered.rs                |
| Flat       | CharClassifier            | crates/finetype-model/src/inference.rs             |
```

Each is a complete classifier implementation:

- **SenseClassifier** — value-level sense pre-classifier (decision
  0041 retired this as the default; the multi-branch's "sense"
  branch replaces it functionally).
- **TieredClassifier** — 34 specialised CharCNN models in a Tier 0 →
  Tier 1 → Tier 2 routing graph. Lives at `models/tiered-v2/` with
  its own `tier_graph.json`. Production never loads it; sweep
  scripts no longer target it.
- **CharClassifier** — flat 239-class CharCNN. The original Sherlock-
  era model.

All three share `crates/finetype-model/src/inference.rs`'s
`ValueClassifier` trait. They're not dead imports — they compile,
test, and are reachable via the flag. They're just never called.

## Adjacent surfaces that move with them

A clean delete touches more than the model crate:

```
| Surface                          | What's coupled                                 |
|----------------------------------|------------------------------------------------|
| crates/finetype-model/src/sense.rs            | SenseClassifier + ~600 LOC tests   |
| crates/finetype-model/src/tiered.rs           | TieredClassifier + tier_graph load |
| crates/finetype-model/src/inference.rs        | CharClassifier (flat path)         |
| crates/finetype-model/src/char_training.rs    | flat-model training entrypoint     |
| crates/finetype-model/src/tiered_training.rs  | tiered-model training entrypoint   |
| crates/finetype-cli/src/main.rs               | ModelType enum + 12 call sites     |
| crates/finetype-cli/src/main.rs               | cmd_train branches for legacy types |
| models/tiered-v2/                             | shipped artefacts (~360 MB)         |
| models/char-cnn-v*/                           | shipped artefacts (multiple)        |
| eval/profile_eval.sh                          | confirm no --model-type usage today  |
| docs/SENSE_AND_SHARPEN_PIPELINE.md            | already stale (per docs memo)        |
| docs/ARCHITECTURE.md                          | references Sense classifier          |
| .claude/skills/finetype-cli/SKILL.md          | --model-type in flag table           |
| CHANGELOG.md                                  | retirement note                      |
```

Estimate: **~2,500–3,500 LOC delete** across two crates, plus four
shipped model directories that no longer have a classifier to load
them. The change is mostly mechanical (delete-and-fix-callers) but
the surface is wide enough to deserve its own spec.

## Why visibility-cleanup is the wrong host

The visibility-cleanup spec ships a shrunken public surface in
v0.6.19. Its scope:

- Hide check, generate
- Remove eval-gittables subcommand, --sharp-only flag, --model flag
- Schema export verbosity reduction
- Validate flag relaxation

Each item in that scope is a small, mechanical change with no
dependency on the model crate. Adding "delete legacy classifiers"
into the same spec would:

- Triple the diff size and review surface.
- Require a model-crate refactor that has its own design questions
  (do we keep the `ValueClassifier` trait? how does the multi-branch
  fit into the simpler shape?).
- Mix two release themes — "tighten the user-facing CLI" and "remove
  unused model code paths" — that ship cleanly when separated.

Defer is the honest call.

## What the follow-up spec looks like

Suggested name:
`orbit/specs/<date>-delete-legacy-classifiers/`

Scope:

```
| Phase | Work                                                           |
|-------|----------------------------------------------------------------|
| 1     | Remove --model-type from clap; ModelType enum becomes single   |
|       | variant (or disappears entirely); call sites collapse to       |
|       | multi-branch path                                              |
| 2     | Delete SenseClassifier, TieredClassifier, CharClassifier from  |
|       | finetype-model; ValueClassifier trait becomes unnecessary if   |
|       | only one impl remains                                          |
| 3     | Delete char_training.rs and tiered_training.rs (training flows |
|       | for retired classifiers); finetype-train's pure Rust path stays |
| 4     | Delete shipped model directories from models/ that aren't       |
|       | referenced post-cleanup                                         |
| 5     | Update docs (SENSE_AND_SHARPEN_PIPELINE.md, ARCHITECTURE.md,    |
|       | SKILL.md); CHANGELOG entry                                      |
```

Acceptance criteria sketch:

- `finetype --help` and `--help-hidden` show no `--model-type` flag.
- `cargo build` compiles with all legacy classifier files removed.
- All tests pass (`cargo test` + golden integration).
- `make ci` passes — taxonomy alignment unchanged, CharCNN-era tests
  removed cleanly.
- Repo size shrinks by the deleted models directory.

## Composition with other memos

Pairs naturally with:

- **`2026-04-27-stale-documentation`** — `docs/SENSE_AND_SHARPEN_PIPELINE.md`
  is named in both memos. Deleting legacy classifiers is the right
  moment to delete the legacy-classifier doc rather than rewriting it.
- **`2026-04-27-repo-cleanliness`** — the `models/` directory carries
  ~360 MB of `models/tiered-v2/` plus older char-cnn-v* snapshots.
  Cleaning the codebase makes those directories cleanly removable.
- **`2026-04-27-sweep-script-graveyard`** — most archived overnight
  sweeps target legacy classifier types. After this spec, the
  archive's only remaining value is historical; before it, the
  archive at least describes a code path that still compiles.

## Sequencing

```
v0.6.19 (visibility-cleanup)
    ↓
v0.6.20 (cli-pipeline-reshape: load → validate, schema → taxonomy + profile)
    ↓
v0.6.21 or v0.7.0 (delete-legacy-classifiers: this memo)
```

The reshape spec doesn't depend on legacy classifier deletion, but
the delete-legacy spec benefits from the reshape having shipped —
fewer call sites carry the `model_type` parameter once `load` is
folded into `validate`.

## Not action yet

Observation memo. Defers the substantive refactor to its own spec,
keeping the visibility-cleanup spec tight and shippable. ~2 hours
to draft the follow-up spec when we're ready; ~1–2 days to
implement.
