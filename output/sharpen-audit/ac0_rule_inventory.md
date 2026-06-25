# ac-0 — Sharpen rule inventory + composed-scoring pin

Spec `2026-06-25-sharpen-stage-audit`. The map the rest of the audit works from.

The Sense→Sharpen pipeline has two stages. **Sense** is the dual-encoder multi-branch
model (m2v8m-s43 default; m2v8m-attn the diagnostic substrate). **Sharpen** is everything
below — a deterministic, value-based rule layer that runs on the Sense label. This doc
enumerates every Sharpen rule on the **live multi-branch path**, in firing order, recording
for each: *what it does*, *when it fires*, and *which weak-Sense failure it was compensating
for*. It closes with how "composed" is scored offline so audit edits are measured correctly.

---

## 0. Which path runs — and the legacy path that does NOT

There are two classify paths in `crates/finetype-model/src/column/mod.rs`:

| Path | Entry fn | Used by | Sharpen driver |
|---|---|---|---|
| **Multi-branch** (LIVE, shipped default + diagnostic) | `classify_multi_branch` (2369), `classify_multi_branch_with_enriched` (2483, sibling-context variant) | m2v8m-s43, m2v8m-attn | `feature_sharpen` + `value_sharpen` + the mod.rs guards |
| Legacy Sense | `classify_sense_sharpen_inner` (1503) | original sherlock Sense model only | inline `disambiguate()` + big inline header block |

**The audit only touches the multi-branch path.** The legacy `classify_sense_sharpen_inner`
(its inline header-hint block, lines 1865–2172, and its `disambiguate()` call) is dead for
the default model. The two share the underlying `disambiguate_*` helpers in `value_sharpen.rs`,
but the multi-branch path reaches them through `value_sharpen()` (R-rules) and its own
`apply_header_sharpen()` — NOT through `disambiguate()`. When editing a helper, check which
caller the diagnostic exercises: it is always `value_sharpen` / `apply_header_sharpen`.

### Live pipeline order (classify_multi_branch / _with_enriched)

```
1.  multi-branch forward pass            → (label, confidence)
2.  [FINETYPE_INJECT_LABEL override]     ← diagnostic-only; forces confidence = 1.0
3.  feature_sharpen(result, features)    F1–F6  (deterministic column stats)
4.  value_sharpen(sample, label, conf)   R1–R32 (value-shape rules; FIRST match wins)
5.  datetime_format_refinement(sample)   deterministic datetime sub-leaf
6.  structured_string_refinement(sample) windows_path / message_id / qualified_name
7.  sharpen_and_guard(header, sample, values):
      a. apply_header_sharpen   → coord_veto, postal_veto, state_code_promote,
                                   measurement, sci_measurement, gender-guard,
                                   geo override, same/cross-domain hardcoded hints,
                                   general/generic/hardcoded/fallback hints,
                                   country_code_post_hint_guard
      b. apply_post_sharpen_guards (run unconditionally on the post-hint label):
            amount_bare_number_veto, url_bare_number_veto, checksum_substance_guard,
            binary_vocab_veto, increment_substance_veto,
            city_region_header_corroboration, country_code_corroboration
8.  apply_username_veto(sample)          full_name → username
9.  post-hoc locale detection
10. finalize_is_generic                  enum reframe: categorical → word
─── then in crates/finetype-cli/src/profile.rs (profile time, after column classify) ───
11. col_validation_veto + resolve_veto_outcome   → veto_shape_fallback (alnum_id / word)
                                                    or "unknown"
```

Every rule in steps 3–11 is RHH-disableable (`rhh::is_disabled("<name>")`) unless noted.
Most are **demotion-only** (can veto a wrong Sense call but never invent a new positive),
which is what keeps them over-emission-safe.

---

## 3. feature_sharpen (F1–F6) — `feature_sharpen.rs`

Column-statistic rules (mean/variance of leading-zero, slash-segment, is-float flags).
Adapted from `feature_disambiguate` with vote-dependent guards stripped (no votes exist
in multi-branch).

| Rule | What / when | Compensates for |
|---|---|---|
| **F1** leading-zero pre-filter | `postal_code`/`cpt` → `numeric_code` when ≥30% of values carry a leading zero | model reads zero-padded codes as postal/medical |
| **F2** docker_ref vs hostname | slash-segment mean ≥1.5 disambiguates | structurally-similar slashed strings |
| **F5** numeric_code split | `numeric_code` → `integer_number` / `decimal_number` by IS_FLOAT mean >0.5 | model can't tell numeric_code from plain number |
| **F6** file.extension | short alphabetic codes (len ≤4, alpha ≥0.8, dot-segments <1.1) → categorical | extension over-emit on short codes |

`feature_disambiguate` (118) is the legacy-path twin (adds F3 hs_code vs decimal, vote-gated);
not on the live path.

---

## 4. value_sharpen (R1–R32) — `value_sharpen.rs:40`

Ordered chain; **first match returns** `Some((label, rule))`. Each rule is gated on the
INPUT label (so it only re-checks types it can correct) except where noted. Rule numbers are
historical and non-contiguous. The driver comment still says "R1–R19" — it actually runs to R32.

| R | rule_name | input label gate | What it does | Compensates for |
|---|---|---|---|---|
| R1 | date_slash_disambiguation | mdy_slash / dmy_slash | first component >12 → dmy, second >12 → mdy | model can't order ambiguous slash dates |
| R2 | short_date_disambiguation | short_mdy / short_dmy | same, dash separator | same, dash form |
| R21 | coordinate_plausibility_gate | latitude / longitude | >10% of parseable values \|x\|>180 → `decimal_number` | lat/long over-emit on plain numerics (depth, error) |
| R3 | coordinate_disambiguation | latitude / longitude | any \|x\|>90 → longitude, else latitude | lat/long swap |
| R4 | ipv4_detection | **any** | ≥80% dotted-quad, octets ≤255 → `ip_v4` | model misses IPv4 |
| R5 | day_of_week_name_detection | **any** | ≥80% day-name vocab → `day_of_week` | model misses weekday words |
| R6 | month_name_detection | **any** | ≥80% month-name vocab → `month_name` | model misses month words |
| R7 | boolean subtype | boolean types / max-frac ≥0.8 | normalises to binary / terms / initials | model picks wrong boolean sub-leaf |
| R8 | gender_detection | **any** | all values in gender set → `gender`; single-char M/F/X → `gender_code` | model misses gender |
| R9 | boolean_override | boolean | numeric/spread/single-char shapes → `integer_number` | boolean over-emit on sparse ints |
| R12 | disambiguate_numeric | increment, integer, decimal, postal_code, year, numeric_code | **the big numeric resolver** — picks among increment (low-variance sequential), integer/decimal, postal (consistent 5-digit, range, non-sequential), year (≥80% 4-digit AND ≥80% in 1900–2100); HTTP-status guard returns None for 3-digit 100–599 | weak numeric-family separation. **Suspect for `integer→increment` and `→ordinal` ac-1 transitions** |
| R13 | si_number override | si_number | no SI suffix present → `decimal_number` | si_number over-emit on plain decimals |
| R19 | percentage_no_sign | percentage | no `%` in any value → `decimal_number` | percentage over-emit |
| R20 | hs_code_validation_gate | hs_code | <50% match HS digit-group format → `decimal_number` | hs_code over-emit on plain decimals |
| R22 | upc_digit_count_gate | upc | <50% 12-digit → ean (if 13/8) else `numeric_code` | UPC/EAN/NPI digit-length confusion |
| R23 | isin_format_gate | isrc | >50% ISIN-shaped → `isin` | ISRC↔ISIN confusion |
| R24 | issn_format_gate | ein | >50% ISSN-shaped (dash@4) → `issn` | EIN↔ISSN dash-position confusion |
| R14 | duration_override | sedol | ≥50% start `P…[YMWDTHS]` → `duration.iso_8601` | ISO durations read as SEDOL |
| R17 | utc_offset_override | **any** | ≥80% match `[+-]HH:MM` exactly → `datetime.offset.utc` | **Suspect for `decimal→utc` ac-1 transition** — unconditional, fires on any label |
| R15 | attractor_demotion (`sharpen_attractor_demotion`) | NUMERIC/TEXT/CODE_ATTRACTORS | demote attractor types on validation-fail / low-conf / low-cardinality | CharCNN-era over-confident attractors (postal, first_name, phone, street_name, icao, ndc, cusip, tld) |
| R16 | text_length_demotion | full_address | median value length >100 → `plain_text` | full_address over-emit on prose |
| R31 | version_dmy_short_dot_gate | dmy_short_dot | ≥30% segments impossible as DD.MM.YY → `version` | version strings read as dates |
| R27 | year_compact_ym_gate | compact_ym | ≥90% 4-digit AND ≥80% in 1900–2100 → `year` | 4-digit years read as compact_ym |
| R32a | schema_fail_demotion (closed-set) | measurement_unit, geohash, **utc**, **url** | >50% fail the label's own validator → categorical (≤20 distinct) / `alphanumeric_id` | narrow pattern-bound types over-emit on free codes/ids (the additive-hard-negative retrain's job, done as a rule) |
| R32b | text_vocab_override | **word** only | 2–12 distinct AND distinct/n ≤0.6 → `categorical` (then reframed to word at step 10) | a status/type vocabulary asserted as free words |

Shared helpers also reachable here: `detect_epoch_seconds` (called at legacy step 7c, not in
value_sharpen), `select_fallback` / `sharpen_select_fallback` (pick the demotion target by value
shape — representation.* preferred, then numeric/categorical/alnum by shape), `schema_validation_gate`
(legacy-path demotion when >50% fail a non-representation validator — **not on the live multi-branch
path**; the live equivalents are R15/R32 + the profile-time veto).

---

## 5–6. Deterministic refinements — `mod.rs`

| Rule (fn) | What / when | Compensates for |
|---|---|---|
| **datetime_format_refinement** (3055) | reads the exact datetime leaf via `finetype_core::datetime_format`. DELIMITED read (`2020-01-03 14:22:09`) asserted unconditionally (gated only on the leaf's own validator passing); BARE-integer read (epoch, 4-digit year) asserted ONLY if model already said `datetime.*` | flat-softmax guesses the wrong datetime sub-leaf, or the veto demotes a real timestamp to unknown. **Relevant to ac-2 `unix_seconds`/`sql_standard` misses** |
| **structured_string_refinement** (3109) | recovers `windows_path` / `message_id` / `qualified_name` from the residual (plain_text/word/unknown, plus url/email for the unambiguous two); gated ≥90% pass the leaf's own validator | 240-dim model cannot predict these three new leaves at all |

---

## 7a. apply_header_sharpen (header hints + corroboration) — `mod.rs:2602`

Runs `header_hint()` (hardcoded table) first, then Model2Vec semantic hint. **Note: each
branch early-`return`s**, which is why the value-identical-boundary guards that act on a label
a hint just CREATED live in `apply_post_sharpen_guards` instead. Three choice-0094
corroboration rules sit at the top (they `return` immediately):

| Rule | What / when | Compensates for |
|---|---|---|
| **coord_header_veto** (2613) | lat/long + header does NOT corroborate coord + values look like generic decimals → `decimal_number` (demotion-only) | lat/long over-emit; the no-hint gap sci_measurement misses |
| **postal_header_veto** (2638) | postal_code + header lacks postal token + generic bare integers (no leading zero) → `integer_number` | postal precision 0.133 on bare-int columns |
| **state_code_promote** (2664) | NOT state_code + state/province header + ≥80% values in STATE_CODES → `state_code` (PROMOTION; the only path to state_code) | state_code P=R=0.000; no existing emit path |

Then the hint cascade (each RHH-flagged): `header_hint_measurement` (height/weight),
`header_hint_sci_measurement` (coord→decimal on measurement header), gender-sibling guard
(don't revert value-chosen gender_code), `header_hint_geo_override` (city↔country within
LOCATION_TYPES), iso_8601 catch-all guard, `header_hint_same_category` / `_cross_domain`
hardcoded overrides, then general `header_hint` / `_generic` / `_hardcoded` (same-domain
conf threshold 0.95, cross-domain 0.85) / `_fallback` (conf <0.3). Closes with
`country_code_post_hint_guard` (country + ≥95% `^[A-Z]{2}$` → country_code).

`header_hint()` itself (`header_sharpen.rs:216`) is the **deprecated** (decision 0042) regex
keyword→label table. It is still wired into the live path — every hardcoded hint above flows
through it. Full keyword table is large (email, url, ip, uuid, gender, age→integer, epoch,
lat/long, country/city/state, currency, ordinal keywords class/grade/rank, boolean keywords,
entity_name keywords publisher/company/venue/…, amount-variant exacts, scientific-measurement
keywords pressure/temperature/…, etc.). Deprecation means "do not extend"; deletion is blocked
on training-data fortification (0094).

## 7b. apply_post_sharpen_guards (vetoes) — `mod.rs:2992`

Run **unconditionally** on the post-hint label so they can catch a label a hint synthesised.
All demotion/corroboration, all RHH-disableable, all value-based (0048).

| Veto (fn) | What / when | Compensates for |
|---|---|---|
| **amount_bare_number_veto** (3427) | `currency.amount` + bare-number values → decimal/integer | money-ish header promotes a plain number to amount (P=0.105) |
| **url_bare_number_veto** (3308) | `url` + bare-number values → decimal/integer | link-ish header promotes a number to url (P=0.721) |
| **checksum_substance_guard** (3238) | any checksum-bearing type (isbn/aba/cusip/sedol) with <50% passing the real check-digit → integer/decimal (bare) or alphanumeric_id | shape-only validator lets big integers look like ISBN/CUSIP |
| **binary_vocab_veto** (3174) | `boolean.binary`, all-integer column, any value ∉{0,1} → `integer_number` | binary over-emit on sparse count columns |
| **increment_substance_veto** (3021) | `increment` + full column is NOT a contiguous near-unique run (`values_form_increment==Some(false)`) → `integer_number` | R12/stepped-sample over-emits increment (gold P=0.056). **NB: needs the FULL column `values`, not the sample — see scoring caveat below** |
| **city_region_header_corroboration** (3342) | `city` + region/county/district/… header → `region` (promotion) | city↔region flat-softmax confusion (region recall 0.467) |
| **country_code_corroboration** (3375) | region/city/country + >50% values are exact ISO codes → `country_code` (promotion; state/state_code deliberately excluded) | 2-letter ISO codes filed under region/country |

## 8. apply_username_veto — `mod.rs:1022`

`full_name` + `is_username_handle_shaped` (≥80% handle-charset, low whitespace, high
distinct-fraction) → `username`. Runs AFTER header hints so a deprecated `author→full_name`
cross-domain hint can't resurrect a handle column. Compensates for full_name being the model's
catch-all for handle columns.

## 10. finalize_is_generic (enum reframe) — `mod.rs:1037`

`representation.discrete.categorical` → `representation.text.word` (choice 0102). Categorical
is retired as an EMITTED leaf — it is the v23-explosion residual attractor. Every Sharpen rule
that produces `categorical` (R15, R32b, schema_fail_demotion, F6) is reframed here at the single
output chokepoint. RHH flag `enum_reframe_residual`.

## 11. Validation-as-veto (`→unknown` crutch) — `profile.rs` + `validation_veto.rs`

After column classification, profile re-checks the final label against its own validator
(`col_validation_veto`, threshold **0.5**, `validate.rs:565`):

- pass_rate < 0.5 **AND** label ∈ `labels/veto_safe.txt` (72 audited-safe labels) → **HARD veto**.
- `resolve_veto_outcome` (`validate.rs:591`) then runs `veto_shape_fallback`: high-cardinality
  letter+digit → `alphanumeric_id` (`veto_fallback:id`); small repeating vocab → `word`
  (`veto_fallback:vocab`); otherwise → **`unknown`**. `FINETYPE_NO_VETO_FALLBACK=1` restores the
  unconditional-unknown behaviour.
- pass_rate < 0.5 but label NOT audited-safe → ADVISORY flag only, label kept.

`decimal_number`, `integer_number`, and `alphanumeric_id` are all on the audited-safe list, so
they CAN be hard-vetoed. **This is the `decimal/alnum→unknown ×7` ac-1 transition** — a
Sense-correct value the veto rejects because the values fail that label's validator at sample
scale. Asymmetric by design (memory `validation-gate-asymmetry`): low pass-rate is a reliable NO,
high pass-rate is an unreliable YES, so validation only ever vetoes, never asserts.

---

## How "composed" is scored offline — the pin

`scripts/compose_predictions.py` produced `buckets.tsv`. Mechanism:

1. For each (sha, column) it writes the column's `sample_values_truncated` to a one-column CSV.
2. Runs the **real release binary**: `finetype profile -f c.csv -o json-schema` with
   `FINETYPE_INJECT_LABEL=<the model's standalone prediction>`.
3. Reads back `x-finetype-label` = the composed label.

The injection point (`mod.rs:2426/2545`) sits **immediately after the multi-branch forward pass
and before `feature_sharpen`**, so the injected label flows through the **identical Sharpen stack
(steps 3–11 above)** a native prediction would. This is faithful — composed offline = composed
native — with **three caveats the audit must hold**:

1. **Confidence is forced to 1.0.** Injection sets `result.confidence = 1.0`. Several header
   rules are confidence-gated as DEMOTIONS requiring `conf < threshold`
   (`header_hint_hardcoded` same-domain <0.95 / cross-domain <0.85; `header_hint` general
   <0.5; geo override ≤0.90; `_fallback` <0.3). At conf=1.0 these **do not fire**. So the
   diagnostic systematically *under*-counts header-hint overrides relative to a native run where
   the model's true confidence is lower. Any audit edit whose effect depends on one of these
   thresholds must be validated natively, not just through compose. (Promotion rules and the
   value-shape vetoes are NOT confidence-gated, so they are measured faithfully.)

2. **`sample_values_truncated` is the whole "column".** The binary sees the truncated sample as
   the full column. `increment_substance_veto` calls `values_form_increment` on what profile
   treats as the full column — i.e. the truncated sample, not the true full column. Its
   contiguity test ("distinct ≈ max−min+1") behaves differently on a truncated/stepped sample
   than in a real corpus profile. Same applies to any rule reasoning about column-global
   structure. Treat increment-related transitions as sample-artefact-prone.

3. **The validation veto IS included.** Because compose reads the profile json-schema output,
   step 11 runs — so `→unknown` and `veto_fallback:*` transitions appear in `buckets.tsv`. Good:
   the `decimal/alnum→unknown` slice is real, not a scoring gap.

For audit edits: measure composed by **re-running `compose_predictions.py` on the same standalone
preds**, then `score_gold_anchor.py`. For any edit touching a confidence-gated header branch
(caveat 1) or column-global structure (caveat 2), additionally confirm with a native `profile`
run on the real columns before trusting the delta.

---

## Bridge to ac-1 / ac-2 — transition → suspect rule

| ac transition | Suspect rule(s) | Note |
|---|---|---|
| integer→increment ×6 (ac-1) | R12 `disambiguate_numeric` (sequential→increment) PROMOTES; `increment_substance_veto` SHOULD catch but doesn't here | spec attributes to the veto; the veto is demotion-only and evidently inert on these — likely caveat-2 sample artefact. Confirm. |
| decimal→utc ×4 (ac-1) | R17 `disambiguate_utc_offset_override` (unconditional, ≥80% `[+-]HH:MM`) | may be a label question (offset columns gold-labelled decimal) → ac-3 |
| decimal/alnum→unknown ×7 (ac-1) | step 11 validation veto + `veto_shape_fallback` | Sense-correct value the validator rejects at sample scale |
| alnum→url ×3 (ac-1) | header hint url promotion; `url_bare_number_veto` only catches bare numbers, not alnum | guard gap for alnum-shaped values |
| integer/decimal→ordinal ×4 (ac-1) | `header_hint` ordinal keywords (class/grade/rank/tier) | header-driven, deprecated table |
| unix_seconds/sql_standard misses (ac-2) | `datetime_format_refinement` not firing | diagnose corroboration/validator gate |
| top_level_domain/url/isbn/integer/alphanumeric_id misses (ac-2) | no rule fires (Sense + composed both wrong) | candidate value-based rules |
| word/plain_text/entity_name residual (ac-2/3) | `text_vocab_override`, entity demotion, structured_string_refinement | likely Sense/label problem, not a rule |
