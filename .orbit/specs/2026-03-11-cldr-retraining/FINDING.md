# CLDR-Enriched Retraining: Regression Analysis

**Tasks:** NNFT-157 (Phase 1), NNFT-158 (Phase 2), NNFT-159 (Phase 3), NNFT-160 (Phase 4), NNFT-161 (Phase 5)
**Date:** 2026-02-28
**Status:** Complete — model rolled back, findings documented

## Question

Can we improve CharCNN accuracy by retraining with CLDR-enriched training data (more locale diversity, 169 types including entity_name and paragraph)?

## Answer

**No — not without targeted fixes first.** The retrained model regressed from 116/120 to 107/120 on profile eval. The regression has three systemic causes that must be addressed before the next retraining attempt.

## Method

1. Downloaded CLDR v46.0.0 date/time patterns (2,823 date + 2,824 time patterns, 706 locales)
2. Expanded `locale_data.rs` from 12 to 31 locales with CLDR-sourced month/weekday names
3. Added `DateOrder` enum for locale-aware DMY/MDY/YMD date generation
4. Generated 84,500 training samples (169 types x 500 samples, seed 42)
5. Trained tiered model (`--seed 42 --epochs 10 --batch-size 64`)
6. Evaluated against profile eval baseline (116/120)
7. Rolled back to v0.3.0 snapshot after regression detected
8. Analysed all 9 regressions from training data, taxonomy, and tier graph

## Findings

### Result: 107/120 (9 regressions from 116/120 baseline)

| # | Column | Predicted | Expected | Root Cause |
|---|--------|-----------|----------|------------|
| 1 | web_analytics.request_url | URI | URL | URL/URI training overlap |
| 2 | web_analytics.url | URI | URL | URL/URI training overlap |
| 3 | ecommerce.tracking_url | URI | URL | URL/URI training overlap |
| 4 | covid_timeseries.Country | nationality | country | T1 routing: location → person |
| 5 | scientific_measurements.pressure_atm | latitude | decimal_number | T1 routing: numeric → coordinate |
| 6 | airports.name | last_name | full_name | T2 person boundary shift |
| 7 | airports.utc_offset | iso_8601_offset | utc | T0 routing + format mismatch |
| 8 | multilingual.country | entity_name | country | T1 routing: location → text |
| 9 | tech_systems.server_hostname | slug | hostname | Training diversity gap |

### Systemic Issue 1: URL/URI Training Data Overlap (3/9 regressions)

URL training data (500 samples) is 100% `http://` or `https://` URLs. URI training data (500 samples) is 37% `http/https` + 22% `mailto:` + 20% `ftp://` + 21% other schemes. The 185 overlapping http/https URI samples are structurally identical to URL samples — the CharCNN cannot learn to distinguish them.

```
URL samples:  https://word.tld/path/to/page  (all 500)
URI samples:  https://example.com/page        (185/500 = 37%)
              mailto:user@example.com          (110/500 = 22%)
              ftp://files.example.com/doc      (100/500 = 20%)
              tel:+1234567890, data:..., etc.  (105/500 = 21%)
```

The retrained model learned URI as the broader category and overcalled it on all http/https URLs.

### Systemic Issue 2: T1 VARCHAR Routing Degradation (3/9 regressions)

The T1 VARCHAR model routes values across 22 semantic categories. Wrong T1 routing means the value never reaches the correct T2 model — a single-point-of-failure with cascading consequences.

Three regressions were T1 routing errors:

- **Country → nationality** (T1 sent to `person` instead of `location`): Country training includes localised names ("Schweden", "Corée du Sud"). Nationality training includes demonyms ("Japanese", "Indisch"). Minimal textual overlap, but the retrained T1 model confused the categories.
- **Multilingual country → entity_name** (T1 sent to `text` instead of `location`): Test values are native-language country names — "Deutschland", "Brasil", "日本". CJK characters ("日本") look like text/entity patterns rather than location patterns to the retrained T1.
- **Pressure → latitude** (T1 DOUBLE sent to `coordinate` instead of `numeric`): Atmospheric pressure values (0.685, 2.395 atm) are small decimals within latitude range (-90 to +90). Structurally identical — distinction requires semantic context, not pattern matching.

### Systemic Issue 3: Training Data Diversity Gaps (3/9 regressions)

- **Hostname vs slug**: All 500 hostname training samples are simple two-part domains (`index.net`, `table.org`) — zero contain hyphens or subdomains. All 500 slug samples are hyphenated words (`data-sun-parse`, `old-summer`). Test hostname `srv-dev-43.example.com` has 3 hyphens, matching slug patterns.
- **UTC offset format mismatch**: All 500 UTC offset training samples are `UTC +05:30` format (with prefix). Test data is bare `+05:30`. The retrained T0 model routed bare offsets to TIMESTAMPTZ (where `iso_8601_offset` lives) instead of VARCHAR (where `utc` lives).
- **airports.name → last_name**: The retrained T2 VARCHAR/person model shifted its full_name/last_name decision boundary. Single-word place names like "Goroka" matched last_name patterns. This broke the entity demotion chain — entity demotion only fires when majority vote is `full_name`, not `last_name`.

## Categorisation

### Rule-fixable (addressable by disambiguation rules without retraining)

1. **URL/URI** — Add URL preference rule: when T2 predicts URI and all values use http/https, override to URL.
2. **UTC offset** — Expand Rule 17 to also trigger on TIMESTAMPTZ routing with bare offset patterns.
3. **airports.name** — Extend entity demotion to also fire on `last_name` majority vote. (Risky — may overcorrect legitimate last_name predictions.)

### Training-fixable (require training data improvements)

All 9 regressions are training-fixable:

1. **URL/URI**: Remove http/https from URI training. URI should only contain `mailto:`, `ftp://`, `tel:`, `data:`, `file://` schemes.
2. **Country/nationality**: Strengthen T1 location signal for localised country names.
3. **Pressure/latitude**: Add more diverse small-decimal numeric training or atmospheric pressure patterns.
4. **airports.name**: Ensure multi-word proper nouns beyond person names don't shift the full_name/last_name boundary.
5. **UTC offset**: Add bare offset patterns (`+05:30`) alongside `UTC +05:30`.
6. **Multilingual country**: Increase non-ASCII/CJK country name representation.
7. **Hostname/slug**: Add diverse hostname formats with subdomains, hyphens, server naming conventions.

### Taxonomy-fixable

1. **URL/URI**: Consider merging into a single type. The distinction is semantic (URLs are a subset of URIs), not structural. Or make URL a subtype of URI.

## Recommendations for Next Retraining

Priority-ordered by expected impact:

1. **Remove http/https from URI training data** — Eliminates 37% overlap. Expected: +3 columns.
2. **Add bare UTC offset patterns** — Include `+05:30`, `-08:00` alongside `UTC +05:30`. Expected: +1 column.
3. **Diversify hostname training** — Add subdomains, hyphens, server naming conventions. Expected: +1 column.
4. **Increase samples from 500 to 1000 per type** — More data should stabilise T1/T2 decision boundaries.
5. **Add disambiguation rules as safety net** — URL preference rule and expanded Rule 17 protect regardless of training quality.
6. **Review full_name/last_name training balance** — Ensure multi-word non-person strings stay as full_name predictions.

Estimated impact of training fixes alone: +5 to +7 of the 9 regressions addressable. Combined with disambiguation rules: all 9 potentially fixable.

## Infrastructure Retained

The following infrastructure from Phases 1-2 is committed and ready for the next attempt:

- `scripts/download_cldr.sh` — Downloads CLDR v46.0.0 JSON packages
- `scripts/extract_cldr_patterns.py` — Maps CLDR patterns to FineType types
- `crates/finetype-core/src/locale_data.rs` — 31 locales with CLDR-sourced month/weekday data
- `crates/finetype-core/src/generator.rs` — `DateOrder` enum, locale-aware date generation
- `data/cldr/README.md` — Attribution and data sources

The training data (`training_cldr_v1.ndjson`, 84,500 samples) is a generated artifact, not committed. Regenerate with `finetype generate -s 500 --seed 42 --priority 1 -o training_cldr_v1.ndjson` after applying fixes to generators.
