# Memo: MCP server surface inherits CLI confusions

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a re-audit after v0.7.0
**Tags:** mcp, cli, surface

## The MCP surface today

```
| Tool      | Mirrors                  | What it does                                   |
|-----------|--------------------------|------------------------------------------------|
| infer     | `finetype infer`         | classify values                                |
| profile   | `finetype profile`       | profile a CSV                                  |
| ddl       | `finetype load` (CTAS)   | emit CREATE TABLE SQL                          |
| taxonomy  | `finetype taxonomy`      | search/filter taxonomy                         |
| schema    | `finetype schema`        | type-mode (key) + table-mode (file)            |
| validate  | `finetype validate`      | schema-driven CSV validation                   |
| generate  | `finetype generate`      | sample data per type                           |
```

Eight tools. Implemented in
`crates/finetype-mcp/src/tools/{ddl,generate,infer,profile,schema,taxonomy,validate}.rs`.

The MCP surface was designed to mirror the CLI verbs as closely as
possible. That was the right call when the CLI was the contract.
After the v0.7.0 polish (the eight CLI memos drafted today), the CLI
contract changes — and the MCP surface needs to follow, but not
necessarily 1:1.

## What the v0.7.0 CLI memos imply for MCP

Walk the CLI memos and ask "does the MCP equivalent need the same
treatment?":

```
| CLI memo                    | CLI change           | MCP equivalent     | MCP change?                    |
|-----------------------------|----------------------|--------------------|--------------------------------|
| schema --file naming        | rename --file → --taxonomy | schema tool params | mirror the rename             |
| schema export verbosity     | drop derivable fields| schema tool output | same drop                      |
| schema/profile overlap      | fold table-mode schema → profile -o json-schema | schema + profile  | fold same way                  |
| validate required flags     | --db/--table optional| validate tool params | already optional in MCP — verify |
| --model flag hide           | hide                 | n/a (MCP runs server-side, no flag) | n/a                |
| --sharp-only remove         | remove               | n/a                | n/a                            |
| --model-type hide           | hide                 | n/a                | n/a                            |
| check internal              | hide                 | not exposed in MCP | n/a (already correct)          |
| generate vs faker           | hide CLI subcommand  | MCP generate tool — KEEP | per-value MCP is its right shape |
| load folds into validate    | remove load          | ddl tool           | does this still have a job?    |
```

Two of these need genuine MCP work; the rest are mechanical mirrors or
no-ops.

## The two real questions

### 1. What happens to the `ddl` MCP tool?

Today it produces the same CSV → CTAS SQL that `finetype load` does.
After load folds into validate (memo
`2026-04-27-load-folds-into-validate`), the CLI's `load` is gone. Two
honest paths for MCP:

**A. Drop `ddl` from MCP.** It was a convenience; the equivalent
agent flow becomes "call `validate` with --db/--table to materialise a
typed table." The agent doesn't read the SQL — DuckDB executes it.
The output is the typed table. SQL-as-a-string was always the wrong
return shape for agents anyway.

**B. Keep `ddl` as a SQL-emitter for agents that want to see the
plan.** Some agents reason about SQL before executing it (e.g.,
"explain what this would do," "modify this CTAS"). The MCP tool stays
as a planning verb even if the CLI subcommand is gone.

Recommendation: **A**. Agents don't reason about SQL — they execute
it. If an agent wants to see what FineType would generate, the
`profile` tool already returns enough information (per-column types
and transforms) for it to construct its own CTAS.

### 2. Does MCP `schema` have the same dual-mode confusion?

Yes. `crates/finetype-mcp/src/tools/schema.rs` accepts `key` (type-
mode) OR `path`/`data` (table-mode), same as the CLI. The CLI memo
`2026-04-27-schema-profile-overlap` proposed folding table-mode into
`profile -o json-schema`. The MCP equivalent should follow:

- `schema` tool: type-mode only — accepts `key` (or glob).
- `profile` tool: gains an `output_format: "json-schema"` parameter.

The change is small (each tool already exists; the dispatch logic
moves between them) and keeps MCP and CLI conceptually aligned.

## What MCP gets right that CLI doesn't

```
| Surface     | CLI                      | MCP                    |
|-------------|--------------------------|------------------------|
| --model     | exposed flag             | not exposed            |
| --sharp-only| exposed flag (no-op)     | not exposed            |
| --model-type| exposed flag (3 unreachable) | not exposed         |
| check       | exposed subcommand       | not exposed            |
| train/eval  | exposed (hidden)         | not exposed            |
```

The MCP server already pruned the maintainer-internal surface that the
CLI still carries. That's because the MCP server was designed for
agents from the start; the CLI was designed for humans and grew
maintainer affordances over time. **The MCP server is what the CLI
should look like after the v0.7.0 polish.**

Useful framing: when in doubt about whether a CLI flag belongs in the
public surface, ask "is it on the MCP server?" If yes, public. If no,
hide it.

## Sequencing

This memo doesn't propose immediate work. The MCP audit only makes
sense after the CLI has settled — otherwise we'd refactor MCP twice.
Order of operations:

1. v0.7.0 CLI polish ships (the eight CLI memos consolidated into one
   spec).
2. MCP audit follows in v0.7.1 — mirror the schema/profile fold, drop
   `ddl`, mirror the schema-export verbosity reduction.
3. Document the "MCP is the public surface; CLI is MCP plus
   maintainer affordances" invariant in `crates/finetype-mcp/CLAUDE.md`
   (or the project CLAUDE.md MCP section).

Doing it in this order means MCP changes are mechanical — port the
decision from CLI, not re-make it.

## Resources, not just tools

MCP also exposes resources:

```
finetype://taxonomy
finetype://taxonomy/{domain}
finetype://taxonomy/{d}.{c}.{t}
```

These are the MCP equivalent of `finetype taxonomy`. They look right
and don't need attention. Worth noting that taxonomy access via
*resource* is more idiomatic for agents than via a tool call, and the
CLI doesn't have an equivalent affordance — agents get a richer
surface than human users for taxonomy browsing. Not a problem; just
worth noting that MCP and CLI aren't always 1:1.

## Not action yet

Observation memo. Defer the MCP audit until after the v0.7.0 CLI
polish lands. Then re-read this memo against the actual shipped CLI
and write a spec for the MCP follow-up. Estimated work post-polish:
~3 hours for the schema/profile fold + ddl removal + verbosity drop +
docs.
