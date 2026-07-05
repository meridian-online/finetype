# Company/financial reference data — weakness audit and action plan

**Date:** 2026-07-04
**Trigger:** An external eval session profiled real-world company/financial reference datasets
(security symbols, industry codes, organisation names, free-text descriptions) and reported four
misclassifications. This audit reproduced them against v0.6.38 (shipped default `m2v8m-s43`),
traced each to an exact mechanism, swept all 245 validators for the same failure class, and
assessed the taxonomy-truncation question.

**Method:** 5 parallel audit passes (empirical repro, validator sweep, machinery trace,
known-gap cross-check, truncation candidates) + adversarial verification of load-bearing claims
+ a completeness critique. ~960k tokens, 11 agents. Fixtures under the session scratchpad
(`repro/*.csv`); every mechanism claim below was verified against source or live binary output.

---

## 1. The four reported failures — repro status and mechanism

### 1a. Ticker → `geography.transportation.icao_code` — REPRODUCED

- 30 four-letter tickers, header `ticker` → `icao_code` (conf 0.66, pass_rate 0.97, no rule fired).
- Full-table repro (symbol + company_name + description + naics_code + employees + founded,
  exercising sibling-context attention) → `symbol` still `icao_code` (conf 0.52, pass_rate 0.85).
  Sibling headers do not pull the column toward finance.
- Header rename to `stock_ticker` RAISES confidence (0.66 → 0.73) — the header branch does not
  know ticker vocabulary.
- Mixed-length and 5-letter tickers escape ICAO but land on **unknown**: Sense emits
  `alphanumeric_id`, whose validator requires a digit, so all-alpha columns hard-veto to unknown.
  **No ticker column has a path to a correct answer** — no ticker type exists in the taxonomy.

**Mechanism (verified):** icao's universal validation `^[A-Z]{4}$`
(`labels/definitions_geography.yaml:1465`) *confirms* 4-letter tickers instead of vetoing them.
Worse, confirmation **disarms the safety rule built for exactly this**: `icao_code` is on
`CODE_ATTRACTORS` (`crates/finetype-model/src/column/value_sharpen.rs:20-25`), but
`sharpen_attractor_demotion` sets `validation_confirmed = true` whenever the label's own pattern
has `fail_rate <= 0.3` (`value_sharpen.rs:633-635`), which skips the confidence demotion
(`:642-649`). A shape-only regex switches off the attractor guard on precisely the collision
families it exists to catch. (At the higher confidences observed on some fixtures, 0.93–0.98,
the <0.85 demotion wouldn't fire anyway — the disarm is decisive for the mid-confidence band,
and the same weak pattern also neutralises the validation veto at every confidence.)

### 1b. NAICS → `integer_number` — REPRODUCED, sharper mechanism than "missing type"

All NAICS fixtures (6-digit, mixed 2–6-digit, in-table) → `representation.numeric.integer_number`
with `disambiguation_rule feature_no_leading_zero:0.00`. Sense actually predicted
`representation.identifier.numeric_code` — the type whose own description names NAICS/SIC as
in-scope (`definitions_representation.yaml:1231`) — and Sharpen rule **F5**
(`crates/finetype-model/src/column/feature_sharpen.rs:62-78`) demoted it: numeric_code with
`leading_zero_ratio < 0.01` → integer. NAICS sectors run 11–92 and never carry leading zeros, so
**NAICS is structurally locked out of its intended type**. The model was right; the
deterministic layer erased it.

### 1c. Org names → `identity.person.full_name` — REPRODUCED (conditionally)

Founder-style org names (Morgan Stanley, Wells Fargo, Charles Schwab…) under header `name` →
`full_name` at conf 0.92, pass_rate 1.0, no veto. Under header `organization` a header-hint rule
rescues them to `entity_name`; with `--no-header-hint` raw Sense says full_name at 0.88 for both.
Suffixed corporate names (`Salesforce, Inc.`) go to entity_name at 0.997 — the failure is
specific to person-derived org names, i.e. much of the financial-services universe.

**Mechanism (verified):**
- full_name's only validation is `^[\p{L}\s'\-.,]+$` (locale_specific with NO
  validation_by_locale) — 84% of a realistic company-name probe passes. It is veto-safe, but the
  validator confirms org names, so the veto is inert.
- `entity_name` has NO pattern at all (length-only) — in any validation-mediated comparison the
  person type is "confirmed" and entity_name never is.
- full_name is deliberately excluded from `TEXT_ATTRACTORS` (`value_sharpen.rs:6-13`; the "false
  positives are rare (2 in eval)" comment is stale — representative-band full_name P=0.167).
- An entity-classifier demotion path (full_name → entity_name) exists at
  `crates/finetype-model/src/column/mod.rs:958-971` but **has no callers** — dead code on the
  multi-branch path.
- Training-side (inferred, plausible): every arm of `gen_entity_name`
  (`crates/finetype-core/src/generator/helpers.rs:352-479`) emits a marker token (suffix, digit,
  "The", "&", institution word); the bare "Capitalised Capitalised" shape is generated only as
  full_name — the learned boundary IS the marker.
- Engine source already documents full_name as "the model's single largest over-emission
  (249,568 corpus columns)" (`mod.rs:1019`; 6.6× the oracle, v19-era substrate — re-baseline
  before using the magnitude), and the designated fix is the negatives retrain
  (task t-000133e418). Note the broad `"* name"→full_name` header arm was retired in **v0.6.37**
  — external runs on older binaries hit this failure even harder.

### 1d. Description → `geography.format.wkt` — NOT REPRODUCED; real latent hole found

Five fixture variants (plain prose, geometry-vocabulary prose, short phrases, `geometry` header,
in-table) on both 0.6.38 and 0.6.25 → all `plain_text`. We could not make Sense assert wkt on
prose. **We need the actual failing column values before spending anything on the Sense side.**

But the audit found the structural hole the report implies: **wkt is not hard-veto-eligible.**
`labels/veto_safe.txt` (71 labels) omits it (rare-type starvation), so
`evaluate_validation_veto` (`crates/finetype-core/src/validation_veto.rs:94-101`) can only raise
`advisory_low`, never null the label. Verified live via `FINETYPE_INJECT_LABEL`: an injected wkt
assertion on a 10-row prose column ships as wkt with pass_rate 0.0 and only an advisory flag.
wkt is also absent from R32 `schema_fail_demotion`'s closed set (`value_sharpen.rs:434-451`)
despite fitting its admission test perfectly. Secondary: the wkt pattern is tail-open — prose
*beginning* `POINT (` validates (`"POINT (of sale) systems"` matches); pure prose fails.

**The same veto-blindness covers the whole collision-prone tail:** the only geography entry in
veto_safe is postal_code — `icao_code`, `iata_code`, `unlocode`, `hs_code`, `wkt`, `geohash`,
`h3`, `mgrs`, `plus_code`, `dms`, `inchi`, `iso6346`, `ndc`, `top_level_domain`, and
**`user_agent`** can never be hard-nulled however badly their own validator rejects the column.
user_agent is the largest single case: 359/13,478 corpus columns (2.66%, 7.93× v19) on the
shipped default — the known wart accepted at ship time — with a strong prefix validator that is
advisory-only.

---

## 2. The systemic finding: validators that confirm instead of discriminate

Sweep of all 245 definitions against ~13 negative probe families (random short uppercase
strings, integers, digit codes, English words, prose, real tickers/NAICS/company names/years):

- **31 types accept ≥50% of at least one negative family; 23 of them are universal-designation
  and live in the shipped model's label space** (only `timezone_abbreviation` is model-invisible).
- Worst offenders: `icao_code` (100% of random 4-letter uppercase), `iata_code`/`locale_code`
  (100% of currency/country/state codes), `eu_vat` (96% of uppercase words), `hostname`/
  `docker_ref`/`top_level_domain` (~100% of words/numbers), bio-sequences (protein accepts 84%
  of US state codes), a numeric-shape cluster (`hs_code`/`color_hex`/`geohash` each confirm 100%
  of NAICS-6 columns; `cpt` = any ZIP; `compact_*` dates + `ean` = any 8-digit ID),
  `excel_format` (~100% of everything incl. empty string), unit-optional `height`/`weight`/
  `file_size` (any bare number), person-name patterns (84% of company names).
- **The engine's own precision test certifies these**: `Validation::is_precise()`
  (`crates/finetype-core/src/taxonomy.rs:105-155`) uses a literal blacklist, so `^[A-Z]{4}$`
  counts as precise. The formal notion of precision cannot see the shape-only class.
- 22 of 28 locale_specific types have **no validation_by_locale** — the Precision Principle's
  designated home for their real validation is empty (so `locale_confirmed` can never fire).
- Checksum gaps despite the machinery existing: `credit_card_number` claims Luhn in its notes
  but carries no directive; `ean`/`upc`/`npi` (80840-prefix Luhn) unwired.
- Bug found: the code-attractor demotion fallback emits the **phantom label**
  `representation.alphanumeric.alphanumeric_id` (`value_sharpen.rs:690`,
  `header_sharpen.rs:461/464/664`) — not a taxonomy key (real key:
  `representation.identifier.alphanumeric_id`); demoted columns get no enrichment and are
  invisible to the veto.

The repo already contains the fix template twice over: `currency_code` replaced its own
`^[A-Z]{3}$` with an ISO-4217 enum citing the Precision Principle, and the current
checksum-directive branch is the same move for finance identifiers.

---

## 3. Why no gate caught any of this

**Company/financial reference data is a structurally unmeasured column family.**
Gold (931 cols, 2026-06-29 headline 804/931 = 0.864) contains **zero** ticker, NAICS/industry,
and wkt columns, exactly **one** icao column (P=R=1.0), 12 entity_name, 1 full_name. The
representative band (260 cols) has none either. The corpus-honest gate's oracle asserts icao on
1 column and wkt on 0 corpus-wide, so relocation onto these labels barely registers. A candidate
that fixed or broke all four boundaries would move gold by ~1 point. These failures were
invisible by construction, not missed by negligence.

---

## 4. The truncation question — answered

**Truncation does not fix any of the four failures.** Ticker and NAICS are missing-home
(additive) problems; org-vs-person is a two-sided learned boundary already designated for the
t-000133e418 retrain; wkt is a veto-scope hole over a validator that is already strict. Pruning
icao_code converts "wrong label" into "unknown" at best (the residuals structurally reject
all-alpha codes). And the label-space reshape experiment already proved model-side truncation
costs ~4.1pp gold (NO-GO, all seeds) — **any prune must ship as taxonomy edit + finalize-time
remap on the frozen model** (the choice-0102 categorical pattern), never a retrain with fewer
classes.

There IS a legitimate zero-gold-cost hygiene prune list (~25 types / 12 clusters):
`excel_format`, `password`, medical codes `cpt`/`ndc`/`hcpcs`/`loinc`/`dea_number` (keep `npi`,
wire its Luhn), `smiles`/`protein_sequence`/`rna_sequence`/`cas_number` (keep `inchi`; gate
`dna` on length), `julian`/`hs_code`/`unlocode` (keep `iso6346`), and merging the ten exotic
`amount_*` formatting variants into `amount` + a format property. Worth shipping under the
scenario-5 no-regression gold gate — but as trailing simplification, not the headline. Caveat:
the zero-emission evidence is a 13,478-column snapshot; re-verify at corpus scale for rare types
before each prune. **Do NOT prune the zero-emission securities/banking identifiers** — they are
the active checksum-guard work on this branch and are what company-reference data needs.

---

## 5. Action plan

Ordered by dependency and cost. Every ship gates as usual (gold no-regression; corpus-honest
blocking; drift proxy for anything touching training).

### W0 — Make the failures scoreable (do first, this week)
1. Obtain the external session's actual failing datasets — mandatory for the un-reproduced
   wkt case, and the cheapest source of truth for the rest. Establish which binary version and
   surface (CLI/MCP/DuckDB extension) it ran.
2. Seed a company/financial-reference gold family per the choice-0095 emission-driven policy:
   tickers, industry codes, founder-style org names, descriptions, geometry columns — from
   those datasets + corpus columns where Sense emits icao/wkt/full_name. Without this, none of
   the fixes below can be priced.

### W1 — Extend the hard-NO surface (deterministic, cheap, on-pattern)
3. Add `wkt` (and the strict-validator veto-blind tail: mgrs, plus_code, dms, inchi, iso6346)
   to R32 `schema_fail_demotion` — its admission test (strict validator, genuine columns pass
   ~100%, over-emissions fail ~100%) fits exactly. Alternative: gate-validated veto_safe
   exceptions (periodicity/decimal_number precedent).
4. `user_agent` hard-veto exception behind a corpus-honest gate run — the largest single-label
   win available (2.66% of all columns). First measure what fraction of the 359 fail the prefix
   validator (currently inferred, not measured).
5. Bugfix the phantom `representation.alphanumeric.alphanumeric_id` fallback label (one-liner
   plus test-pin updates).

### W2 — Validator tightening: closed sets and checksums (the currency_code move, at scale)
6. `icao_code`/`iata_code` → closed-set membership over the published airport code lists
   (~11k/9k entries); `top_level_domain` → IANA list (~1.4k); `locale_code` → BCP-47 subtags;
   `h3` → bit-structure check (pure function); `unlocode` → ISO-3166 alpha-2 prefix check.
7. Unit becomes mandatory for `height`/`weight`/`file_size`; month/day range checks for
   `compact_*` dates; length floors for bio-sequences; `eu_vat` per-country formats in
   validation_by_locale.
8. Wire the missing checksums on the current branch's pattern: `credit_card_number` Luhn,
   `ean`/`upc` mod-10, `npi` 80840-Luhn, ABN mod-89.
9. Engine: fix `is_precise()` (measured acceptance-rate against shape-matched random probes, or
   require enum/checksum/literal-anchor structure) and gate the `validation_confirmed`
   short-circuit in `sharpen_attractor_demotion` on genuinely-precise validators — a shape-only
   pattern must never disarm an attractor guard.

### W3 — New types through the proven no-retrain playbook
10. Measure the volume bar first (≥1,000 distinct corpus datasets, timezone_abbreviation
    precedent): ticker-shaped columns in the alphanumeric/word/unknown residuals; NAICS-shaped
    in the numeric_code/integer residuals.
11. If cleared: `finance.securities.ticker` (set-membership validator; per the identifiers
    research in the private planning repo, which already prescribes add-as-validated-set for
    ticker/NAICS/MIC) and an industry-code leaf (NAICS/SIC: 2–6 digits, sector 11–92,
    public-domain code list). Ship as taxonomy leaf + deterministic Sharpen recovery
    (timezone_abbreviation pattern: corroborating header + ≥90% set membership), NO retrain.
12. F5 escape hatch so numeric_code survives fixed-width code columns with code-ish headers
    (a 6-digit constant-width column named `*code` is not an integer quantity).
13. Design note (verified risk): recovery must outrank a validator-confirmed icao assertion —
    W2's icao tightening is what makes room (once icao stops confirming tickers, the attractor
    demotion fires and recovery can land). Sequence W2 before W3, and test rule ordering
    explicitly.

### W4 — Org-vs-person: the designated retrain (t-000133e418), now with a sharper recipe
14. Training-side: add a founder-style arm to `gen_entity_name` (bare Firstname-Lastname and
    Surname-Surname company names) + distilled hard negatives mined from securities/company
    tables; full proxy pre-check + post-train distribution check as mandated.
15. Short-term pilots behind gates: (a) wire the existing-but-dead entity-classifier demotion
    into the multi-branch path; (b) a value-based corporate-marker rule (Inc/Ltd/LLC/GmbH/&/
    Holdings routes person.* → entity_name) — helps the suffixed half only; the bare-name half
    is retrain territory (values are genuinely person-shaped).
16. Give `entity_name` at least a structural validator so person-vs-entity comparisons stop
    being one-sided.

### W5 — Trailing hygiene prune (scenario-5 gate)
17. The §4 prune list via finalize-remap, after corpus-scale emission re-verification. Bundle
    with the deferred simplification work.

### Cross-cutting instruments
18. Re-baseline corpus-scale over-emission on the shipped m2v8m-s43 default (the 1,084-icao /
    249k-full_name figures are v22/v19-era substrate; the shipped-model snapshot disagrees).
19. Unexamined dimensions worth a follow-up look (from the completeness critique): confidence
    calibration as a user-facing trust signal (every reproduced failure shipped at 0.66–0.99
    conf); what `validate` does when enforcing a wrong schema (the failure the analyst actually
    lives with downstream); MCP/DuckDB surface parity; header-branch coverage of business
    vocabulary; whether sibling-context attention earns its keep on business tables.

---

## 6. What we don't know yet

- The actual cause of the description→wkt report — never reproduced; all mechanisms are
  hypotheses until we have the failing values (W0.1).
- Whether ticker/NAICS clear the ≥1,000-dataset volume bar (W3.10 measures it).
- The shipped model's true corpus-scale emission profile for these labels (W-18).
- Whether the generator-shape story fully explains the learned person/org boundary (the
  training mix includes noisy distilled labels; no ablation run).
- Whether a Sharpen recovery rule can reliably outrank a confirmed attractor assertion — rule
  ordering untested (W3.13).

**One line for a stakeholder:** the external eval hit the one data family none of our
instruments can see — the fixes are mostly deterministic (closed-set validators, veto scope, two
new types via the established no-retrain playbook), truncation is not the answer, and the first
move is to turn those datasets into gold so every fix is scoreable.

---

# Execution log — 2026-07-04 (same day)

## Shipped (branch figi-checksum, gate-tested)

**W1 (f726040):** R32 `schema_fail_demotion` widened to the veto-blind strict-validator tail —
`wkt`, `user_agent`, `mgrs`, `plus_code`, `dms`, `iso6346`, `inchi` — plus the phantom
`representation.alphanumeric.alphanumeric_id` fallback label fixed to the real taxonomy key
(5 code sites; demoted columns now get enrichment and are veto-eligible). 1004 tests green.
**Gold gate: 806/931 = 0.866 vs 804 baseline (+2, no regression); representative 183/260 =
0.704 vs 0.691.** Corpus-honest gate: see below.

**W2a (e474993):** the `membership:` taxonomy directive — closed-set sibling of `checksum:`
(deliberately guard-owned, not validator-folded, same rationale) — with
`membership_substance_guard` and embedded OurAirports sets (`labels/sets/`).
`icao_code → icao_airports` (10,249 codes, 2.2% of the 4-letter space: decisive) and
`iata_code → iata_airports` (9,056 codes with a DOCUMENTED limit: 52% of the 3-letter space is
a real airport — GBP/JPY/CHF/AUD included — so major-currency columns stay above the ≥50% keep
bar; demote-only, never worse than shape-only). Needs its own gate run.

## Measurements (three parallel agents)

**user_agent payoff (per-column, the exact 1,000-file drift-snapshot sample):** 355 columns
carry the user_agent label; **355/355 fail the prefix validator below 50% (353 at 0.0)**;
genuine UA columns in-sample: **zero**. 81% are prose under a `content` header. The W1 R32
demotion therefore removes the shipped default's largest single over-emission wart entirely,
with zero measured collateral. The lone corpus wkt over-emission (a malformed-CSV artifact
column) is likewise removed. Data: session scratchpad `ua-measure/target_passrates.tsv`.

**Real external datasets (the actual failing repo, profiled on 0.6.38):**
- ticker→icao REPRODUCES exactly as analysed: SEC EDGAR ticker column → icao_code at conf
  0.978, pass rate 0.476, `validation_advisory_low` — ships anyway (pre-W2 binary).
- NAICS REPRODUCES: 6-digit codes → integer_number via `feature_no_leading_zero`.
- org→person does NOT reproduce on this data: every org-name column → entity_name (0.58–1.0).
  The person-boundary failure is real but specific to founder-style names under bare `name`
  headers (synthetic repro) — not this product's columns.
- description→WKT does NOT reproduce and NO WKT-shaped data exists in the repo: the only
  description column is ICD-10 diagnosis text → entity_name (0.93). The originally reported
  WKT miss is unconfirmable on committed data; the structural veto hole it pointed at is real
  and now closed (W1).
- NEW misses (free eval, all gold-invisible): pipe-compound codes (`ICD-10|B34.9`) →
  postal_code (vpr 0.5); GLEIF `category` enum → region (conf 0.31); `reg_status` →
  state_code at vpr 0.0 (and a DIFFERENT answer on the resorted twin file); normalized-name
  column → swift_bic at vpr 0.007 (swift_bic is veto-blind — R32/veto-safe candidate, same
  class as W1); ELF legal-form codes and ISO 3166-2 subdivisions have no type (subdivision
  currently lands unknown via the unlocode veto).
- ROBUSTNESS: the same 3.36M-row table in two sort orders gets different types on three
  columns — profiling samples the first 100 rows, so sorted production files make
  low-confidence columns non-deterministic. Sampling strategy is a real follow-up.

**Volume bar (ticker/NAICS/MIC vs the ≥1,000-distinct-dataset admission bar):**
- Ticker: 1,586 guarded files — but 84% is ONE per-ticker dump (one file per stock, constant
  symbol column); diversity-honest count 261 files / 185 genuine multi-ticker columns.
  **Passes the bar's letter, fails its intent.**
- NAICS/SIC: 258 (word-boundary-guarded; unguarded `sic` substring is ~99% junk). **Clear
  fail** (~4× short).
- MIC exchange codes: 2. **Clear fail.**

## Decision needed (author): ticker + NAICS leaves despite the volume bar

The plan made the volume bar the arbiter, and on its intent neither type clears it. But the
bar measures the gittables corpus — general web tables — and these types are the target domain
of the company-lookup product surface; the failing datasets are the product's own. Options:
(a) hold the line: no new leaves; W1+W2 make the failures honest (ticker → unknown instead of
airport; NAICS stays integer_number) and the residual homes are documented; or (b) author-waive
the bar on product-strategy grounds and ship both as header-gated Sharpen-recovery leaves
(timezone_abbreviation pattern; NAICS list is public domain, SEC ticker file is US-gov public
domain; near-zero over-emission risk when header-gated). **Recommendation: (b) for NAICS only
— its value validator is tight (set membership + sector range), the header gate is unambiguous
(`naics`/`sic`), and the product need is concrete; hold ticker at (a) until the MIC/exchange
context question is settled, since a ticker leaf without exchange scoping invites its own
over-emission.** Either way the residual-home documentation ships.

## Instrument finding: the corpus-honest gate's rule mode broke at the model swap

The W1 gate run returned NO-GO — but the movers are the categorical retirement (39,014→0,
choice 0102), the numeric_code and isbn deltas, and the rest of the cumulative v19→0.6.38
history. Cause: `eval_rule.sh --default` never passes `--baseline`, so
`corpus_honest_gate.py` defaults its TRANSITION baseline to
`output/ydf-validation-gate/v19_gated.parquet` — the retired v19's predictions. Every gate GO
in history is from June 15–16, pre-swap, when candidate-vs-v19 measured exactly the rule delta;
**no rule change has run this gate since m2v8m became the default** (the only post-swap runs
are the two structurally-unpassable model-swap NO-GOs). W1 is the first, and the verdict
measures two weeks of already-shipped history, not W1. Per the standing rule ("all new work
gates fresh-vs-fresh… NEVER against the retired v19") the NO-GO is not a valid decision input.

Resolution: a fresh-vs-fresh gate — the same 33k stratified pass re-run with the pre-W1
binary (worktree at b4919ab, same model), oracle (`ydf_prediction_gated`, column-intrinsic)
joined from the v19 pass, then `corpus_honest_gate.py --baseline <pre-W1 pass> --candidate
<W1 pass>`. **Verdict: GO, zero band triggers**
(`eval_w1_hardno/corpus_honest_gate_freshbase.txt`). The comparison is not vacuous — 5,500+
column transitions, all W1-shaped: user_agent → unknown/word/alphanumeric_id (3,849 on the
user_agent-oversampled sample), the phantom-label columns resolving to real outcomes (1,414),
wkt/mgrs/iso6346/dms demoting only where schema-contradicted while the sample's 296 genuine
WKT columns keep their label. The oracle-aware bands correctly scored all of it as honest
demotion. **W1 clears every gate: tests, gold +2 (806/931), representative +, corpus-honest
fresh-vs-fresh GO.** Residual noted: 28 entity_name→unknown transitions (0.003%) consistent
with the known run-to-run wobble on degenerate junk columns; the gate did not band it.
**Harness follow-up: rule mode needs a maintained current-default baseline pass** (or an
explicit two-pass protocol) or every future rule ship hits this same false NO-GO.

## End-to-end verification on the real failing data (W1+W2a binary)

The SEC EDGAR ticker column — the reproduced headline failure, icao_code at conf 0.978 —
now returns `unknown` via `membership_substance_guard:geography.transportation.icao_code`
(guard demotes to alphanumeric_id, whose validator the all-alpha tickers then fail, hard-veto
to unknown). Honest abstention until a ticker leaf exists; company `name` → entity_name 0.999
and `cik` → integer unchanged.

## W2a gate results

Gold **807/931 = 0.867** (+1 over W1, +3 over the 804 baseline; representative held 0.704).
Corpus-honest fresh-vs-fresh (baseline = the W1 pass): **GO, zero band triggers**
(`eval_w2a_membership/corpus_honest_gate_freshbase.txt`). The delta is surgical: 405
icao_code and 115 iata_code demotions (→ word/unknown), nothing else moved. End-to-end on
the real failing data: the SEC EDGAR ticker column returns honest `unknown` via
`membership_substance_guard` instead of icao_code@0.978.

`eval_rule.sh` now takes `--gate-baseline <parquet>` and rule mode WARNS loudly when the
gate would default to the retired-v19 transition baseline — the false-NO-GO trap this
campaign hit is documented in the script's help.

**Campaign scoreboard (2026-07-04):** gold 804 → **807** (+3); the shipped default's largest
over-emission wart (user_agent, 2.66% of corpus columns) eliminated with measured-zero
collateral; wkt/icao/iata/strict-tail assertions now honest; phantom label fixed; two gate-GO
rule ships on the branch awaiting merge.

---

# Execution log — 2026-07-05 (W2b, the substance-check batch)

## Shipped (branch figi-checksum, gate GO)

**W2b substance checks — 4 types, gate GO, gold 843 → 844/986 = 0.856 (+1, no regression):**
wired every *shape-only* algorithmic/closed-set validator to its real substance check where the
gate allows. (Correction 2026-07-05: the shipped 4-type headline is **844 (+1)**, the TLD
precision fix. The earlier "846 (+3)" was the *6-type* run with npi/upc active — holding npi/upc
gave back its +2, see below.)

- **Checksums** (`checksum:` directive → `checksum_substance_guard`, demote-only): `credit_card_number`
  (Luhn), `ean` (GS1 mod-10), `abn` (ATO modulus-89). New `gs1`/`npi`/`abn` fns in
  `finetype-core::checksum`; proven end-to-end (a 16-digit Luhn-fail column demotes to
  `integer_number` via the guard; a Luhn-pass column stays credit_card).
- **IANA TLD closed set**: `membership: tld` on `technology.internet.top_level_domain`,
  `labels/sets/tld_codes.txt` (1,437 delegated TLDs incl. punycode IDNs). Density: only 0.24% of
  dictionary words are delegated TLDs (2.2% of short words) — a strong discriminator despite the
  new-gTLD program delegating dictionary words (`.data`, `.app`). **Gold: `top_level_domain`
  precision 0.667 → 1.000, recall held (the guard removed one FP).**
- **Generator fixes (latent bugs surfaced by wiring the validators):** the NPI generator used the
  wrong Luhn parity; `ean_check_digit` was left-anchored (correct only for even-length payloads →
  EAN-8's 7 digits were wrong); the ABN generator emitted 11 *random* digits with no mod-89 check.
  All three now emit genuine check-digit-valid instances, pinned by a generator↔validator agreement
  test across every `checksum:` type. Sharpen-only (no effect on the shipped model), but it stops a
  future retrain training on fakes the guard would demote.

Local: 291 core + 499 model tests green, taxonomy alignment 100% (12,300/12,300 samples), clippy
`-D warnings` + fmt clean. Gate: `eval_w2b_substance/` (fresh baseline
`w3_baseline_with_oracle.parquet`); 4-type candidate `gate_w2b-4type.json` = **GO, zero triggers**.

## Held for author adjudication: `npi` + `upc` checksum guards (correct but gate-blocked)

The 6-type gate run (all five checksums + TLD) returned **NO-GO**, two `collapse` triggers: `npi`
and `upc`. Diagnosis with the candidate/baseline parquets:

- **The model over-emits both as pure numeric attractors.** `npi` marginal 42,675 and `upc` 12,873
  in the 33k-file gate sample — any 10-digit number → npi, any 12-digit → upc. The checksum guards
  demote ~87% (npi) / ~95% (upc) of them.
- **Every demoted "oracle-confirmed" column is a shape-match, not a real identifier.** 20/20 sampled
  demoted-npi columns are financial figures (`ebit`, `grossProfit`, `marketCap`, `totalAssets`,
  `longTermInvestments` — 10-digit billions); demoted-upc columns are `particleId`/product-id runs.
  The keep rate (npi 13%, upc 5%) tracks the ~10% random-Luhn-pass rate, confirming these are random
  numbers, not identifiers (a real NPI/UPC passes its check by construction and is kept).
- **So the guards are per-column CORRECT, but the demotion collapses the over-emitted label past the
  gate's blocking threshold, and gated-YDF co-signs the shape-match** (the instrument-audit's "wrong
  42% on contested ground" — here the *contested ground* is 10/12-digit numeric columns). This is a
  new instance of the gate's oracle-trust blind spot: not the *fixed* honest-abstention case (oracle
  refuted), but oracle-CONFIRMED shape-matches the checksum correctly rejects.

**DECISION (author, 2026-07-05): fold npi/upc into the retrain (t-000133e418), do NOT override the
gate.** Rationale — the Sharpen checksum guard is the wrong fix for these two:

1. **Gold +2 (CORRECTED 2026-07-05 — earlier read as +0).** Three gold columns are model-predicted
   `npi`: `longTermDebt` and `TEAM_ID` (gold `integer_number`) and `utc` (gold `unix_seconds`). With
   the npi guard on, `longTermDebt` and `TEAM_ID` demote to `integer_number` = gold-correct (+2);
   `utc` demotes to `integer_number` but gold is `unix_seconds`, so +0 there. The earlier "+0"
   verdict eyeballed only `TEAM_ID`'s *truncated* sample (one coincidental Luhn-passing value) and
   missed that the guard scores the fuller sample, where these mostly fail. So holding npi COST
   these 2 gold columns — the shipped 4-type is 844, the npi-active 6-type was 846. The corpus-scale
   value is still the main story; but the gold upside is +2, not zero.
2. **Leaky by construction.** A 10-digit number passes NPI-Luhn ~10% of the time by chance, so the
   guard demotes ~90% and leaves a coincidental-pass tail mislabelled.
3. **The referee is proven blind here.** gated-YDF is a shape-matcher for numeric checksum types —
   it asserts npi correctly 9.6% of the time, upc 2.7%, ean 0%, credit_card 1.5%, aba 5.5%, isbn
   16.8%; only isin (structured) is reliable at 100%. Root cause: gating NULLs a prediction only on
   *shape*-validation failure, so it can never filter a shape-match for a checksum type. Full table:
   `output/company-reference-audit/gated_ydf_checksum_reliability.md`. This confirms the npi/upc
   NO-GO is a false alarm — but it also means no current instrument can *credit* the fix, so shipping
   it via Sharpen buys unmeasurable precision at the cost of a blocking-gate override. Not worth it.

The over-emission is a raw-MODEL wart; fix it at source. **Retrain recipe (t-000133e418):** mine hard
negatives from the exact columns the checksum guard demoted — they are a ready-made, labelled set:
```sql
-- financial-figure 10-digit columns the model calls npi; train as integer_number
SELECT file_path, column_name, sample_values_truncated
FROM read_parquet('.../eval_w2b_substance/sample_pass/corpus_pass/columns.parquet')
WHERE sense_prediction='representation.numeric.integer_number'  -- guard already demoted these
  AND column_name RLIKE '(?i)(ebit|revenue|profit|assets|equity|cap|debt|income|cash)';
```
Teach the model that a 10-digit number under a financial header is `integer_number`, under a
provider/healthcare header may be npi — header context is the discriminator the checksum can't be
(checksums are post-hoc, not a training signal). Same for upc (12-digit product/particle IDs →
integer_number/alphanumeric_id). npi/upc **left HELD** in the tree (directives commented out,
`checksum::npi`/`gs1` retained, `gs1` live on `ean`) — do not wire until the retrain removes the
over-emission; a wired guard would re-trip the gate for no gold gain. **W2b CLOSED.**

## Remaining follow-ups (this campaign)

1. ~~Author call: `npi`/`upc`~~ **RESOLVED 2026-07-05: folded into the retrain (t-000133e418), held
   in-tree, do not wire. gated-YDF proven a shape-matcher on numeric checksum types (reliability
   table above). W2b CLOSED.**
2. swift_bic into the veto-blind treatment (R32 or veto_safe exception) — measured shipping at
   vpr 0.007 on real data.
3. Sampling determinism: sort-order-dependent types on low-confidence columns.
4. **W2c structural validators** (need a new structural-validator surface — demotion target isn't
   numeric/alphanumeric_id): BCP-47 subtag check, H3 bit-structure. Plus the height/weight/file_size
   unit policy (a decision, not a mechanic — bare byte-counts are legitimately valid file sizes).
   Bio-sequence length floors were **dropped**: the repr gate's real `dna_sequence` row
   (`effect_allele.exposure`) is single-base GWAS alleles a floor would demote — recall risk for
   unmeasurable benefit.
5. W4 retrain (t-000133e418) unchanged — the founder-style negative recipe stands; now also the
   home for the npi/upc over-emission fix.
