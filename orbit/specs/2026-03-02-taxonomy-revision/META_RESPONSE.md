## DATETIME FORMATS
I firmly agree with this point:
`40 datetime types have unique format_strings — each is a distinct transformation contract. Merging them would lose strptime precision.`

This is the acid test. If an analyst still has to hand craft the strptime format string then velocity is lost, so is the joy and the value of the library.

Given the variety of formats in the wild, not to mention the locale factor I expect our number of datetime formats to increase, not consolidate.

Let's get creative and structured. If we need to split things out by 'Date', 'Time' and 'Datetime' that's fine. Analysts will thank us.

## CURRENCY
I also agree firmly with this: `Currency amount with symbol is the #1 gap`

If we consider Excel's custom formats - it's clear that we've barely scratched the surface.

Again, we need to name each format, provide a casting method and detect locale as mentioned by 'locale-aware number'.

## INCREMENT AND UUID

Incrementing is an important database concept: https://pandas.pydata.org/docs/reference/api/pandas.Series.is_monotonic_increasing.html

So is UUID - handled well by DuckDB.

Are these worthy of their own category?

## HTML
Isn't HTML an instance of XML?
- If not yes, let's create a new type?
- If it is, we can still classify it and treat it with similar casting.

There are options to handle both here: https://duckdb.org/community_extensions/extensions/webbed

## CATEGORICALS
Something we're taking for granted is the 'categorical' - which is one of the most important analytical tools available.

DuckDB has a blog on this: https://duckdb.org/2021/11/26/duck-enum comparing enum to Python (Pandas Categorical) and R (Factors)
And of course the docs: https://duckdb.org/docs/stable/sql/data_types/enum

We support this already and it's amazing. We need to celebrate this. It also gives us more opportunity to cast things like 'periodicity'. 

```
╰─ ❯ finetype profile -f iris.csv
Loading model from "models/default"
Loaded semantic hint classifier (Model2Vec)
Loaded entity classifier (full_name demotion gate)
Loaded taxonomy for attractor demotion (163 types, 163 validators cached, 5 with locale validators)
Reading "iris.csv"
Found 5 columns: ["sepal_length", "sepal_width", "petal_length", "petal_width", "species"]
Read 150 rows
FineType Column Profile — "iris.csv" (150 rows, 5 columns)
════════════════════════════════════════════════════════════════════════════════

  COLUMN                    TYPE                                            CONF
  ──────────────────────────────────────────────────────────────────────────────
  sepal_length              representation.numeric.decimal_number          80.0% [si_number_override_no_suffix]
  sepal_width               representation.numeric.decimal_number          85.0% [si_number_override_no_suffix]
  petal_length              representation.numeric.decimal_number          80.0% [si_number_override_no_suffix]
  petal_width               representation.numeric.decimal_number          80.0% [si_number_override_no_suffix]
  species                   representation.discrete.categorical            66.0% [attractor_demotion_cardinality:identity.person.username]
  ```
  
  
## OTHER AMMENDMENTS
- Add geography.location.country_code - agree. We should also 
- Remove datetime.component.century - agree.
- Remove CVV - agree.
- Review datetime.component.periodicity - I think we should keep this and cast to ENUM.
- Add technology.internet.uuid - agree, but this is broader than technology. It appears in database design often.


___

## REGARDING YOUR QUESTIONS
  1. identity.payment restructuring scope: I believe finance should be it's own domain
  2. Currency amount design: we MUST tackle locale
  3. Datetime components: I've provided feedback above
  4. Scope of this revision: Make as many changes as we need to, the taxonomy is our most important feature.
