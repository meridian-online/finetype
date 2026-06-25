# ac-1 — triage of the 33 Sense-right → composed-wrong columns

Spec `2026-06-25-sharpen-stage-audit`. Diagnostic: m2v8m-attn-s42, 931 gold cols.
Each row below was reproduced by running the real binary with `FINETYPE_INJECT_LABEL`
on the column's corpus sample values (the same path that built `buckets.tsv`), reading
back the firing `disambiguation_rule` + `validation_vetoed`/`vetoed_type`.

## Headline

**The Sharpen layer's single biggest self-inflicted wound on now-correct Sense is the
deprecated regex `header_hint` table (decision 0042).** 21 of the 33 breaks are a header
*substring* match promoting the model's correct value-based prediction to a type the column's
values contradict. The promotion then either stands wrong (utc/ordinal/url/state/full_name)
or is hard-vetoed to `unknown` — because when the validation veto rejects a header-hint
label, the fallback drops to `unknown`/`word` instead of reverting to the model's correct
pre-hint label.

This is the gold evidence decision 0042 was waiting for: the header-hint table doesn't just
fail to help, it actively overrides a Sense stage that is now right. Retiring/​constraining its
worst arms is **net rule-surface reduction** (ac-1's deliverable) and the highest-leverage move.

## Root-cause groups (all 33)

### Group 1 — deprecated header_hint substring over-fire (21 cols) ★ the lever

| Header substring → hinted type | Columns | Outcome | Values contradict because |
|---|---|---|---|
| `"year"` → `datetime.component.year` | priceEpsCurrentYear, (year), CitesPerYear, SetForWholeYear | → veto → **unknown** | "3.22", "0.0", "Yes" are not years; veto fires, fallback can't revert to decimal/boolean |
| `"rank"`/`"grade"` → `discrete.ordinal` | Grade, GlobalRank, TldRank, Region Rank | → **ordinal** (stands) | continuous decimals (-0.757) and large-spread ints are not bounded ranks |
| `"utc offset"` → `datetime.offset.utc` | utc_offset ×4 | → **utc** (stands; utc not veto-safe) | values are millisecond integers (28800000.0), not `[+-]HH:MM` |
| `"link"`/`"url"` → `technology.internet.url` | link_id ×2, episode_url_id, link_description, remove_compare_link | → **url** (stands) | values are msg-ids / prose / "Yes"; `url_bare_number_veto` only catches *numbers* |
| `" name"` suffix → `identity.person.full_name` | venue_localized_country_name (→full_name), template_name (→veto→unknown) | → full_name / unknown | "USA" is a country; "projects/…/tagTemplates/…" is a path |
| `"epoch"` → `datetime.epoch.unix_seconds` | epoch_number | → veto → **unknown** | "11.0","12.0" fail unix_seconds; fallback can't revert |
| `"state"` → `geography.location.state` | state | → **state** (same-category override reverts state_code) | values "CA","TX" are state *codes*, not names |

Mechanism is uniform: substring match in a compound header → cross-domain / same-category /
generic hint override (`header_hint_cross_domain`, `header_hint_generic`, `header_hint_same_category`)
→ the model's correct value-based label is discarded.

### Group 2 — validation veto + fallback sends correct ids/text to unknown/word (4 cols)

| Column | Transition | Cause |
|---|---|---|
| ipni_id ×2 | alphanumeric_id → **unknown** | `alphanumeric_id` validator requires letters+digits; "69732-1" (digits+separator) fails → hard veto → `veto_shape_fallback` finds no letters → unknown |
| coord_id | alphanumeric_id → **unknown** | same — "15_77227420" is digit+underscore |
| username | username → **word** | handle values fail the `username` validator → veto → vocab fallback → word |

Real id columns whose *separator-bearing numeric* shape the `alphanumeric_id` validator does not
admit. Either the validator is too strict, or the veto should keep alphanumeric_id rather than
demote a confusable to unknown.

### Group 3 — increment over-emit (6 cols)

| Column | Values | Verdict |
|---|---|---|
| ID ×4–5 | 9211,9210,9209,… (contiguous descending) | **Label question (→ ac-3)**: a contiguous ID run is defensibly `increment`; gold says `integer_number`. Not a crutch bug — the increment_substance_veto correctly does NOT demote a genuinely contiguous run |
| State ID | 1,2,3,4 | same — contiguous → ac-3 |
| deferredLongTermLiab | 7272000,6603000,7583000,5957000 (NOT contiguous) | **Real veto miss**: R12 emitted increment on non-sequential financial data and `increment_substance_veto` did not demote — confirm against caveat-2 (truncated sample treated as full column) |

### Group 4 — F6 extension reframe (1 col)

`discriminator_type` (csv,csv,json) `file.extension` → F6 short-alpha-code → categorical →
enum-reframe → **word**. F6 over-fires on a genuine extension column.

### Unreproduced (1 col)

`mag` (1.0,2.0,1.5) → composed `unknown` in the diagnostic, but clean decimals validate (pass=1.0)
and do NOT reproduce the veto. Data-specific — the real column likely carried non-numeric values
the truncated sample omits. Low priority.

## Recommended actions (ordered by leverage × safety)

The header_hint table is the lever (Group 1 = 21 cols). Two ways to constrain it:

- **(A, recommended) Surgical, value-corroborated, gold-gated per arm.** A header hint that
  promotes to type T must be **value-corroborated** — reverted to the model's prediction when
  the column's values fail T's own validator (the reliable-NO direction of the asymmetry; choice
  0094 pattern). This is net surface reduction (the bare-substring arms get tighter or go away)
  and it generalises the existing per-type vetoes (`url_bare_number_veto`, `amount_bare_number_veto`)
  into one principled guard. Lower blast radius than a global cascade rewrite; gold-gateable
  incrementally. Start with the `"year"` and `"utc offset"` arms (8 cols, all currently broken,
  near-zero regression risk).
- **(B) Revert-to-original on veto.** Thread the model's pre-hint label into `resolve_veto_outcome`
  so a hard-vetoed header-hint label falls back to the model's prediction, not `unknown`. Recovers
  the entire "→unknown after a bad hint" slice (priceEps, (year), CitesPerYear, SetForWholeYear,
  template_name, epoch_number). Complementary to A, but A removes the bad hint at source so the
  veto never has to fire — prefer A; keep B as the safety net for veto'd labels A doesn't cover.

Group 2 (ids → unknown) is an **alphanumeric_id validator / veto-fallback** fix (separate from
the header hints). Group 3 contiguous-IDs and the `utc_offset`/`(year)` label edges feed **ac-3**.
Group 4 is a one-line F6 tightening.

**Expected recovery:** Group 1 (21) + Group 2 (4) are genuine crutch breaks recoverable here;
Group 3 contiguous (≈5) is ac-3 (label), so the realistic ac-1 composed recovery is ~+2.5–2.8pp
of the headline +3.5pp ceiling, with the rest banked as ac-3 label corrections. Each fix clears
gold no-regression + corpus-honest (blocking) before it ships.
