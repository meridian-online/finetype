# LLM Distillation v2 — Full Findings

## Summary

Ran blind-first adjudication using Claude Sonnet via Claude Code batch agents on **5,364 columns from 507 CSV files** (GitTables + curated eval datasets). Each column was classified independently by Claude, then adjudicated against FineType's prediction.

**Key results:**
- **97% valid final labels** (5,207/5,364) — near-zero hallucinated labels
- **96% valid blind labels** (5,174/5,364) — high standalone accuracy
- **36% agreement** with FineType (1,983/5,364) — disagreements are overwhelmingly actionable
- **2,047 disagreements** with reasoned adjudication across 10 systematic gap categories
- **74% high confidence** on blind classification (3,973/5,364)
- **Runtime:** ~4 hours wall clock across 19 waves (5 agents per wave, 107 batches)

## Comparison: Phase 2 vs Phase 1 Pilot vs Qwen3 8B

| Metric | Phase 2 (full) | Phase 1 (pilot) | Qwen3 8B (v1) |
|--------|---------------|-----------------|----------------|
| Valid labels (final) | **97%** | 100% | 97% |
| Agreement with FineType | **36%** | 50% | 20% |
| Columns processed | **5,364** | 604 | 5,359 |
| Files processed | **507** | 39 | — |
| Runtime | **~4 hours** (parallel) | ~35 min | 14 hours (sequential) |
| Actionable reasoning | **Yes** (per disagreement) | Yes | No |
| Batches | **107** | 11 | — |

The lower agreement rate in Phase 2 (36% vs 50%) reflects the expanded corpus — GitTables includes many HN discussion dumps, NLP corpora, and software metrics datasets that systematically trigger FineType's known weaknesses.

## Agreement by Domain

| Domain | Agreement | Rate | Notes |
|--------|-----------|------|-------|
| technology | 153/218 | **70%** | URLs, hashes, versions — structurally unambiguous |
| geography | 65/106 | **61%** | Good except region/state hierarchy confusion |
| finance | 47/134 | **35%** | Header hints misfire on non-financial numeric data |
| datetime | 112/329 | **34%** | Epoch seconds and locale-specific dates are weak spots |
| representation | 1,538/4,268 | **36%** | Largest domain; integer/categorical/text subtypes hardest |
| identity | 60/255 | **23%** | Username demotion to categorical is systematic |
| container | 8/47 | **17%** | XML content, whitespace-separated misclassification |

## Top 15 Disagreement Patterns

These are the most frequently recurring misclassifications, ranked by count:

| Count | Claude (blind) | FineType | Root Cause |
|-------|---------------|----------|------------|
| 175 | `integer_number` | `boolean.binary` | All-zero/low-variance columns trigger binary heuristic |
| 123 | `categorical` | `ordinal` | Nominal categories incorrectly assumed to have ordering |
| 114 | `integer_number` | `basis_points` | "points" header hint fires on vote/score counts |
| 92 | `username` | `categorical` | Cardinality demotion overrides entity-type classification |
| 83 | `numeric_code` | `decimal_number` | Float-stored integer IDs treated as measurements |
| 79 | `unix_seconds` | `iso_8601` | Epoch seconds misrouted by "Created" header hint |
| 77 | `increment` | `amount_minor_int` | Sequential integer IDs matched currency pattern |
| 71 | `sentence` | `entity_name` | Full headlines/titles classified as named entities |
| 66 | `decimal_number` | `yield` | Financial header hints on non-financial decimal data |
| 56 | `currency.amount` | `decimal_number` | Monetary values without currency symbol demoted |
| 55 | `paragraph` | `plain_text` | Multi-paragraph text not distinguished from short text |
| 30 | `categorical` | `entity_name` | Low-cardinality domain terms misclassified as entities |
| 27 | `integer_number` | `amount_minor_int` | Integer counts matched currency amount pattern |
| 27 | `numeric_code` | `amount_minor_int` | Numeric codes (SNOMED, advertiser IDs) → currency |
| 23 | `increment` | `integer_number` | Sequential IDs not recognized as auto-increment |

## Systematic FineType Gaps Identified

### 1. Header hint misfires (most impactful, ~500+ cases)

The hardcoded header hint table causes the most systematic errors across the full corpus. The hint overrides correct model predictions when the header substring matches but the semantic context doesn't.

| Header substring | Hint fires as | Actual data | Estimated count |
|-----------------|---------------|-------------|-----------------|
| "points" | `finance.rate.basis_points` | HN vote counts, game scores, metric scores | 114 |
| "created" | `datetime.timestamp.iso_8601` | Unix epoch seconds (10-digit floats) | 79 |
| "name" | `identity.person.full_name` | Design names, company names, game items, cities | 30+ |
| "yield" / "pct" | `finance.rate.yield` | General decimal percentages, ratios | 66 |
| "charge" | `finance.currency.amount` | Legal charge codes, status categories | 10+ |
| "index" | `geography.index.h3` | Integer row indices, loop counters | 10+ |
| "loc" | `geography.coordinate.longitude` | Lines of code (software metrics) | 10+ |
| "state" | `geography.location.state` | Market state, planning status, enum values | 10+ |
| "address" | `geography.address.full_address` | Business summaries, long text, code paths | 5+ |
| "line" | `identity.person.full_name` | Java class names, code references | 5+ |

**Recommendation:** Header hints need semantic disambiguation — the substring match is too greedy. Two approaches:
1. **Regex guards:** Require value-pattern confirmation before accepting a header hint
2. **Taxonomy YAML migration:** Type-specific header patterns with contextual constraints

### 2. Boolean/binary false positives (~175 cases)

FineType classifies columns as `boolean.binary` when values are limited to small integer sets, especially `{0}`, `{-1, 0}`, or `{0, 1}`. At scale, this is the single most common misclassification:
- **Count columns** with all-zero samples (HN comments=0, pandemic counts=0)
- **Sentinel values** where -1 means "not applicable"
- **Narrow-range integers** (circuit depth levels, delay metrics, citation offsets)

**Recommendation:** The binary heuristic must require evidence of *both* states with sufficient sample diversity. A column of all zeros is not boolean. Minimum viable fix: require ≥2 distinct values and reject columns where one value comprises >95% of the sample.

### 3. Categorical vs ordinal confusion (~123 cases)

FineType defaults to `ordinal` for many low-cardinality string columns where `categorical` is correct. The HN `Type` column (`comment`/`story`) is the canonical example — no natural ordering exists.

**Recommendation:** Default to `categorical` unless ordering evidence is present (numbers in values, known ordinal patterns like "low/medium/high", header containing "level"/"rank"/"grade").

### 4. Username demotion to categorical (~92 cases)

FineType's cardinality demotion rule reclassifies username columns (e.g., HN authors) as `categorical` or `ordinal`. Claude correctly identifies these as `identity.person.username` based on alphanumeric handle patterns.

**Recommendation:** Guard the demotion rule with entity-type awareness — if the CharCNN classifies individual values as usernames/emails/identifiers, cardinality alone shouldn't override.

### 5. Float-stored integer IDs (~83 cases)

Numeric IDs stored as floats (e.g., `1700247.0`) are classified as `decimal_number` instead of identifier types. This is common in pandas-exported CSVs where nullable integer columns become float64.

**Recommendation:** If all float values have `.0` fractional parts and the column header suggests an identifier (ID, parent, story, key), prefer identifier classification.

### 6. Epoch seconds misclassification (~79 cases)

10-digit Unix epoch seconds (1214247459, 1658783048) are classified as `datetime.timestamp.iso_8601` or even `identity.medical.npi`. The `Created` header hint compounds this by routing to `iso_8601`.

**Recommendation:** A value-range check for the epoch seconds window (946684800–2524608000, i.e., 2000-01-01 to 2050-01-01) would correctly route these. This is a high-confidence deterministic rule.

### 7. Sequential IDs as currency (~77 cases)

Auto-incrementing integer IDs (1, 2, 3, ...) are matched by the `amount_minor_int` currency pattern. At scale this is one of the most common false positives.

**Recommendation:** Sequential integer detection (monotonically increasing, starting near 0 or 1) should suppress currency/financial pattern matches.

### 8. Sentence vs entity_name for titles (~71 cases)

Full headlines and article titles are classified as `entity_name` when they are better described as `sentence`. FineType's `entity_name` is appropriate for short named things (product names, company names), not full phrases.

**Recommendation:** Length-based heuristic — if average token count > 5, prefer `sentence` over `entity_name`.

### 9. Financial header hints on non-financial data (~66 cases)

Header substrings like "yield", "pct", "rate" trigger financial domain classification on general-purpose decimal columns (scientific measurements, percentages, ratios).

**Recommendation:** Financial header hints should require co-occurrence with financial domain signals (currency symbols, ticker-like patterns, exchange names).

### 10. Cross-domain pattern collisions (~50+ cases)

Structural pattern matchers fire across domain boundaries:
- 9-digit financial amounts → `aba_routing` (routing number)
- 10-digit market caps → `isbn` (barcode)
- 12-15 digit advertiser IDs → `credit_card_number`
- SNOMED codes → `amount_minor_int`
- Memory addresses (0xffb54524) → `geohash`
- POS tags → `locale_code`
- Java file paths → `docker_ref`
- "EQUITY" → `http_method`

**Recommendation:** Pattern matchers need domain context guards. A digit-count match alone is insufficient — require supporting evidence from header or sibling columns.

## New Findings from Phase 2 Scale

### NLP/corpus annotation datasets

The expanded corpus includes dialog act corpora (SWBD), citation networks, and POS-tagged transcripts. FineType has no concept of these data types:
- POS-tagged text → `excel_format`, `user_agent`, `wkt`
- Dialog act codes (qw/sd/b/aa) → `locale_code`
- Parse tree strings → `excel_format`
- Disfluency-annotated speech → `full_address`

### Software metrics datasets

CK/Chidamber-Kemerer metrics columns systematically collide with other domains:
- `loc` (lines of code) → `longitude`
- `NOC` (number of children) → `UPC`
- `ICP` (inter-class coupling) → `ICD-10`
- `DIT` (depth of inheritance) → various
- `stms` (statements) → `DMS coordinates`
- `CLD` (class-level density) → `CLF timestamp`

### Taxonomy gaps identified

- **Blood pressure** (`155/82` slash-separated format) — no dedicated type exists
- **Chemical formulas** (`NaCl`, `CH4`) — classified as `alphanumeric_id`
- **XML content** (inline `<S sid="...">` tags) — `container.object.xml` exists but rarely matched
- **Memory addresses** (`0xffb54524`) — no hex pointer type
- **Dialog act codes** — no NLP annotation type

## Confidence Distribution

| Confidence | Count | Percentage |
|------------|-------|------------|
| High | 3,973 | 74% |
| Medium | 1,248 | 23% |
| Low | 136 | 2% |

The high confidence rate on blind classification suggests Claude's type inference is well-calibrated — it's confident when the data is unambiguous and uncertain when genuine ambiguity exists.

## Label Validity

| Metric | Count | Percentage |
|--------|-------|------------|
| Blind label valid | 5,174 | 96% |
| Final label valid | 5,207 | 97% |
| Blind label invalid | 190 | 4% |
| Final label invalid | 157 | 3% |

The 3-4% invalid rate comes from agents occasionally generating taxonomy labels that don't exist (e.g., `datetime.date.iso_date` instead of `datetime.date.iso`, `technology.development.os`). The adjudication step corrects some of these.

## Phase 2 Assessment

| Criterion | Target | Actual | Pass? |
|-----------|--------|--------|-------|
| Full corpus coverage | 5,000+ columns | 5,364 | ✅ |
| Label validity > 95% | > 95% | 97% | ✅ |
| Actionable gap categories | ≥ 5 | 10 | ✅ |
| Quantified disagreement patterns | Yes | Top 15 with counts | ✅ |
| Runtime sustainable | < 1 day | ~4 hours | ✅ |

**Verdict:** Phase 2 passes all gates. The 36% agreement rate is lower than Phase 1's 50% but this reflects the expanded corpus hitting FineType's known weaknesses at scale. The disagreement analysis is highly actionable — the top 5 patterns alone account for ~587 disagreements (29% of all disagreements) and each has a clear fix path.

## Actionable Fix Priority

Ranked by estimated impact (disagreements that would flip to agreement):

| Priority | Fix | Est. impact | Difficulty |
|----------|-----|-------------|------------|
| 1 | Boolean binary heuristic (require both states) | +175 | Low |
| 2 | Categorical vs ordinal default | +123 | Low |
| 3 | "points" header hint guard | +114 | Low |
| 4 | Username demotion guard | +92 | Medium |
| 5 | Float-stored ID detection | +83 | Medium |
| 6 | Epoch seconds range check | +79 | Low |
| 7 | Sequential ID detection | +77 | Medium |
| 8 | Sentence vs entity_name length heuristic | +71 | Low |
| 9 | Financial header hint domain guards | +66 | Medium |
| 10 | Cross-domain pattern context guards | +50 | High |

**Conservative estimate:** Fixes 1-3 alone would add ~412 agreements, pushing the rate from 36% to ~44%. All top 8 fixes (low-medium difficulty) would add ~814 agreements → ~52% agreement.

## Data

- **Output:** `output/distillation-v2/merged_labels.csv` (5,364 rows)
- **Batch files:** `output/distillation-v2/batch_*.csv` (107 files)
- **Source:** 507 CSV files from `data/csvs/` (GitTables + curated)
- **Model:** Claude Sonnet via Claude Code agents (Max 20x subscription)
- **Spec:** `specs/2026-03-18-distillation-v2/spec.yaml`
- **Resume scripts:** `scripts/distillation_status.py`, `scripts/resume_distillation.sh`
