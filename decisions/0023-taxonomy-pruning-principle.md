---
status: accepted
date-created: 2026-03-03
date-modified: 2026-03-11
---
# 0023. Taxonomy pruning principle — remove types indistinguishable from generic

## Context and Problem Statement

As FineType's taxonomy grew, certain types were found to be false-positive magnets — their validation patterns matched generic integer or text inputs. For example, `port` (1-65535) matches any small integer, `http_status_code` (100-599) matches any 3-digit number, and `street_number` matches any short integer. These types generated more false positives than true positives in practice.

The question: when should a type be removed from the taxonomy, and what operational criteria justify removal?

## Considered Options

- **Keep types, add disambiguation rules** — More rules to prevent false positives. Increases pipeline complexity without addressing the fundamental problem (the type is indistinguishable from generic input at the character level).
- **Keep types, require context** — Only predict the type when header hints or column context confirms it. Viable but adds maintenance burden for low-value types.
- **Remove types that fail the Precision Principle** — "A validation that confirms 90% of random input is not a validation." If a type's validation pattern cannot meaningfully distinguish "is this type" from "is not this type", remove it.

## Decision Outcome

Chosen option: **Remove types that fail the Precision Principle**, applied across 3 cleanup rounds:

- **NNFT-192** (2026-03-03): Removed `street_number` (indistinguishable from integer) and `age` (205 SOTAB false positives). Added `numeric_code` as the proper home for digit strings that aren't integers.
- **NNFT-233** (2026-03-06): Removed 7 low-precision types: `pin`, `day_of_month`, `credit_card_network`, `os`, `programming_language`, `software_license`, `stage`. Recategorized color types.
- **NNFT-242** (2026-03-07): Removed `http_status_code` and `port` — integer-range types with no distinguishing character patterns.

Net: 19 types removed across 3 rounds, taxonomy went from 216 → 207 → 250 (with expansion filling the gaps).

### Consequences

- Good, because false-positive rates dropped dramatically — users no longer see `port` for every integer column
- Good, because the remaining types have meaningful validation contracts
- Good, because removal is reversible — types can be re-added with stricter validation patterns if a distinguishing signal is found
- Bad, because users who relied on `http_status_code` or `port` predictions lose them
- Bad, because the boundary between "prunable" and "keepable" is a judgment call — some types are borderline (e.g., `year` was kept because 4-digit pattern + range is somewhat distinguishing)
