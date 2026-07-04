# Membership sets

Closed code lists backing the taxonomy `membership:` directive (sibling to
`checksum:` — see `crates/finetype-core/src/membership.rs`). A type whose
values are drawn from a published, enumerable code list gets a set file here;
the model's `membership_substance_guard` re-checks a column's values against
the real list, because a value of the right *shape* but outside the set is not
that type (Precision Principle — `^[A-Z]{4}$` confirms every 4-letter stock
ticker; the ICAO list does not).

Format: one code per line, sorted, `#` lines and blank lines ignored.
Lookups are case-folded to UPPERCASE. Each file records its source, licence,
snapshot date, and extraction rule in its header.

## Regeneration

- `icao_airport_codes.txt` / `iata_airport_codes.txt` — from OurAirports
  (public domain): download `https://davidmegginson.github.io/ourairports-data/airports.csv`,
  then
  `duckdb -csv -noheader -c "SELECT DISTINCT icao_code FROM read_csv('airports.csv') WHERE icao_code ~ '^[A-Z]{4}$' ORDER BY icao_code"`
  (and the same for `iata_code` with `^[A-Z]{3}$`). Refresh opportunistically;
  the sets churn slowly (~tens of codes/year).

Enrolling a new set: add the file here, one entry in
`membership::resolve()`, and a one-line `membership:` directive in the type's
YAML — no new bespoke veto.
