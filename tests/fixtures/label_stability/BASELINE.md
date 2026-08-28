---
measured: 2026-08-28
binary: finetype 0.6.57 (released, embedded model)
draws: 60 windows of 100 values per fixture, seed 20260828
delimiter: pinned with --delimiter ',' — see the note at the foot
---

# Label stability baseline

Each fixture is a pool of real values from one free-text column. The test draws independent
100-value windows from it and profiles each one. **A column that types reliably returns the same
label from every window.** These are the figures the shipped build returns today.

| fixture | agreement | modal label | status |
|---|---|---|---|
| `edgar_company_name` | 0.917 | `representation.text.entity_name` | unstable |
| `edgar_corpus` | 0.817 | `representation.text.plain_text` | unstable |
| `gleif_corpus` | 0.800 | `representation.text.plain_text` | unstable |
| `edgar_gleif_corpus` | 0.683 | `representation.text.plain_text` | unstable |
| `naics_corpus` | 1.000 | `unknown` | undecided |
| `naics_description` | 1.000 | `unknown` | undecided |

**Two different failures, and the second is why agreement alone is not the test.** Four columns
return the right label most of the time and a wrong one on the rest — a draw. The two `naics`
columns are perfectly stable and perfectly wrong: every window returns `unknown`, so an agreement
check scores them healthy. The test therefore asserts both that the modal label is what the
baseline records **and** that a column marked `undecided` is the only place `unknown` is allowed.

**Progress is the `undecided` count going to zero and the agreement column going to 1.000.** Update
this file when either moves, in the commit that moves it.

## Pin the delimiter when you reproduce these

**`profile` reads the two NAICS fixtures as eight columns unless `--delimiter ','` is given, and the
files are valid single-column CSV.** `read_csv_input` (`crates/finetype-cli/src/profile_io.rs:167`)
shells DuckDB with `auto_detect=true, all_varchar=true, null_padding=true`, and `null_padding` is
the option that does it — on DuckDB v1.5.5, against `naics_description.csv`:

| options | columns |
|---|---|
| `auto_detect=true` | 1 |
| `auto_detect=true, all_varchar=true` | 1 |
| `auto_detect=true, null_padding=true` | **8** |
| `auto_detect=true, all_varchar=true, null_padding=true` | **8** |

`sniff_csv` on the same file reports one column, delimiter `,`, quote `"`. Python's `csv` module
reads it as 460 rows of exactly one field. The intent recorded at `profile_io.rs:158` is that
`null_padding` pads *short* ragged rows — the analogue of the `csv` crate's `flexible(true)` — but it
also lets the sniffer widen the schema, which is the opposite. Prose containing semicolons is what
triggers it here; the first row that does is line 63 of `naics_description.csv`.

**The labels in the table above are unaffected** — the two NAICS fixtures return `unknown` at 1.000
and 0.9997 either way, measured both with and without the pin. But a figure that agrees by luck is
not a figure, so every run behind this file pins the delimiter.
