# Residual measurement — ticker→geography and org-name→region over-emission

**Question:** is the geography over-emission on tickers / org-names (company-reference
audit seams 1a/1c) a material corpus seam worth a Sharpen guard + taxonomy leaf, or a
long tail to log and move on?

**Answer: long tail. Do not build a corpus-scale guard or a ticker leaf for this.**
The at-risk population is ~hundreds of columns, the *current* shipped model already
resolves the clear cases, and the stale corpus parquet was overstating the residual.

## Method + caveat

- Corpus prediction substrate: `eval/gittables/corpus_pass/columns.parquet` — 6,590,432
  columns / 505,693 files; 4,242,484 non-trivial. **Dated 22 May — its `sense_prediction`
  is a v22-era model (PRE the shipped m2v8m-s43, 24 Jun).** v22 over-emitted geography
  (country −31.5% / region −12.8% vs v19), so these counts are an **upper bound** on the
  current model. Values + headers are model-independent and used as-is.
- Current-model behaviour spot-checked by re-profiling real corpus columns with
  `target/release/finetype` (v0.6.47).

## Population at risk (by header, stale parquet)

| population | count | as % of 4.24M |
| --- | --- | --- |
| ticker-headed (`ticker`/`stock_symbol`/`bloomberg`…) | 120 | 0.003% |
| bare `symbol`-headed (ambiguous: also chemical/currency) | 1,793 | 0.04% |
| explicit org-name-headed (`company`/`entity`/`issuer`…) | 4,692 | 0.11% |

Total geography.location.* attractor (stale): ~82k (city 38k, region 20k, country 11k,
country_code 6k, state 3.9k, state_code 1.6k, continent 1.1k).

## Header-contradicted misfires (predicted geography, stale model)

| seam | count | note |
| --- | --- | --- |
| ticker/`symbol` header → geography.location.* | 168 | genuine (e.g. `DCO.DE`/`DIC.DE` XETRA tickers → region), but includes chemical symbols (`H,He,Li,Be`) and airport codes |
| explicit org/company/entity header → geography | 108 | |
| generic `name` header → region | 924 | **mostly NOT misfires** — sample is dominated by genuine regions (US states, Australian states, Scottish/US counties) + non-org misfires (species names, game characters, error codes); org-names are a handful |

**ticker→state_code specifically is tiny (~9).** Ticker-headed columns mostly land on
`iata_code`/`icao_code` (41/15) — airport codes share the short-uppercase shape — not state.

## Current-model reality check (v0.6.47, re-profiled)

- Canadian tickers (`GSPTSE,CP,DRT,FNV,MAG,MX,AEM…`), header `symbol` → **`unknown`**
  (raw model alphanumeric_id, vetoed). NOT geography.
- US tickers (`AAPL,MSFT,NVDA,KO,PG,XOM…`), header `Symbol` → **`unknown`**. NOT geography.
- gleif-like org-name column (`Siemens AG`, `Deutsche Bank AG`, `Nestle SA`…) WITH
  `jurisdiction`/`city`/`country` siblings → **`entity_name` 0.9992 (high)**, isolated or
  in context. NOT region.
- The current model only leans geography when the ticker values *heavily overlap real
  state codes* (`GA,IN,OK,MA,DE,OR,PA,MO` → region 0.52) — an adversarial edge case, not
  a typical ticker column.

So the current model over-emits geography *materially less* than the stale parquet shows;
the live residual is a fraction of the 168/108 header-contradicted counts.

## Verdict

- **Corpus-scale ticker→geography guard: NO.** Population ~120 headed; current model already
  routes clear tickers to unknown/alphanumeric_id; below any volume bar.
- **Ticker taxonomy leaf: NO** (on volume). ~120–1,900 columns is well under the ≥1,000
  *material-and-distinct* bar once the ambiguous `symbol` (chemical/currency) and the
  already-handled cases are netted out. Revisit only if a specific customer needs typed tickers.
- **Corpus-scale org-name→region guard: NO.** Current model → `entity_name`; the `name`→region
  bucket is mostly genuine regions or non-org noise.
- **The gleif/sec_edgar seams (audit 1a/1c) are EXTERNAL-TABLE edge cases, not a corpus
  pattern** — handle any real miss through the external-band triage queue (targeted gold rows),
  not a broad guard.

Net: measuring dissolved the seam. The rule pile does not need to grow here — which also
answers the strategic worry: we are not forced into more rule tweaks; there is no material
target to tweak toward.
