# Currency-variant recognition — design investigation (2026-06-12)

Carved out of the false-veto trio (spec 2026-06-12-false-veto-trio-resolution,
ac-04 price) once the investigation showed price is not a veto bug but a
locale-format recognition gap. Author chose "start price design now" + "batch
the release with gender".

## The symptom

`multilingual.csv` price column → `unknown [header_hint_hardcoded:price]
⊘ vetoed:finance.currency.amount (33% pass)`. Values are `9140,88 €`,
`2647,27 €`, … (comma decimal, € suffix).

## Three measured paths (0.6.29 release binary)

| Input | Result | Why |
|-------|--------|-----|
| `9140,88 €` (shipped fixture), header `price` | `unknown`, vetoed amount 33% | Value matches NO amount_* validator — no thousands separator on a 4-digit int. |
| `9.140,88 €` (proper CLDR), header `price` | `finance.currency.amount` (generic US) | `header_hint_cross_domain:price` forces US amount; value happens to pass its loose pattern. WRONG variant. |
| `9.140,88 €`, neutral header | `representation.numeric.decimal_number` | Model does not recognise euro currency shape on its own — currency lost. |

The correct type for `9.140,88 €` is `finance.currency.amount_comma_suffix`
(validator `^-?[0-9]{1,3}(\.[0-9]{3})*(,[0-9]{1,2})?\s?[€…]$`, CLDR default for
DE/FR/ES/IT/NL). No path reaches it.

## Two tangled problems

1. **Fixture is doubly artificial.** `9140,88 €` lacks the thousands dot, so it
   matches none of the 12 `amount_*` validators. Corrected `9.140,88 €` matches
   `amount_comma_suffix` AND `amount_comma`. npi-class fixture bug.
2. **Header hint overrides value shape.** `header_hint("price") →
   finance.currency.amount` (US: comma-thousands, dot-decimal). Even with a
   correct euro fixture, this forces the generic US variant. And dropping the
   hint loses currency entirely (→ decimal_number), because the model does not
   classify the euro shape itself.

## Representativeness — variants are real

Stratified candidate sample (33,007 cols): `amount` 23,048, `amount_accounting`
866, `amount_space` 516, `amount_comma` 408, `amount_nodecimal` 280,
`amount_apostrophe` 181, `amount_comma_suffix` 101, `amount_multisym` 39, …
So the locale variants are live in real data and worth recognising precisely.

## The fix (mirrors gender)

A **value-aware amount-variant selector**: when a column is currency-family
(price/cost/amount hint fires, or the model lands on any `amount*`), pick the
`amount_*` variant whose validator the values actually satisfy, instead of
forcing the generic US `amount` or leaving `decimal_number`. Same principle as
the gender guard — a value-grounded sibling beats the same-family header hint
(decision 0048, value-based). Scope strictly to the `amount_*` family; widen no
validator. The 12 variant patterns are the dispatch table; the column's sample
pass-rate against each is the selector.

## Honest scope limits

- The selector is a precision lever with its own over-emit risk (e.g. a plain
  decimal column nudged into `amount_*`). Gated by gold no-regression + the
  corpus-honest relocation detector on the `amount_*` family.
- The model still won't recognise currency without a header signal; this fix
  improves the *hinted* path and the *already-currency* path, not bare decimals.
  Teaching the model the euro shape is a separate (training) lever.
