# Memo: `finetype check` is a maintainer/CI tool, not a user command

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, ci, maintainer-tools

## What the command does

```
finetype check
    Validate generator ↔ taxonomy alignment

Options:
  -t, --taxonomy <TAXONOMY>  [default: labels]
  -s, --samples <SAMPLES>    Number of samples to generate per definition [default: 50]
      --seed <SEED>          Random seed for reproducibility [default: 42]
  -p, --priority <PRIORITY>  Minimum release priority to check (0 = all)
  -v, --verbose              Show verbose failure details
```

For every taxonomy entry in `labels/*.yaml`, it generates N synthetic
samples via the type's generator and asserts each sample passes the
type's `validation` block. The promise: **if you author a new type, its
generator and validator must agree.**

## Who actually uses it

```
| Caller                        | Why                                          |
|-------------------------------|----------------------------------------------|
| `make ci`                     | CI gate — taxonomy editor protection         |
| `cargo run -- check`          | Maintainer pre-commit                        |
| Pre-promotion sweep scripts   | Confirms taxonomy is internally consistent   |
| External users                | …never                                       |
```

A user who installs FineType via Homebrew has no `labels/` directory —
the taxonomy is embedded into the binary. Running `finetype check`
against the default `labels` path **fails immediately** with "taxonomy
not found." Even if a user supplied `--taxonomy <path>`, the result
("236/240 generators agree with their validators") is a maintainer
metric, not actionable for the user.

## Three options

**A. Hide it (`#[command(hide = true)]`).** Same treatment as `train`
and `eval` already get (`main.rs:121, 482, 382`). One-line change.
Stays available for internal scripts; disappears from `--help`.
Zero churn for CI.

**B. Move to a separate maintainer binary.** Repurpose
`finetype-build-tools` (which already exists as a workspace crate) to
host check, eval-gittables, eval, and any other dev-loop subcommands.
The user binary `finetype` ships only user-facing commands. CI invokes
`cargo run -p finetype-build-tools -- check`.

**C. Keep public.** Document it as "taxonomy author tool." Lowest
churn but doesn't solve the underlying clutter.

Recommendation: **A** as a quick fix; **B** as the proper structural
move once we've collected enough hidden subcommands to justify a
separate binary.

The hidden subcommands today:

```
| Command         | Status today                       |
|-----------------|------------------------------------|
| train           | #[command(hide = true)]            |
| eval            | #[command(hide = true)]            |
| eval-gittables  | #[command(hide = true)]            |
| check           | public — proposing hide            |
| generate        | public — proposing hide (separate memo) |
```

That's five maintainer commands sharing space with seven user commands.
Option B becomes natural once five-of-twelve are dev-loop tools.

## Why option A first

- Zero-risk: `make ci` and other internal callers use `cargo run --
  check` (binary-internal), which is unaffected by `hide`.
- Releases the slot in `--help` immediately.
- Defers the binary-split decision until we know how the other memos
  shake out — if they collapse the surface enough, option B may not
  be needed.

## Composition with other memos

This is one of three "hide it from end users" memos so far:

```
| Memo                         | Hide candidate              |
|------------------------------|-----------------------------|
| cli-model-flag               | --model flag                |
| cli-sharp-only-flag          | --sharp-only flag           |
| cli-model-type-flag          | --model-type flag           |
| cli-check-internal (this)    | check subcommand            |
```

The flags-to-hide are dev-loop concerns leaking into user commands;
the subcommands-to-hide are dev-loop *commands* in their own right.
Same diagnosis, different surface.

## Not action yet

Observation memo. Trivial to action when ready: add `#[command(hide =
true)]` above `Commands::Check` in `main.rs:274`. CI continues to work
unchanged.
