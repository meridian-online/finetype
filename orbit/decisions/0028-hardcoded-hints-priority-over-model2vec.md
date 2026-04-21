---
status: accepted
date-created: 2026-03-03
date-modified: 2026-03-11
---
# 0028. Hardcoded header hints priority over Model2Vec semantic hints

## Context and Problem Statement

FineType has two header hint systems: hardcoded exact/substring match rules (`header_hint()`) and Model2Vec semantic similarity matching. Originally, Model2Vec took priority — the idea was that learned semantic matching would generalize better than curated rules.

In practice, Model2Vec occasionally overrode known-good hardcoded hints with incorrect semantic matches (e.g., matching "tracking_url" to a non-URL type because of semantic similarity with "tracking").

## Considered Options

- **Model2Vec first (original design)** — Learned matching takes priority. More general but occasionally wrong on well-known headers.
- **Hardcoded first, Model2Vec fallback** — Curated knowledge takes priority. Model2Vec only fires when no hardcoded hint matches.
- **Merged scoring** — Combine hardcoded and Model2Vec signals with weighted scoring. More complex, harder to debug.

## Decision Outcome

Chosen option: **Hardcoded first, Model2Vec fallback**, because curated domain knowledge should trump learned associations for known headers. If the pipeline knows exactly what "email" or "timestamp" or "postal_code" means, the semantic model shouldn't override that.

Model2Vec semantic hints remain valuable for novel headers not covered by hardcoded rules — they provide graceful degradation on unseen column names. The two systems are complementary, not competing.

This reflects the broader "rules over features" philosophy (decision-0011): transparent, debuggable curated knowledge before learned signals.

### Consequences

- Good, because known headers always produce correct hints — no regression from semantic model noise
- Good, because debugging is straightforward — check hardcoded rules first, then semantic matching
- Good, because adding new hardcoded hints is a targeted fix with predictable impact
- Bad, because hardcoded rules require manual curation — novel headers are only covered by Model2Vec
- Neutral, because the Sense model (decision-004) will eventually subsume both hint systems by treating the header as a direct model input
