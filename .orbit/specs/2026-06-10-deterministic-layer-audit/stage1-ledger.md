# FineType Stage-1 Audit Ledger

## 1. Step Ledger

| Step | Intent | Fires? | Matches Intent | Verdict | Action | Flag→Stage 2 |
|------|--------|--------|----------------|---------|--------|--------------|
| `header_hint_coord_veto` (choice 0094, col 2381) | Demotion-only lat/lon veto on non-coord numeric headers | yes | matches | KEEP | No action | no |
| `header_hint_measurement` (col 2439; legacy 1060/1716) | Height↔weight within-family header disambiguator | no (dead candidate) | cannot_trigger | DEAD CANDIDATE — cross_domain owns all height/weight promotions | Stage-2 ablation → remove all 3 copies | yes |
| `header_hint_sci_measurement` (col 2452; Sense twin 1736) | Demote sci-measurement-header floats grabbed as latitude | yes (only on contrived header) | drifted | SUPERSEDED/SHADOWED by coord-veto | Stage-2 ablation → remove both copies | yes |
| `header_hint_person_override` (col 2482) | Hardcoded person-name header overrides location prediction | yes | matches | FIRES & MATCHES, but drift-leaning vs 0042/0048; value-blind | Stage-2 net-value ablation | yes |
| `header_hint_geo_override` (col 2498) | Hardcoded geo keyword overrides model location subtype | yes | matches | FIRES & MATCHES, but deprecated-pattern; overrides 98% model | Stage-2 net-value ablation | yes |
| `header_hint_same_category` (col 2528) | Hardcoded hint wins unconditionally within taxonomy category | yes | matches | FIRES as intended (choice 0028) | KEEP; Stage-2 ablation vs v19 | yes |
| `header_hint_cross_domain` (col 2551) | Cross-domain hardcoded hint over wrong-domain model prediction | yes | matches | KEEP — keep_required, load-bearing-by-policy | KEEP | yes |
| `header_hint_generic` (col 2585) | Last-resort hint when model generic & hint not in votes | yes | matches | KEEP — live, correct on trigger | KEEP; Stage-2 isolated ablation | yes |
| `header_hint_hardcoded` (col 2602) | Hardcoded hint corrects low-confidence wrong prediction | yes | matches | KEEP — intent-faithful, deletion already decided-against | KEEP | yes |
| `header_hint_fallback` (col ~2620) | Lowest-priority Model2Vec catch-all (conf<0.3) | no (dead candidate) | cannot_trigger | DEAD CANDIDATE — preconditions never co-occur | Stage-2 ablation → remove if zero-fire | yes |
| `header_hint` (bare catchall, col 2573) | Re-assert hinted type when value rule demoted top-1 to generic | yes | drifted | DRIFT — reverts correct value-rule (violates 0048) | Stage-2 ablation; gate fix vs value-rule provenance | yes |
| `header_hint()` source table (col 4236+) | Maps headers → curated type; feeds every apply family | yes | drifted | Live & load-bearing but intentional drift vs 0042 (deprecated) | KEEP per-family; Stage-2 ledger refresh | yes |
| F1 `feature_leading_zero` (col 2720) | postal/cpt → numeric_code when leading zeros present | yes | matches | KEEP — correct, narrowly scoped | No action | no |
| F2 `feature_slash_segments` (col 2739) | hostname → docker_ref on slash-segment mean ≥1.5 | yes | drifted | **BUG** — emits orphan label `technology.container.docker_ref` (not in label space/taxonomy) | Stage-2 (label-string fix scope) | yes |
| F5 `feature_no_leading_zero` (col 2756) | numeric_code → integer/decimal when no leading zeros | yes (integer branch) | matches | KEEP — integer branch load-bearing; float branch unreachable | Stage-2: confirm float branch dead | yes |
| F6 `feature_short_code_not_extension` (col 2774) | Demote short non-extension codes mislabelled file.extension | yes | drifted | DRIFTED — now fires on CORRECT bare-extension columns; model fixed the leak upstream | Stage-2 net-value ablation → likely remove | yes |
| F3/F4 (removed, col 2746) | hs_code/git_sha promoters removed from MB path | no (dead candidate) | matches | Correctly removed in MB path; but F3 emitter still live in legacy `feature_disambiguate` (3505) | Stage-2: confirm legacy path unreachable | yes |
| `value_sharpen` R1–R32 (col 2819) | Value-only disambiguation cascade (0048) | yes | matches | ALIVE & matches; R21/R3 coordinate gate possibly redundant under current ordering | Stage-2: ablate R21/R3 only | yes |
| `sharpen_attractor_demotion` R15 (col 3270) | Demote over-eager attractor predictions (3 signals) | yes | matches | KEEP — cardinality path load-bearing | No action | yes |
| `detect_locale_from_validation` (col 314) | Annotate detected_locale (post-hoc, label-unchanged) | yes | drifted | DRIFT — non-deterministic locale on value-identical patterns (no tie-break) | Stage-2: ablate tie-noise into 0001 demotion-skip | yes |
| Validation veto (hard + advisory, veto.rs:75) | Asymmetric NO-only veto, allowlist-scoped (0091) | yes | matches | KEEP — fully aligned, all 3 branches confirmed | No action | no |
| ENUM cardinality threshold (`--enum-threshold`, json_schema.rs:193) | ENUM vs VARCHAR: label-gate + cardinality cap | yes | drifted | Cardinality cap correct; **label-gate ABSENT on json-schema path** — recreates `enum_overfit` | Stage-2: route json-schema enum through label gate | yes |
| Legacy/Sense-path steps (epoch, geo_rescue, sense_header_hint*) | Post-process non-MB/non-fusion models | no (dead candidate) | cannot_trigger | CONFIRMED DEAD in default binary by construction (0041) — retained for alt models | Stage-2 only if legacy retirement taken up | yes |

---

## 2. Validator Precision Summary

| Domain | Total | Precise | Weak | Locale-gap |
|--------|------:|--------:|-----:|-----------:|
| container | 11 | 6 | 5 | 0 |
| datetime | 84 | 80 | 0 | 4 |
| finance | 28 | 24 | 2 | 2 |
| geography | 25 | 18 | 3 | 4 |
| identity | 33 | 27 | 1 | 5 |
| representation | 33 | 25 | 8 | 0 |
| technology | 26 | 18 | 8 | 0 |
| **TOTAL (240)** | **240** | **198** | **27** | **15** |

**Weak validators (27)** — confirm ~any input:
- **container (5):** `container.object.csv`, `container.array.comma_separated`, `container.array.pipe_separated`, `container.array.semicolon_separated`, `container.array.whitespace_separated`
- **finance (2):** `finance.currency.currency_code`, `finance.payment.credit_card_number`
- **geography (3):** `geography.address.full_address`, `geography.transportation.iata_code`, `geography.transportation.icao_code`
- **identity (1):** `identity.person.password`
- **representation (8):** `representation.text.plain_text`, `representation.text.word`, `representation.text.entity_name`, `representation.file.extension`, `representation.file.excel_format`, `representation.scientific.smiles`, `representation.discrete.categorical`, `representation.discrete.ordinal`
- **technology (8):** `technology.internet.hostname`, `technology.internet.top_level_domain`, `technology.cryptographic.token_urlsafe`, `technology.code.imei`, `technology.code.locale_code`, `technology.development.docker_ref`, `technology.identifier.tsid`, `technology.identifier.snowflake_id`

**Locale-gap validators (15)** — `locale_specific` designation but no `validation_by_locale`:
- **datetime (4):** `datetime.date.abbreviated_month`, `datetime.date.long_full_month`, `datetime.date.weekday_abbreviated_month`, `datetime.date.weekday_full_month`
- **finance (2):** `finance.banking.aba_routing`, `finance.banking.bsb` (real validation in universal block, not per-locale)
- **geography (4):** `geography.location.country`, `geography.location.region`, `geography.location.city`, `geography.address.street_name`
- **identity (5):** `identity.person.full_name`, `identity.person.first_name`, `identity.person.last_name`, `identity.person.username`, `identity.government.eu_vat`

---

## 3. False-Veto Resolutions

| Column | File | Veto | Category | Resolution |
|--------|------|------|----------|------------|
| `gender` | medical_records.csv | hard veto, 0% pass | **step_conflict** | `disambiguate_gender` emits `identity.person.gender` (enum `[male,female,other,unknown]`) for single-char `M`/`F`. **Fix the rule:** route exclusively single-char sex codes (M/F/X, ISO-5218 0/1/2/9) to `identity.person.gender_code` (enum `[M,F,X,0,1,2,9]`, passes 100%). Do NOT relax the gender enum. |
| `npi` | medical_records.csv | hard veto, 32% pass | **invalid_data** | Validator `^[12]\d{9}$` is correct per CMS — real NPIs begin 1 or 2. 41 of 60 fixture values begin 3–9. **Fix the fixture, not the engine:** regenerate the column as 10-digit numbers beginning 1 or 2. Validator untouched. |
| `price` | multilingual.csv | hard veto, 33% pass | **step_conflict** | Hardcoded `header_hint("price")→finance.currency.amount` (US format) collides with a multi-locale column (de-DE comma-decimal, pt-BR multi-symbol). **Fix the hint:** make `price` value-aware — select among `amount`/`amount_comma`/`amount_comma_suffix`/`amount_multisym` by separator/symbol shape (per 0042/0048), or drop `price` from the hardcoded map. Do NOT widen the US amount validator. |

---

## 4. Stage-2 Work-List (gated hand-off to corpus ablation)

Every step flagged `flag_for_stage2=true`, with the one-line reason:

1. **`header_hint_measurement`** — confirm zero corpus firings (cross_domain owns all height/weight promotions); remove all 3 copies only if it clears the full header-hint instrument map.
2. **`header_hint_sci_measurement`** — confirm no real corpus header both names a measurement AND carries a coordinate token to dodge coord-veto; else remove both copies as superseded-by-0094.
3. **`header_hint_person_override`** — net-value: do `author`-headed columns hold more genuine person-names (rescues) than locations (wrecks)?
4. **`header_hint_geo_override`** — net-value: how often does the hardcoded keyword overrule a high-confidence (98%) learned location subtype, and is the delta correct-gain or correct-loss?
5. **`header_hint_same_category`** — does the v19 header branch independently produce the correct same-category type on ≥80% of columns this rule rescues (the spec's retire threshold)?
6. **`header_hint_cross_domain`** — quantify latitude/longitude advisory-survival as a corpus relocation source; decide whether lat/lon should join veto_safe.txt (separate decision).
7. **`header_hint_generic`** — isolate THIS apply tier (not whole table): is it net-positive in bulk or silently forcing labels where the model would honestly abstain?
8. **`header_hint_hardcoded`** — fold into the in-flight per-family RHH net-value tracking (bulk-harm vs load-bearing on url/isbn/swift/npi).
9. **`header_hint_fallback`** — confirm byte-identical labels with the family disabled (predicted zero firings → remove as inert dead code).
10. **`header_hint` (bare catchall)** — measure value-rule-reversion harm vs semantic-hint-rescue benefit; the catchall tests a STALE pre-sharpen top-1 and reverts correct value rules.
11. **`header_hint()` source table** — keep the per-family net-value ledger fresh against the next retrain; per-family DEFER/remove only the bulk-forcing finance/representation arms.
12. **F2 `feature_slash_segments`** — **label-string bug**: emits orphan `technology.container.docker_ref`; confirm corpus columns it flips are currently orphaned (empty broad type) so the correct `technology.development.docker_ref` is the safe target.
13. **F5 float branch** — confirm `feature_decimal_over_numeric_code` fires on zero corpus columns under the MB model (unreachable), then drop the float branch.
14. **F6 `feature_short_code_not_extension`** — split corpus flips into real-extension columns (loss) vs genuine short codes still reaching file.extension (gain); expected loss-heavy → remove.
15. **F3 (legacy emitter)** — confirm the legacy `feature_disambiguate` path (callers 850/1598, live F3 hs_code emitter at 3505) is unreachable from every shipped entry point; then delete the dormant block.
16. **`value_sharpen` R21/R3** — confirm the raw MB model never returns a coordinate label at the value_sharpen call site (over-emit only enters later via header hints) → R21/R3 dead under current ordering.
17. **`sharpen_attractor_demotion`** — quantify how often signals 1 (validation) and 2 (confidence) fire in production vs being effectively unit-test-only; the cardinality path dominates.
18. **`detect_locale_from_validation`** — ablate whether the tied-numeric locale noise propagates into the decision-0001 attractor-demotion-skip path at corpus scale.
19. **ENUM label-gate gap** — count json-schema columns carrying an open-domain enum (year/iana) that would reject valid unseen values in the round-trip; quantifies the precision cost of the missing label gate.
20. **Legacy/Sense-path steps** — only if formal legacy retirement is taken up: confirm no released/HuggingFace/FINETYPE_MODEL target loads as non-multi-branch AND non-fusion.

---

## 5. Headline Findings

**Most of the deterministic layer does exactly what it was built to do — but a handful of rules now fire on the wrong columns, and Stage 1 alone can't tell "dead" from "rare-but-right".** Stage 1 can prove a rule never triggered on anything we threw at it; it cannot prove the rule is useless across the whole corpus. Only Stage 2's corpus ablation distinguishes a genuinely dead rule from one that's quietly correct on rare data. Read every "DEAD CANDIDATE" verdict as *couldn't trigger here* — a flag for the corpus check, not a verdict to delete.

**What an analyst lives, rule by rule:**

- **The freshly-shipped coordinate veto (v0.6.25) works exactly as advertised** — false latitudes on plain numeric columns get demoted, real latitude columns survive. This is the one clean precision win, and it's done its job everywhere we tested. *(Spark joy: fewer wrong "this is a coordinate" calls.)*

- **Two rules are now firing on the columns they were meant to protect.** F6 was built to catch short codes mislabelled as file extensions — but the current model already fixed that leak, so F6 now demotes *correct* bare-extension columns (txt, pdf, csv) into "categorical". And the bare header catchall reverts a correct value-rule decision (a 1,2,3… counter column the engine rightly called an increment) back to plain integer because the header says "pages" — the exact metadata-beats-data inversion decision 0048 forbids. *(Precision Principle: both are silent correctness losses on common shapes.)*

- **One real bug:** F2 (hostname→docker_ref) writes a label that doesn't exist in the taxonomy or the model's label space (`technology.container.docker_ref` instead of `technology.development.docker_ref`). Every column it "corrects" becomes an orphan with no validator and no type — strictly worse than leaving it alone. Verdict-only; the fix is a three-site string change for Stage 2.

- **One reproducibility wart:** locale annotation on plain 5-digit US ZIPs comes back random across runs (AR_MA, then SV, then ID, then HE) because many locales share the identical `^\d{5}$` pattern and the tie-break is non-deterministic. It can't mislabel a column (annotation-only), but the output isn't reproducible — and via decision 0001 it could make precision behaviour itself run-dependent on tied numeric codes.

- **An output-schema gap:** the JSON-Schema path attaches a closed `enum` to open-domain types (years, IANA timezones) whenever a small sample happens to be low-cardinality — recreating the very `enum_overfit` failure the eligibility gate was written to kill. The regression tests pass because they test a *parallel* code path the CLI doesn't ship.

**Validator precision across 240:** 198 are precise, **27 are weak** (confirm roughly any input — these aren't validations, they're shape-checks: hostname, plain_text, categorical, the optional-delimiter container splitters, the registry-less code lists like iata/icao/currency_code), and **15 have a locale-gap** (declared locale-specific but missing the per-locale rules where the real distinction lives — names, cities, regions, abbreviated-month dates). The weak count matters most for the hard veto: a weak validator can neither confirm nor refute, so it offers no protection where the model is uncertain. The locale-gaps are the Precision Principle's own to-do list — expanding `validation_by_locale` is the path to closing them.

**False vetoes — all three resolved, none requires loosening a validator:** one is a Sharpen rule aiming at the wrong sibling (`gender` should be `gender_code` for single-char codes), one is bad fixture data (`npi` synthesised without the 1-or-2 first-digit rule — the validator is *right* to reject it), and one is a hardcoded `price` hint asserting US currency format for a multi-locale column. The pattern: the validators are mostly doing their job; the conflicts are upstream Sharpen/hint decisions and test data, not over-strict validation.

**What we still don't know:** Stage 1 is a single instrument. It can say "this rule never fired on my adversarial CSVs" but not "this rule never fires across the corpus". Five steps flagged DEAD CANDIDATE, three rules flagged DRIFTED, and the net-value of every hardcoded header-hint family all hinge on the Stage-2 corpus ablation — and per the standing multi-instrument discipline, a single GO is not safety: nothing gets deleted until the corpus-honest gate, m-19, the rare-type scoreboard and gold-anchor clear *together*. We also don't yet know how often the weak validators silently pass wrong types at corpus scale, nor how many real columns the JSON-Schema enum-overfit gap actually harms in the round-trip. **One-liner for a stakeholder: the deterministic layer is mostly sound, but three rules now correct columns that no longer need correcting and one writes a broken label — all flagged for the corpus check, none touched.**