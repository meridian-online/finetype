# Design: Validate-precision corpus curation — iter-2

**Date:** 2026-04-28
**Interviewer:** Nightingale (guided)
**Card:** orbit/cards/0014-profile-validate-precision.yaml
**Mode:** design

---

## Context

Iter-1 (card 0014, spec `2026-04-28-validate-precision-corpus`) shipped:
- `make validate-corpus` round-trip harness
- 7-CSV corpus (pokemon, rio2016_athletes, us_baby_names, co2_emissions_by_nation, world_population, un_locode, global_temp_annual)
- Headline `3 of 7 datasets pass at P=99%`
- Per-mechanism breakdown:
  - enum_overfit: 3 cols / 2 datasets
  - **format_diversity: 0 cols / 0 datasets** (zero coverage)
  - misclassification: 7 cols / 4 datasets (dominant)
  - **code_vs_canonical: 0 cols / 0 datasets** (zero coverage)
  - unknown: 0; no_gt: 0
- Two preventive fixes: `--enum-threshold` 50→32 + `decimal_number` widened for scientific notation
- 3 MADRs (0072 round-trip metric, 0073 m-19 reuse, 0074 in-scope fix partition)
- Stub follow-up card 0015 (this card)

**Iter-2 framing:** the metric has zero signal on two of the four designed mechanisms. Adding more datasets without targeting that gap would grow M with the same shape. Curation, not engineering — harness already accepts manifest-only growth.

---

## Q&A

### Q1: Iter-2 size
**Q:** How many datasets should iter-2 add to the validate-precision corpus (currently 7)?
**A:** +5 datasets (12 total). ~71% growth, enough to halve per-dataset noise and add 2+ datasets to each underrepresented mechanism. Curation work is mechanical once choices are settled.

### Q2: Mechanism focus
**Q:** Which mechanism(s) should drive iter-2 dataset selection?
**A:** format_diversity AND code_vs_canonical equally. Both at 0 attributions in iter-1's breakdown. Equal priority closes the zero-coverage gap so the per-mechanism metric has signal beyond enum_overfit + misclassification.

### Q3: Source strategy
**Q:** Where should iter-2 source candidate datasets from?
**A:** Public open-data portals only. Same pattern as iter-1: data.gov, worldbank, kaggle public datasets, UN data portals, NYC TLC. Preserves licence + provenance discipline; clean PR-only growth path.

### Q4: GT density
**Q:** Should every column in new datasets have a GT sidecar entry (matching iter-1's 100% policy)?
**A:** 100% — every column gets `expected_label`. Same policy as iter-1. Keeps the rule simple; no column becomes invisible due to missing GT; no_gt mechanism stays at 0.

### Q5: Exit criterion
**Q:** What's iter-2's exit criterion (when can we ship)?
**A:** Mechanism coverage — ≥1 attribution each in format_diversity + code_vs_canonical. The point of iter-2 is signal in the per-mechanism breakdown. Ship when both zero-coverage mechanisms have at least one attribution; pass-rate change is observable but not required.

### Q6: Row cap
**Q:** Should iter-2 keep iter-1's 5000-row sample cap on large datasets?
**A:** Yes — same 5000-row cap via deterministic head-sampling. Continuity with iter-1. Caps repo footprint; harness already accepts un-capped via manifest. Per-dataset noise above ~5000 rows is small for P=99% gating.

### Q7: Datasets
**Q:** Of three candidate sets, which should iter-2 add?
**A:** Set A — NYC Taxi (FD), GDELT events (FD), FIFA players (CvC), NASDAQ tickers (CvC), OECD employment (CvC mixed). 2 format_diversity + 3 code_vs_canonical. Public-domain or CC0/CC-BY.

Per-dataset target mechanisms (best-effort, may differ at attribution time per Q9):
- **NYC Taxi (FD):** `pickup_datetime` ISO 8601 + traditional MM/DD/YYYY HH:MM:SS variance; `tip_amount` decimal precision variance
- **GDELT events (FD):** datetime in `YYYYMMDDHHMMSS` short form (no separators) — tests format-diversity against `datetime.timestamp.iso`
- **FIFA players (CvC):** `nationality` as country names not canonical (e.g. "England" vs "United Kingdom" vs "GB") — tests `geography.location.country` vs validator alpha-2/alpha-3
- **NASDAQ tickers (CvC):** ticker symbols with exchange suffixes (AAPL vs AAPL.US vs AAPL.NYSE) — tests `finance.market.ticker_symbol` validator
- **OECD employment (CvC mixed):** country codes alpha-2 / alpha-3 mixing in same column — tests `geography.location.country_code` strictness

### Q8: Fix budget
**Q:** Should iter-2 include in-scope engine fixes (like iter-1's enum-threshold + decimal_number widening), or stay pure curation?
**A:** Pure curation — no engine fixes. Iter-2 is data + manifest + sources + GT only. No taxonomy edits, no CLI changes. Keeps the iteration mechanical and PR-only growth. Engine fixes belong to follow-up cards once MADR 0066 retrain criteria is met.

### Q9: Mismatch policy
**Q:** What if a chosen dataset doesn't trigger its expected mechanism (e.g., NYC Taxi lands in misclassification not format_diversity, like iter-1's un_locode)?
**A:** Document the mismatch and ship anyway — mismatch is itself signal. Iter-1's un_locode showed misclassification can mask format_diversity downstream. Treating that as a finding (not a defect) keeps the curation pipeline honest. The dataset still contributes to M and to mechanism counts; the report shows what the harness actually attributes.

---

## Summary

### Goal

Grow the validate-precision corpus from 7 → 12 datasets via PR-only manifest growth (no harness code change), prioritising datasets that exercise the two zero-coverage mechanisms (format_diversity + code_vs_canonical) so the per-mechanism breakdown has signal across all four designed mechanisms.

### Constraints

- **Pure curation.** No engine fixes (no taxonomy edits, no CLI changes, no harness code changes).
- **Public open-data sources only.** Each dataset has a verifiable source URL and SPDX-allowlisted licence.
- **5000-row cap** via deterministic head-sampling for datasets exceeding that size; preserves header + first 5000 data rows.
- **100% GT coverage** per column in new datasets — every column gets `expected_label` in the sidecar.
- **Realism floor passed** via `scripts/prescreen_eval.py` against MADR 0055 floors.
- **Leakage firewall extended** — new dataset rows added to `eval/row_hashes.tsv` via `compute_row_hashes.py`.
- **Sources.yaml schema reuse** — new datasets registered with `role: validate` (no schema change; additive rows only).
- **Harness contract frozen** — `validate_corpus.rs` not modified.
- **Existing sources/manifest rows byte-unchanged** — additive only.

### Success criteria

- 5 new datasets added, each meeting the constraints above
- Updated baseline + post-state reports committed (`validate_corpus.md`, `validate_corpus.iter2.md`)
- Per-mechanism breakdown shows ≥1 attribution in format_diversity AND ≥1 attribution in code_vs_canonical
- `make ci` exits 0
- `make validate-corpus` runs end-to-end on 12 datasets
- Card 0015 updated: scenarios reflect iter-1+iter-2 ship trail; maturity stays `emerging` until corpus reaches ≥20 datasets per card goal

### Decisions surfaced

- **Pure curation** chosen over reactive fixes — fixes wait for the next retrain card. (No new MADR; iter-2 references MADRs 0072/0073/0074.)
- **Mechanism mismatch is signal, not defect** — un_locode precedent. Documented in the report; no re-pick policy.
- **Set A (2 FD + 3 CvC)** chosen over format-heavy or code-heavy alternatives — keeps mechanism coverage symmetric.

### Implementation notes

- Each dataset addition is a 4-file PR fragment: CSV + manifest row + sources.yaml entry + GT sidecar
- Source URL discipline: pinned URL with `fetched_date` matching commit date
- Per-dataset row count expected before sampling: NYC Taxi (millions), GDELT (millions), FIFA (~18k), NASDAQ tickers (~3-4k), OECD employment (~10-50k); all need head-sampling cap except possibly NASDAQ tickers
- Pre-pick verification: each candidate dataset must pass realism floor; if a dataset fails (e.g., dominated by NULLs or single-value columns), drop and pick alternate within the same mechanism bucket
- Expected vs actual mechanism mapping documented per dataset in the spec; mismatches recorded in the iter-2 report

### Open questions

None — all design decisions resolved.
