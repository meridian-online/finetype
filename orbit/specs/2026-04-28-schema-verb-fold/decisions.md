# Decision Pack — `schema` verb fold

**Card:** `orbit/cards/0006-command-line-interface.yaml`
**Rally:** `orbit/specs/2026-04-28-v0619-cli-consolidation-rally/`
**Sibling cards:**

- `orbit/cards/0003-tabular-data-profiling.yaml` (table-mode replacement → `profile -o json-schema`)
- `orbit/cards/0005-schema-driven-data-validation.yaml` (type-mode replacement is consumed here as `taxonomy KEY -o json-schema`)

This card owns the **deprecation/removal mechanics of `finetype schema`** and the **type-mode replacement** (`taxonomy KEY -o json-schema`). The two sibling cards each own one half of the user-facing destination. PR #51 has already shipped the visibility cleanup that this rally is built on — `schema` survives in v0.6.18 with its export verbosity already trimmed (label + pii only).

Six decisions follow.

---

## Decision 1 — Removal cadence

**Title:** Hard removal of `schema` in v0.6.19 vs one-release deprecation

**Context.** The original `schema-profile-overlap` memo (lines 137–141) proposed a deprecation cycle: "Add `-o json-schema` to profile, deprecate `schema <file.csv>` for one release, remove in v0.7.x." The revised "Both modes fold" section (lines 207–214) repeats this — "Deprecate `finetype schema` (both modes) for one release; emit a warning pointing to the replacement verb. Remove `schema` verb entirely in the following release." But the visibility-cleanup spec just shipped (`orbit/specs/2026-04-27-cli-visibility-cleanup/spec.yaml`) committed the project to a hard-removal posture for the v0.6.19 line — quote: "Hard removal posture — no deprecation cycle. Removed flags/subcommands error immediately via clap's unknown-arg/unknown-subcommand handling. Scripts pinning old surfaces fail loudly rather than drift silently." `--model`, `--sharp-only`, and `eval-gittables` all went via that path with no detour.

**Options.**

- **Option A — Hard removal in v0.6.19.** Delete `Commands::Schema`, the `cmd_schema` function (`crates/finetype-cli/src/main.rs:2565-2639`), the `cmd_schema_table` function (`main.rs:2699-`), and the path-sniffer dispatch arm (`main.rs:595-629`). `finetype schema …` errors via clap's unknown-subcommand handler. Both replacements (`taxonomy -o json-schema`, `profile -o json-schema`) ship in the same release, so users always have somewhere to go.
- **Option B — One-release soft deprecation.** Keep the `Schema` variant in clap, add `#[command(hide = true)]` so it disappears from `--help`, change the body of `cmd_schema`/`cmd_schema_table` to print a stderr warning ("`finetype schema` is deprecated — use `finetype taxonomy KEY -o json-schema` / `finetype profile -f FILE -o json-schema`. Removing in v0.6.20.") and then delegate to the replacement code path. Remove in v0.6.20.
- **Option C — Hard error with a hint.** Keep `Commands::Schema` but make `cmd_schema` immediately exit 2 with a tailored message (no taxonomy load, no profile run). Same shape as B but doesn't actually run the replacement, just points at it.

**Trade-offs.**

- **A** matches the precedent set by PR #51 — visibility-cleanup spec line 16: "Scripts pinning old surfaces fail loudly rather than drift silently." It collapses a parallel-maintenance window (no two emitters, no hidden alias, no warning suppression flags). The replacements ship in the same PR, so the migration path is one-line per script. Cost: any external user with a script that calls `finetype schema` gets a clap "unrecognized subcommand" error on first run after upgrade. The error message is generic; users have to read the CHANGELOG to learn the migration. README.md:77, README.md:82, `.claude/skills/finetype-cli/SKILL.md:40-65`, and `.claude/skills/finetype-pipeline/SKILL.md:57,140` all need same-PR migration.
- **B** is what the memos drafted — but they were written before the visibility-cleanup spec settled the hard-removal posture. The warning-emit path keeps both code paths alive for one release (≈600 LOC of duplicated emitter logic), and the warning is suppressible by piping stderr to `/dev/null`, so silent drift is still possible. Honest downside: a deprecation cycle is a contract — we'd have to track "remove in v0.6.20" and actually do it, adding a card to the next rally. The visibility-cleanup spec deliberately did not pay that cost.
- **C** is the worst of both: the verb still appears in the surface (clap parses it), it doesn't actually do anything, and users have to make the same one-line migration as in **A**. The only thing it adds over **A** is a hand-tuned hint string. clap's "unrecognized subcommand" error is already informative enough — the migration is captured in the CHANGELOG.

**Recommendation: A — hard removal in v0.6.19.** This is the same posture the rally is already executing. The two replacement verbs ship in the same release as the removal (sibling cards 0003 and 0005 are explicit dependencies in the rally manifest). No parallel-maintenance window, no future-promise tracking, one CHANGELOG section per release. The CHANGELOG entry should explicitly list the two replacements verbatim so a `grep schema CHANGELOG.md` lands the migrations.

---

## Decision 2 — Shape of `taxonomy KEY -o json-schema` for multi-match input

**Title:** How `taxonomy` emits multiple JSON Schema documents when KEY is a glob

**Context.** Today `finetype schema "datetime.date.*"` (referenced in README.md:77 and `crates/finetype-cli/src/main.rs:2569-2585`) accepts a glob and emits a JSON array of schemas — `cmd_schema` lines 2624-2629: "if schemas.len() == 1 → single object; else → wrap in array." After the fold, `taxonomy KEY -o json-schema` must support the same input shapes (exact key, glob like `"identity.person.*"`, wildcard `"*"`). Today, `taxonomy` already accepts no positional argument and filters via `--domain`/`--category`/`--priority` (`main.rs:172-196`, `cmd_taxonomy` at `main.rs:2400-2488`). It returns a JSON array of definition summaries — never a single object. The output-format set is `Plain | Json | Csv | Markdown | Arrow` (`main.rs:476-483`); we need to add `JsonSchema`.

**Options.**

- **Option A — Always emit a JSON array.** Even single-match input returns `[{ ...schema }]`. Consistent with `taxonomy`'s existing JSON output (always an array). Breaks output-byte parity with `cmd_schema`'s single-result branch — pipelines like `finetype schema email | jq .pattern` become `finetype taxonomy email -o json-schema | jq '.[0].pattern'`.
- **Option B — Single-result returns a single object; multi-result returns an array.** Mirror `cmd_schema` exactly (lines 2624-2629). Preserves byte-level migration ("rename verb, output unchanged"). Costs internal asymmetry — `taxonomy -o json` always returns an array; `taxonomy -o json-schema` sometimes returns an object, sometimes an array.
- **Option C — JSON Lines (one schema per line).** Stream-friendly, jq-ndjson-compatible, no array wrapping. Different shape from any current `taxonomy` output — would surprise users who expect `-o json` and `-o json-schema` to be sibling-shaped.

**Trade-offs.**

- **A** is the cleanest mental model — `taxonomy` returns a collection in every output format. `Plain` already iterates and prints (`main.rs:2438-2447`); `Json` builds `Vec<labels>` and serializes (`main.rs:2451-2470`); `Csv` is a header + rows (`main.rs:2472-2484`). `JsonSchema` would naturally be a `Vec<schema>`. Cost: README.md:77 example (`finetype schema "datetime.date.*" --pretty`) becomes `finetype taxonomy "datetime.date.*" -o json-schema` and the output shape changes from "array because glob" to "array always." Pipelines that worked on `finetype schema email` (single object) need a `.[0]` jq projection.
- **B** preserves byte-level output for single-key migrations, which is a real ergonomic win — `finetype schema email --pretty` and `finetype taxonomy email -o json-schema --pretty` could produce identical bytes. Cost: `taxonomy`'s output shape becomes context-dependent (object-or-array) for one format only, which is the kind of footgun the original `--file`/positional collision memo (`schema-cli-flag-collision`) called out.
- **C** is operationally appealing for downstream tooling but is a third shape entirely. Out of step with both `cmd_schema` (object-or-array) and `cmd_taxonomy` (always array). No memo proposes it; introducing it here adds a decision the rally doesn't need.

**Recommendation: A — always an array.** Consistency with the rest of `cmd_taxonomy`'s output formats wins over byte-parity with the deprecated verb. The single-line jq fix (`-o json-schema | jq '.[0]'`) goes in the CHANGELOG; the migration is mechanical. Pipelines that want a single document should target an exact key, accept the array, and project. This also matches the memo's framing (`schema-profile-overlap` line 191): "JSON Schema is just one more output format on a verb that already does type inspection." Output formats should not change shape based on cardinality.

---

## Decision 3 — Where the JSON Schema emitter lives

**Title:** Lift `build_json_schema` into a shared module vs duplicate it

**Context.** Today `build_json_schema` is a private function inside `crates/finetype-cli/src/main.rs:2642-2691` — emits a per-type JSON Schema document with `$schema`, `$id`, `title`, `description`, validation keywords, `examples`, `x-finetype-pii` (since PR #51's verbosity reduction). The table-mode emitter (`cmd_schema_table` at `main.rs:2699-`) inlines its own per-column emitter with a slightly different shape (a property within a parent object, no `$schema`/`$id` at the property level). The MCP server has a third copy at `crates/finetype-mcp/src/tools/schema.rs:30-79` (still verbose — visibility-cleanup spec deliberately left it untouched, constraint line 19). After fold, the type-mode emitter is called by `cmd_taxonomy` (this card) and the table-mode emitter is called by `cmd_profile` (sibling card 0003).

**Options.**

- **Option A — Lift to `finetype-core`.** Move `build_json_schema` (per-type) to `finetype-core::json_schema` or similar. Add a sibling `build_table_property_json_schema` (per-column-within-table). Both `cmd_taxonomy` and `cmd_profile` import them. The MCP `schema` tool can also import once the visibility-cleanup carve-out is lifted in v0.6.20 (mcp-surface-audit).
- **Option B — Lift to a CLI-internal helper module** (`crates/finetype-cli/src/json_schema.rs` or `crates/finetype-cli/src/schema_emitter.rs`). Same callers within the binary, no cross-crate dependency. MCP keeps its private copy until the post-rally MCP audit.
- **Option C — Duplicate inline.** Each command (`cmd_taxonomy`, `cmd_profile`) builds its own emitter. Simplest diff for this card; costs drift risk (next verbosity tweak has to land in three places — `cmd_taxonomy`, `cmd_profile`, MCP).

**Trade-offs.**

- **A** is the most principled — JSON Schema emission is a domain operation (taxonomy → JSON Schema), it's already domain-pure (no model state, no IO), and `finetype-core` is the natural home. Cost: introduces a new public surface in `finetype-core` that the model + DuckDB extension also gain visibility into. Worth doing eventually but adds API surface to the core crate. Test reach widens — you'd want unit tests at the core level rather than only via golden CLI tests.
- **B** matches the actual scope of this rally (two CLI callers, MCP deferred). The helper lives next to the binary it serves. No new public surface in `finetype-core`. Cost: when the MCP audit ships in v0.6.20 the helper has to move again, or MCP adds a CLI dependency (which it doesn't have today — `crates/finetype-mcp/Cargo.toml` depends only on `finetype-core` and `finetype-model`, per the workspace dep graph in `CLAUDE.md`).
- **C** kicks the can. The verbosity-reduction work in PR #51 already had to edit two emitters in `main.rs` (type-mode + table-mode), recorded in the visibility-cleanup spec at lines 152-159 — that's exactly the drift this option locks in further.

**Recommendation: B — CLI-internal helper module.** This rally has only CLI callers; MCP is deferred (rally `notes` and `mcp-surface-audit` memo line 116). The helper module captures the de-duplication that was deferred in PR #51 (the spec's implementation_notes:306 explicitly accepted the dual edit) without introducing core-API surface for one current consumer. When MCP picks up the format in v0.6.20, the natural move is to lift the same module to `finetype-core` — or, more likely per `mcp-surface-audit` line 41, the MCP `schema` tool dies entirely and the MCP `taxonomy`/`profile` tools call the same CLI-helper-equivalent via a small port. We can revisit at that point with a concrete second consumer in hand.

---

## Decision 4 — Disposition of the MCP `schema` tool

**Title:** Drop, hide, or alias the MCP `schema` tool in v0.6.19

**Context.** `mcp-surface-audit` memo (lines 80-90) recommends mirroring the CLI fold in MCP: drop `schema`, gain `taxonomy_schema` (or extend `taxonomy`), add `output_format: "json-schema"` to `profile`. The visibility-cleanup spec deliberately deferred this — constraint line 19: "MCP server is out of scope. MCP's `schema` tool stays verbose for v0.6.19 — it dies entirely in pipeline-reshape (v0.6.20)." That carve-out was for the verbosity reduction; this card asks the prior question: does the schema *verb* fold also leave MCP intact, or do they ship together? `mcp-surface-audit` Sequencing section (lines 116-118): "1. v0.7.0 CLI polish ships … 2. MCP audit follows in v0.7.1 … Doing it in this order means MCP changes are mechanical."

**Options.**

- **Option A — Leave MCP `schema` tool untouched in v0.6.19; mirror in v0.6.20.** CLI says one thing, MCP says another, for one release. The MCP server's tool count stays at 8 (CLAUDE.md:157, `crates/finetype-mcp/src/lib.rs:101-109`). The `schema` tool description still says "Export JSON Schema for a type key or a CSV file" (lib.rs:102). Mirror lands in the post-rally MCP audit spec.
- **Option B — Mirror MCP changes in this rally.** `crates/finetype-mcp/src/tools/schema.rs` becomes `tools/json_schema.rs` and is deleted as a top-level tool. `tools/taxonomy.rs` gains a `format: Option<String>` parameter that toggles between definition-summary and JSON Schema output. `tools/profile.rs` gains the same. `lib.rs:104-109` registration is removed.
- **Option C — Stub the MCP `schema` tool to return an error pointing at the replacements.** Tool stays registered; calling it returns `ErrorData::invalid_params("MCP schema tool removed; use taxonomy with format=json-schema or profile with format=json-schema")`. MCP clients calling the old tool name discover the migration interactively; the surface still shows in tool listings.

**Trade-offs.**

- **A** matches the visibility-cleanup spec's constraint and the memo's sequencing recommendation. MCP changes are mechanical when the CLI is settled. Cost: the CLI-vs-MCP-mirror invariant (card 0006 scenario "Public surface mirrors the MCP server" — line 37) is broken for one release. Honest counter: that scenario is about the public-vs-internal distinction (no `--model` in MCP, no `train` in MCP); the schema fold is a same-capability verb rename and the temporary asymmetry is bounded by a tracked spec.
- **B** keeps CLI and MCP in lockstep. Costs ~100-200 LOC of MCP edits (delete schema.rs (357 lines), edit taxonomy.rs and profile.rs, update lib.rs, update server description at lib.rs:142-148, update tool listings in CLAUDE.md:157 and README.md:115-124). The visibility-cleanup spec explicitly chose not to pay this cost in v0.6.19 — that was a sized decision, not an oversight.
- **C** is a half-measure that retains a registered tool that doesn't work. MCP clients see the tool name and get a runtime error — worse UX than either "tool gone" (clients see the absence and adapt) or "tool present and works." `mcp-surface-audit` line 4 names this as the right-after-CLI-settles work; doing it half-measure here adds entropy.

**Recommendation: A — defer to v0.6.20 MCP audit.** The visibility-cleanup spec's MCP carve-out (constraint line 19) was deliberate and remains correct: MCP audit comes after the CLI shape is settled, so MCP changes are mechanical mirrors. The rally's stated scope is the CLI fold; expanding to MCP would re-open the carve-out, doubling the diff for this rally. Document the temporary CLI/MCP asymmetry in CLAUDE.md (the MCP tools table at line 157 must keep `schema` for v0.6.19) and add an explicit follow-up reference in this card's spec to the MCP audit spec. This is the same posture the visibility-cleanup spec took for the schema verbosity reduction itself.

---

## Decision 5 — Migration surface (which docs/tests/examples change)

**Title:** Pin the exact files that need migration in v0.6.19

**Context.** Removing the verb breaks every example that calls it. The card requires we name the surface, not the migration text. Confirmed callers of `finetype schema` in this repo (Grep on `"finetype schema"`):

- `README.md:77` — `finetype schema "datetime.date.*" --pretty`
- `README.md:82` — `finetype schema data.csv --stdout > schema.json`
- `CLAUDE.md:208` — public-vs-internal command table row
- `CLAUDE.md:165, 211` — MCP tools table mentions `schema` (gated by Decision 4 — stays in v0.6.19)
- `.claude/skills/finetype-cli/SKILL.md:40-65, 244, 251` — finetype-cli skill reference
- `.claude/skills/finetype-pipeline/SKILL.md:57, 140` — pipeline skill reference
- `crates/finetype-cli/tests/cli_golden.rs:143-159` — `run_schema_json` helper, called by `golden_schema_email` (line 638) and `golden_schema_iso_date` (line 740)
- `CHANGELOG.md` — needs a v0.6.19 entry (existing entries reference `finetype schema`)
- `orbit/decisions/0031-table-level-schema-via-finetype-schema.md` — superseded record (status update)

**Options.**

- **Option A — Same-PR migration of every locus above except orbit/ artefacts and the MCP tools table (per Decision 4).** Tests (`run_schema_json` helper rewritten to call `taxonomy` with `-o json-schema` for `golden_schema_*`, becoming `golden_taxonomy_json_schema_email` / `golden_taxonomy_json_schema_iso_date`). README.md, CLAUDE.md command table, both SKILL.md files. CHANGELOG.md gains a v0.6.19 entry. Decision 0031 status flips to `superseded by NNNN`.
- **Option B — Migrate code/tests now, defer docs to a follow-up PR.** Visibility-cleanup spec line 23 already deferred a full doc-refresh PR ("Doc refresh (README, full CLAUDE.md command-table audit, docs/* sweep) is a follow-up PR"). Same shape: ship the verb removal, ship a doc-refresh PR right after. Risk: README contains `finetype schema` after the verb is gone. Anyone copy-pasting from README hits a clap error.
- **Option C — Keep the schema verb's `--help` long_about pointing at the replacements (option B from Decision 1) and defer doc migration.** Equivalent to "soft deprecate"; ruled out by Decision 1.

**Trade-offs.**

- **A** is sized — six small doc edits + two test renames + one decision-status flip. Net effect: the v0.6.19 release is internally consistent. The doc-refresh follow-up PR (per visibility-cleanup spec) becomes a smaller scope (other stale references, not the schema verb).
- **B** matches the precedent of PR #51 (which deferred doc refresh) but the situation differs: PR #51 hid commands behind `#[command(hide = true)]`, so existing examples kept working. This PR hard-removes a verb — README examples will hard-error against the binary. Visibility-cleanup spec line 23's deferred doc-refresh assumed the surface still worked. Removing a verb without updating the docs that demonstrate it ships a known-broken README.
- **C** is dominated by Decision 1.

**Recommendation: A — same-PR migration.** The README and skills docs are the first thing a new user reads; shipping a release where copy-paste from README errors is a Spark-Joy violation (Meridian Pillar 1). Two test renames is small; the README and skill edits are mechanical (one-line search-and-replace). Flip Decision 0031's status to `superseded by NNNN` where NNNN is the new MADR (Decision 6, below). Out of scope: orbit/specs/2026-04-22-duckdb-extension-ergonomics/ historical references (those are spec history, not user-facing docs).

The two new tests should preserve the current assertions:

```
golden_schema_email          → golden_taxonomy_json_schema_email
golden_schema_iso_date       → golden_taxonomy_json_schema_iso_date
```

Both shift from `cargo run -- schema KEY --pretty` to `cargo run -- taxonomy KEY -o json-schema --pretty` (or whatever the final flag form lands on — clap `OutputFormat::JsonSchema` likely). Per Decision 2 (always-array), the JSON shape changes from `obj` to `[obj]`, so each test gains a `.as_array().unwrap()[0]` projection at the top of the assertion block. The existing assertion bodies (presence of `pattern`, `x-finetype-pii`, absence of `x-finetype-broad-type` etc.) stay unchanged — the verbosity reduction has already shipped. The sibling card `0003-tabular-data-profiling.yaml` owns the `golden_profile_json_schema_*` test additions; this card does not duplicate them.

---

## Decision 6 — Decision-register entry for the fold

**Title:** Whether and where to record the schema-verb fold in the MADR register

**Context.** Visibility-cleanup spec line 374-375 names the deferred decision: "Schema verb folds entirely (settled in interview Q1) — MADR `decisions/<NNNN>-schema-verb-folds-entirely.md`, linked from both this spec and the pipeline-reshape spec." The fold has been agreed in memos but not yet written to the formal register. Decision 0031 (`0031-table-level-schema-via-finetype-schema.md`) accepted "extend `schema` to detect file paths" — that decision is being reversed. Memo `schema-cli-flag-collision` is already marked Superseded by `schema-profile-overlap`. The next free MADR number is 0070 (current high is 0069 per CLAUDE.md and the directory listing).

**Options.**

- **Option A — Record now, in this rally's spec dir, as MADR 0070.** Single new file `orbit/decisions/0070-schema-verb-folds-entirely.md` capturing: context (overlap with `taxonomy`/`profile`), considered options (rename, soft deprecation, hard fold), decision (hard fold, both replacements ship same release), consequences (CLI surface tightens to four+taxonomy verbs; MCP audit deferred per Decision 4). Update Decision 0031's frontmatter to `status: superseded by 0070`.
- **Option B — Record as part of the sibling cards' spec dirs.** Either 0003 or 0005 captures it; this card just deletes code. Cost: the deletion is the load-bearing change of the fold and lives in this card; recording the decision under a sibling is misattribution.
- **Option C — Defer the MADR until v0.6.20 ships the MCP mirror.** Treat the CLI fold as an internal-cleanup artefact and only formalize when the full surface (CLI + MCP) is consistent. Cost: a code change of this magnitude (a public verb dies) without a register entry violates the "Decisions captured, not forgotten" Mission value (CLAUDE.md line 5).

**Trade-offs.**

- **A** is the cleanest and matches the visibility-cleanup spec's stated plan. Cost: one new file + one frontmatter flip on 0031. The MADR template (`orbit/decisions/_template.md`) is short. The MADR can be written from the memos almost verbatim (`schema-profile-overlap` lines 154-204 already supplies Considered Options + Decision Outcome + Consequences).
- **B** is wrong attribution — sibling cards add features (json-schema output to taxonomy and profile); this card removes the feature they replace. The decision belongs where the deletion lives.
- **C** violates Mission. The fold is a contract-level change; it warrants a register entry now, not later.

**Recommendation: A — write MADR 0070 in the same PR as the code change.** Title: "Schema verb folds entirely — type-mode → taxonomy, table-mode → profile." Status `accepted`. Reference 0031 as superseded; flip 0031's frontmatter to `status: superseded by 0070`. Cross-link from this card's spec, the rally manifest, and the two sibling spec dirs. This is the canonical record for any future "why did we remove `schema`?" question.

---

## Side note — `--file`/`--taxonomy` flag-collision is moot

Memo `schema-cli-flag-collision` (lines 5-6) already self-marks: "Superseded by `2026-04-27-schema-profile-overlap.md` (revised) — schema verb folds entirely; rename is moot." This card confirms: with `Commands::Schema` deleted, the `-f, --file` flag at `crates/finetype-cli/src/main.rs:204-205` (`#[arg(short, long, default_value = "labels")] file: PathBuf`) goes away with the verb. No carry-forward action. The same memo's parting note (lines 14-17) — that other subcommands (`generate`, `taxonomy`, `check`) share the same `-f, --file` taxonomy-directory pattern — remains a separate observation for a future card; explicitly **not** in scope here.

---

## Summary — recommended path

```
| #  | Decision                                  | Recommendation                          |
|----|-------------------------------------------|-----------------------------------------|
| 1  | Removal cadence                           | Hard removal in v0.6.19                 |
| 2  | taxonomy -o json-schema multi-match shape | Always emit a JSON array                |
| 3  | Where the emitter lives                   | CLI-internal helper module              |
| 4  | MCP schema tool disposition               | Defer to v0.6.20 MCP audit              |
| 5  | Migration surface                         | Same-PR migration of code + docs + tests|
| 6  | Decision-register entry                   | Write MADR 0070; supersede 0031         |
```

These six together describe a single hard-removal PR that ships in the v0.6.19 release alongside the two sibling-card replacements, leaves MCP unchanged for one release, and records the architectural move in the formal register.
