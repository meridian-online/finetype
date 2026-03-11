---
status: accepted
date-created: 2026-02-25
date-modified: 2026-03-11
---
# 0001. Validation Precision — locale-only confirmation for locale-specific types

## Context and Problem Statement

FineType's taxonomy marks certain types as `designation: locale_specific` (e.g., postal_code, phone_number, calling_code). These types have a universal `validation` regex that is intentionally broad, plus precise `validation_by_locale` patterns for specific locales (e.g., US 5-digit ZIP, UK postcode format).

The problem: should the broad universal validation pattern be allowed to *confirm* a classification, or should it only be able to *reject* implausible values?

A permissive universal pattern (e.g., "any 3-10 digit string" for postal_code) confirms 90% of random integer input — violating the Precision Principle that "a validation that confirms 90% of random input is not a validation."

## Considered Options

- **Option A — Universal validation confirms and rejects equally.** Simpler implementation. Risk: false positives on integer columns misclassified as postal codes.
- **Option B — For locale-specific types, universal validation can only reject; locale validation confirms.** More precise. Requires the attractor demotion system (Rule 15) to treat locale-confirmed predictions differently.

## Decision Outcome

Chosen option: **Option B — locale validation confirms, universal validation can only reject**, because it enforces the Precision Principle. If a type is designated `locale_specific`, its real validation lives in `validation_by_locale`, not the universal `validation` block.

This means:
- Locale-confirmed predictions skip attractor demotion Signals 2-3 (confidence, cardinality)
- Universal validation failure still demotes (Signal 1)
- Types without any locale match remain subject to full attractor demotion

### Consequences

- Good, because false positives on integer-like columns (postal_code, calling_code) are dramatically reduced
- Good, because expanding locale coverage directly improves precision — each new locale pattern adds a confirmation signal
- Bad, because types with no `validation_by_locale` patterns yet (e.g., a newly added locale-specific type) get no confirmation at all until locale patterns are added
- Neutral, because this creates a dependency between taxonomy coverage and classification quality — but that dependency already existed implicitly
