# Rare-type scoreboard canonicalisation (choice 0093 follow-up)

**Date:** 2026-06-10 · **Method:** the 195-row review sample
(`review_sample.csv`) adjudicated by two independent agent panels (one
prompted as skeptic), the same process the author calibrated on the gold
corpus queue (40/40 spot-check, panel-author agreement >= 0.91 at 95%).
**Result file:** `review_sample_verified.csv`.

## Verdict

**The scoreboard's header-anchored gold logic is 193/195 = 99.0% accurate
on its own sample (panel agreement 195/195, zero splits).**

Per pool: latitude negatives 80/80 hold; utc negatives 35/35 hold; url
positives 8/8 hold; url negatives 70/72 — the two failures are columns of
protocol-relative links (`//www.crunchbase.com/...`, `//purl.org/...`),
which ARE url columns the scoreboard's negative pool wrongly admits.

## Consequences

1. **Choice 0093's open caveat closes:** the scoreboard graduates from
   "relative comparator + NO-GO early-warning" to a validated absolute
   instrument at the ~99% level on the contested types it covers.
2. **One refinement recommended:** treat `//`-prefixed values as
   url-positive in the scoreboard's value predicates (and in the gold
   lens), so protocol-relative columns stop polluting the negative pool.
3. The verified sample is fold-in evidence for the gold corpus spec's
   ac-04 (task t-0001692818b74c8d50b76340).
