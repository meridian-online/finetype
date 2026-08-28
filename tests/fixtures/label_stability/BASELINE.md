---
measured: 2026-08-28
binary: finetype 0.6.57 (released, embedded model)
draws: 60 windows of 100 values per fixture, seed 20260828
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
