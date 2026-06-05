# Roadmap memo — the precision release(s)

**Date:** 2026-06-05
**Status:** raw roadmap awaiting distillation into cards/specs. Author to react first.
**Origin:** "Let's make an ambitious plan to release the best finetype package we can using everything we've learned" — directly after shipping validation-as-veto in profile (spec 2026-06-05-validation-gate-precision-fixes, choice 0091).

## The thesis

The win we just shipped names the whole programme. Validation-as-veto works because
it is a hard check the model **cannot talk itself out of** — and that is the
principle the release should be built around. The headline is not "a better model".
It is: **FineType stops being confidently wrong. It checks its own answers against
your data and says "I don't know" instead of guessing.**

## Three learnings we are cashing in

1. **Validation-as-veto is the precision lever that needs no retraining.** Shipped
   in profile today. It is asymmetric — a reliable NO, an unreliable YES (memory
   `validation-gate-asymmetry`). The NO half is deployable now, everywhere a type
   is emitted, with zero training data.

2. **Value features alone crack the hardest disambiguation.** The CharCNN +
   schema-gate probe drove latitude collateral to *zero* on the 150-file corpus —
   beating the shipped v19 default — using value shape + the JSON-Schema gate, with
   *no header* (memory `stage1-charcnn-precision-schema-gate`). This is the
   encouraging precursor for the late-fusion architecture (memory
   `late-fusion-architecture-plan`, the author's 2026-06-04 direction).

3. **We cannot yet honestly score a model bet.** The cell-2 eval metric is polluted
   by YDF mislabels — `msg_id`→iso6346, team codes→country_code. Half the
   "regressions" we chased (v22, v23) were noise, not capability loss (memories
   `v22-true-band`, `v23-ac01-finding`, `v23-ac08-outcome`). Until ground truth is
   cleaned, every retrain flies blind.

## Author decisions (2026-06-05)

- **Model bet = late-fusion, full build.** CharCNN-value + JSON-Schema features
  (`validation_features.rs`) + Model2Vec header, fused late instead of mid. This is
  the ambitious model direction — NOT a patch. It is its own larger release, gated
  on honest ground truth.
- **Near-term ships as a PATCH (0.6.24), not a bundled 0.7.** The veto-everywhere +
  release-polish work is additive and needs no model promotion, so it ships now as a
  patch rather than waiting on the model programme.
- **Capture = roadmap memo first** (this file) before any cards/specs.

### Strategic direction added 2026-06-05 — YDF as a means of production, not a shipped artifact

The headline goal of the precision programme is now explicit: **maximise the amount
of quality labelled real-world data.** The mechanism:

1. We hold very high-quality AUTHORITATIVE reference inputs — Geonames (geographic
   names), CLDR (locale/datetime/number/currency/language/territory), our own
   synthetic generators, ISO code lists already baked into taxonomy enums, IANA
   registries — and more to be catalogued.
2. Use those authoritative inputs to train **specialised YDF sieves** (per
   reference-backed type) that recognise real-world values of that type.
3. Run the sieves over GitTables (public corpus) to **mine** the real-world columns
   that are genuinely that type — converting unlabelled corpus into labelled data.
4. Train the **best-ever FineType (Sense) model** on those harvested real-world
   values.

YDF flips role: **judge → miner.** It is NOT shipped. It is a tool that improves our
*means of production* (Pillar 2: agent self-learning / the data factory; Pillar 4:
long-running R&D). This is distant supervision / data programming (the Snorkel
recipe): authoritative inputs → labelling functions (specialised YDF) → aggregate
and filter → train the end model.

This directly attacks the corpus-starvation wall that closed spec 2026-06-04
INCONCLUSIVE (10 distinct latitudes in 18M rows). Starvation only exists if you find
labels using the labels you already have; manufacturing them from reference data
dissolves it.

**Provenance correction (verified 2026-06-05):** the YDF lens trains on PUBLIC data
only (sherlock_distilled GitTables samples + synthetic generator) — see corrected
memory `private-dataset-ydf-training`. The earlier "private upstream dataset" belief
was a misunderstanding. So repurposing/specialising YDF carries NO leak risk.

**The load-bearing constraint — eval independence.** Once YDF mines the training
data it CANNOT also be the independent eval judge (training Sense on YDF-mined labels
then scoring against YDF is circular). So the curated gold eval anchor (B1 below) is
MANDATORY and must be held out from the mining pipeline — it shares NO inputs with
the factory.

**Mining precision is the whole game — the harvest is a funnel, not one classifier:**
specialised sieve (recall) → validation-as-veto / JSON-Schema gate (precision NO,
shipped today) → cross-sieve disagreement drop (no column mined as two incompatible
types) → optional per-type human spot-check. The validation gate shipped this morning
is a load-bearing stage of the factory — release and factory share one engine.

**Honest scope.** Strong for reference-backed types (geography, locale/datetime/
currency, ISO/IANA code lists) — 10–100× real-world coverage. Partial for shared-shape
numerics (latitude vs longitude vs temperature): mining gives real values but a bare
float column is still shape-ambiguous; that residual is where late-fusion's header +
sibling context earns its keep. The factory feeds better fuel; the model still
disambiguates.

## Two release lines

### Line A — the precision PATCH (0.6.24), ships now

Additive, non-breaking, no model promotion. The precision win that lands regardless
of the model programme.

- **Veto everywhere.** Extend validation-as-veto from `profile` → `validate`,
  `infer`, the DuckDB extension, and MCP. Make `unknown ⊘ vetoed:X` a first-class,
  documented output across every surface that emits a type.
- **Release-readiness polish** (close the half-done umbrella spec
  2026-06-02-public-release-readiness): website rewrite landed, **240** as the one
  canonical type count across binary/README/website, model-weight download path then
  untrack the ~52MB of tracked safetensors, strip remaining internal jargon from
  `--help`.
- **DuckDB community republish + `ft_` table verbs** (specs
  2026-06-02-duckdb-extension-republish, 2026-06-03-duckdb-extension-table-verbs).
- **MCP polish.**
- **Open question for distill:** does the safe `v24-numeric-precision` retrain
  (spec 2026-06-03, reachability-safe numeric targets) ship as an interim default
  in this patch line, or do we hold the default at v19 until late-fusion lands?
  v24 was the fallback when late-fusion was the chosen bet — it may still be worth
  shipping as a known-good interim precision lift, or it may be churn we skip.

### Line B — the data factory + late-fusion model programme (later, ambitious)

The high-ceiling bet. Lands in its own release when it clears the bar. Restructured
2026-06-05 around the YDF-as-miner direction above.

- **B0 — Reference-data inventory.** Catalogue every authoritative input and the
  type(s) it backs: Geonames, CLDR, ISO code lists (already in enums), IANA
  registries, our generators, + the others the author knows exist. Breadth of this
  catalogue caps breadth of the harvest. Do early — it is the raw material.
- **B1 — Gold eval anchor (do FIRST, independence guarantee).** Small CURATED
  ground truth for the confusion families, held out from the mining pipeline,
  sharing NO inputs with the factory. Labels come from NEITHER model under
  comparison. This replaces the old "YDF-cleaned cell-2 baseline" framing — once YDF
  becomes the miner it cannot be the judge.
- **B2 — The mining factory.** Specialised YDF sieves per reference-backed type →
  validation/cross-sieve precision funnel → harvested real-world labelled corpus.
  YDF is a means of production here, never shipped.
- **B3 — Best-ever Sense (late-fusion full build).** CharCNN-value + JSON-Schema
  (`validation_features.rs`) + Model2Vec header, fused late instead of mid, trained
  on the B2 harvested corpus. Gate-promote against B1 using the mandatory
  Sense-distribution pre/post check (CLAUDE.md). Schema-gate probe is the precursor;
  v23 categorical-explosion is the guard.

## Sequencing

1. Line A patch (0.6.24) — ships in days. Veto-everywhere + polish + DuckDB/MCP.
2. Line B B0 + B1 (inventory + gold anchor) — runs in parallel; unblocks judging.
3. Line B B2 (mining factory) — builds the labelled corpus from reference data.
4. Line B B3 (late-fusion on harvested corpus) — the multi-week build, promoted
   against B1.

## One-liner for a stakeholder

FineType now validates its own type predictions against your data and tells you when
it doesn't know — instead of guessing.

## Open questions to resolve at distill

- v24 interim default: ship in the patch line, or hold at v19 until late-fusion?
- Late-fusion release number: 0.7 (minor) or 1.0 (the "stops guessing" story is a
  1.0-worthy headline)?
- Does veto-everywhere need a config/threshold surface per command, or is the
  profile default (`--no-validation-veto`, 50% threshold) the right uniform shape?
