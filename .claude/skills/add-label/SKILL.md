---
description: >-
  Mint a new taxonomy leaf (label/type) in FineType. Adding a definition is FOUR coordinated moves, not one — the YAML definition, a sample generator arm, `cargo run -- check`, and the precise_audit count-bound + fixture. Miss any and `check` or `cargo test` fails. Minting alone is INERT (nothing predicts/emits a new leaf); a value-determinable type also needs a deterministic Sharpen recovery guard + the gate battery to actually appear in output.
when_to_use: User says "add a label", "add a type", "mint a leaf", "new taxonomy type", or names a `domain.category.type` to introduce. Also when a mining/audit finding proposes a new leaf and the author has approved it. Adding a taxonomy type is author-approved — confirm the type + its placement before minting.
argument-hint: "<domain.category.type>"
arguments: label_key
allowed-tools: Bash, Read, Edit, Write
---

# /add-label

Mint a new taxonomy leaf. The taxonomy is the contract the whole engine reads, and adding a leaf
touches four files that MUST stay in lockstep. This skill is the checklist so none is missed — both
omissions below have broken CI in practice.

**Author-approval gate:** adding a type is an author decision (every taxonomy addition in CLAUDE.md is
author-approved). Confirm the *type* and its *placement* before minting — placement is load-bearing
(a bare filename is `technology.filesystem.filename`, a sibling of `windows_path`; it is NOT
`representation.file.*`, which holds file *properties* — extension/size/mime_type — not the file entity).

## The four coordinated moves

### 1. Definition — `labels/definitions_<domain>.yaml`

Add the leaf under the right `domain.category.type` key, matching the fields of a nearby sibling
(copy `windows_path`/`message_id` as a template). Required: `title`, `description`, `designation`,
`locales`, `broad_type`, `frictionless`, `transform`, `validation`, `tier`, `release_priority`,
`aliases`, `samples`, `notes`. `samples` MUST pass the leaf's own `validation` (the check enforces it).

**Precision Principle for the validator.** If the value *shape* is genuinely precise (drive-letter path,
`<a@b>` message-id), write a tight regex/enum. If the shape is NOT precise — `word.word`, a short
uppercase token — do NOT write a loose regex and call it validation ("a validation that confirms 90% of
random input is not a validation"). Instead use the **ticker pattern**: a shape validator + the real
precision carried by the recovery guard's detector (a curated set / checksum / structural check). Say so
in `notes` ("the SET/structure is the substance, per the Precision Principle").

### 2. Generator — `crates/finetype-core/src/generator/<domain>.rs`

Add a match arm under `match (category, type_name)`:

```rust
("<category>", "<type>") => {
    // build a sample that PASSES this leaf's own validator
    Ok(format!("..."))
}
```

**Skip this and `cargo run -- check` fails with "missing generators".** (This bit the filename add.)

### 3. Check — `cargo run -- check`

Must print `✅ ALL CHECKS PASSED`: `Generators found: N/N`, `Samples: 100.0%`, every domain ✅. This is
the alignment gate for moves 1–2.

### 4. precise_audit count-bound + fixture

`crates/finetype-core/tests/precise_audit.rs` hard-asserts the taxonomy leaf count
(`(lo..=hi).contains(&n)` around line 96) — **bump the bound and the message** to the new total. Then:

```
cargo test -p finetype-core --test precise_audit
```

regenerates the tracked fixture `tests/fixtures/precise_audit.tsv` (workspace root, NOT under the crate).
**Commit its diff.** Skip the bound bump and `cargo test` fails "expected N ± 1 taxonomy rows".

## Verify all four

```
cargo run -- check                                   # move 3
cargo test -p finetype-core -p finetype-model        # moves 1,2,4 + everything
cargo fmt --all --check && cargo clippy -- -D warnings
```

## Then: make it actually emit (usually a separate ship)

Minting a leaf is **inert** — the shipped model is 244-dim and will never predict a new leaf, and no
rule recovers it yet. To make columns of this type appear in `profile` output:

- **Value-determinable type (decision 0096 — the common case):** add a deterministic Sharpen **recovery
  guard** in `crates/finetype-model/src/column/guards.rs` (a `*_recovery` fn wired into
  `apply_post_sharpen_guards`), with the substance detector usually in
  `crates/finetype-core/src/structure.rs`. Then run the **gate battery**: corpus-honest fast gate (blocking)
  + gold + representative + the **mandatory distinct-cardinality spot-check** (gold is ~blind to these
  slices, so the spot-check catches sub-floor FP tails). Templates: `output/{filename,url-override,qualified-name}-recovery/finding.md`.
- **Semantic type:** needs a retrain (rare; prefer a rule per decision 0038/0096).

Gate the recovery guard, don't gate the mint — the mint is just the contract.

## Gotchas (each has bitten)

- **Missing generator** → `check` FAILS. Move 2 is mandatory, not optional.
- **Stale count bound** → `cargo test` FAILS. Bump `precise_audit.rs` + regen the fixture.
- **Fixture location** — `tests/fixtures/precise_audit.tsv` is at the **workspace root**, and IS tracked.
- **New leaf not in `labels/veto_safe.txt`** → its validation veto is ADVISORY-only until
  `scripts/false_veto_sweep.py` measures it. Usually fine; don't hand-add it.
- **CLAUDE.md taxonomy count + version** ("N definitions", per-domain counts) and the changelog are updated
  at RELEASE time (the ship-to-main-then-release-pending pattern), not in the minting commit.
- **Placement** — pick the family by what the value IS (an entity vs a property; a code symbol vs a locator).

Worked example: `technology.filesystem.filename` (commit b93d2a4) — all four moves + the recovery guard,
findings in `output/filename-recovery/finding.md`.
