# Header-hint ablation — which arms still earn their keep on the attention model

Spec `2026-06-25-sharpen-stage-audit`. Re-baselines the stale verdict that blocked header-hint
deletion (`-4pp` on m-19, task `t-0001692a`) — that was the OLD model against the OLD gate. This
measures each header-hint family on the **attention model** (m2v8m-attn-s42 standalone Sense,
composed through the real Sharpen) scored on **gold** (the canonical gate). Instrumented binary
(`--features rhh-instrumentation`), `RHH_DISABLE_HINTS` per family.

## Per-family ablation (gold headline; baseline = 745/931 = 0.800)

| Family disabled | Gold | Δ vs baseline | Verdict |
|---|---|---|---|
| `substring_matcher_datetime` | 0.800 | **+0** | dead weight — the corroboration guard already neutralised the year/epoch damage |
| `substring_matcher_representation` | 0.803 | **+3** | net DAMAGE (rank/grade/class→ordinal) |
| `substring_matcher_identity` | 0.802 | **+2** | net DAMAGE (` name`→full_name) |
| `substring_matcher_technology` | 0.797 | −3 | net POSITIVE (url/ip) — keep |
| `substring_matcher_geography` | 0.797 | −3 | net POSITIVE (postal/street) — keep |
| `substring_matcher_finance` | 0.791 | −9 | net POSITIVE (amount) — keep |
| all substring matchers | 0.789 | −10 | wholesale removal still bad (old −4pp confirmed) |
| `header_hint_table` (exact dict) | 0.719 | **−76** | massively load-bearing — keep |

**The headline:** blanket header-hint deletion is still wrong (−10 substring, −76 table) — the
old intuition holds. But the blanket verdict masked the real story: **two specific families are
pure damage on the stronger Sense** — `representation` (+3) and `identity` (+2) — while
technology/geography/finance/table all still earn their keep. Selective, not wholesale.

## Combined retirement (representation + identity) → 0.805, +4 net

Disabling both: **749/931 = 0.805** (RECOVER 6, REGRESS 2).

```
RECOVER  Region Rank            ordinal -> decimal_number   gold=decimal_number
RECOVER  TldRank                ordinal -> integer_number   gold=integer_number
RECOVER  SpeculativeGenerality  ordinal -> integer_number   gold=integer_number
RECOVER  usageclass             ordinal -> word             gold=word
RECOVER  template_name          unknown -> plain_text       gold=plain_text
RECOVER  venue_…_country_name   full_name -> country        gold=country
REGRESS  Count_read      integer_number -> year             gold=integer_number
REGRESS  Grade           decimal_number -> ordinal          gold=decimal_number
```

The 2 regressions prove the **RHH family granularity is too coarse**: retiring the whole
`representation` matcher also kills the useful `"count"→integer` arm (Count_read regresses), and
`Grade`'s ordinal leaks from a different path. The clean cut is per-ARM, not per-family:

- **Retire the ordinal-keyword arm** (`rank`/`grade`/`class`/`tier`/`level`/`pclass` → ordinal).
  Value-blind, pure damage: Region Rank, TldRank, GlobalRank, usageclass all wrong. ordinal is a
  bounded ordered set — a rank header on continuous/large-spread numbers is not ordinal.
- **Retire/tighten the `" name"`-suffix arm** (→ full_name). Breaks `country_name`, `template_name`,
  `agency_name`, `gis_nta_name`. A header ending "name" is not a person name.
- **Keep** `"count"`/`"num"`→integer, email, phone, first/last name, and all of
  technology/geography/finance/the exact table.

Expected clean gain ≈ +5–6 with the `count`→integer regression avoided. Same direction as
decision 0042 (header hints deprecated) and ac-1/ac-2's "net rule-surface reduction" — now backed
by per-family gold evidence instead of a stale blanket verdict. Ship gate: gold no-regression
(measured here) + corpus-honest relocation gate (blocking, H05) before promotion.

## Shipped (per-arm cut) — gold-clean on BOTH models

Removed three arms from `header_hint()` (`header_sharpen.rs`): the exact ordinal arm
(`class`/`pclass`/`grade`/`rank`/`level`/`tier`/`rating`/`priority`/`score`), the substring
ordinal arm (`class`/`grade`/`rank`/`tier`), and the broad `ends_with(" name")` → full_name arm.
Kept `count`/`num`→integer, `ticket`/`cabin`→alphanumeric_id, and the first/last/full-qualified
name arms.

| Model | before | after | Δ | regressions |
|---|---|---|---|---|
| attention composed (m2v8m-attn, the co-ship) | 0.794 | **0.807** (751/931) | **+12** | 0 |
| default model (m2v8m-s43, shipped) | 0.797 | **0.800** (742/927) | **+3** | 0 |

The +12 on the attention model combines this cut with the year/epoch corroboration guard (ac-1).
The cut is gold-clean on BOTH — fixing it helps the shipped model today, not only the eventual
attention co-ship. Per-arm beat the per-family RHH retirement: removing the exact ordinal arm too
recovered `Grade`, and keeping `count`→integer avoided the `Count_read` regression. Remaining
blocker before promotion: the corpus-honest relocation gate (H05) on these arm removals.
