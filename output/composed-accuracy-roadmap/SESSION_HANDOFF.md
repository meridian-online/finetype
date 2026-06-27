# Session handoff → next session is SENSE (speed + accuracy)

_Updated 2026-06-27 (end of the Sharpen Tier-A sprint)._

## Headline

**Composed gold 0.832 → 0.847** this session: 8 swap-proof Tier-A Sharpen rules, every one
gold-no-regression + corpus-honest GO. Plus the m2v-witness NO-GO and the corpus-pass value audit
banked earlier. All on main, pushed.

## What this session shipped (all on main, pushed)

| commit | what | composed gold |
|---|---|---|
| 180e18e | 2 gold re-adjudications (filename→windows_path, zip→postal_code) | 0.812→0.814 |
| 8a664e7 | Tier-A: schema_fail_demotion scope +6 | 0.819 |
| f002619 | Tier-A: URL recovery reader | 0.823 (legacy scoring) |
| cbd331c | re-sharpen fast path (`compose_from_sense` + `finetype resharpen`) | — |
| 0791eb2 | fast re-anchored corpus-honest gate (`corpus_honest_gate_fast.sh`) | — |
| 1b6cb52 | m2v-witness pilot NO-GO (cheap per-class embedding witness ⇒ dead-end) | — |
| d4a0456 | corpus-pass value audit (substrate alive, predictions stale) | — |
| **8ac0e19** | **Tier-A Batch 1: #4 #6b #7 #9 #10** | **0.832→0.841 (reframe)** |
| **e3f149c** | **Tier-A Batch 2: #5 #8 #11** | **0.841→0.847 (reframe)** |

**Live composed gold (reframe PRIMARY headline) = 785/927 = 0.847.** Raw Sense ~0.52.
(NB the canonical headline is the `--reframe` number; the legacy non-reframe number reads ~0.82.)

## Tier-A status — clean batch DONE, tail queued

- **Shipped (8 rules):** #4 numeric-residual-fallback, #6b ISBN-10/13 header recovery, #7 long-prose
  entity/word→plain_text, #9 retired state→region hint, #10 decimal→integer IS_FLOAT, #5 epoch
  recovery, #8 full_address whitespace guard, #11 unlocode→postal/unknown. +14 gold, 0 regressions.
  Full result: `output/sharpen-audit/tierA/BATCH1_RESULT.md`.
- **HELD — #6 NPI Luhn** (task t-00016c8f): corpus-wide checksum-guard change + n_unique==1 backstop
  touching ALL checksum types; ~0 new gold (#6b took its isbn cols). Gate it ALONE watching the
  collapse band on isbn/aba/cusip/sedol recall.
- **DEFERRED — #3 utc** (task t-00016c8f): the CORROBORATION_SCOPE add is inert/regressive (confirmed);
  the correct bare-number veto demotes a value-identical gold=utc sibling, so it's gold-blocked until
  that one gold row is re-adjudicated utc→decimal. Bundle with the re-adjudication.

## Sharpen runway still open (separate, riskier classes)

- **Eval cleanup (roadmap ac-2) is higher-ROI than new rules.** Re-pulling the 31 values-missing gold
  columns lands rules ALREADY shipped (#4's idx 74/103/104) for free, and unblocks #3's sibling
  re-adjudication. This is curation/data-ops, not engine work. The backlog's "14 clean
  re-adjudications" count is SUSPECT — the audit's version did not survive value inspection; re-verify
  each before trusting it.
- **Tier-B (#12–#16)** — kill-switched residual-precedence (choice 0096): increment→integer (+8
  claimed), veto_shape_fallback id/epoch arms, binary_vocab full-column. Each needs both-sides
  evidence + BLOCKING corpus-honest + RHH kill switch. Projects ~0.88. Do as a focused standalone pass.
- **Tier-C (#17–#27)** — header-corroboration battlegrounds (country/region/IATA/TLD). The wall:
  backlog predicts ~1-in-4 NO-GO + relocate; needs IANA TLD + IATA lists added to the repo first.

## How to gate a new Sharpen rule fast (built this session)

The reusable raw-Sense cache makes any rule gate in seconds (vs the 9h corpus pass):
```
scripts/gate_from_cache.sh \
  output/sharpen-audit/tierA/sense_cache.tsv \   # 837,625-col cache, model-intrinsic
  <candidate-bin> <baseline-bin> <workdir> <label>
```
Build the candidate (rule ON), keep the prior binary as baseline (rule OFF). Gold:
`score_gold_anchor.py predict … --binary <bin>` then `score … --reframe` (PRIMARY headline 0.847).
Baseline binary preserved at `output/sharpen-audit/tierA/finetype_base_0823` (and per-batch binaries).
ONE gotcha proven this session: composed-path feature rules go in `feature_sharpen` (mod.rs:1437 +
compose_from_sense:1537), NOT `feature_disambiguate` (raw-classify only). Verify against the call graph.

## NEXT SESSION = SENSE (speed + accuracy). Playbook unchanged, baseline now 0.847.

**Step 0 — lock the live baseline (minutes).** Re-score on the shipped binary; PRIMARY reframe = 0.847.

**Step 1 — SPEED (free, no accuracy risk; you want this).**
- Batch-path plumbing: hoist `load_taxonomy`+`compile_validators` out of the per-file loop
  (`profile.rs:243,257`); N files per duckdb spawn. Accuracy-identical, marginal ~halved. Speeds the
  eval loop both threads use — pull it FORWARD.
- Deterministic fast-path before model load (card 0006).
- Single-potion-4M value-encoder revert (−20-40ms). First confirm the 0.769-vs-0.794 composed A/B;
  gate gold-parity + relocation review (0104), not corpus-honest.

**Step 2 — the ACCURACY bet.** The abstaining loose-vs-tight head on the existing static encoder —
take the over-tighten decision (56% of the error mass) OUT of the flat softmax. Latency-neutral,
projects raw Sense ~0.60-0.66. NOT the hierarchical-domain head (that tested worse, 0.685 vs 0.718).
Caveats: (1) unproven recover-vs-relocate (6 prior additive retrains all relocated — gate hard);
(2) composed is rule-bound — measure the **composed pass-through**, not raw Sense; a raw-Sense win
that ties composed is a no-ship.

**Step 3 — after any Sense swap.** Re-fit the model-coupled Tier-B vetoes; rebuild the corpus pass for
fresh mining + prune dead orphans (task t-00012f25). Don't rebuild before the swap.

## Honest scope
Raw Sense caps ~0.57-0.60 (oracle 0.599); 0.75 is unreachable by any model change, and composed is
rule-bound so Sense accuracy gains mostly don't pass through. The bankable composed pp lives in
Sharpen — but Tier-A's clean rules are now spent (0.847), and what's left (Tier-B/C, eval cleanup) is
riskier or curation. So next session's Sense work is a SPEED win (real, free) + an UNPROVEN accuracy
bet, not a guaranteed composed gain. Decide the session's purpose accordingly.

## Guardrails (carry-over)
- Model swap → gold-parity + gold-adjudicated relocation review (0104), NOT corpus-honest GO (0% pass).
- Sharpen rule → `gate_from_cache.sh` (~seconds) or `corpus_honest_gate_fast.sh` (~6min cold).
- Do NOT chase geo/entity over-emission attractors with rules (0-for-6) — retrain's job (t-000133e418).
- Dead branches, do not re-propose: m2v per-class witness, hierarchical-domain head, value-fusion,
  encoder/attention swap, calibrated-abstaining-YDF.

## Key memories to read at session start
`sense-sharpen-session-sequencing-seam`, `sense-stage-ceiling-and-free-latency-wins`,
`corpus-pass-value-and-staleness`, `specialiser-is-the-abstaining-witness`.

## How to continue
1. `orbit session prime`, then `orbit spec show 2026-06-27-composed-accuracy-roadmap` (ac-1..ac-7).
2. Read this file + the memories above.
3. Step 0 first. Then decide: more Sharpen (Tier-B / eval cleanup), or Sense speed + the abstaining bet.
