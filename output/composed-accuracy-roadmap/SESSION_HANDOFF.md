# Session handoff → next session is SENSE (speed + accuracy)

_Updated 2026-06-27 (end of the Sharpen/eval/specialiser session)._

## What this session banked (all on main, pushed)

| area | result |
|---|---|
| Gold re-adjudication ×2 | `filename`→windows_path, `zip`→postal_code. Composed 0.812→0.814 |
| Tier-A Sharpen ×2 | `schema_fail_demotion` scope +6; URL recovery reader. Composed →**0.823**, 0-regress |
| Re-sharpen fast path | `ColumnClassifier::compose_from_sense` + `finetype resharpen` (real Sharpen on cached Sense, 99.4% parity) |
| Fast re-anchored corpus-honest gate | `corpus_honest_gate_fast.sh` (m2v8m rule-OFF→ON, ~6min, constructive) |
| **m2v-witness pilot — NO-GO** | spec closed. Cheap per-class embedding witness carves only on CLOSED-vocab types (3 of 4 already Sharpen-owned); the open residual is uncrackable. Folded `country` into the gazetteer backlog. Memory `specialiser-is-the-abstaining-witness` |
| **Corpus-pass value audit** | substrate ALIVE as value-cache + frozen oracle (powers the only blocking relocation gate); its `sense_prediction` column is STALE v19; ~⅓ of mining scripts are dead orphans. Memory `corpus-pass-value-and-staleness`; task t-00012f25 |

**Live composed gold ≈ 0.823.** Raw Sense ~0.52.

## The next-session playbook (Sense: speed + accuracy)

Sequencing is settled (memory `sense-sharpen-session-sequencing-seam`): clean the ruler → speed
(free) → the accuracy bet → re-fit the co-adapted layer. In order:

**Step 0 — lock the live baseline (minutes, do before anything).**
Re-score on live 0.6.37 (roadmap ac-1). The `errors_enriched.tsv` snapshot is stale; ~8 rows
(idx 27,29,63-67,78) are already correct in live, so the true baseline is ~0.821, not 0.812.
Both threads measure against gold — start from the real number.

**Step 1 — SPEED (free, no accuracy risk, the user wants this).**
- Batch-path plumbing (roadmap ac-6): hoist `load_taxonomy`+`compile_validators` out of the
  per-file loop (`profile.rs:243,257`); N files per duckdb spawn. Accuracy-identical, per-file
  marginal ~halved. Also speeds the eval loop both threads use — pull it FORWARD.
- Deterministic fast-path before model load (card 0006).
- Single-potion-4M value-encoder revert: Sense-flat, −20-40ms load. **First confirm the confounded
  0.769-vs-0.794 composed A/B** — it's a model change, so gate gold-parity + relocation review (0104),
  not corpus-honest.

**Step 2 — the ACCURACY bet (the real experiment, eyes open).**
Two-stage / **abstaining loose-vs-tight head** on the EXISTING static encoder — take the over-tighten
decision (56% of the error mass) OUT of the flat softmax. Latency-neutral. Projection raw Sense
~0.60-0.66. **NOT the hierarchical-domain head** (that tested worse, 0.685 vs 0.718 — do not
confuse them). Two honest caveats, both load-bearing:
  1. UNPROVEN recover-vs-relocate — 6 prior additive retrains all relocated. Gate hard.
  2. May be **Sharpen-redundant at composed** — composed is rule-bound, so measure the **composed
     pass-through**, not raw Sense. A raw-Sense win that ties composed is a no-ship.

**Step 3 — after any Sense swap lands.**
- Re-fit the co-adapted Tier-B vetoes (veto-misfire / decline rules) against the new model — they
  were bracketed this session precisely because they're model-coupled.
- Rebuild the corpus pass against the shipped model for fresh mining + prune the dead orphans
  (task **t-00012f25**). Do NOT rebuild before the swap — it would re-stale within a session.

## Honest scope — go in knowing this

Raw Sense is near its ceiling: caps ~0.57-0.60 for this architecture, oracle 0.599
(memory `sense-stage-ceiling-and-free-latency-wins`). A 0.75 raw-Sense target is above the oracle —
unreachable by any model change. And composed is **rule-bound**, so Sense accuracy gains mostly
don't pass through. **The bankable composed-accuracy lever is still Sharpen, not Sense** — the
Tier-A/B sprint is paused mid-way (2 of ~11 Tier-A rules shipped; remaining: utc value-shape veto,
NPI Luhn, ISBN-10, numeric residual fallback — each gates in ~6min). So:
- want **composed pp** → resume the Sharpen Tier-A/B sprint (the proven, 4-for-0 path).
- want **speed + a Sense architecture bet** → Step 1 + Step 2 above.
The two are not the same lever. Decide which the session is for.

## Guardrails (carry-over)
- Model swap → gold-parity + gold-adjudicated relocation review (choice 0104). NOT a corpus-honest
  GO (structurally unpassable by any retrain — 0% pass rate).
- Sharpen rule → `corpus_honest_gate_fast.sh <wd> <cur-bin> <rule-OFF-bin> <label>` (~6min).
- Do NOT chase the geo/entity over-emission attractors with rules (0-for-6) — that's the retrain's
  narrow job (task t-000133e418), gold-invisible corpus-scale warts.
- Dead branches, do not re-propose: m2v per-class witness (this session), hierarchical-domain head,
  value-fusion, encoder/attention swap, calibrated-abstaining-YDF.

## How to continue
1. `orbit session prime`, then `orbit spec show 2026-06-27-composed-accuracy-roadmap` (ac-1..ac-7).
2. Read this file + memories `sense-sharpen-session-sequencing-seam`,
   `sense-stage-ceiling-and-free-latency-wins`, `corpus-pass-value-and-staleness`.
3. Step 0 first. Then decide: Sharpen pp, or Sense speed+bet.
