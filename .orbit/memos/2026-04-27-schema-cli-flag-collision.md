# Memo: `finetype schema` flag collision

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Superseded by `2026-04-27-schema-profile-overlap.md` (revised) — schema verb folds entirely; rename is moot
**Tags:** cli, ux, schema, superseded

> **Note:** This memo proposed renaming `schema`'s `-f, --file` flag
> to `--taxonomy` to remove the trap. The revised
> `schema-profile-overlap` memo folds the `schema` verb away entirely
> — type-mode → `taxonomy KEY -o json-schema`, table-mode →
> `profile -f file.csv -o json-schema`. The flag goes away with the
> verb, so the rename in this memo no longer applies. Kept here for
> the trail. The same observation about other subcommands' `-f` flags
> still applies — review whether `generate`, `taxonomy`, and `check`
> share the same `-f, --file` taxonomy-directory pattern when this
> spec lands.

## What happened

```
$ finetype schema -f eval/datasets/csv/ecommerce_orders.csv
error: the following required arguments were not provided:
  <TYPE_KEY>

$ finetype schema --file eval/datasets/csv/ecommerce_orders.csv
error: the following required arguments were not provided:
  <TYPE_KEY>

$ finetype schema eval/datasets/csv/ecommerce_orders.csv
Loading model from "models/default"
...
Wrote table schema to "eval/datasets/csv/ecommerce_orders.schema.json"
```

The first two invocations failed; the third worked. From a user's mental
model that's surprising — `--file <some.csv>` reads as "the file I want to
operate on."

## Why

`finetype schema` (definition at `crates/finetype-cli/src/main.rs:193-215`)
has two parameters whose names collide with each other in the user's head:

```
| Parameter         | Kind        | Default      | Actual purpose                                |
|-------------------|-------------|--------------|-----------------------------------------------|
| <TYPE_KEY>        | positional  | required     | Type key OR glob OR CSV path (overloaded)     |
| -f, --file <FILE> | option      | "labels"     | Taxonomy directory to load definitions from   |
```

So:

- `-f, --file` is **not** the input file. It points at the taxonomy
  directory (default `labels/`). The flag name is misleading — it sounds
  like "input file," but it's really `--taxonomy-dir`.
- The positional `<TYPE_KEY>` is overloaded: the dispatcher at `main.rs:642-667`
  sniffs the string — if it looks like an existing file path with a known
  extension (`.csv`, `.tsv`, `.parquet`), it routes to table mode;
  otherwise it routes to type-key mode.

The first two invocations failed because:

1. clap saw `-f my.csv` as setting `--file` (taxonomy dir) to a CSV path,
2. clap then required the positional `<TYPE_KEY>` which wasn't given,
3. so it errored on the missing positional before any of our code ran.

The third invocation worked because the CSV path **was** the positional,
`-f` kept its default of `labels/`, and the path-sniffer routed to table
mode.

## Why this is a trap, not just a quirk

- The flag name is the most natural English for "the file I want to
  operate on." Users with no taxonomy mental model will reach for it first.
- The `-h` output describes `--file` as "Taxonomy file or directory" —
  technically accurate but not the framing a new user has.
- Type-key mode and table mode serve genuinely different intents
  (type-level schema vs table-level schema) but share a single positional,
  so the help text can't easily clarify which is which.
- Several flags in the help (`--stats`, `--stdout`, `--enum-threshold`,
  `--model`) are tagged "table mode only" — implicit acknowledgement that
  this is really two subcommands wearing one hat.

## Options (not deciding here, just naming them)

**A. Rename `-f, --file` to `--taxonomy` (or `--labels`).** Cheapest fix.
Keep `--file` as a hidden deprecated alias for one release, then drop.
Removes the name collision; doesn't touch the overloaded positional.

**B. Add a dedicated `-i, --input <FILE>` flag for table mode.** Lets
users write `finetype schema --input my.csv` without the positional. The
positional becomes optional in table mode. Slightly chattier surface but
matches user mental model.

**C. Split into two subcommands.** `finetype schema type <KEY>` and
`finetype schema table <FILE>`. Most honest but breaks every existing
script using `finetype schema my.csv` or `finetype schema email`.

**D. Better error message when path-shaped value lands in `--file`.**
Detect `--file <path-with-csv-ext>` and emit "did you mean `finetype
schema <FILE>`?" before clap's missing-positional error fires.

Recommendation when we get to deciding: **A + D** — rename the flag,
keep `--file` as a deprecated alias for one release, add a hint when
the deprecated alias is used with a path-shaped value. Cheapest path
that closes the trap without breaking existing CLIs.

The same `-f, --file` taxonomy-directory pattern shows up in other
subcommands (`generate`, `taxonomy`, `check`) — any rename should sweep
all of them in one PR for consistency.

## Not action yet

This is an observation memo, not a card or spec. Promote to a card
when we decide it's worth the breaking change.
