# ac-01 — calibration map / abstention policy (design for approval)

**Spec:** 2026-06-18-calibrated-confidence-abstention (GATE, doc)
**Date:** 2026-06-18 · inputs: `ac00_reliability_finding.md`
**Status:** design — needs author sign-off on the analyst-facing behaviour before ac-02 ships code.

## Recommendation in one line

**Add a `quality_band` (high / medium / low) to the profile output, plus the
runner-up type on the `low` band — a presentation layer over the existing
confidence. No calibration map, no conversion to `unknown`, no model change.**

## The two design forks, decided from the ac-00 data

### (a) Calibration map? — NO (not now)

The reliability curve is monotonic but over-confident (0.99 bin → 0.85 actual on
repr). A band cut reads only the *ranking*, which is sound, so it does not need a
map. An isotonic/Platt map would be required only to surface an honest *numeric*
"calibrated confidence" — and it would have to be fit around the rule/hint
confidence pollution (the gold [0.70,0.85) dip). **Defer the numeric map**; keep the
raw `confidence` field as-is for backwards compatibility and add the band beside it.
A calibrated numeric field can be a later increment if an analyst asks for the number.

### (b) Abstention = convert to `unknown`? — NO. Flag + runner-up instead.

Converting low-confidence predictions to `unknown` would **destroy the best guess
and regress the headline** (the ac-00 trade table shows abstaining trades real
recall). The honest move — and the determinability-probe's standing recommendation
(memory `determinability-probe-gold-is-the-ceiling`: "ship the runner-up, not 'not
determinable'") — is to **keep the typed prediction, mark it `low`, and surface the
runner-up** so the analyst sees "probably X, maybe Y, low confidence." Information
preserved, uncertainty honest.

## The bands (from the reliability curve)

| band | confidence | repr accuracy | gold accuracy | analyst meaning |
|---|---|--:|--:|---|
| **high** | ≥ 0.85 | 0.79–0.85 | 0.91 | trust it |
| **medium** | 0.70–0.85 | 0.71–0.76 | 0.69–0.92 | usable, spot-check |
| **low** | < 0.70 | 0.35–0.58 | 0.64–0.81 | scrutinise — runner-up shown |

The load-bearing cut is **low at 0.70**: on representative data it isolates the
half of all wrong predictions (51%) into a bucket that is only ~53% accurate, while
the kept ≥0.70 columns rise to ~0.80. The `high` cut at 0.85 marks the columns an
analyst can take on trust (0.85–0.91 accurate).

## What ships (ac-02 scope, on approval)

- `quality_band: high|medium|low` — additive field in profile json/csv/plain.
- `runner_up: <type>` — populated on the `low` band, from the existing
  `vote_distribution` second-best (dependency: confirm it is populated on the
  multi-branch path; if not, derive from the branch logits in ac-02).
- Raw `confidence` unchanged. No `unknown` conversion. No default-prediction change
  — purely additive, so existing consumers keep working.

## Open decisions for the author (before ac-02)

1. **Default-on or behind a flag?** The field is purely additive (no existing field
   changes type/meaning), so default-on is low-risk — but it touches the output
   schema that MCP and the DuckDB extension consume (B07 audit in ac-02). Recommend
   **default-on**, audited.
2. **Band thresholds** — 0.70 / 0.85 as above, or tune. Recommend as-is (data-driven).
3. **Runner-up on `medium` too, or only `low`?** Recommend `low` only (keeps the
   high/medium output clean).

## Non-regression commitment

Because nothing converts to `unknown` and no existing field changes, the gold/repr
headline is unchanged by construction. ac-03 will confirm the band *separates*
correctness (high-band precision ≫ low-band) on held-out data — that is the test of
whether the signal is real rather than cosmetic.
</content>
