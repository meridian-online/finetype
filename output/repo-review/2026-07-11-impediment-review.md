# Grading Your Own Homework — FineType repository review

**Date:** 2026-07-11 · **Goal under review:** make data profiling as **fast and accurate** as possible ·
**Question:** which decisions in this repo are getting in the way of that goal?

> **Headline.** The engine is at ceiling *where it is measured* — but it is measured against a curated
> fixture drawn from the model's own training corpus, refereed by a model it retired. This is not a pile
> of latent bugs (three candidate findings were killed on inspection). It is a **strategy-and-instrument**
> critique: nearly every surviving finding is a **velocity or blindness** problem, not a demonstrated loss
> of accuracy.

## The goal, graded on three axes

| Axis | Grade | Reading |
|---|---|---|
| **Accurate** | **A — self-graded** | Head of the distribution is solved (country 0.96, latitude 0.98, city/dates/coords/url ~0.9–1.0). But the ruler is a curated-hard fixture drawn from the same gittables corpus the model trains on. |
| **Fast** | **— unmeasured** | Half the stated goal has *zero* committed number anywhere: no latency step in CI or the gate stack, the one bench harness (`bench_infer.py`) is never invoked, "~free" is an assertion. |
| **Velocity** | **C — throttled** | Every ship pays a tax to a blocking gate whose baseline is a retired model. The Sense model has been frozen since 2026-06-24; all new accuracy work is demote-only rules. |

## The verdict — the apparatus optimises "prove we didn't regress"

The through-line: the one lever proven to find real accuracy holes — external, third-party data — was run
**once** and archived. The company-reference audit fed in genuine outside tables (GLEIF, SEC EDGAR, NYC DOB)
and exposed ticker / NAICS / org-name / WKT failures that drove **~7 ships in a week**, then the builder went
to `scripts/archive/` and the snapshots froze at 2026-06-10. Meanwhile gold is curated-*hard*, drawn from the
same corpus the model trains on; the only production-random ruler (`representative`, n=260) is advisory,
single-LLM-labelled and frozen; and "fast" is measured nowhere. **You cannot see the goal you are optimising.**

That is the Meridian pillars misfiring: an analyst-visible hole (a column returned `unknown`) is measured
nowhere *(pillar 1)*; the gate/gold/repr stack has sprawled into a fixture-defender *(pillar 2)*; the model is
frozen at 244 classes while the taxonomy grows to 247, each new type a hand-written recovery guard *(pillar 3)*.

---

## Six moves — ranked by leverage, one action each

1. **Stand up a small *rotating* external-data band.** Fresh third-party tables the model never trained on
   (gov open-data, filings, domain corpora), re-drawn each promotion round, reported advisory. It is the only
   instrument that measures the actual goal — and the audit proved external data finds real holes at ~10× the
   yield of gittables gold-polishing. *(c4)*
2. **De-fossilise the gate.** Remove the `v19_gated.parquet` default (`scripts/corpus_honest_gate.py:63`), make
   `--baseline` required, keep one standing current-default reference pass. The blocking gate silently defaults
   to a *retired* 240-label model's oracle; it false-NO-GO'd W1/W2a (which flipped to GO fresh-vs-fresh) and
   taxes every rule ship with a manual re-run. Keep it blocking; stop attributing the model freeze to it. *(t1, t6)*
3. **Run the two genuinely-free model experiments the team's own brief named.** A `--logit-adjust-tau` A/B
   (predict-time, no retrain, implemented-never-enabled) and a single-seed two-stage / abstaining-head pilot —
   scored on **composed**, not raw Sense. 56% of Sense error is the flat-softmax over-tighten pathology; this
   settles whether "Sense at its ceiling" is true for the product number, for near-zero cost.
   *(t3, t5 · `output/next-train-research/RESEARCH.md`)*
4. **Commit one speed number.** Run the orphaned `scripts/bench_infer.py` against the dual-encoder default vs
   the prior baseline; commit cols/sec + peak RSS to `output/`. Converts "~free" from assertion to evidence and
   re-admits "fast" to the definition of better. Defer any blocking CI gate until a number shows a regression
   worth gating. *(t9)*
5. **Instrument what you ship.** Add ~5–10 verified positives + hard-negatives to gold at each guard ship
   (choice 0095 already says to); add a per-type *recall / under-emission* column to the scoreboard; add an
   aggregate `unknown`/`word` abstention-rate line to the promotion report. You currently quote guard efficacy
   ("npi −87%") off gated-YDF — the instrument you call "42% wrong." *(t7, t4, c3)*
6. **Cheap hygiene.** Extract the byte-identical Sharpen firing tail into one `run_sharpen()` called by all
   three entry points + a native-vs-compose parity test (today the gate can score a *different* pipeline than
   production ships, `mod.rs:1540`). Fix the stale "model is 240-dim" line in CLAUDE.md to 244. *(t10, t11)*

---

## The findings, marked (13 survivors)

Each was given its strongest defence (steelman) before judgement. Almost all were **marked down** from "high" —
the drama stripped out, the real kernel kept.

| # | Mark | Axis | Finding | The action |
|---|---|---|---|---|
| t1 | Marked down | Velocity | The one blocking gate is refereed by a retired model's oracle (`corpus_honest_gate.py:63`). Real footgun + per-ship tax — but it did *not* freeze the model (swaps bypass via choice 0104; the shipped default itself shipped while NO-GO). | Require `--baseline`; keep a standing current-default reference. Stop attributing the freeze to it. |
| t2 | Marked down | Velocity | The gate's blind referee + `over_emit` band select for "rules-only." The credit instrument June prescribed (`build_rare_type_gold.py`) was built and shelved. But the rules pivot is independently grounded, not "manufactured." | Stop citing the gate as evidence for a Sense ceiling; wire in the shelved rare-type credit side if retrains revive. |
| t3 | Marked down | Velocity | "Sense is at its ceiling" conflates encoder-capacity with model-capacity; June called the plateau a measurement artifact. But no *current* accuracy was lost — banked `attneg2-s44` prevented a real `cpt` regression. | Build correction #2 — a value-verified attractor-column fixture — and gate the round-4 247 retrain with it. |
| t4 | Marked down | Velocity | Demote-only can't fix recall & strands the taxonomy (0038 a dead letter). But the pipeline *is* 247-ready (`m2v8m-247-config.json` exists) and 6 recovery guards relabel-up. Gap: you can't *see* where guards stop paying. | Add a per-type recall column; launch the drafted round-4 247 retrain only if recall is shown to be the binding constraint. |
| t5 | Marked down | Velocity | The cures were coded/GO'd then never trained. Overstated on specifics (head is not a stub; encoder built 3× and NO-GO'd for documented reasons). Survives: shipped `head_type` is Flat and `--logit-adjust` is implemented but never A/B'd. | Run the two free levers once, scored on composed — settle the one load-bearing unknown. |
| t6 | Marked down | Velocity | The gate taxes every ship for a verdict that no longer decides. "Blocks nothing" is refuted (it hard-blocks the unbounded class). But a demote-only guard *can't* relocate mass, yet runs the full 33k relocation gauntlet. | Add a demote-only fast lane (gold-flat + per-column trace); keep the full gate for retrains. |
| t7 | Marked down | Velocity | The headline eval is blind to the guard campaign it grades. Gold covers 21% of types, ~0 rows for every guard since June, so "gold flat" checks unrelated labels. Efficacy is read off the instrument you call blind. | Add verified positives/negatives per guard; label the gated-YDF delta "directional," not efficacy. |
| t8 | Marked down | Accurate | The production-random ruler is single-LLM, advisory and frozen. Both headlines *are* reported and the gap *is* reconciled (gold 0.855 vs production 0.708). But `representative` is soft-labelled, frozen, n=260, upgrade un-actioned. | Execute the documented upgrade: blind multi-model panel, persist flips, grow to ~600 cols. |
| t9 | Marked down | Fast | "As fast as possible" has no instrument anywhere. Factual core undisputed: zero speed instrument in gate stack or CI. Downside bounded (static inference, once-per-column guards) — a hygiene gap, not a demonstrated regression. | Commit one number (move 4); defer a blocking CI gate until it shows a regression worth gating. |
| t10 | Marked down | Velocity | Sharpen precedence is implicit source-order, hand-synced across 3 copies — one (`compose_from_sense`) is exactly what the gate scores, with no parity test. If a copy drifts, the gate scores a different pipeline than ships. | Extract one shared `run_sharpen()` tail + a native-vs-compose parity assertion. |
| t11 | Stands (narrow) | Velocity | The decision register is un-auditable from the repo, buried in a 32KB prompt. Cron/no-PR + private-register strands are defensible-as-designed. Stands cleanly: CLAUDE.md has a stale 240-dim vs shipped-244 contradiction + a historical block loading every turn. | Hygiene pass only: fix the dim line; relocate the historical snapshot to a tier-2 doc. |
| c3 | Marked down | Velocity | The analyst-visible failure — a column returned `unknown` — is measured nowhere. Reframe-collapse half refuted (correct anti-gaming). Survives: no aggregate abstention-rate number at promotion — free to compute. | Add an aggregate `unknown`/`word` abstention-rate line to the promotion report. |
| c4 | Marked down | Velocity | The operative goal has drifted to "don't regress the gittables fixtures." Systemic framing overreaches on 3 counts — but one kernel stands strongly: there is no *standing* production-representative instrument. | Stand up the rotating external band (move 1) — the single highest-leverage change here. |

## Struck on inspection (4)

- **Refuted — "Model classifies the first 50 rows in file order."** Factually wrong; it strides evenly across
  the whole column (`mod.rs:808`). The grep missed the arithmetic-index striding.
- **Overstated — "Full-materialises every file for 100 values."** The scan feeds exact null/enum/quality/unique
  reporting; ~20ms spawn is choice 0100's accepted cross-platform trade.
- **Refuted — "Header-hints mask a good Sense stage."** Already shipped (`e6049d9`). The finding read the
  ablation table and missed the ship log below it.
- **Overstated — "Corpus-frequency steering inverts analyst impact."** Flagship evidence was a misread: country
  −31.5% is a *rejected* candidate's regression, not a live hole. The head is at ceiling.

---

## What we don't know yet

The single load-bearing unknown, which move 3 answers: **does an over-tighten Sense gain survive Sharpen, or is
it redundant?** If redundant, the rules-only strategy is vindicated *with evidence* and you stop second-guessing
it. If it converts — the fine-tuned encoder hit 0.82 on contested columns — the biggest accuracy lever reopens.
Right now you are inferring the answer instead of measuring it, for the price of one predict-time A/B.

> **One line for a stakeholder.** FineType's accuracy is at ceiling where it's measured — but it's measured
> against a fixture it generates itself. The fastest wins now: point a rotating external ruler at it, unfreeze
> the gate from a retired model, and run the two free experiments that would tell us whether the learned model
> is actually done.

---

*How this review was produced.* Six parallel deep-dives (harness · labelling · modelling · value-rules · speed ·
gating) → 33 raw findings → consolidated to 13 theses + 4 blind-spots → each adversarially steelmanned before
judgement. 25 agents, ~1.4M tokens, 13 survivors / 4 struck. Every citation re-checked against the live tree
(e.g. the "recompile validators inside the per-file loop" latency win is already fixed at `profile.rs:151`).
