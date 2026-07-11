# External band — full-pool split (39 non-trio misses, 2026-07-11)

The trio (gleif/sec_edgar/nyc_dob, 16 misses) was adjudicated in the first reading; the 5 gold errors there were corrected. This splits the remaining **39 misses across the other 12 tables**, using **representative stratified samples** (24 values evenly strided across each full column) + the whole-column shape distribution — the fix for the first-rows sampling bias. Each verdict was adversarially verified (a skeptic defends the rejected label). Substrate: `fullpool_split_2026-07-11.json`.

## Split

| verdict | n | meaning |
|---|---|---|
| MODEL_ERROR | 21 | model emitted a wrong specific type |
| MODEL_ABSTAINED | 11 | model returned `unknown`/generic where a real type fits (under-emission) |
| MODEL_CORRECT | 3 | model right, gold arguably wrong (all in the contested finance/geo band) |
| GOLD_ERROR | 2 | gold wrong (both low-conf, in the contested band) |
| BOTH_IMPERFECT | 2 | neither label fits (mixed-content or missing leaf) |

Net: the model is wrong on ~31 of 39 (21 errors + ~10 clean abstentions); gold is questionable on ~7, **none cleanly enough to auto-correct** (see below).

## Three failure classes

1. **The geography seam recurs** (10 columns): short-code→geo 4, org-name→geo 4, tld→geo 2. e.g. majestic `TLD`/`IDN_TLD` (`com`/`net`) → **continent**; nyc `agency_name`, ourairports/openflights `name`, usgs `place` → **region**; usgs `net`, seattle `checkouttype` → region. Same mechanism as the trio — the model target.
2. **Abstention is a large second class** (11 columns): the model returns `unknown` (or a generic catch-all) on a plainly-typeable column — airport/company `name`→entity_name, `county`/`iso_region`→region, `date_of_transfer`→sql_standard, `x_coordinate`/`mag`→number, `publicationyear`→year. This is the analyst-visible hole (a column comes back `unknown`) the review flagged — a *different* problem from the geography over-emission, and invisible to a relocation-only gate.
3. **The finance `amount`-vs-`decimal_number` question is contested** (5 money columns): bare-number salary/price columns (no currency symbol) gold-labelled `finance.currency.amount`, model says `decimal_number`/`unknown`. **The adjudicators split on semantically identical columns** (`base_salary`/`total_other_pay`→decimal, but `total_ot_paid`/`price`→amount) — which proves this is a taxonomy judgment call, not a mislabel. **Author call needed:** is a bare-number money column `finance.currency.amount` or `representation.numeric.decimal_number`? Whichever, apply it consistently.

## Why no full-pool gold corrections were applied

Unlike the trio's 4 date columns (mechanical 83/17 format evidence + unanimous blind panel), the full-pool gold-error candidates are **low-confidence judgment calls in the contested finance/text band** (`pay_basis`→plain_text conf 0.68, `total_other_pay`→decimal 0.80, `neighborhoods…`→city 0.61) — and the finance ones hinge on the unresolved `amount`-vs-`decimal` taxonomy question. Per the lesson from the trio (only correct gold on mechanical evidence + panel consensus), **none was auto-corrected**; the finance question is surfaced to the author instead.

## Confirmed model errors (21)

| table | column | model said | correct | mechanism | conf |
|---|---|---|---|---|---|
| majestic | TLD | continent | top_level_domain | tld->geography | 0.99 |
| ourairports | home_link | qualified_name | url | none | 0.99 |
| majestic | IDN_TLD | continent | top_level_domain | tld->geography | 0.98 |
| chicago | iucr | compact_dmy | numeric_code | code->date | 0.97 |
| nyc | agency_name | region | entity_name | org-name->geography | 0.97 |
| usgs | id | geohash | alphanumeric_id | short-code->geography | 0.97 |
| ourairports | name | full_address | entity_name | org-name->geography | 0.96 |
| seattle | checkouttype | region | word | org-name->geography | 0.95 |
| sf | naic_code | integer_number | numeric_code | none | 0.92 |
| naics | title | entity_name | plain_text | none | 0.9 |
| nyc | leave_status_as_of_june_30 | terms | word | none | 0.9 |
| uk | transaction_id | json | alphanumeric_id | none | 0.9 |
| usgs | net | region | word | short-code->geography | 0.9 |
| usgs | place | full_address | plain_text | org-name->geography | 0.9 |
| chicago | fbi_code | numeric_code | alphanumeric_id | none | 0.85 |
| chicago | location_description | entity_name | word | none | 0.85 |
| ourairports | continent | continent | word | short-code->geography | 0.85 |
| chicago | block | alphanumeric_id | plain_text | none | 0.8 |
| compound | title | entity_name | plain_text | none | 0.78 |
| sf | naic_code_description | entity_name | word | none | 0.72 |
| majestic | GlobalRank | increment | integer_number | none | 0.62 |

## Model abstentions (11 — `unknown`/generic where a type fits)

| table | column | should be | conf |
|---|---|---|---|
| chicago | x_coordinate | integer_number | 0.97 |
| ourairports | iso_region | region | 0.97 |
| usgs | mag | decimal_number | 0.97 |
| openflights | name | entity_name | 0.95 |
| uk | county | region | 0.95 |
| seattle | materialtype | word | 0.92 |
| nyc | title_description | plain_text | 0.9 |
| uk | date_of_transfer | sql_standard | 0.9 |
| seattle | publicationyear | year | 0.83 |
| nyc | total_ot_paid | amount | 0.8 |
| uk | price | amount | 0.7 |
