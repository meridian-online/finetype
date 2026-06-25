# ac-1 — measurement: value-corroboration guard (header_hint_value_corroboration)

Spec `2026-06-25-sharpen-stage-audit`. A/B measured by composing the **attention model's**
standalone Sense predictions (`output/embed-frontier/preds/m2v8m-attn-s42_sense.tsv`) through
the binary's real Sharpen stack (the same `compose_predictions.py` path that built `buckets.tsv`),
guard ON vs the pre-change baseline (`buckets.tsv` composed). The fix recovers columns only on
the **attention** model — on the shipped default (m2v8m-s43) it is a clean no-op (gold 0.797
guard-on == guard-off, 0 columns changed), because the 33 breaks are Sense-right *only* on the
attention model. This is the audit's thesis: the wins exist because attention Sense is now right.

> **Tooling note:** `RHH_DISABLE_HINTS` only works in a binary built `--features
> rhh-instrumentation`. The shipped release binary ignores it (every `is_disabled` is a const
> `false`), so the genuine guard-OFF baseline is the pre-change `buckets.tsv` composed, not an
> RHH-disabled run. The first A/B "0 diff" was this, not a null result.

## The broad guard was net +6 but NOT gold-clean

First cut declined any header-hint override whose hinted type the values failed (<50%, then ≤10%
— identical result). Attention composed **0.794 → 0.800**, but per-column:

**RECOVER = 14, REGRESS = 8, net = +6.** The threshold (0.5 vs 0.10) made zero difference.

| | columns |
|---|---|
| RECOVER | (year), CitesPerYear, priceEpsCurrentYear, SetForWholeYear (year→…); epoch_number (unix_seconds); link_id ×2, episode_url_id, link_description, remove_compare_link (url→…); utc_offset ×4 (offset.utc→decimal) |
| REGRESS | href ×2 (url→plain_text, **gold=url**); Country Code (country_code→iata, **gold=country_code**); TZ (iana→iata, **gold=iana**); perm_unlink ×3 (integer→binary, **gold=integer**); utc_offset ×1 (utc→loinc, **gold=utc**) |

## Root cause of the regressions: unreliable universal validators

The guard assumed "values fail the hinted type's validator" ⇒ "values contradict the type" (the
reliable-NO direction of the validation asymmetry). That holds only when the validator is precise.
It isn't for four labels:

- **`url`** rejects genuine URLs (href, gold=url, passes ~0% — the protocol-relative `//` gap, task
  `t-00007f4d`). So the url validator cannot separate `link_id`=msg-ids (block, correct) from
  `href`=real URLs (block, **wrong**) — both pass ~0%.
- **`geography.location.country_code`** — "Country Code" values fail the enum at ~0% yet are
  gold=country_code.
- **`datetime.offset.iana`** — "TZ" values fail at ~0%, gold=iana.
- **`datetime.offset.utc`** — the gold=utc `utc_offset` column fails the utc validator at ~0% (its
  values are the same millisecond-integer encoding as the four gold=**decimal** `utc_offset`
  columns — a likely **gold inconsistency**, → ac-3).
- **`perm_unlink`** is a different failure: blocking the `url` hint removed the `url→integer` path
  that `url_bare_number_veto` used downstream, so the 0/1 column fell to `binary` instead. A
  guard–rule interaction, not a validator issue.

The threshold change didn't help because the bad validators reject real members at ~0%, the same
rate as true contradictions — pass-rate carries no separating signal there.

## Shipped: scoped to the gold-clean subset (+5, 0 regress)

Restricted the guard to the labels whose universal validator reliably separates real members from
over-emits: **`datetime.component.year`, `datetime.epoch.unix_seconds`,
`datetime.epoch.unix_milliseconds`**. Recovers priceEpsCurrentYear, (year), CitesPerYear,
SetForWholeYear, epoch_number — **+5, zero regression** — same scoping discipline as R32. Gold
no-regression holds; corpus-honest gate is the remaining blocking check before any promotion.

## Deferred (the rest of the 33) — follow-up fixes, NOT this guard

| Cluster (cols) | Why the validator guard can't do it | Next fix |
|---|---|---|
| url over-emit on non-url (link_id ×2, episode_url_id, link_description, remove_compare_link) — 5 | url validator rejects real URLs too | extend `url_bare_number_veto` to demote `url`→alphanumeric_id/text when values are non-url-shaped (no `://`), independent of the validator; OR fix the url validator's `//` gap (t-00007f4d) first, then the guard handles url cleanly |
| utc over-emit on ms-integers (utc_offset ×4) — 4 | utc validator rejects ms-integers AND a gold=utc sibling | dedicated `utc_offset_bare_number` demotion (ms-integers → decimal); the gold=utc sibling with identical values → **ac-3 gold inconsistency** |
| ordinal over-emit (Grade, GlobalRank, TldRank, Region Rank) — 4 | ordinal has no/loose validator | tighten the `rank`/`grade` header_hint arm, or value-gate ordinal to small bounded rank sets |
| alphanumeric_id → unknown (ipni_id ×2, coord_id) — 3 | alphanumeric_id validator rejects digit+separator ids | widen the alphanumeric_id validator, or keep alphanumeric_id on veto rather than demote-to-unknown |
| increment over-emit (ID ×n contiguous, State ID) | contiguous IDs are defensibly increment | **ac-3 label question** (gold=integer vs increment) |
| deferredLongTermLiab integer→increment | R12 on non-contiguous + increment_substance_veto miss | confirm vs caveat-2 (truncated sample), tighten R12/veto |
| state_code → state (state) | header_hint "state" same-category override | guard the override against value-confirmed state codes |
| venue_…_country_name → full_name; template_name → unknown | " name" substring → full_name (loose validator) | tighten the `" name"` header_hint arm to word-boundary / exclude `*_country_name`, `*_template_name` |
| discriminator_type file.extension → word | F6 over-fires on genuine extensions | tighten F6 |
| mag → unknown | not reproduced from sample (data-specific) | low priority |

**Net for ac-1 so far:** +5 gold-clean banked; the bigger clusters (url 5, utc 4, ordinal 4)
need per-label value tests because the universal validators are too imprecise to gate on. The
header_hint table remains the lever — but the surgical per-arm value-corroboration, not a single
validator-based guard, is what clears gold.

## Shipped (utc cluster): `utc_offset_bare_number_veto` (+4, 0 regress)

The deferred utc cluster (4 of the 33 breaks) is now closed by a dedicated value-based
veto, NOT the validator guard (which can't gate — the utc validator rejects ms-integers
AND genuine hour-integer offsets at ~0%, so pass-rate carries no separating signal).

**MAGNITUDE is the discriminator.** The 4 break columns (gold=decimal) store offsets in
**milliseconds** — `28800000.0`, `-18000000.0`, `-14400000.0` (tens of millions). The
`utc offset` header-hint promotes the attention model's correct `decimal` Sense to
`datetime.offset.utc` value-blind. `utc_offset_bare_number_veto` (in
`apply_post_sharpen_guards`, where it can see the label the hint synthesised) demotes
`datetime.offset.utc` → decimal/integer when ≥80% of values are bare numbers AND ≥80%
exceed the max sane offset (+14:00 = 50_400 s). Demotion only, value-based (0048).

**The note's "gold inconsistency" is DISPROVED.** The task flagged the 5th utc_offset
(sha `e2695df2e762`, gold=utc) as having "the SAME ms encoding" as the 4 → ac-3. It does
not: its values are small signed **hour** integers (`10`, `-3`, `-4`, `0`) — a genuine UTC
offset, gold correctly utc. Magnitude (|v| ≤ 14 ≪ 50_400) separates it cleanly, so the veto
spares it. **No ac-3 gold change is needed for the utc cluster.**

**Complete gold A/B (no recompose needed).** Only 6 of 931 gold columns ever reach
composed=`datetime.offset.utc`; the veto's precondition is exactly that label, so checking
those 6 is the full A/B. Verified live on the release binary (attention Sense injected):

| sha | column | values | gold | old composed | new composed | Δ |
|---|---|---|---|---|---|---|
| 1b9ad1b9d168 | utc_offset | ms (−18M…) | decimal | utc | **decimal** | +1 |
| 2e8097c169f8 | utc_offset | ms (28.8M) | decimal | utc | **decimal** | +1 |
| 73b732b1c088 | utc_offset | ms | decimal | utc | **decimal** | +1 |
| a020d7e3865c | utc_offset | ms | decimal | utc | **decimal** | +1 |
| e2695df2e762 | utc_offset | hours (10,−3) | utc | utc | utc (spared) | 0 |
| 48e9044292d1 | date | — | sql_standard | iso_milliseconds (not utc) | unchanged | 0 |

**Net on the attention model: +4 columns, zero regressions.** Gold-safe by construction —
the veto fires only on bare numbers in the millions, which gold never labels utc.
Corpus-honest gate (H05, blocking) deferred to ac-4's combined promotion, as with the
year/unix scoped guard.
