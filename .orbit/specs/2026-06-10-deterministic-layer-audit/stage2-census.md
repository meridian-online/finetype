# Stage-2 Corpus Census — Deterministic Layer Audit

**Sample & method.** 4,131 files (every 8th of the 33,250-file stratified sample; 26 of 4,157 failed parquet→CSV). Two passes, both on the 0.6.26 release binary against `models/default`:
- **Feature-rule pass** — batch `profile --files … -o json-schema` with `RUST_LOG=finetype_model=debug`, stderr **piped** (a file redirect silently suppresses the DEBUG subscriber — a trap worth noting). `Reading "<path>"` lines attribute each `Feature sharpen rule applied` event to its file. 3,443 feature fires captured.
- **Header-hint pass** — single-file `profile -v` (header-hint applies tag only the per-column summary line in the MB path; they emit no DEBUG), ~6 files/s, 19,873 tagged lines. Victim values pulled from source parquet.

## Q1 — Dead-candidate firing census (4,131 files)

| Rule | Fires | Verdict |
|---|---|---|
| `header_hint_measurement` | **0** | **CORROBORATES-DEAD** |
| `header_hint_fallback` | **0** | **CORROBORATES-DEAD** |
| `feature_decimal_over_numeric_code` (F5 float branch) | **8** | **NOT DEAD** — fires `numeric_code → decimal_number` on 8 files. Stage-1's "unreachable" call is wrong; the float branch is live. Do **not** remove. |
| legacy F3 `hs_code` emitter | **0** | **CORROBORATES-DEAD** |
| `value_sharpen` R21/R3 (coord gate) | no coordinate-label transition observed | **CORROBORATES-DEAD** under current ordering |

Zero firings corroborate but don't prove dead. F5-float is the one reversal: it fires, so it stays.

## Q2 — F6 `feature_short_code_not_extension` drift

Fires **53 times** (45 distinct victim columns), all `representation.file.extension → representation.discrete.categorical`. Classified by sample values:
- **GAIN (genuine non-extension short codes correctly demoted): 43/45** — `protocol`=UDP/TCP, `Language_iso`=ISO-639-3, `act_tag`=speech-act codes, node IDs, `regressor`=poly/rbf/linear, `Component`=GUI/Core/Doc.
- **LOSS (real file extensions wrongly demoted): 2/45** — `EXT`=jpg, `mime`=PDF.

**Verdict: CONTRADICTS Stage-1.** Stage-1 predicted loss-heavy and "likely remove"; the corpus says F6 is **gain-heavy, ~21:1**. Loss magnitude is tiny (2 columns, a mild precision nick). Do **not** remove; F6 is net-positive.

## Q3 — Bare `header_hint` catchall (decision-0048 inversion)

Fires **187 times**. Every fire lands on a **generic** type — `integer_number` (120), `binary` (20), `decimal` (1), plus Count/Entry-size variants. **Zero** fires produced a non-generic override. Inspected victims (`api_count`/`count`→integer where the value already parsed integer; `deceased`/`active`→binary where values are 0/1): in the `api_count` case `feature_no_leading_zero` had **already** chosen integer — the catchall re-asserted the same label.

**Verdict: INCONCLUSIVE — drift hazard real but unobserved.** The metadata-beats-data inversion **did not occur once** in 4,131 files; every catchall output matched the data. The Stage-1 "pages-counter" hazard is structurally possible but too rare to surface here. (Could not A/B-disable: the `rhh-instrumentation` feature is off in the release binary.)

## Reconciliation with Stage 1

- **Remove-safe (zero corpus firings, pending the rest of the multi-instrument map):** `header_hint_measurement`, `header_hint_fallback`, legacy `hs_code` F3 emitter, value_sharpen R21/R3.
- **Stage-1 verdicts OVERTURNED by corpus evidence:** F5 float branch is **live** (Stage-1 said dead); F6 is **gain-heavy 43:2** (Stage-1 said loss-heavy/remove). Keep both.
- **Hazard downgraded, not cleared:** the bare-catchall 0048 inversion — unobserved across 4,131 files; leave in place.

Verdict-only; no removals made, no pipeline code touched. The one confirmed defect (F2 orphan label) was fixed separately (commit 6f8a341).
