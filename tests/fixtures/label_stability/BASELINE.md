---
measured: 2026-09-16
binary: finetype 0.6.59 (built from this tree)
build: 472f2faf
read_path: post-#124 sniff-first read, crates/finetype-cli/src/profile_io.rs
draws: 60
window: 100
seed: 20260828
---

# Label stability baseline

Each fixture is a pool of real values from one free-text column. `scripts/check_label_stability.py` draws independent 100-value windows from it and profiles each one. **A column that types reliably returns the same label from every window.** These are the figures the build named in the frontmatter returns.

The **agreement** is the modal label's share of the windows that read. The **windows read** is how many of the windows drawn came back as one column; it is not always all of them, and the section below says why. Neither figure is asserted — `scripts/check_label_stability.py --remeasure` rewrites both, and refuses to touch a modal label.

| fixture | agreement | windows read | modal label | status |
|---|---|---|---|---|
| `edgar_company_name` | 0.883 | 60/60 | `representation.text.entity_name` | unstable |
| `edgar_corpus` | 0.750 | 60/60 | `representation.text.plain_text` | unstable |
| `gleif_corpus` | 0.783 | 60/60 | `representation.text.plain_text` | unstable |
| `edgar_gleif_corpus` | 0.700 | 60/60 | `representation.text.plain_text` | unstable |
| `naics_corpus` | 0.983 | 60/60 | `unknown` | undecided |
| `naics_description` | 1.000 | 60/60 | `unknown` | undecided |

**Two different failures, and the second is why agreement alone is not the test.** Four columns return the right label most of the time and a wrong one on the rest — a draw. The two `naics` columns are perfectly stable and perfectly wrong: every window returns `unknown`, so an agreement check scores them healthy. The test therefore asserts both that the modal label is what the baseline records **and** that a column marked `undecided` is the only place `unknown` is allowed.

**Progress is the `undecided` count going to zero and the agreement column going to 1.000.** Re-measure with `scripts/check_label_stability.py --remeasure` in the commit that moves either, and say in that commit what moved them.

## What consumes these fixtures, and what it refuses

`scripts/check_label_stability.py` draws the windows the frontmatter describes, profiles them in one batch through `profile --files`, and refuses the tree on either of two rules:

- **the modal label moved** — a pool returns a label other than the one its row records, naming the pool, the recorded label and the observed one;
- **a pool went `unknown`** — `unknown` is the modal label of a pool this table does not mark `undecided`. `unknown` is written by the demotion guard and never by the model, so such a pool has lost its label rather than changed it.

It is registered in `.github/gate-self-tests.tsv` as `label_stability` and runs in the `Label stability baseline` job of `.github/workflows/ci.yml`. Its own self-test plants each of those failures and requires the gate to redden for it.

**The figures above were re-derived by that gate on the build the frontmatter names**, replacing the `finetype 0.6.57` measurement that predated #124's sniff-first read. Every modal label survived the change of read path; four of the six agreement figures moved, and `naics_corpus` left 1.000.

Every figure in the table then reproduced digit for digit on the x86_64 Linux runner under duckdb v1.5.3, against a measurement taken on arm64 macOS under v1.5.5. Both the seeded draw and the inference are reproducible across those two platforms; that is measured, not assumed, and it is why the seed is in the frontmatter.

## The one-window-in-sixty widening is fixed, and this card is why

**This section recorded a defect and said it was not fixed here. It is fixed on this branch now.**

`naics_description` used to read as three semicolon-delimited columns on one of its sixty windows — window 2 of seed 20260828, measured on duckdb v1.5.5. #124 fixed the whole file; this narrower defect survived it because `choose_sniff`, arbitrating between the strict and padded sniffs, confirmed a candidate against whichever row its own `SkipRows` landed on without requiring a row to follow it. On this window the strict sniff reported `;`, three columns and `SkipRows` of 100 on a 101-line file — landing on the file's own last row, which happens to split into three under `;` — and that lone row, nothing behind it, confirmed the sniff that skipped to it.

The fix is in `header_field_count` and `choose_sniff` in `crates/finetype-cli/src/profile_io.rs`: a sniff no longer confirms against a row with no row after it, and when neither candidate confirms, the fallback re-sniffs with `SkipRows` pinned to zero — the file's row that is actually first, read as it stands — rather than trust an unconfirmed guess. The window is persisted at `tests/fixtures/naics_description_window2.csv` (outside `label_stability/`, whose every CSV this gate requires a baseline row for) and asserted by name in `tests/smoke.sh`, section 10, so the defect stays under CI rather than only under an occasional draw.

## The delimiter pin this file used to require is gone, and #124 is why

**This section recorded a defect and said it was not fixed here. It is fixed on `main` now** — by
#124, which took `naics_description.csv` from this branch as its own regression fixture 16 hours
after the pin was written.

The defect was that `profile` read the two NAICS fixtures as eight columns unless `--delimiter ','` was given, because DuckDB's `null_padding=true` lets the sniffer widen a schema rather than only padding short ragged rows, and prose containing semicolons triggers it — first at line 63 of `naics_description.csv`. #124 sniffs the shape first and then reads with the column list the sniff pinned, so no pin is needed and none should be added back. The section above records what #124 left, and what closed it.

`tests/smoke.sh` now asserts the fixed behaviour on this fixture by name — *"single-column prose CSV profiles as one column"*, expecting `Found 1 columns: ["description"]` — and that assertion runs in CI. **If a future reader reaches for `--delimiter ','` here, that smoke assertion is the thing to read first**, and the section above is the thing to read second.
