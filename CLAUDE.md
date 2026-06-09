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
- **`training_data_addition` retrains read `safety_score` as advisory + a Sense-distribution pre/post check** (specs `2026-05-29-cluster-reachability-scoring` Path C, `2026-05-30-reachability-metric-v2` Path C, `2026-05-31-reachability-safety-score` shipped): v23 closed Failed because three of its six diagnostic-surfaced clusters trained cleanly into their FP targets while pulling 48k v22 geography columns into categorical. Two single-score reachability metrics closed Path C (v1: cluster-vs-correct_label distance mis-scored utc as risky; v2: absorption × (1-risk) collapsed cluster ranking into structural correct_label density). v3 ships as an **advisory** column `safety_score` on `corroborated_gaps.parquet`, derived from v2's risk term (`safety = 1 - mean fraction of cluster columns' 100 NN where ydf disagrees AND sense disagrees`). Spec authors read `safety_score` AND cluster size AND Sense-distribution context as inputs to their retrain bet — not as a replacement for the Sense-distribution pre/post check, which **stays mandatory**. Advisory bands: ≥ 0.80 HIGH (safe candidate), 0.50–0.80 MODERATE (requires the pre/post check), < 0.50 LOW (prefer Sharpen rule or taxonomy). Precedent for "advisory diagnostic column shipping while validation accrues from real bets" — the model for future metric work where one labelled outcome is insufficient ground truth. Substrate: `output/v23-precision-retrain/relitigation_memo.md`, `output/cluster-reachability/redesign_memo_v3.md`, `eval/gittables/corpus_pass/report.md` (cluster headers).
- **Every `training_data_addition` retrain runs the destination-drift proxy pre-check BEFORE the overnight run** (spec `2026-06-05-destination-drift-precheck`): `safety_score` is structurally blind to destination drift — it scores a cluster's categorical-direction distinctness, not where the softmax rebalance sends untargeted neighbours. v23 (categorical +529%) and v24 (`geography.coordinate.latitude` 4.3×) both passed their HIGH-safety gate and still exploded an UNTARGETED boundary. So additive hard-negative retrains are 0-for-2 on first-try success, and the post-train check is too late — it spends the overnight run to learn the bet was bad. The pre-check pays that back: `scripts/proxy_pretrain.sh` trains ONE seed for ~10 epochs on the candidate's already-built FTMB (≤ ~20% of the overnight wall-clock), snapshots its Sense distribution on a fixed corpus file list, and runs `scripts/drift_report.py` — the full-label-space gate (calibrated band `--abs-floor 0.0040 --rel-mult 3.0 --direction up`) that **supersedes the snapshot's hand-picked `watch` block** because it measures the whole label vector, so an unwatched boundary cannot hide. NO-GO (exit 1) means do not launch the overnight run. This is **mandatory and ALONGSIDE** the post-train Sense-distribution check — the post-train check **stays mandatory**; the proxy is the cheaper early gate, not a replacement. Cross-model snapshots MUST share one fixed file list (`snapshot_sense_distribution.py --file-list`) — DuckDB reservoir sampling is not seed-reproducible. Substrate: `output/destination-drift-precheck/calibration.md`.
- **Every promotion candidate clears the corpus-honest gate POST-train, BEFORE the `models/default` swap** (spec `2026-06-07-corpus-honest-quality-gate`): three retrains (v22, v23, latdec) each passed every curated/fast instrument and were corpus-scale regressions only the 9-hour full pass caught. latdec was the sharpest — its own FP metric (sense=latitude AND candidate-ydf=decimal) hit ZERO because the candidate's YDF was 100% NULL, while it RELOCATED ~4,417 latitude FPs onto feature-floats (ver, cam, HitRate). The gate closes that: `scripts/corpus_honest_gate.py` scores a candidate's predictions on the ac-01 stratified sample (`scripts/build_stratified_sample.py`, 33,250 files / 6.6% of the corpus, rare labels quota'd up so latitude lands 2,213 cols vs the proxy's ~18) against the **stable baseline GATED-YDF oracle** (read once from `output/ydf-validation-gate/v19_gated.parquet`; column-intrinsic, so the candidate's own ydf is irrelevant). Three bands fire (`over_emit`, `collapse`, `oracle_fp`). It reproduces all four labelled verdicts from the sample alone (v19 GO; v22/v23/latdec NO-GO — see `output/corpus-honest-gate/ac03_four_verdict_reproduction.md`). **The bands are oracle-aware as of 0.6.24** (`output/corpus-honest-gate/refined/oracle_aware_bands.md`): the gate reads the oracle against BOTH ends of each transition A→B, so `oracle_fp` counts only CREATED false positives (oracle confirmed A, refutes B) and `collapse` measures loss of oracle-CONFIRMED support, not raw marginal. This closes a NO-GO false-alarm blind spot — the original bands scored honest abstention (a label the oracle ALREADY refuted, demoted to `unknown` by the validation veto) identically to a real regression, blocking the 0.6.24 precision patch despite it being +858 columns more correct vs the oracle. The four-verdict regression is preserved under the refined bands. **A NO-GO is blocking (H05); a GO is advisory** — proven NO-GO detector, GO-precision unvalidated until a first genuine GO candidate clears it without false alarm (safety_score precedent). **The instrument map — what each can and cannot see:** gold anchor (240 curated cols) = EFFICACY, does the fix land on the hard columns; m-19 (448-row manifest) = curated breadth; destination-drift proxy (1,000-file, 10-epoch) = PRE-train over-emit on COMMON boundaries; **corpus-honest gate = POST-train rare-label RELOCATION incl. the ydf-abstain bucket** — the only instrument that catches a fix that relocates error rather than removes it. gold-anchor + ship-gate is NOT sufficient to promote; corpus breadth is. Updates B08 (memory `finetype-retrain-authority`).

## Project state

**Version:** 0.6.25
**Taxonomy:** 240 definitions across 7 domains (container 11, datetime 84, finance 28, geography 25, identity 33, representation 33, technology 26) — all generators pass, 100% alignment.
**Shipped default Sense-stage model:** `models/default` → **`sherlock-v19-relu-s42`** (multi-branch, inside the Sense→Sharpen pipeline). This is what every `finetype` invocation runs unless `FINETYPE_MODEL` overrides it. v19 baseline profile eval = 369/448 on the 448-row manifest.
**Campaign head (NOT shipped):** `sherlock-v22-boundary-relu-s44` is the newest trained multi-branch model (5-branch: char+embed+stats+header+validation, ReLU+BatchNorm, val_acc 0.9305) and the training-target baseline the corpus diagnostic ran against. **Promotion to default was deferred** (spec `2026-05-26-v22-gated-direction-review`): gated cell-2 vs v19 = **−10.4% (Partial band)** on 503k columns — country **−31.5%**, region −12.8%, city −10.2% per `output/v22-direction-review/` — and it is not published to HuggingFace. So the diagnostic's gaps are *v22* false positives; on the shipped v19 default they may differ (re-baseline before treating them as ground truth — see v24 ac-00). Original Sense implementation remains in code as an alternative.
**Codebase:** ~20k lines of Rust across 9 crates. Zero Python dependencies (build + runtime).
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server.

## Current sprint

**Precision "ceiling" diagnosed as a measurement artifact; first fix shipped (0.6.25).** The flat precision across v22/v23/v24/latdec was the *metric*, not the model: aggregate corpus precision (~0.49 cell-2 vs gated-YDF) is 52% plain integer/decimal and ~0.0003% the rare types the rounds fought over (latitude/longitude/url/utc) — so it counts a bet's collateral damage but never its rare-type gain. Full diagnosis: `output/eval-ceiling-diagnosis/finding.md`; memory `eval-precision-metric-blind-to-rare-targets`.

What changed:
- **Headline gate is now the rare-type scoreboard** (`scripts/build_rare_type_gold.py`, choice 0093): oracle-free, header-anchored FP-rate/recall on the contested types, which *moves* per round where corpus precision is flat. It NEVER overrides a NO-GO corpus-honest gate — the breadth gates still own cross-type regression protection. Aggregate corpus precision is demoted to a context guardrail.
- **Shipped (v0.6.25):** the coordinate header-veto (`header_hint_coord_veto`, choice 0094) — the first fix the scoreboard unblocked. Demotes latitude/longitude false positives on non-coordinate numeric headers (~12× fewer, no recall loss, GO on every instrument). Sharpen-only patch on the v19 model — no model swap.
- **Superseded:** the "next v22+ retrain bet" path. Additive hard-negative retrains are 0-for-N because they optimised against the blind metric. New rounds judge on the **full instrument map together** (rare-type scoreboard + corpus-honest gate + gold-anchor + m-19) — a single GO is not safety (memory `header-hint-deletion-blocked-multi-instrument`).

Open follow-ups (orbit tasks): canonicalise the scoreboard via human review (`scripts/rare_type_gold_review.py`); header-hint deletion is blocked on training-data fortification (0094); re-baseline the multi-lens diagnostic on v19 (its gaps were v22 FPs scored by the blind metric). The diagnostic deliverable `eval/gittables/corpus_pass/report.md` and decisions 0055/0056/0057/0066/0087 remain valid background.

## Eval baseline — gated YDF is canonical

The corpus-pass scoring lens is `ydf_prediction_gated` (per spec `2026-05-26-ydf-validation-gate`). The `--fill-ydf` phase of `scripts/gittables_corpus_pass.py` writes two columns:

- `ydf_prediction` — raw YDF top-1 label (kept for lens-disagreement diagnostics).
- `ydf_prediction_gated` — same prediction with NULLs substituted when fewer than 50% of the column's sample values pass the predicted label's JSON Schema validation. Stops the metric from penalising Sense for disagreeing with demonstrably-wrong YDF labels (msg_id → iso6346, stock_id → mgrs, team-codes → country_code).

When scoring v22 against the gated baseline, v22 lands at **−10.4% cell-2 vs v19 (Partial band)** — up from the noisy baseline's −8.9% (Failed). See `output/ydf-validation-gate/v22_re_baseline.md`. Cell-delta scripts (`compute_v22_cell_deltas.py`) prefer the gated column when present.

**Aggregate corpus precision is context-only, NOT the headline (choice 0093).** The ~0.49 cell-2 number is structurally blind to rare-type fixes — its denominator is 52% plain integer/decimal, while the rounds' battlegrounds (latitude/longitude/url/utc) are 0.0003% of scored columns, and it counts only a bet's common-type collateral, never its rare-type gain (the "ceiling" that flatlined v22–v24/latdec). Root cause + evidence: `output/eval-ceiling-diagnosis/finding.md`; memory `eval-precision-metric-blind-to-rare-targets`. The **headline progress signal and fast pre-promotion check is the rare-type scoreboard** (`scripts/build_rare_type_gold.py`): oracle-free, header-anchored FP-rate/recall on the contested types, which *moves* per round where corpus precision is flat. **Gate precedence (0093): the scoreboard NEVER overrides a NO-GO corpus-honest gate (blocking, H05) or a common-type regression on corpus precision — it can only withhold or surface, never greenlight past a breadth-gate failure.** Promotion order: gold-anchor → drift proxy (pre-train) → rare-type scoreboard (headline) → corpus-honest gate (blocking) → swap. Scoreboard GO-precision is unvalidated until its gold set is human-checked; until then it is a relative comparator + NO-GO early-warning. Header-corroboration for value-identical boundaries is **proposed** (choice 0094, amends 0048 — awaiting author review).

## Decision register

48 architectural decisions in `.orbit/choices/` (MADR format). Browse: `ls .orbit/choices/` or Ctrl+B (fzf + glow preview).

## Code-review & audit tooling

The review model is **audit-before-edit + verify**, not PR-gated (memory `finetype-cron-no-pr-merge-policy`): direct push to main, quality held by B07 consumer-graph audit + H02 post-edit verify + H03 load-bearing-edit-without-audit halt + H10 binary-push halt + green CI. Local gates: pre-commit (`fmt`) → pre-push (`fmt` + `clippy -D warnings`, the precise CI mirror) → CI (adds `cargo test`, taxonomy `check`, CLI smoke, **`cargo machete`** unused-deps).

To run the **B07 consumer-graph audit** before a load-bearing edit, use the **codegraph** MCP (`codegraph_impact` / `codegraph_callers`) — enumerates consumers in 1–5 calls. Restart the agent to load the `codegraph_*` tools; `.codegraph/` is a gitignored local index (`codegraph sync` after edits). **Caveat:** codegraph is a *static* call-graph — Rust trait dispatch and the `#[test]` harness are invisible, so "no callers" is NOT dead-code truth (clippy + ripgrep stay canonical; codegraph is a fast first pass to verify against). Recipes + limits: memory `codegraph-usage-and-limits`.

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
