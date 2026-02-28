# Discovery: Architectural Pivot — Sense & Sharpen

**Date:** 2026-02-28
**Author:** @hughcameron
**Status:** Accepted — decision-004 recorded, Phase 0 (NNFT-162) and Phase 1 (NNFT-163) created

## Context

FineType's current architecture — a tiered graph of 46 CharCNN models with 18 disambiguation rules, a Model2Vec semantic classifier, and a Deep Sets entity classifier — has reached diminishing returns. The CLDR-enriched retraining (NNFT-157–161) regressed from 116/120 to 107/120 on profile eval, revealing systemic fragility rather than isolated training data issues. Three categories of root cause were identified: training data overlap (URL/URI), T1 routing degradation from shifted decision boundaries, and training diversity gaps. The model was rolled back to v0.3.0.

Meanwhile, the disambiguation rule count continues to grow. Each new rule patches a specific failure mode but adds interaction complexity — Rule 18 (entity demotion) already requires a guard to prevent header hints from overriding it. We are spending more effort on rule engineering than on the core model's ability to classify.

The CharCNN architecture has a documented capacity ceiling: it handles ≤20 labels per T2 model well, degrades between 20–50, and fails above 50 (NNFT-126, `docs/LOCALE_DETECTION_ARCHITECTURE.md`). This is a fundamental limit of fixed filter banks operating on local character n-grams. It means we cannot scale the taxonomy (more types, locale variants) without fracturing the model graph further.

We have evidence that this ceiling is not inherent to the problem. The original finetype prototype (`hughcameron/finetype`) used a Transformer model (Burn framework) with 4-level locale-in-label classification and succeeded where the CharCNN failed — particularly on locale-aware classification and larger label spaces.

This document proposes an architectural pivot that retains our strengths (fast character-level format detection, validation infrastructure, taxonomy contracts) while addressing the core limitations.

## Principles (unchanged)

Our north star remains the same. FineType should spark joy for analysts:

- **Makes the grudge work of wrangling far simpler** — profile a dataset and immediately know what you're working with
- **Saves the project by finding quality problems sooner** — detect mismatched types, locale inconsistencies, and format drift before they cause downstream failures
- **Makes the 'too hard' tasks for vast heterogeneous datasets actually achievable** — handle real-world messiness at scale, not just clean demo data

## Problem Statement

### Evidence of architectural limits

1. **CLDR regression (NNFT-157–161):** A well-planned, five-phase retraining with richer locale data made things worse. 9 regressions from 3 systemic causes — all traceable to the CharCNN's inability to handle increased training complexity without cascading confusion across the tier graph.

2. **CharCNN capacity ceiling:** ≤20 labels per T2 model (documented in NNFT-126). VARCHAR/person at 94 labels hit 51% accuracy. VARCHAR/location at 65 labels hit 55%. This ceiling blocks both taxonomy expansion and locale integration.

3. **Rule proliferation:** 18 disambiguation rules and growing. The entity classifier (NNFT-151), attractor demotion (Rule 15), text length demotion (Rule 16), UTC offset override (Rule 17), and entity demotion (Rule 18) are all compensating for what the model cannot learn. Each rule is correct in isolation but the interaction surface is becoming difficult to reason about.

4. **Entity overcall:** `full_name` is FineType's biggest false positive — 3,500+ SOTAB columns incorrectly classified. The entity classifier (75.8% 4-class accuracy) helps, but the embedding spike (NNFT-150) showed within-class spread is 3–4× larger than between-class centroid distance. Individual value embeddings cannot solve entity disambiguation alone.

5. **Locale remains out of reach:** Post-hoc locale detection (Decision 002) was the right call given CharCNN limits, but locale is critical for analyst workflows. Date format interpretation, phone number validation, address parsing — all require locale awareness that the current architecture cannot provide at the model level.

### What's working well

Not everything needs replacing. The following are genuine strengths:

- **Format-type detection** for structurally distinct types (emails, UUIDs, IPs, URLs, ISO timestamps, MAC addresses) — CharCNN excels here and accuracy is high
- **Validation schemas** — regex + function validators are a powerful safety net and produce actionable DuckDB transforms
- **CLDR infrastructure** — extraction scripts, 31-locale data, DateOrder enum (NNFT-157–158) are ready for reuse
- **Entity classifier training data** — 2,911 labelled SOTAB entity columns across 4 categories
- **Model2Vec embeddings** — `potion-base-4M` for header hints and entity features, with pure Rust inference
- **Evaluation infrastructure** — profile eval (120 columns), SOTAB mapping (92 types), GitTables 1M pipeline
- **Taxonomy contracts** — YAML definitions with validation schemas, DuckDB type mappings, SQL transforms

## Proposed Architecture: Sense & Sharpen

The core idea: use an attention-based model to *sense* what a column contains (semantic category, entity type, locale signal), then *sharpen* the classification with character-level pattern detection and validation rules.

### Pipeline: three stages

```
SENSE (Transformer, column-level)
  → SHARPEN (CharCNN + validation, value-level)
    → CONFIRM (rules, edge-case safety net)
```

**Stage 1 — Sense (Transformer, column-level)**

A small transformer operates on *column-level representations* — not raw character sequences of individual values. For each column, sample the top-K most commonly occurring values (or a representative sample), embed them, and feed the aggregated representation through an attention model that produces:

- Broad semantic category (format, entity, numeric, temporal, text, geographic)
- Entity subtype signal (person, place, organisation, creative work — when relevant)
- Locale signal (date ordering, number formatting, language indicators)
- Confidence estimate

This is where the Burn prototype's strengths are recaptured. The transformer sees *column context* — the distribution of values, not just individual strings — which is exactly the signal needed for semantic disambiguation. A column of ["Microsoft", "Apple", "Google"] reads differently from ["Melbourne", "Sydney", "Brisbane"] even though individual values are indistinguishable at the character level.

The key insight from the entity disambiguation spike (NNFT-150): individual value embeddings have massive within-class overlap, but *column-level aggregations* carry usable signal (73.6% with off-the-shelf Random Forest, 75.8% with the trained MLP). A purpose-built attention model over sampled values should exceed both.

**Stage 2 — Sharpen (CharCNN + validation, value-level)**

The CharCNN's strength — fast, accurate character-level pattern detection for format types — is retained but *scoped* by the Sense stage. Rather than running 46 models in a blind T0→T1→T2 cascade, the Sense output narrows the classification space:

- Sensed as "temporal" → run only date/time CharCNN models
- Sensed as "entity/organisation" → skip CharCNN entirely, classify as `organisation`
- Sensed as "format/internet" → run internet-category CharCNN models
- Sensed as "numeric" → run numeric CharCNN models with validation

This reduces cascading errors (a Tier 0 misroute no longer corrupts the entire chain) and improves speed (fewer models invoked per column).

Validation schemas continue to operate here — they're one of our strongest assets. A value classified as `email` still gets validated against the email regex. Validation failures feed back as a confidence signal.

**Stage 3 — Confirm (rules, reduced set)**

A small set of rules handles known edge cases that neither model can resolve:

- Date format disambiguation (US vs EU) based on component ranges
- Coordinate resolution (lat vs lon) based on value ranges
- Boolean detection for ambiguous integer/character columns

The goal is to reduce the current 18 rules to approximately 5–8, with the Sense stage absorbing most of what Rules 14–18 currently do.

### Why this order?

The intuition is: *sense what we're seeing before trying to classify it precisely*. The current architecture does the reverse — it classifies first (CharCNN), then tries to correct mistakes (disambiguation rules, entity demotion, attractor demotion). This leads to the "patching up" dynamic we're experiencing.

With Sense first, the system knows it's looking at organisation names before attempting character-level classification. It knows the column contains Australian-formatted dates before trying to parse them. The CharCNN then operates in a much smaller, better-defined problem space.

## Taxonomy Redesign

### Philosophy

The taxonomy should reflect what analysts actually need to know about their data, not what a model can theoretically distinguish. This means:

- **Expand where it matters:** entity types (person, place, organisation), financial instruments (tickers, ISINs), transport identifiers, and other types that frequently appear in analytical datasets
- **Collapse where it doesn't:** niche types that analysts would be satisfied to see as a broader category, and types the model consistently confuses
- **Separate format from semantics:** format types are structurally distinguishable (the CharCNN's strength); semantic types require context (the transformer's role)

### Proposed type tiers

**Format types** — structurally distinguishable, high-precision targets. These are FineType's bread and butter and should remain granular:

- Dates and timestamps (ISO 8601, US slash, EU slash, abbreviated month, etc.)
- Identifiers (UUID, email, URL, URI, IP addresses, MAC addresses)
- Codes (ISIN, SEDOL, SWIFT/BIC, postal codes, phone numbers, calling codes)
- Numeric formats (integer, decimal, scientific notation, percentage, currency amounts)
- Structured text (JSON, XML paths, file paths, semantic versions, cron expressions)

**Semantic types** — require column context, benefit from the Sense stage:

- `person` (names of people)
- `organisation` (company names, institutions, NGOs)
- `place` (cities, regions, landmarks — geographical entities)
- `country` (kept distinct — high analytical value and partially format-detectable via ISO codes)
- `creative_work` (book titles, film names, album titles)
- `financial_instrument` (stock tickers, fund names — new)
- `transport_identifier` (flight codes, vessel names, route identifiers — new)
- `product` (product names, SKUs where not code-formatted — new)

**Broad types** — catch-all categories for text the model can classify broadly but not specifically:

- `plain_text`, `sentence`, `paragraph`
- `word`, `categorical`
- `entity_name` (retained as a fallback when semantic classification is low-confidence)

### Types to collapse

| Current Type | Proposed Mapping | Rationale |
|---|---|---|
| `technology.hardware.cpu` | `plain_text` | Not structurally detectable; niche |
| `technology.hardware.generation` | `plain_text` | Same — no format signal |
| `identity.academic.university` | `organisation` | Universities are organisations |
| `identity.academic.degree` | `categorical` or `plain_text` | Low cardinality, no format signal |
| `representation.text.slug` | `plain_text` or merge with identifiers | Slugs are format-detectable but rarely analytically important |
| `identity.person.nationality` | `categorical` | Usually a short enumerated list |
| `identity.person.occupation` | `categorical` or `plain_text` | Free text, no format signal |

This is not exhaustive — a full audit should be done as part of the implementation. The principle is: *if an analyst would say "that's fine, just call it X"*, then collapse it.

### Types to add

| Proposed Type | Domain | Rationale |
|---|---|---|
| `identity.financial.ticker` | identity | Stock tickers (AAPL, BHP.AX) — very common in analytical data |
| `identity.financial.isin` | identity | International Securities Identification Number — 12-char format |
| `identity.financial.cusip` | identity | North American security identifier — 9-char format |
| `geography.transport.flight_code` | geography | IATA flight codes (QF1, UA100) — frequent in logistics/travel data |
| `geography.transport.iata_airport` | geography | 3-letter airport codes — compact, enumerable |
| `geography.transport.port_code` | geography | UN/LOCODE port identifiers |
| `technology.internet.domain_name` | technology | Bare domain names (not full URLs) — common in web analytics |
| `representation.text.product_name` | representation | Product names — common in e-commerce data |

These additions are justified by frequency in real-world analytical datasets. Financial and transport identifiers have strong format signals (making them good CharCNN candidates), while product names are semantic (benefiting from the Sense stage).

### Locale integration

The Burn prototype demonstrated that locale-in-label classification works with a transformer architecture. With the Sense stage providing locale signal at the column level, we can reintroduce locale-aware classification without hitting the CharCNN's capacity ceiling.

The approach: the Sense stage detects column-level locale (e.g., "these dates are in DMY order", "these phone numbers are Australian format"). This locale context is passed to the Sharpen stage, which uses it to select the appropriate validation schema and format interpretation. Locale becomes a *column attribute* rather than a *label suffix* — avoiding the label explosion that broke tiered-v3.

The CLDR infrastructure (NNFT-157–158) — 706 locales of date/time patterns, 31 locales of month/weekday names, DateOrder enum — feeds directly into this. It was the right investment; it just needs the right model architecture to leverage it.

## Speed

`finetype profile` should feel fast. The aspiration is that profiling a typical analytical CSV (50 columns, 10,000 rows) is effectively instantaneous — the kind of speed where you run it reflexively on every new dataset without thinking about cost.

We don't have firm latency targets yet, and setting arbitrary numbers before the spike work would be premature. Instead, the approach is:

1. **Establish a measurement framework early** — instrument the Sense and Sharpen stages independently so we can see where time is spent
2. **Benchmark against the current architecture** — the CharCNN baseline (580 val/sec tiered, 1,500 val/sec flat) and the DuckDB extension throughput (2,093 val/sec batch) give us reference points
3. **Sample, don't scan** — the Sense stage operates on sampled values (top-K or representative sample), not every value in the column. This is a fundamental speed advantage: for a 10,000-row column, sensing 50 representative values is 200× less work than classifying all 10,000
4. **Let targets emerge from the spike** — once we have a working Sense model, we'll know its actual throughput and can set informed targets

The risk here is spending engineering time optimising for a speed target that turns out to be the wrong one. Better to build the right architecture and then optimise than to constrain the design around premature performance targets.

One firm constraint: `finetype profile` must remain a single-binary CLI tool with no Python runtime dependency. The Candle framework (pure Rust ML inference) continues to be the right foundation for this.

## Evaluation

### Current state

The existing evaluation infrastructure measures:

- **Profile eval:** 120 annotated columns, currently 116/120 (96.7%). Useful as a regression gate but too small to be a comprehensive benchmark.
- **SOTAB CTA:** 16,840 columns from real websites. Currently 43.3% label / 68.3% domain accuracy. The schema mapping (92 types) makes this the most rigorous external benchmark.
- **GitTables 1M:** 45,428 columns from GitHub repositories. Currently 47% label accuracy. Broad coverage but noisy ground truth.

### What needs to change

The pivot changes both the taxonomy and the model, which means evaluation targets need recalibration:

- **Taxonomy changes invalidate current scores.** If we merge `university` into `organisation`, the SOTAB mapping changes. Scores will shift for structural reasons, not quality reasons. We need to re-baseline after the taxonomy redesign.
- **New types need evaluation data.** Financial instruments, transport identifiers, and other additions need annotated test columns. SOTAB and GitTables may not have sufficient coverage — we may need to curate additional eval data.
- **"Spark joy" is hard to measure.** The analyst experience is about more than label accuracy — it's about whether the profile output *helps*. A column classified as `organisation` instead of `entity_name` is more useful even if the model isn't more accurate in aggregate. We should consider usability-oriented metrics alongside raw accuracy.

Concrete evaluation work for the spike:

1. **Re-map SOTAB schema** to the revised taxonomy and establish a new baseline
2. **Extend profile eval** to 200+ columns including new types (financial, transport)
3. **Define a "utility score"** that weights types by analytical importance — correctly identifying a date format matters more than distinguishing `slug` from `plain_text`
4. **Benchmark speed** with the measurement framework described above

## Migration Path

### What's retained

| Asset | Status | Role in new architecture |
|---|---|---|
| Validation schemas (regex + functions) | Keep as-is | Sharpen stage — value-level validation |
| CLDR extraction infrastructure | Keep as-is | Locale data for Sense stage training |
| Model2Vec embeddings | Keep as-is | Feature input for Sense stage |
| Entity classifier training data (SOTAB) | Keep, expand | Training data for Sense stage entity subtyping |
| Taxonomy YAML format | Keep structure | Definitions updated for new type set |
| DuckDB type mappings | Keep, update | Mappings revised for new taxonomy |
| Profile eval framework | Keep, extend | Extended with new types and utility weighting |
| SOTAB/GitTables eval pipelines | Keep, re-map | Schema mappings updated for new taxonomy |
| CharCNN format-type models | Keep selectively | Sharpen stage — format-type classification |

### What's deprecated

| Asset | Reason |
|---|---|
| Tiered T0→T1→T2 graph routing | Replaced by Sense stage routing |
| Disambiguation Rules 14–18 | Absorbed by Sense stage |
| Entity classifier as separate bolt-on | Entity subtyping moves into Sense stage |
| Semantic hint classifier (header hints) | Header signal becomes a Sense stage input |

### What's new

| Component | Purpose |
|---|---|
| Sense model (transformer) | Column-level semantic classification |
| Sense training pipeline | Training data curation and model training |
| Column sampling strategy | Representative value selection for Sense input |
| Revised taxonomy definitions | Updated YAML with new/merged types |
| Updated SOTAB schema mapping | Re-mapped for revised taxonomy |

## Risks

1. **Transformer latency.** Even a small transformer is slower per-inference than a CharCNN. The column-level sampling strategy mitigates this (50 values not 10,000), but we need to validate that the end-to-end pipeline meets speed expectations. If the Sense stage adds >100ms per column, it may need to be optional or run in a "quick" vs "thorough" mode.

2. **Training data for the Sense stage.** The entity classifier used 2,911 SOTAB columns. The Sense stage needs broader coverage — not just entity types but also format categories, locale signals, and the new semantic types (financial, transport). Building this dataset is non-trivial and is likely the longest-pole item.

3. **Regression on format types.** The CharCNN is genuinely good at format types. If the Sense stage misroutes a column (e.g., classifying structured codes as "text"), the Sharpen stage won't have a chance to get them right. The Confirm stage needs to catch these cases — some disambiguation rules may persist.

4. **Scope creep in taxonomy expansion.** Adding financial instruments, transport identifiers, and product names is justified, but each new type needs training data, validation schemas, and eval coverage. There's a risk of expanding the taxonomy faster than the model and evaluation can keep up. Recommend adding new types in batches, validated against eval data before proceeding.

5. **Two-model coordination complexity.** Sense + Sharpen is simpler than the current 46-model tier graph, but it introduces a different kind of complexity: the handoff between stages, confidence calibration across different model types, and the interaction between column-level and value-level signals. This needs careful interface design.

6. **Data ingestion.** The Sense stage assumes it can sample representative values from a column. For well-formed CSVs this is straightforward. For complex formats (XML, nested JSON, malformed CSVs), value extraction is a separate problem. This is explicitly deferred — solving it later is the right call — but it should be acknowledged as a dependency for real-world robustness.

## Proposed Spike Plan

Before committing to the full pivot, a time-boxed spike should validate the core hypothesis: that a column-level transformer can outperform the current Sense-equivalent (entity classifier + semantic hints + disambiguation rules) on semantic classification.

**Spike scope (suggested 2–3 days):**

1. Build a column-level transformer prototype (Python/PyTorch, not Rust yet) that takes top-K sampled values per column and predicts broad semantic category + entity subtype
2. Train on SOTAB entity columns (2,911) plus format-type columns from profile eval and GitTables
3. Evaluate against current system on the same columns
4. Benchmark inference speed (values/sec at the column level)
5. Document findings in a FINDING.md with go/no-go recommendation

**Go criteria:**

- Semantic classification accuracy exceeds current entity classifier (>76% on 4-class entity task)
- Broad category routing accuracy exceeds current T0 (>98%)
- Column-level inference completes in <50ms for 50 sampled values
- Clear path to Rust/Candle implementation

## Open Questions

1. **What transformer architecture for Sense?** Options range from a lightweight attention layer over Model2Vec embeddings (minimal, fast) to a small BERT-style encoder over character sequences (powerful, slower). The spike should test at least two variants.

2. **How many sampled values per column?** The entity classifier used up to 20. More samples = better signal but slower. The spike should test 10, 20, and 50 to find the sweet spot.

3. **Should the CharCNN be retrained for the revised taxonomy?** If niche types are collapsed, the remaining types may be easier to classify — the existing CharCNN might improve without architectural changes, just from a cleaner label space. Worth testing.

4. **Where does the header/column name signal go?** Currently it's a post-classification override (semantic hint classifier). In the new architecture, it could be an input to the Sense stage — the column name "company_name" is strong prior signal for organisation. This would unify two currently separate systems.

5. **DuckDB extension implications.** The DuckDB extension currently embeds CharCNN weights. A two-stage pipeline means embedding both Sense and Sharpen models. Size and load-time implications need assessment.
