# Spike: Header Hint Sources

**Date:** 2026-03-18
**Purpose:** Determine whether the problematic header hints come from the hardcoded `header_hint()` table or the model2vec semantic classifier, to inform the taxonomy YAML migration approach.

## Method

Profiled test CSVs with the exact headers from the distillation findings. Examined `disambiguation_rule` in JSON output to identify which system produced the hint.

## Findings

### Source mapping for each problematic hint

```
| Header      | Hint fires as              | Source                  | Disambiguation rule                    |
|-------------|----------------------------|-------------------------|----------------------------------------|
| "points"    | finance.rate.basis_points  | model2vec               | sense_header_hint_generic:points       |
| "yield"     | finance.rate.yield         | model2vec               | sense_header_hint_generic:yield        |
| "pct_change"| finance.rate.yield         | model2vec               | sense_header_hint_generic:pct_change   |
| "created"   | identity.medical.npi       | NO HINT (CharCNN only)  | (none)                                 |
| "created_at"| identity.medical.npi       | NO HINT (CharCNN only)  | (none)                                 |
| "created_date"| datetime.timestamp.iso_8601| hardcoded `h.contains("date")` | sense_header_hint_cross_domain |
| "timestamp" | datetime.timestamp.iso_8601| hardcoded `h.contains("timestamp")` | sense_header_hint_cross_domain |
| "score"     | representation.discrete.ordinal | hardcoded exact match | sense_header_hint_generic:score       |
| "charge"    | finance.currency.amount    | hardcoded `h.contains("charge")` | (not tested, from code review)  |
```

### Key insight: Two distinct problems per fix

**Fix 3 ("points" → basis_points) and Fix 9 ("yield"/"pct" → financial types):**
- Source: **model2vec semantic classifier**, NOT `header_hint()`
- The model2vec label index contains `finance.rate.basis_points` and `finance.rate.yield`
- When model2vec returns these labels with similarity above threshold (0.65), and the CharCNN result is "generic", the hint overrides
- **Taxonomy YAML migration doesn't help here** — there's no hardcoded entry to move
- Fix approach: either (a) remove these labels from the model2vec label_index, (b) add value-pattern post-guards in the pipeline, or (c) adjust the similarity threshold

**Fix 6 ("created" → epoch seconds misrouted):**
- TWO sub-cases:
  1. Bare "created" / "created_at" — **no hint fires at all**. The CharCNN classifies 10-digit integers as NPI. This is a CharCNN/disambiguation problem, not a header hint problem.
  2. "created_date" / "timestamp" — hardcoded `h.contains("date")` and `h.contains("timestamp")` fire, routing to `iso_8601` when values are actually epoch seconds. This IS a hardcoded hint problem.
- Fix approach for case 1: value-range detection rule for epoch seconds (946684800–2524608000) in the disambiguation pipeline
- Fix approach for case 2: either add epoch detection before the hint fires, or make the hint epoch-aware

### Impact on PR-1 grouping

The reviewer was right: **taxonomy YAML migration is not the right approach for fixes 3 and 9**. Those hints come from model2vec, and the fix must either:
1. Remove/restrict the problematic labels in the model2vec training data (requires retraining)
2. Add post-classification value-pattern guards in the pipeline (code change in column.rs)
3. Add a blocklist/allowlist mechanism that taxonomy YAML can feed into

Option 2 (value-pattern guards) is the pragmatic choice — it's the "regex guards in code" approach from interview Q7, just applied to model2vec output instead of hardcoded hints.

### Revised understanding of header_hint() vs model2vec

```
| System                | What it covers                                        | How to fix problems |
|-----------------------|-------------------------------------------------------|---------------------|
| header_hint()         | ~80 exact + ~30 substring rules in Rust code          | Edit Rust match arms, or migrate to YAML |
| model2vec             | 250 type labels via cosine similarity (threshold 0.65)| Retrain, adjust threshold, or add post-guards |
```

The header_hint() table is actually well-curated (NNFT-065/091/102/127/128/156/254 iterations). The problems in the distillation are overwhelmingly from **model2vec** overriding correct CharCNN predictions when the header semantically matches a financial type.

### Recommendation

1. **Drop the taxonomy YAML migration from PR-1 scope.** It's solving the wrong problem — the misfiring hints aren't in `header_hint()`.
2. **Add value-pattern post-guards for model2vec financial hints.** When model2vec suggests basis_points/yield, check if values actually look financial before accepting.
3. **Add epoch seconds value-range detection** as a disambiguation rule (covers both bare "created" and "created_date" cases).
4. **Reconsider PR-1 grouping:** The header hint fixes are really pipeline guard fixes, not a taxonomy migration.

## Test commands

```bash
# Reproduce: model2vec routes "points" to basis_points
echo -e "100\n200\n50\n75\n150" > /tmp/vals.txt
# Profile with problematic headers
cargo run --release --bin finetype -- profile --file /tmp/hint_test.csv -o json
```
