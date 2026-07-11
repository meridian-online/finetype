# Membership sets

Closed code lists backing the taxonomy `membership:` directive (sibling to
`checksum:` — see `crates/finetype-core/src/membership.rs`). A type whose
values are drawn from a published, enumerable code list gets a set file here;
the model's `membership_substance_guard` re-checks a column's values against
the real list, because a value of the right *shape* but outside the set is not
that type (Precision Principle — `^[A-Z]{4}$` confirms every 4-letter stock
ticker; the ICAO list does not).

Format: one code per line, sorted, `#` lines and blank lines ignored. Case
folding is per-set and chosen by the `membership.rs` predicate to match the
code's convention: the airport sets fold to UPPERCASE, `tld_codes` to
lowercase, `naics_codes` is digits (trim-only). Each file records its source,
licence, snapshot date, and extraction rule in its header.

## Regeneration

- `icao_airport_codes.txt` / `iata_airport_codes.txt` — from OurAirports
  (public domain): download `https://davidmegginson.github.io/ourairports-data/airports.csv`,
  then
  `duckdb -csv -noheader -c "SELECT DISTINCT icao_code FROM read_csv('airports.csv') WHERE icao_code ~ '^[A-Z]{4}$' ORDER BY icao_code"`
  (and the same for `iata_code` with `^[A-Z]{3}$`). Refresh opportunistically;
  the sets churn slowly (~tens of codes/year).
- `naics_codes.txt` — from the US Census Bureau NAICS 2022 2–6 digit code list
  (public domain): distinct codes matching `^[0-9]{2,6}$`, with the three
  sector ranges (31-33, 44-45, 48-49) expanded into individual 2-digit codes,
  sorted.
- `tld_codes.txt` — from IANA (public data): download
  `https://data.iana.org/TLD/tlds-alpha-by-domain.txt`, drop the leading
  `# Version` comment, lower-case, sort. Refresh opportunistically; new gTLD
  delegations land a few times a year.
- `unlocode_codes.txt` — from UNECE Recommendation 16 (public data), via the
  `datasets/un-locode` mirror
  (`https://raw.githubusercontent.com/datasets/un-locode/main/data/code-list.csv`):
  `duckdb -csv -noheader -c "SELECT DISTINCT upper(Country)||upper(Location) AS code FROM read_csv('code-list.csv', header=true, all_varchar=true) WHERE Country ~ '^[A-Za-z]{2}$' AND Location ~ '^[A-Za-z2-9]{3}$' ORDER BY code"`
  (5-char = 2-letter country + 3-char location, the type's own shape). All
  statuses are kept deliberately — the guard is demote-only, so an inclusive
  set never demotes a genuine locode column over a recently-retired code.
  ~116k codes; refresh opportunistically (UNECE publishes twice a year).
- `iso_3166_2_codes.txt` — ISO 3166-2 subdivision codes (`CC-SSS`), from the
  `pycountry` package (bundles the Debian iso-codes data, derived from the ISO
  3166 standard; the code list is factual/public):
  `uv run --with pycountry python -c "import pycountry; print(chr(10).join(sorted(s.code for s in pycountry.subdivisions)))"`.
  ~5046 codes / 200 countries; refresh opportunistically. Unlike the others this
  set backs a GUARD (`geo_subdivision_membership_promote`), not a leaf
  `membership:` directive — it promotes an ISO-3166-2 column to the generic
  `geography.location.region`, whose shape-only validator must not be narrowed.

Enrolling a new set: add the file here, one entry in
`membership::resolve()`, and a one-line `membership:` directive in the type's
YAML — no new bespoke veto.
