# Gold corpus — external-source fetch plan (ac-02, external share)

> **Executed 2026-06-10.** 10 sources fetched, PII-screened (employee/owner/permittee
> name columns stripped before vendoring), vendored under
> `eval/datasets/gold_external/` (~30 MB), snapshot-registered with role=gold, and
> extracted to `gold_corpus_candidates_external.tsv` — **123 columns across all 18
> strata** (vs the ~150–180 target; ~8% of the 1,511-column corpus vs the memo's ~30%
> aspiration). The binding constraint is column-yield per table (~12 useful columns
> each) — reaching 30% needs ~40 tables. Deviation surfaced to the author 2026-06-10;
> expanding the table pool is the recorded crank if the ac-07 audit shows a
> GitTables/external accuracy gap. Sources actually used differ slightly from the
> plan below: ONS/USAspending/Wikipedia were replaced by OurAirports (ISO country
> codes, full URLs), Chicago crimes (mdy dates, state-plane near-miss coords), SF
> businesses (US zips), and UK price paid (UK postcodes, GBP amounts, OGL-3.0).

**State:** the GitTables draw filled 1,148 of 1,160 net-new candidate slots
(`gold_corpus_candidates.tsv`; + 240 anchor seeds = 1,388 of 1,400). The only hard
shortfall is `technology.internet.top_level_domain` (12 columns — too few tld-ish
headers in the corpus). The external share below therefore serves three purposes, in
priority order:

1. **Close the tld shortfall** (12 columns).
2. **Real positives where GitTables labels are suspect** — the sizing memo flags
   utc-offset positives as ~absent in GitTables (the drawn `header_pos` bucket is
   timezone-adjacent and may adjudicate to near-zero true offsets); isbn/postal/amount
   gain locale diversity per the Precision Principle.
3. **Generalisation check** — columns from data nobody tuned against, so the
   instrument audit (ac-07) can compare GitTables-stratum vs external-stratum accuracy
   and detect corpus overfit.

Target ~150–180 external columns (≈12% of the corpus). This is below the memo's ~30%
aspiration — recorded deviation: GitTables filled every quota, so external is targeted
where it adds information, not volume. If the ac-07 audit shows a GitTables/external
accuracy gap, expanding the external share is the turn of the crank.

Budget note: external columns are additive (corpus grows to ~1,550). Estimated queue
impact +25–35 adjudications; if the queue breaches the 350 cap, Tier-2 quotas trim
first per the signed memo.

## Sources (fetch → vendor under eval/datasets/gold_external/ → register via scripts/dataset_register.py, role=gold)

| source | licence | strata served | notes |
|---|---|---|---|
| OpenFlights airports.dat (openflights.org) | ODbL 1.0 (attribution + share-alike) | utc offset (real ±HH offsets), latitude, longitude, city, country_code, tz names | the utc-positive workhorse; ~7,700 rows |
| USGS earthquake catalog CSV (earthquake.usgs.gov) | US public domain | latitude, longitude, decimal backbone, iso dates, plain_text | already familiar from the earthquake round-trip work — reuse its source registration if present |
| Seattle Public Library checkouts (data.seattle.gov) | public domain (city open data) | isbn, year, integer backbone, categorical (material type) | real circulation data, messy ISBNs |
| NYC OpenData — DOB permit issuance (data.cityofnewyork.us) | public domain (NYC open data) | postal_code, city, date formats, alphanumeric_id (permit numbers) | US zip+4 mess included |
| UK ONS postcode directory sample (ons.gov.uk) | OGL v3 (attribution) | postal_code (UK format), region, country_code | locale diversity for postal |
| USAspending award CSV extract (usaspending.gov) | US public domain | amount, year, alphanumeric_id (PIID), categorical (award type) | currency amounts at federal-data messiness |
| Majestic Million (majestic.com/reports/majestic-million) | CC-BY 3.0 | top_level_domain (closes the 12-col shortfall), url | tld column is literal TLDs |
| Wikipedia page-view sample or GH Pages dataset (dumps.wikimedia.org) | CC-BY-SA / public domain metadata | url, plain_text backbone | only if a url/text diversity gap remains after the above |

Rules (binding, from the signed memo + spec ac-02):
- Vendor every file locally; register a snapshot (per-file SHA256, size, fetched-date,
  licence) BEFORE any column is drawn; the fixture references the snapshot, never a URL.
- Every source enters `eval/datasets/sources.yaml` with `role=gold` (leakage firewall,
  ac-05).
- Realism pre-screen per choice 0055 applies — these are analyst tables, not reference
  vocabularies; GeoNames/CLDR stay lens-only (decisive-lens rule).
- Columns drawn from each file are recorded in the fixture with
  `source=<registry name>`, `licence`, `stratum`, same key discipline
  (`file_content_sha256`, `column_name`).
