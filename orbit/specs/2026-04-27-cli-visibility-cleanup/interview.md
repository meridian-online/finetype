# Design: CLI visibility cleanup

**Date:** 2026-04-27
**Interviewer:** Nightingale
**Card:** orbit/cards/0006-command-line-interface.yaml
**Target release:** v0.6.19

---

## Context

Card 0006 — *Command-line interface* — six scenarios, goal: *Four
public verbs (infer | profile | validate | mcp + taxonomy with
json-schema output) composing into pipelines; maintainer affordances
hidden; surface mirrors MCP*.

**Prior specs touching CLI:**
- `2026-02-27-cli-types` / `2026-03-11-cli-types` — shipped the
  original surface
- (No prior visibility/polish work)

**Today's source material — interview-equivalent:** seven memos
folded into this spec:

```
| Memo                          | Resolution                                              |
|-------------------------------|---------------------------------------------------------|
| cli-model-flag                | REMOVE flag; migrate 4 scripts to FINETYPE_MODEL env    |
| cli-sharp-only-flag           | REMOVE flag + dead gate code at 4 call sites             |
| cli-model-type-flag           | DEFER — separate spec (delete-legacy-classifiers)       |
| cli-check-internal            | HIDE                                                    |
| cli-generate-vs-faker         | HIDE CLI subcommand; MCP `generate` is public surface   |
| schema-export-verbosity       | drop derivable x-finetype-* fields (label + pii only)   |
| validate-required-flags       | --db/--table optional-but-mutually-required             |
```

Plus one item surfaced during the audit:
- **eval-gittables subcommand** — REMOVE (zero callers; Makefile uses
  the standalone `eval-gittables-cli` binary, not the subcommand).

Plus one item dropped after Hugh spotted the broader fold:
- **schema-cli-flag-collision** — superseded; schema verb dies
  entirely in the pipeline-reshape spec, so renaming `--file` is moot.

**Gap:** implementation hasn't shipped. The *what* is settled per
memo; this design session settled the *how* of risk posture,
visibility mechanism, scope boundaries, and the cull-vs-hide split.

---

## Q&A

### Q1: Schema verb consolidation

**Q:** Can `type-mode taxonomy export` live under the `taxonomy`
command, dissolving the `schema` verb entirely?

**A:** Yes. Both schema modes fold to their natural sibling:
- Type-mode → `finetype taxonomy KEY -o json-schema`
- Table-mode → `finetype profile -f file.csv -o json-schema`

The `schema` verb disappears. Public CLI surface tightens from five
verbs to four (+ taxonomy). This fold lives in the pipeline-reshape
spec (out of scope here), but it shapes this spec by making the
`schema --file` rename moot.

**Card update:** goal restated to "Four public verbs."
**Memo update:** schema-profile-overlap revised to cover both folds;
schema-cli-flag-collision marked superseded.

### Q2: Risk posture for user-visible breaking surfaces

**Q:** Hard removal in v0.6.19 vs deprecation cycle vs mixed?

**A:** **Hard removal in v0.6.19.** `--sharp-only` errors immediately;
schema export drops derivable fields day one. Cleanest end-state in
one release. Scripts pinning the old shape get an explicit error
rather than silent drift.

### Q3: Visibility mechanism for maintainer-only subcommands

**Q:** clap `#[command(hide)]` vs cargo feature gate vs separate
binary?

**A:** **clap `#[command(hide)]`** — paired with a documented
internal-API stability tier in CLAUDE.md.

Reasoning:
1. Already in use for train/eval/eval-gittables; consistent pattern.
2. Zero coordination cost for internal callers (`make ci`,
   `prepare_multibranch_data.py`, sweep scripts).
3. The signal we want — "not for end users" — is achieved by
   absence from `--help`. MCP's tool list (which already excludes
   these) is the strongest "public boundary" signal; CLI hide
   aligns with that.
4. Forward path preserved — hide → feature-gate or hide → separate-
   binary is a one-shot refactor later if needed.

**Caveat raised:** *"only if it doesn't force us to maintain functions
we don't actually need."* This caveat reshaped the spec — see Q4.

### Q4: Cull-vs-hide audit (response to Q3 caveat)

**Q:** For each candidate, do we actually use it?

**A:** Audit-and-cull. Three buckets emerged:

```
| Item                  | Audit                                             | Action              |
|-----------------------|---------------------------------------------------|---------------------|
| check                 | actively used (CI, Makefile)                      | HIDE                |
| generate              | actively used (training data prep + 2 scripts)    | HIDE                |
| train                 | actively used (sweep wrappers)                    | HIDE (already)      |
| eval                  | actively used (scripts/eval.sh)                   | HIDE (already)      |
| eval-gittables (sub)  | ZERO callers; Makefile uses standalone binary     | REMOVE              |
| --model               | 4 scripts; FINETYPE_MODEL env var covers all uses | REMOVE flag,        |
|                       |                                                   | migrate scripts     |
| --model-type          | entangled with legacy classifier code paths       | DEFER — separate    |
|                       | (~2,500–3,500 LOC across 2 crates)                | spec (memo 2026-04- |
|                       |                                                   | 27-delete-legacy-   |
|                       |                                                   | classifiers)        |
| --sharp-only          | gate at 4 call sites always evaluates false       | REMOVE + dead gate   |
|                       | (multi-branch is always present in production)    | cleanup             |
```

Net: 2 hides, 3 removals, 1 defer. The deferred item gets its own
memo (`2026-04-27-delete-legacy-classifiers.md`) capturing the
entanglement and follow-up scope.

### Q5: MCP `schema` tool tracking

**Q:** Does MCP's schema tool's emitter track the verbosity reduction
in v0.6.19, or stay verbose since it dies in pipeline-reshape anyway?

**A:** **Stays verbose.** Don't invest in a tool that disappears in
the next release. CLI gets lean schema in v0.6.19; MCP catches up
when its schema tool is replaced by `taxonomy` + `profile`
json-schema output modes during pipeline-reshape.

### Q6: Documentation refresh scope

**Q:** Minimum-only vs full refresh vs follow-up PR?

**A:** **Doc refresh follow-up PR.** Code + `--help` text + minimum
CLAUDE.md command-table updates land in v0.6.19. README/CLAUDE/docs
full refresh follows as a separate PR before release, riding the
doc-drift CI gate from the stale-documentation memo.

This keeps the code change tight and shippable; the doc refresh has
its own logic (informed by the proposed CI gate) that benefits from
not being entangled with the surface-change diff.

---

## Summary

### Goal

Ship a tight public CLI surface for v0.6.19: five subcommands hidden
or removed, two flags removed (with one deferred), one wire format
tightened (schema export), one flag pair relaxed (validate
`--db/--table`). Document the internal-API stability tier.

### Constraints

- **Hard removal in v0.6.19** — no deprecation cycle. Scripts that
  pin removed surfaces error, don't drift silently.
- **Internal callers must not break.** `make ci`, training data prep,
  sweep wrappers, eval scripts continue working unchanged where
  possible. The `--model` migration to env var is the one
  internal-script edit.
- **MCP changes are out of scope.** MCP's `schema` tool stays
  verbose; the surface mirror happens in pipeline-reshape (v0.6.20).
- **Doc refresh is a follow-up PR**, not part of this spec.
- **`--model-type` and the legacy classifier code paths are deferred
  to a separate spec.** Touching them here would triple the diff
  and entangle two release themes.

### Success Criteria

- `finetype --help` shows four public verbs (infer, profile,
  validate, mcp) plus taxonomy. Schema verb still present in
  v0.6.19 (folds in pipeline-reshape v0.6.20).
- `finetype check` and `finetype generate` callable but absent from
  `--help`.
- `finetype eval-gittables` returns "unknown subcommand" error.
- `--sharp-only`, `--model` flags rejected by clap (unknown arg).
- Schema export emits only `x-finetype-label` and `x-finetype-pii`
  extension fields.
- `finetype validate -f data.csv --schema schema.json` works without
  `--db`/`--table`; passing one without the other errors via clap's
  `requires_all`.
- 4 internal scripts (`distill_batches.sh`, `eval.sh`, `train.sh`,
  `llm_label.py`) migrated from `--model <path>` to `FINETYPE_MODEL=
  <path>` env var.
- All tests pass; `make ci` green.
- CLAUDE.md updated with: (a) public/internal API stability tier
  paragraph, (b) command table refreshed.

### Decisions Surfaced

1. **Schema verb folds entirely** (both modes — type-mode →
   taxonomy, table-mode → profile). Card 0006 goal updated to
   "four public verbs." → `decisions/<NNNN>-schema-verb-folds-entirely.md`
   (to record).

2. **Hard removal posture for v0.6.19.** No deprecation cycle.
   Scripts that pin old surfaces should error explicitly. → record
   in spec, no MADR (it's a per-spec posture, not a standing
   architectural decision).

3. **Hide-in-clap is the visibility mechanism.** Internal API exists
   intentionally; stability tier documented in CLAUDE.md. → record
   in spec; if the stability-tier doc gets pushback it can graduate
   to MADR.

4. **Cull-not-just-hide.** `--model` flag removed entirely (env var
   covers); `eval-gittables` subcommand removed (zero callers);
   `--sharp-only` removed including dead gate cleanup. Hide is for
   things genuinely used internally. → record in spec.

5. **`--model-type` and legacy classifiers deferred.** Memo
   `2026-04-27-delete-legacy-classifiers.md` captures the follow-up
   scope. → record cross-reference in spec.

### Implementation Notes

Surfaced from codebase exploration during the design session:

- **`--sharp-only` plumbing.** `sharp_only: bool` is parameterised
  through 12 call sites in `crates/finetype-cli/src/main.rs` (lines
  90, 255, 365, 574, 588, 679, 692, 725, 737, 1313, 1713, 3159,
  4142). The actual gate is at 4 sites (1482, 1772, 3213, 4199),
  always `if !sharp_only && !column_classifier.has_multi_branch()`.
  Plus one alternative pipeline branch at line 3330 (`let pipeline
  = if sharp_only`). Cleanup is mechanical: drop the param at every
  signature, drop the dead gate code, collapse the alternative
  pipeline branch.

- **Scripts using `--model`** (4 files):
  - `scripts/distill_batches.sh`
  - `scripts/eval.sh`
  - `scripts/train.sh`
  - `scripts/llm_label.py`

  Migration pattern: `--model "$MODEL_PATH"` →
  `FINETYPE_MODEL="$MODEL_PATH"` exported before the call. (The
  CLI already reads `FINETYPE_MODEL` per the model-name env var
  table in CLAUDE.md.)

- **Schema export emitter** is shared between schema's two modes.
  The verbosity reduction edits the emitter once and propagates to
  both modes (and to `profile -o json-schema` and `taxonomy KEY -o
  json-schema` after pipeline-reshape).

- **`finetype check`** definition at `main.rs:274`. Existing hidden
  commands (`train`, `eval`, `eval-gittables`) use
  `#[command(hide = true)]` — same attribute applies here.

- **`finetype generate`** definition follows the same pattern.

- **`eval-gittables` subcommand** at `main.rs:383` (struct) and 743
  (dispatch). No internal callers found via grep across .sh / .py /
  Makefile / .yaml / .yml. Removal: delete the variant, the
  dispatch arm, and `cmd_eval_gittables` if it exists. The
  standalone binary `crates/finetype-eval/src/bin/eval_gittables_cli.rs`
  remains untouched — Makefile uses it directly.

- **`validate --db/--table` relaxation.** clap's `requires_all`
  attribute on `--db` and `--table` (each requiring the other)
  makes them optional-but-mutually-required. When both absent,
  validation runs in check-only mode (no .db written).

- **Stability tier doc.** Add a paragraph to CLAUDE.md:
  ```
  ## Public vs internal CLI surface

  **Public commands** (`infer`, `profile`, `validate`, `mcp`,
  `taxonomy`, plus `schema` until v0.6.20): semver-stable. Breaking
  changes ship with major version bumps and migration notes.

  **Internal commands** (`check`, `generate`, `train`, `eval`):
  maintainer-only — present for CI, training, and evaluation
  workflows. Hidden from `--help`. May break across minor versions
  without notice. Use the MCP server's `generate` tool or
  appropriate libraries (e.g., Python `faker`) for production
  needs.
  ```

### Open Questions

None at intent level. Two implementation-level items the spec
should call out (not blockers):

- **Stability tier doc location.** CLAUDE.md is suggested above;
  alternative is a CONTRIBUTING.md or docs/INTERNAL_API.md. Spec
  can pick.
- **MADR for the schema-verb-folds-entirely decision.** Whether to
  record now (in this spec) or with the pipeline-reshape spec
  (where the change actually ships). I lean: record now, link from
  both specs — the decision is settled even if the implementation
  ships later.

---

**Next step:** `/orb:spec` to generate spec.yaml from this record.
