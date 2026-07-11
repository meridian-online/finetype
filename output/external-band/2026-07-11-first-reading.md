# External-data advisory band — first real reading

**Date:** 2026-07-11 · **Binary:** shipped default `m2v8m-s43` (v0.6.45) · **Status of the instrument:** ADVISORY, now wired into the promotion order between the representative band and the blocking corpus-honest gate.

> **Headline as the analyst lives it.** Point FineType at three real reference tables an analyst would actually load — a company register (GLEIF), a stock-symbol list (SEC EDGAR), and a building-permits export (NYC DOB) — and 16 of 32 verified columns come back disagreeing with truth. A full-column re-check splits that 16 into **11 genuine model errors** and **5 places where *gold* is wrong and the model is right** — so the band did two jobs at once: it found real model holes *and* audited the fixture we grade against. The model errors are nearly all one seam: it types stock tickers (`NVDA`) as **US state codes**, company names as **regions**, permit codes (`OT`, `EW`) as **states/countries**. None of the three standing instruments (gold 0.855, representative 0.708, corpus-honest gate) shows any of this, because none profiles a whole external table the model never trained on. That blind spot is exactly what this band closes.

## What was run

- **Runner:** `scripts/external_band.py` (promoted from the `output/repo-review/scaffolds/external-band/` stub). Profiles each full table with the shipped binary (sibling context live), scores every column that has a held label, triages the rest.
- **Held labels — live-derived, not frozen.** Every gold row in `eval/gold/gold_corpus.tsv` whose `file_path` points at a vendored external CSV becomes a held label. This auto-includes the `external:*` rows (openflights etc.) **and** the `compref:*` rows (gleif, sec_edgar, naics…) the company-reference audit adjudicated, **and** tracks gold growth. The scaffold's frozen `held_labels_v0.tsv` had gone stale — it held 7 nyc_dob labels; gold now has 16.
- **Two reads:** the **company-reference trio** you named (gleif/sec_edgar/nyc_dob), and the **full 15-table pool** for context.

## The numbers

| read | disagree / scored | headline | of the disagreements |
|---|---|---|---|
| **company-reference trio** (gleif, sec_edgar, nyc_dob) | 16 / 32 | **0.500** | 11 model errors · 5 gold errors |
| full external pool (15 tables) | 90 / 145 | **0.621** | (not yet fully re-adjudicated) |

Per-table (trio, raw disagreement): gleif **9/13**, sec_edgar **2/3**, nyc_dob **5/16**. After the gold-error split, the model is genuinely wrong on **11 of 32**.

These are *advisory* absolutes. Because the held labels overlap the gold headline, the number to act on across candidates is the **candidate-vs-baseline delta**, not the absolute — same rule as the representative band. This first reading is the baseline the next candidate is compared against.

## The seam — one failure class, most of the model errors

8 of the 11 model errors (and 1 of the 3 triage errors) are one mechanism: **a short uppercase code or an org name gets pulled onto a geography type** (`state_code`, `country_code`, `region`, `postal_code`), because those attractors' validators are *shape-only* (`^[A-Z]{2}$`, length-only) — they *confirm* the wrong type and *disarm* the attractor guard built to catch exactly this. It is the mechanism the company-reference audit traced, still live on the tables that motivated it.

| column | model says | correct | why it survives |
|---|---|---|---|
| sec `ticker` | state_code | word | only 14% of tickers even fit `^[A-Z]{2}$`; the shape-match on 2-char tickers disarms the veto |
| gleif `name` | region | entity_name | region is length-only, so company names pass its shape |
| gleif `category` | region | word | `FUND`/`GENERAL` pass region's length-only shape |
| gleif `legal_form` | postal_code | (numeric/alnum code) | 4-digit ELF codes fit a numeric postal shape |
| nyc `work_type` | state_code | word | `OT`/`EQ`/`MH` are 2 uppercase letters → pass `^[A-Z]{2}$`, but aren't states |
| nyc `permit_type` | country_code | word | `EW`/`FO`/`DM` pass the 2-letter shape; some even hit the ISO enum |
| nyc `gis_nta_name` | region | plain_text | neighbourhood names read as country subdivisions |

The remaining model errors: `street_name`→`street_address` (its own card excludes bare street names), `job__`→`word` (9-digit integers typed as text), `job_type`→`alphanumeric_id`, and **`gis_longitude`→`latitude`** (a lat/lon swap — both ranges validate, the header says longitude).

## The band audited gold — 5 labels it got wrong (the unexpected second job)

A full-column re-check (below) found that **5 of the 16 "misses" are gold mistakes, not model errors**:

- **4 NYC-DOB date columns** (`filing_date`, `issuance_date`, `expiration_date`, `job_start_date`) — gold says `datetime.date.mdy_slash`, but each column is **~83% `YYYY-MM-DD` ISO + ~17% `MM/DD/YYYY` slash** (the export mixes two date formats). The model's `datetime.date.iso` **validates the 83% majority**; gold's `mdy_slash` would reject it. The model is the more-correct call. *(These columns are also genuinely 17% un-validatable under any single date leaf — a real data-quality signal FineType could surface but doesn't yet.)*
- **gleif `entity_status`** (`ACTIVE`/`INACTIVE`) — the model's `representation.boolean.terms` (whose definition explicitly names "active/inactive") beats gold's generic `representation.text.word`.

These were **gold-fix candidates** (panel-proposes / author-ratifies, per the growth policy). **APPLIED 2026-07-11**: a blind 3-panel — given representative stratified samples + the format distribution, blind to which label was model-vs-gold — returned **unanimous 3/3** on all five (dates → `iso`, entity_status → `boolean.terms`), independently corroborating the mechanical evidence. `gold_corpus.tsv` corrected in place (5 rows, provenance appended recording the prior label + reason). The band trio re-scores **16/32 → 21/32 = 0.656** against corrected gold; the model gains **+5 correct** columns on the gold headline (these five flip model-wrong→model-right). Panel record: `output/external-band/gold_correction_panel_2026-07-11.json`. They also make a point that lands for the whole review: the band **cross-checks the fixture against real full-column data**, and the fixture had errors the curated-hard process missed (the 4 dates were sampled from the `iso` stratum, then a 2026-06-10 2-panel over-corrected them to `mdy_slash` on the same first-rows bias).

## Honest scope — a sampling-bias catch

My first pass over-counted. To adjudicate each column I first fed a 37-agent adversarial workflow the **first 12 non-null values** of each column. For the NYC date columns those first rows are almost all slash-format (the data is clustered), so the adjudicators — reasoning correctly from what they saw — ruled all four "model says iso but the values are slash → error." The **full column** is the opposite (83% iso). The lesson, now baked into the reading: **sample-based adjudication must be backed by a full-column validation-fraction check** before a verdict counts. The corrected verdicts above use the full column: the objective test is which label's pattern validates more of the actual data. The 11 model errors survive that test; the 4 dates flip to gold errors. Adjudication substrate (pre-correction): `output/external-band/adjudication_2026-07-11.json`.

## Cross-reference to the company-reference audit (the tables it used)

The audit shipped per-manifestation fixes (v0.6.39–0.6.44: unlocode guard, geo-vote, W2b entity_name, s_expression). The band shows what those fixes did and did **not** close on the very tables that motivated them:

- **Fixed & confirmed:** gleif `jurisdiction` → `country_code` ✓ (the `geo_code_membership_vote` fix holds); gleif `lei`/`city`/`country`/`region` ✓; sec_edgar `cik`/`name` ✓.
- **Still live (relocated, not removed):** the audit's **1a ticker** failure — ticker was `icao_code`, now `geography.location.state_code` on `NVDA`/`GOOGL`. The audit's **1c org-name** failure — was `person.full_name`, now `geography.location.region` on gleif `name`. The fixes moved the attractor; they did not land the column on the right type.

## Triage queue (candidate expansion — not in any headline)

10 profiled trio columns have no held label yet. Seven are plausibly-correct numeric IDs (`block`, `lot`, `community_board`, `house__`, `permit_sequence__`, `gis_census_tract`, `gis_council_district` — all `integer_number`). The **three flagged suspicious are all model errors** (full-column checked): `bldg_type` (values `1`/`2` → `compact_dmy`, an 8-digit-date pattern that rejects 100% of them — a genuine validator-rejection), `permit_subtype` (`OT`/`MH` → `region`, the same seam), `site_fill` (phrases → single-`word`). That is the failure-hunting yield: pointing the band at a real table found three more errors the headline never counted. Turning them into headline signal needs the panel-proposes/author-ratifies path.

## Growth policy (author-decided 2026-07-11)

1. **Truth tier for triage-queue adjudications: panel proposes, author ratifies.** Blind multi-model panel proposal, author ratifies contested calls before a label joins gold. Mirrors how the `compref:*` gold was built. Until ratified, triage stays triage-only.
2. **Gold overlap: keep external rows in both, delta-only + tier disclosure.** The band reports the candidate-vs-baseline delta and the tier-mix of scored labels, never the absolute in a blocking number — no re-baseline of gold. (Shipped behaviour.)
3. **Rotation source (v1): fixed on-disk pool for now.** Network-free and reproducible; rotate subsets with `--rotate/--seed`. Quarterly fresh-fetch is deferred.

## What we don't know yet, and what to do

- **There is no slash-date bug** — the recommendation I opened with was wrong, caught by the full-column re-check. The date detector correctly abstains on a genuinely-mixed column and the model's `iso` is the majority-correct call. Do not touch the detector.
- **Two real actions.** (a) **The 5 gold corrections — DONE** (blind panel 3/3, applied, band trio 16→21/32). (b) **The short-code / org-name → geography seam** is the genuine model target — 8 of the trio's errors and directly the audit's 1a/1c mechanism; it is a Sharpen-guard or retrain job, gated by the blocking corpus-honest gate, not a quick edit. Still open.
- **The full pool is split — DONE** (`output/external-band/full_pool_split_2026-07-11.md`). Post-correction the pool scores **95/145 = 0.655**; the 39 non-trio misses split into **21 model errors + 11 abstentions + a contested finance band**. Three findings: (1) the geography seam recurs (10 more — `TLD`→continent, `agency_name`/`name`→region); (2) **abstention is a big second failure class** — 11 columns return `unknown` on typeable data (names, dates, coordinates), the analyst-visible hole a relocation gate can't see; (3) a **contested `finance.amount` vs `decimal_number` taxonomy question** on bare-number money columns — the adjudicators split on identical columns, so it's a judgment call, not a mislabel. **No full-pool gold corrections were auto-applied** (unlike the trio dates, these are low-confidence judgment calls); the finance question goes to the author.
- This reading is one binary — it establishes the baseline, not a regression. The delta signal exists only once a *candidate* is scored against it.
