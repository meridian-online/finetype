# Discovery: Do our benchmarks measure real-world type inference quality?

**Task:** NNFT-144
**Date:** 2026-02-27
**Status:** Finding complete

## The Problem

We have three benchmarks that tell three different stories:

| Benchmark | Label accuracy | Domain accuracy | Columns |
|---|---|---|---|
| Profile eval | 93.2% (69/74) | 93.2% | 74 |
| GitTables 1M | 47.1% | 56.5% | 45,428 |
| SOTAB CTA | ~24.3% | ~61.5% | 21,077 |

On the surface, this looks like FineType is great on toy data and terrible in the real world. **That conclusion is wrong.** The benchmarks are measuring different things, and none of them measure what analysts actually care about.

## Finding 1: The headline numbers are structurally misleading

The SOTAB and GitTables benchmarks use ground truth labels from schema.org and DBpedia — type systems that classify data by **semantic meaning**, not by **format**. FineType classifies by format. These are different questions.

Of the 192 ground truth labels in our schema mapping:

| Match quality | Labels | % of total | What it means |
|---|---|---|---|
| **direct** | 18 | 9.4% | FineType has a 1:1 format match |
| **close** | 21 | 10.9% | FineType has a near match |
| **partial** | 33 | 17.2% | Sometimes detectable, depends on data format |
| **semantic_only** | 120 | 62.5% | No format signal — FineType cannot detect by design |

**62.5% of ground truth labels are semantic-only.** Labels like `category`, `description`, `rating`, `product`, `manufacturer`, `species` describe what data *means*, not how it's *formatted*. FineType correctly identifies these as `representation.text.word` or `representation.numeric.decimal_number` — which is the right *format* answer, even though it doesn't match the *semantic* label.

The headline label accuracy is penalised for not being a semantic classifier. It's like grading a French exam with a German answer key.

### SOTAB error budget breakdown

| Detectability tier | Columns | % of total | Label accuracy | Domain accuracy |
|---|---|---|---|---|
| Format-detectable (direct + close) | 15,796 | 74.9% | 30.9% | 72.9% |
| Partially detectable | 1,928 | 9.1% | 12.2% | 41.6% |
| Semantic-only | 3,353 | 15.9% | 0.0% | 19.0% |

Even within format-detectable types, label accuracy is only 30.9% — because the mapping allows multiple correct FineType labels per GT label (e.g., GT "Date" could match any of 12+ FineType date formats). Domain accuracy (72.9%) is the more meaningful signal.

## Finding 2: FineType excels where analysts need it most

When we look at precision (not recall), the picture is completely different. These are the metrics that determine whether an analyst can *trust* FineType's output:

### Datetime detection — the biggest grudge-work saver

| GT label | Columns | Detected as datetime | Recall |
|---|---|---|---|
| DateTime | 616 | 611 | **99.2%** |
| Date | 616 | 580 | **94.2%** |
| Time | 127 | 121 | **95.3%** |
| Duration | 587 | 386 | 65.8% |

**When FineType says "this is a date/time": 96.6% precision** (1,698 of 1,757 datetime predictions are actually datetime). An analyst searching for "all date columns across 500 files" gets near-complete coverage with almost no false positives.

This is the "saves the grudge work" metric. Parsing dates across heterogeneous formats is *the* classic analyst pain point — and FineType nails it.

### High-value type precision on SOTAB

| Predicted type | Times predicted | Actually correct | Precision |
|---|---|---|---|
| Email | 51 | 51 | **100%** |
| Phone number | 341 | 340 | **99.7%** |
| Postal code | 307 | 196 | 63.8% |
| Country/city/region | 1,597 | 737 | 46.1% |
| URL | 838 | 275 | 32.8% |

Email and phone are near-perfect. When FineType says "this is an email" or "this is a phone number", the analyst can act on it immediately. These are the types that enable data quality checks — "flag any non-email values in the email column."

URL precision (32.8%) and geography precision (46.1%) need work — many non-URL and non-geographic strings get misclassified.

## Finding 3: full_name overcall is the single biggest quality problem

3,500+ SOTAB columns are predicted as `identity.person.full_name`. The actual composition:

| GT label | Columns | % of all full_name predictions |
|---|---|---|
| Text (free-form) | 270 | 7.7% |
| MusicRecording/name | 190 | 5.4% |
| Mass | 175 | 5.0% |
| Organization | 173 | 4.9% |
| Person/name (correct!) | 172 | 4.9% |
| MusicArtistAT | 156 | 4.4% |
| Recipe/name | 149 | 4.2% |
| LocalBusiness/name | 142 | 4.0% |
| Place | 133 | 3.8% |
| Person (correct!) | 129 | 3.7% |

**Only ~8.6% of full_name predictions are actually person names.** The rest are music recordings, organizations, recipes, restaurants, books, events — anything with proper noun strings. This is the entity_name problem identified in NNFT-137 and NNFT-145.

From an analyst perspective, this is the "don't mislead me" problem. If FineType says "this column contains person names" and it's actually restaurant names, the analyst loses trust in all FineType predictions.

On GitTables, `full_name` is the 3rd most common prediction (4,521 of 45,428 columns = 10%). The overcall rate is likely similar.

## Finding 4: The profile eval is a regression detector, not a quality metric

The profile eval (69/74) tests 74 hand-curated columns from 20 small CSV files. It measures:

- **What it actually tests**: Can FineType correctly classify well-structured, single-type columns with clear headers?
- **What it doesn't test**: Mixed-type columns, messy data, missing values, ambiguous formats, large-scale consistency
- **What it's good for**: Catching regressions — if a code change drops from 69/74 to 67/74, something broke
- **What it's bad for**: Measuring real-world quality. 74 columns is too small to be statistically significant for 171 types.

The profile eval serves its purpose as a smoke test. It should stay. But it should not be the number we optimise for.

## What should we measure instead?

### The analyst joy framework

An analyst encounters FineType in three scenarios. Each needs its own metric:

#### Scenario 1: "Make the grudge work simpler" — Safe type casting

> *"I have 200 CSV files. Which columns are dates? What format are they in? Can I safely CAST them?"*

**Metric: Actionability rate** — For columns where FineType predicts a specific type with a `format_string` (dates, timestamps), what fraction can be safely `TRY_CAST`ed without data loss?

This is measurable: run `TRY_CAST(col AS type USING format_string)` on the actual data. Count NULL results (cast failures). Report the success rate.

**Target**: >95% actionability for datetime types. If FineType says "this is an ISO 8601 date", `TRY_CAST` should succeed on >95% of non-null values.

#### Scenario 2: "Find quality problems sooner" — Data quality flags

> *"Which columns have unexpected values? Is this really an email column? Are there phone numbers mixed in with names?"*

**Metric: Precision per type** — When FineType says "this is type X", how often is it actually type X?

From SOTAB data:

| Type class | Precision | Analyst trust level |
|---|---|---|
| Datetime | 96.6% | High — act on it |
| Email | 100% | High — act on it |
| Phone | 99.7% | High — act on it |
| Boolean | ~95% | High — act on it |
| Postal code | 63.8% | Medium — verify first |
| Geography | 46.1% | Low — overcall risk |
| URL | 32.8% | Low — overcall risk |
| Person name | ~8.6% | Very low — mostly wrong |

**Target**: >80% precision for every type FineType reports. Types below 80% should either be improved or reported with a confidence caveat.

#### Scenario 3: "Make the too-hard tasks achievable" — Large-scale data profiling

> *"I have 10,000 columns across 500 heterogeneous CSVs. What are they?"*

**Metric: Domain accuracy on real-world data** — At the domain level (datetime, geography, identity, representation, technology), how consistently does FineType classify?

On SOTAB format-detectable types: **72.9% domain accuracy.** This means an analyst profiling 10,000 columns would get the right domain for ~7,300 of them. The remaining ~2,700 would mostly be domain-adjacent (e.g., geography classified as identity, or numeric classified as identity.person.age).

**Target**: >80% domain accuracy on format-detectable types. The path here is fixing the overcall problems (full_name, full_address, URL).

### Proposed evaluation structure

| Eval | What it measures | Frequency | Target |
|---|---|---|---|
| **Profile eval** (existing) | Regression detection | Every build | No regressions from baseline |
| **Precision eval** (new) | Per-type precision on SOTAB | Monthly / post-release | >80% per type |
| **Actionability eval** (new) | TRY_CAST success rate | Monthly / post-release | >95% for datetime |
| **Domain eval** (existing, reframed) | Domain accuracy on SOTAB format-detectable | Monthly / post-release | >80% |

The precision eval is the most impactful addition. It directly answers the analyst's question: "Can I trust what FineType tells me?"

## Ambition: What "spark joy" looks like

The findings above explain WHY our headline numbers look bad. But explaining the gap is not the same as closing it. The 8.6% precision on full_name is a real problem — it actively misleads analysts. The 46.1% geography precision means an analyst can't trust location classifications. These need to get genuinely better.

### Precision targets — the analyst trust threshold

Every type FineType claims to detect should meet **>80% precision** on real-world data. Types currently above this threshold stay there. Types below it either get improved or get honest about their uncertainty.

| Type class | Current precision (SOTAB) | Target | Path to get there |
|---|---|---|---|
| Datetime | 96.6% | >95% | Maintain — already excellent |
| Email | 100% | >95% | Maintain |
| Phone | 99.7% | >95% | Maintain |
| Boolean | ~95% | >90% | Maintain |
| Postal code | 63.8% | >80% | Expand locale validation (CLDR) |
| Geography | 46.1% | >80% | CLDR city/country/region lists as validation |
| URL | 32.8% | >80% | Tighten URL validation, reduce false positives |
| Person name | 8.6% | >80% | Name-dataset validation (see below) |

### Data assets that make this achievable

We already have or can access authoritative data for the hardest problems:

**Person name disambiguation** — `/home/hugh/github/philipperemy/name-dataset`:
- 727,556 first names, 983,826 last names
- 105 countries with per-name country distribution and gender data
- US alone: 25,525 first names. Combined with 984K last names, this gives real validation signal
- Use case: when FineType predicts full_name, validate sample values against the name lists. If <20% of values match known names, demote to entity_name or text. This alone could fix the 8.6% precision problem.

**Geography** — CLDR + existing locale data:
- 14 locales of postal code patterns (libaddressinput)
- 15 locales of phone number patterns (libphonenumber)
- CLDR city/country/region name lists can validate geography predictions the same way name lists validate person predictions
- World cities dataset already in our eval data (23,000+ cities)

**SOTAB + GitTables as validation corpora** — Not just for scoring, but for building validation rules:
- 16,765 SOTAB columns with ground truth labels = training signal for disambiguation
- 45,428 GitTables columns = scale testing for false positive rates
- These are FREE labelled data we can use to tune thresholds and validate improvements

### Architecture evolution: right model for the right tier

The tiered model graph already supports different model architectures at different tiers. The CharCNN excels at format detection (regex-like character patterns) but struggles at entity disambiguation (is "London" a city or a surname?). These are fundamentally different problems.

**Where CharCNN works**: Value-level format detection. Is this string an email? A date? A UUID? Character patterns are definitive. CharCNN is fast, accurate, and well-suited.

**Where CharCNN struggles**: Entity disambiguation at T1/T2 boundaries. Is this column of proper nouns people, places, organisations, or products? Character patterns can't distinguish "Tokyo" (city) from "Toyota" (company) from "Tanaka" (surname). This is a word-level problem.

**Opportunity**: A small transformer or word-embedding classifier at the VARCHAR T1 router or entity-related T2 nodes could use word-level semantics that CharCNN can't capture. The tiered graph architecture already supports this — `ValueClassifier` trait is polymorphic. A tier node could use CharCNN for format tiers and a lightweight transformer for entity tiers.

This is not about replacing CharCNN — it's about using the right tool at the right decision point in the graph. The same principle as the existing Model2Vec header hints, but applied to value classification instead of header classification.

### Evaluation that drives improvement

The evaluation should make us uncomfortable when we're not good enough, and celebrate when we are.

**Proposed evaluation structure:**

| Eval | What it measures | Target | Frequency |
|---|---|---|---|
| **Profile eval** (existing) | Regression smoke test | No regressions from baseline | Every build |
| **Precision eval** (new) | Per-type precision on SOTAB | >80% every type | Post-release |
| **Recall eval** (new) | Per-format-detectable-type recall | >90% for datetime/email/phone | Post-release |
| **Actionability eval** (new) | TRY_CAST success rate | >95% for datetime | Post-release |
| **Overcall eval** (new) | False positive rate for full_name, URL, geography | <20% false positive rate | Post-release |
| **Domain eval** (reframed) | Domain accuracy on SOTAB format-detectable | >85% | Post-release |

**The overcall eval is the key new piece.** It specifically measures the analyst trust problem: when FineType says "person name" and it's actually restaurant names, that's a measurable false positive. When we bring full_name precision from 8.6% to >80%, analysts win.

### Concrete next steps

1. **Add precision + overcall reporting to SOTAB eval SQL** — Flip the existing eval from recall-only to precision-and-recall. SQL change, no code change. This gives us the baseline.

2. **Build name validation using name-dataset** — Extract first/last name lists, add as `validation_by_locale` or a dedicated validation lookup. When full_name is predicted, validate sample values. High-impact, known path (same pattern as phone/postal validation).

3. **Build geography validation using CLDR** — City/country/region name lists as validation for geography predictions. Same pattern as name validation.

4. **Investigate transformer at T1 VARCHAR** — Spike: can a small transformer (distilled BERT, or Model2Vec at value level) improve the person/location/organisation T1 routing decision? Time-boxed 4-6 hours.

5. **Reframe SOTAB/GitTables eval** — Replace headline "label accuracy" with per-type precision table. Make the precision targets visible. Red/yellow/green by type.

### What NOT to do

- **Don't add semantic types** — `category`, `rating`, `product` are out of scope. But don't use this as an excuse for bad precision on types we DO detect.
- **Don't declare victory at 72.9% domain accuracy** — That's the starting line, not the finish line.
- **Don't rely solely on model retraining** — Training is non-deterministic. Validation-based approaches (name lists, CLDR data) are deterministic, testable, and composable. Use both.

## Data sources

**Evaluation corpora:**
- SOTAB CTA validation set: 21,077 columns (16,766 with predictions), 99 GT labels
- GitTables 1M: 45,428 columns, 3,771 tables, 94 topics
- Profile eval: 74 format-detectable columns, 20 datasets
- Schema mapping: 192 GT labels (18 direct, 21 close, 33 partial, 120 semantic-only)
- SOTAB schema mapping: 99 labels (17 direct, 32 close, 9 partial, 41 semantic-only)

**Validation data assets:**
- Name dataset: 727,556 first names + 983,826 last names across 105 countries (`~/github/philipperemy/name-dataset`)
- CLDR locale data: postal codes (14 locales), phone numbers (15 locales), calling codes (17 locales), month/day names (6 locales)
- World cities: 23,000+ cities across evaluation datasets
- Google libaddressinput: postal code patterns (Apache 2.0)
- Google libphonenumber: phone number patterns (Apache 2.0)

**Reference:**
- [TAXONOMY_COMPARISON.md](../../docs/TAXONOMY_COMPARISON.md) — format vs semantic classification analysis
