---
measured: 2026-08-28
binary: finetype 0.6.57 (released, embedded model)
draws: 60 windows of 100 values per fixture, seed 20260828
read_path: pre-#124 — see "These figures predate the sniff-first read"
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

## These figures predate the sniff-first read, and nothing has re-measured them

**Every number above was produced by `finetype 0.6.57` before #124 changed how a CSV is read**, and
this branch carries no code that would notice if they moved. Treat the table as a recorded
measurement with its instrument named, not as a baseline something defends — the test that consumes
these fixtures is still unwritten, and writing it is where the figures get re-derived against the
read path that ships.

The four non-NAICS fixtures were read as one column before #124 and are read as one column after it,
so their agreements are expected to survive; *expected* is doing real work in that sentence and no
run has replaced it.

## The delimiter pin this file used to require is gone, and #124 is why

**This section recorded a defect and said it was not fixed here. It is fixed on `main` now** — by
#124, which took `naics_description.csv` from this branch as its own regression fixture 16 hours
after the pin was written.

The defect was that `profile` read the two NAICS fixtures as eight columns unless `--delimiter ','`
was given, because DuckDB's `null_padding=true` lets the sniffer widen a schema rather than only
padding short ragged rows, and prose containing semicolons triggers it — first at line 63 of
`naics_description.csv`. #124 sniffs the shape first and then reads with the column list the sniff
pinned, so no pin is needed and none should be added back.

`tests/smoke.sh:348` now asserts the fixed behaviour on this fixture by name — *"single-column prose
CSV profiles as one column"*, expecting `Found 1 columns: ["description"]` — and that assertion runs
in CI. **If a future reader reaches for `--delimiter ','` here, that smoke assertion is the thing to
read first.**
