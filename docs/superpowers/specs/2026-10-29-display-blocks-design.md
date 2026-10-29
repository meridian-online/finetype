# How a type's values are shown — design

Status: draft, written while the project owner was away. Every decision the author made on the owner's behalf is marked *Call* and listed again at the end for approval.

## Intent

A FineType definition describes the input side of a type in full: its pattern, its samples, its strptime format, the SQL that casts it, its DuckDB and Frictionless types. It says nothing about output. A caller that charts or tabulates a typed column has to guess how its values should be printed, and the guess is usually wrong in a way the label already answers: a year printed `2,024`, an accounting amount printed `-1234.5`, a rate printed `0.045`.

Frictionless's field `format` is a parsing pattern, and no other standard a datapackage reader follows carries a display format, so FineType states it itself. Each definition whose cast yields a number or a date gains a `display` block: a d3-format specifier for numbers or a d3-time-format specifier for dates and times (the formatting languages Mosaic and Observable Plot read), plus the facts a specifier cannot express.

Brightfield is the first reader. Its approved design for formatting tile axes, grid cells and tooltips from a column's type reads, per label: an optional number specifier, an optional date specifier, whether the value is money, an optional fixed currency, whether the value is a fraction shown as a percentage, and whether it is a size in bytes. Its plan cannot reach the step that reads these until a FineType release carries them.

## Goals

1. Every definition whose cast yields a number, a date, a time or a timestamp carries a valid `display` block, and `finetype check` (the taxonomy gate CI runs) fails when one is missing or malformed.
2. The block holds only what the label implies. Anything that depends on a column's data, such as scale (bn, MiB) or the precision a column's spread needs, stays with the caller.
3. A caller can read every block without linking FineType: from the definition YAML at a release tag, the way Brightfield already reads `transform`, and from `finetype taxonomy -o json`.
4. A Frictionless datapackage FineType emits carries each field's block, so a datapackage reader can format a column without the taxonomy.
5. The block never contradicts the cast: a block that says "fraction" belongs to a label whose transform divides.

## Non-goals

- **Changing a cast.** The percentage label's cast storing a fraction is a separate change (below, Dependencies). This work does not edit any `transform` or `broad_type`.
- **Correcting the twelve currency and rate labels typed `VARCHAR`** whose transforms return numbers. That is separate work; this design gives those labels blocks anyway, keyed off what their transform returns (section 3), so they are ready when their type is corrected.
- **Locale.** Specifiers are written for d3's default English (US) locale. A label whose convention groups differently (the Indian lakh) says so through its currency, not through a specifier d3 cannot express.
- **Currency symbols in specifiers.** See section 2.
- **A new CLI verb or output format.** `profile`, `infer`, `taxonomy`, `validate` and `generate` keep their surfaces; agents reach them through arcform, which shells out to them.
- **Durations.** `datetime.duration.iso_8601` casts to `INTERVAL`, which neither d3-format nor d3-time-format can print. It carries no block.

## Anti-goals

- **A block that lies about a value.** A specifier that prints a currency the label does not fix, a literal `Z` on a value a caller may format in local time, or `%` on a column that stores 45 rather than 0.45 is worse than no block.
- **Precision chosen for the data.** `.1%` versus `.0%` is the caller's choice from the column's spread; a block that hard-codes it makes every column of that label wrong for someone.

## Dependencies

- **The percentage cast stores a fraction** (a separate FineType change, tracked on its own and not yet started in the tree). Today `representation.numeric.percentage` casts `45%` to 45 while `finance.rate.yield` divides by 100. The percentage block (`~%`, fraction) is correct only after that change. Until it lands, the completeness gate (section 4) carries the percentage label on a named pending list; the task that adds its block also removes it from the list, and waits on the cast change. *Call:* this work does not take the cast change in and does not wait for it. Everything else here can merge and release on its own; Brightfield's formatting step needs a release carrying both, which is whichever release first does.

## Approaches

- **A. A `display` block on each definition, validated by `check`, exported by the existing outputs (chosen).** Mirrors how `frictionless` was added: a typed struct on `Definition`, a validator, and a gate that requires it where it applies. The caller reads it wherever it already reads definitions. Cost: 107 definitions edited (28 numeric, 79 temporal).
- **B. A separate `labels/displays.yaml` keyed by label.** Keeps definition files untouched, but splits one type's description across two files, so a renamed or added label can silently lose its display. The gate would have to cross-check two files.
- **C. Derive the display at runtime from `broad_type` and `format_string`.** Nothing to author, but it cannot express money, fixed currency, fraction or bytes, which are the facts Brightfield most needs, and it moves a taxonomy fact into code.

*Call:* A, because the taxonomy is the one place FineType keeps facts about a type, and it is the pattern `frictionless` already set.

## The design

### 1. The block

```yaml
finance.currency.amount_accounting:
  ...
  display:
    number: "(,.2f"      # d3-format specifier
    money: true

representation.numeric.percentage:
  display:
    number: "~%"
    fraction: true

representation.file.file_size:
  display:
    number: ",d"
    bytes: true

finance.currency.amount_lakh:
  display:
    number: ",.2f"
    money: true
    currency: INR

datetime.date.dmy_slash:
  display:
    time: "%d/%m/%Y"     # d3-time-format specifier
```

Fields, all optional individually:

| Field | Meaning |
|---|---|
| `number` | d3-format specifier for the stored number |
| `time` | d3-time-format specifier for the stored date, time or timestamp |
| `money` | the value is an amount of money |
| `currency` | ISO 4217 code, only when the label itself fixes one currency |
| `fraction` | the stored value is a fraction and is shown as a percentage |
| `bytes` | the value is a size in bytes, so a caller may scale it by powers of 1024 |

Rules the validator enforces:

- Exactly one of `number` and `time`.
- `number` parses as a d3-format specifier: `[[fill]align][sign][symbol][0][width][,][.precision][~][type]`, with `type` one of `e f g r s % p b o d x X c` or absent.
- `time` contains only literal text and d3-time-format directives (`%a %A %b %B %c %d %e %f %g %G %H %I %j %L %m %M %p %q %Q %s %S %u %U %V %w %W %x %X %y %Y %Z %%`, each optionally with a `0`, `_` or `-` padding modifier).
- `money`, `currency`, `fraction` and `bytes` need `number`. `currency` needs `money` and is three uppercase letters. `fraction` needs a `%` or `p` type. `bytes` and `money` exclude each other, and `fraction` excludes both.
- Booleans are written only when true. *Call:* an absent flag means false, so a caller reads a missing key and a `false` the same way, and the JSON stays small.
- `number` never contains `$` (section 2).

The block is serialised with these snake_case keys everywhere it appears (YAML, `taxonomy -o json`, datapackage), so a caller copies it verbatim. *Call:* one shape, not a camelCase variant for JSON, because the YAML is what a vendoring caller reads.

### 2. Money and currency

`money: true` marks every `finance.currency.amount*` label. `currency` is set only where the label fixes one: `amount_lakh` (INR) and `amount_apostrophe` (CHF). *Call:* `amount_accounting` gets no currency even though its cast strips only `$`, because `$` names a dozen currencies and its values may carry no symbol at all.

*Call:* the specifier never carries `$`. The issue's example for `amount_accounting` was `($,.2f`; this design writes `(,.2f`. d3's `$` prints the locale's symbol, which is `$` in the default locale, so a block with `$` would print dollars on a sterling column. The symbol a particular file used is data, which the caller reads from the column (Brightfield records it from the raw values and writes `$` into a tick format only for a dollar column).

### 3. Which definitions get a block

A definition needs a block when its cast yields a number or a temporal value. That is decided from two facts, because twelve labels are typed `VARCHAR` while their transform returns a number:

- **numeric** when `broad_type` is `SMALLINT`, `BIGINT`, `DECIMAL` or `DOUBLE`, or the transform ends in a cast to `DECIMAL`, `DOUBLE`, `INTEGER`, `BIGINT` or `SMALLINT`, optionally followed by a division by a constant;
- **temporal** when `broad_type` is `DATE`, `TIME`, `TIMESTAMP` or `TIMESTAMPTZ`.

A numeric definition takes `number`; a temporal one takes `time`. `BOOLEAN`, `UUID`, `JSON`, `POINT`, `INTERVAL` and the remaining `VARCHAR` definitions take none, and a block on one of them is a check failure. *Call:* the gate is strict in both directions so a later `broad_type` change cannot leave a stale block behind.

Exceptions:

- `datetime.component.year` is `SMALLINT` and takes `number: "d"` (no grouping), the format the issue names, not `time`.
- `technology.identifier.snowflake_id` is typed `BIGINT`, but its transform (`epoch_ms((col >> 22) + …)`) returns the ID's creation timestamp. The gate treats it as temporal and it takes `time: "%Y-%m-%d %H:%M:%S.%L"`. *Call:* the block describes what the cast stores, so a caller that casts the column prints a timestamp correctly; an ungrouped `d` would also be unfaithful in d3, since a 17-to-20-digit ID exceeds the largest integer a JavaScript number holds exactly. The gate keeps this as a named override of `broad_type`, so correcting the label's type later removes the override. It is the only label whose `broad_type` and transform result disagree in this way (the twelve `VARCHAR` labels are covered by the transform rule above).

### 4. The gate

`finetype check` (`cmd_check` in `crates/finetype-cli/src/cmd_taxonomy.rs`) and its `cargo test` mirror beside `test_every_definition_has_valid_frictionless` in `crates/finetype-core/src/taxonomy.rs` fail when:

- a numeric or temporal definition has no block, unless it is on the pending list (initially only `representation.numeric.percentage`, until its cast stores a fraction);
- a block fails the rules in section 1, or its kind disagrees with section 3;
- `fraction: true` and the transform does not end in a division by 100 or 10,000. This is the invariant the percentage cast change exists to restore, kept so it cannot regress.

### 5. Number blocks

*Call (the whole table):* the specifiers below. The principle is the label's convention, grouped where the label's values are quantities and ungrouped where they are identifiers or coordinates, with `~` trimming insignificant zeros wherever precision is the caller's choice.

| Label | `number` | Flags |
|---|---|---|
| `finance.currency.amount`, `_comma`, `_comma_suffix`, `_space`, `_code_prefix`, `_multisym`, `_neg_trailing` | `,.2f` | money |
| `finance.currency.amount_accounting` | `(,.2f` | money |
| `finance.currency.amount_lakh` | `,.2f` | money, currency INR |
| `finance.currency.amount_apostrophe` | `,.2f` | money, currency CHF |
| `finance.currency.amount_nodecimal` | `,d` | money |
| `finance.currency.amount_crypto` | `,.8~f` | money |
| `finance.rate.yield` | `~%` | fraction |
| `finance.rate.basis_points` | `~%` | fraction |
| `representation.numeric.percentage` | `~%` | fraction (pending the cast change) |
| `representation.numeric.integer_number` | `,d` | |
| `representation.numeric.decimal_number`, `_comma` | `,` | |
| `representation.numeric.scientific_notation` | `~e` | |
| `representation.numeric.si_number` | `~s` | |
| `representation.file.file_size` | `,d` | bytes |
| `representation.identifier.increment` | `d` | |
| `geography.coordinate.latitude`, `longitude` | `~f` | |
| `identity.person.height`, `weight` | `,~f` | |
| `datetime.component.year` | `d` | |

Notes on the less obvious rows:

- `~%` rather than the issue's `%`: d3 gives `%` a default precision of six, so `%` prints `45.000000%`; `~%` prints `45%`. Precision from spread stays the caller's.
- Basis points are shown as a percentage (125 bps as `1.25%`) because the cast already divides by 10,000 and d3 has no basis-point type. *Call.*
- `amount_crypto` keeps up to eight decimals (one satoshi), trimmed.
- `increment` is a number but ungrouped, since `1,234,567` reads as a quantity, not a row number.

### 6. Time blocks

The `time` specifier writes the value the way the label's source writes it, translated from the definition's strptime `format_string` into d3-time-format. The translation is mechanical for most labels (the directives coincide), with these rules:

- DuckDB `%g` (milliseconds) becomes d3 `%L`; `%f` (microseconds) stays `%f`; `%z` (offset) becomes `%Z`.
- A literal `Z` suffix becomes `%Z`. *Call:* d3 formats in local time unless the caller asks for UTC, so a literal `Z` would claim UTC for a value printed in some other zone; `%Z` prints the true offset in either mode.
- **Compact forms are shown as ISO.** `compact_ymd`, `compact_dmy`, `compact_mdy` and `compact_ym` show as `%Y-%m-%d` (or `%Y-%m`), and `iso_8601_compact` as `%Y-%m-%dT%H:%M:%S`. *Call:* a compact date is a machine encoding, and `20240115` on an axis reads as a number.
- **Epochs are shown as timestamps.** `unix_seconds`, `unix_milliseconds`, `unix_microseconds` and `epoch_nanoseconds` cast to `TIMESTAMP` and show as `%Y-%m-%d %H:%M:%S`. *Call*, for the same reason.
- **Forms d3 cannot write**: `rfc_2822_ordinal` (`15th`) shows as `%d %b %Y %H:%M:%S %Z`; `jp_era_short` and `jp_era_long` (era years) show as `%Y-%m-%d`. *Call.*
- **Labels with no `format_string`** are written by hand: `julian` `%y-%j`, `chinese_ymd` `%Y年%-m月%-d日`, `korean_ymd` `%Y년 %-m월 %-d일`, `period.quarter` `Q%q %Y`.

The translated values are committed into the definitions, not computed at runtime: the YAML is the contract, and the translation script that produced them is a one-off.

### 7. Export

- **The YAML at a release tag** is the primary export. A caller that vendors `transform` (Brightfield's vendor script writes `transforms.json` from `labels/definitions_*.yaml` at the pinned tag) writes the blocks the same way, into a file beside it. The shape a caller should write, keyed by label and holding only labels that have a block:

  ```json
  { "tag": "v0.6.66", "labels": { "datetime.component.year": { "number": "d" }, "...": {} } }
  ```

  *Call:* FineType does not publish its own `displays.json` file. The caller already generates `transforms.json` from the YAML, and a second FineType-maintained artefact would be one more thing to keep in step with the definitions. Extending Brightfield's vendor script is Brightfield's work and is outside this plan.
- **`finetype taxonomy -o json`** includes `display` on each label that has one, in both the short and the `--full` forms.
- **`finetype profile -o datapackage`** writes `x-finetype-display` on each field whose label has a block, holding the block verbatim, beside `x-finetype-label`. *Call:* the issue left this to this design; it is cheap, and it lets any datapackage reader format a column without vendoring the taxonomy. JSON Schema output is unchanged. *Call:* JSON Schema describes validity, and a display format there is noise for its readers.
- The DuckDB extension is unchanged.

## Testing

- Unit tests on the validator: each rule in section 1, accepting and refusing, including `$` refused and a `fraction` with a `f` type refused.
- The gate's `cargo test` mirror over the real `labels/`: every numeric and temporal definition outside the pending list has a valid block of the right kind, and no other definition has one.
- A round-trip test for every `time` specifier: format each definition's first sample's parsed value with the specifier and confirm the result parses back with the definition's own strptime format, except for the labels section 6 deliberately shows in another form and the labels with no `format_string` (checked against their samples by hand-written expectations instead). This needs a d3-time-format implementation in Rust; the test carries a minimal one covering the directives in section 1. *Call:* a dev-only test helper, not a runtime dependency.
- Golden tests: `taxonomy -o json` for one money label shows its block; `profile -o datapackage` on a fixture with a currency and a year column shows `x-finetype-display` on both and on nothing else.

## Left to the builder

- How the numeric-cast detection reads the transform (a regex over its end is enough).
- Where the validator lives (beside `Frictionless::validate` in `taxonomy.rs`, or its own module).
- The order of definitions edited, provided the gate lands after the blocks it requires.

## Calls made on the owner's behalf

1. Approach A, a block on each definition, over a separate file or runtime derivation.
2. Absent flags mean false; one snake_case shape everywhere.
3. No `$` in any specifier, departing from the issue's `($,.2f` example; the symbol is the caller's, read from the data.
4. Only `amount_lakh` (INR) and `amount_apostrophe` (CHF) fix a currency; `amount_accounting` does not.
5. Which definitions take a block is decided from `broad_type` and the transform's final cast, strict in both directions.
6. The number-specifier table in section 5, including `~%` over `%`, basis points as a percentage, and `increment` ungrouped.
11. `snowflake_id` takes a time block (its cast stores a timestamp), through a named override of its `broad_type`.
7. Time specifiers mirror the source's written form, except compact forms and epochs (shown as ISO), Japanese eras (ISO) and the ordinal-suffix RFC 2822 form; literal `Z` becomes `%Z`.
8. No FineType-published `displays.json`; callers write it from the YAML as they write `transforms.json`.
9. `x-finetype-display` in datapackage output, not in JSON Schema output.
10. The percentage block waits on the percentage cast change through a pending list in the gate; this work neither absorbs that change nor waits for it.
