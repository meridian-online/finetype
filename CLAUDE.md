# FineType

FineType is a type inference engine that detects and classifies data types in tabular datasets. It's the core analytical engine of the Meridian project.

## The Meridian Pillars

Every decision in this repo should reflect these principles:

1. **Spark joy for analysts** — Type inference should feel magical, not tedious. Clear output, helpful error messages, sensible defaults.
2. **Write programs that do one thing and do it well** — Each command has one job: `profile` discovers, `taxonomy` generates schema, `validate` enforces and materialises typed output. Separate concerns for separate tools.
3. **Design for the future, for it will be here sooner than you think** — The type taxonomy, model architecture, and extension interfaces should accommodate new data types and formats without breaking existing behaviour.

### Precision Principle

Precision is what makes FineType valuable. Every validation pattern, locale rule, and disambiguation heuristic must meaningfully distinguish "is this type" from "is not this type."

- Prefer precise locale-specific validation over permissive universal patterns. If a type is `designation: locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.
- A validation that confirms 90% of random input is not a validation.
- Expanding locale coverage is the path to accuracy, not relaxing heuristics.

## Communicating with the analyst

The author is the project owner — technically deep, reads fast, decides faster. They're not the implementer in the moment; they want to know *what shifted* and *what to do about it*, not how the sausage was made. `.orbit/STYLE.md` governs the agent's voice for every response. The discipline below applies specifically to **synthesis** — session summaries, finding reports, status updates, anything that compresses technical detail into product-level reasoning.

- **Lead with the headline as user experience, not a stat.** "8 in 100 tables round-trip cleanly" beats "gate_score = 0.0819". Numbers belong inside user experience, not as the user experience.
- **Translate each measurement into what the user lives.** Not "28% fail criterion B" but "FineType says these are emails, then validation rejects 5% of them — which is right?". The second form makes the finding load-bearing for product decisions.
- **Connect findings to the four pillars.** Every result is in service of one of them. Naming the link is how the author prioritises.
- **End with what we don't know yet.** Honesty about scope is more useful than over-claiming. Separate "what the data tells us" from "what the next step will tell us".
- **Plain words.** "Round-trip", "long tail", "honest scope" land. Jargon (cardinality, sibling-context, materialise) must earn its place by reframing the user-level story.
- **One-line for a stakeholder at the end.** If the author could paste a single sentence to someone else and it would carry the finding, the writing is done.

Engineering-internal detail (Rust traces, SQL, perf numbers, error-bucket counts) stays engineering-internal unless it reframes the product-level story. The level matches the question, not the agent's depth of knowledge.

## Architectural direction (settled — do not re-ask)

- **Multi-branch implements the Sense stage** (decision 0041): The Sense→Sharpen pipeline has two stages — Sense (broad classification) and Sharpen (rule-based post-processing). The Sense stage is currently implemented by the multi-branch model; historically it was implemented by the original Sense model. Both remain in code; multi-branch is the v0.6.19 default.
- **Regex header hints deprecated** (decision 0042): Hardcoded regex `header_hint()` rules are deprecated in favour of learned approaches — multi-branch header branch (Model2Vec), sibling-context attention, semantic matching.
- **Value-based rules only** (decision 0048): New disambiguation rules check actual column values, not header metadata.
- **Strength through simplification** (decision 0038): Prefer retraining over adding disambiguation rules. Rules are a last resort.
- **`training_data_addition` retrains need a reachability pre-flight** (spec `2026-05-29-cluster-reachability-scoring`, Path C): v23 closed Failed because three of its six diagnostic-surfaced clusters trained cleanly into their FP targets while pulling 48k v22 geography columns into categorical. The v1 reachability metric mis-scored two of six v23 fixture clusters and did not ship; the redesign is scoped at `output/cluster-reachability/redesign_memo.md` (neighbour-label composition algorithm + worked predictions). Until a v2 metric passes its v23 fixture, no `training_data_addition` retrain may rely on per-cluster FP-rate Met alone — it must additionally include a Sense-distribution pre/post check across the correct_label and its nearest neighbours. Precedent: `output/v23-precision-retrain/relitigation_memo.md`.

## Project state

**Version:** 0.6.19
**Taxonomy:** 240 definitions across 7 domains (container 11, datetime 84, finance 28, geography 25, identity 33, representation 33, technology 26) — all generators pass, 100% alignment.
**Default Sense-stage model:** Multi-branch (sherlock-v22-boundary-relu-s44) inside the Sense→Sharpen pipeline. 5-branch: char+embed+stats+header+validation, ReLU+BatchNorm, val_acc 0.9305. Single forward pass per column. Gated cell-2 vs v19: **−10.4% (Partial band)** on 503k columns — country **−31.5%**, region −12.8%, city −10.2% per `output/v22-direction-review/`. v19 baseline profile eval (369/448 on the 448-row manifest) stands; a fresh v22 profile eval has not been run. Original Sense implementation remains in code as an alternative.
**Codebase:** ~20k lines of Rust across 9 crates. Zero Python dependencies (build + runtime).
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server.

## Current sprint

**Multi-lens diagnostic SHIPPED — next Sense retrain bets follow.** Spec `2026-05-20-gittables-multi-lens-diagnostic` closed 13/13 ACs (commit `31fe887`); it superseded m-19 via MADR 0087 and folded the three m-19 deliverables in as design constraints (realism = corroboration filter; coverage = full-corpus pass; leakage firewall = `file_content_sha256 MOD 2` partition). The v18+ retrain block is lifted.

The diagnostic's deliverable is `eval/gittables/corpus_pass/report.md` — a ranked list of corroborated `(criterion × mechanism)` gaps. Each gap entry carries `action: {training_data_addition, model_retrain, validator_widening, taxonomy_addition, fallback_adjustment}`. Top gaps under `reject_rate_ceil × misclassification` are dominated by v22 false positives (`identity.person.gender_code` on basketball `START_POSITION`, `datetime.offset.utc` on integer columns) — these are the load-bearing input for the next v22+ patch retrain spec. Decisions 0055/0056/0057/0066/0087.

## Eval baseline — gated YDF is canonical

The corpus-pass scoring lens is `ydf_prediction_gated` (per spec `2026-05-26-ydf-validation-gate`). The `--fill-ydf` phase of `scripts/gittables_corpus_pass.py` writes two columns:

- `ydf_prediction` — raw YDF top-1 label (kept for lens-disagreement diagnostics).
- `ydf_prediction_gated` — same prediction with NULLs substituted when fewer than 50% of the column's sample values pass the predicted label's JSON Schema validation. Stops the metric from penalising Sense for disagreeing with demonstrably-wrong YDF labels (msg_id → iso6346, stock_id → mgrs, team-codes → country_code).

When scoring v22 against the gated baseline, v22 lands at **−10.4% cell-2 vs v19 (Partial band)** — up from the noisy baseline's −8.9% (Failed). See `output/ydf-validation-gate/v22_re_baseline.md`. Cell-delta scripts (`compute_v22_cell_deltas.py`) prefer the gated column when present.

## Decision register

48 architectural decisions in `.orbit/choices/` (MADR format). Browse: `ls .orbit/choices/` or Ctrl+B (fzf + glow preview).

## Tier-2 references — load on demand

**Before modifying the engine, model pipeline, taxonomy, MCP, DuckDB extension, training, or eval infrastructure:** Read `docs/ARCHITECTURE.md`.

**Before changing CLI surface, env vars, or build/test commands:** Read `docs/DEVELOPMENT.md`.

**Before promoting a model or cutting a release:** Read `docs/RELEASE.md`.

**For shipped feature history and release notes:** See `CHANGELOG.md`.


<!-- BEGIN ORBIT-STATE INTEGRATION -->
## Orbit-state Substrate

This project uses **orbit-state** as its agent substrate — files-canonical state under `.orbit/` (cards, choices, specs, tasks, memories), with a SQLite index and an MCP server that share the same Rust core. Run `orbit session prime` at session start.

### Quick Reference

```bash
orbit session prime         # Surfaces open specs + recent memories
orbit task ready            # Claimable work (open, no claim)
orbit task show <id>        # Inspect a task
orbit task claim <id>       # Claim a task
orbit task done <id>        # Complete a task
orbit spec list             # Open specs
orbit memory remember "..." # Persist a decision across sessions
orbit memory search <kw>    # Search prior memories
```

### Rules

- Use `orbit` verbs for ALL task and spec tracking — do NOT use TodoWrite, TaskCreate, or markdown TODO lists.
- Run `orbit session prime` at the start of every session.
- Use `orbit memory remember` for persistent knowledge — do NOT use MEMORY.md files.

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File tasks for remaining work** — open new tasks under the active spec for anything that needs follow-up.
2. **Run quality gates** (if code changed) — tests, linters, builds.
3. **Update task status** — mark finished tasks done; append updates on in-progress items.
4. **PUSH TO REMOTE** — this is MANDATORY:
   ```bash
   git pull --rebase
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** — clear stashes, prune remote branches.
6. **Verify** — all changes committed AND pushed.
7. **Hand off** — provide context for next session.

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds.
- NEVER stop before pushing — that leaves work stranded locally.
- NEVER say "ready to push when you are" — YOU must push.
- If push fails, resolve and retry until it succeeds.
<!-- END ORBIT-STATE INTEGRATION -->


@.orbit/METHOD.md


@.orbit/STYLE.md
