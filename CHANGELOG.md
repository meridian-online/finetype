# Changelog

All notable changes to FineType will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.6.57] - 2026-08-13

### Changed

- **A published field's `constraints.pattern` is fitted to the column, not copied
  from the taxonomy leaf.** The pattern on a Data Package field was taken verbatim
  from the definition of whatever label the column was given, and that pattern
  describes the type in the abstract. A correctly-typed column carrying legitimate
  variants the canonical pattern rejects therefore shipped a descriptor that fails
  its own data, so a consumer running `frictionless validate` against a descriptor
  we published saw failures on rows that are correct. Two shapes in shipped data
  show it: a country-code column, correctly typed, mixing ISO 3166-1 alpha-2 with
  ISO 3166-2 subdivision codes, where `^[A-Z]{2}$` rejects every `US-DE` / `CA-ON`
  row; and a four-character legal-form code column whose canonical pattern demands
  at least one letter *and* at least one digit, where all-digit and all-letter
  codes are both legal under the standard and both fail it. The emitted constraint
  now describes the values observed. (#111)

- **Two Sharpen guards, both default ON, correct labels that had reached a
  published descriptor.** Neither adds a taxonomy leaf, a membership set or a
  retrain — the shipped model is unchanged.

  `entity_name_title_header_demotion` reads a `title`-tokened header. A
  classification label or a work/role title is neither single-token enough for
  `entity_name_vocab_veto` nor lowercase prose enough for `entity_prose_override`,
  so it shipped confidently as a named entity. The header token is the
  discriminator, matching `naics_industry_recovery`'s own naics-token gate.

  `geo_hyphenated_region_margin_promote` reads a legal-jurisdiction column mixing
  bare ISO 3166-1 country codes with hyphenated ISO 3166-2 subdivision codes.
  `geo_subdivision_membership_promote`'s purity bar fails on the country tail, and
  `geo_code_membership_vote`'s subdivision side reads the bare `state_code` locale
  enum, which a hyphenated `US-DE` never matches — so nothing reached the shape.
  The guard **promotes to `region` only**; demotion toward `country_code` is left
  to `geo_code_membership_vote`, which already does it safely, because a
  bidirectional version regressed bare Canadian province codes that collide with
  country codes. (#117)

- **The published samples and SQL expressions are gated against the tree they
  describe.** `samples:` is a publication surface — it reaches the type registry,
  the MCP resources and `finetype taxonomy` — and nothing had looked at it:
  `finetype check` validates the values the *generator* produces, not the values in
  `labels/`. A sample that failed its own leaf's `validation.pattern` left CI
  green. Two gates now close it. `validate-samples` applies each leaf's own
  compiled validator to its own samples through the product's code rather than a
  second implementation of it; and every published SQL expression is resolved
  against a hermetic DuckDB catalog with this tree's build loaded, with an
  allowlist of six SQL keywords each carrying its reason. The corrections that
  found reach the shipped taxonomy: cast targets and call heads DuckDB does not
  have, and check-digit samples that fail the shipped Rust implementation. (#102,
  #104)

- **`representation.identifier.numeric_code` no longer offers a NAICS-shaped
  sample.** `5112` matched `identity.industry.naics`'s validation pattern through
  its sector prefix while the same leaf's description defers industry
  classification codes to that other leaf. Replaced rather than deleted, so the
  list keeps its length: `06` is the FIPS state code whose state+county extension
  already sits below it, and its leading zero demonstrates why the leaf is a string
  rather than an integer. (#103)

### Removed

- **BREAKING: the six un-prefixed scalars are no longer registered.** `finetype`,
  `finetype_detail`, `finetype_cast`, `finetype_unpack`, `finetype_validate` and
  `finetype_version` were kept as aliases when 0.6.23 made `ft_` the taught
  surface, on a stated overlap of one release. `ft_` is now the whole surface,
  and a call to any of the six raises `Catalog Error`.

  **Four of the six are a rename. Two are not, and a mechanical find-and-replace
  on those two returns worse answers or a type error rather than failing.**

  | Removed call | Replacement | What changes |
  |---|---|---|
  | `finetype(col)` typing a column | `ft_profile(col)` | `finetype` pooled the DuckDB chunk as its sample, so it answered at *column* level. `ft_profile` is an **aggregate**: it returns `STRUCT("type" VARCHAR, confidence DOUBLE, duckdb_type VARCHAR)` for the column, not a VARCHAR per row. Take `.type` for the label, and expect one row where you had one per input row. |
  | `finetype(value)` probing one literal | `ft_infer(value)` | Same VARCHAR label, but `ft_infer` is a sample of one with no sibling context — strictly weaker than a column. Reach for it only when the input really is a single literal. |
  | `finetype(list(col))` / `finetype(list(col), header)` | `ft_profile(col)` / `ft_profile(col, header)` | The `list()` wrapper goes; the aggregate reads the column directly, and `GROUP BY` gives you the per-group form. Returns the STRUCT above. |
  | `finetype_validate(value, schema)` | `ft_validate_text(value, schema)` | **Not** `ft_validate`, which is a table macro over a whole table. And the return type changes: `finetype_validate` returned a VARCHAR — the bare string `'valid'`, or an error message — where `ft_validate_text` returns `STRUCT("valid" BOOLEAN, "constraint" VARCHAR, message VARCHAR)`. `WHERE finetype_validate(v, s) = 'valid'` becomes `WHERE ft_validate_text(v, s).valid`, and the failed constraint is now named in `.constraint` rather than buried in a message string. |
  | `finetype_detail(…)` | `ft_detail(…)` | Rename. Same impl, same overloads, same JSON VARCHAR. |
  | `finetype_cast(value)` | `ft_cast(value)` | Rename. Same impl. |
  | `finetype_unpack(json)` | `ft_unpack(json)` | Rename. Same impl. |
  | `finetype_version()` | `ft_version()` | Rename. Same impl. |

  The last four rows are the only safe find-and-replace in the set.

  The extension's registered surface is now 6 scalars, 1 aggregate and 2 table
  macros; `scripts/check_duckdb_catalog.py` compares that against
  `duckdb_functions()` of a loaded build on the `doc-surface` CI job, in both
  directions, so a doc that still teaches a removed name fails there.

- **`finetype_spike` is gone from the built artifact.** The trivial VTab that
  proved `vtab` is active under `loadable-extension` was registered in every
  production build, carrying its own comment saying it is not a production
  function — so it sat in every user's catalog and autocomplete. It now
  registers only under the non-default `spike` cargo feature. The evidence it
  exists for is a compile, and the `doc-surface` CI job still pays for it:
  `cargo check -p finetype_duckdb --features spike`.

### Fixed

- **The taxonomy reader refuses a line its own top-level patterns miss.**
  `LEAF_RE` and `PROP_RE` decide between them whether anything under a key is read
  at all, and a line either one missed was dropped: `_read_file` took the `PROP_RE`
  match and continued when there was none. A published key spelled a way the regex
  does not recognise therefore left the gate at exit 0 on content it never read —
  the fail-open direction, in the guard whose subject is the public type registry.
  Both are refusals now, raising through the same path the value-shape refusals
  used and naming the file and the line. (#106)

- **A private planning repo is no longer named from this public tree.**
  `CLAUDE.md`'s pillars preamble and the release skill's eval-baseline step each
  disclosed its existence, name and role. The pointer is dropped rather than
  redirected — there is no published page to cite in its place, and the four
  pillars are stated in full immediately beneath the sentence that pointed away
  from them. The release step keeps its instruction and loses the parenthetical,
  which was history rather than an instruction. (#107)

- **The release job proves the four assets the tap formula publishes before pushing
  it.** `update-homebrew` wrote four urls and four sha256 values into
  `Formula/finetype.rb` and pushed, having fetched only the `.sha256` sidecar beside
  each asset and never the asset — so the checksums were honest about the files the
  packaging produced and said nothing about whether the urls the heredoc composed
  resolved to them. Nothing downstream looked either; the tap has no pull-request
  CI. Two of the four are built on a runner of another architecture — per the
  matrix in `.github/workflows/release.yml`, `aarch64-unknown-linux-gnu` is
  cross-compiled on `ubuntu-latest` and `x86_64-apple-darwin` is built on
  `macos-latest`, which is arm64 — so a wrong url, or a checksum written beside
  the wrong one, produced a green release and surfaced on a stranger's machine at
  `brew install`. `scripts/check-formula-asset.sh` parses the formula and fetches
  what it names. (#108)

- **The tracked harness report carries a verdict of its own, and a dormant fixture
  row cannot hide behind a carve-out.** Routing both vci3 fixture tests through one
  panicking entry point closed the report-missing hatch at the line it was on but
  not the act: restoring the skip at both call sites, bypassing the entry point
  rather than editing inside it, left the suite green with the report gone (#109).
  Separately, `compare_to_fixture` walks the report, so a carve-out over a column
  the report does not attribute was never reached — the flag suppressed nothing,
  and a regression in that column would have been absorbed into a green run. A
  second sweep now runs over the fixture, with `expect_no_attribution` as its dual
  flag, so a column that leaves a dataset's failing set is locked in rather than
  merely tolerated. (#116)

- **Each gate's self-test is routed to the diffs that change that gate.** The
  self-tests in this repository ran on every pull request, so a README typo paid
  for the whole set and a diff that rewrote a gate paid the same — and the rule
  that a changed gate re-proves itself was held by a reviewer remembering it.
  `.github/gate-self-tests.tsv` names the commands that prove each gate and the
  paths that invalidate the proof; `.github/scripts/gate-self-tests.py` publishes
  one boolean per row and each self-test step is guarded by its own. An
  unconditional audit refuses a tree where the wiring has rotted — a guard naming
  an undeclared output, an unneeded job or a mistyped job name each skip silently
  and green, which is the failure this change exists to close. Every uncertainty
  routes *more* work: no base commit, an unfetchable base, a diff that will not run,
  or a change to the manifest, the router or the workflow selects the whole set and
  says so. (#112)

- **The comment reader's quote-parity rule is pinned in both directions.** The
  quote half of `comment_body` could be deleted outright and `--self-test` stayed
  green: the case whose label names that behaviour planted its apostrophe inside a
  balanced pair of double quotes, so the parity filter never decided anything about
  it. The shipped behaviour was correct on every input the fixtures construct; what
  was missing was a fixture that could tell. (#105)

## [0.6.56] - 2026-08-04

### Changed

- **`ft_profile` is a DuckDB aggregate, so it types a column directly.**
  `SELECT ft_profile(email) FROM people` is the call a reader reaches for first,
  and it did not bind: `ft_profile` was a scalar over an assembled `LIST`, so the
  column form was `ft_profile(list(email))` and the `ft_profile(tbl)` table macro
  existed partly to hide that. The reason on record was that duckdb-rs exposes no
  aggregate-UDF registration API — still true of the wrapper, never true of
  DuckDB, whose aggregate entrypoints sit in the pinned `libduckdb-sys` loadable
  bindings this extension already talks to directly. **The call shape changes:**
  `ft_profile(list(col))` becomes `ft_profile(col)`, and
  `ft_profile(list(col), header)` becomes `ft_profile(col, header)`. `GROUP BY`,
  `FILTER` and `DISTINCT` work on it. The `ft_profile(tbl)` table macro keeps its
  name and its output shape, and now groups the melted table by column name to
  pass that name as the header hint rather than pooling with `list()` and calling
  the scalar. Call shapes are tabulated in
  [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md), and the registered surface — name,
  kind and return type — is compared against `duckdb_functions()` of a built
  extension by `scripts/check_duckdb_catalog.py` on the `doc-surface` CI job, so
  a documented shape that the extension does not register fails there.

  **An aggregate-level `ORDER BY` inside the call is not supported.**
  `ft_profile(col ORDER BY col)` reads out of bounds inside DuckDB. The fault is
  in the C API's shared update path rather than in this aggregate: the sorted path
  leaves the state vector constant while `CAPIAggregateUpdate` flattens the input
  vectors without flattening the state, where its `combine` and `finalize`
  siblings flatten both. Reported upstream (duckdb/duckdb#21537, closed unfixed
  and deferred), the callback cannot detect it, and the workaround is to order the
  statement instead of the aggregate. (#97)

### Removed

- **The `ft_profile(LIST<VARCHAR>)` and `ft_profile(LIST<VARCHAR>, VARCHAR)`
  scalars.** An aggregate cannot be registered at a name a scalar already holds —
  the C API registers with `ALTER_ON_CONFLICT`, the catalog throws because the
  existing entry is a different kind, and the C API swallows the message and
  returns an error state — so the `LIST` scalars retired to free the name for the
  aggregate above. Rewrite `ft_profile(list(col))` as `ft_profile(col)`. (#97)

### Fixed

- **The release pre-flight's model-drift check can fail.**
  `.github/scripts/check-ci-model-drift.sh` had no non-zero exit path: it compared
  `FINETYPE_CI_MODEL` against the `models/default` target, printed a `::warning::`
  when they disagreed, and exited 0 — so `CI model vs symlink drift` reported the
  same green check as the jobs that can go red, and the binary-release pre-flight
  opens by reading exactly that signal. Drift is the condition under which every
  platform build of a release fails: `.github/scripts/download-model.sh` fetches
  into `models/<FINETYPE_CI_MODEL>` while `crates/finetype-cli/build.rs` resolves
  the `models/default` link target and panics unless that directory holds
  `label_map.json` and `config.json`. The check now exits non-zero on drift, with
  no promotion-PR exclusion, and a companion CI step plants drifts in scratch
  trees — the symlink form, the plain-text form git leaves on Windows, a target
  differing only by a path component, a `models/default` resolving to no name, and
  a whitespace pin — holding each to an exit status *and* a message, since a
  status alone cannot tell a refusal from a crash. `docs/RELEASE.md` no longer
  describes the check as non-blocking. (#98)

- **The comment-citation gate reads every file type it claims to cover.** It
  resolved citations in Rust comments only, so a dead path or a misattributed
  constant written in a workflow file, a shell script, a Python module or a
  manifest was never read — citations in `ci.yml`'s own comments named Python
  scripts absent from the tree. The file set and the comment matcher widen
  together, with a per-syntax comment marker rather than Rust's assumed
  everywhere: widening the file set alone reads more files and sees nothing in
  them, which is now a `--self-test` case that reddens when the matcher is left on
  Rust markers. Shell and YAML get their own handling so an unbalanced apostrophe
  in prose cannot swallow the rest of the line and hide a citation.

## [0.6.55] - 2026-08-02

### Fixed

- **`finance.securities.lei` accepts the alphanumeric LOU prefix ISO 17442
  defines.** The validation pattern required four digits at characters 1-4,
  where the standard puts a 4-character alphanumeric LOU (Local Operating Unit)
  prefix — so an LEI carrying a letter there failed validation while passing the
  ISO 7064 check digits named by the same leaf's `checksum:` directive, and a
  column of real LEIs could not be confirmed as LEIs. The pattern is now
  `^[A-Z0-9]{4}[A-Z0-9]{14}[0-9]{2}$`; the check digits stay `[0-9]{2}` and the
  length stays 20. Measured over `eval/datasets/gold_external/gleif_entities.csv`
  (200 rows): 171 matched before, 200 match now — both counts asserted in
  `crates/finetype-core/tests/precision_widenings.rs`, so a revert reddens a
  named test.

  **`finetype generate` moves with the pattern.** The synthetic LEI generator
  drew its LOU prefix from a digit-only list; it now draws from the full
  alphanumeric alphabet as well, so the shape the widened pattern accepts is
  exercised on every run. LEI output at a fixed `--seed` therefore differs from
  0.6.54. Two letter-prefixed values were added to the leaf's shipped
  `samples:`. (#92)

- **Documented SQL that failed on paste.** Fenced SQL in `README.md` and the
  `docs/` guides is now run against a locally built extension, which is what
  caught it — among the examples fixed, a `LOAD` naming a filename the build does
  not write (`Makefile:6` names `target/release/finetype.duckdb_extension`,
  which `build-extension` writes at `Makefile:146`), a "filter by detected
  locale" recipe in `docs/LOCALE_GUIDE.md` reading a `$.locale` key that is not
  in the JSON `finetype_detail` builds
  (`crates/finetype-duckdb/src/column_fn.rs:328` emits `type`, `confidence`,
  `duckdb_type`, `samples`, `votes`, and `disambiguation` when a rule fires),
  and a call to `parse_fr_month`, which the loaded catalog does not report.
  (#88)

- **Taxonomy sizes quoted in the documentation are checked against the shipped
  taxonomy.** The headline and the per-domain table are compared, row by row,
  against the seven `labels/definitions_*.yaml` files that `taxonomy.rs` embeds
  with `include_str!` — 251 semantic types across 7 domains — and the documented
  DuckDB registry is compared by name, kind and return type against
  `duckdb_functions()` of a built extension. Gated by
  `scripts/check_doc_taxonomy_counts.py`, `scripts/check_duckdb_catalog.py` and
  `scripts/check_sql_examples.py` on the `evidence` and `doc-surface` CI jobs;
  `make check-docs` runs them locally. (#88)

### Changed

- **The README teaches the `ft_` surface the extension registers, and the
  community `INSTALL` is documented again.** The DuckDB section led with the
  un-prefixed scalars, which `README.md:189-190` records as aliases superseded
  by the `ft_` verbs since 0.6.23; the examples now open on `ft_profile()` over
  a table and `ft_validate()` against a JSON Schema. The note that withdrew the
  community build is replaced by `INSTALL finetype FROM community;` and a table,
  measured 2026-07-30, of what that channel serves across four releases of the
  DuckDB 1.5 line — one of which has no published build, where `INSTALL` returns
  HTTP 404. A migration block names the swaps that are not renames:
  `finetype()` pools the DuckDB chunk as its sample, so its replacement is
  `ft_profile` rather than the single-value `ft_infer`; and `finetype_validate`
  maps to `ft_validate_text`, which returns
  `STRUCT("valid" BOOLEAN, "constraint" VARCHAR, message VARCHAR)` where the
  alias returned a `VARCHAR` — `WHERE finetype_validate(...) = 'valid'` becomes
  `WHERE ft_validate_text(...).valid`.

## [0.6.54] - 2026-07-28

### Fixed

- **Eight-digit numbers no longer type as confident dates — on either compact
  leaf.** Both `datetime.date.compact_ymd` (year-first) and
  `datetime.date.compact_dmy` (day-first) validated on `^\d{8}$`, so each
  confirmed *every* eight-digit token at 100% and the hard validation veto had
  nothing to push back with. Financial figures (`grossProfit` 71132000,
  `sharesOutstanding` 25012600) and surrogate keys (an NBA `GAME_ID` 21601092, a
  `PostId` 18502653) came back as confident dates **with a `strptime` transform
  attached** — a downstream consumer that follows the transform does not get a
  wrong label, it gets a corrupted column. Both validators now carry day, month
  and year windows, the shape their `compact_ym` sibling has always had. Not
  per-month day counts: `20240230` still validates, and the windows carry the
  discrimination the corpus needs. `compact_mdy` is the honest remainder — still
  `^\d{8}$`, still imprecise, and recorded as such in
  `tests/fixtures/precise_audit.tsv`.

  **The defect predates both fixes, and it was three column families, not one.**
  The day-first work was opened on the premise that tightening the year-first
  leaf had *relocated* the false positive — a low-confidence integer becoming a
  high-confidence date — and that premise is false. A four-sided probe settled
  it: the released 0.6.53 binary run outside the checkout, the parent of the
  year-first change, the year-first change, and both leaves closed. The released
  binary and the pre-tightening parent agree record for record, and the released
  binary is *more* confident than the state that was blamed:

  | probe fixture | released 0.6.53 | year-first fixed | both fixed (0.6.54) |
  |---|---|---|---|
  | `compact_dmy_ymd_reject_set` | `compact_dmy` 0.9878 high, `%d%m%Y` | `compact_dmy` 0.9064 high, `%d%m%Y` | `integer_number` 0.4866 low |
  | `compact_dmy_sequential_ids` | `compact_dmy` 0.8341 medium, `%d%m%Y` | `compact_dmy` 0.8110 medium, `%d%m%Y` | `increment` 0.4938 low |
  | `compact_dmy_round_hundred_share_counts` | `compact_dmy` 0.9999 high, `%d%m%Y` | `compact_dmy` 0.9996 high, `%d%m%Y` | `integer_number` 0.9847 high |
  | `compact_dmy_genuine_ymd_dates` (control) | `compact_ymd` 0.9992 high | `compact_ymd` 0.9992 high | `compact_ymd` 0.9972 high |
  | `compact_dmy_unconstrained_eight_digit` (control) | `integer_number` 0.6000 | `integer_number` 0.6000 | `integer_number` 0.6000 |

  Full emitted record for all four sides, six fixtures:
  `docs/compact-date-residual.tsv`, regenerate with
  `scripts/probe_compact_date_residual.sh`. Each side is built **and run** inside
  its own label state, because `profile` resolves `labels/` against the working
  directory and prefers it over its embedded copy — the first version of this
  probe ran every baseline side against the candidate taxonomy and reported the
  shape-only sides as already fixed. Locale is noise on this pipeline
  (`docs/compact-date-residual.tsv.locale-control`), so no locale claim is made.

- **The day-first fix costs 68 genuine `YYYYMMDD` columns their type, and that is
  in the ledger next to what it buys.** A two-sided profile pass over **1,723
  tables / 97,599 columns**, each side built and run inside its own label state
  (`docs/compact-dmy-blast-radius.txt`, `scripts/compact_dmy_blast_radius.sh`):

  | label | before | after |
  |---|---:|---:|
  | `datetime.date.compact_dmy` | 978 | **32** |
  | `datetime.date.compact_ymd` | 230 | **162** |
  | `datetime.date.compact_mdy` | 92 | 71 |
  | `datetime.date.compact_ym` | 353 | 353 |

  Of the 946 columns that left `compact_dmy`: 615 → `integer_number`, 187 →
  `unknown`, 76 → `numeric_code`, 51 → `word`, 11 → `increment`, 4 →
  `alphanumeric_id`, 2 → `compact_mdy` (946 total). **Zero columns were newly
  typed `compact_dmy`.** These 946 were not individually audited for
  genuineness the way the 68 `compact_ymd` losses were — the table records where
  they went, not what they were. The `compact_mdy` row (92 → 71) is likewise
  measured but unaudited: whether those 21 were false positives removed or
  genuine dates downgraded is not established here.

  The `compact_ymd` row is the cost, and it is collateral rather than intent: a
  validator's per-label pass rate is an **input** to the multi-branch model's
  validation branch, so tightening the day-first leaf moves a feature on every
  eight-digit column including year-first ones — a genuine `YYYYMMDD` value
  scores 1.0 on the day-first validator before the change and 0.0 after it,
  because its middle pair is a day-of-month and overflows the month window. All
  **68** of the columns that moved have **every sampled value a valid
  `YYYYMMDD`**; they are headed `date` (65) and `game_date` (3); 67 became
  `unknown` and 1 became `representation.text.word`. So: **946 confidently-wrong
  dates removed against 68 correct ones downgraded to `unknown`.** The trade is
  favourable on this product's stance — a confident picture that is wrong is the
  failure class, and `unknown` is honest rather than wrong — but it is a real
  precision loss and it is one row of the report so it cannot be quoted
  selectively.

- **Every clause of the day-first change is defended by a test that dies without
  it.** Six mutations, both suites run for each
  (`docs/compact-dmy-mutation-matrix.md`): the whole change reverted kills 8 of
  14; each of the three windows deleted on its own kills 6; the veto-safe
  allowlist entry deleted kills 3; and *adding* a century year window kills 6 —
  so the year staying `\d{4}` is an enforced decision rather than an omission.
  The three control rows stay green under the full revert, which is what makes
  them controls rather than more of the same assertion.

- **The day-first change touches no genuine non-US date column.** Four realistic
  fixtures — compact day-first with every day 13–31, compact day-first from
  1844–1880, `DD/MM/YYYY`, and `DD.MM.YYYY` under a German header — profiled on
  both sides and compared on the full emitted record, all four byte-identical
  (`docs/compact-dmy-day-first-reality-check.tsv`). The world writes day-first
  *with separators*, and those columns are served by `dmy_slash` and `dmy_dot`,
  which keep their labels, format strings and transforms untouched. An honest
  second finding this surfaced, which the change neither causes nor worsens: a
  genuine compact `DDMMYYYY` column where month-first is arithmetically
  impossible for every value is typed `integer_number` on **both** sides — the
  compact day-first leaf does not recognise its own family. That is a separate
  recall defect.

- **NAICS sector codes published as hyphenated ranges now validate.** The US
  Census publishes three NAICS sectors as hyphenated pairs — **31-33**
  Manufacturing, **44-45** Retail Trade, **48-49** Transportation and
  Warehousing — and they appear verbatim in business reference data. The
  canonical `identity.industry.naics` pattern admitted only sector-prefixed
  digit runs, so all three rejected and downstream contracts had to hand-widen
  their own copy of the pattern to compensate. Widened by literal alternation
  (`|^(31-33|44-45|48-49)$`), not by a general `(-[0-9]{2})?` suffix — that
  would admit `11-99`, which is not a published sector range. There are exactly
  three, and the pattern now names all three and nothing else (Precision
  Principle). `labels/sets/naics_codes.txt` is untouched: it deliberately
  expands the three ranges into individual 2-digit codes, so a column consisting
  only of hyphenated ranges is still not *detected* as NAICS. This fixes the
  contract you validate against, not the detection of a column of range labels.

- The stale "NAICS/SIC industry codes" clause is struck from
  `representation.identifier.numeric_code`'s description. Industry codes moved
  to `identity.industry.*` in the 2026-07-04 carve-out; `numeric_code` had been
  advertising a home NAICS no longer has, and now defers explicitly.

### Changed

- **Column typing is about twice as fast on multi-column tables.** Two
  independent fixes. The ≥90% validation gate that Sharpen and the deterministic
  fast path both consult was calling the error-**collecting** validator once per
  value and reading one boolean off the result — every failing value formatted a
  human-readable message that was dropped on the next line. The gate now
  resolves the leaf's validator once, calls the boolean-only path, and stops
  scanning as soon as the 0.9 bar is arithmetically out of reach. Then
  per-column classification runs in parallel — one column's answer depends on
  nothing but that column, and the process was leaving cores idle.

  Re-derived end to end against the **published `v0.6.53` macOS arm64 binary**
  (sha256 `45d93cf9…`), five alternating repeats over
  `eval/bench/multicolumn-38.txt`, two independent sittings:

  | sitting | v0.6.53 median | 0.6.54 median | speedup (median) |
  |---|---:|---:|---:|
  | 1 (`docs/bench-0.6.54-vs-0.6.53.tsv`) | 3.6654 s | 1.7626 s | **2.08×** |
  | 2 (`docs/bench-0.6.54-vs-0.6.53.sitting-2.tsv`) | 3.6683 s | 1.7626 s | **2.08×** |

  Reproduce with `scripts/bench_profile_ab.sh`, which now emits its provenance
  header (both binaries' paths, versions and sha256) and its own summary block
  into the same file, so the figure a note quotes is inside the artefact the note
  points at. Runs are alternated rather than blocked: a blocked layout hands all
  thermal drift to whichever binary ran second.

  The speedup is workload-shaped. The gain comes from dropped error formatting
  and from column parallelism, so a corpus of *single*-column files gives the
  parallelism nothing to do and gains essentially nothing; wide tables are where
  it lands. Both optimisations are answer-preserving and were diffed byte for
  byte. The A/B spans everything that landed between the two releases, not this
  change alone — which is what a user upgrading actually experiences.

  Column order is contract — consumers read the emitted sequence positionally —
  so every parallel collect is from an **indexed** iterator and the smoke suite
  asserts the emitted `column,type` *pairing* across repeated runs, not just the
  column-name sequence. The first version of that assertion compared names only,
  and a deliberately order-losing collect walked straight past it.

- **The value encoder is stored in half precision, and the release binary is
  ~22.6% smaller.** A Model2Vec encoder's `model.safetensors` is a lookup table,
  and `Model2VecResources::from_bytes` up-casts it to F32 before a single token
  is embedded — so F32 storage was packaging, costing twice the download and
  twice the `include_bytes!` payload for nothing inference can see. The header
  encoder in the same binary had always shipped F16; the dual-encoder value
  branch had not. The artifact goes **30,236,760 → 15,118,424** bytes (sha256
  `16709ebe…`, mode preserved). Held to a dtype-only A/B on one source state, the
  macOS arm64 release binary goes **67,388,816 → 52,148,240**, **−22.6%**, both
  endpoints reproduced from a cold rebuild; end to end, the published v0.6.53
  binary is 67,424,752 bytes and this release builds to 52,148,256, **−22.7%**.

  **This is label-invariant on the gold fixture, not output-invariant.** On
  fixture `gold-2026-07-14`, over the 843 of 1037 gold columns that resolve from
  the corpus parquet, through `finetype profile -o csv`: **0 / 843** on label,
  quality band, runner-up, disambiguation, broad type, format string and
  transform; **170 / 843** on `confidence`, maximum |Δ| **0.0007**; 171 / 843 on
  the whole record. That is the arithmetic consequence of storing fewer bits, not
  a regression — but it *is* a change to emitted output, and a note calling the
  output unchanged would overstate what was measured. The one `detected_locale`
  flip is excluded as noise: a same-binary, same-artifact repeat moves the same
  column by itself. Full workings, including the byte-level accounting of the
  binary delta: `evidence/half-precision-value-encoder.md`.

- The loader now states the storage dtypes it accepts instead of trusting
  `to_dtype`, which is a converter and not a validator: an integer tensor would
  previously have loaded as a matrix of plausible-looking garbage.
- **`finetype resharpen` emits the whole composed record**, not just the label:
  `id`, label, confidence, quality band, runner-up, disambiguation rule. A
  label-only diff cannot see a rule that keeps the answer and halves the
  confidence, and that is exactly the failure mode this verb is used to rule
  out. `scripts/compare_composed_records.py` diffs two such passes and reports
  per-field and per-transition counts. `detected_locale` is excluded: it is not
  yet run-to-run stable on a fixed binary.
- **The dev binary and the released binary now classify the same way.**
  `finetype profile` used to load `models/sibling-context/model.safetensors`
  whenever that directory happened to exist and run a 396,800-parameter
  cross-column attention layer over the headers before the multi-branch header
  branch. `build.rs` never embedded that artifact, so no released binary could
  do this — every number measured from a repo checkout described a pipeline no
  user runs. Inference no longer loads it. The artifact is still trained and is
  still loaded frozen by the multi-branch trainer, which is where it earns its
  keep.

### Added

_Evaluation and release tooling. None of this changes what `finetype` outputs;
it changes what a claim about that output has to survive before it is made._

- **`evidence/` — the tracked record behind every accuracy number this repo
  quotes**, and a manifest (`evidence/fixtures.json`) that enforces its own
  rules. Gold fixture *versions* are content-addressed by sha256, baselines are
  recorded per fixture version, and a comparison is only offered when the bar
  and the candidate were measured on the same ground truth. The motivating
  failure is on the record: a `0.853` bar written on 2026-06-28 survived 37
  label re-adjudications and 106 added columns, and by 2026-07-25 the *unchanged
  shipped model* scored 25 columns above it. The bar had gone stale, not the
  model. `scripts/evidence.py verify` runs in CI.
- **The release report is generated, never written.** `evidence/release-<v>.md`
  is rendered from the manifest, and `verify` re-renders every committed report
  and fails on a byte of drift — so a number in the prose that the manifest does
  not carry cannot survive a PR. The renderer refuses to subtract two scores
  measured on different fixture versions; it lists them as **refused
  comparisons** with both fixture ids instead.
- **A baseline now records the taxonomy its measuring binary was built with**
  (`record-baseline --taxonomy`), which is a different fact from the
  fixture-level taxonomy its labels were *adjudicated* under. The report states
  the stamp per score and names the scores that lack one, rather than making a
  blanket claim on behalf of all of them; a stamp inferred after the fact is a
  guess wearing a hash, so pre-manifest scores stay unstamped.
- **`scripts/test_evidence_mutations.sh`** — six realistic wrong
  implementations of `evidence.py`, each of which must redden a case aimed at
  it. Without it the evidence suite is known to pass and not known to detect.
  Runs in CI.
- **Branch ablation on the shipped model** — one `--zero-*` flag per branch of
  the 5-branch multi-branch model, so each branch's inference-time contribution
  is measurable on the model that actually ships
  (`docs/branch-ablation-m2v8m-s43.md`).
- **`scripts/encoder_dtype.py`** converts a Model2Vec encoder's lookup table
  F32 → F16 for packaging. It refuses (exit 1, nothing written) rather than
  storing a value F16 cannot hold as `inf`, reports elements that round to zero,
  is a byte no-op on an already-converted file, and preserves the file's mode.
  Rounding is IEEE round-half-to-even out of CPython's `struct`, so the same
  input produces the same bytes on a laptop and on a CI runner — these bytes get
  compiled into a binary, and a per-platform difference there is a byte-drift
  failure waiting to happen.
- **`scripts/encoder_dtype_record_diff.py`** drives `finetype profile -o csv`
  once per gold column and diffs the **whole emitted record** — label,
  confidence, quality band, runner-up, disambiguation, locale, format string,
  transform. Two matching label-only prediction files establish label invariance
  and nothing more, which is the finding that got two earlier pull requests
  refused. Its 48 self-test cases and 11 mutation cases run in CI.
- **`scripts/bench_profile_ab.sh`** — alternated wall-clock A/B of two binaries
  over one file list, emitting provenance and summary into one artefact, so a
  quoted speedup outlives the pull request that produced it.
- **Compact-date instruments**: `scripts/probe_compact_date_residual.sh` (four
  sides, whole emitted record, each side built *and run* in its own label
  state), `scripts/compact_dmy_blast_radius.sh` (the genuine two-sided corpus
  pass, `REPORT_ONLY=1` to re-derive the analysis without re-running it),
  `scripts/compact_dmy_corpus_family.py` (year-policy scoring over 820,173
  profiled columns) and `scripts/compact_dmy_mutation_matrix.sh`.
- **`docs/compact-dmy-gate-coverage.md`** records that the fast corpus-honest
  gate returned GO with zero triggers on the day-first change **and could not
  see it**: the gate is a sharpen-rule instrument that computes the validator
  pass-rate vector in the cached, shared Sense stage, so a validator edit cannot
  propagate to either side of its comparison. A gate verdict is not coverage,
  and here it is not even evidence.

### Deprecated

- **The `finetype-mcp` server role**, in favour of arcform's `arc mcp`
  entrypoint. `FineTypeServer::new` and `serve_stdio` carry `#[deprecated]`
  notes; `finetype mcp` still works. Non-breaking: the crate and all of its
  library types are retained — the datapackage / JSON-schema emitters, taxonomy
  resources and tool request/response types are untouched and remain supported.

### Removed

- **Two unreachable classifiers and a header-hint classifier no user could run
  (−2,777 lines).** `SemanticHintClassifier` (`semantic.rs`) had no construction
  site outside `#[cfg(test)]` code — `ColumnClassifier::set_semantic_hint` had no
  caller at all — so the `semantic_hint` field was `None` in every binary ever
  shipped and the Model2Vec header matching it advertised never ran. `Classifier`
  and `CharClassifier` in `inference.rs` had no construction site anywhere,
  tests included, taking their `post_process` / `pattern_validate` tails and
  `extract_validation_patterns` with them; multi-branch is the only inference
  path (choice 0107 removed its callers, not the code). `InferenceError`,
  `ClassificationResult` and the `ValueClassifier` trait stay — `ColumnClassifier`
  still holds a `Box<dyn ValueClassifier>`. `build.rs` and the CI model download
  no longer fetch `type_embeddings.safetensors` / `label_index.json`: they were
  the semantic classifier's artifacts and were never `include_bytes!`-embedded.
- **One header-hint family, and it is unreachable rather than unused.**
  `header_hint_fallback` — confidence under 0.3 with the hint absent from the
  votes, promoting the hinted type at 0.4 — closed the header-hint else-if
  chain. Reaching it required the arm above it to decline, and that arm fires
  for every hardcoded hint not in the votes; with the Model2Vec hint source gone
  (see above), every hint that reaches the chain IS hardcoded. So the fallback
  needed the hint to be simultaneously present and absent from the votes. It was
  live only through the semantic classifier no released binary contained. Its
  removal moves nothing on the 837,625-column stratified sample: not the label,
  the confidence, the quality band, the runner-up, or the rule.

  Four other families on the same measured-dead list — `header_hint_measurement`,
  `header_hint_sci_measurement`, `header_hint_geo_override` and
  `header_hint_person_override` — are **kept**, all four on evidence. They are
  reachable, and removing any of them changes what `finetype profile` prints:

  | family | corpus columns moved | reproduction |
  |---|---|---|
  | `header_hint_geo_override` | 294 (rule) | a `city` column at 0.80 becomes 0.50, band `medium` -> `low`, runner-up surfaced |
  | `header_hint_person_override` | 133 (label + rule) | `authors`, `LastName`, `Surname`, `billing_lastname` columns revert to city/region/word |
  | `header_hint_measurement` | 0 | a `height` column at 0.90 becomes 0.50, band `high` -> `low`, runner-up surfaced |
  | `header_hint_sci_measurement` | 0 | a `temperature` column at 0.80 becomes 0.90 or 0.60, band moves either way |

  The two that move zero corpus columns are not therefore inert — the stratified
  sample simply contains no height/weight column, and `header_hint_coord_veto`
  (added later) shadows the scientific-measurement arm whenever a coordinate
  column's values are numeric, which is nearly always. Zero hits on a corpus
  measures the corpus, not the rule.

### Changed

- **`finetype resharpen` emits the whole composed record**, not just the label:
  `id`, label, confidence, quality band, runner-up, disambiguation rule. A
  label-only diff cannot see a rule that keeps the answer and halves the
  confidence, and that is exactly the failure mode this verb is used to rule
  out. `scripts/compare_composed_records.py` diffs two such passes and reports
  per-field and per-transition counts. `detected_locale` is excluded: it is not
  yet run-to-run stable on a fixed binary.


- **The dev binary and the released binary now classify the same way.**
  `finetype profile` used to load `models/sibling-context/model.safetensors`
  whenever that directory happened to exist and run a 396,800-parameter
  cross-column attention layer over the headers before the multi-branch header
  branch. `build.rs` never embedded that artifact, so no released binary could
  do this — every number measured from a repo checkout described a pipeline no
  user runs. Inference no longer loads it. The artifact is still trained and is
  still loaded frozen by the multi-branch trainer, which is where it earns its
  keep.

## [0.6.53] - 2026-07-18

### Fixed

- Company-name / legal-name columns no longer mistype as
  `representation.file.excel_format` (new `excel_format_prose_demotion` value
  rule; fixes Spanish/dotted org-name distributions such as
  `A.E.R.C.O. S.A.` / `… SLU` / `… SICAV`). The excel_format taxonomy pattern
  ends in `\w`, so real names pass it and no schema-fail rule could trip; the
  new rule strips quoted `"..."` literals and `[...]` sections, flags any
  remaining alphabetic char outside the Excel bare-token alphabet
  `{a,d,e,g,h,m,p,s,y}`, and demotes to `representation.text.entity_name`
  when >50% of values are non-format AND the median whitespace-token count is
  ≥2 (single-token id columns are spared). Pre-existing since ≤0.6.50.
- Low-cardinality single-token controlled-vocabulary enum columns no longer
  mistype as `representation.text.entity_name` (new `entity_name_vocab_veto`
  guard demotes them to `representation.text.word`, the residual sink for
  small enums). `entity_name` has no validator and sits outside the attractor
  lists, so a 5-value enum column asserted at low confidence previously
  shipped uncorrected; the ≥90%-single-token gate spares genuine multi-word
  entity-name columns and the `org_suffix_ratio < 0.1` gate spares org-name
  columns.
- Short uppercase ticker columns no longer degrade to
  `representation.scientific.protein_sequence`. Two complementary fixes: a
  new `protein_sequence_length_veto` guard (demote-only, no header gate)
  demotes a protein_sequence assertion to `unknown` when ≥90% of values are
  ≤8 chars — tickers are ≤7 chars, real proteins ≥10, so length separates
  with margin where the letter set cannot (56% of tickers are
  all-amino-acid); and the `finance.securities.ticker` recovery membership
  set is widened (SEC + Nasdaq Trader SymbolDirectory), lifting real
  ticker-headed columns over the guard's ≥90% membership bar. The veto runs
  immediately before `ticker_membership_recovery`, so a headered member
  column still promotes to ticker while headerless/residual columns stay
  honestly `unknown`.

### Data

- `labels/sets/us_tickers.txt` refreshed + widened, 9,304 → 16,348 symbols
  (SEC `company_tickers.json` refresh UNION Nasdaq Trader
  `nasdaqlisted.txt` + `otherlisted.txt`; test issues and footers dropped).
  Covers the ETF / warrant / unit / right / preferred classes the SEC
  company map omits. OTC/ADR `-F` and delisted symbols have no free
  authoritative bulk list and are deliberately not chased — the length veto
  covers that residual tail.

## [0.6.52] - 2026-07-17

### Changed

- **The per-type `glyph` field is renamed to `icon` and now holds a
  library-agnostic icon *name* instead of an emoji.** The previous value was a
  raw emoji (e.g. 🏦, 📅); it is now a neutral kebab-case name (e.g.
  `building-bank`, `calendar`, `world`) that a host UI maps onto its own icon
  set (Tabler, Lucide, …). All 251 leaf types carry an `icon`; the 45 distinct
  names were authored from the prior emoji vocabulary. The field surfaces in
  `taxonomy --full --output json` and on the MCP type-detail resource under the
  key `icon` (previously `glyph`). This is a breaking rename of the output key,
  landed one release after `glyph` first shipped and before any consumer other
  than the website depends on it.

## [0.6.51] - 2026-07-16

### Added

- **Every type now carries a `glyph` and, where one exists, a `canonical` spec URL.**
  Two new per-type metadata fields land on the taxonomy: `glyph` (a short, neutral
  emoji or ascii marker for visual identification, authored consistently per category)
  is set on all 251 leaf types, and `canonical` (the single authoritative
  specification for the type — e.g. ISO 8601 for an ISO timestamp, RFC 5322 for an
  email, ISO 4217 for a currency code, ISO 6346 for a container number) is set on the
  82 types with a stable standards-body spec. `canonical` is orthogonal to the existing
  `references` list, which stays as supplementary `{title, link}[]`. Both fields surface
  in `taxonomy --full --output json` and on the MCP type-detail resource, feeding the
  website type registry.

### Changed

- **`taxonomy` plain-text output is cleaner.** The `(priority: N, {designation})` suffix
  is dropped from the human-readable `taxonomy` listing — a type now prints simply as
  `key → broad_type`. The `--priority` / `-d` / `-c` filter flags and every machine
  output (`--output json` / `csv` / `--full`) are unchanged: `release_priority` and
  `designation` remain load-bearing for training-data generation.
- **The internal `resharpen` diagnostic verb is hidden from `--help`.** It stays fully
  invokable for internal gate scripts; it is just no longer advertised in the CLI help.

## [0.6.50] - 2026-07-14

### Added

- **A bare filename is now its own type.** A file name with a real extension
  (`report_final.xlsx`, `IMG_0042.png`, `atarisy2.cpp`) used to scatter across "plain text",
  "an entity name", "an identifier", even "a crypto wallet". It now types as
  `technology.filesystem.filename` — a sibling of the file-*path* type, placed in the
  filesystem family rather than with file *properties* (extension, size). The shape `word.word`
  is not precise on its own, so the substance is a curated set of real extensions plus a
  letter-bearing stem; a bare ccTLD domain (`gov.md`) is deliberately left as a hostname.
  Recovered deterministically at profile time; the 244-dim model does not predict the new leaf,
  and no retrain was needed. (reservoir-mining sweep)

### Fixed

- **Software versions are no longer thrown away.** A column of `1.6.1` / `1.11.23` under a
  `version` / `ver` / `build` header — including glued camelCase headers like `psychopyVersion`
  or `AffectsVersions` — now types as `technology.development.version` where it used to be
  discarded as unknown. The header is load-bearing because `1.2.3` is shape-identical to a
  `DD.MM.YY` date and a `YYYY.MM.PATCH` calendar version, so a value-only rule would over-fire;
  a four-digit-year veto keeps dates and calver out. (reservoir-mining sweep)

- **Delimited lists are recognised as lists.** A column of `Biography|Comedy|Drama`,
  `subjects: nanoparticles;polymers`, or `[20000, 10000, 15000]` now types as the matching
  `container.array.*` type instead of "plain text" or "an entity name". Only the self-precise
  delimiters are recovered — brackets that disambiguate a comma, plus bare pipe and semicolon —
  because a bare `City, Region` comma is structurally identical whether it separates a list or
  sits inside one place name, so bare-comma columns are deliberately left alone. (reservoir-mining sweep)

- **URLs are recovered from confident mislabels.** A URL column the model had filed as an IPv6
  address, an XML blob, an ARN, or an entity name now types as a URL — the URL validator
  (a scheme and a dotted host) is the check, so a genuine member of those types is never touched.

- **Dotted code namespaces are recognised.** `.NET` / `Java` namespaces
  (`ICSharpCode.NRefactory6`, `Abot2.Tests.Integration`) now type as `technology.code.qualified_name`
  instead of plain text, an entity name, or a hostname. Real hosts (`www.breitbart.com`) are
  spared by a stricter check on the confident-mislabel side.

- **Numeric identifier columns split by what they are.** A sequential row identifier now types as
  an auto-increment, while an opaque numeric code types as a numeric code — resolving a boundary
  the gold and representative fixtures had disagreed on, and improving both.

- **Legal-form (ELF) codes no longer read as postal codes.** A column of entity legal-form codes
  under a `legal_form` / `elf` header that the model reached for a postal code is demoted off
  postal.

## [0.6.49] - 2026-07-13

### Added

- **Stock tickers are now a first-class type instead of a misread US state.** A column of
  listed symbols (`AAPL`, `MSFT`, `BRK-B`) under a `ticker`/`symbol` header now types as
  `finance.securities.ticker`, backed by the SEC's 9,304-symbol list — where it used to be
  guessed as `geography.location.state_code` (the model reaches for a place when it sees a
  short uppercase code). A ticker has no check digit and its shape confirms any short
  uppercase token, so membership in the published list is the substance; the header gate is
  load-bearing because 15 of the 50 US state codes (MA, TX, …) are themselves real tickers.
  Recovered deterministically at profile time (the 244-dim model does not predict the new
  leaf; no retrain). US-listed for now — other venues compose additively behind the same
  type. The set refreshes via a scheduled `scripts/fetch_us_tickers.py` download into the
  checked-in list, so the build stays offline. (company-reference external band)

- **The enum value-domain (`x-finetype-enum`) now surfaces by default** on the json-schema
  and MCP profile surfaces — a bounded column (a `status`/`level` controlled vocabulary)
  shows its members without `--stats`, matching the CLI's default JSON and the datapackage
  output. The heavier observed-data constraints (cardinality, null-rate, min/max, the closed
  `enum` keyword) stay under `--stats`. (choice 0102)

### Fixed

- **Company names no longer masquerade as places.** A column of organisation names
  (`Apple Inc`, `NIGERIAN BREWERIES PLC`, `Oakmark International Fund`) that the model
  reached for a city/region/country now types as `representation.text.entity_name`. The
  tell is self-precise — a company name carries an org/fund suffix (`… PLC`, `… Fund`,
  `… LP`) and a place name never does — so the correction needs no header hint. Scoped to
  the place-*name* leaves only: a street address is legitimately multi-word free text whose
  tokens collide with company forms (`4th Street SE`, `Royal Trust Tower`, `Bairro Asa`), so
  address columns are deliberately left untouched. Deterministic, no retrain.
  (company-reference external band, seam 1c)

- **Top-level-domain columns are recovered from a geography misread.** A `TLD`/`IDN_TLD`
  column of `com`/`org`/`uk` that the model guessed as a place (the short lowercase tokens
  look like country codes) now types as `technology.internet.top_level_domain` when the
  header names a TLD column and ≥90% of values are IANA-delegated. The header gate is
  load-bearing — a pure-ccTLD column is value-identical to a country-code column.
  Deterministic, no retrain. (company-reference external band)

- **`region` stops being a dumping ground for short catalog words.** A column of seismic
  network codes (`us`/`ak`/`ci`), event types (`earthquake`), business categories
  (`GENERAL`/`FUND`), or product tiers (`Horizon`/`OverDrive`) that the model reached
  `geography.location.region` for now types as text when fewer than half its distinct
  values are real places. "Real place" is checked against a 42k GeoNames gazetteer
  (states/provinces/regions + countries + cities≥15k), ISO-3166-2 codes, bare state codes,
  and `City, State` composites — so genuine city/county/region columns (`Austin`, `Durham
  County, NC`, `US-TX`) are left untouched. Deterministic, no retrain. (external band,
  tier-3 geography seam)

## [0.6.48] - 2026-07-12

### Added

- **Eight model-blind certainty types now round-trip to themselves instead of a lookalike.**
  A column of CUSIPs, SEDOLs, DEA numbers, IMEIs, CPT codes, HS codes, UN/LOCODEs, or
  `rgb(...)` colours used to come back as the value-identical attractor the 244-dim model
  falls back to (a bare `word`, a numeric id) because none of these leaves is in the model's
  label space. Eight deterministic recovery guards restore them at `profile` time, each keyed
  on the scheme's own **check digit or membership set** — not shape — so a lookalike cannot
  trip them:
  - `cusip` / `sedol` / `dea_number` — value-only check-digit recovery (≥3 distinct passing
    values, so a constant column that coincidentally clears the checksum cannot assert the type).
  - `imei` — header-gated Luhn (a 15-digit Amex card is Luhn-valid by construction, so the
    `imei` header is the discriminator).
  - `cpt` — header-gated on a distinctive `cpt`/`procedure` token (a 5-digit CPT is
    value-identical with a ZIP; no checksum backstop, so the header is the sole gate).
  - `hs_code` — header-gated + median-length ≥ 6 (kills 4-digit years).
  - `unlocode` — value-only membership in the published UN/LOCODE set, **≥3 distinct passing
    values** (the 110k-entry set makes 5-char collisions common, so a constant `symbol`/`city`
    column that matches one entry by coincidence is rejected).
  - `color_rgb` — anchored `rgb(`/`rgba(` prefix (a bare comma-triple stays ambiguous by design).
- **`infer` on a single value now resolves 31 value-determinable certainty types** — e.g.
  `infer -i "❤️"` → `emoji`, or a UUID / JWT / WKT / IBAN / InChI → its own type, instead of the
  model's guess. `infer` previously skipped the value-recovery layer `profile` runs; the
  deterministic fast-path is that layer, its leaf set is now 31 (was 6), gated to
  exactly-one-match so it abstains on ambiguity.

### Fixed

- **Three real label bugs surfaced by the taxonomy examples round-trip test.** An orphaned
  `identity.credential.password` label key now maps to the real `identity.person.password`;
  bare `street_name` / `street_suffix` values are no longer swallowed into `street_address`;
  and a valid ISIN mislabelled `isrc` (they share a 12-char shape) is corrected by an ISIN
  check-digit guard the shape validator cannot see.

### Testing

- **Taxonomy examples round-trip test** (`make test-examples`) — builds one pure column per
  taxonomy type from its `examples` array and asserts each round-trips to its own label
  through `profile`; regression-gated (baseline 241/249).

## [0.6.47] - 2026-07-12

### Added

- **ISO-3166-2 subdivision columns now resolve to `region`** (`geo_subdivision_membership_promote`
  guard + a 5046-code roster). A column of hyphenated subdivision codes (`US-PA`, `GB-ENG`,
  `JP-30`) is a `geography.location.region`, but the flat softmax — which never learned the
  hyphenated form — filed it under a residual (`alphanumeric_id`) or a lookalike (a date format,
  `last_name`), and the unlocode membership guard demoted its `unlocode` guess to `unknown`
  (external band: ourairports `iso_region` US-PA → unknown). The `CC-SSS` shape is not precise —
  product/OS/locale hyphen-codes share it — so the guard keys on published ISO-3166-2 membership
  (`labels/sets/iso_3166_2_codes.txt`, 5046 codes across 200 countries, from the iso-codes
  project): a column ≥90% real subdivision codes is promoted to region. Value-based (decision
  0048), promote-only, no retrain. Corpus-honest gate GO (7 columns promoted — JP/BD/VN
  subdivisions, all verified genuine; zero bands). Gold +2.

### Fixed

- **Gold: `gleif region` re-adjudicated `state_code` → `region`** (author-ratified). Its values
  are 100% hyphenated ISO-3166-2 (`US-MA`/`US-DE`/`CZ-10`) — the general subdivision type, which
  fails `state_code`'s bare-code validator; consistent with the identical-shape ourairports
  `iso_region` gold. Surfaced by the new promote guard (the external band's gold-audit role).

## [0.6.46] - 2026-07-11

### Added

- **Minute-precision timestamps no longer come back `unknown`** (`sql_minutes` +
  `iso_minutes` leaves). A column of `2021-11-05 00:00` or `2019-08-05T16:27` — a
  space- or `T`-separated timestamp to the minute, no seconds — is a first-class
  DuckDB `TIMESTAMP`, but the model guessed a zoned/seconds sub-leaf (`rfc_3339`,
  `iso_8601`) whose validator the veto then hard-rejected to `unknown`. The
  taxonomy carried `dmy_hm` (slash/day-first minute) but not the year-first
  ISO/SQL-standard forms. Added `datetime.timestamp.sql_minutes` (`%Y-%m-%d %H:%M`)
  and `datetime.timestamp.iso_minutes` (`%Y-%m-%dT%H:%M`), recovered
  deterministically by `datetime_format_refinement` — no retrain (value-determinable
  format, decision 0096). Add-not-broaden: `sql_standard`/`iso_seconds` keep their
  seconds-required patterns, so second-precision timestamps still resolve to their
  own leaves. Ship gates: corpus-honest gate GO (20 columns corrected from
  `iso_8601`/`iso_seconds`/`unknown` → the correct minute leaf, zero bands fired),
  gold flat (no gold column is minute-precision), external band flat.

## [0.6.45] - 2026-07-10

### Fixed

- **`password` no longer over-emitted on free text** (whitespace anti-guard). The
  `identity.person.password` validator is `minLength: 1, maxLength: 255` — it certifies nothing, so
  the model scattered `password` onto i18n/UI string tables, song/anime/movie titles, artist names,
  and prose at corpus scale. A password has no positive substance to test (it is a high-entropy
  secret — the *absence* of structure), so `password_substance_guard` keys on the one self-precise
  anti-signal: a credential never contains internal whitespace. It demotes a `password`-labelled
  column to `unknown` when fewer than half its values are whitespace-free. Value-based, demote-only;
  deliberately partial (the whitespace-free residual stays, a harmless keep — genuine password
  columns are PII and essentially absent from the corpus). Per-column trace: all demotes are genuine
  non-password text, zero false demotes; corpus-honest gate GO; gold / representative flat.

## [0.6.44] - 2026-07-10

### Fixed

- **`locale_code` no longer over-emitted on any 2–3 letter word** (substance guard). A locale code
  leads with a real ISO 639 language and any script/region subtag is a real ISO 15924 / ISO 3166-1
  code; the taxonomy pattern `^[a-zA-Z]{2,3}(?:[-_][a-zA-Z]{2,4})*$` accepted any short word, so the
  model labelled survey fragments, dialogue-act tags, and country-code columns as locale codes. The new
  `locale_code_substance_guard` demotes a `locale_code`-labelled column to `unknown` when fewer than half
  its values pass `finetype-core::structure::is_locale_code` (four embedded closed sets — ISO 639-1 /
  639-2-3 / 3166-1 / 15924 — delimiter-tolerant for locale lists like `en_US:es_ES`). Value-based,
  demote-only; the 2-letter ISO-639 collision (`na` is Nauru, `os` Ossetian) is paid as harmless
  under-cleaning, never a false demotion. Gold / representative neutral, corpus-honest gate GO (zero
  triggers), per-column verified (zero false demotes on 150 files).
- **`query_string` no longer over-emitted on low-cardinality enums / short codes**. Its validator
  `^[^=&]+=…` requires a `k=v` pair, but the label was absent from the veto allowlist by rare-type
  starvation, so the flat softmax could label a five-value enum (`sector`, `subsector`, …) a query string
  at full confidence. It now joins the veto-blind strict-validator demotion set: a column whose values
  overwhelmingly fail the `=`-requiring pattern demotes to the vocabulary residual (`word`). Genuine
  query-string columns (with `=`) are untouched (zero false demotes on the corpus-honest sample).

## [0.6.43] - 2026-07-09

### Fixed

- **`mime_type` no longer over-emitted on `word/word` strings** (structural substance guard). A media
  type leads with one of the ten top-level types RFC 6838 closes off (`application`, `audio`, `text`,
  `video`, …); the taxonomy pattern `^[a-zA-Z]+/…` accepted any word as the top-level, so the model
  labelled conference codes (`ccs/stc2010`), slugs (`recipes/deep-mediterranean-quiche`), company names
  (`IAC/InterActiveCorp`), and image paths as MIME types. The new `mime_type_substance_guard` demotes a
  `mime_type`-labelled column to `unknown` when fewer than half its values pass the structural check
  (`finetype-core::structure::is_mime_type`). A structural check, not a registry list — the closed
  top-level-type set carries the certainty while the open `x-`/`vnd.` subtype trees stay valid
  (`application/x-7z-compressed` is kept). Value-based, demote-only; gold / representative neutral,
  corpus-honest gate GO (zero triggers), full-pipeline per-column verified. ~1,200 spurious MIME labels
  become honest `unknown` at corpus scale.

## [0.6.42] - 2026-07-08

### Added

- **UN/LOCODE closed-set membership guard** (`membership: unlocode`, `labels/sets/unlocode_codes.txt`,
  116,064 codes). The 5-char shape (`^[A-Z]{2}[A-Z2-9]{3}$`) admits any short uppercase code, so the
  model over-emitted `unlocode` on EDGAR stock tickers and other short codes; membership in the
  published UN/LOCODE list is the substance the shape cannot supply. `membership_substance_guard`
  demotes columns that are mostly non-members. Corpus-honest gate GO; gold / representative neutral.
- **Set-vs-set "voting" reconciliation for geo codes** (`geo_code_membership_vote`). When a column is
  labelled `state_code` or `country_code`, a dominant-member vote between the country-code enum and the
  union of subdivision-code enums (US/CA/AU) assigns the winner at ≥0.70 coverage + ≥0.20 margin —
  fixing GLEIF `jurisdiction` (89% ISO-3166-1 countries + 11% subdivisions) that the simple-majority
  `country_code_corroboration` could not, since 31 of 56 US-state codes are also country codes. Reads
  existing taxonomy enums, no new set files. Gold +1; corpus-honest gate GO.
- **Real check-digit validation for six more identifier types** (`checksum:` directives, demote-only
  via `checksum_substance_guard`): `imei` (Luhn), `issn` (ISO 7064 Mod 11-2), `orcid` (Mod 11-2),
  `cas_number` (weighted mod-10), `iso6346` (shipping-container check digit), and `dea_number`. A value
  of the right shape but wrong check digit is demoted off the type — a 15-digit number that fails Luhn
  is not an IMEI; a book/licence id that fails the DEA formula is not a DEA number. Five new algorithms
  in `finetype-core::checksum`. Gold neutral; corpus-honest gate GO; per-column verified.

### Fixed

- **`jwt` no longer over-emitted on file paths, URLs, and prose** (structural substance guard). A JSON
  Web Token is three base64url segments whose header decodes to a JSON object with an `alg` key; the
  shape pattern checks only the three-segment form, which any dotted token satisfies. The new
  `jwt_substance_guard` demotes a `jwt`-labelled column to `unknown` when fewer than half its values
  carry that decodable `alg` header (`finetype-core::structure::is_jwt`). Value-based, demote-only;
  gold / representative neutral, corpus-honest gate GO, full-pipeline per-column verified.

## [0.6.41] - 2026-07-07

### Fixed

- **`npi` / `upc` no longer over-emitted on any 10-/12-digit number** (check-digit substance
  guards wired). The model treated every 10-digit column as an NPI and every 12-digit column
  as a UPC — sweeping in financial figures (`ebit`, `marketCap`, `grossProfit`,
  `sharesOutstanding`), Unix-epoch timestamps, and product/particle-id runs (~42k npi + ~13k
  upc columns in the 33k-file corpus sample). The `checksum: npi` (Luhn) and `checksum: gs1`
  (GS1 mod-10) directives now demote a value of the right shape but wrong check digit to
  `integer_number`, cutting the over-emission ~87% (npi) / ~95% (upc). Genuine NPIs/UPCs carry
  a valid check digit and keep their type. Value-based (decision 0048); gold +2, representative
  neutral, per-column verified (demoted columns are financial figures / epochs / product ids).
  The corpus-honest gate's collapse NO-GO on npi/upc is the documented checksum-blind false
  alarm — gated-YDF is 9.6% / 2.7% reliable on these and co-signs the shape-match, so it cannot
  referee a checksum demotion — adjudicated by gold + per-column evidence per choice 0104's
  structural-blindness carve-out.

## [0.6.40] - 2026-07-05

### Changed

- **Shape-only validators no longer disarm the attractor-demotion guard** (`is_precise` now requires
  literal structure). A pattern built only from character classes and quantifiers — `^[A-Z]{4}$`
  (icao_code), `^[a-zA-Z0-9_\-.]{3,32}$` (username), `^\d{11}$` — matches every string of its shape,
  so it no longer counts as confirmation that a column really is that type. Previously such a pattern
  "confirmed" the column and skipped the demotion (a stock ticker passing `^[A-Z]{4}$` stayed
  `icao_code`). At corpus scale this correctly demotes ~2,100 shape-only over-emissions
  (icao/cusip/username) a shape pattern had been protecting; gold-neutral, representative +1,
  corpus-honest gate GO. Substance for these types lives in a checksum/membership/enum, not the
  shape. (Supersedes the earlier leniency that counted any anchored char-class body as precise.)

### Added

- **Real check-digit validation for three more identifier types** (`checksum:` directives wired to
  the canonical algorithms, demote-only via `checksum_substance_guard`): `credit_card_number`
  (Luhn), `ean` (GS1 mod-10), and `abn` (Australian Business Number — ATO modulus-89). A value of
  the right shape but wrong check digit is demoted off the type — a 16-digit number that fails Luhn
  is not a credit card. New `gs1`, `npi`, and `abn` functions in `finetype-core::checksum`.
- **IANA top-level-domain closed set** (`membership: tld`, `labels/sets/tld_codes.txt`, 1,437
  delegated TLDs incl. punycode IDNs). The shape pattern confirms any lowercase word ≥2 chars as a
  TLD; the closed list is what distinguishes a TLD from a word (only 0.24% of dictionary words are
  delegated TLDs). `membership_substance_guard` demotes columns that are mostly non-TLDs; on the
  gold corpus this lifted `top_level_domain` precision 0.667 → 1.000 with recall held.
- **New type `container.object.s_expression`** — a general S-expression / balanced-nested-paren
  type (constituency parse trees, code ASTs, Lisp/Scheme). Mined from 1,292 corpus columns
  (`trees`/`parse_tree`/`ast`) that were mislabelled `container.array.comma_separated` because a
  parse tree's Penn comma-tokens `(, ,)` fool the delimiter detector into reading a CSV array.
  Recovered at `profile` time by the deterministic `s_expression_recovery` Sharpen guard when ≥90%
  of a column's values pass the balanced-nested-paren structural check
  (`finetype-core::structure::is_s_expression`, truncation-tolerant) — value-only, no header gate,
  because the structure is self-precise (corpus over-recovery: zero). No retrain; corpus-honest gate
  GO, gold +1.

### Fixed

- **Synthetic generator emitted invalid check digits for NPI, EAN-8, and ABN.** The NPI generator
  used the wrong Luhn parity, `ean_check_digit` was left-anchored (correct only for even-length
  payloads, so EAN-8's 7 digits were wrong), and the ABN generator emitted 11 random digits with no
  modulus-89 check. All three now produce genuine check-digit-valid instances, pinned by a
  generator↔validator agreement test across every `checksum:` type. (Latent since the types were
  added; surfaced by wiring the validators. No effect on the shipped model — Sharpen-only — but it
  stops a future retrain training on fake instances the guard would then demote.)

### Discovery

- **`npi` and `upc` checksum guards are correct but gate-blocked (held).** The model over-emits
  both as pure numeric attractors (~42k `npi` / ~13k `upc` columns in the 33k-file corpus-honest
  sample — financial figures like `ebit`/`marketCap` for npi, `particleId` runs for upc). The
  checksum guards demote ~87–95% of these correctly (and are +2 on the gold corpus — they fix
  `longTermDebt`/`TEAM_ID`, real 10-digit columns that fail the check digit), but the demotion
  collapses the over-emitted label past the corpus-honest gate's blocking threshold, and gated-YDF
  co-signs the shape-match. The directives are held (algorithms retained) until the over-emission is
  fixed at source (retrain) or the guards gain header corroboration. See
  `output/company-reference-audit/` W2b log.

## [0.6.39] - 2026-07-05

Company-reference accuracy release: an external eval on company/financial lookup
datasets exposed four misclassification families; every fix below shipped through
gold no-regression + a fresh-baseline corpus-honest gate GO. Gold headline
843/986 = 0.855 on the expanded fixture (+55 company-reference columns).

### Added

- **`identity.industry.naics` (taxonomy 245 → 246).** NAICS 2022 industry codes get a first-class
  type, validated by closed-set membership against the published US Census list (2,129 codes,
  2-digit sector through 6-digit national industry; `labels/sets/naics_codes.txt`). Recovered
  deterministically at `profile` time by the `naics_industry_recovery` guard — a `naics` header
  admits any code level, a generic code-ish header admits ≥4-digit codes only — no retrain.
- **The `membership:` taxonomy directive** — closed-set sibling of `checksum:` for types whose
  substance is a published code list. `membership_substance_guard` re-checks a column's values
  against the real list and demotes non-member columns by value shape. First tenants:
  `icao_code` (10,249 published airports) and `iata_code` (9,056), plus NAICS above.
- **Real check-digit validation for FIGI, ISIN, LEI, and IBAN** (`checksum:` directives wired to
  the canonical algorithms, incl. the real FIGI check digit) — a value of the right shape but
  wrong check digit is no longer "valid".
- **`x-finetype-unknown-reason` on every profile surface.** When a column stays `unknown`,
  `profile` now explains why — e.g. *"validation rejected 'email': only 0% of values matched its
  format"*, *"too few non-null values to classify"*, or *"no type matched with sufficient
  confidence"*. Previously only `-o json-schema` carried the reason; it now also appears on
  `-o json` (verbose) and across the MCP `profile` tool. The MCP surfaces emit the two non-veto
  reasons (no veto signal there); the CLI additionally surfaces the *"validation rejected …"* case.
- **Gold corpus: +55 company-reference columns** (tickers, industry codes, org names, GLEIF
  registry fields, compound codes; blind 3-panel adjudicated) — the previously unmeasured data
  family is now on the scoreboard (931 → 986 rows).

### Fixed

- **Stock tickers are no longer "ICAO airport codes".** `^[A-Z]{4}$` confirmed every 4-letter
  ticker and thereby disarmed the attractor demotion; membership against the real airport list
  now demotes non-airport columns (honest `unknown` until a ticker type exists).
- **`user_agent` over-emission eliminated.** The shipped model's largest single wart (2.66% of
  corpus columns labelled user-agent, measured 355/355 junk with zero genuine columns) — the
  schema-fail demotion now covers `user_agent`, so prose stops shipping under that label.
- **The veto-blind strict-validator tail closed.** `wkt`, `mgrs`, `plus_code`, `dms`, `iso6346`,
  `inchi`, and `swift_bic` carried strict validators that could never hard-veto (absent from the
  audited-safe allowlist); a schema-contradicted assertion of any of them now demotes instead of
  shipping with an advisory flag.
- **Industry/registrant code columns are no longer stripped to plain integers.** The
  leading-zero heuristic structurally locked no-leading-zero code systems (NAICS, SEC CIK) out
  of `numeric_code`; a code-corroborating header now restores them.
- **Organisation-name over-reach on prose.** `entity_name` asserted on sentence-like free text
  (titles, descriptions) demotes to `plain_text` — measured zero false fires on genuine
  organisation-name columns, including connector-word names ("Bank of America Corporation").
- **Phantom fallback label.** Code-attractor demotions emitted the non-existent key
  `representation.alphanumeric.alphanumeric_id`, which skipped taxonomy enrichment and was
  invisible to the validation veto; they now emit the real `representation.identifier.alphanumeric_id`.
- **`cargo install finetype-cli` from crates.io.** The `embed-models` build fallback (taken when no
  local `models/` symlink is present, as in the crates.io source tarball) still fetched the retired
  `char-cnn-v11` as the default and then panicked — it is not a multi-branch model — so a clean
  crates.io install could not build. The fallback now fetches the live multi-branch default
  (`m2v8m-s43` plus its dual-encoder value model) from the `finetype-model` repo, mirroring
  `download-model.sh`. GitHub-release and Homebrew builds were unaffected.

## [0.6.38] - 2026-06-28

### Changed

- **Sharpen accuracy: composed gold 0.832 → 0.852** from a campaign of value-based rules — five
  Tier-A value rules plus pilots #5/#8/#11, the `utc_bare_number_veto` (bare-number columns under
  UTC-ish headers no longer mislabel as a timestamp), a URL recovery reader in
  `structured_string_refinement`, and `schema_fail_demotion` extended to six identifier/code
  over-emit labels.
- **Faster inference.** A deterministic fast-path runs *before* the model loads — value-determinable
  columns (no header) skip the model entirely (`deterministic_fast_path`, wired at `main.rs`); the
  taxonomy compile is hoisted out of the per-file profile loop (roughly halves the per-file batch
  marginal); single-column `infer` skips the sibling-context model load. CSV ingestion is more
  robust (non-parallel DuckDB retry on read failure).

### Removed

- **Legacy inference paths removed (choice 0107).** The value-level model architecture
  (CharCNN/Tiered/Transformer), the abandoned late-fusion path, and the legacy Sense→Sharpen path
  are gone — multi-branch is the only inference path (−3,973 lines). The
  `representation.discrete.categorical` sentinel is collapsed; producers emit `word` directly.

### Fixed

- CI hygiene: rustfmt drift in `profile_io.rs`; dropped the now-unused `chrono` dependency from
  `finetype-cli` (orphaned by the choice-0107 removal).

### Discovery

- **Model label-space reshape: NO-GO** (choice 0108 rejected). Dropping the 134 validator-ownable
  leaves and recovering them deterministically costs ~4pp gold across 3 seeds — the kept-class model
  degrades on its residual boundary.
- **Clean-label retrain: NO-GO — training-label quality is not the accuracy ceiling** (spec
  `2026-06-28-clean-label-retrain`). Swapping the geography/person training labels for
  GeoNames/Wikidata vocab-membership clean positives left composed gold flat (0.845 vs 0.853); the
  shipped model already saturates semantic gold (city 0.96, country_code 0.93, continent 1.00).
  Replacing real columns with synthetic clean ones *regressed* −7.9pp (a distribution shift, not a
  label-quality signal). 4th confirmation that composed accuracy is rule-bound.

## [0.6.37] - 2026-06-25

### Added

- **New type `datetime.offset.timezone_abbreviation`** (EST/EDT/CEST/GMT/UTC/…) mined from the
  `word`/`iana` residual (spec `2026-06-25-timezone-abbreviation-type`; 1,504 distinct corpus
  datasets cleared the volume bar). Closed-enum validator, UPPERCASE-only (case discriminates
  `WET` the zone from `wet` the word). Recovered deterministically at `profile` time via the
  `timezone_abbreviation_recovery` Sharpen guard (tz-ish header + ≥90% closed-set match) — the
  240-dim model does not predict it directly; NO retrain. Gold +5 (the 6 affected columns
  re-adjudicated to the new leaf). Taxonomy 244 → 245.
- **Frictionless Data Package output: `finetype profile -f <file> -o datapackage`** (choice 0105,
  spec `2026-06-24-frictionless-datapackage-profile-output`). Emits a conformant Frictionless
  v2.0 **Data Package** descriptor — one Data Resource (`name`/`path`/`format`/`mediatype`/
  `encoding`/`bytes`/`sha256` hash) wrapping a Table Schema whose field `type`/`format` come from
  a new **authoritative per-leaf `frictionless:` map** added to all 244 taxonomy definitions (the
  canonical 244→16 fold FineType owns for the Meridian family — dovetail and arcform consume it
  rather than re-deriving it). Constraints come from the type's validation; FineType richness
  rides as `x-finetype-*` custom properties (`label`/`confidence`/`pii`/`locale`/`enum-domain`).
  `$schema` pins the v2.0 profile (vendored at `vendor/frictionless/`); emitted descriptors are
  conformance-tested against it. Mirrored on the MCP `profile` tool (`format: "datapackage"`).
  Additive — the `json-schema` output is unchanged; the executable DuckDB `transform` is
  deliberately omitted (a Data Package *describes*, it does not *execute*).
- **`finetype_core::frictionless_for(label)`** (feature `embedded-taxonomy`) — the accessor that
  exposes the label→Frictionless `{type, format}` map to in-workspace crate consumers without a
  `labels/` dir at runtime. The taxonomy `check` gate now fails on any leaf missing a valid
  `frictionless:` block.

### Changed

- **Retired two value-blind header-hint arms** (spec `2026-06-25-sharpen-stage-audit`): the
  `class`/`grade`/`rank`/`tier` → `ordinal` arm and the broad `…name` → `full_name` arm. A
  per-family ablation against the strongest Sense showed both were net damage (they over-rode a
  now-correct value-based prediction on compound headers). Aligns with decision 0042. Gold +3 on
  the shipped default, zero regressions; recovers `Grade`/`GlobalRank`/`Region Rank`/
  `template_name`/`…country_name`.

### Fixed

- **Header hints no longer override a value-contradicted prediction** (`header_hint_value_corroboration`):
  a `…year`/`epoch` header that promoted a decimal/id column to `year`/`unix_seconds` over the
  column's own values is now declined when the values fail the hinted type's validator.
- **URL over-emission on non-URL columns** (spec `2026-06-25-sharpen-stage-audit`): a `link`/`url`
  header no longer promotes msg-id / prose / flag columns to `url`. The URL validator also now
  accepts protocol-relative `//host/path` URLs (closes the `//` gap). Gold +5 on the default.
- **`geography.index.h3` over-emission** on generic alphanumeric id columns — added to the R32
  schema-fail demotion scope (its sibling `geohash` was already covered).

## [0.6.36] - 2026-06-24

### Changed

- **Default Sense model: `sherlock-v19-relu-s42` → `m2v8m-s43` (potion-8M dual-encoder) —
  v19 retired.** The new default is **reproducible** (`scripts/overnight_potion.sh`) and
  **244-label** (tracks the live taxonomy; v19 was 240-label and could not predict the new
  leaves, stranding taxonomy growth). Gold composed 0.794 ties v19's 0.797 within CI;
  Sense 0.522 > 0.502; latency ~free (static encoders). The swap was gated by **gold parity
  plus a gold-adjudicated relocation review** (choice 0104), not a corpus-honest GO — that
  gate is structurally unpassable by any model retrain (it measures deviation from v19).
  Known residual: short-code / `user_agent` over-emission (gold-invisible corpus-scale
  warts) deferred to a follow-up retrain with better negatives.

### Fixed

- **`currency_code` validator → ISO-4217 membership** (was `^[A-Z]{3}$`, which accepted any
  three uppercase letters — UDP, TCP, EDT, team/airport codes). Per the Precision Principle,
  a validation that confirms most random input is not a validation; this lets the validation
  veto suppress non-currency 3-letter codes. Real currencies (incl. lowercase) unaffected.
- **Validation type-key fallback** — when a model config omits `type_index_keys` but has a
  trained validation branch, the index order is derived from the live taxonomy instead of
  silently feeding the branch zeros (which made such a model mis-predict). Config-pinned
  keys still take precedence.

### Added

- **Dual-encoder native inference.** `MultiBranchClassifier` can load a second Model2Vec
  encoder for the value-aggregation branch (model config `value_embed_model`), so a model
  can use potion-8M (256-dim) for values while potion-4M (128-dim) drives the header branch
  and the semantic/entity/sense classifiers. The value encoder is co-located in the model
  dir (`value_model2vec/`), embedded in release binaries (build.rs `MB_VALUE_*`), and
  fetched by `download-model.sh`. Single-encoder models (v19, m2v-244) are unaffected.

- **Three types mined from the `plain_text` residual** (spec
  `2026-06-19-plain-text-type-discovery`, card 0001). `representation.text.plain_text` is
  FineType's largest bucket (447k corpus columns); mining it by structural shape and ranking
  candidates by distinct-dataset breadth × blind-panel nameability surfaced three precise,
  high-volume types now added to the taxonomy: `technology.filesystem.windows_path`
  (7,651 datasets, panel 0.97), `technology.internet.message_id` (RFC 2822, 3,032 datasets,
  0.95), and `technology.code.qualified_name` (reverse-DNS FQN, 1,677 datasets, 0.79). Each
  ships a generator (100% taxonomy-check alignment) and a precision-validated validator
  (qualified_name: 0 prose false positives; windows_path: 1.1%). Rejected candidates
  (numeric ranges, quantity+unit, identifier soup) cleared volume but were panel-flagged
  "mixed" — unwriteable as a precise validation. All three emit **live** at `profile` time via
  a deterministic gated reader (`structured_string_refinement`, below) — no retrain required.

- **`structured_string_refinement` reader** — covers the three plain_text-discovery types
  deterministically in the Sharpen layer, mirroring the datetime reader. Because they are
  value-determinable (precise validators) and the 240-dim Sense model cannot predict them, a
  gated reader recovers them: a **corroboration gate** fires only where the model gave up
  (`plain_text`/`word`/`unknown`, plus the unambiguous `windows_path`/`message_id` validators
  also recover path/email mispredictions), and a **veto-consistency gate** asserts a leaf only
  if ≥90% of values pass that leaf's own validator. `qualified_name` is residual-only (it
  structurally overlaps `hostname`/`url`), verified not to eat a confident hostname. Smoke- and
  unit-tested; structurally cannot relocate non-matching columns. RHH-disableable.

- **Two zoneless ISO-8601 datetime leaves** (spec
  `2026-06-19-zoneless-iso-datetime-leaves`, card 0002). `datetime.timestamp.iso_seconds`
  (`2013-06-04T01:02:03`) and `datetime.timestamp.iso_milliseconds` (`…:03.123`) — the
  zoneless siblings of `iso_8601` / `iso_8601_milliseconds`, mirroring the existing
  `iso_microseconds`. Add-not-broaden: the zoned leaves keep their Z-required patterns so
  round-trip `validate` transforms stay correct; the deterministic datetime detector now
  requires a trailing Z on the zoned leaves and routes zoneless values to the new siblings.
  Live at `profile` time today via the `datetime_format_refinement` Sharpen rule — values
  that previously matched no datetime leaf (the veto declined them against the Z-required
  patterns) now name and round-trip correctly. Gold/representative/corpus gates deferred to
  the corpus run.

### Changed

- **`representation.discrete.categorical` retired as a taxonomy leaf** (choice 0102).
  Categorical is now exclusively the orthogonal enum-domain property (`x-finetype-enum`),
  not a competing semantic label. The reframe shipped incrementally over prior releases
  (gold migrated, eval `--reframe` residual, the model's internal categorical sentinel
  remapped to `representation.text.word` at finalize, enum-domain emission); this release
  removes the last vestige — the taxonomy leaf and its category-map entry. Runtime
  predictions are unchanged (the internal sentinel + fusion cardinality gate remain).
  Net taxonomy: 240 → 242 (−1 categorical, +3 plain_text-discovery types).

- **Deterministic datetime sub-format reader** (spec
  `2026-06-19-deterministic-datetime-parser`). A delimited datetime string resolves to
  exactly one taxonomy leaf by its shape and field ranges, so a value-based Sharpen rule
  (`datetime_format_refinement`) now reads the format deterministically instead of trusting
  the model's guess between near-identical sub-formats (iso_8601 vs `…_milliseconds` vs
  `sql_standard` vs rfc_3339). It recovers timestamps the model dropped to `unknown` and
  fixes sub-leaves. Over-emission-safe by construction: bare integers (epoch/year) are only
  read as datetime when the model already agrees, and a format is only asserted when the
  column passes that leaf's own taxonomy validator — so the rule cannot relocate
  non-datetime columns into datetime (corpus A/B over 2,000 columns: zero relocation, zero
  over-emission). Gold +1, representative held; RHH-disableable.

## [0.6.35] - 2026-06-19

### Added

- **Honest confidence signal in `profile` output** (spec
  `2026-06-18-calibrated-confidence-abstention`, card 0020). Each column now carries
  a `quality_band` (`high` ≥ 0.85 / `medium` / `low` < 0.70) over the existing
  confidence, plus a `runner_up` type on the `low` band — so a shaky column reads
  "probably X, maybe Y" instead of a bare guess. Purely additive: the predicted label
  and raw confidence are unchanged. Thresholds are the data-driven knees from the
  gold + representative reliability curve; on representative data `high`-band columns
  are ~0.82 accurate vs ~0.54 for `low`. The shipped confidence ranks correctness but
  is not calibrated, so the band reads the ranking, not the raw number. Trust the
  `high`/`low` bands; the `medium` tier is statistically indistinct.
- **Username recovery rule** (spec `2026-06-17-full-name-username-veto`). High-cardinality
  login-handle columns the model called `full_name` (a distinct handle per row) are
  recovered as `identity.person.username` — a value-based rule, with a cardinality guard
  so low-cardinality repeating vocabularies (exchange codes, drug names) are left alone.

## [0.6.34] - 2026-06-17

Enum reframe: `representation.discrete.categorical` is retired as an emitted label.
Bounded-domain columns now report their real representation type, with bounded-ness
carried by the descriptive `x-finetype-enum` property shipped in 0.6.33 — completing
choice 0102 ("categorical is a property, not a type").

### Removed

- **`representation.discrete.categorical` is no longer emitted** (spec
  `2026-06-17-enum-accuracy-reframe`, choice 0102 deferred scope). It was the single
  biggest production error mass and, as a flat-softmax residual attractor, the cause
  of past retrain explosions (decision 0096). Re-adjudication of the gold categorical
  columns confirmed 71 of 73 have no tighter semantic type — they are genuinely
  bounded vocabularies (exchange codes, status flags, content tags), not a distinct
  data type.

### Changed

- Columns that previously profiled as `categorical` now report their honest
  representation type — `representation.text.word` for short single-token
  vocabularies, `representation.text.plain_text` for phrase-shaped ones — alongside
  the `x-finetype-enum` bounded-domain flag. Implemented as an output-boundary remap
  in the Sharpen layer (`finalize_is_generic`, validation veto-fallback); the Sense
  model is unchanged. Gold corpus headline is unaffected (reframe scorer 0.800), and
  the corpus-honest gate is GO (the relocation is oracle-clean: categorical shed
  −25,128 oracle-confirmed columns, `text.word` gained them at correct_ratio 1.0).

## [0.6.33] - 2026-06-17

Enum-domain emission: `profile` now reports each column's observed bounded value
domain, and the JSON-Schema `enum` keyword is hardened against `enum_overfit`.

### Added

- **Enum-domain emission** (choice 0102, patch increment — spec
  `2026-06-17-enum-domain-emission`). `profile` now reports each column's observed
  bounded value domain as a descriptive `x-finetype-enum` extension
  (`{ open, domain, distinct, rows, cohesion }`), **decoupled from the semantic
  label**: a `country_code` column emits its domain `[FR, GB, US]` just as a
  categorical column does — enum-ness is a representation property, not a competing
  type. Detected from full-column cardinality (`distinct <= 32`,
  `distinct/rows <= 0.5`) with a denylist (numeric / coordinate / datetime /
  identifier / url); a character-shape **cohesion** score rides along as an analyst
  signal. The CLI (`profile`, `-o json` and `-o json-schema --stats`) and the MCP
  `profile` tool share one policy (`finetype_core::enum_domain`).

### Changed

- The validation-enforced JSON-Schema **`enum` keyword stays conservative**
  (categorical/boolean labels only). This **fixes** a prior cardinality-only
  over-emission in the json-schema / MCP path that could freeze an open domain into
  a closed constraint (`enum_overfit`, card 0014). The open domain now lives in the
  descriptive `x-finetype-enum` extension, which validators ignore — so the
  `profile -> validate` round-trip is unaffected.

## [0.6.32] - 2026-06-16

Ingestion and interface release: `profile` now reads CSV **and Parquet** through
the DuckDB engine, and the MCP server's tool surface is brought back into parity
with the CLI. **Operational note:** the `duckdb` CLI is now a hard runtime
dependency — install it (`brew install duckdb`, or your platform package manager)
before running `profile`/`validate`.

### Changed

- **CSV/Parquet ingestion routes through the `duckdb` CLI** (choice 0100).
  `finetype profile` replaces its bespoke CSV reader with a shell-out to the
  external `duckdb` binary: DuckDB's parallel sniffer handles dialect detection,
  quoting, and ragged rows, and the same path now reads **Parquet** (the old
  reader could not). This is a shell-out, not a link — the release binary is
  unchanged across platforms (no `libduckdb` compile), so no Windows/MSVC risk.
  NULL rendering is pinned (`.nullvalue ''`) so ingestion is independent of the
  user's `~/.duckdbrc`. `validate` already shelled out to duckdb; this unifies
  the two ingestion paths.
- **The `duckdb` CLI is a hard runtime dependency** (choice 0100). `profile` and
  `validate` fail with a single actionable error when it is absent
  (`could not invoke duckdb CLI (is duckdb on PATH?) … Install it from
  https://duckdb.org/docs/installation`). Documented in the README +
  docs/DEVELOPMENT.md; declared in the Homebrew formula.
- **MCP tool surface now mirrors the CLI** (choice 0101). One capability surface,
  enforced by a parity-guard test: an agent and an analyst see the same FineType.

### Added

- **MCP `taxonomy` tool gains JSON Schema export.** Pass a `key`/glob plus
  `format: "json-schema"` to get a per-type Draft-2020-12 schema — identical to
  CLI `taxonomy KEY -o json-schema`. This absorbs the type-mode of the retired
  `schema` tool.

### Removed

- **MCP `schema` tool** — folded into `taxonomy` (type-mode, `format:
  "json-schema"`) and `profile` (table-mode, already present), completing the
  CLI's choice-0070 fold on the MCP side.
- **MCP `ddl` tool** — retired (parity-down). The CLI has no DDL command; the
  typed-table surface is `validate --db/--table`.

### Discovery

- **Gold corpus re-adjudication: 738 → 745 / 931 (0.793 → 0.800).** A
  mixed-panel (Opus/Sonnet/Haiku blind + adversarial) re-adjudication of the
  heuristic gold tiers applied 33 label corrections in place to
  `eval/gold/gold_corpus.tsv` (now a single canonical file — git + per-row
  provenance is the version history). Corrections skew *away* from the shipped
  model, so the modest headline shift confirms re-adjudication cleans gold rather
  than inflating it.

## [0.6.31] - 2026-06-16

A batch of gold-gated deterministic Sharpen rules on value-identical and
column-level boundaries — verified gold-corpus accuracy **0.741 → 0.793**
(690 → 738 / 931), every rule a corpus-honest GO with no broad regressions. The
release also records the architecture-search conclusion: model-side delivery is
exhausted; the next accuracy lever is full-column statistics fed as value-based
rules, not a bigger model.

### Added

- **`increment_substance_veto`** — the first full-column-statistics rule. The
  value-level sequential check runs on the 100-value *stepped* sample, which
  cannot see contiguity (a true `1..N` run sampled every *k*-th value looks like
  `1, k, 2k…`), so it over-emitted `representation.identifier.increment` on any
  evenly-spaced numeric column. The veto re-checks the **full column**: a genuine
  auto-increment fills its own range (`distinct ≈ max−min+1`) with near-no
  duplicates; otherwise it is a plain integer and is demoted to `integer_number`.
  Gold `integer_number` recall 0.796 → 0.847, `increment` false positives 17 → 7
  (precision 0.056 → 0.125), headline 0.782 → **0.793**, corpus-honest GO.
- **`country_code_corroboration`** — two-letter ISO 3166-1 codes (`US`, `HK`)
  the flat softmax filed under `region`/`state`/`city`/`country` are promoted to
  `country_code` when their values are mostly valid ISO codes (the taxonomy's
  most precise geographic enum). Value-based, promotion-only.
- **`binary_vocab_veto`** — `representation.boolean.binary` over-emitted on sparse
  integer count columns; an all-integer column with any value outside `{0,1}` is
  a count, demoted to `integer_number` (gold 0.757 → 0.770).
- **`url_bare_number_veto`** — a link-headed column of bare numbers cannot be a
  URL; demote to decimal/integer by value shape (gold 0.747 → 0.753).
- **`city_region_header_corroboration`** — a `city` prediction under a header
  naming an administrative division (`region`/`county`/`district`/`province`) is
  promoted to `region` (gold 0.741 → 0.747).
- **`checksum_substance_guard`** — one generic check-digit guard for the
  self-validating identifier types: a column labelled with a `checksum:`-bearing
  type (ISBN, ABA, CUSIP, SEDOL) whose values mostly fail the real check-digit
  arithmetic is demoted by value shape. ABA/CUSIP/SEDOL enrolled with an
  alphanumeric guard branch.

### Changed

- Large source files split into module trees for maintainability
  (`column.rs`, `multi_branch.rs`, `main.rs`, `generator.rs`); behaviour
  unchanged. Six duplicate eval harnesses consolidated into one parameterised
  `scripts/eval_rule.sh`.

### Removed

- The ISBN-specific check-digit veto, superseded by the generic
  `checksum_substance_guard`.

### Fixed

- **Training `config.json` written at training start**, not only on completion,
  so an interrupted run leaves a loadable model directory.
- **CI HuggingFace model download** hardened against transient `curl` exit-22
  errors with a retry.

### Discovery

- **The Sense model architecture is at its accuracy ceiling for this corpus.**
  Six model-side bets were ruled out with evidence — additive hard-negative
  retrains (0-for-5), value-level late-fusion (additive *and* deferral), a
  sibling-context attention head (thin gold-recall headroom: its clean target,
  coordinates, is already solved), and a hierarchical Domain→Family→Type head
  (trained and falsified — worse than the flat head, because splitting the
  output head does not fix interference that lives in the shared representation).
  The surviving lever is **full-column statistics fed as value-based rules**
  (cardinality, increment signature, binary domain), which separate the residual
  recall gaps the per-value model is structurally blind to — free inside DuckDB
  and likely a net inference *saving*. `increment_substance_veto` is its first
  shipped instance.

## [0.6.30] - 2026-06-15

Two gold-gated Sharpen rules on value-identical boundaries; verified gold-corpus
accuracy 0.719 → **0.741** (690/931), both corpus-honest GO with no broad regressions.

### Added

- **`--logit-adjust-tau` on `train-multi-branch`** (choice 0097): logit-adjusted
  loss (Menon et al., ICLR 2021) for strengthening a frequency-starved class by
  reweighting the training gradient rather than adding data volume. Default `0.0`
  (off); training-time only, **zero inference cost**; flat head only. Banked for a
  future frequency-starved (not value-shape-overlapping) class.

### Fixed

- **`state_code` detection** (`header_hint_state_code_promote`): a column of
  closed-vocabulary subdivision codes (US/CA/AU, from the taxonomy
  `validation_by_locale`) with a `state`/`province` header now profiles as
  `geography.location.state_code` instead of the state-name type or `region`. Gold
  `state_code` precision/recall **0.000 → 0.857** (0 → 6 of 7), headline 0.719 →
  0.725, zero new false positives. The state header is load-bearing — 2-letter codes
  overlap ISO country codes (`CA` = California and Canada).
- **`currency.amount` over-emission** (`amount_bare_number_veto`): a bare-number
  column (`netIncome` 795000000, `interestExpense` -68000000) promoted to
  `currency.amount` by a money-ish header is demoted back to `integer`/`decimal`.
  A genuine amount carries a currency signal (`£45.17`, `EUR 4 459 807`); the false
  positives are bare numbers. Gold `currency.amount` precision was 0.105 (17 false
  positives); `integer_number` recall 0.592 → 0.673; headline 0.725 → **0.741**.

## [0.6.29] - 2026-06-12

### Added

- **Word vocabulary override (R32)** (spec `2026-06-12-text-vocab-override`):
  a column labelled `representation.text.word` that repeats a small
  vocabulary (2–12 distinct, ≤60% distinct ratio) now profiles as
  `representation.discrete.categorical` — the missing correction for the
  no-validator text family. Gold corpus: 662 → 669 of 931 (0.711 → 0.719),
  categorical recall 0.396 → 0.465 with precision up (0.870), zero
  regressions. Deliberately scoped to `word` only: the corpus-honest gate
  measured that low-cardinality entity_name/plain_text columns are usually
  genuinely entities/prose (5,867 oracle-refuted moves in the broad
  variant, which was rejected).

### Changed

- **Corpus-honest gate `over_emit` band is composition-aware** (sibling of
  the 0.6.24 oracle-aware refinement): oracle-confirmed correct growth is
  netted out of the ratio, so consecutive honest fixes in one direction
  cannot stack into a false NO-GO while relocation is still caught in full.
  All preserved verdicts re-validated, including the broad-R32 negative
  control (`output/corpus-honest-gate/refined/composition_aware_over_emit.md`).

## [0.6.28] - 2026-06-12

### Added

- **Veto shape-fallback** (spec `2026-06-12-veto-shape-fallback`): when the
  validation veto hard-rejects a Sense assertion, the column's value shape
  now decides between the two residual labels instead of an unconditional
  `unknown` — mostly-distinct letter+digit values fall back to
  `representation.identifier.alphanumeric_id`; a small repeated vocabulary
  falls back to `representation.discrete.categorical`. Gold corpus: 635 →
  662 of 931 (0.682 → 0.711), alphanumeric_id recall 0.111 → 0.593 with
  precision held, zero per-label regressions. Corpus-honest gate GO with
  zero bands fired (alphanumeric_id's oracle-contradicted count net
  NEGATIVE — the fallback removes more wrong assertions than it adds).
  Recorded in profile output as `veto_fallback:id` / `veto_fallback:vocab`;
  disable with `FINETYPE_NO_VETO_FALLBACK=1`.

## [0.6.27] - 2026-06-10

The first release whose accuracy claims are verified against human-checked
ground truth. This cycle built FineType's gold corpus — 931 real-world columns
with verified labels — measured every eval instrument against it, and shipped
the first fix the new instrument unblocked.

### Fixed

- **FineType no longer calls your volume and count columns postal codes.**
  Bare 4–5-digit integer columns (trading volumes, employee counts, sequence
  numbers) are value-identical with Nordic/Australian postcodes, and the model
  over-asserted postal on them — measured precision 0.133 on verified data
  (wrong 26 times for every 4 right). A header-corroboration veto now demotes
  a postal prediction to plain integer unless the header carries a postal
  token (`zip`, `postal`, `postcode`, `PLZ`, `CEP`, `pincode`, …). Leading-zero
  values (`01219`) are treated as postal evidence and are never demoted, and
  the rule is demotion-only — a header can never create a postal code. On the
  gold corpus: postal precision 0.133 → 0.667 with recall unchanged at 1.000,
  overall verified accuracy 65.5% → 68.2%; corpus-honest gate GO with zero
  triggers. (`header_hint_postal_veto`, spec 2026-06-10-postal-header-veto)
- **F2 emits valid `technology.development.docker_ref`** (was an orphan label).

### Changed

- **Eval doctrine (choice 0095):** the gold corpus is FineType's canonical
  accuracy eval; the gated-YDF oracle is demoted to a mining/corroboration
  lens after measuring its error rate on contested columns (42% of its
  assertions wrong where promotion fights happened). Engine behaviour is
  unchanged — this governs how FineType is measured, not how it infers.

## [0.6.26] - 2026-06-09

A validation-honesty patch. The thread through every fix: FineType was being
wrong about *real* data — rejecting values that were valid, or asserting a type
its own values contradict. This release makes the checks honest: accept what's
genuinely valid, withdraw a guess the data refutes, and never abort a run.

### Fixed

- **Case-insensitive enum validation.** A column matched to a learned value set
  (e.g. `gender`) no longer rejects valid values over capitalisation alone —
  `Male`/`MALE`/`male` all pass where the learned set happened to be lower-case.
  Membership folds case while a co-attached pattern stays exact; no taxonomy
  enum distinguishes two members by case, so nothing is wrongly merged. Fixes
  every row of a Title-cased column being rejected against a lower-cased enum.
- **IANA timezones validate by pattern.** `datetime.offset.iana` ANDed a correct
  `Region/City` pattern with a 12-zone enum stub, so every real timezone outside
  those twelve was rejected (~5,700 spurious rejects on one corpus file). The
  stub is gone; the structural pattern — which already matches the full 500+ tz
  database — is the validator. A hardcoded enum would only go stale and silently
  reject newly-added zones.
- **No crash on text nanosecond-epoch columns.** A text-stored
  `datetime.timestamp.epoch_nanoseconds` column aborted the whole `validate` run
  (its transform had no binder overload for text input). It now casts then
  converts, matching the sibling unix-epoch types, so a bad cell becomes an
  ordinary reject instead of killing the run.
- **Two more over-emitted types step back to `unknown` instead of asserting
  wrong.** FineType was confidently labelling order-status columns as a "how
  often" type (`periodicity`) and composite values like `155/82` as decimals —
  ~14,000 corpus columns for periodicity alone, none confirmed by the gated-YDF
  oracle. When a column's values fail its predicted type's validation, these two
  now demote to `unknown` rather than assert a label the data refutes. Cleared
  the corpus-honest gate with zero regressions; the safety net only acts on
  types proven safe at corpus scale.

## [0.6.25] - 2026-06-09

A precision patch built on a diagnosis. The starting point was a worry that the
eval suite or the data was capping precision — and it was: the metric we judged
progress with was structurally blind to the rare-type errors recent rounds were
trying to fix. The headline change is the first fix that diagnosis unblocked —
FineType no longer mistakes plain numbers for map coordinates.

### Fixed

- **Coordinate header-veto.** A `latitude`/`longitude` prediction on a column
  whose header carries no coordinate token (e.g. `magnitude`, `gpa`, `rms`,
  `error`, `price`) and whose values are generic numbers is now demoted to
  `decimal_number`. These false positives are sibling-context-driven and
  value-identical to real coordinates — only the header separates them — so the
  rule vetoes a false coordinate by header and never promotes one. Cuts the
  latitude false-positive rate ~12× on the corpus with no recall loss and no
  cross-type regression (cleared the corpus-honest gate and the curated breadth
  eval). Choice 0094.

### Discovery

- **The precision "ceiling" was a measurement artifact.** Aggregate corpus
  precision (~0.49 vs the gated-YDF oracle) is 52% plain integer/decimal, while
  the rare types recent rounds fought over (latitude/longitude/url/utc) are
  ~0.0003% of scored columns — so it counted a fix's collateral damage but never
  its gain. Added an oracle-free, header-anchored **rare-type scoreboard** as the
  headline pre-promotion check (choice 0093). Full diagnosis in
  `output/eval-ceiling-diagnosis/`.
- **Hardcoded header hints are mixed, not removable yet.** A corpus-scale +
  curated multi-instrument ablation showed deleting/deferring them regresses
  (curated −4pp, corpus gate NO-GO) even though the corpus gate alone green-lit
  defer — the hints remain load-bearing for url/datetime/isbn until the model
  can cover them. Lesson recorded: a single GO is not safety.

### Changed

- Removed 10 genuinely-unused dependencies and added a `cargo machete` CI gate;
  consolidated six duplicate `get_device` helpers; added a pre-push lint gate.

### Removed

- Stripped ~700 internal issue-tracker identifiers from the public-repo source
  and docs, and pruned stale "removed in vN" history comments.

## [0.6.24] - 2026-06-08

A precision-hardening patch. The theme is **FineType says fewer wrong
things**: when a column is labelled a type but its own values won't validate
as that type, the engine now steps back to a safer label instead of asserting
the type anyway. Net effect for analysts — fewer confident mislabels on the
hard, ambiguous columns, with no loss on the columns that were already right.

### Added

- **Validation-as-veto, on by default.** A new Sharpen-stage funnel
  (`finetype-core/src/validation_veto.rs`): after Sense proposes a type, the
  column's sampled values are checked against that type's JSON Schema, and a
  prediction whose own values demonstrably fail their validator is vetoed
  rather than emitted. Wired into the default profile path; `--no-validation-veto`
  restores the prior behaviour. This is the Precision Principle made
  load-bearing — a label that 90% of the column's values reject is not a label.
- **Gold evaluation anchor — independent ground truth for the confusion
  families.** A 240-column curated set with hand-verified labels
  (`eval/gold/`), scored by `scripts/score_gold_anchor.py`. It measures
  *efficacy* — does a fix actually land on the hard columns — separately from
  corpus breadth. v19 baseline: macro precision 0.956, recall 0.754.
- **Corpus-honest quality gate (promotion infrastructure).** A post-train,
  pre-swap gate (`scripts/corpus_honest_gate.py`) that scores a candidate's
  predictions on a 33,250-file stratified sample against a stable gated-YDF
  oracle, catching rare-label *relocation* — a fix that moves an error rather
  than removing it — which curated instruments miss. A NO-GO is blocking; a GO
  is advisory. Plus a destination-drift pre-check (`scripts/proxy_pretrain.sh`
  + `scripts/drift_report.py`) that rejects a bad retrain bet before the
  overnight run rather than after.

### Changed

- **`schema_fail_demotion` widened to `datetime.offset.utc` and
  `technology.internet.url`.** When the majority of a column's values fail the
  predicted type's validator, the prediction is demoted. Clean A/B on the
  corpus sample: utc over-emission 54 → 1, url 403 → 392, with zero geography
  collateral.
- **Over-emitted `measurement_unit` / `geohash` demoted on majority
  schema-fail.** Closes a long-tail false-positive band — on the earthquake
  reference set the reject rate fell 0.1494 (grade F) → 0.0130 (grade C).
- **`geohash` reconciled to a deliberate 6–12 character floor.** The validator
  and its description now agree on the minimum length, ending spurious short-
  token geohash matches.

## [0.6.23] - 2026-06-04

### Added

- **DuckDB extension gains a `profile → schema → validate` table surface
  (spec `2026-06-03-duckdb-extension-table-verbs`, amends choice 0064).**
  The extension was six per-value scalars with no table verb — validating
  a loaded table meant hand-writing an `UNPIVOT` + `json_keys` join. It now
  ships an `ft_`-prefixed surface that mirrors the CLI:
  - `ft_profile(table)` — one row per column with its detected type,
    confidence, and recommended DuckDB type. A SQL **table macro**, so it
    reads a catalog table by name (the path the Rust `BindInfo` catalog
    wall blocked — choice 0064 is amended to record that macros reach
    table verbs even though Rust table functions remain walled).
  - `ft_validate(table, schema)` — answers "is this table valid?" against a
    JSON Schema, returning per-column `total` / `rejects` / `sample_message`.
    The one `schema` argument auto-detects an inline JSON literal, a
    `getvariable` value, or a file path in a single argument.
  - `ft_infer(value)`, `ft_validate_text(value, schema)`,
    `ft_profile(list(col))` — scalar probes; `ft_detail` / `ft_cast` /
    `ft_unpack` / `ft_version` utilities.
  Because the model is column-oriented, `ft_profile` over a row sample is
  the *accurate* path, not sugar over the single-value `ft_infer`.
  **Nested-column guard (Precision Principle):** `COLUMNS(*)::VARCHAR`
  flattens STRUCT/LIST to display text that is not valid JSON; rather than
  silently FALSE-rejecting it, the table verbs surface the column with a
  message to unnest / `to_json` / extract first. Verified end to end on
  DuckDB 1.5.3.

### Changed

- **`ft_` is the taught surface; the un-prefixed scalars stay as aliases
  for one release.** `finetype()`, `finetype_validate()`, etc. still
  resolve so a v0.6.22 community install does not break, but docs and the
  community `description.yml` now teach the `ft_` names.

## [0.6.22] - 2026-06-03

> 0.6.21 was tagged but shipped no artifacts — all five release builds
> failed, so the tag was withdrawn. 0.6.22 carries the 0.6.21 changes plus
> the recovery fixes below.

### Fixed

- **Recovered the failed 0.6.21 release.** Three regressions sank every
  0.6.21 build job:
  - *Model drift.* `models/default` had been repointed to
    `sherlock-v22-boundary-relu-s44`, which is not published on
    HuggingFace, while CI fetched the v19 model — so every build failed
    "Flat model not found". Reverted `models/default` to
    `sherlock-v19-relu-s42` (the shipped default; the v22 promotion is
    deferred until the model is published — see the Added note below).
  - *Windows extension build.* The DuckDB extension forced vendored
    OpenSSL on all platforms; the Windows runner's MSYS Perl cannot
    configure an OpenSSL source build, so the Windows artifact failed.
    Vendored OpenSSL is now scoped to non-Windows (native-tls uses
    SChannel on Windows, the Security framework on macOS).
  - *Windows binary build.* `finetype-train` bundles DuckDB's C++
    amalgamation, which fails to compile under the Windows release
    runner's MSVC on DuckDB 1.5.3. `finetype-train` is now an optional
    dependency behind a `train` feature; the shipped `cpu` build links
    zero DuckDB. GPU dev builds (`cuda`/`metal`) re-include it, so
    training from source is unchanged.
- **DuckDB community extension republished against the stable C API
  (spec `2026-06-02-duckdb-extension-republish`, choice 0063).**
  `INSTALL finetype FROM community; LOAD finetype;` was returning
  HTTP 404 on DuckDB 1.5.2/1.5.3: the old standalone source built
  against the *unstable* C API, which version-locks each artifact to
  one exact DuckDB release, so the registry rebuild produced no
  matching binary. The extension is now consolidated in-tree
  (`crates/finetype-duckdb`), bumped to DuckDB 1.5.3, and stamps the
  stable `C_STRUCT` ABI at the `v1.2.0` floor — one artifact loads on
  DuckDB 1.2 through 1.5+, so a routine DuckDB patch no longer breaks
  the install. The community build contract (extension-ci-tools
  submodule, `configure/`, root `Makefile` targets) and a CI
  distribution job are vendored so a tagged ref is rebuildable by
  community-extensions CI with no manual steps. The extension also now
  loads its model correctly from any working directory (Model2Vec
  resources are fetched co-located with the model files).

### Added

- **v22 boundary-training Sense model landed in-tree; promotion to
  default deferred (spec `2026-05-26-v22-gated-direction-review`,
  card 0002).** Multi-branch `sherlock-v22-boundary-relu-s44` is in the
  model tree with its full training campaign. Gated cell-2 vs v19 lands
  at **−10.4% (Partial band)** on 503k columns of the gittables corpus
  pass; per-subtype gains: country **−31.5%**, region −12.8%, city
  −10.2%, longitude −14.3% (the four monotone-movers absorb 95% of v19
  cell-2 misses). val_acc 0.9305 (+0.0132 over v19). The
  v19→v20→v21→v22 ratchet is monotone on the dominant subtypes; no
  v22-jumpers — the recipe works as a campaign, not a v22-only spike.
  Trajectory at `output/v22-direction-review/per_subtype_trajectory.md`.
  **`models/default` remains `sherlock-v19-relu-s42`** — v22 is not yet
  published to HuggingFace, so promoting it would break the release
  fetch (see the recovery note under Fixed). Promotion follows once v22
  is published.
- **Gated YDF baseline is the canonical scoring lens (spec
  `2026-05-26-ydf-validation-gate`).** `scripts/apply_ydf_validation_gate.py`
  writes `ydf_prediction_gated` alongside the raw `ydf_prediction`:
  NULL when fewer than 50% of a column's sample values pass the
  predicted label's JSON Schema validation. Stops the metric from
  penalising Sense for disagreeing with demonstrably-wrong YDF labels
  (msg_id → iso6346, stock_id → mgrs, team-codes → country_code).
  Wired into `scripts/gittables_corpus_pass.py --fill-ydf`. v22's
  position shifts from Failed (−8.9% noisy) to Partial (−10.4% gated).
- **YDF validation gate applies pattern AND enum jointly (spec
  `2026-05-26-taxonomy-country-code-enum-cleanup`).** Removed the
  dead `ENUM_SKIP_LABELS` workaround in
  `scripts/apply_ydf_validation_gate.py`. The gate now matches the
  joint semantics already in
  `crates/finetype-core/src/validator.rs::CompiledValidator` and the
  taxonomy's `to_json_schema()`. New regression test
  `ac06_country_code_enum_rejects_state_and_province_codes` pins the
  country_code enum at exactly 249 ISO 3166-1 alpha-2 codes and
  asserts rejection of US state / CA province codes that are NOT in
  ISO (AK, FL, OK, UT, AB, BC, ON, QC, ...) while accepting the
  collision set that IS in ISO (AL=Albania, CA=Canada, MN=Mongolia,
  TN=Tunisia, NL=Netherlands, ...).
- **Broad audit of every `validation.enum` in
  `labels/definitions_*.yaml`** (same spec, ac-02). All 13 universal
  blocks audit clean; no contamination, no duplicates, no deprecated
  members in any locale-keyed enum either. Audit at
  `spec 2026-05-26-taxonomy-country-code-enum-cleanup (enum_audit.md)`.

### Changed

- **CLAUDE.md sprint section refreshed.** m-19 (eval-corpus expansion)
  shipped under successor spec `2026-05-20-gittables-multi-lens-diagnostic`
  per MADR 0087 (13/13 ACs closed). The v18+ retrain block is lifted.
  The corroborated-gaps report at `eval/gittables/corpus_pass/report.md`
  is now the load-bearing input for the next Sense retrain bet.
- **Card 0002 goal updated to v22.** Reflects the v22 training target,
  gated cell-2 numbers, and the long-tail subtypes (full_address,
  street_name, postal_code) deferred pending a different intervention
  class. (The shipped default stays v19 until v22 is published — see
  Fixed.)

### Closed without shipping

- **v23-sharpen-code-discriminator (spec
  `2026-05-26-v23-sharpen-code-discriminator`).** R26 country_code
  Sharpen rule reverted in commit `000b2cd`. ac-01 audit revealed
  premise invalidation: of YDF-labeled `iso6346` cols (the "biggest
  regression" at +40.3%), ZERO match the iso6346 regex — they're
  dominated by `msg_id` columns that YDF mislabeled. R26 shipped to
  a 3-column footprint after anti-collision guards; not worth
  keeping. Surfaced the need for the gated baseline (which the next
  spec delivered).
- **v23-precision-retrain (spec
  `2026-05-27-v23-precision-retrain`).** Closed Failed at ac-04 →
  ac-05 Path C. FP-rate component Met (top-6 corroborated clusters
  dropped **−70.8%**, three by 92–96%), but cell-2 component Failed:
  v23 gated cell-2 vs v19 regressed to **+5.1%** (worse than v19
  baseline). v22's monotone-mover gains all collapsed: country
  **+70.3%**, region +29.0%, city +14.1%. Mechanism: opt-in
  `--include-column-level-types` let the 50k
  `representation.discrete.categorical` hard negatives train the
  model to fire categorical on 548k columns (+529.6% vs v22), drawing
  ~48k of those from columns v22 classified as
  `geography.location.city`. v22 remains the default; v23 candidates
  (`models/sherlock-v23-precision-relu-s{42,43,44}`) stay in tree for
  diagnostic access. Full post-mortem at
  `output/v23-precision-retrain/relitigation_memo.md`. Successor
  bet: spec `2026-05-29-cluster-reachability-scoring`.
- **cluster-reachability v3 / safety_score (spec
  `2026-05-31-reachability-safety-score`).** Ships the v3
  `safety_score` advisory column on
  `eval/gittables/corpus_pass/corroborated_gaps.parquet` and into
  the cluster headers of `eval/gittables/corpus_pass/report.md`
  (format: `Rank #N — gap_id… — N cols — action: ... — safety: 0.NN`).
  Algorithm: `safety_score = clip(1 - risk, 0, 1)` where `risk` is
  v2's risk term (mean fraction of cluster columns' 100 NN where
  ydf disagrees with correct_label AND sense also disagrees), over
  v2's 50,000-row stratified neighbour pool. Bands: ≥ 0.80 HIGH
  (safe candidate), 0.50–0.80 MODERATE (requires the Sense-
  distribution pre/post check), < 0.50 LOW (prefer Sharpen rule
  or taxonomy). v23 fixture illustration: integer_number-target
  clusters score 0.91–0.95 (HIGH), categorical-target clusters
  span 0.43–0.70 (LOW to MODERATE) — correctly flagging the
  categorical-bleed risk that the v23 retrain didn't see. v3 ships
  as **advisory** rather than blocking gate; one labelled retrain
  outcome (v23) is illustration not validation, so true validation
  accrues from the next two or three retrain bets. CLAUDE.md
  paragraph updated; `safety_score` is an input to the Sense-
  distribution pre/post check, which stays mandatory.
- **cluster-reachability-scoring v2 (spec
  `2026-05-30-reachability-metric-v2`).** Closed Path C (Mismatch)
  at ac-04. v2 implemented the redesign memo's neighbour-label
  composition algorithm (absorption × (1 - risk) over 100-NN from
  a 50,000-row stratified pool). v23 fixture: six crossovers.
  Both `gender_code → categorical` (rank 5) and
  `alphanumeric_id → categorical` (rank 6, score 0.173) sit below
  all three LOW-expected clusters (boolean.binary 0.856, url
  0.824, periodicity 0.468). Root cause: the absorption term
  tracks structural correct_label density rather than cluster
  specificity — integer_number-target clusters dominate the
  ranking regardless of cluster-specific risk. v3 redesign at
  `output/cluster-reachability/redesign_memo_v3.md` proposes
  splitting the score: drop absorption, keep risk, ship
  `safety_score` (= 1 - risk) as an advisory column rather than a
  hard gate. v2's `cluster_scores_v2.parquet` ships as reference;
  `reachability_score` column is NOT folded into corroborated_gaps.
  CLAUDE.md's `training_data_addition` paragraph extended to cite
  both v1 and v2 Path C closures + the v3 redesign pointer; the
  interim Sense-distribution pre/post check requirement STAYS until
  v3 ships.
- **cluster-reachability-scoring v1 (spec
  `2026-05-29-cluster-reachability-scoring`).** Closed Path C
  (Mismatch) at ac-04. The v1 metric (char 3-gram + length + char-
  class embedding, cosine distance, tightness × specificity)
  correctly ranked four of the six v23 fixture clusters but
  mis-scored two: `datetime.offset.utc → integer_number` (HIGH-
  expected) landed at rank 6 / 6, below all three LOW-expected
  clusters. Root cause: the utc cluster's actual values are
  predominantly all-zero integer columns Sense mislabelled as
  `datetime.offset.utc`, so the cluster IS the integer_number
  population — specificity reads "indistinguishable from
  correct_label" as risk when in fact it means training will be
  absorbed harmlessly. v1 score is NOT folded into
  `corroborated_gaps.parquet`. Redesign at
  `output/cluster-reachability/redesign_memo.md` proposes a v2
  neighbour-label composition metric (per cluster column, fraction
  of 100 NN whose ydf_prediction equals correct_label) with worked
  predictions against the v23 fixture. Until v2 ships, CLAUDE.md's
  architectural-direction section requires every
  `training_data_addition` retrain to include a Sense-distribution
  pre/post check on the correct_label and its neighbours.

### Fixed

- **Gate-script alignment with codebase joint pattern+enum semantics**
  (spec `2026-05-26-taxonomy-country-code-enum-cleanup` ac-05). The
  gate's `_compile_spec` previously picked one validation kind in
  priority order (pattern > enum > locale > range), diverging from
  `CompiledValidator`. Now applies pattern AND enum together when
  both attached. Cell-2 delta from the alignment was −6 columns
  refused; v22's headline cell-2 unchanged.

### Infrastructure

- **`--include-column-level-types` flag in
  `scripts/prepare_multibranch_data.py`.** Opt-in (default OFF
  preserves all prior retrain behaviour). When set, distilled rows
  whose `final_label` is in `COLUMN_LEVEL_TYPES` (categorical,
  ordinal, increment) are kept rather than dropped. Used by
  v23-precision-retrain; documented as a known negative-transfer
  risk lever in spec `2026-05-29-cluster-reachability-scoring`.
- **New scripts.** `scripts/extract_v23_hard_negatives.py` (corpus-
  pass-derived hard-negative extractor with MADR 0056 leakage
  firewall integration), `scripts/build_v23_distilled.py` (additive
  blend builder), `scripts/overnight_v23_precision.sh` (v23 training
  pipeline), `scripts/compute_v23_per_cluster_fp_rate.py` +
  `scripts/compute_v23_cell_deltas.py` (ac-04 band components),
  `scripts/compute_per_subtype_trajectory.py` (v19→v22 four-way
  per-subtype trajectory for the direction-review spec).
- **`eval/datasets/sources.yaml`.** Added provenance entry for the
  v23 hard-negative parquet inheriting from the parent
  `dataset://gittables` source.

## [0.6.20] - 2026-04-29

### Added

- **`make validate-corpus` round-trip precision harness (spec
  `2026-04-28-validate-precision-corpus`, card 0014)** — A new
  `make validate-corpus` target shells `finetype profile -o json-schema`
  and `finetype validate --db --table` over every CSV in
  `eval/datasets/validate_manifest.csv`, computes per-dataset round-trip
  pass rate at P=99%, and writes
  `eval/eval_output/validate_corpus.md` with a corpus headline
  (`N of M datasets pass at P=99%`), per-mechanism breakdown
  (`enum_overfit | format_diversity | misclassification | code_vs_canonical
  | unknown | no_gt`), and per-dataset / per-column attributions. Iter-1
  ships 7 fresh datasets sourced from public open-data URLs (pokemon,
  rio2016_athletes, us_baby_names, co2_emissions_by_nation,
  world_population, un_locode, global_temp_annual) under
  `eval/datasets/validate_corpus/csv/`, all with full per-column GT
  sidecars. The harness is local-run-only (no CI gate this iteration);
  `make eval-report` surfaces the headline alongside profile-eval and
  actionability-eval. See MADRs 0072, 0073, 0074.

- **Validate-corpus integration with the m-19 leakage firewall** —
  `eval/datasets/sources.yaml` `role` enum extended from
  `{eval, train, both-forbidden}` to include `validate`;
  `scripts/compute_row_hashes.py` aggregates row hashes from BOTH
  `eval/datasets/manifest.csv` AND
  `eval/datasets/validate_manifest.csv` so validate-corpus rows are
  forbidden in training data via the same MADR 0056 mechanism. New
  flags `--validate-manifest`, `--no-validate`, `--dry-run` on
  `compute_row_hashes.py`. Test coverage at
  `scripts/eval_leakage/test_validate_corpus_firewall.py`.

### Changed

- **`finetype profile --enum-threshold` default lowered 50 → 32**
  (`crates/finetype-cli/src/main.rs`, ac-09 sub-fix a). More
  conservative enum emission across all profile output formats
  (json-schema, json, csv, markdown). Users who pass
  `--enum-threshold N` explicitly are unaffected. The label-family
  gate (label must be enum-eligible) is unchanged.
- **JSON Schema enum-emission gate extended for boolean labels**
  (ac-09 sub-fix b) — `representation.boolean.terms`,
  `representation.boolean.binary`, and `representation.boolean.initials`
  now receive enum emission alongside the pre-existing
  `representation.discrete.categorical` gate, since they are
  taxonomically finite-domain. The shared library primitive lives at
  `crates/finetype-cli/src/enum_emission.rs` so the binary and the
  integration tests exercise the same gate (no drift).
- **Taxonomy validator widening — `representation.numeric.decimal_number`
  accepts scientific notation** (ac-10) — Pattern widened from
  `^-?[0-9]+(\.[0-9]+)?$` to
  `^-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$` so values like `6e-04` and
  `1.5E+10` validate alongside plain decimals. Surfaced by
  `us_baby_names.percent` in the iter-1 baseline. The widening still
  rejects clearly-invalid input (`abc`, `1.2.3`, `12px`, `1e`, `e5`) —
  precision principle (MADR 0001) preserved. Regression tests at
  `crates/finetype-core/tests/precision_widenings.rs`.

- **Validate-corpus iter-3 — mechanism-attribution refactor (spec
  `2026-04-28-validate-corpus-iter3`, card 0014)** — The harness's
  attribution cascade is rewritten as 7 explicit rules emitting
  `(Mechanism, &'static str)` tuples (mechanism + 6 trigger labels:
  `path-a-pattern`, `path-b-prefix`, `path-b-codetype`,
  `enum-constraint`, `prediction-error`, `fallthrough`). Per-column
  table in `eval/eval_output/validate_corpus.md` gains a `Trigger`
  column. New analyst-facing doc at `docs/mechanism-attribution.md`
  explains the four buckets, their triggers, examples, and fix paths.
  Code-typed allowlist (`CODE_TYPED_LABELS`, 38 entries) tested for
  taxonomy validity. Per-`(dataset, column)` fixture at
  `eval/datasets/validate_corpus_expected_attributions.yaml` (80 rows:
  5 Phase 1 hand-authored anchors + 75 Phase 2 harness-derived) is the
  anti-regression lock; `pending_escalation` flag documents
  known-pending rows (REF_AREA model collision, GICS Sector taxonomy
  gap). 21 `vci3_*` tests cover positive/negative attribution,
  cascade order, allowlist validity, fixture parity, and anchor
  shape. MADRs 0075 (bucket coalesce), 0076 (fixture lock), 0077
  (label-only attribution).

- **Validate-corpus iter-4 — `finance.currency.amount` bare-decimal
  widening (spec `2026-04-29-validate-corpus-iter4`, card 0014)** —
  `finance.currency.amount`'s `validation.pattern` at
  `labels/definitions_finance.yaml:131` gains a 4th alternation
  borrowed verbatim from `representation.numeric.decimal_number`'s
  canonical pattern at `definitions_representation.yaml:79`:
  `^-?[0-9]+(\.[0-9]+)?([eE][+-]?[0-9]+)?$`. Closes the
  format-diversity gap surfaced by the discovery vehicle
  `eval/datasets/csv/ecommerce_orders.csv` — `total_price` reject
  count drops from 63 → 0 (4-digit unsymboled values like
  `1914.96` previously failed the comma-required alternations 1–3).
  iter-3's `vci3_fixture_attribution_regression_match` test continues
  to pass; zero substantive row-level drift on the 12-dataset corpus
  (the regenerated `validate_corpus.md` differs only in the
  `Generated:` timestamp). Three new `vci4_*` regression tests in
  `crates/finetype-eval/src/bin/validate_corpus.rs` cover
  bare-decimal acceptance, format-preservation, and non-money
  rejection. The two misclassification findings surfaced alongside
  (`status` → `datetime.component.periodicity`, `order_id` →
  `finance.securities.sedol`) are recorded as deferred-to-retrain in
  the iter-4 progress.md and tracked on follow-up card 0015. MADR
  0078 records the precedent: validator alternations may compose
  canonical sibling-type patterns (composition over invention),
  with a comment-annotation contract for traceability.

## [0.6.19] - 2026-04-28

### Removed

- **Retire the load verb (v0.6.19, MADR 0071)** — The standalone
  `finetype load` verb is gone. The typed-DuckDB-table output path
  migrates to `finetype validate` with `--db`/`--table`. Migration
  map (verbatim from the spec):

  ```
  finetype load FILE.csv -t TABLE  →
    finetype profile -f FILE.csv -o json-schema > schema.json
    finetype validate FILE.csv schema.json --db out.db --table TABLE
  ```

  The new path adds a JSON-Schema-driven quality gate, TRY-wrapped
  per-column transforms, and a queryable reject sidecar in a single
  pass. `finetype load …` now errors via clap's stock
  unknown-subcommand handler with exit code 2 — no shim, no warning,
  no carve-out. `Commands::Load`, `cmd_load`, `build_load_expr`,
  `build_load_expr_enum`, the orphan `sanitise_identifier`, and 6
  obsolete unit tests deleted. Public CLI surface drops to **5
  verbs**: infer, profile, validate, mcp, taxonomy. See MADR 0071
  (refines MADR 0064).

- **Retire the schema verb (v0.6.19, MADR 0070)** — The standalone
  `finetype schema` verb is gone. JSON Schema export migrates to its
  two natural homes:

  - `finetype schema KEY` → `finetype taxonomy KEY -o json-schema`
  - `finetype schema FILE.csv` → `finetype profile -f FILE.csv -o json-schema`

  `finetype taxonomy` gains a positional `KEY` argument with the same
  exact-match-or-glob predicate the retired verb used, plus
  edit-distance suggestions on unknown keys. The new `json-schema`
  output format always emits a JSON array (even for single matches),
  matching `taxonomy`'s other output formats. Pretty-printing is
  unconditional — the old `--pretty` flag is gone. The MCP `schema`
  tool's type-key branch is retained for v0.6.19; the v0.6.20 audit
  will mirror the CLI fold. See MADR 0070 (supersedes MADR 0031).

### Changed

- **`finetype validate --db --table` now materialises typed columns
  (v0.6.19, MADR 0071)** — Valid rows land in a typed DuckDB table
  whose column types are driven by the schema's `x-finetype-label` per
  column. Per-column transforms are applied via TRY-wrapped projection
  built by the new `build_transform_projection` helper. Cells that
  pass JSON-Schema validation but fail the typed cast (e.g.
  `"2024-02-30"` matches a date pattern but `strptime` rejects it) are
  detected pre-CTAS and routed to the reject sidecar instead of
  crashing the CTAS or silently NULL-coercing. Staging-NULL → typed-NULL
  is **not** a transform failure — the predicate is `col IS NOT NULL
  AND TRY(transform) IS NULL`. SQL `CAST(col AS VARCHAR)` is the
  documented escape hatch when typed columns aren't wanted; there is
  no `--no-transform` flag. Unlabelled columns pass through as VARCHAR
  (graceful-degradation contract, MADR 0064 ac-11).

- **ENUM emission dropped from `validate --db --table` (v0.6.19,
  MADR 0071)** — The retired `finetype load` verb optionally promoted
  low-cardinality columns to `CREATE TYPE … AS ENUM` declarations
  before the CTAS. `finetype validate --db --table` produces no
  `CREATE TYPE` regardless of column cardinality. Low-cardinality
  columns retain the schema's `duckdb_type` (typically VARCHAR). The
  `--enum-threshold` flag is **not** carried over to `validate`. Users
  who want enum semantics should declare them explicitly in the JSON
  Schema's `enum` keyword and let DuckDB's `IN` constraints do the
  work.

- **Reject ontology gains TRANSFORM_FAILED + transform (v0.6.19,
  MADR 0071)** — The 13-column `finetype_reject_errors` schema
  (REJECT_SIDECAR_DDL) is unchanged. Two existing fields take new
  enum values:

  - `error_type` gains `'TRANSFORM_FAILED'` (existing
    `'SEMANTIC_TYPE'` plus the 7-token DuckDB `reject_errors` enum
    are byte-identical).
  - `constraint_failed` gains `'transform'` (existing tokens like
    `'pattern'`, `'enum'`, `'minLength'`, `'minimum'`, etc. are
    byte-identical).

  `error_message` carries a literal `transform_failed: <transform-expr>`
  on TRANSFORM_FAILED rows, exposing the failing transform at query
  time without joining back to the schema. SEMANTIC_TYPE rows
  continue to carry the engine's free-form failure reason.

- **`x-finetype-label` now emitted on type-mode JSON Schema** — The
  pre-existing `schema KEY` verb only emitted `x-finetype-pii` on
  per-type schemas. The new `taxonomy KEY -o json-schema` emits BOTH
  `x-finetype-label` and `x-finetype-pii`, matching the verbosity
  contract from PR #51 / table-mode export. Both surfaces now carry
  both extensions; downstream consumers reading type-mode schemas can
  rely on the label being present.

## [0.6.15] - 2026-04-14

### Fixed

- **Fix `load | duckdb` with reserved-word columns** — `finetype load` generated SQL with `normalize_names=true` in the `read_csv()` call, which renames DuckDB reserved-word columns (`name` → `_name`, `type` → `_type`, `source` → `_source`) before the SELECT clause references them, causing `Binder Error: Referenced column "name" not found`. All column names are now always double-quoted in generated SQL, and `normalize_names` is no longer used. Supersedes decision 0036 (see decision 0047).

### Added

- **Smoke test: `load | duckdb` round-trip** — New smoke test verifies `finetype load` output with reserved-word column names is valid DuckDB SQL.

## [0.6.14] - 2026-04-14

### Fixed

- **Embed multi-branch model in release binary** — The multi-branch model (9.2 MB safetensors + config + label map) was not embedded in the CLI binary, causing `finetype profile` to fail with "Failed to read config.json" when installed via Homebrew or run outside the repo. Added `MultiBranchClassifier::from_bytes()` constructor, embed the 3 model files via `build.rs`, and fall back to embedded bytes when the model directory doesn't exist on disk. All commands now work standalone.

## [0.6.13] - 2026-04-14

### Changed

- **Default model: sherlock-v11** — Multi-branch 4-branch model (char+embed+stats+header) replaces sherlock-v4-sibling as default. Single forward pass per column. Published to HuggingFace (`meridian-online/finetype-model`).
- **Profile eval: 201/227 (88.5% label, 91.2% domain)** — Expanded from 190 to 227 columns across 35 datasets. 6 ground-truth corrections, 3 broken transform fixes, phantom label match cleanup.
- **Sharpen header bugfixes** — Fixed bitcoin address false match, IPv6 routing before IPv4 catch-all, same-category unconditional override, same-domain threshold raised 0.50→0.95, added ICAO/author header hints, guarded date keyword from month-specific formats. Three confirmed eval fixes (phone→ssn, abbreviated_month_date, long_full_month_date).
- **CLI smoke tests updated for multi-branch** — All `infer` tests now use `--mode column` (multi-branch is column-level only). 24 tests passing.

### Added

- **Pure Rust training crate** (`finetype-train`) — Candle-based training with Metal auto-detection. GELU+LayerNorm infrastructure. TUI dashboard with ratatui.
- **Autoresearch infrastructure** — PyTorch training loop + RunPod remote training. Overnight retraining scripts.
- **`/release` skill** — Model publishing to HuggingFace + GitHub binary release workflow.
- **12 capability cards** — Distilled from project docs and codebase for capability tracking.

### Fixed

- **CI fully green** — Fixed cargo fmt drift, clippy `RangeInclusive::contains` lint, `.gitignore` patterns for current model format.
- **Dead code warnings** — Removed unused `IS_HEX_STRING`, `gen_paragraph`, `cross_entropy_loss`, `device` field.

### Discovery

- **Hint/threshold ceiling at 201/227** — Remaining 26 misclassifications are model-level (high-confidence false positives, cross-domain confusion). Rule-based tuning cannot reach them. Retraining needed (decision 0038).

## [0.6.12] - 2026-03-13

### Fixed

- **Sense→Sharpen safety valve** — When Sense was confident but wrong (e.g., 0.999 predicting "Text" for numeric data), and >90% of CharCNN votes were masked out, the valve didn't fire. Now falls back to unmasked votes when masked_out_fraction > 0.9. Fixed earthquake `horizontalError` misclassified as `boolean.initials` instead of `decimal_number`. (#7)

### Added

- **Pipeline tracing with `--verbose`** — `finetype profile --verbose` and `finetype load --verbose` enable debug-level tracing at 6 decision points: Sense prediction, raw votes, mask application, header hint, feature rule, and final result. Zero-cost when inactive. Replaces need for `RUST_LOG`. (decision 0035, #6)

## [0.6.11] - 2026-03-13

### Added

- **`finetype validate` command** — Schema-driven CSV validation as a standalone quality gate. Generates table-level JSON Schema from profiling, then validates rows against it. Per-row error collection, per-column pass rates, file-level quality grade. Outputs `.valid.csv`, `.invalid.csv`, `.errors.jsonl`. Also available via MCP server. (#2)
- **Disambiguation rule F6** — Demote `file.extension` for short (2-3 char) alphabetic codes without dots. Fixed earthquake columns (`magType`, `net`, `locationSource`, `magSource`) misclassified as file extensions. (#4)
- **Multi-label eval mapping** — Coarse eval labels now map to all valid FineType types, fixing measurement gap where correct predictions counted as misses. 275 gt_label entries. (#5)

### Changed

- **README restructured for end users** — Trimmed developer-focused content, fixed broken links, added early release disclaimer. (#1)

### Discovery

- **Learned disambiguator spike** — Extracted 144-dim features for 278 eval columns. Explored replacing rule cascade with a trained model. Removed `id`→`increment` header hint that caused earthquake ID misclassification (decision 0034). (#3)

## [0.6.10] - 2026-03-11

### Added

- **Sibling-context attention training** — Training pipeline for the 2-layer self-attention module over Model2Vec header embeddings. FrozenSense with constant tensors for gradient isolation. Evaluation report documented.
- **Disambiguation rule F5** — Demote `numeric_code` to `integer_number` when values have no leading zeros.

### Changed

- **Rebrand Noon → Meridian** — Updated GitHub org (`noon-org` → `meridian-online`), domain (`noon.sh` → `meridian.online`), HuggingFace org. Product name (FineType) unchanged. (decision 002)
- **Categorical broad_type changed from VARCHAR to ENUM** — More accurate DuckDB type mapping for categorical columns.

## [0.6.9] - 2026-03-11

### Fixed

- **DuckDB `duckdb_type_from_broad_type` missing SMALLINT and 4 other types** — `SMALLINT`, `UTINYINT`, `USMALLINT`, `UINTEGER`, and `UBIGINT` broad types were unmapped, causing DuckDB extension to fall through to VARCHAR. All 5 now correctly map to their DuckDB types.

### Added

- **Hierarchical classification head** — Tree softmax replacing flat 250-class output: 7 domains → 43 categories → 250 leaf types. Multi-level CE loss (λ=0.2/0.3/0.5). Accessible via `--hierarchical` CLI flag. char-cnn-v15-250: 84.2% type, 90.9% domain, 96.5% category training accuracy. Profile eval matches flat baseline at 180/186. Backward compatible — flat head remains default.
- **Sibling-context attention module** — 2-layer pre-norm transformer self-attention (4 heads, 128-dim, 396K params) over Model2Vec column header embeddings. Enriches per-column headers with cross-column context before Sense classification. Architecturally complete but inert until trained — no model artifact means pipeline is unchanged. Multi-column entry point: `classify_columns_with_context`.
- **Sherlock-style features** — FEATURE_DIM 34→36: `has_negative_prefix` (starts with '-' + digit) and `has_percent` (contains '%'). Rule F3 enhanced with negative-prefix guard and dot-variance confidence check for hs_code vs decimal_number disambiguation.
- **Financial header hints** — `price`, `cost`, `salary`, `fare`, `fee`, `toll`, `charge`, `revenue`, `income`, `wage`, `budget`, `expense` now hint to `finance.currency.amount` instead of generic `decimal_number`. Informed by LLM distillation findings.

### Improved

- **Column feature expansion** — FEATURE_DIM 32→34 (has_colon, has_dash). ColumnFeatures struct with mean/variance/min/max aggregation. Rule F4: zero length-variance + all hex + len=40 → git_sha. Rule F3 enhanced with float-parseability Path B.

### Accuracy

- **Profile eval: 96.8% label, 98.4% domain** (180/186 columns, up from 179/186 in v0.6.8). git_sha misclassification fixed by F4 rule.

### Discovery

- **LLM distillation** — Qwen3 8B on 5,359 columns: 97% valid labels, 20% agreement with FineType. Strong on technology domain (62%), weak on representation (17%) and finance (3%). Useful as complementary signal, not standalone teacher. Scaling recommendations documented.
- **Sibling-context attention spike** — Validated 2-layer self-attention over Model2Vec in Candle. 396K params, 112μs–1.3ms latency, single-column graceful degradation.

## [0.6.8] - 2026-03-08

### Improved

- **Profile accuracy: 96.2% label, 98.4% domain** (179/186 columns, up from 178/186). ~30 new header hints for epoch/unix timestamps, age, altitude, duration, attendance, categorical text (language, sport, species, exchange). Cross-domain hardcoded hint override with domain-aware thresholds (0.85 cross-domain, 0.5 same-domain). 7 substring matching bug fixes ("count" vs "country", "address" vs "mac_address", etc.).

### Added

- **Golden integration test suite** — 13 structured Rust integration tests covering `profile`, `load`, `taxonomy`, and `schema` commands. 4 real-world dataset tests (datetime_formats, ecommerce_orders, titanic, people_directory), 3 focused fixture tests (ambiguous headers, numeric edge cases, categoricals), 2 load DDL tests, 2 taxonomy tests, 2 schema tests. Gated with `#[ignore]` for fast dev workflow.

### Discovery

- **Feature-augmented retrain confirmed: keep rules** — Confirmed that feature_dim=0 + expanded header hints outperforms feature-augmented CharCNN (which regresses -1.6pp due to city attractor). Decided item #22: rules over feature-augmented model.

## [0.6.7] - 2026-03-08

### Added

- **Feature-augmented inference pipeline** — 32 deterministic features (parse tests, character statistics, structural patterns) extracted per value and used for post-vote disambiguation. Three rules: F1 leading-zero detection (postal_code/cpt → numeric_code), F2 slash-segment counting (hostname → docker_ref), F3 digit-ratio + dot pattern (decimal_number → hs_code). Features are computed in the Sense→Sharpen pipeline alongside CharCNN classification.
- **CharCNN feature fusion architecture** — `feature_dim` config parameter enables parallel feature vector fusion at the classifier head (fc1 input = total_filters + feature_dim). Backward compatible: `feature_dim=0` (default) preserves existing model behaviour. Training pipeline supports `--use-features` flag.

### Fixed

- **`finetype load` CAST for generic numeric types** — Types like `decimal_number` (DOUBLE) and `integer_number` (BIGINT) were output as bare VARCHAR because `is_generic` conflated classification uncertainty with cast safety. Now broad_type flows directly from taxonomy — all non-VARCHAR types get their CAST applied.

### Accuracy

- **Profile eval: 95.7% label, 97.3% domain** (178/186 columns). Feature disambiguation rules resolved cpt (100%), hs_code (100%), and docker_ref (100%) confusion pairs.
- **Actionability eval: 99.9%** — 232,321/232,541 values transformed successfully.

### Discovery

- **Feature-augmented retrain** — Training CharCNN with feature_dim=32 improves training accuracy +5pp (86.6% → 91.6%) but regresses profile eval -1.6pp due to city attractor from character statistic features. Recommendation: keep feature_dim=0 with post-vote rules. See `discovery/feature-retrain/FINDING.md`.

## [0.6.6] - 2026-03-08

### Added

- **`finetype load` command** — Generates runnable DuckDB `CREATE TABLE AS SELECT` statements from file profiling. Pipe directly into DuckDB: `finetype load -f data.csv | duckdb`. Features: taxonomy transform expressions for typed columns, column name normalization via SQL aliases (default on, `--no-normalize-names` to opt out), trailing `SELECT * LIMIT 10` preview (`--limit N` to control), `all_varchar=true` for FineType-controlled type casting.
- **`profile -o arrow`** — Arrow IPC JSON schema output format, moved from the retired `schema-for` command.

### Removed

- **`schema-for` command** — Retired entirely. Its three output modes are now covered by `load` (runnable CTAS), `profile -o json` (superset with confidence/locale/quality), and `profile -o arrow`. No deprecation period — command was young with no known external consumers.

## [0.6.5] - 2026-03-07

### Fixed

- **Missing taxonomy definitions** — 25 types (10 geography, 15 identity) from the taxonomy expansion were not embedded in the v0.6.4 binary due to uncommitted YAML files. Taxonomy now correctly includes all 250 types.

## [0.6.4] - 2026-03-07

### Added

- **MCP server** — `finetype mcp` subcommand exposing type inference to AI agents via Model Context Protocol. 6 tools (infer, profile, ddl, taxonomy, schema, generate) + taxonomy resources. Built on rmcp v1.1.0, stdio transport, JSON + markdown dual output.
- **Taxonomy expansion to 250 types** — 43 new type definitions across all domains: geography +10 (wkt, geojson, h3, geohash, plus_code, dms, mgrs, iso6346, hs_code, unlocode), technology +11 (ulid, tsid, snowflake_id, aws_arn, s3_uri, jwt, docker_ref, git_sha, cidr, urn, data_uri), identity +15 (icd10, loinc, cpt, hcpcs, vin, eu_vat, ssn, ein, pan_india, abn, orcid, email_display, phone_e164, upc, isrc), finance +3 (figi, aba_routing, bsb), representation +4 (cas_number, inchi, smiles, color_hsl).
- **PII field** — `pii: Option<bool>` on Definition struct, 11 types tagged. `x-finetype-pii` in JSON Schema output.
- **`x-finetype-transform-ext`** — Extended transform metadata in schema output.

### Changed

- **Taxonomy precision cleanup** — Removed 2 low-precision integer-range types: `http_status_code` and `port` (false positives on plain integers). Renamed 7 currency amount types to format-structural names (amount_us→amount, amount_eu→amount_comma, etc.). Old names preserved as aliases.
- **Duration regex** — Expanded to full ISO 8601 spec. `iso_8601_verbose` aliased to `iso_8601`.
- **bcp47 dedup** — Aliased to `locale_code`.

### Model

- **CharCNN v14** — Retrained on 250-type taxonomy (1500 samples/type, 372k total, 10 epochs, 86.6% training accuracy).
- **Sense classifier** — Retrained with 250-type category mappings (87.1% broad accuracy, 78.5% entity accuracy).
- **Model2Vec** — Refreshed type embeddings for all 250 types (750 embeddings × 128 dim).

### Accuracy

- **Profile eval: 95.7% label, 97.3% domain** (178/186 columns) on expanded eval suite with 43 new type columns. 3 new false positives from type overlaps (cpt/postal_code, hs_code/decimal_number, docker_ref/hostname).
- **Actionability eval: 99.9%** — 232,321/232,541 values transformed successfully.

## [0.6.3] - 2026-03-07

### Taxonomy

- **Taxonomy cleanup** — Removed 7 low-precision types, recategorized color types. Net: 216→209 types across 7 domains.
- **Geographic name removal** — Renamed 10 types from locale-based names to format-structural names: `eu_slash`→`dmy_slash`, `us_slash`→`mdy_slash`, `american`→`mdy_12h`, `european`→`dmy_hm`, `decimal_number_eu`→`decimal_number_comma`, plus 5 short-form date variants.

### Accuracy

- **Profile eval: 97.9% label, 98.6% domain** (143/146 columns correct) — up from 92.5% after v13 retrain. Five targeted pipeline fixes for entity/geography confusion: hardcoded geo override ignores confidence threshold, person-name hints override location predictions, 20+ entity-name header hints (company, venue, station, etc.), bare "address"→full_address, hardcoded hints apply at low confidence.
- **Actionability eval: 99.3%** — 226,951/228,512 values transformed successfully across 238 columns and 82 types.
- **CharCNN v13** — Retrained on 209-type taxonomy (1000 samples/type, 10 epochs, 88.1% training accuracy).

### Fixed

- Clippy `collapsible_str_replace` and `fmt` compatibility for Rust 1.94 CI.

## [0.6.2] - 2026-03-06

### Added

- **`DdlInfo` API** — new `finetype-core` struct and `Taxonomy::ddl_info` method for DDL-oriented metadata extraction (broad_type, transform, format_string, format_string_alt, decompose). Foundation for schema generation tools.
- **`finetype schema-for` command** — profile a CSV/JSON file and output DuckDB `CREATE TABLE` statement with correct types and inline transformation comments. Supports `--table-name` override and `--output json` for structured schema.
- **`--output arrow` for schema-for** — exports Arrow IPC JSON schema format compatible with arrow-rs and pyarrow. Maps DuckDB types to Arrow DataTypes.
- **`x-finetype-*` extension fields in `finetype schema`** — JSON Schema output now includes `x-finetype-broad-type`, `x-finetype-transform`, `x-finetype-format-string` for programmatic DDL generation.

### Changed

- `finetype schema` output now includes DDL contract fields (x-finetype-*) alongside format strings, enabling direct SQL code generation.

## [0.6.1] - 2026-03-06

### Accuracy

- **Actionability eval: 99.7%** — expanded to cover transform-based types (Tier B: epochs, currency, JSON, numeric). Tier A (strptime formats) 96.2%, Tier B (transforms) 99.8%, combined 99.7% on 80 types across 204 columns.

### Added

- **`profile --validate`** — new CLI flag to run JSON Schema validation per column after classification. Outputs valid/invalid/null counts and validity rates.
- **Quality scores and file-level grades** — `ColumnQualityScore` with type_conforming_rate, null_rate, completeness metrics. File-level grade: A≥95%, B≥85%, C≥70%, D≥50%, F<50%. Available in JSON and markdown output.
- **`--output markdown`** — pipe-separated tables for profile and validate commands. Clean formatting suitable for GitHub issues and documentation.
- **Quarantine samples in validation reports** — up to 5 sample invalid values per column in JSON, markdown, and plain output. Helps users quickly understand validation failures without inspecting full dataset.
- **`format_string_alt` field** — new YAML field for type definitions with alternate format strings (e.g., ISO 8601 with/without fractional seconds). Wired through taxonomy JSON export (`--full --output json`) and schema output.

### Fixed

- **Currency broad_type mismatch** — `amount_us` and `amount_eu` now declare `broad_type: DECIMAL` to match transform output (previously VARCHAR). Fixes schema-for DDL generation.
- **Accounting notation support** — `amount_us` validation and generator now accept parenthesized negatives like `($1,234.56)`.
- **Transform stubs completed** — `julian_date` and `rfc_2822_ordinal` now have working DuckDB transforms and generators, eliminating dead-end definitions.

### Changed

- **Evaluation infrastructure** — eval binaries now test both strptime-based formats and SQL transform-based types, providing comprehensive actionability coverage.

## [0.6.0] - 2026-03-05

### Accuracy

- **Profile eval: 111/116 label (95.7%), 114/116 domain (98.3%)** — with CharCNN-v12 model (216 classes) and targeted pipeline fix.
- **Actionability eval: 96.2%** — 2760/2870 datetime values parse correctly.

### Added

- **Format Coverage expansion — 53 new type definitions** (163→216 types, 33% increase).
  - **40 datetime formats:** 15 timestamps (Apache CLF, syslog BSD/ISO, ctime, W3C DTF, ISO 8601 milliseconds/microseconds/date-only, RFC 3339 nano, SQL microseconds, Unix milliseconds/microseconds), 23 dates (Chinese 年月日, Korean 년월일, Japanese era 令和, dot-separated variants, slash variants with 2-digit year, month-first/day-first with leading zeros, abbreviated month, year-month, year-quarter), 2 periods (quarter, fiscal year).
  - **13 finance formats:** 11 currency (Indian lakh/crore, Swiss apostrophe, Brazilian real, Japanese yen, Chinese yuan, Korean won, Scandinavian comma, accounting parentheses negative, minor unit integer, cryptocurrency, generic symbol), 2 rates (basis points, yield percentage).
  - New YAML categories: `datetime.period.*` (span-based dates) and `finance.rate.*` (rates, not amounts).
- **CLI output format alignment** — `label` field added to JSON output, locale suffix in human-readable output.

### Changed

- **Model: CharCNN v11 → v12** — retrained on 216-type taxonomy with 212k samples (1000/type, 10 epochs, seed 42, 87.97% training accuracy). 44 types graduated from release_priority 1-2 → 3 to include in training data.
- **LabelCategoryMap expanded** — updated for 216 types: temporal 45→85, currency 16→29. New types routed to correct Sense categories for masked vote aggregation.
- **Header-hint location override (Step 7b-pre)** — when a hardcoded header hint points to a LOCATION_TYPE (country/city/state/region/continent) but the prediction is not a location type, the hint overrides directly. Catches Sense misrouting where country names get masked to temporal types.

### Known Issues

- **5 remaining misclassifications** — address→street_address (expected full_address), abbreviated_month_date→long_full_month, airports.name→city (expected full_name), npi→isbn, company→last_name (expected entity_name). Mix of CharCNN limitations and keyword-match ambiguity in header_hint.
- **multilingual.date actionability** — mixed date formats across locales; not addressable without multi-format support.

## [0.5.3] - 2026-03-04

### Accuracy

- **Profile eval: 113/116 label (97.4%), 114/116 domain (98.3%)** — recovered from 110/116 (94.8%) in v0.5.2. Five targeted pipeline fixes: Rule 17 UTC offset guard removal, rfc_2822/rfc_3339/sql_standard header hints before generic catch-all, same-category hardcoded hint override at ≤0.80 confidence, enhanced geography protection using unmasked votes at low Sense confidence, full_address header hint distinguished from street_address.
- **Actionability eval: 97.9%** — up from 95.4% in v0.5.2. rfc_2822_timestamp column now correctly classified (was misrouted to iso_8601 by generic `contains("timestamp")` catch-all). Remaining gap: multilingual.date mixed-format column (known limitation).

### Added

- **Locale Foundation — Layer 1: Validation expansion** — expanded locale-specific validation patterns across three type families.
  - `postal_code`: 14 → 50+ locales. Patterns sourced from Google libaddressinput and CLDR.
  - `phone_number`: 15 → 40+ locales. Patterns derived from Google libphonenumber.
  - `month_name` / `day_of_week`: 6 → 30+ locales. Validation lists from Unicode CLDR v46.0.0.
- **Locale Foundation — Layer 2: Generator expansion** — expanded synthetic training data generators to match validation coverage.
  - `postal_code` generator: 14 → 65 locales with format-aware random generation.
  - `phone_number` generator: catch-all countries promoted to named locales (46 total).
  - CLDR date/time patterns wired into `month_name`, `day_of_week`, and datetime generators (32 locales).
- **CI Sense model download** — `.github/scripts/download-model.sh` now fetches the Sense classifier model from HuggingFace, enabling the Sense→Sharpen pipeline in CI builds.

### Changed

- **Model: CharCNN v10 → v11** — retrained on locale-expanded training data (161k samples, 10 epochs, seed 42, 88.3% training accuracy). Expanded locale coverage in generators provides richer training signal for geography, identity, and datetime types.
- **Header hints refined** — specific rfc_2822, rfc_3339, and sql_standard timestamp hints now take priority over generic `iso_8601` catch-all. Bare "name" header no longer forces `full_name` — lets Sense + CharCNN decide. `full_address` distinguished from `street_address` via header keyword.
- **Same-category hint override** — when a curated `header_hint` and CharCNN prediction share the same `domain.category` (e.g., `datetime.timestamp.*`), the header is authoritative — but only when model confidence ≤0.80 to avoid overriding correct high-confidence predictions.

### Fixed

- **UTC offset misclassification** — Rule 17 guard removed. The `[+-]HH:MM` pattern validator at ≥80% is sufficient; the guard requiring top CharCNN vote to be a time type was too restrictive after v11 retrain.
- **rfc_2822_timestamp misclassification** — was being matched by generic `contains("timestamp")` → `iso_8601` catch-all in `header_hint`. Now matched by specific `rfc 2822` check first. Note: header normalization replaces underscores with spaces.
- **Geography protection enhanced** — when Sense confidence is very low (<0.30), checks unmasked CharCNN votes for location types instead of relying on masked (potentially empty) votes. Recovers correct predictions when Sense misroutes columns.
- **Eval manifest GT correction** — `sports_events.venue` ground truth corrected from "name" to "entity name". Venue names (stadiums, arenas) are entities, not person names.

### Known Issues

- **3 remaining misclassifications** — countries.name (→region, correct domain), world_cities.name (→full_name, Sense misroute), sports_events.venue (→city, expected entity_name). All require model retrain to resolve — CharCNN cannot distinguish geography subtypes from person names via character patterns alone.
- **multilingual.date actionability** — 60 values, 0% parse rate. Mixed date formats across locales; not addressable without multi-format support.

## [0.5.2] - 2026-03-04

### Accuracy

- **Actionability eval: 98.7%** — 2990/3030 datetime values parse via `TRY_STRPTIME`. Up from 96.0%. long_full_month_date now correctly classified.
- **Profile eval: 110/116 label (94.8%), 110/116 domain (94.8%)** — regressed from 117/119 (98.3%) due to CharCNN v10 retrain boundary shifts. 6 misclassifications (utc_offset→excel_format, ean→credit_card_number, 3× name disambiguation, countries.name→full_name). Root cause: model retraining, not logic changes. Follow-up investigation planned for v0.5.3.

### Changed

- **Taxonomy: 164 → 163 types** — two removals, one addition. Net -1.
  - Removed `geography.address.street_number` — validation pattern indistinguishable from `integer_number`, causing false positives on plain numeric columns. Demotion rules in column.rs cleaned up.
  - Removed `identity.person.age` — `CAST(col AS SMALLINT)` identical to `integer_number`. 205 SOTAB false positives at 0.995 confidence. Resolved entirely.
  - Added `representation.identifier.numeric_code` — all-digit VARCHAR codes with leading zeros and consistent length (ISO country numeric 840/036, NAICS, SIC, FIPS, product codes). Preserves leading zeros where integer cast would lose data. Addresses #2 analyst frustration from taxonomy revision research.

- **Model: CharCNN v9 → v10** — retrained on 163-type taxonomy. 161k samples (priority ≥1), 5 epochs, seed 42, 83.6% training accuracy. Model2Vec type embeddings regenerated (489 rows = 163 × 3 FPS). Default symlink updated.

### Fixed

- **Sense LabelCategoryMap** — updated for removed (street_number, age) and added (numeric_code) labels.
- **Measurement type detection** — only height/weight remain in MEASUREMENT_TYPES; age removed.
- **Numeric attractor demotion** — street_number rules eliminated; postal_code remains only numeric attractor.

### Known Issues

- **Profile eval regression under investigation** — 6 misclassifications after v10 retrain. Deferred to v0.5.3 follow-up task for accuracy recovery.

## [0.5.1] - 2026-03-03

### Accuracy

- **Profile eval: 98.3% label (117/119), 100% domain (119/119)** — up from 96.7% (116/120). Six new disambiguation mechanisms: validation-based candidate elimination (JSON Schema contracts reject impossible types), Rule 19 (percentage without '%' → decimal_number), expanded header hints (timezone, publisher, measurement keywords), hardcoded hint priority over Model2Vec, same-domain geo override, geography rescue from unmasked votes.
- **Actionability eval: 96.0%** — 2910/3030 datetime values parse successfully via `TRY_STRPTIME`. Improved from 92.7% via `format_string_alt` support for ISO 8601 fractional seconds.

### Added

- **Finance domain** — 16 new types: IBAN, SWIFT/BIC, ISIN, CUSIP, SEDOL, LEI, ISO 4217 currency codes, currency symbols, currency amounts, and more.
- **Identifier category** — alphanumeric_code, html_content, locale_number added to taxonomy.
- **Pure Rust ML training** — `finetype-train` crate with 4 binaries: `train-sense-model`, `train-entity-classifier`, `prepare-sense-data`, `prepare-model2vec`. All training via Candle, zero Python dependencies. Dual-format `SenseClassifier` supports both Python-trained (MHA) and Rust-trained (simple attention) models.
- **Validation-based candidate elimination** — after vote aggregation, validates top candidates against JSON Schema contracts. Eliminates candidates where >50% of sample values fail validation.
- **Rule 19: percentage demotion** — percentage winner with no '%' in values → decimal_number.
- **Geography rescue** — recovers location types from unmasked CharCNN votes when Sense misroutes location columns.
- **`format_string_alt` taxonomy field** — alternative format strings for types with common variants (e.g., ISO 8601 with optional fractional seconds). Eval tries multiple format strings per type.

### Changed

- **Taxonomy: 163 → 164 types, 6 → 7 domains** — net +3 types (IBAN, currency amounts, html_content, locale_number, alphanumeric_code added; cvv, century, screen_size, ram_size removed). New finance domain with 16 types split from identity.
- **CharCNN v9 model** — retrained on clean 164-type taxonomy (1,000 samples/type). Refreshed Model2Vec type embeddings, Sense + Entity classifiers. `remap_collapsed_label` eliminated — models now natively produce 164-class outputs.
- **Header hints expanded** — timezone, publisher, measurement keywords. Hardcoded hints now take priority over Model2Vec semantic hints.

### Removed

- **Python training scripts** — 11 Python files removed. All training migrated to `finetype-train` Rust crate.
- **`remap_collapsed_label`** — no longer needed; models trained on clean 164-type taxonomy.

## [0.5.0] - 2026-03-01

### Accuracy

- **Sense & Sharpen pipeline** — two-stage column classification. Model2Vec cross-attention predicts broad category (temporal/numeric/geographic/entity/format/text) + entity subtype, then CharCNN votes are masked to category-eligible labels. Safety valve falls back to unmasked when confidence is low. 116/120 label (96.7%), 120/120 domain (100%), 0 regressions vs legacy.
- **Taxonomy consolidation** — collapsed 8 niche types (171→163) with backward-compatible `remap_collapsed_label`. Zero regressions.

### Added

- **`SenseClassifier`** — Candle port of Architecture A (cross-attention over Model2Vec). 6 broad categories + 4 entity subtypes. ~3.6ms/column.
- **`Model2VecResources`** — shared tokenizer/embedding loading across Sense, semantic hints, and entity classifier. Net memory increase: 1.4MB (Sense weights only).
- **`LabelCategoryMap`** — maps all 163 types to Sense categories for output masking.
- **Snapshot learning** — auto-backup before model overwrite, `--seed N` for deterministic training, `manifest.json` provenance.
- **`--sharp-only` CLI flag** — opt into legacy tiered-only pipeline (disables Sense).
- **A/B evaluation infrastructure** — `eval/eval_output/sense_ab_diff.json` comparing Sense vs legacy per-column.

### Changed

- Default CLI pipeline: Sense→Sharpen replaces direct tiered cascade. Falls back to tiered when Sense model absent.
- Taxonomy: 171 → 163 types. 8 niche types collapsed.
- Profile eval expanded: 116/120 label (96.7%), 120/120 domain (100%).
- Test suite: 388 tests (7 core + 98 model + 252 CLI + 31 DuckDB). (was 187 at v0.1.0)

### Fixed

- Model2Vec `encode_batch` L2-normalisation mismatch — batch path now matches individual encoding.
- Geography protection fall-through in Sense pipeline — person-name hints no longer block general hint logic.
- Coordinate disambiguation guard — only fires when coordinate labels have competitive vote share (≥1/3 of top).

## [0.4.0] - 2026-02-27

### Accuracy

- **Entity classifier integration** — Deep Sets MLP classifies columns as person/organization/place/creative_work using Model2Vec value embeddings. When CharCNN votes full_name but column values are non-person entities, demotes to entity_name. Fires as Rule 18 between disambiguation and header hints. Entity demotion guard prevents header hints from overriding data-driven decisions. SOTAB domain: +3.9pp (64.4% → 68.3%), 3,027 columns affected (18.1%). Profile eval unchanged at 113/120
- **Phone validation precision overhaul** — Established Precision Principle: for locale-specific types, only locale-confirmed validation gates confidence signals. Universal validation can reject but cannot confirm. Expanded phone locale patterns with extension suffixes, (0) trunk prefix, ZA locale, slash/en-dash separators. Telephone cardinality demotions: 254 → 24. SOTAB label: +3.0pp (39.5% → 42.5%)
- **Text length demotion (Rule 16)** — full_address predictions with median value length >100 demoted to sentence. 441 columns corrected. SOTAB domain: +1.8pp (62.6% → 64.4%)
- **Duration/TLD disambiguation (Rule 14)** — SEDOL override when ≥50% of values match ISO 8601 duration pattern. TLD added to CODE_ATTRACTORS. SOTAB label: +9.0pp (30.5% → 39.5%), domain: +4.7pp (54.8% → 59.5%)
- **UTC offset override (Rule 17)** — when ≥80% of values match `[+-]HH:MM` pattern, overrides time predictions to datetime.offset.utc. Distinguishes offsets from plain time values by mandatory leading sign

### Added

- **CLI `schema` command** — export JSON Schema for any type, supports glob patterns. `taxonomy --full --output json` exports all 19 fields per type
- **Entity name and paragraph types** — `representation.text.entity_name` and `representation.text.paragraph` added to taxonomy (171 total). Addresses full_name overcall on non-person entities
- **Post-hoc locale detection** — after type classification, runs sample values against `validation_by_locale` patterns. Returns locale with highest pass rate above 50%. CLI JSON output includes `"locale"` field. Works for phone_number (15 locales) and postal_code (14 locales)
- **Expanded locale validation** — added 36 additional locale patterns for day_of_week and month_name (6 locales each). Locale detection re-runs after header hint changes
- **Designation-aware is_generic** — four additive signals: attractor-demoted, boolean, hardcoded list, and taxonomy designation (broad_words/broad_characters/broad_numbers/broad_object). Hardcoded list always applies; designation expands the set further
- **Richer designation metadata** — added `broad_words`, `broad_characters`, `broad_numbers`, `broad_object` designations to taxonomy definitions for disambiguation confidence gating

### Changed

- **Profile eval expanded** — 74 → 120 columns across 21 datasets. 8 new datetime types, improved coverage for geography, identity, and measurement columns. Current: 113/120 label (94.2%), 114/120 domain (95.0%)
- **Evaluation package** — precision per type (🟢≥95%, 🟡80-95%, 🔴<80%), actionability eval (98.7% TRY_STRPTIME success), confidence calibration, overcall analysis for 10 high-risk types. Unified `make eval-report` dashboard
- **CLI batch mode** — `finetype infer --mode column --batch` reads JSONL for bulk column classification. Python eval scripts pipe benchmark columns through CLI for SOTAB/GitTables scoring
- **Retraining regression fix** — restored v0.3.0 models from HuggingFace after non-deterministic retraining caused world_cities.name regression. Snapshot learning safeguards planned

## [0.3.0] - 2026-02-25

### Accuracy

- **Geography-aware header hint** — when Model2Vec maps a "name" column to full_name, new geography protection checks prevent overriding correct location predictions. Two cases: (1) keep model prediction when it's already a location type, (2) rescue attractor-demoted predictions when geography votes exist. Fixes world_cities.name → city. Profile eval 68/74 → 69/74
- **Measurement disambiguation** — age, height, and weight are numerically indistinguishable (small integers in overlapping ranges). When the header provides a specific measurement hint but the model predicts a different measurement type, the header now wins. Fixes medical_records.height_in → height. Profile eval 69/74 → 70/74

## [0.2.2] - 2026-02-25

### Accuracy

- **Locale-aware phone number validation** — per-locale validation patterns (14 locales) for phone_number integrated into attractor demotion Signal 1. Patterns derived from Google libphonenumber (Apache 2.0), embedded in taxonomy YAML. Phone_number added to TEXT_ATTRACTORS enabling demotion of false positives while locale-confirmed predictions are preserved

## [0.2.1] - 2026-02-25

### Accuracy

- **Locale-aware postal code validation** — per-locale validation patterns (14 locales) integrated into attractor demotion Signal 1. Locale-confirmed predictions skip demotion. Patterns sourced from Google libaddressinput (Apache 2.0), embedded in taxonomy YAML
- **Model2Vec threshold tuned** — lowered from 0.70 to 0.65, recovering 12 additional correct semantic matches (timezone, postal codes, status codes, price variants) with one accepted borderline FP (data→form_data at 0.687)
- **Targeted synonyms** — added header hint synonyms for IANA timezone, postal code, URL, HTTP status code, and MIME type to improve column name matching

### Changed

- **Max-sim matching for Model2Vec** — replaced mean-pooled single centroids with K=3 representative embeddings per type using Farthest Point Sampling (FPS). Eliminates centroid dilution from diverse synonyms. `type_embeddings.safetensors` uses interleaved layout `[n_types*K, embed_dim]`; K inferred at load time for backward compatibility with K=1 artifacts. `prepare_model2vec.py` adds `--max-k` and `--legacy` flags

## [0.2.0] - 2026-02-24

### Accuracy

- **Multi-signal attractor demotion** — Rule 14 demotes over-eager specific type predictions (postal_code, cvv, first_name, icao_code) to generic types using validation failure, confidence threshold, and cardinality signals. 17 predictions improved, 0 format-detectable regressions
- **Numeric range validation** — added `maximum: 99999` constraint to postal_code and street_number validation schemas, eliminating false positives on salary, ticket number, and byte count columns

### Changed

- **JSON Schema validation engine** — migrated from hand-rolled regex to `jsonschema` crate (v0.42.1, pure Rust, Draft 2020-12). `CompiledValidator` pre-compiles schemas once; taxonomy caches validators via `compile_validators`. Hybrid strategy: string keywords delegated to jsonschema, numeric bounds handled manually for string→f64 parsing. Enables future `format`, `oneOf`, `if/then` keywords

## [0.1.9] - 2026-02-24

### Added

- **Model2Vec semantic header hints** — column name classification using Model2Vec static embeddings (potion-base-4M, 7.4MB float16) with cosine similarity against pre-computed type embeddings. Threshold 0.70 tuned for zero false positives on generics
- **Unified column-level disambiguation** — consolidated all column disambiguation rules into a single pipeline. Profile eval 55/74 → 68/74 format-detectable correct (+13, 0 regressions)

### Changed

- **DuckDB community extension v0.2.0** — updated with tiered model, 168 types, 19 new DuckDB type mappings
- finetype-core and finetype-model published to crates.io at v0.1.9

## [0.1.8] - 2026-02-18

### Performance

- **30× tiered inference throughput** — group-then-batch processing in `classify_batch` improves from ~17 to ~580 val/sec; flat model ~1,500 val/sec
- **Batched CLI inference** — all model types process in chunks of 128 (was per-value)
- **`--bench` flag** — prints throughput and per-tier timing breakdown to stderr
- **`TierTiming` struct** — public API for per-tier performance measurement

### Accuracy (72.6% → 92.9%)

- **`header_hint_generic` override** — header hints now override generic model predictions (integer, username, phone_number, iata_code, etc.) even when the hinted type isn't in the vote distribution. This single change lifted accuracy by +7.1pp
- **IPv4 disambiguation rule** — dotted-quad pattern detection with 0–255 octet validation
- **Day/month/boolean disambiguation** — value-level rules for day-of-week names, month names, and boolean sub-type normalization
- **Gender expansion** — +22 inclusive values (Non-binary, Other, Prefer not to say, etc.)
- **Expanded header hints** — alpha-2/3 country codes, occupation/job title, IP variants, UTC offset, CVV/SWIFT/ISSN/EAN/NPI, weight/height, OS, subcountry
- **Expanded `is_generic`** — phone_number, iata_code, and increment added
- **Eval scoring interchangeability** — boolean sub-types, time sub-types, geographic hierarchy, timestamp precision

### Fixed

- **Column mode with tiered model** — `--mode column` now works with all model types via `Box<dyn ValueClassifier>`; was char-cnn only, broken since v0.1.7 default change
- **Windows build.rs symlink resolution** — `read_link` fallback now reads `models/default` as plain text file when symlink isn't available (git on Windows checks out symlinks as text files)

### Changed

- **`--model-type` help text** documents performance/accuracy tradeoff (~600 vs ~1,500 val/sec)
- **Windows release target** — `x86_64-pc-windows-msvc` added to release CI matrix
- `download-model.sh` gains `readlink`/`cat` fallback for Windows symlink compatibility
- Release workflow steps use explicit `shell: bash` for cross-platform builds

## [0.1.7] - 2026-02-18

### Added

- **Tiered model graph** as default inference engine — 34 specialized CharCNN models in a hierarchical T0→T1→T2 architecture
- **`ValueClassifier` trait** — polymorphic dispatch enabling both flat `CharClassifier` and `TieredClassifier` through a single interface
- **SI number disambiguation** — improved handling of values with SI prefixes in tiered profile evaluation

### Changed

- Default model: `models/default` → `char-cnn-v5` tiered (was `char-cnn-v6` flat)
- Profile evaluation improved by +4.5 percentage points with tiered model
- Inference engine: single flat classifier replaced by tiered graph dispatch

## [0.1.6] - 2026-02-17

### Added

- **Automated profile-and-compare evaluation pipeline** — benchmark column detection across model versions
- **20 curated benchmark datasets** with 206 ground truth column annotations
- **Machine-readable type mapping** — schema.org/DBpedia → FineType crosswalk for external taxonomy alignment

### Fixed

- **Numeric type disambiguation** — fixed training label mapping bug causing incorrect type resolution

### Changed

- Expanded GitTables 1M evaluation with CharCNN v6

## [0.1.5] - 2026-02-16

### Breaking Changes

- **Boolean taxonomy restructured**: `technology.development.boolean` replaced by three format-specific subtypes:
  - `representation.boolean.binary` — 0/1 values
  - `representation.boolean.initials` — T/F, Y/N (single character, any case)
  - `representation.boolean.terms` — true/false, yes/no, on/off, enabled/disabled, active/inactive (any case)
  - All three map to DuckDB `BOOLEAN` type with normalization support
  - Legacy `technology.development.boolean` label is no longer emitted by the model

### Added

- **3 boolean subtypes** with dedicated generators producing case variants
- **Small-integer ordinal disambiguation** rule for columns like Pclass, ratings
- **30+ column header hints** for domain-specific columns: class/rank/tier, count/qty, survived/alive, ticket/cabin, fare/fee, embarked/terminal
- **Centralized `BOOLEAN_LABELS` constant** prevents label mismatch bugs across disambiguation rules
- **Early-development disclaimer** in README
- **Pre-commit hook** for automated fmt/clippy/test checks before commits
- 11 new tests for column disambiguation, header hints, boolean override behaviour

### Fixed

- **Boolean label mismatch** — `disambiguate_boolean_override` was checking non-existent labels instead of actual model output
- Clippy warnings: `useless_format` in build.rs, `manual_range_contains` in generator.rs, `collapsible_str_replace` in column.rs

### Changed

- **CharCNN v6 model** trained on 169 types (up from 168), 89.15% accuracy
- Default model: `models/default` → `char-cnn-v6` (was char-cnn-v5)
- Taxonomy: 168 → 169 types (net +1: removed 1 boolean, added 3 boolean subtypes)
- Test suite: 213 tests (73 core + 109 model + 31 duckdb), up from 182
- DuckDB normalization: all three boolean subtypes routed to `normalize_boolean()`
- JSON boolean literals now annotated as `representation.boolean.terms` (was `technology.development.boolean`)

## [0.1.4] - 2026-02-16

### Added

- **17 new taxonomy types** expanding coverage to 168 types:
  - Medical identifiers: DEA number, NDC, NPI
  - SI-prefix numbers: `representation.numeric.si_number`
  - Excel custom number format detection: `representation.file.excel_format`
  - Expanded phone number generator with NATIONAL/INTL/E164 formats
  - Expanded address generator with locale-specific format templates
  - Categorical, ordinal, and alphanumeric_id types
  - Name format diversity and designation audit
- **Pattern-gated post-processing** using taxonomy validation patterns for deterministic corrections
- **Column-name header hints** as soft inference signal for ambiguous types
- **Cardinality disambiguation** for low-cardinality columns
- **Per-topic evaluation harnesses** for GitTables 1M
- **GitTables 1M formalized** as standard evaluation benchmark
- **Pre-commit hook** infrastructure with `.githooks/pre-commit` and Makefile setup target
- Embedded taxonomy in binary; developer-only CLI commands hidden from help

### Fixed

- **Port disambiguation false positive** on age/count columns
- Windows build.rs: normalized backslash paths in `include_bytes!()` macros
- Smoke test URL assertion for v5 taxonomy label changes

### Changed

- Taxonomy expanded: 159 → 168 types
- CharCNN v5 model trained on 168 types, 90.09% accuracy
- Default model: `models/default` → `char-cnn-v5` (was char-cnn-v4)
- Dynamic model download from HuggingFace in CI/release workflows

## [0.1.3] - 2026-02-15

### Added

- **7 financial identifier types**: ISIN, CUSIP, SEDOL, SWIFT/BIC, LEI, ISO 4217 currency code, currency symbol
  - Check digit validation: Luhn (ISIN), weighted sum (CUSIP, SEDOL), ISO 7064 Mod 97-10 (LEI)
  - All types include DuckDB transformation contracts and decompose expressions
- **char-cnn-v4 model** trained on 159 types (up from 151) with v4 training data (129K samples)
  - Overall accuracy: 91.62%, Top-3: 99.21%
  - New type accuracy: LEI 96.6% F1, currency_code 94.3% F1, SEDOL 89.9% F1, CUSIP 84.6% F1
- 8 new unit tests for finance identifier generators with known-value verification

### Changed

- Default model updated: `models/default` → `char-cnn-v4` (was char-cnn-v2)
- Taxonomy expanded: 151 → 159 types
- Test suite: 73 unit tests (was 65)

### Known Issues

- `currency_symbol` type has low recall (2.5%) — single Unicode characters ($ € £) are confused with `emoji` by the character-level model. Post-processing rule planned.
- `isin` recall is 49.5% — 12-char ISINs starting with 2-letter country code confused with SWIFT/BIC codes

## [0.1.2] - 2026-02-14

### Added

- **Column-mode inference** with distribution-based disambiguation for ambiguous types
- **Year disambiguation rule** — detects columns of 4-digit integers predominantly in 1900-2100 range
- **Post-processing rules** — 6 deterministic format-based corrections applied after model inference:
  - RFC 3339 vs ISO 8601 offset (T vs space separator)
  - Cryptographic hash vs hex token (standard hash lengths: 32/40/64/128)
  - Emoji vs gender symbol (character identity check)
  - ISSN vs postal code (XXXX-XXX[0-9X] pattern)
  - Longitude vs latitude (out-of-range check for |value| > 90)
  - Email rescue (@ sign check for hostname/username/slug predictions)
- **`finetype profile`** command — detect column types in CSV files using column-mode inference
- **`finetype eval-gittables`** command — benchmark column-mode vs row-mode on GitTables real-world dataset
- **`finetype validate`** command — data quality validation against taxonomy schemas with quarantine/null/fill strategies
- **`models/default`** symlink — CLI now works with default `--model models/default` path out of the box
- **DuckDB extension functions**: `finetype_detail`, `finetype_cast`, `finetype_unpack`, `finetype_version`
- Real-world evaluation against GitTables benchmark: 85-100% accuracy on format-detectable types (2,363 columns, 883 tables)
- **DOI type** — `technology.code.doi` with regex validation and Crossref decompose expression

### Fixed

- Postal code rule no longer false-positives on year columns
- Year detection threshold relaxed from 100% to 80% to handle outliers
- Fixed accuracy number in documentation (91.97%, matching eval_results.json)
- Regenerated training/test data with corrected RFC 3339 format (space separator, not T)
- Profile command output formatting and edge cases

### Improved

- Macro F1 improved from 87.9% to 90.8% via post-processing rules (+2.9 points without retraining)
- ISSN precision: 76% → 100%, recall: 73% → 97%
- Hash recall: 94.3% → 100%
- Emoji and gender symbol both reach 100% precision and recall
- Year generator range widened from 1990-2029 to 1800-2100

### Changed

- README.md comprehensively updated with all 9 CLI commands, 5 DuckDB functions, column-mode docs
- DEVELOPMENT.md deprecated in favour of README + backlog tasks
- Column-mode disambiguation rules: date slash, coordinate, numeric types (port, increment, postal code, street number, year)
- Test suite expanded: 155 tests (65 core + 62 model + 28 CLI)
- Homebrew formula auto-updated on release via CI workflow

## [0.1.1] - 2026-02-13

### Added

- **Embedded model** in CLI binary — `finetype infer` works standalone without external model files
- **Published to crates.io** — finetype-core and finetype-model available as Rust library crates
- **Published to HuggingFace** — model weights hosted at noon-org/finetype-char-cnn
- **CI model download** — release and CI workflows fetch model from HuggingFace instead of bundling in git
- **CLI smoke tests** for release validation

### Changed

- Build system: model weights embedded via `include_bytes!()` in build.rs
- CI/release workflows updated to download model before build

## [0.1.0] - 2026-02-11

### Initial Release

FineType is a semantic type classification engine for text data. Given any string value, it classifies the semantic type from a taxonomy of **151 types** across **6 domains**.

### Features

- **151 semantic types** across 6 domains: datetime (46), technology (34), identity (25), representation (19), geography (16), container (11)
- **Locale-aware taxonomy** with 16+ locales for dates, addresses, phone numbers
- **Flat CharCNN model** (char-cnn-v2): 91.97% test accuracy on 151 classes
- **Tiered hierarchical model**: 38 specialized models (Tier 0 broad type, Tier 1 category, Tier 2 specific type), 90.00% test accuracy
- **CLI commands**: `infer`, `generate`, `train`, `eval`, `check`, `taxonomy`
- **DuckDB extension** with embedded model weights — `finetype()` scalar function
- **Pure Rust** with Candle ML framework (no Python dependency)
- **Synthetic data generation** with priority-weighted sampling (500 samples/type default)
- **Taxonomy validation** via `finetype check` (validates YAML definitions, generators, regex patterns)
- **GitHub Actions CI/CD**: fmt, clippy, test, taxonomy check gates; cross-compile release workflow

### Taxonomy

Each type definition includes:
- Validation schema (regex + optional function)
- SQL transform/cast expression
- DuckDB target type
- Tier assignment for hierarchical models
- Locale assignments where applicable
- Example values and descriptions

### Model Architecture

- **CharCNN**: Character-level CNN with vocab=97, embed_dim=32, num_filters=64, kernel_sizes=[2,3,4,5], hidden_dim=128
- **Flat model**: Single 151-class classifier, 331KB safetensors weights
- **Tiered model**: Tier 0 (15 broad types, 98.02%) -> Tier 1 (5 trained + 10 direct-resolved) -> Tier 2 (32 models, 18 at 100%)

### Performance

- Model load: 66ms cold, 25-30ms warm
- Single inference: p50=26ms, p95=41ms (includes CLI startup)
- Batch throughput: 600-750 values/sec on CPU
- Memory: 8.5MB peak RSS
