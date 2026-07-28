# finetype 0.6.54

The release body for `v0.6.54`. The workflow creates the GitHub release with
`generate_release_notes: true`, which produces a commit and pull-request list; this is
the human text that goes above it, applied with `gh release edit v0.6.54 --notes-file
docs/release-notes-0.6.54.md`.

---

Four things change for anyone typing columns with finetype, and a larger set of changes
that only affect how this repository measures itself. The two are separated below,
because a release note that mixes them makes internal work look like a product benefit.

Every number here is re-derivable, and the note says how: from a committed script and the
committed file holding its output, from a release build you can make yourself, or from the
published `v0.6.53` assets. Nothing is quoted from a pull-request body.

## What you get

### Eight-digit numbers stop being read as confident dates

A column of financial figures — `grossProfit` at `71132000`, `sharesOutstanding` at
`25012600` — or of surrogate keys like an NBA `GAME_ID` or a forum `PostId` came back as a
compact date, at high confidence, **with a date-parsing transform attached**. That last
part is what made it expensive: a downstream job that follows the transform does not get a
wrong label, it gets a corrupted column.

Two validators were at fault, not one. Both the year-first (`YYYYMMDD`) and the day-first
(`DDMMYYYY`) compact-date leaves accepted any eight digits at all, so each confirmed every
eight-digit token at 100% and the confidence veto had nothing to push back with. **Both are
now closed** — each checks day, month and year ranges, which is what their month-precision
sibling had been doing all along.

**If you are upgrading from 0.6.53, this is a straight fix, and the defect you are on is
worse than it looks.** The day-first false positive is not new and it is not a side effect
of fixing the year-first one: the released 0.6.53 binary already types the reject values as
a day-first date at **0.9878** confidence with a `%d%m%Y` parse attached. After the
year-first fix alone it was 0.9064 — same label, same transform, slightly less confident.
With both leaves closed it is `integer_number` at 0.4866, vetoed. Three column families
carried this, not one.

That is measured on four sides, not inferred: the released 0.6.53 binary run outside the
checkout, the state before either fix, the state after the year-first fix, and this
release — compared on the full emitted record, each side built *and run* inside its own
label state. Table: `docs/compact-date-residual.tsv`. Reproduce:
`scripts/probe_compact_date_residual.sh`.

**What it costs, stated plainly.** Over **1,723 tables / 97,599 columns**
(`docs/compact-dmy-blast-radius.txt`, from `scripts/compact_dmy_blast_radius.sh`):

| | before | after |
|---|---:|---:|
| columns typed day-first compact date | 978 | **32** |
| columns typed year-first compact date | 230 | **162** |
| columns *newly* typed as a compact date | — | **0** |

The 946 columns that lost the day-first date type were financial figures and identifiers:
615 became `integer_number`, 187 `unknown`, 76 `numeric_code`, 51 `word`, 11 `increment`.

The second row is a real cost and it is not hidden. A validator's pass rate is an input to
the model, so tightening the day-first leaf also moves a feature on year-first columns: a
genuine `YYYYMMDD` value passes the day-first validator before this change and fails it
after, because its middle pair is a day-of-month and overflows the month window. **All 68
of the columns that moved are genuine dates** — every sampled value a valid `YYYYMMDD`,
headed `date` (65) and `game_date` (3). 67 of them now come back `unknown` and one comes
back as text.

So the trade is **946 confidently-wrong dates removed against 68 correct ones downgraded to
`unknown`**. We think that is the right way round — a confident picture that is wrong is
the failure this product exists to prevent, and `unknown` is honest rather than wrong — but
if you consume compact `YYYYMMDD` date columns, 68 in 230 is the number to plan against.

Genuine day-first dates *with separators* are untouched. `DD/MM/YYYY` and `DD.MM.YYYY`
columns keep their labels, format strings and transforms, byte for byte across the change
(`docs/compact-dmy-day-first-reality-check.tsv`).

Two limits worth knowing. Month-first compact dates (`MMDDYYYY`) are still shape-only and
still imprecise — that is the honest remainder, recorded in
`tests/fixtures/precise_audit.tsv`. And a genuine *compact* day-first column is typed
`integer_number` both before and after this change: the compact day-first leaf does not
recognise its own family. That is a separate recall defect, neither caused nor worsened
here.

### NAICS sector codes published as hyphenated ranges now validate

The US Census publishes three NAICS sectors as hyphenated pairs — **31-33** Manufacturing,
**44-45** Retail Trade, **48-49** Transportation and Warehousing — and they appear that way
verbatim in business reference data. finetype's NAICS pattern rejected all three, so anyone
validating against the schema finetype emits was getting false rejections on correct data
and had to hand-widen their own copy of the rule. The `pattern` in an emitted JSON Schema
for an `identity.industry.naics` column now reads:

```
^(11|2[1-3]|3[1-3]|42|4[4-5]|4[8-9]|5[1-6]|6[1-2]|7[1-2]|8[1-2]|92)[0-9]{0,4}$|^(31-33|44-45|48-49)$
```

Nothing else was loosened. It is an exhaustive literal alternation of the three real ranges,
not a general `(-[0-9]{2})?` suffix, so `11-99` is still rejected — there are exactly three,
and the pattern names all three and nothing else.

**What this does not do:** a column consisting *only* of hyphenated ranges is not
auto-detected as NAICS. Detection runs through a closed membership set, and that set
deliberately expands the three ranges into their individual two-digit codes, so `31-33` as a
literal string is not a member. This release fixes the contract you validate against, not
the detection of a column of range labels.

### Typing a multi-column table is about twice as fast

Measured end to end against the binary you have today — the published `v0.6.53` macOS arm64
release, sha256 `45d93cf9…` — over a batch of 38 multi-column files, five alternating
repeats, two independent sittings:

| sitting | `v0.6.53` median | `0.6.54` median | speedup (median) | speedup (mean) |
|---|---:|---:|---:|---:|
| 1 — `docs/bench-0.6.54-vs-0.6.53.tsv` | 3.6654 s | 1.7626 s | **2.08×** | 2.08× |
| 2 — `docs/bench-0.6.54-vs-0.6.53.sitting-2.tsv` | 3.6683 s | 1.7626 s | **2.08×** | 2.08× |

Reproduce it:

```sh
scripts/bench_profile_ab.sh --a <v0.6.53-binary> --b ./target/release/finetype \
    --files eval/bench/multicolumn-38.txt --repeats 5
```

Each file carries its own provenance header — both binaries' paths, versions and sha256 —
and its own summary block, so the figure above lives inside the artefact this note points
at rather than beside it.

The speedup depends on the shape of the work, and it is worth knowing which shape you have.
The gain comes from two places — the validation gate no longer builds error messages it
immediately discards, and columns are now typed in parallel — so a corpus of *single*-column
files gives the parallelism nothing to do and gains little. Wide tables are where it lands.

Neither optimisation changes an answer. What backs that here is the accuracy section
below — both gold fixtures re-measured on this tree at **+0 columns** — plus a smoke
assertion that compares the emitted `column,type` *pairing* across repeated runs, not just
the column-name sequence. The comparison above spans everything that landed between the two
releases, not any one change, which is what a user upgrading actually experiences.

### The binary is about 22.6% smaller

The value-branch encoder is a lookup table that finetype converts to full precision the
moment it loads, so storing it at full precision on disk was paying twice the bytes for
something inference never sees. It now ships at half precision. The embedded payload drops
by 15,118,336 bytes on every platform.

Held to a controlled A/B where the *only* thing that changes is the storage dtype — same
source, same machine, both endpoints reproduced from a cold rebuild — a macOS arm64 release
binary goes **67,388,816 → 52,148,240 bytes, −22.6%**. Full byte-level accounting of where
the extra 122,240 bytes go: `evidence/half-precision-value-encoder.md`.

End to end, which is the number you will actually see: the published `v0.6.53` macOS arm64
binary is **67,424,752** bytes and this release builds to **52,148,256** — **−22.7%**. The
two figures differ because the second pair also spans the source changes between releases.

For reference, the published `v0.6.53` macOS arm64 archive is 53,291,389 bytes. The 0.6.54
archive is built on the release runner and its size is on the release page rather than
predicted here.

This one carries a caveat worth stating plainly: **it is label-invariant on our gold
fixture, not output-invariant.** Every label and every quality band is identical. The
confidence *number* moves on 170 of 843 measured columns, by at most 0.0007. That is the
arithmetic consequence of storing fewer bits, not a regression — but if you compare
confidence values across this release expecting them to be identical, they will not be, and
we would rather you heard it here. Fixture `gold-2026-07-14`; full workings in
`evidence/half-precision-value-encoder.md`.

### Also

The built-in MCP server is now marked deprecated in favour of arcform's `arc mcp`. Nothing
is removed and nothing breaks — `finetype mcp` still runs, and every library type,
including the datapackage and JSON-schema emitters, is untouched and supported.

## Accuracy

Unchanged, and measured rather than assumed. Every figure in this section is a gold-fixture
accuracy and nothing else.

On gold fixture **`gold-2026-07-14`** the full pipeline scores **819/931 = 0.880**, exactly
the 0.6.53 result — **+0 columns**. On the older **`gold-2026-06-28`** fixture it scores
**805/931 = 0.865**, also **+0**. Both re-measured on this tree, under taxonomy
`tax-48864103893f`. `evidence/release-0.6.54.md` carries both numbers, the fixture content
hashes, and the comparisons this repo refuses to make.

Three limits on that, because a number without its limits is worth less than no number:

- **The typing fixes above move zero gold columns.** The gold fixture contains no
  eight-digit financial column typed as a compact date and no hyphenated NAICS sector
  column, so it cannot see either fix. Their evidence is the two-sided corpus pass, the
  four-sided probe and the validator boundary tests quoted earlier in these notes — not
  this score.
- **A gate verdict is not coverage.** The fast corpus-honest gate returned GO with zero
  triggers on the day-first change and **could not see it**: it computes the validator
  pass-rate vector in a cached stage shared by both arms, so a validator edit cannot reach
  either side of its comparison. That is a structural blindness, not a sampling limit, and
  it is the stronger caveat — stronger than the usual "a small, non-adversarial slice of
  GitTables". Recorded in `docs/compact-dmy-gate-coverage.md`. Nothing in this release
  rests on it.
- **931 of the fixture's 1037 rows are scored.** The corpus-pass FTMB behind the offline
  Sense stage does not cover the 106 columns added since 2026-06-22.

## Internal — no effect on what finetype outputs

- **`evidence/`**, a tracked manifest of gold fixture *versions*, content-addressed by
  hash, with accuracy bars recorded against a specific (model, fixture, binary) triple. The
  motivating failure is on the record: a bar written down as a bare `0.853` survived 37
  label re-adjudications and 106 added columns, and eventually rejected an unchanged model
  that had scored 25 columns above it. The bar had gone stale, not the model. Release
  reports are generated from the manifest and CI fails if the prose and the manifest
  disagree by a byte. A baseline now also records the taxonomy its measuring binary was
  built with, which is a different fact from the one its fixture was adjudicated under.
- **Instruments that are themselves tested.** The evidence suite is run against six wrong
  implementations of `evidence.py`, and the whole-record differ against eleven wrong
  implementations of itself — four of which passed when first written, which is the whole
  argument for running them rather than asserting they bite. Both gate CI.
- **Branch ablation on the shipped model** — one flag per branch of the five-branch model,
  so each branch's contribution is measurable on the model that actually ships.
- **~2,800 lines removed**: two classifiers with no construction site outside tests, one
  header-hint rule reachable only through a classifier no released binary contained, and a
  cross-column attention layer that only ever ran in a repo checkout — meaning every
  measurement taken from a checkout described a pipeline no user was running.
- **Compact-date instruments** — a four-sided residual probe, a two-sided corpus blast-radius
  pass, a year-policy study over 820,173 columns, and a mutation matrix in which every clause
  of the change is deleted in turn and must kill a named test. All committed with their
  output.
- **Tooling for the half-precision conversion**, including a whole-record differ. Two
  earlier attempts at this change were refused for citing a label-only comparison as
  evidence that output had not changed; it cannot show that, and now something else does.

## Install

```sh
brew install meridian-online/tap/finetype     # or: brew upgrade finetype
curl -fsSL https://install.meridian.online/finetype | bash
cargo install finetype-cli
```

`finetype profile` and `finetype validate` shell out to the `duckdb` CLI for CSV and
Parquet ingestion, so it must be on `PATH`.

If you fetch a release archive directly, use `curl` rather than a browser. macOS attaches a
quarantine flag to browser downloads, and an unsigned binary carrying that flag is killed on
launch with no error message — `curl`, `tar` and Homebrew formulae attach nothing.
