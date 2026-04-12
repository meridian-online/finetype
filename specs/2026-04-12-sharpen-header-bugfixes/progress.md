# Implementation Progress

**Spec:** specs/2026-04-12-sharpen-header-bugfixes/spec.yaml
**Started:** 2026-04-12

## Hard Constraints
- [x] v11 model weights frozen — no retraining, only Sharpen changes
- [x] Only fix bugs in existing hints/thresholds, plus add <=2 new header hints
- [x] No changes to feature_sharpen (F1-F6) or value_sharpen (R1-R19+)
- [x] All changes in crates/finetype-model/src/column.rs (header_hint and apply_header_sharpen)
- [ ] No actionability regression (>=99.8%, current v11 level) — gate, needs eval on Mac
- [x] Every fix must have a unit test
- [x] New hints must be unambiguous — exclude column names with multiple valid meanings

## Acceptance Criteria
- [x] ac-01: Fix header_hint() to exclude "bitcoin" from address keyword match — added !h.contains("bitcoin") + btc/crypto/wallet guards, test: ac01_bitcoin_address_not_captured_by_address_hint
- [x] ac-02: Fix header_hint() to return ip_v6 when header contains "v6" or "ipv6" — added v6 check before ip_v4 catch-all, test: ac02_ipv6_header_returns_ip_v6
- [x] ac-03: Fix two threshold paths in apply_header_sharpen() — (A) removed confidence threshold for same-category hardcoded overrides, (B) raised same-domain threshold 0.5→0.90, tests: ac03a/ac03b_*
- [x] ac-04: Add 2 hardcoded header hints: icao→geography.transportation.icao_code, author/authors→identity.person.full_name — tests: ac04_icao_header_hint, ac04_author_header_hint
- [ ] ac-05: Gate — profile eval >= 205/227 — needs eval on Mac with v11 model
- [ ] ac-06: Gate — actionability >= 99.8% — needs eval on Mac with v11 model
- [x] ac-07: Gate — all existing tests pass — 372 passed, 0 failed, 1 ignored

## Test Summary

8 new tests, all passing:
- ac01_bitcoin_address_not_captured_by_address_hint (7 assertions)
- ac02_ipv6_header_returns_ip_v6 (5 assertions)
- ac03a_same_category_hardcoded_override_unconditional (phone→ssn@1.00)
- ac03b_same_domain_hardcoded_override_below_090 (year→compact_ym@0.83)
- ac03b_same_domain_hardcoded_override_url_over_docker_ref (url→docker_ref@0.60)
- ac03b_same_domain_hardcoded_does_not_override_at_090 (regression guard)
- ac04_icao_header_hint (3 assertions)
- ac04_author_header_hint (3 assertions)

## Changes Made

All in `crates/finetype-model/src/column.rs`:

1. **header_hint()** line ~4027: Added `!h.contains("bitcoin") && !h.contains("btc") && !h.contains("crypto") && !h.contains("wallet")` to address keyword guard
2. **header_hint()** line ~3998: Added ipv6 check (`h.contains("v6") || h.contains("ipv6")`) before ip_v4 catch-all
3. **apply_header_sharpen()** line ~2245: Removed `result.confidence <= 0.80` threshold from same-category hardcoded check — now unconditional
4. **apply_header_sharpen()** line ~2319: Changed same-domain threshold from `0.5` to `0.90`
5. **header_hint()** exact match: Added "icao" | "icao code" → geography.transportation.icao_code
6. **header_hint()** exact match: Added "author" | "authors" | "author name" → identity.person.full_name

## Next Steps

- Push branch, pull on Mac, run profile eval with FINETYPE_MODEL_DIR=models/sherlock-v11
- Expected: ac-01 fixes 1, ac-02 fixes 1, ac-03 fixes 4, ac-04 fixes 2 = +8 → 211/227
- Conservative estimate with interactions: +5 → 208/227
