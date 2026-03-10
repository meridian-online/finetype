# LLM Distillation Findings — NNFT-269

## Summary

Ran Qwen3 8B via Ollama on 5,359 columns from 508 CSV files (96% GitTables, 4% FineType synthetic/curated). The LLM classified each column into FineType's 250-type taxonomy using header + 10 sample values.

**Results:**
- **97% valid labels** (5,234/5,359) — model outputs valid taxonomy labels
- **20% exact agreement** (1,056/5,234) with FineType predictions
- **125 invalid labels** — plausible hallucinations (e.g., `datetime.date.year` instead of `datetime.component.year`)
- **Runtime:** 832 minutes (~14 hours) on M1 MacBook at 0.1 col/s

## Agreement by Domain

| Domain | Agreement | Rate | Notes |
|--------|-----------|------|-------|
| technology | 170/274 | 62% | URLs, UUIDs, hashes — structurally unambiguous |
| container | 21/52 | 40% | Small sample |
| identity | 76/285 | 26% | Good on email/phone, weak on subtypes |
| datetime | 77/336 | 22% | Confuses epochs/timestamps, compact formats |
| representation | 645/3,593 | 17% | Can't do fine-grained numeric/text subtypes |
| geography | 57/373 | 15% | Misses region/country subtypes |
| finance | 10/321 | 3% | Can't distinguish basis_points/yield/amount types |

## Systematic Disagreement Patterns

### 1. LLM defaults to `container.array.*` (1,744 cases)

The 8B model's biggest failure mode. It classifies single words, booleans, ordinals, and entity names as `container.array.comma_separated` or `container.array.whitespace_separated`.

Examples:
- `["U", "U", "U"]` → LLM: `comma_separated`, FT: `ordinal`
- `["airport", "airport"]` → LLM: `comma_separated`, FT: `ordinal`
- `["true", "false", "true"]` → LLM: `whitespace_separated`, FT: `boolean.binary`

FineType correctly classifies these as: `boolean.binary` (332), `entity_name` (269), `integer_number` (173), `ordinal` (169), `numeric_code` (108).

### 2. LLM can't distinguish FineType's representation subtypes

The `representation.*` domain (3,593 columns, 17% agreement) contains FineType's most granular type distinctions — types that require statistical or structural reasoning:

| FineType type | What it means | LLM tends to say |
|---------------|---------------|------------------|
| `increment` | Sequential IDs (1,2,3…) | `integer_number` or `ssn` |
| `numeric_code` | Codes with leading zeros | `integer_number` |
| `ordinal` | Small-domain ordered values | `container.array.*` |
| `categorical` | Low-cardinality strings | `container.array.*` or `username` |
| `binary` | true/false, 0/1, yes/no | `container.array.*` |
| `entity_name` | Named entities (companies, places) | `container.array.*` or `full_name` |

These types require column-level statistical reasoning (cardinality, distribution shape, sequential patterns) that a per-column prompt can't provide.

### 3. Integer vs decimal — the ".0" problem (325 cases)

Biggest single disagreement: LLM says `integer_number`, FineType says `decimal_number` for values like `["225422.0", "225254.0"]`. These are integers stored as floats in pandas/CSV export. FineType is technically correct (the values contain a decimal point), but both are defensible.

### 4. Where the LLM is genuinely better

~990 cases where the LLM gives a more specific label than FineType:

| LLM label | FineType label | Count | Assessment |
|-----------|---------------|-------|------------|
| `finance.currency.amount` | `decimal_number` | 64 | **LLM correct** — `price_usd`, `total_price` columns |
| `identity.person.username` | `categorical` | 115 | **LLM correct** — GitHub usernames like `st3fan`, `icco` |
| `text.paragraph` | `plain_text` | 71 | **LLM arguably better** — multi-sentence descriptions |
| `container.object.html` | `plain_text` | 57 | **LLM correct** — values contain HTML tags |
| `identity.person.full_name` | `entity_name` | 16 | **Debatable** — book titles aren't person names |
| `datetime.component.day_of_week` | `categorical` | 18 | **LLM wrong** — values are `["F", "C", "G"]` (basketball positions, not days) |

The amount/username/HTML cases are genuinely valuable — they reveal where FineType's header hints or Sense classifier could be improved.

### 5. Invalid label patterns (125 cases)

| Hallucinated label | Count | Closest valid label |
|-------------------|-------|---------------------|
| `identity.person.comment` | 27 | `representation.text.plain_text` |
| `finance.currency.amount_number` | 15 | `finance.currency.amount` |
| `datetime.date.year` | 13 | `datetime.component.year` |
| `identity.person.player_id` | 13 | `representation.identifier.increment` |
| `representation.file.path` | 10 | `technology.internet.url` |
| `identity.person.team_abbreviation` | 7 | `representation.discrete.categorical` |

Most are 1-2 edits from a valid label. A fuzzy matcher (Levenshtein distance ≤ 3) would rescue ~60% of invalids.

## Ceiling Assessment

Qwen3 8B is a **good coarse classifier** but a **poor fine-grained classifier** for FineType's 250-type taxonomy:

- **Strong (>50% agreement):** technology domain — structurally unambiguous types
- **Moderate (20-40%):** identity, datetime, container — gets common types right, misses subtypes
- **Weak (<20%):** representation, geography, finance — can't reason about distributions, cardinality, or domain-specific semantics

The 8B model lacks the capacity to:
1. Distinguish statistical types (ordinal vs categorical vs binary) from 10 sample values
2. Recognize FineType-specific types (increment, numeric_code, entity_name)
3. Apply domain knowledge to finance types (basis_points vs yield vs amount)

## Recommendations for Scaling

### Option A: Larger local model (Qwen3 32B/72B)

**Setup:** Cloud VM with GPU (e.g., GCP `g2-standard-8` with L4 GPU, ~$1.50/hr)

| Model | VRAM needed | Expected speed | Cost for 50K columns |
|-------|-------------|----------------|---------------------|
| Qwen3 32B Q4 | ~20 GB | ~2-3 col/s | ~$7 (5 hrs) |
| Qwen3 72B Q4 | ~42 GB | ~0.5-1 col/s | ~$21 (14 hrs) |

**Expected improvement:** 32B would likely fix the `container.array.*` catch-all problem and improve finance/geography accuracy. The invalid rate should drop below 1%. Estimated agreement: 35-45%.

The 72B is probably not worth the cost increase — the fundamental limitation is that LLMs reason about types differently than FineType's statistical pipeline.

**VM setup:**
```bash
# GCP L4 GPU instance (~$1.50/hr)
gcloud compute instances create llm-labeller \
  --machine-type=g2-standard-8 \
  --accelerator=type=nvidia-l4,count=1 \
  --boot-disk-size=100GB \
  --image-family=ubuntu-2204-lts

# Install Ollama + pull model
curl -fsSL https://ollama.com/install.sh | sh
ollama pull qwen3:32b

# Run labelling
python3 scripts/llm_label.py /data/csvs/ output/llm_labels_32b.csv --model qwen3:32b
```

### Option B: Two-stage prompting

Instead of asking for one of 250 labels, split into two stages:

1. **Stage 1 — Domain classification (7 choices):** "Is this column: container, datetime, finance, geography, identity, representation, or technology?"
2. **Stage 2 — Type classification (≤84 choices):** "Given this is a {domain} column, classify it as one of: {domain_types}"

This reduces the label space per prompt from 250 to max 84, likely improving both accuracy and speed. Could be implemented with the existing script by adding a `--two-stage` flag.

### Option C: API-based (Claude/GPT-4o)

| Provider | Model | Cost per 50K columns | Speed |
|----------|-------|---------------------|-------|
| Anthropic | Claude Sonnet | ~$15-25 | ~5-10 col/s |
| OpenAI | GPT-4o-mini | ~$5-10 | ~10-20 col/s |
| OpenAI | GPT-4o | ~$30-50 | ~5-10 col/s |

**Pros:** No infrastructure, faster, likely higher quality. **Cons:** Cost, data leaves local machine, rate limits.

### Option D: Constrained decoding (Ollama grammar)

Ollama supports JSON schema / grammar-based output. Force the model to only output valid labels:

```json
{
  "format": {
    "type": "string",
    "enum": ["container.array.comma_separated", "container.array.pipe_separated", ...]
  }
}
```

This would eliminate all 125 invalid labels (100% valid rate) and may improve accuracy by preventing the model from "almost" picking a valid label. Can be combined with any model size.

### Recommended path

1. **Immediate:** Add constrained decoding to the script (Option D) — free, eliminates invalids
2. **Next:** Run Qwen3 32B on cloud VM (Option A) with two-stage prompting (Option B) on 50K GitTables columns — ~$10, ~5 hours
3. **Evaluate:** Compare 8B vs 32B agreement rates. If 32B reaches >40% agreement, the disagreement data becomes a valuable training signal for FineType improvements
4. **Long-term:** The LLM labels are most useful as a *complementary signal* — not as ground truth, but as a second opinion to identify where FineType's pipeline has systematic gaps (especially header hint coverage for finance/geography)

## Data

- **Output:** `output/llm_labels.csv` (5,359 rows)
- **Source:** 508 CSV files from `data/csvs/` (96% GitTables, 4% synthetic)
- **Model:** Qwen3 8B via Ollama (thinking disabled, temperature=0)
- **Script:** `scripts/llm_label.py`
