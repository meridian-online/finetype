---
status: accepted
date-created: 2025-12-15
date-modified: 2026-03-11
---
# 0012. Taxonomy label format — domain.category.type dotted hierarchy

## Context and Problem Statement

FineType needed a labelling scheme for its type taxonomy. The label format determines how types are organized, referenced in code, displayed to users, and used for evaluation. It must support hierarchical grouping (broad domains down to specific types), be human-readable, and enable programmatic operations like domain extraction and category filtering.

## Considered Options

- **Flat labels** — e.g., `email`, `phone_number`, `iso_8601`. Simple but no hierarchy — makes disambiguation, evaluation by domain, and taxonomy navigation harder.
- **Dotted hierarchy (domain.category.type)** — e.g., `identity.person.email`, `datetime.timestamp.iso_8601`. Three levels: 7 domains → 43 categories → 250 types. Enables domain-level evaluation, category-based output masking, and natural organization.
- **Slash-separated paths** — e.g., `identity/person/email`. Functionally equivalent to dotted but conflicts with filesystem conventions and URL parsing.

## Decision Outcome

Chosen option: **Dotted hierarchy (domain.category.type)**, because it provides natural grouping for evaluation (report accuracy by domain), enables the Sense→Sharpen output masking pipeline (mask by category), and is human-readable in CLI output.

Locale is a YAML field on the type definition, not encoded in the label — see decision-002 for why model-level locale classification was rejected in favor of post-hoc detection.

### Consequences

- Good, because domain-level accuracy metrics (98.4% domain accuracy) give meaningful signal even when leaf-type accuracy is lower
- Good, because category-based output masking (LabelCategoryMap) enables the Sense→Sharpen pipeline without retraining
- Good, because hierarchical labels enable the tree softmax classification head (decision-0013)
- Bad, because 3-level labels are verbose in CLI output — `identity.person.email` vs just `email`
- Neutral, because the hierarchy is derived from labels at build time (string splitting), not from a separate graph structure
