# Locale Detection Architecture

## Why CharCNN Can't Classify Locales (and What Can)

This document captures the findings from the tiered-v3 locale training experiment
(NNFT-126, February 2026) and the comparative analysis with the original finetype
prototype. It explains why we chose post-hoc locale detection over model-based
locale classification, and what would need to change to revisit that decision.

## The Experiment

We trained tiered-v3 with 4-level locale labels (e.g., `identity.person.phone_number.EN_US`)
to see if the CharCNN could learn to classify both type and locale in a single pass.

### Training Results

| T2 Model | v2 labels | v3 labels | v3 accuracy | Assessment |
|---|---|---|---|---|
| VARCHAR/person | 13 | 94 (7.2x) | 51% | Failed |
| VARCHAR/location | 5 | 65 (13x) | 55% | Failed |
| VARCHAR/address | 5 | 48 (9.6x) | 71% | Degraded |
| DATE/date | 17 | 53 (3.1x) | 65% | Degraded |
| VARCHAR/contact | — | 16 (new) | 88% | OK |
| VARCHAR/payment | 13 | 10 | 91% | Fine |
| Models with ≤10 labels | — | — | 95%+ | Fine |

**Profile eval regressed from 70/74 to 67/74.** The 3 regressions were all caused by
changed vote distributions at T2 — not code bugs. The expanded label space produced
different confusion patterns that shifted which type won the plurality vote, breaking
header hint disambiguation for 3 columns.

### The Capacity Ceiling

The CharCNN architecture (character-level CNN with 64 filters, 128 hidden dim)
handles ≤20 labels per T2 model very well, degrades between 20–50, and fails
beyond 50. This is a fundamental architectural limit, not a training data issue.

## CharCNN vs Transformer: The Architecture Gap

The original finetype prototype (`hughcameron/finetype`) used a **Transformer model**
(via the Burn framework) with 4-level locale-in-label classification — and it worked.
The key differences:

| Aspect | CharCNN (current) | Transformer (old prototype) |
|---|---|---|
| Architecture | Character-level CNN | Self-attention + positional encoding |
| Capacity | Low — fixed filter bank | High — attention can route information |
| Speed | ~1,500 val/sec (flat), ~580 (tiered) | Slower (exact numbers not benchmarked) |
| Label ceiling | ~20 labels per model | Much higher (hundreds) |
| What it sees | Local character n-grams | Global sequence patterns |

### Why the Difference Matters for Locales

**CharCNN operates on local character patterns.** It slides filters over character
n-grams and pools the results. This is excellent for structural patterns:

- Phone numbers have digits, parentheses, hyphens, plus signs
- Email addresses have @ symbols and dots
- IP addresses have digit groups separated by dots
- UUIDs have hex characters and dashes in fixed positions

These are **type-level** features. The character patterns for "phone number" vs "email"
are fundamentally different. CharCNN excels here.

But **locale-level** features in text types are distributed and overlapping:

- "John Smith" (EN) vs "Jean Dupont" (FR) vs "Hans Müller" (DE)
- The character distributions overlap heavily
- Distinguishing features (ü, ñ, ø) are sparse and unreliable
- Many names are used across multiple locales

A Transformer's self-attention mechanism can learn to weight rare distinguishing
characters in context. A CharCNN's fixed-width filters treat all positions equally
and can't attend to the one ü that distinguishes DE from EN.

**For phone numbers**, the locale distinction IS in character patterns (country codes,
digit grouping), but it's subtle — +1 (202) 555-0100 vs +44 20 7946 0958. The
CharCNN can partially learn this, but 17 locale variants of phone_number dilute
the training signal. Meanwhile, a regex pattern distinguishes them with 100% precision.

## The Principle: Right Tool for Each Job

This leads to a clean architectural separation:

### CharCNN → Type Classification

**"Is this a phone number or an email?"**

- Structurally distinct character patterns
- 13 labels at T2 VARCHAR/person → excellent accuracy
- The tiered architecture keeps each model's label count manageable
- Speed matters for batch processing

### Validation Patterns → Locale Detection

**"Is this a UK phone number or a US phone number?"**

- Precise structural rules (digit count, grouping, country code)
- `validation_by_locale` patterns match with near-perfect precision
- Each locale pattern is independently testable
- New locales are added incrementally without retraining
- The Precision Principle: "A validation that confirms 90% of random input is not
  a validation." Locale patterns are precise by design.

### The Composability Argument

This separation aligns with Meridian's second pillar: *Write programs that do one thing
and do it well.*

- The classifier does one thing: identify the semantic type
- The validator does one thing: confirm the locale
- Each can be improved independently
- Neither blocks the other

## When to Revisit This Decision

The decision to use post-hoc locale detection is optimal **given our current model
architecture**. It should be revisited if:

1. **We adopt a more capable model.** A Transformer, attention-augmented CNN, or
   even a lightweight attention layer on top of CharCNN could handle larger label
   spaces. The old prototype proves this works in principle.

2. **We need locale for types without validation patterns.** Post-hoc detection
   requires `validation_by_locale` patterns. Types like `full_name` or `occupation`
   don't have structural patterns that distinguish locales — their locale can only
   be inferred from the text content itself (which requires a more capable model)
   or from cross-column context (which is a different problem entirely).

3. **Performance requirements change.** If we need locale detection to be faster
   than running N locale patterns per value, a single model pass would be more
   efficient. But at current scale this isn't a bottleneck.

## Current Locale Coverage

As of NNFT-141, `validation_by_locale` patterns exist for 5 types:

| Type | Locales | Source |
|---|---|---|
| phone_number | 15 (EN, EN_AU, EN_GB, EN_CA, EN_US, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO, AR, ZA) | libphonenumber (Apache 2.0) |
| postal_code | 14 (US, GB, CA, DE, FR, AU, JP, BR, IN, IT, NL, ES, CH, SE) | Google libaddressinput (Apache 2.0) |
| calling_code | 17 (EN, EN_US, EN_CA, EN_GB, EN_AU, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO, AR, ZA) | ITU-T E.164 (public domain) |
| month_name | 6 (EN, FR, DE, ES, IT, PT) | Unicode CLDR (Unicode License) |
| day_of_week | 6 (EN, FR, DE, ES, IT, PT) | Unicode CLDR (Unicode License) |

The old prototype supported 36 Mimesis locales. The full locale list serves as a
roadmap for expanding validation coverage to more types and locales.

### Expansion Roadmap

22 types are designated `locale_specific` in the taxonomy. Of these, 5 now have
`validation_by_locale` patterns. Priority candidates for future expansion:

- **Addresses** (full_address, street_name) — complex, locale-specific ordering
- **Date formats with month names** (abbreviated_month, long_full_month) — CLDR abbreviated month names
- **Names** (full_name, first_name, last_name) — requires name databases, not structural patterns

## References

- **Decision record:** `decisions/0002-locale-detection-post-hoc-validation.md`
- **Old prototype:** `hughcameron/finetype` (Python + Burn Transformer)
- **Tiered-v3 training log:** NNFT-126 implementation notes
- **Precision Principle:** NNFT-132, decision-001
- **Locale validation infrastructure:** NNFT-118, NNFT-121, NNFT-136, NNFT-141
