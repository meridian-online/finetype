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
- **Strength through simplification** (decision 0038): Prefer retraining over adding disambiguation rules. Rules are a last resort — **except residual-precedence decisions, which are rule-shaped by measurement** (decision 0096, 2026-06-12): six consistent results (v23, v27 proxy ×2, fusion gate ×2, deferral selector) prove `categorical`/`alphanumeric_id`-style "no tighter type fits" decisions cannot be trained into a flat softmax or additive value head — they become universal attractors. Such rules must be value-based (0048), gate-shipped with both-sides evidence, and ledgered in the deterministic-layer audit with kill switches.
- **General value-fusion Sense replacement: abandoned** (spec `2026-06-08-late-fusion-sense-classifier`, closed 2026-06-12, fork resolved option 3): the ship gate ran 2026-06-08 — NO-GO twice (label-space collapse at corpus breadth), and the learning-to-defer variant banked no win. Findings preserved in `output/late-fusion/`; any future architecture bet must design precedence semantics in and beat the rule baseline through the same gates. There is no 0.7.0 fusion candidate.
- **`training_data_addition` retrains read `safety_score` as advisory + a Sense-distribution pre/post check** (specs `2026-05-29-cluster-reachability-scoring` Path C, `2026-05-30-reachability-metric-v2` Path C, `2026-05-31-reachability-safety-score` shipped): v23 closed Failed because three of its six diagnostic-surfaced clusters trained cleanly into their FP targets while pulling 48k v22 geography columns into categorical. Two single-score reachability metrics closed Path C (v1: cluster-vs-correct_label distance mis-scored utc as risky; v2: absorption × (1-risk) collapsed cluster ranking into structural correct_label density). v3 ships as an **advisory** column `safety_score` on `corroborated_gaps.parquet`, derived from v2's risk term (`safety = 1 - mean fraction of cluster columns' 100 NN where ydf disagrees AND sense disagrees`). Spec authors read `safety_score` AND cluster size AND Sense-distribution context as inputs to their retrain bet — not as a replacement for the Sense-distribution pre/post check, which **stays mandatory**. Advisory bands: ≥ 0.80 HIGH (safe candidate), 0.50–0.80 MODERATE (requires the pre/post check), < 0.50 LOW (prefer Sharpen rule or taxonomy). Precedent for "advisory diagnostic column shipping while validation accrues from real bets" — the model for future metric work where one labelled outcome is insufficient ground truth. Substrate: `output/v23-precision-retrain/relitigation_memo.md`, `output/cluster-reachability/redesign_memo_v3.md`, `eval/gittables/corpus_pass/report.md` (cluster headers).
- **Every `training_data_addition` retrain runs the destination-drift proxy pre-check BEFORE the overnight run** (spec `2026-06-05-destination-drift-precheck`): `safety_score` is structurally blind to destination drift — it scores a cluster's categorical-direction distinctness, not where the softmax rebalance sends untargeted neighbours. v23 (categorical +529%) and v24 (`geography.coordinate.latitude` 4.3×) both passed their HIGH-safety gate and still exploded an UNTARGETED boundary. So additive hard-negative retrains are 0-for-2 on first-try success, and the post-train check is too late — it spends the overnight run to learn the bet was bad. The pre-check pays that back: `scripts/proxy_pretrain.sh` trains ONE seed for ~10 epochs on the candidate's already-built FTMB (≤ ~20% of the overnight wall-clock), snapshots its Sense distribution on a fixed corpus file list, and runs `scripts/drift_report.py` — the full-label-space gate (calibrated band `--abs-floor 0.0040 --rel-mult 3.0 --direction up`) that **supersedes the snapshot's hand-picked `watch` block** because it measures the whole label vector, so an unwatched boundary cannot hide. NO-GO (exit 1) means do not launch the overnight run. This is **mandatory and ALONGSIDE** the post-train Sense-distribution check — the post-train check **stays mandatory**; the proxy is the cheaper early gate, not a replacement. Cross-model snapshots MUST share one fixed file list (`snapshot_sense_distribution.py --file-list`) — DuckDB reservoir sampling is not seed-reproducible. Substrate: `output/destination-drift-precheck/calibration.md`.
- **Every promotion candidate clears the corpus-honest gate POST-train, BEFORE the `models/default` swap** (spec `2026-06-07-corpus-honest-quality-gate`): three retrains (v22, v23, latdec) each passed every curated/fast instrument and were corpus-scale regressions only the 9-hour full pass caught. latdec was the sharpest — its own FP metric (sense=latitude AND candidate-ydf=decimal) hit ZERO because the candidate's YDF was 100% NULL, while it RELOCATED ~4,417 latitude FPs onto feature-floats (ver, cam, HitRate). The gate closes that: `scripts/corpus_honest_gate.py` scores a candidate's predictions on the ac-01 stratified sample (`scripts/build_stratified_sample.py`, 33,250 files / 6.6% of the corpus, rare labels quota'd up so latitude lands 2,213 cols vs the proxy's ~18) against the **stable baseline GATED-YDF oracle** (read once from `output/ydf-validation-gate/v19_gated.parquet`; column-intrinsic, so the candidate's own ydf is irrelevant). Three bands fire (`over_emit`, `collapse`, `oracle_fp`). It reproduces all four labelled verdicts from the sample alone (v19 GO; v22/v23/latdec NO-GO — see `output/corpus-honest-gate/ac03_four_verdict_reproduction.md`). **The bands are oracle-aware as of 0.6.24 and `over_emit` is composition-aware as of 0.6.29** (`output/corpus-honest-gate/refined/oracle_aware_bands.md`, `refined/composition_aware_over_emit.md` — confirmed-correct growth is netted out of the over_emit ratio so stacked honest fixes cannot false-alarm; relocation verdicts preserved incl. a live negative control): the gate reads the oracle against BOTH ends of each transition A→B, so `oracle_fp` counts only CREATED false positives (oracle confirmed A, refutes B) and `collapse` measures loss of oracle-CONFIRMED support, not raw marginal. This closes a NO-GO false-alarm blind spot — the original bands scored honest abstention (a label the oracle ALREADY refuted, demoted to `unknown` by the validation veto) identically to a real regression, blocking the 0.6.24 precision patch despite it being +858 columns more correct vs the oracle. The four-verdict regression is preserved under the refined bands. **A NO-GO is blocking (H05); a GO is advisory** — proven NO-GO detector, GO-precision unvalidated until a first genuine GO candidate clears it without false alarm (safety_score precedent). **The instrument map — what each can and cannot see:** gold anchor (240 curated cols) = EFFICACY, does the fix land on the hard columns; m-19 (448-row manifest) = curated breadth; destination-drift proxy (1,000-file, 10-epoch) = PRE-train over-emit on COMMON boundaries; **corpus-honest gate = POST-train rare-label RELOCATION incl. the ydf-abstain bucket** — the only instrument that catches a fix that relocates error rather than removes it. gold-anchor + ship-gate is NOT sufficient to promote; corpus breadth is. Updates B08 (memory `finetype-retrain-authority`).

## Project state

**Version:** 0.6.35
**Taxonomy:** 244 definitions across 7 domains (container 11, datetime 86, finance 28, geography 25, identity 33, representation 32, technology 29) — all generators pass, 100% alignment. v0.6.36 changes: added three types mined from the `plain_text` residual (`technology.filesystem.windows_path`, `technology.internet.message_id`, `technology.code.qualified_name`; spec 2026-06-19-plain-text-type-discovery); added two zoneless ISO datetime leaves (`datetime.timestamp.iso_seconds`, `datetime.timestamp.iso_milliseconds`; spec 2026-06-19-zoneless-iso-datetime-leaves); retired `representation.discrete.categorical` as a leaf (choice 0102 — now the orthogonal enum-domain property). The three plain_text types are taxonomy/validation-live but not predicted until the next retrain (model is 240-dim); the two datetime leaves ARE live at `profile` time via the deterministic `datetime_format_refinement` Sharpen rule (the detector now requires a trailing Z on the zoned leaves and routes zoneless values to the new siblings). Categorical's internal sentinel + finalize→`word` remap are unchanged.
**Shipped default Sense-stage model:** `models/default` → **`sherlock-v19-relu-s42`** (multi-branch, inside the Sense→Sharpen pipeline). This is what every `finetype` invocation runs unless `FINETYPE_MODEL` overrides it. v19 baseline profile eval = 369/448 on the 448-row manifest.
**Campaign head (NOT shipped):** `sherlock-v22-boundary-relu-s44` is the newest trained multi-branch model (5-branch: char+embed+stats+header+validation, ReLU+BatchNorm, val_acc 0.9305) and the training-target baseline the corpus diagnostic ran against. **Promotion to default was deferred** (spec `2026-05-26-v22-gated-direction-review`): gated cell-2 vs v19 = **−10.4% (Partial band)** on 503k columns — country **−31.5%**, region −12.8%, city −10.2% per `output/v22-direction-review/` — and it is not published to HuggingFace. So the diagnostic's gaps are *v22* false positives; on the shipped v19 default they may differ (re-baseline before treating them as ground truth — see v24 ac-00). Original Sense implementation remains in code as an alternative.
**Codebase:** ~77k lines of Rust across 9 crates (measured 2026-06-10). Zero Python dependencies in build + runtime; the eval/training tooling under `scripts/` is ~110 Python files, not shipped.
**Ingestion (v0.6.32, choice 0100):** `profile` and `validate` read CSV/Parquet by **shelling out to the external `duckdb` CLI** (`read_csv_input` in `crates/finetype-cli/src/profile_io.rs`; `-csv` output mode, `.nullvalue ''` pinned). The `duckdb` CLI is a **hard runtime dependency** (on PATH; clear actionable error otherwise; Homebrew `depends_on "duckdb"`). It is a shell-out, NOT a link — the release binary is unchanged across platforms (no `libduckdb` compile, no Windows/MSVC risk).
**Distribution:** GitHub releases (Linux x86/arm, macOS x86/arm, Windows), Homebrew tap, crates.io (core + model), DuckDB community extension (v0.2.0 merged), MCP server.

## Current sprint

**The gold corpus shipped — FineType's accuracy is now scored against human-verified truth (card 0019, spec `2026-06-10-human-verified-gold-corpus`, choice 0095).** 931 verified columns (`eval/gold/gold_corpus_v1.tsv`), leakage-firewalled (`make leakage-guard`), per-row provenance (anchor / lens-consensus / llm-2panel / author tiers). **v19 baseline = 610/931 = 0.655 (95% CI 0.624–0.685)** — the number every candidate must beat (`output/gold-corpus/baseline_v19.md`).

**First gold-gated fix shipped (0.6.27):** the postal header-veto (`header_hint_postal_veto`, spec `2026-06-10-postal-header-veto`) — postal precision 0.133→0.667 on gold, recall held, verified headline 0.655→0.682, corpus-honest gate GO. Second consecutive ship on the 0094 header-corroboration pattern.

**Fourth gold-gated fix shipped (0.6.29):** the word-vocab override (`R32 text_vocab_override`, spec `2026-06-12-text-vocab-override`) — a `word`-labelled column repeating a small vocabulary profiles as categorical. Gold 0.711→**0.719**, categorical R 0.396→0.465 with P up (0.870), zero regressions. The corpus-honest gate earned its keep twice here: round 1 killed the broad text-family variant (5,867 oracle-refuted entity/plain moves gold could not see), and round 2's false-alarm exposed the over_emit stacking blind spot, fixed by the author-accepted composition-aware band.

**Third gold-gated fix shipped (0.6.28):** the veto shape-fallback (`veto_shape_fallback`, spec `2026-06-12-veto-shape-fallback`) — when the validation veto hard-rejects a Sense assertion, value shape decides between alphanumeric_id (high-cardinality letter+digit) and categorical (small repeating vocab) instead of unconditional unknown. Gold headline 0.682→**0.711**, alphanumeric_id recall 0.111→0.593 (P held 0.842), zero regressions, corpus-honest GO with alnum's oracle-contradicted count net NEGATIVE. Born from the v27 retrain's Failed-informative close: **categorical is a residual category and cannot be trained as a flat-softmax shape class** (memory `categorical-is-a-residual-category`; the `COLUMN_LEVEL_TYPES` guard in `prepare_multibranch_data.py` is load-bearing). Also discovered: per-type distilled caps were cosmetic for v4-format training (memo `2026-06-12-ordered-distilled-cap-bug`).

**The v19 first verified reading (2026-06-10 SNAPSHOT — historical, DO NOT treat as the current target list):**
- **postal_code precision 0.133**, **state_code P=R=0.000**, **categorical recall 0.390** at support 100, **alphanumeric_id recall 0.113**, city over-emit (P=0.667, R=1.0). Coordinates/dates/decimals/booleans held at P≈1.0.
- ⚠️ These are v19-baseline numbers and many have since been fixed (e.g. postal→0.667, state_code→0.857, alphanumeric_id→0.667, categorical→0.465). **For the CURRENT per-type gap list, regenerate from the latest gold report** (`scripts/score_gold_anchor.py score …` → `report_*.md` per-type table), never from this snapshot. Gold headline has moved 610→728 across the shipped fixes.
- **The instrument audit** (`output/gold-corpus/instrument_audit.md`, author-accepted 2026-06-10): on the 809 contested-ground columns, v19 = 68.2% vs the gated-YDF oracle's 57.9% precision-when-asserting — the model was better than its referee where promotions were decided. The four failed retrains were judged by that referee.

Open follow-ups: card 0019 scenario 5 is live — deletions/simplifications gate on a no-regression gold run, unblocking the deferred simplification work (giant-file splits, dead-rule removal, instrument-tail cleanup); header-hint deletion still blocked on training-data fortification (0094); re-baseline the multi-lens diagnostic on v19; taxonomy-gap ledger seeds a future taxonomy discovery (see choice 0095 evolution policy). The scoreboard canonicalisation follow-up is DONE (validated 99.0%, `output/eval-ceiling-diagnosis/scoreboard_canonicalisation.md`).

## Eval baseline — the gold corpus is canonical (choice 0095)

**The headline accuracy eval is the gold corpus**: `eval/gold/gold_corpus.tsv` (931 human-or-calibrated-verified columns, per-row provenance, leakage-firewalled; single canonical file — version history is git + the per-row `provenance` string, which records each label's prior value; the v1/v2 filename scheme was retired 2026-06-16) scored by `scripts/score_gold_anchor.py` (`build-gold` → `predict` → `score`; per-type precision/recall with Wilson CIs + one headline number). v19 baseline = 0.655 (CI 0.624–0.685). Gold labels are append-only facts; expansion is emission-driven each promotion round; the fixture records its taxonomy version (full evolution policy in choice 0095).

**Gold re-adjudication (2026-06-16, spec `2026-06-16-mixed-teacher-gold-readjudication`):** mixed-panel (Opus/Sonnet/Haiku blind + Opus adversarial) re-adjudication of the heuristic gold tiers, applied in place to `eval/gold/gold_corpus.tsv` (single canonical file; git + per-row provenance is the append-only record). Phase 1 (`ac-03` tier): 24 label changes (23 panel + 1 author override `idPedido`→unix_milliseconds) + 23 author-confirmed upgrades; shipped model **738→742/931 = 0.797**. Phase 2 (148 header-heuristic + 47 datetime-rerun): 186 CONFIRM + 9 PROPOSE applied (all `datetime.date.iso → datetime.timestamp.sql_standard`, conf 0.95+ — timestamps mislabelled as dates; author-accepted), 0 contested (the constrained datetime vocab fixed the phase-1 noise). **Net across both phases: 33 label corrections, shipped model 738→745/931 = 0.800** (membership identical, leakage unaffected). Integrity held throughout (control confirm 82%/88%; corrections skew away from the model — the modest headline shift proves re-adjudication cleans gold rather than inflating it). The eval harness (`eval_rule.sh`) scores against `gold_corpus.tsv`. Spec closed; unresolvable heuristic columns (no source values on disk) are the only remainder.

**Promotion order (0095, + representative band per spec 2026-06-18-representative-accuracy-gate):** gold-anchor (efficacy) → drift proxy (pre-train) → **gold corpus accuracy + rare-type scoreboard** (headline; scoreboard validated at 99.0%) → **representative accuracy (ADVISORY)** → corpus-honest gate (**blocking**, H05 — the only relocation detector, role unchanged) → swap. No headline ever overrides a blocking NO-GO.

**The representative band** (`eval/repr/representative_corpus.tsv`, 260 uniform-random non-trivial corpus columns, panel tier; scored by `score_gold_anchor.py … --reframe`, v19 baseline **0.691**, CI 0.632–0.744) is the only instrument that measures accuracy on a column drawn at random from production — gold is curated-*hard*, the drift proxy measures distribution-not-correctness, and the corpus-honest gate measures rare-label *relocation*. It is **ADVISORY, never blocking**: at n=260 the CI is ≈±6pp, so it cannot separate marginal candidates, and its panel-tier labels carry a known ~3pp anti-model bias. That bias is common-mode (every candidate scored against the same labels), so the trustworthy signal is the **candidate-vs-v19 delta**, not the absolute — the advisory flag fires when a candidate's representative headline drops more than the CI below the v19 baseline. gold + corpus-honest stay the blocking gates. Rationale + tier decision: `output/representative-accuracy-gate/ac00_tier_and_band_policy.md`.

**Gated-YDF is demoted to a mining/corroboration lens (audit-measured, author-accepted 2026-06-10):** on contested ground it is wrong in 42.1% of its assertions (`output/gold-corpus/instrument_audit.md`) — it must not adjudicate per-column correctness in any gate or metric without a gold cross-check. The mechanics remain (corpus-pass `--fill-ydf` writes `ydf_prediction` and `ydf_prediction_gated`, NULL-substituted below a 50% schema-validation pass rate, per spec `2026-05-26-ydf-validation-gate`): the gated parquet is still the corpus-honest gate's transition substrate and the candidate-generation workhorse — its per-label counts are directional, not exact. Aggregate corpus precision (~0.49 cell-2) is retired as a decision input (context guardrail only; blindness evidence: `output/eval-ceiling-diagnosis/finding.md`). Header-corroboration for value-identical boundaries remains **proposed** (choice 0094, amends 0048 — awaiting author review).

## Decision register

96 architectural decisions in `.orbit/choices/` (MADR format). Browse: `ls .orbit/choices/` or Ctrl+B (fzf + glow preview).

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
