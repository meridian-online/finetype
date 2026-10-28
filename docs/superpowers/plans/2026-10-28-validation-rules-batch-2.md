# Validation rules, batch 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the published validation rule of 42 more types agree with the source that defines each type, without changing which type any column is given except where the source says the old answer was wrong, and close the four follow-ups batch 1 left.

**Architecture:** Batch 1 separated a type's rule into four roles (`docs/ARCHITECTURE.md`, "The four roles of a type's rule"). The shipped model reads a frozen snapshot, so a published-rule edit cannot move it. Each fix here edits the published `validation` block to follow the source, adds a `validation_notes` entry where an accepted form is not the source's preferred one, and registers a discrimination detector in `finetype_core::discrimination::detector_for` wherever the source-shaped rule would confirm values that belong to a neighbouring type. No new mechanism is built; this batch only uses the structure batch 1 shipped.

**Tech Stack:** Rust workspace (finetype-core, finetype-model, finetype-cli, finetype-duckdb), taxonomy YAML under `labels/`, Python eval scripts under `scripts/` (run via `eval/gittables/.venv/bin/python`).

**Spec:** the design is the batch-1 spec, folded into `docs/ARCHITECTURE.md` ("The four roles of a type's rule", and the `profile` note on `validation_pass_rate`). The original is at commit `7ddb919e`, `docs/superpowers/specs/2026-10-26-validation-roles-design.md`. Each type's disagreement, with its sources, is recorded on that type's tracker issue; the **Rule** block in each task below is the decision taken from it.

## Global Constraints

- One cargo build at a time on Hugh's Mac (16 GB). Do not run implementers in parallel, and do not start a build while another session holds the build slot.
- `infer`/`profile`/`taxonomy`/`validate`/`generate` stay stable for `arc mcp`. Every output change is additive.
- Rules stay value-based. No header-based discrimination is added.
- Public repo: no tracker ids, private repo names or local paths in tracked text, commit messages or PR text.
- Gold may fall by at most 2 points cumulatively from 0.850 legacy / 0.853 reframe (batch 1 used none of it). A task that would exceed it stops for Hugh's ruling.
- The corpus-honest gate stays blocking for rule changes. A NO-GO is adjudicated per transition against the defining source, and only Hugh can override it.
- Gold re-labels go through panel-proposes / Hugh-ratifies.
- **Read each type's current rule before editing it, and do not reject a form the old rule accepted unless the source requires it.** Where the source requires it, say so in the commit message.
- **A published rule confirming random text is not a discrimination rule.** If a fix raises a type's `discrimination_ascii` in `random_confirm_rate.tsv`, or raises a recorded baseline exception's rate, the type gets a detector in the same task. Check the token population too: a widened rule on a ceded leaf or an attractor list (`TEXT_ATTRACTORS`, `CODE_ATTRACTORS` in `crates/finetype-model/src/column/value_sharpen.rs`) needs a detector even when the ASCII rate stays low.
- A detector never accepts what the published rule rejects (enforced by `random_confirm_rate.rs`).
- Do not add `validation_by_locale` to a type that lacks it: doing so exempts the type from `schema_validation_gate`.
- Patterns are written in the dialect the taxonomy already uses. Lookahead is in use (`datetime.duration.iso_8601`); inline flags such as `(?i)` are not, and stay out, because the JSON Schema output carries the pattern to ECMA-262 consumers. Spell case-insensitivity as character classes. Non-ASCII characters are written as YAML `\u` escapes so the literal character reaches the published pattern.
- Before each commit, run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` plus the tests the change touches. CI runs the full gate.

## Review Focus

1. **A one-distinct-value column of a fixed type** (40 copies of one value). It keeps its type through the veto and Sharpen, and a detector does not demote it. Pinned by every fix's `<type>_constant_column_keeps_type`, which must use a value the **old** rule rejected, or the test proves nothing.
2. **A ceded leaf whose widened rule now matches a neighbouring column** (a column of 4-digit years against `color_hex`, of bare names against `query_string`, of uppercase words against `protein_sequence`). `ceded_leaf_recovery` must not promote it. Pinned by the `<type>_does_not_recover_<neighbour>` tests named in Tasks 2, 4, 6, 7 and 12.
3. **A type whose own samples failed its rule** (`plus_code`, `color_rgb`, `ip_v6`). Fixing the rule must remove it from `SAMPLES_CHECK_EXCEPTIONS`, and the staleness check from Task 1 fails if it is left listed.
4. **Published expressions on newly accepted forms.** A form the published rule now accepts must evaluate through every expression the type publishes (`transform`, each `decompose` part, `transform_ext`), or the type's `notes` must say it does not. Fourteen of the 42 types have `decompose` parts (coordinates, protein_sequence, color_rgb, color_hsl, data_uri, calver, jwt, version, docker_ref, s3_uri, amount_multisym, cusip, amount_lakh, isbn) and four have `transform_ext` (h3, wkt, coordinates, ip_v6). Pinned by every fix's samples step: `scripts/check_registry_expressions.py` (run in CI) evaluates each `transform` and `decompose` part on every sample, so a new form added to `samples` is checked. `transform_ext` needs the spatial or h3 extension, which that script does not load, so Tasks 3, 4 and 8 check it by hand.
5. **Geography guards that read a type's enum** (`geo_code_membership_vote`, `geo_nonmembership_demotion`). Adding `AE`, `FM`, `MH`, `PW` to the US state enum makes them both state codes and ISO country codes. Pinned in Task 3.

## Rulings made while writing this plan

Hugh sees these when he reviews the plan. Each can be undone and leaves the design unchanged.

- **No new spec.** The design is batch 1's, already in `docs/ARCHITECTURE.md`; this plan applies it. The brainstorm was skipped for that reason.
- **The gate baseline is re-measured on 0.6.64.** The batch-1 baseline (`output/validation-roles/baseline/gate_baseline.parquet`) predates batch 1's own fixes, so measuring against it would mix batch 1's 885 transitions into batch 2's. Task 0 builds a new baseline from the released 0.6.64 binary, which needs no cargo build.
- **Batch 2 also closes four batch-1 follow-ups** (Task 14): `boolean.terms`' Frictionless true/false values, a `currency_code` detector for the `XXX`/`TRL` false positives, a CLDR-based `abbreviated_month` detector, and `ft_validate_text` applying the checksum and case-folded enum checks `finetype validate` applies. They are children of the same parent and use the same structure.
- **Short Plus Codes are not `plus_code`.** A short code (`CWC8+R9`) needs a reference location to mean a place, so a column of them is an address column. The description says so; the rule stays full codes.
- **`query_string` requires at least one `name=value` pair.** The URL Standard's parser accepts any string, which would confirm every word. The rule accepts bare names and empty sequences beside a pair (`debug&page=2`), not on their own (`debug`).
- **`color_hex`'s 4- and 8-digit forms require the `#`.** CSS requires `#` on every form. The bare 3- and 6-digit forms stay accepted (the old rule did) with a `nonstandard` note; extending the bare leniency to 4 and 8 digits would confirm years and CRC32 values.
- **`color_rgb` does not accept Matplotlib float tuples.** `(0.1, 0.2, 0.5)` is a numeric tuple with no colour marker; the description says the type is CSS notation.
- **`protein_sequence` publishes every IUPAC one-letter code, and a detector keeps the old letter set.** With B, Z, U, O (and J if the IUPAC table lists it) the class covers nearly every uppercase letter, so the published rule alone would confirm uppercase words.
- **`ndc` does not accept a bare 12-digit number.** RxNorm's 12-digit 6-4-2 form is accepted hyphenated; unhyphenated it is the shape of a UPC and cannot be told apart.
- **`ein` publishes the unhyphenated form with a `non_canonical` note, and a detector keeps the hyphen.** A bare 9-digit number is any 9-digit number; batch 2 does not make bare-digit columns keep `ein`.
- **`naics` publishes Canada's sectors 41 and 91; its membership set stays the US 2022 list.** The recovery guard therefore still recovers only US codes. A NAICS Canada membership set is a separate piece of work.
- **`ticker` publishes 1–6 digit numeric tickers; a detector keeps the alphabetic shape.** Numeric venues need set-vs-set voting before any recovery can use them.
- **`version` keeps leading zeros (`1.02.3`) with a `nonstandard` note**, under batch 1's ruling for real-world forms with no other home. SemVer forbids them; real columns carry them.
- **`iso_week` keeps one-digit weeks (`2024-W3`) with a `non_canonical` note, and rejects week 00 and 54–99**, which ISO 8601 does not define.
- **`coordinates` drops its Frictionless `geopoint` mapping to `string`.** Frictionless geopoint is longitude-first in every format, and this type is latitude-first. The description's claim that DMS is accepted is removed rather than the rule widened.
- **`street_suffix` publishes the national forms; its detector keeps the old enum plus the added full words.** Two- and three-letter abbreviations such as `BD`, `AV`, `CRT` collide with country, currency and airport codes, so they grade as conforming without counting for discrimination.
- **`geohash` publishes 1–20 characters; its detector keeps 6–20.** One- to five-character geohashes are indistinguishable from short lowercase tokens.
- **`calver` publishes one- or two-digit months and days; its detector requires a two-digit month or a third component**, so `2024.1`, which is also a decimal, does not count for discrimination.

---

### Task 0: Baseline on 0.6.64

No code and no build.

- [ ] **Step 1:** Confirm `finetype --version` on PATH is the Homebrew 0.6.64 binary, and that the gittables corpus root the eval scripts read resolves to the 33,250-file stratified sample.
- [ ] **Step 2:** Run the corpus pass with that binary over the stratified sample into `output/validation-roles/batch2-baseline/`, then join it with the oracle the way the batch-1 baseline was built (the batch-1 ledger, `output/validation-roles/sdd-ledger.md`, Task 0 corpus entry) into `output/validation-roles/batch2-baseline/gate_baseline.parquet`. Several hours. Expected: about 761,709 columns, and a self-check of the gate against itself is GO.
- [ ] **Step 3:** Gold, representative and external headlines for 0.6.64 are already in `output/validation-roles/release-0.6.64/`; copy their headline lines into `output/validation-roles/batch2-baseline/README.md`. Expected: gold 885/1037 reframe.
- [ ] **Step 4:** Record the paths in the tracker comment (`output/` is git-ignored).

### Task 1: Staleness check for `SAMPLES_CHECK_EXCEPTIONS`

**Files:**
- Modify: `crates/finetype-core/src/taxonomy.rs` (tests module; the list is at :38)

- [ ] **Step 1: Write the failing test** `samples_check_exceptions_are_not_stale`: load `labels/`; for each listed key, assert the type exists and at least one of its `samples` fails its published rule, with the message `"{key}: every sample now passes; remove it from SAMPLES_CHECK_EXCEPTIONS"`.
- [ ] **Step 2:** Run `cargo test -p finetype-core samples_check_exceptions`. Expected: PASS on main (all ten are genuine). Prove it can fail by temporarily removing one bad sample locally, then restore.
- [ ] **Step 3:** Commit: `Fail when a samples-check exception no longer fails its samples`.

### The fix tasks (2–13)

Every fix follows the same steps; only its **Rule** blocks differ. A task covers several types, with **one commit per type**.

**Files (each type):** the type's entry in `labels/definitions_<domain>.yaml`; ACCEPT/REJECT cases in `crates/finetype-core/tests/precision_widenings.rs` or `precision_tightenings.rs`; the constant-column test in `crates/finetype-model/src/column/tests/validation_gate.rs`; a detector in `crates/finetype-core/src/discrimination.rs` where the Rule says so; the generator arm in `crates/finetype-core/src/generator/` when samples change; `tests/fixtures/precise_audit.tsv` and `tests/fixtures/random_confirm_rate.tsv` regenerated.

- [ ] **Step 1: Write the failing tests**
  - `<type>_accepts_source_forms` (ACCEPT) and `<type>_rejects_undefined_forms` (REJECT), through `validate_value_for_label`;
  - `grade_value_for_label` assertions for each Noted value;
  - `<type>_constant_column_keeps_type`: 40 copies of the first ACCEPT value **the old rule rejected**. `evaluate_validation_veto` is not vetoed and `schema_fail_demotion` returns `None`;
  - for a detector: `<type>_detector_<what it keeps out>` with the listed values, through `passes_detector_or_rule`;
  - any extra test the Rule names.
- [ ] **Step 2:** Run `cargo test -p finetype-core --test precision_widenings --test precision_tightenings` and `cargo test -p finetype-model validation_gate`. Expected: FAIL.
- [ ] **Step 3:** Edit the YAML (and the transform, samples, description or notes the Rule names). Register any detector in `detector_for`. Add one sample for each structurally new form the rule now accepts (not for Noted-only forms), and update every `transform` and `decompose` part so it returns a non-NULL value on each sample. A cast part the form does not have (a two-part calver's `day`) returns NULL, not an error (`TRY_CAST(NULLIF(REGEXP_EXTRACT(…), '') AS …)`), `notes` say so, and that form is tested through the duckdb CLI in `crates/finetype-cli/tests/build_transform_projection.rs` rather than added to `samples`, because the expression check rejects NULL. String parts keep returning `''` when absent (as `version`'s `prerelease` does on `1.2.3`), so existing samples stay. Length bounds (`minLength`/`maxLength`) that contradict the new pattern are changed or removed.
- [ ] **Step 4:** Run the tests again, then `cargo run -- check`, `scripts/check_registry_expressions.py` when the type has a `transform` beyond a VARCHAR cast or any `decompose` part (it needs `make build-extension`, a second build; otherwise leave it to CI), `cargo test -p finetype-core --test random_confirm_rate` and `cargo test -p finetype-core samples_check_exceptions`. Expected: PASS; no label's `discrimination_ascii` exceeds 0.5 and no baseline exception rose.
- [ ] **Step 5:** If the type has gold rows (`isbn` 31, `h3` 17, `ein` 15, `state_code` 7, `wkt` 4, `iana` 3, `hostname` 2, `csv`, `coordinates`, `ticker`, `naics` 1 each), build the release binary and run `scripts/score_gold_anchor.py` predict + score with `--reframe` (~15 min). Expected: no gold column moves, or each move is a correction by the source, listed in the commit message.
- [ ] **Step 6:** Commit: `<type>: agree with <source>`. Close the type's tracker child with the commit as evidence.

### Task 2: Containers

**Rule — `container.object.json`** (RFC 8259): whitespace, including line breaks, before, after and inside: `^\s*\{[\s\S]*\}\s*$`. ACCEPT: `{"a": 1}`, ` {"a": 1} `, `{\n  "a": 1\n}`. REJECT: `[1, 2]`, `{"a": 1`, `a {}`.

**Rule — `container.object.xml`** (XML 1.0): an optional XML declaration, then either one empty-element tag or a start tag through a closing end-tag, across lines; surrounding whitespace allowed. ACCEPT: `<root/>`, `<br />`, `<?xml version="1.0"?><a>1</a>`, `<a>\n  <b>1</b>\n</a>`. REJECT: `<a>1`, `a <b/>`, `<>`, `<1a/>`.

**Rule — `container.object.yaml`** (YAML 1.2.2): a document may open with the `---` marker, with a comment line followed by a YAML line, or with a top-level sequence (`- item`) on a multi-line value; keys may contain `-` and `.`. ACCEPT: `---\nname: x`, `# c\nname: x`, `- a\n- b`, `my-key.v2: 1`, `name: John\nage: 30`. REJECT: `#hashtag`, `- just a bullet`, `hello world`, `: x`. Notes: replace "DuckDB does not have native YAML support" with "DuckDB core does not read YAML; the `yaml` community extension adds a YAML type and extraction functions."

**Rule — `container.object.csv`** (RFC 4180): fields may be empty, and a quoted field may contain commas, doubled quotes and line breaks: a record is `field(,field)*` with `field = "(?:[^"]|"")*" | [^,"\r\n]*`. ACCEPT: `a,,c`, `"a,,b",1`, `"say ""hi""",2`, `John,30,true`. REJECT: `"unterminated,1`. Detector: published rule **and** the old rule's non-empty-fields shape or at least one quoted field, so `container.object.csv`'s discrimination rate does not rise above its recorded baseline exception (0.9770).

**Rule — `container.key_value.query_string`** (WHATWG URL Standard), per the ruling: at least one `name=value` pair; bare names and empty sequences allowed beside it. ACCEPT: `debug&page=2`, `a=1&&b=2`, `q=`, `name=John&age=30`. REJECT: `debug`, `&&`, `=x`, `hello world`. Extra: `query_string_does_not_recover_word_column`: a ceded-leaf recovery over `["alpha","beta","gamma"] × 14` returns no promotion.

### Task 3: Geography codes

**Rule — `geography.location.state_code`** (USPS Publication 28 Appendix B; ISO 3166-2:AU): `EN_US` enum gains `FM`, `MH`, `PW`, `AA`, `AE`, `AP`; universal pattern becomes `^[A-Z]{2,3}$`; description says two or three letters and gives AU-NSW as the ISO 3166-2 form. Detector: two letters, or one of the `EN_AU` three-letter codes, so three-letter airport and currency codes do not count. ACCEPT: `NSW`, `ACT`, `AE`, `TX`. REJECT: `NSWX`, `N`, `ny`. Extra (Review Focus 5): run `cargo test -p finetype-model geo` and `geo_code_membership_vote`'s and `geo_nonmembership_demotion`'s tests; add `state_code_vote_keeps_uae_country_column`: a country-code column containing `AE` among other ISO country codes still resolves to `country_code`.

**Rule — `geography.index.h3`** (H3 docs; Snowflake output): `^[0-9a-fA-F]{15}$`, `canonical` `{kind: non_canonical, name: lowercase, pattern: "^[0-9a-f]{15}$"}`. Description: the 15-character form is a cell index (mode 1); directed-edge and vertex indexes are not this type. ACCEPT: `881F1D4887FFFFF`, `8928308280fffff`. REJECT: `8928308280ffff`, `8928308280fffffg`. Extra: run `transform_ext` by hand in DuckDB with the h3 extension on `881F1D4887FFFFF`; if it does not accept upper case, it lower-cases first.

**Rule — `geography.format.wkt`** (OGC 06-103r4, 7.2.1 and 7.2.2): keywords case-insensitive (character classes), and `TRIANGLE`, `POLYHEDRALSURFACE`, `TIN` added; the description's keyword list matches. ACCEPT: `Point (10 10)`, `point(1 2)`, `TIN Z (((0 0 0, 0 1 0, 1 0 0, 0 0 0)))`, `TRIANGLE ((0 0, 0 1, 1 0, 0 0))`, `POLYGON EMPTY`. REJECT: `POINTS (1 2)`, `circle(1 2)`. Extra: run `transform_ext` by hand with the spatial extension on `point(1 2)` and the `TIN` and `TRIANGLE` values; any it cannot read are named in `notes`.

### Task 4: Geography coordinates

**Rule — `geography.coordinate.plus_code`** (Open Location Code specification): case-insensitive input with `canonical` `{kind: non_canonical, name: upper case, …}`; full codes of 2–15 digits; padding `0` only before `+`, in pairs, with nothing after `+` when padded; at most 7 characters after `+`; first character within 90° latitude (`[2-9C]`) and second within 180° longitude (`[2-9CFGHJMPQRV]`). Remove the invalid sample `7FG49W00+2V`, and remove the type from `SAMPLES_CHECK_EXCEPTIONS`. Description per the ruling on short codes. ACCEPT: `849VCWC8+R9`, `8FVC9G8F+`, `6P000000+`, `849vcwc8+r9`, `8FVC9G8F+6WXQRJH` (7 after). REJECT: `CWC8+R9` (short), `7FG49W00+2V`, `X49VCWC8+R9`, `8FVC9G8F+6WXQRJHV` (8 after).

**Rule — `geography.coordinate.mgrs`** (MGRS; FGDC-STD-011-2001): band letters omit I and O; UPS references (`A`, `B`, `Y`, `Z` with no zone number); an even count of 2–16 digits: `^(?:\d{1,2}[C-HJ-NP-X]|[ABYZ])[A-HJ-NP-Z]{2}(?:\d\d){1,8}$`. Source requires rejecting odd digit counts and I/O bands; say so in the commit. The 16-digit (1 mm) case is FGDC-STD-011-2001's own special-application example; cite it, and change `notes` from a 1 m maximum to "usually 1 m (10 digits); special applications go to 1 mm (16 digits)". ACCEPT: `18SUJ2348306479`, `18SUJ2348316806479498`, `ZAH1234567890`. REJECT: `18SUJ234`, `18IUJ2348306479`, `18SUJ`.

**Rule — `geography.coordinate.geohash`** (PostGIS ST_GeoHash; BigQuery ST_GEOHASH): published `^[0-9b-hjkmnp-z]{1,20}$`; description says lengths 1–20, 12 or fewer being the common precision. Detector per the ruling: the old class at 6–20 characters. ACCEPT: `dr5r`, `9q8yyk8yuvrs4kz5qqvz` (20). REJECT: `9q8yyk8yuvrs4kz5qqvzq` (21), `dr5a`. Detector keeps out: `dr5r`, `bed`.

**Rule — `geography.coordinate.coordinates`** (DuckDB spatial; Frictionless Table Schema; Paris Vélib' export): separator is a comma with optional spaces or one or more spaces: `^-?[0-9]+(\.[0-9]+)?(?:\s*,\s*|\s+)-?[0-9]+(\.[0-9]+)?$`. `transform_ext` becomes `ST_Point(lon, lat)`. Frictionless maps to `string` per the ruling. Description drops the DMS claim. ACCEPT: `48.8761373390584, 2.3680844979417`, `40.7128,-74.0060`, `35.6762 139.6917`. REJECT: `40.7128,,-74.0060`, `N40 44.0 W73 59.0`. Extra: the transform test casts `48.8761, 2.3681` to lat 48.8761, lon 2.3681, and `transform_ext` places 2.3681 as x.

### Task 5: Street suffix

**Rule — `geography.address.street_suffix`** (UPU thoroughfare compilation; Canada Post): the enum gains the cited national forms for the type's listed locales: `CRT`, `LANE`, `RUE`, `AV`, `BOUL` (Canada), `BVD` (Australia), `BD`, `AV` (France), `str`, `Str.` (Germany), and `Crescent`, `Terrace`, `Close`; take each further addition from the UPU table for a listed locale, cited in the commit. No `validation_by_locale` (Global Constraints). Detector per the ruling. ACCEPT: `CRT`, `BVD`, `str`, `Crescent`, `Street`. REJECT: `Strasse Nord`, `XYZ`. Detector keeps out: `BD`, `AV`, `CRT`.

### Task 6: Text and scientific

**Rule — `representation.text.emoji`** (UTS #51): keycap sequences `[0-9#*]️?⃣` added beside the existing class. ACCEPT: `1️⃣`, `#️⃣`, `1⃣` (unqualified), `😀`. REJECT: `1`, `#`, `a⃣`.

**Rule — `representation.scientific.smiles`** (OpenSMILES): `*` (wildcard atom) and `$` (quadruple bond) added. ACCEPT: `[*]CC`, `*C`, `[Re]$[Re]`. REJECT: `C C`, `C;C`.

**Rule — `representation.scientific.protein_sequence`** (IUPAC one-letter code; NCBI BLAST): `B`, `Z`, `U`, `O`, gap `-`, and `J` if the IUPAC table lists it. Description: the ambiguity codes it accepts. Detector per the ruling: the old class (20 standard letters, `X`, `*`). ACCEPT: `MKVBZU`, `MKV-LLI`, `MKOVL`. REJECT: `MKV LLI`, `MKV1`. Extra: `protein_sequence_does_not_recover_uppercase_words`: `["HELLO","WORLD","BOOK"] × 14` is not kept as protein_sequence by `schema_fail_demotion`.

### Task 7: Colours

**Rule — `representation.format.color_rgb`** (CSS1; CSS Color 3 and 4): `rgb(`/`rgba(` with three integer or three percentage components and an optional alpha (number or percentage), comma-separated (legacy) or space-separated with alpha after `/` (modern). The bare and parenthesised forms the old rule accepted stay. Remove the type from `SAMPLES_CHECK_EXCEPTIONS` (its `rgba(0, 0, 255, 0.5)` sample now passes). ACCEPT: `rgba(0, 0, 255, 0.5)`, `rgb(100% 0% 0% / 50%)`, `rgb(255 0 0)`, `rgb(255, 0, 0)`, `255,0,0`. REJECT: `rgb(255, 0)`, `(0.1, 0.2, 0.5)`, `rgb(100%, 0, 0)` (mixed).

**Rule — `representation.format.color_hsl`** (CSS Color 4; MDN): the modern space-separated syntax with optional `/ alpha`; hue as a number or with `deg`; saturation and lightness as percentages or numbers; decimals; `none`; alpha as number or percentage. Description matches. ACCEPT: `hsl(120deg 75% 25% / 60%)`, `hsl(540 0 0)`, `hsl(120.5, 100%, 50%)`, `hsl(none 100% 50%)`, `hsla(240, 100%, 50%, 0.5)`. REJECT: `hsl(120 75%)`, `hsl(120, 75% 25%)`.

**Rule — `representation.format.color_hex`** (CSS Color 4), per the ruling: `#` with 3, 4, 6 or 8 hex digits, or bare 3 or 6 digits; `canonical` `{kind: nonstandard, name: "leading #", pattern: "^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"}`. Description matches. ACCEPT: `#0000ffcc`, `#f00c`, `#FF0000`. Noted: `FF0000`. REJECT: `0000ffcc`, `f00c`, `#12345`. Extra: `color_hex_does_not_recover_year_column`: `["2019","2020","2021"] × 14` is not promoted by ceded-leaf recovery.

### Task 8: Internet

**Rule — `technology.internet.ip_v6`** (RFC 4291 section 2.2; RFC 5952): the full, `::`-compressed and mixed `x:x:x:x:x:x:d.d.d.d` forms; zone IDs stay out. Remove the type from `SAMPLES_CHECK_EXCEPTIONS`. ACCEPT: `2001:db8:85a3::8a2e:370:7334`, `::1`, `::`, `fe80::1`, `::ffff:192.0.2.128`, `2001:0db8:85a3:0000:0000:8a2e:0370:7334`. REJECT: `1::2::3`, `12:34:56:78:9a:bc`, `::ffff:192.0.2.256`, `2001:db8::g`. Extra: run `transform_ext` by hand on `::1` and `::ffff:192.0.2.128`.

**Rule — `technology.internet.hostname`** (RFC 3696): an optional trailing full stop. ACCEPT: `example.com.`, `localhost`. REJECT: `example..com`, `.example.com`.

**Rule — `technology.internet.data_uri`** (RFC 2397; Fetch Standard): data may be empty, and `base64` is case-insensitive. ACCEPT: `data:,`, `data:text/plain;BASE64,SGk=`. REJECT: `data:`, `data:text/plain`.

**Rule — `technology.cloud.s3_uri`** (AWS CLI S3Uri): an access-point ARN in place of the bucket. ACCEPT: `s3://arn:aws:s3:us-west-2:123456789012:accesspoint/myaccesspoint/mykey`, `s3://my-bucket/data/file.csv`. REJECT: `s3://arn:aws:s3:us-west-2:1234:accesspoint/x`, `s3://A`.

### Task 9: Development and tokens

**Rule — `technology.development.version`** (SemVer 2.0.0), per the ruling: pre-release and build identifiers allow hyphens; leading zeros stay accepted with `canonical` `{kind: nonstandard, name: "no leading zeros", pattern: <SemVer core>}`. `decompose` `prerelease` and `metadata` allow hyphens, so `1.2.3-alpha-1` gives `alpha-1` and `1.0.0+build-7` gives `build-7`. ACCEPT: `1.2.3-alpha-1`, `1.0.0+build-7`, `v1.2.3`. Noted: `1.02.3`. REJECT: `1.2`, `1.2.3-`, `1.2.3-al pha`. Extra: run `version_string_recovery`'s tests (`cargo test -p finetype-model version_string`).

**Rule — `technology.development.calver`** (calver.org), per the ruling: one- or two-digit month and day. Detector: two-digit month or a third component. `decompose` `month` and `day` extract one or two digits (today they extract exactly two, and `CAST('' AS SMALLINT)` errors on `2024.2.9`); `day` is NULL on `2024.1` and `2024.12` instead of an error. Sample added: `2024.2.9`; no two-part sample. Extra: `calver_decompose_day_is_null_without_a_day` in `crates/finetype-cli/tests/build_transform_projection.rs`, modelled on `locale_code_decompose_extracts_language_and_region` (`#[ignore]`, run by CI with `--include-ignored`): on `2024.1` and `2024.12`, `year` and `month` are right and `day` is NULL. ACCEPT: `2024.2.9`, `2024.12.31`, `2024.1`. REJECT: `24.2`, `2024..1`. Detector keeps out: `2024.1`, `2020.5`.

**Rule — `technology.development.docker_ref`** (OCI distribution and image specs): tag at most 128 characters matching `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}`; digests `sha256` (64 hex), `sha512` (128 hex) and `blake3` (64 hex). The old upper-case hex stays accepted. ACCEPT: `app:_dev`, `app@sha512:<128 hex>`, `app@blake3:<64 hex>`. REJECT: `app:.dev`, `app:<129-char tag>`, `app@md5:<32 hex>`.

**Rule — `technology.cryptographic.jwt`** (RFC 7519; RFC 7516): the unsecured JWS form ending in `.` (empty signature) and the five-part JWE compact form (encrypted key may be empty). Description names both serialisations. `decompose` keeps its three JWS parts and gives an empty `signature` for the unsecured form; a five-part JWE is not split (its parts are header, encrypted key, IV, ciphertext and tag), which `notes` say, so no JWE goes in `samples`. ACCEPT: RFC 7519's unsecured example, RFC 7516's JWE example, the existing samples. REJECT: a four-part token, `a.b.c`. Extra: `finetype_core::structure::is_jwt` (`structure.rs:103`) and its guard agree on every ACCEPT value; if the guard decodes the header, both new forms must pass it.

### Task 10: Timestamps and durations

**Rule — `datetime.timestamp.pg_short_offset`** (PostgreSQL ISO output): fractional seconds optional, 1–6 digits; length bounds dropped. Transform parses both with and without a fraction. ACCEPT: `2022-01-06 18:00:00+03`, `1997-12-17 07:37:16-08`, `2022-01-06 18:00:00.5+03`, `2024-01-15 14:30:00.123456-05`. REJECT: `2022-01-06 18:00:00+3`, `2022-01-06 18:00:00.1234567+03`. Extra: run `crates/finetype-model/src/column/tests/datetime.rs`; `datetime_format_refinement` routes each ACCEPT value to `pg_short_offset`, and every ACCEPT value casts.

**Rule — `datetime.duration.iso_8601`** (ISO 8601): a decimal fraction, with comma or full stop, on the smallest component; the alternative format `PYYYY-MM-DDThh:mm:ss`. The transform casts designator forms with fractions (comma normalised); `notes` say the alternative format is accepted but does not cast, unless the transform can parse it. ACCEPT: `P0,5Y`, `PT0.5S`, `PT1H30,5M`, `P0003-06-04T12:30:05`, `P2W`. REJECT: `P0.5Y1M` (fraction not last), `P`, `PT`.

### Task 11: Dates and zones

**Rule — `datetime.date.month_year_abbrev`** (Metadata.NSW; US BLS): upper-case months (`JAN 2024`), `Sept`, and BLS's `June`/`July`; length bounds dropped. Transform normalises case and rewrites `Sept` → `Sep`, `June` → `Jun`, `July` → `Jul`. ACCEPT: `JAN 2024`, `Sept 2026`, `June 2026`, `Jan 2024`. REJECT: `Janu 2024`, `Jan 24`. Extra: run `datetime.rs`; a column of full month-year values (`January 2024`, …) keeps the leaf it gets today, and every ACCEPT value casts.

**Rule — `datetime.date.iso_week`** (ISO 8601), per the ruling: `^\d{4}-W(?:0[1-9]|[1-4]\d|5[0-3]|[1-9])$`, `canonical` `{kind: non_canonical, name: two-digit week, …}`. ACCEPT: `2024-W03`, `2020-W53`. Noted: `2024-W3`. REJECT: `2024-W00`, `2024-W54`, `2024-W99`. Source requires rejecting week 00 and 54–99; say so in the commit.

**Rule — `datetime.date.chinese_ymd`** (gov.cn): zero-padded month and day. Transform casts padded values. ACCEPT: `2024年01月11日`, `2024年1月15日`. REJECT: `2024年13月01日`, `2024年1月`.

**Rule — `datetime.offset.iana`** (tz theory; `etcetera`): name components allow `.`, `-` and `_`, and `Etc/GMT+N` for N in 0–12 and `Etc/GMT-N` for N in 0–14, the zones `etcetera` defines. ACCEPT: `America/Port-au-Prince`, `Etc/GMT+4`, `Etc/GMT-14`, `America/Argentina/Buenos_Aires`. REJECT: `Etc/GMT+13`, `Etc/GMT-15`, `America/New York`, `GMT+4`.

### Task 12: Finance

**Rule — `finance.currency.amount_multisym`** (CLDR cs, pl, da): the symbol may follow the number (`1 234,56 Kč`, `1 234,56 zł`), and `kr.` is a symbol. Samples change to the CLDR order for Czech and Polish. Transform strips a trailing symbol. ACCEPT: `1 234,56 Kč`, `1 234,56 zł`, `kr. 1.234,56`, `Kč 1 234,56`. REJECT: `Kč`, `1 234,56 €` (not a listed symbol). Extra: the transform casts each ACCEPT value to 1234.56.

**Rule — `finance.currency.amount_lakh`** (DICGC): a space after `₹`. ACCEPT: `₹ 5,00,000`, `₹4,99,000`. REJECT: `₹ 5,0,000`.

**Rule — `finance.securities.ticker`** (Wikipedia, Ticker symbol), per the ruling: 1–6 digit numeric tickers. Detector: the old alphabetic shape. ACCEPT: `5`, `0005`, `AAPL`, `BRK-B`. REJECT: `1234567`, `aapl`. Detector keeps out: `5`, `0005`. Extra: `ticker_membership_recovery`'s tests pass unchanged.

**Rule — `finance.securities.cusip`** (CUSIP Global Services; Wikipedia): `*`, `@`, `#` in the positions the source reserves for private placements; `finetype_core::checksum::cusip` (`checksum.rs:127`) values them 36, 37, 38. Description names the Committee on Uniform Security Identification Procedures (ABA, 1964). ACCEPT: a private-placement CUSIP with `*` and a valid check digit, computed in the test. REJECT: the same with a wrong check digit, and a value with `*` in the check-digit position. Extra: cusip is in `CODE_ATTRACTORS`; `cusip_checksum_recovery`'s tests pass unchanged.

### Task 13: Identity

**Rule — `identity.person.gender_code`** (ICAO Doc 9303 part 4): `<` (unspecified) added to the enum; description names the MRZ convention. ACCEPT: `<`, `M`, `9`. REJECT: `<<`, `U`.

**Rule — `identity.medical.ndc`** (21 CFR 207.33; RxNorm), per the ruling: 6-digit labeler forms 6-4-1 and 6-3-2, and the hyphenated 12-digit 6-4-2. ACCEPT: `123456-7890-1`, `123456-789-01`, `123456-7890-12`, `0002-1433-80`. REJECT: `123456789012`, `1234567-890-1`. Extra: ndc is in `CODE_ATTRACTORS`; a column of 12-digit UPCs is still demoted from ndc by attractor demotion.

**Rule — `identity.industry.naics`** (NAICS Canada 2022 v1.0), per the ruling: sectors 41 and 91 in the sector alternation. ACCEPT: `41`, `411110`, `91`, `911110`. REJECT: `40`, `90`. Extra: `naics_industry_recovery`'s tests pass unchanged.

**Rule — `identity.government.ein`** (IRS valid EIN prefixes; IRS SOI extract), per the ruling: `^\d{2}-?\d{7}$`, `canonical` `{kind: non_canonical, name: hyphenated, pattern: "^\\d{2}-\\d{7}$"}`. Detector: the old hyphenated rule. Notes: the first two digits name the IRS campus or online issue; prefix 31 is the Small Business Administration. `minLength`/`maxLength` 10/10 become 9/10. ACCEPT: `12-3456789`. Noted: `010548657`. REJECT: `12-345678`, `1234567890`. Detector keeps out: `010548657`.

**Rule — `identity.commerce.isbn`** (ISBN Users' Manual, 7th edition): spaces as separators, one separator kind per value. ACCEPT: `978 0 13 110362 7`, `0 13 110362 8`, `978-0-13-110362-7`. REJECT: `978-0 13-110362-7`, `978  0 13 110362 7`.

### Task 14: Batch-1 follow-ups

One commit each.

- [ ] **`representation.boolean.terms`'s Frictionless values** (Frictionless Table Schema, boolean): split the enum into true terms and false terms and declare both as `trueValues`/`falseValues`. Test: a `crates/finetype-cli/tests/datapackage_conformance.rs` case asserts both lists, including `yes`/`no`, `on`/`off`, `enabled`/`disabled`, `active`/`inactive`.
- [ ] **`finance.currency.currency_code` detector** (from batch 1's corpus transitions): the detector excludes `XXX`, `XTS` and the `validation_notes.historic` codes, so placeholder and street-type columns are not confirmed; published rule and grades unchanged. Tests: `currency_code_does_not_recover_placeholder_column` (`XXX` × 40) and `…_street_type_column` (`TRL` × 40) give no ceded-leaf promotion; `ceded_leaf_recovery_promotes_a_list_one_currency_column` still passes.
- [ ] **`datetime.date.abbreviated_month` detector** (CLDR abbreviated months for the type's listed Latin-script locales): a value passes when its month token is a CLDR abbreviated month for one of those locales, so `4 February 2023` and `Room 12, 2023` stop counting; the published rule stays source-shaped. Values the transform cannot cast (`4. Feb 2023`, `Feb 4 2023`) are documented in `notes`. Tests: the detector keeps out `4 February 2023`, `Room 12, 2023`; keeps `4 Feb 2023`, `4 janv. 2023`, `4 ene 2023`. Re-measure the type's veto allowlist entry in Task 15.
- [ ] **`ft_validate_text` applies the checksum and case-folded enum checks**: expose `table_validator`'s per-column value check from finetype-core as `pub fn check_value_against_property(value: &str, property: &serde_json::Value) -> ValueCheck`, where `pub struct ValueCheck { pub valid: bool, pub constraint: Option<String>, pub message: Option<String> }` is what `table_validator` already computes per value, and call it from `crates/finetype-duckdb/src/validate.rs`. Test: rename `test_detail_checksum_label_not_applied` to `test_detail_checksum_label_applied` and invert it (a right-shape value with a bad check digit is `valid=false, grade="fails"`); a case-folded enum value conforms. Run `make build-extension` and a SQL smoke.

### Task 15: Gates, accounting, fold and hand-off

No new code.

- [ ] **Step 1:** Build the release binary and run `scripts/eval_rule.sh --label batch2 --out-dir output/validation-roles/batch2 --gate-baseline output/validation-roles/batch2-baseline/gate_baseline.parquet`, plus the representative and external bands, into `output/validation-roles/batch2`. Several hours.
- [ ] **Step 2:** Write `output/validation-roles/batch2/accounting.md` in batch 1's form: headlines; each gold column that moved, with its source; the cumulative gold delta (≤ 2 points or stop for Hugh); every corpus transition onto or off a fixed type, sampled and adjudicated against the source; the gate verdict with any band adjudicated; the veto allowlist re-measured with `scripts/false_veto_sweep.py`, listing joins and exits (`abbreviated_month` and the open `compact_ymd` question included); gold labels proposed for re-label.
- [ ] **Step 3:** Run the panel on any proposed re-labels, and flag the tracker issue `needs-human` for Hugh to ratify them and rule on any NO-GO.
- [ ] **Step 4:** Fold: add the detectors this batch registered to the "Detectors keep discrimination precise" paragraph of `docs/ARCHITECTURE.md`; add a `CHANGELOG.md` entry listing which types now accept or reject which forms; delete this plan.
- [ ] **Step 5:** Run `roborev review --branch --wait`, open the PR, wait for `gh pr checks <pr> --watch`, link it in Forge, and report it ready with the test totals and the rulings above. Brightfield's pin gets a tracker issue to take the release.
