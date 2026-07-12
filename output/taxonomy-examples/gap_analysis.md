# Taxonomy examples round-trip — gap analysis

**Scope.** Every taxonomy type carries an `examples` array (surfaced by `finetype
taxonomy -o json-schema`). Those examples are meant to be *representative values*
of the type, so a column of them should classify back as that type. The harness
`scripts/test_taxonomy_examples.py` builds one isolated pure column per type and
runs the real `finetype profile` pipeline (full Sense + Sharpen — **not** `infer`,
which classifies a single value with no distribution and is deliberately weak; that
is why `finetype infer -i "#FF0000"` returns `integer_number` while a *column* of
hex values round-trips to `color_hex`).

**Headline.** 229/249 round-trip (92.0%) on v0.6.47 with each column headed by its
own leaf name. The 20 that do not split into five buckets below. Only ~8 are
worth acting on; the rest are model-blind-by-design or residual categories.

Adjudicated per-type against the taxonomy + guard code (workflow `wf_62d19fcb-004`,
21 agents). `profiled_as` = what a column of the type's own examples classifies as.

**Update (2026-07-12).** Two lines of follow-up landed:
- **`infer` fast-path extended** — 24 value-self-sufficient certainty leaves (emoji,
  uuid, jwt, wkt, aws_arn, iban, …) added to `deterministic_fast_path`, so single-value
  `infer` now resolves them deterministically instead of guessing (`infer -i "❤️"` →
  emoji). Profile round-trip is unaffected (fast-path is infer-only).
- **Weak-example bucket A actioned.** `long_full_month`'s day-first sample fixed → now
  round-trips (baseline 229→230). `weekday_full_month`'s sample was likewise corrected to
  month-first (the example was genuinely wrong), but it still lands on sibling
  `weekday_dmy_full` — a model sibling-confusion, not example-fixable, so it stays a gap.
  `increment`'s samples extended to 6 contiguous integers; it round-trips on real
  monotonic data (`1..10` → increment, 0.979) but the harness *cycles* samples to fill
  rows, which breaks monotonicity — so it remains a harness false-negative, not a taxonomy
  gap. Net acknowledged gaps: 20 → 19.
- **Bucket B real bugs fixed (3 of 4).** `identity.credential.password` orphan → real key
  `identity.person.password` (header_sharpen.rs); `street_name`/`street_suffix` headers no
  longer swallowed into `street_address` (bare-arm split); valid ISINs no longer mislabelled
  `isrc` — new `isin_checksum_recovery` guard uses the ISIN check digit (which the regex-only
  `ceded_leaf_recovery` can't see) to correct the shape-overlap misassertion. All three now
  round-trip (baseline 230→233). Gold no-regression confirmed (847/988 = 0.857; the touched
  labels are ~absent from gold, so efficacy rides round-trip + unit tests). The 4th (`inchi`→
  `plain_text`) is now moot at `infer` via the fast-path; the profile-side query_string
  overlap remains. Net acknowledged gaps: 19 → 16.

---

## A. Weak examples — the example VALUES are wrong (fix `labels/definitions_*.yaml`)

These are the direct answer to "our examples don't match inference": the example
strings themselves are wrong or insufficient. A clean real column of the type
*would* round-trip. Cheapest, highest-signal fixes.

| type | profiled as | root cause | fix |
| --- | --- | --- | --- |
| `datetime.date.long_full_month` | `datetime.date.dmy_space_full` | declares `%B %d, %Y` but sample `"15 January 2024"` is `%d %B %Y` — **byte-identical to sibling `dmy_space_full`'s own sample** | replace with a month-first value e.g. `"June 1, 2000"` |
| `datetime.date.weekday_full_month` | `datetime.date.weekday_dmy_full` | declares `%A, %B %d, %Y` but sample `"Tuesday, 31 December 2019"` is the sibling `weekday_dmy_full`'s `%A, %d %B %Y` — 50% of the column is literally the sibling format | replace with `"Tuesday, December 31, 2019"` |
| `representation.identifier.increment` | `representation.numeric.integer_number` | only 3 samples (`1,2,3`); the `values_form_increment` veto needs ≥5 contiguous to confirm the run, so the label is not held | extend samples to `1,2,3,4,5,6` |

## B. Real bugs — the pipeline emits a wrong or non-existent label

| type | profiled as | root cause | fix |
| --- | --- | --- | --- |
| `identity.person.password` | `identity.credential.password` | **orphan label**: `header_sharpen.rs:662` returns `"identity.credential.password"`, which is not a taxonomy key (`finetype taxonomy identity.credential.password` → "unknown type"). A stale deprecated header-hint (decision 0042). | change the string to `"identity.person.password"` |
| `geography.address.street_name` | `geography.address.street_address` | `header_sharpen.rs` bare `h.contains("street")` arm swallows the `street_name` header before a more specific arm can fire | add `street`+`name`→`street_name` / `street`+`suffix`→`street_suffix` arms first, or gate the bare arm with `!name && !suffix` |
| `finance.securities.isin` | `identity.commerce.isrc` | `ceded_leaf_recovery` re-asserts ISRC over a valid 12-char ISIN (both fixed-width alnum); the ISIN Luhn is never consulted | checksum-gate the ISRC re-assertion on the value FAILING `checksum::isin`, or add ISIN as a competing checksum-gated recovery leaf |
| `representation.scientific.inchi` | `representation.text.plain_text` | `query_string`'s loose `key=value` validator matches `InChI=…` and trips the exactly-one-match ambiguity gate | add an anchored InChI guard (`^InChI=1S?/`, any source label) ahead of `ceded_leaf_recovery`, mirroring `s_expression_recovery` |

## C. Near-miss sibling — header-hint blind spot

| type | profiled as | root cause | fix |
| --- | --- | --- | --- |
| `datetime.timestamp.rfc_2822_ordinal` | `datetime.timestamp.rfc_2822` | values only match the ordinal validator, but the `header_sharpen.rs:625` rfc_2822 arm is ordinal-blind and the header prior overrides value recovery | before returning rfc_2822, `if h.contains("ordinal") { return rfc_2822_ordinal }` |

## D. Model-blind, no recovery guard — certainty-roadmap guard candidates (NOT bugs)

The 244-dim model does not (reliably) predict these leaves; the model lands on a
value-identical attractor (`word` / `numeric_code` / `alphanumeric_id` / `integer`)
and no Sharpen guard promotes it back. Each is a future guard (checksum / membership
/ anchored substance), gate-shipped per decision 0096 — not a quick fix. This bucket
IS the harvest list for the certainty roadmap.

| type | profiled as | proposed guard |
| --- | --- | --- |
| `finance.securities.cusip` | `word` | header `cusip` + ≥90% `checksum::cusip` (mirror `isbn_header_recovery`) |
| `finance.securities.sedol` | `numeric_code` | ≥90% `^[B-DF-HJ-NP-TV-Z0-9]{6}[0-9]$` + `checksum::sedol` |
| `identity.medical.cpt` | `word` | header `cpt`/`procedure_code` + ≥90% `^\d{5}$|^\d{4}[FTU]$` (header-gated — bare 5-digit is the ZIP/numeric attractor) |
| `identity.medical.dea_number` | `alphanumeric_id` | header `dea` + ≥90% `checksum::dea` (examples are valid-checksum DEAs; model picks the alnum attractor, no promote guard) |
| `technology.code.imei` | `integer_number` | header `imei` + 15 digits + ≥90% Luhn |
| `geography.transportation.unlocode` | `word` | ≥90% `membership::unlocode` promote (the existing unlocode guard is demote-only; add the promote twin of `geo_subdivision_membership_promote`) |
| `geography.transportation.hs_code` | `word` | header hs/tariff/harmonized + ≥90% `is_hs_code_format` (header-gated to avoid decimal/year over-emission) |
| `representation.file.excel_format` | `plain_text` | anchored format-token signature (`#`/`0` placeholders, `0.00E+00`, `mm/dd/yyyy`, `h:mm:ss AM/PM`), ≥90% |
| `representation.format.color_rgb` | `word` | anchored `rgb(...)` prefix ≥90% (bare `255,0,0` / `(128,128,128)` stay genuinely ambiguous — also weak examples) |

## E. Expected / by-design — no action

| type | profiled as | why it is fine |
| --- | --- | --- |
| `container.array.whitespace_separated` | `representation.text.plain_text` | validator `^[^\s]+(\s+[^\s]+)*$` matches nearly all prose — not structurally exclusive; deliberately excluded from `CEDED_RECOVERY_LEAVES` |
| `identity.person.username` | `representation.text.word` | residual; `john.smith`/`user_123` are genuinely indistinguishable from `word` without context |
| `representation.discrete.ordinal` | `representation.text.entity_name` | documented residual category — cannot be a flat-softmax class (memory `categorical-is-a-residual-category`) |

---

## Note: run against the CURRENT build

The `finetype` on `$PATH` can lag the working tree. This audit initially ran against
a v0.6.41 PATH install and spuriously failed `sql_minutes`/`iso_minutes` (leaves + guards
added in v0.6.46/47). The harness now defaults to `target/release/finetype` and prints
the binary version; always confirm it reads the version you intend.
