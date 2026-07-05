# Gated-YDF is a shape-matcher for numeric checksum types (2026-07-05)

**Question (author, W2b npi/upc adjudication):** the npi/upc corpus-honest NO-GO
rests on the gate's `collapse` band, which counts loss of *oracle-confirmed*
support and treats `ydf_prediction_gated` as ground truth. Is that oracle
reliable for these types?

**Method:** for every checksum-bearing label L that gated-YDF asserts
(`ydf_prediction_gated = L`) in the 33k-file corpus-honest sample, compute the
fraction of that column's sample values which carry a **valid check digit** by
the real algorithm (`crates/finetype-core/src/checksum.rs`, reimplemented and
cross-checked). A reliable oracle asserts L only for genuine L; a shape-matcher
asserts L for anything the right length. Script:
`output/company-reference-audit/ydf_reliability.py` (against `eval_w2b_substance` candidate +
`w3_baseline_with_oracle`).

**Result — gated-YDF's validation reliability, worst first:**

| oracle asserts | cols | median passrate | % cols ≥50% valid | % cols <10% valid | reading |
|---|---|---|---|---|---|
| identity.commerce.ean | 7 | 0.00 | 0.0% | 100% | SHAPE-MATCH |
| finance.securities.cusip | 16 | 0.12 | 0.0% | 31% | SHAPE-MATCH |
| finance.securities.sedol | 1 | 0.00 | 0.0% | 100% | SHAPE-MATCH |
| finance.payment.credit_card_number | 67 | 0.12 | 1.5% | 45% | SHAPE-MATCH |
| identity.commerce.upc | 188 | 0.00 | 2.7% | 67% | SHAPE-MATCH |
| finance.banking.aba_routing | 438 | 0.00 | 5.5% | 65% | SHAPE-MATCH |
| **identity.medical.npi** | **508** | **0.00** | **9.6%** | **63%** | **SHAPE-MATCH** |
| identity.commerce.isbn | 1056 | 0.00 | 16.8% | 60% | SHAPE-MATCH |
| finance.banking.iban | 40 | 0.00 | 45.0% | 55% | mixed |
| finance.securities.isin | 3 | 1.00 | 100.0% | 0% | RELIABLE (control) |

**Reading.** Gated-YDF asserts `npi` for 508 columns; only ~49 (9.6%) carry a
valid NPI check digit — the other ~459 are financial figures and random 10-digit
numbers. `upc` 2.7%, `ean` 0%, `credit_card` 1.5%, `aba` 5.5%, `isbn` 16.8%. The
only RELIABLE row is `isin` (100% of 3) — a type whose country-prefix +
alphanumeric structure resists accidental shape-match. So gated-YDF is a
shape-matcher for length-defined numeric checksum types and reliable only where
real structure exists.

**Root cause.** The "gated" in gated-YDF NULLs a prediction only when the
column's **schema-validation pass rate** (the *shape* pattern) is below 50%. A
column of 10-digit numbers passes the npi *pattern* 100%, so it survives gating.
Gating is therefore structurally blind to the exact shape-vs-substance
distinction the checksum guards exist to make — it cannot filter a shape-match
for a checksum type.

**Consequence for the gate.** The corpus-honest `collapse` band's
"oracle-confirmed support" is, for every numeric checksum type, mostly FALSE
confirmations. The npi/upc NO-GO is a false alarm: the guards demote columns the
oracle "confirmed" but which fail the real check digit (per-column correct;
proven separately — 20/20 sampled demoted-npi columns are `ebit`/`marketCap`/
`totalAssets`). Extends the instrument audit ("gated-YDF wrong 42% on contested
ground") with a per-type number: on numeric checksum types it asserts correctly
2–17% of the time. This does NOT touch the gate's `over_emit`/`oracle_fp`
relocation-detection role for non-checksum types — only its authority to
adjudicate checksum-type demotions via the collapse band.
