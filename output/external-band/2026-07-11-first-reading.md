# External-data advisory band — first real reading

**Date:** 2026-07-11 · **Binary:** shipped default `m2v8m-s43` (v0.6.45) · **Status of the instrument:** ADVISORY, now wired into the promotion order between the representative band and the blocking corpus-honest gate.

> **Headline as the analyst lives it.** Point FineType at three real reference tables an analyst would actually load — a company register (GLEIF), a stock-symbol list (SEC EDGAR), and a building-permits export (NYC DOB) — and **roughly half the columns come back mis-typed**: of 32 columns with a verified label, 16 disagree with truth, and an adversarial adjudication (37 agents, a skeptic told to defend the model on every call) confirmed **15 as genuine model errors** — the 16th turned out to be a *gold* mistake the tool actually got right. It types stock tickers (`NVDA`, `GOOGL`) as *US state codes*, company names as *regions*, and US slash-dates (`06/17/2020`) as *ISO dates* — and in **9 of those cases FineType's own `validate` would then reject the very values it just labelled**. The three unlabelled columns flagged for triage were *also* all wrong. None of the three standing instruments (gold 0.855, representative 0.708, corpus-honest gate) shows any of this, because none profiles a whole external table the model never trained on. That blind spot is exactly what this band closes.

## What was run

- **Runner:** `scripts/external_band.py` (promoted from the `output/repo-review/scaffolds/external-band/` stub). Profiles each full table with the shipped binary (sibling context live), scores every column that has a held label, triages the rest.
- **Held labels — live-derived, not frozen.** Every gold row in `eval/gold/gold_corpus.tsv` whose `file_path` points at a vendored external CSV becomes a held label. This auto-includes the `external:*` rows (openflights etc.) **and** the `compref:*` rows (gleif, sec_edgar, naics…) the company-reference audit adjudicated, **and** tracks gold growth. The scaffold's frozen `held_labels_v0.tsv` had gone stale — it held 7 nyc_dob labels; gold now has 16.
- **Two reads:** the **company-reference trio** you named (gleif/sec_edgar/nyc_dob), and the **full 15-table pool** for context.

## The numbers

| read | correct / scored | headline | unlabelled (triage) |
|---|---|---|---|
| **company-reference trio** (gleif, sec_edgar, nyc_dob) | 16 / 32 | **0.500** | 10 |
| full external pool (15 tables) | 90 / 145 | **0.621** | 52 |

Per-table (trio): gleif **9/13**, sec_edgar **2/3**, nyc_dob **5/16**.

These are *advisory* absolutes. Because the held labels overlap the gold headline, the number to act on across candidates is the **candidate-vs-baseline delta**, not the absolute — same rule as the representative band. This first reading is the baseline the next candidate is compared against.

## The seam — one failure class, many columns

The per-type recall table isolates it. Everything that should be a plain word/code/text and everything that is a US-format date is where the damage is:

| gold label | recall (trio) | what the model does instead |
|---|---|---|
| `representation.text.word` | **3/9 = 0.33** | short uppercase codes → geography attractors (`state_code`, `country_code`, `region`) or `boolean.terms` |
| `datetime.date.mdy_slash` | **0/4 = 0.00** | US slash-dates → `datetime.date.iso` (pattern that rejects the values) |
| `representation.text.plain_text` | **0/2 = 0.00** | street names / neighbourhood names → `geography.*` |
| `geography.coordinate.longitude` | **0/1 = 0.00** | longitude → latitude (lat/lon swap) |
| `representation.numeric.integer_number` | 0/1 | 9-digit job number → `word` |
| `representation.identifier.alphanumeric_id` | 0/1 | ELF codes → `geography.address.postal_code` |

The through-line is the one the company-reference audit already traced: **short uppercase codes and org names get scattered across geography / datetime / boolean attractors** whose validators are *shape-only* (`^[A-Z]{2}$`, `^\d{8}$`), which both *confirm* the wrong type and *disarm* the attractor guard built to catch exactly this.

## Cross-reference to the company-reference audit (the tables it used)

The audit shipped per-manifestation fixes (v0.6.39–0.6.44: unlocode guard, geo-vote, W2b entity_name, s_expression). The band shows what those fixes did and did **not** close on the very tables that motivated them:

- **Fixed & confirmed:** gleif `jurisdiction` → `country_code` ✓ (the `geo_code_membership_vote` fix holds); gleif `lei`/`city`/`country`/`region` ✓; sec_edgar `cik`/`name` ✓.
- **Still live (relocated, not removed):** the audit's **1a ticker** failure — ticker was `icao_code`, now `geography.location.state_code` on `NVDA`/`GOOGL` (a 4–5-char value under a `^[A-Z]{2}$` pattern). The audit's **1c org-name** failure — was `person.full_name`, now `geography.location.region` on gleif `name`. The fixes moved the attractor; they did not land the column on the right type.

## Adjudication — which misses are real

*(A 37-agent adversarial workflow: each of the 19 misses/triage columns adjudicated independently, then attacked by a skeptic told to defend the model. ~1.26M tokens. Substrate: `output/external-band/adjudication_2026-07-11.json`.)*

**18 of 19 columns adjudicated as genuine model errors; the adversarial skeptic pass refuted 0.** Each verdict came from an independent adjudicator (sample values + taxonomy definitions in hand), then a second agent instructed to *defend the model* attacked it — none survived. The one non-error is a **gold** mistake the band surfaced.

**Confirmed model errors — hard misclassifications (11):**

| column | correct leaf | mechanism | audit ref | own validator rejects the data? | conf |
|---|---|---|---|---|---|
| sec `ticker` | word | short-code→geography | 1a-ticker | **yes** | 0.97 |
| gleif `name` | entity_name | org-name→geography | 1c-orgname | no | 0.98 |
| gleif `category` | word | short-code→geography | new | no | 0.95 |
| gleif `legal_form` | alphanumeric_id | short-code→geography | 1a-ticker | no | 0.90 |
| nyc `street_name` | plain_text | none | new | no | 0.85 |
| nyc `job__` | integer_number | number→text | new | no | 0.92 |
| nyc `job_type` | word | text→id | new | **yes** | 0.90 |
| nyc `work_type` | word | short-code→geography | 1a-ticker | no | 0.95 |
| nyc `permit_type` | word | short-code→geography | 1a-ticker | **yes** | 0.95 |
| nyc `gis_longitude` | longitude | lat/lon-swap | new | no | 0.99 |
| nyc `gis_nta_name` | plain_text | none | new | no | 0.68 |

**Confirmed model errors — date-format granularity (4):** all four NYC-DOB date columns are US `MM/DD/YYYY` slash-dates (e.g. `06/17/2020`) emitted as `datetime.date.iso`, whose pattern `^\d{4}-\d{2}-\d{2}$` **rejects every value** — a `validate` pass would null the whole column. Columns: `filing_date`, `issuance_date`, `expiration_date`, `job_start_date` (conf 0.98–0.99, all validator-rejecting).

**Gold-label correction the band found (1):** gleif `entity_status` (ACTIVE/INACTIVE) — the shipped model says `representation.boolean.terms`, whose definition explicitly names "active/inactive". The adjudicator ruled the model **more precise than gold's `representation.text.word`** (a blind-panel call made without the taxonomy in view). It counts against the raw headline but *for* the model, and is a gold-fix candidate. Net: the model is genuinely wrong on **15 of 32**, not 16.

**Triage queue — all 3 suspicious emissions confirmed as errors (the failure-hunting yield):**

| column | predicted (shipped) | correct leaf | mechanism | conf |
|---|---|---|---|---|
| nyc `bldg_type` | compact_dmy | integer_number | code→date | 0.92 |
| nyc `permit_subtype` | region | word (categorical enum) | short-code→geography | 0.90 |
| nyc `site_fill` | word | plain_text (categorical enum) | none | 0.60 |

**Seam tally (confirmed errors):** short-code→geography **6** (ticker, category, legal_form, work_type, permit_type, permit_subtype), org-name→geography **1** (name), date-format **4**, lat/lon-swap **1**, number→text **1**, code→date **1**, text→id **1**. **9 confirmed errors fail their own emitted validator** — FineType labels the column, then its own `validate` would reject the data. **5 cross-reference directly to the audit's 1a-ticker failure and 1 to 1c-org-name** — the exact holes the audit's per-manifestation fixes were meant to close.

## Triage queue (candidate expansion — not in any headline)

10 profiled trio columns have no held label yet. Seven are plausibly-correct numeric IDs (`block`, `lot`, `community_board`, `house__`, `permit_sequence__`, `gis_census_tract`, `gis_council_district` — all `integer_number`, no obvious error). The **three flagged as suspicious were adjudicated above and all confirmed wrong** (`bldg_type` `1`/`2` → 8-digit date; `permit_subtype` `OT`/`MH` → region; `site_fill` phrases → single-word). That is the failure-hunting yield: pointing the band at a real table found three more errors the headline never counted. Turning them into headline signal needs a truth tier (author call below).

## Growth policy (author-decided 2026-07-11)

1. **Truth tier for triage-queue adjudications: panel proposes, author ratifies.** New over-emissions the triage queue surfaces get a blind multi-model panel proposal; the author ratifies contested calls before the label joins gold and can count toward a headline. Mirrors how the `compref:*` gold was built (llm-3panel-blind + author overrides). Until ratified, triage stays triage-only.
2. **Gold overlap: keep external rows in both, delta-only + tier disclosure.** The band reports the candidate-vs-baseline delta and the tier-mix of scored labels, never the absolute in a blocking number — no re-baseline of the gold headline. (Shipped behaviour.)
3. **Rotation source (v1): fixed on-disk pool for now.** Network-free and reproducible; rotate subsets with `--rotate/--seed`, grow the pool opportunistically as new tables land on disk. Quarterly fresh-fetch (network + PII screen + snapshot register) is deferred, not adopted.

## What we don't know yet

This reading is against **one** binary (the current default) — it establishes the baseline, not a regression. The delta signal only exists once a *candidate* is scored against it. And the trio is three tables; the seam is consistent across the full 15-table pool (0.621), but the band's value grows as the pool rotates onto tables the model has never been near.
