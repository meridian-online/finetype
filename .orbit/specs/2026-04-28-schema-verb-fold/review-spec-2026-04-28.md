# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** `/Users/hugh/github/meridian-online/finetype/.orbit/specs/2026-04-28-schema-verb-fold/spec.yaml`
**Verdict:** REQUEST_CHANGES

---

## Review Depth

```
| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 7 |
| 2 — Assumption & failure | content signals (cross-system boundary, MCP carve-out, doc migration), 2 HIGH findings in Pass 1 | 3 |
| 3 — Adversarial | not triggered (no cascading or rollback risk surfaced; Pass 2 findings are all locally fixable) | — |
```

## Findings

### [HIGH] `taxonomy` clap variant has no positional `KEY` argument — spec is missing the AC that adds it
**Category:** missing-requirement
**Pass:** 1
**Description:** The spec's central UX is `finetype taxonomy KEY -o json-schema`, where `KEY` is an exact match (`identity.person.email`) or glob (`"datetime.date.*"`, `"identity.person.*"`). But `Commands::Taxonomy` in `crates/finetype-cli/src/main.rs:172-197` is `[OPTIONS]`-only — it accepts `--domain`, `--category`, `--priority`, but no positional argument. I confirmed this by running the CLI: `cargo run -p finetype-cli -- taxonomy email -o json-schema` fails with `error: unexpected argument 'email' found`. AC-02 reads as if `cmd_taxonomy` already has the matched defs in hand, but no AC actually adds the positional `KEY` arg or the glob-matching logic.
**Evidence:** `main.rs:172-197` (current Taxonomy variant); `main.rs:2433-2453` (current `cmd_taxonomy` filtering uses domain/category/priority only, not a positional KEY); `main.rs:2605-2648` (current `cmd_schema` glob-matching logic that needs to be ported). Spec ac-02 only says "iterates the matched `defs: Vec<...>` (already filtered/sorted by domain/category/priority)" — silent on positional input.
**Recommendation:** Add an explicit AC (or amend ac-02) covering: (1) `Commands::Taxonomy` gains an optional positional `type_key: Option<String>`; (2) when present, it filters via the same exact-match-or-glob predicate as the current `cmd_schema`; (3) when absent, the existing `--domain`/`--category`/`--priority` behaviour is preserved; (4) the "edit-distance suggestions on KEY-not-found" behaviour from `cmd_schema` (lines 2629-2647) is ported into `cmd_taxonomy` (the spec's implementation_notes mentions this conditionally but it's not an AC). Without this, the spec's verification commands do not parse.

### [HIGH] `cmd_taxonomy` has no `--pretty` flag — README migration target is broken or implicit
**Category:** constraint-conflict
**Pass:** 1
**Description:** Spec ac-05 migrates `README.md:77` from `finetype schema "datetime.date.*" --pretty` to `finetype taxonomy "datetime.date.*" -o json-schema --pretty`. But `Commands::Taxonomy` does not have a `--pretty` flag — only `Commands::Schema` does (`main.rs:208-210`). AC-02's verification says output is "pretty-printed" unconditionally, which is internally consistent but contradicts the explicit `--pretty` token in the README migration target. Either: (a) `--pretty` is unconditional and the README example should drop `--pretty`; (b) add `--pretty` to `Commands::Taxonomy`; (c) accept `--pretty` as a no-op for backwards compatibility.
**Evidence:** `main.rs:172-197` shows no `pretty` field on the Taxonomy variant. Spec line 99-100 (ac-05 README migration) and line 49 (ac-02 "pretty-printed") are inconsistent.
**Recommendation:** Pick one. Recommendation: drop `--pretty` from the README migration since ac-02 already commits to unconditional pretty-printing for `-o json-schema`. Add a constraint that JSON output formats on `taxonomy` are pretty-printed by default (mirroring the existing behaviour of `OutputFormat::Json` at `main.rs:2506`, which already calls `serde_json::to_string_pretty`). Then ac-05's README target becomes `finetype taxonomy "datetime.date.*" -o json-schema`.

### [MEDIUM] Helper module location stated three different ways — spec / interview / decisions / actual code disagree
**Category:** constraint-conflict
**Pass:** 1
**Description:** This card's spec says (constraint #3, line 12, and ac-01) the helper lives at `crates/finetype-mcp/src/json_schema.rs`. The interview Q3 (line 82-85) and decisions D3 (line 78) say it lives at `crates/finetype-cli/src/json_schema.rs`. The companion spec (`2026-04-28-profile-json-schema-output/spec.yaml` ac-02) also says `crates/finetype-cli/src/json_schema.rs`. The actual shipped helper from PR #53 lives at `crates/finetype-mcp/src/json_schema.rs` (per `main.rs:8` `use finetype_mcp::json_schema;` and `crates/finetype-mcp/src/json_schema.rs` exists with `emit_table_schema`). The new spec is the one telling the truth about the as-shipped state, but it leaves the interview / decisions / companion spec contradicting it without a paragraph explaining the drift.
**Evidence:** `main.rs:8` (`use finetype_mcp::json_schema;`), `crates/finetype-mcp/src/json_schema.rs` exists; spec line 12, 196-202 say finetype-mcp; interview line 82-85 says finetype-cli; decisions D3 says CLI-internal helper; companion spec ac-02 says `crates/finetype-cli/src/json_schema.rs`.
**Recommendation:** Add a short note in the spec's implementation_notes block explicitly stating: "Interview Q3 and decisions D3 both pre-date the card 0003 implementation, which placed the helper in `finetype-mcp` because finetype-cli already depends on finetype-mcp and the interview's recommendation B was satisfied by reachability rather than physical CLI residency. This card extends that decision; no module relocation is in scope." This avoids future archaeologists thinking the spec contradicts itself. (The first bullet of implementation_notes is on the right track but doesn't fully spell out the drift.)

### [MEDIUM] `cmd_schema_table` is not "a thin caller" — spec's deletion claim understates the LOC change
**Category:** test-gap
**Pass:** 1
**Description:** Spec implementation_notes line 217-221 says: "The `cmd_schema_table` function still exists in main.rs as a thin caller into `json_schema::emit_table_schema`. Card 0003 left it in place so the legacy `schema FILE.csv` invocation kept working through v0.6.19. With the verb gone, the function is dead code and gets deleted with the dispatch arm." But `cmd_schema_table` is **~200 lines** (`main.rs:2735-~2960`), not a thin caller — it loads the model, wires sibling-context, reads the CSV, classifies all columns, then calls `emit_table_schema` at the end. The deletion is bigger than "dead code with the dispatch arm."
**Evidence:** `main.rs:2735-2960` shows model loading (lines 2745-2781), taxonomy compilation (2784-2793), CSV reading (2796-2798), per-column classification with sibling-context branch (2814-2906), then the `emit_table_schema` call at 2932. ~200 LOC.
**Recommendation:** Update implementation_notes to state correctly that `cmd_schema_table` is ~200 LOC of model+classification+emit logic; deleting it cleanly requires confirming that all the wiring (`load_multi_branch_classifier`, `wire_model2vec_and_siblings`, etc.) is still reachable from `cmd_profile`. Add a short sanity AC or constraint: "After deletion, no profile/load/validate code path becomes orphaned (all helpers used only by `cmd_schema_table` are either deleted with it or already used elsewhere)." A grep on the helper function names will catch this in implementation, but right now the spec underestimates the deletion size and doesn't ask for a dead-code sweep.

### [MEDIUM] ac-02 says "always array even for single matches" but doesn't specify "when no matches"
**Category:** test-gap
**Pass:** 1
**Description:** AC-02 verification covers single-match (`taxonomy email`) and multi-match (`--domain identity`). It is silent on the unknown-key case. Today `cmd_schema` exits with code 1 plus edit-distance suggestions when the key is unknown (`main.rs:2629-2647`). Implementation_notes line 224-227 says: "The KEY-not-found branch with edit-distance suggestions stays in `cmd_taxonomy` if not already there (verify during implementation; if absent, port the suggestion logic to `cmd_taxonomy` as a `Display`-time fallback)." A "verify during implementation" line is not an AC — there's no test that catches a missing port. Constraint #2 says the output is always an array; under JSON-schema mode for unknown KEY, what's the contract? Empty array `[]`? Exit code 1 + suggestions on stderr? The spec is silent.
**Evidence:** Spec ac-02 verification (lines 51-56) and implementation_notes line 224-227 vs `main.rs:2629-2647` (edit-distance behaviour today).
**Recommendation:** Add a sub-bullet to ac-02 (or a new ac): "`finetype taxonomy unknown_key -o json-schema` exits 1, prints edit-distance suggestions to stderr, prints nothing to stdout (NOT an empty array — for parity with today's `cmd_schema`)." This pins the migration contract; otherwise scripts using `finetype schema BAD_KEY 2>&1` will silently break in v0.6.19.

### [MEDIUM] Migration grep verification will return false-positive hits and partial false-negatives
**Category:** test-gap
**Pass:** 1
**Description:** AC-05 verification: `grep -RE "finetype schema" README.md CLAUDE.md .claude/skills/`. This catches `finetype schema KEY` but does not catch `finetype schema FILE.csv` if the example uses tabs/newlines, and does not catch hidden in code blocks where `\` continues a command. More problematically, **the MCP tools row in `CLAUDE.md:165`** has `| `schema` | Export JSON Schema...` — the literal string `finetype schema` does not appear, but the MCP tool *named* `schema` is the v0.6.19 carve-out (constraint #4) and grepping `"finetype schema"` will not assert the MCP row stays put. Conversely, the verification grep claim "excludes orbit/ and CHANGELOG" is implemented by the `-R` path argument list — `-R README.md CLAUDE.md .claude/skills/` is fine, but the spec says "(excluding orbit/ and CHANGELOG historical entries)" which is misleading because those paths are not searched anyway. False precision.
**Evidence:** Spec ac-05 (line 121-125); CLAUDE.md:165 (MCP tools row); CLAUDE.md:208 (CLI command row).
**Recommendation:** Tighten the verification:
1. `grep -RE "finetype schema" README.md CLAUDE.md .claude/skills/` returns zero hits — keep this.
2. Add a positive assertion: `grep -E "^\| .schema. \|" CLAUDE.md` returns exactly one hit (the MCP tools row, retained per constraint #4).
3. Add: `grep "finetype taxonomy.*json-schema\|finetype profile.*json-schema" README.md` returns at least 2 hits (the new examples).
4. Drop the misleading parenthetical about orbit/ and CHANGELOG.

### [MEDIUM] Verbosity-contract claim about "fix-in-passing" silently ships a behaviour change
**Category:** assumption
**Pass:** 1
**Description:** Implementation_notes line 203-209 says: "The pre-existing `build_json_schema` in `main.rs:2642` does NOT emit `x-finetype-label` (only `x-finetype-pii`). The new `emit_type_schema` MUST emit BOTH per the verbosity contract, matching `emit_table_schema`'s behaviour. This is a fix-in-passing." I confirmed: the current `build_json_schema` at `main.rs:2678-2727` emits only `x-finetype-pii` (line 2724). This means the v0.6.19 `taxonomy KEY -o json-schema` output will carry an extension that the v0.6.18 `schema KEY` output did not. That's a behaviour change in the schema export contract, slipped in as a "fix-in-passing." Constraint #5 frames this as preservation but it's actually an addition. The two existing golden tests (`golden_schema_email`, `golden_schema_iso_date`) currently do NOT assert `x-finetype-label` presence — they only assert presence of `pattern`, `x-finetype-pii`, and absence of dropped extensions (e.g., `x-finetype-broad-type`). The renamed tests in ac-04 say "Existing assertion bodies stay unchanged." So the new behaviour is not actually tested.
**Evidence:** `main.rs:2678-2727` (current `build_json_schema` — no `x-finetype-label`); `cli_golden.rs:638-680` (current assertions don't check label presence); spec implementation_notes line 203-209; ac-01 verification *does* require `x-finetype-label` presence at the unit-test level (line 36); ac-04 says assertion bodies stay unchanged.
**Recommendation:** Either (a) drop the "fix-in-passing" — keep type-mode output emitting `x-finetype-pii` only, matching v0.6.18 byte-for-byte (the migration message is then "rename verb, output unchanged"); or (b) keep the fix-in-passing but call it out properly: add a CHANGELOG bullet under "Changed" mentioning that `x-finetype-label` is now emitted on type-mode JSON Schema output, AND add `x-finetype-label` presence + correct value to the renamed golden tests' assertion bodies. Recommendation: do (b) — symmetry with `emit_table_schema` is a real win and the contract was already meant to be `label + pii`. But the spec must record the change explicitly, not call it a "fix in passing."

### [MEDIUM] Public-surface count is off-by-one in the README migration target
**Category:** constraint-conflict
**Pass:** 2
**Description:** Spec ac-05 line 106-107: "The public surface is now: `infer`, `profile`, `validate`, `load`, `mcp`, `taxonomy`." That is 6 commands. CLAUDE.md:186 currently says "Public (v0.6.19)| `infer`, `profile`, `schema`, `validate`, `load`, `mcp`, `taxonomy`" — 7 commands (with `schema`). Removing `schema` brings it to 6. But CLAUDE.md:177 also says "As of v0.6.19, `finetype --help` lists **only the 7 public commands**." That sentence will now be wrong. The spec's ac-05 doesn't mention updating the count.
**Evidence:** CLAUDE.md:177 ("only the 7 public commands"), CLAUDE.md:186 (the table); spec ac-05 (table row deletion only, count not updated).
**Recommendation:** Add to ac-05: "CLAUDE.md:177 sentence updated from `7 public commands` → `6 public commands`." Also touch `crates/finetype-cli/src/main.rs` doc comments / `--help` long_about if any reference a count (a quick grep on "7 public" or "six public" would catch).

### [LOW] CLAUDE.md `Sprint Goal` section unaffected — confirm
**Category:** missing-requirement
**Pass:** 2
**Description:** CLAUDE.md has dense "Recent work" entries that include `finetype schema` mentions in historical context (e.g., line 42 mentions `schema` in the context of PR #51's verbosity reduction). The spec excludes "orbit/ historical references" but doesn't say whether CLAUDE.md historical entries get touched. Reading closely: line 42 talks about `schema` as a noun for the export, not as an invocation — so not a copy-paste-broken reference. But verification grep `"finetype schema"` will catch any literal invocation that might exist in CLAUDE.md historical text. Worth confirming none exist.
**Evidence:** CLAUDE.md line 42 mentions schema export but no `finetype schema` invocation; no other historical entry uses the literal verb. Per my grep above, only line 208 has `finetype schema`.
**Recommendation:** Add to ac-05's verification: "CLAUDE.md historical 'Recent work' entries are out of scope; `grep 'finetype schema' CLAUDE.md` should return zero hits after migration (the only current hit is line 208 which is being deleted)." This is a nit but pins what "in scope" means for CLAUDE.md.

### [LOW] AC-09 asserts "zero new clippy warnings" but doesn't pin the baseline
**Category:** test-gap
**Pass:** 2
**Description:** AC-09 verification: "make ci exits 0. Zero new clippy warnings introduced." But "new" relative to what? The current main is in a partially-clippy-clean state per the spec's own parenthetical: "(the pre-existing finetype-eval rust-1.95 clippy lints (out of scope, pre-existing on main) are not regressed)." A reviewer running `make ci` against this card's PR will see warnings; how do they know which are pre-existing? A `make ci` run on `main` immediately before review-pr would establish the baseline.
**Evidence:** Spec ac-09 (lines 167-177).
**Recommendation:** Tighten the AC: "AC-09 baseline: run `make ci` on `main` at HEAD immediately before this card's branch is rebased; any warnings present in that run are the pre-existing baseline. The card's PR must not introduce warnings absent from that baseline." Or, more simply: "make ci exits 0; if there are pre-existing warnings, count them on `main` and assert the count is unchanged."

---

## Honest Assessment

The spec is structurally sound — the goal is crisp, the constraints are mostly internally consistent, the ACs cover the deletion / replacement / docs / decision-record axes. The plan is small, the rally context is clear, and the precedent (PR #51's hard-removal posture, card 0003's helper) is well-cited.

The biggest risk is finding #1: the spec's central UX (`taxonomy KEY -o json-schema`) does not currently parse, because `Commands::Taxonomy` has no positional argument. The implementation will inevitably add it, but the spec doesn't tell the implementer to — there's no AC for "add positional KEY arg to Taxonomy clap variant" or "port the glob-matching logic from cmd_schema to cmd_taxonomy." Without that AC, an implementer could plausibly read the spec as "wire `cmd_taxonomy`'s existing filtering into a JsonSchema arm" and ship something where every invocation requires `--domain X --category Y` — losing the exact-match-by-KEY ergonomics that PR #51 left intact on the schema verb. That risks a v0.6.19 release where the only way to get the email JSON Schema is `taxonomy --domain identity --category person -o json-schema` and then jq-filter the result.

Finding #2 (no `--pretty` flag on taxonomy) is the same shape: the spec's verification target doesn't compose with the binary's actual surface. Pick one.

Findings #3-#7 are quality — the contradictions about helper-module location (already resolved in code, but the spec leaves the interview/decisions/companion-spec all saying something else), the underestimated `cmd_schema_table` deletion, the under-specified unknown-key behaviour, the verification-grep precision, and the silent `x-finetype-label` addition. Each is locally fixable; together they suggest a quick second sweep to align the spec with the as-shipped state of card 0003 before this card's implementation begins.

Recommend a small spec revision pass addressing findings #1 and #2 (mandatory — verification commands don't parse without them), and #3-#7 (recommended — pre-empts review-pr nits and prevents a silent contract change). Findings #8-#9 are LOW and can be folded in alongside the others without reopening the design conversation.
