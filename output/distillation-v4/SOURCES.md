# Distillation v4 — Data Sources

Authoritative per-type record of where v17 training data comes from.
This file is the human-readable companion to the `sourcing_table:` field
in `specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3).

Policy (decision 0050): public datasets with permissive licenses, OR
synthetic generators improved in place. No restricted-registry scraping,
no click-through agreements, no PII.

---

## Overview

```
| Type                                | Path            | Source                                                |
|-------------------------------------|-----------------|-------------------------------------------------------|
| technology.internet.user_agent      | public_dataset  | ua-parser/uap-core test fixtures (Apache-2.0)         |
| identity.medical.loinc              | public_dataset  | NIH NLM Clinical Tables LOINC API (LOINC ToU)         |
| finance.banking.swift_bic           | generator       | crates/finetype-core/src/generator.rs (improved v17)  |
| identity.medical.cpt                | generator       | crates/finetype-core/src/generator.rs (improved v17)  |
| representation.file.excel_format    | generator       | crates/finetype-core/src/generator.rs (improved v17)  |
| identity.government.ssn             | generator       | crates/finetype-core/src/generator.rs (improved v17)  |
| technology.internet.http_method     | schema_only     | labels/definitions_technology.yaml (27-variant enum)  |
```

---

## technology.internet.user_agent

**Path:** `public_dataset`
**Loader:** `output/distillation-v4/loaders/user_agent.py`
**Output:** `output/distillation-v4/user_agent.csv` — 17,812 unique rows
**Target:** ≥1,000 unique values — **achieved 17×**

**Source.** [ua-parser/uap-core](https://github.com/ua-parser/uap-core) test
fixtures, specifically:
- `tests/test_ua.yaml` (browser/bot fixtures)
- `tests/test_device.yaml` (device/brand fixtures, largest — ~16k UAs)
- `tests/test_os.yaml` (OS fixtures)

These files bundle real-world User-Agent strings with expected parse
results — the raw UA strings are extracted (the regex patterns in
`regexes.yaml` are intentionally NOT harvested — that's test oracle
data, not input data).

**License.** Apache-2.0 — compatible with redistribution inside a
training corpus.

**Date accessed.** 2026-04-20.

**Deviation from spec sourcing_table.** None. Primary candidate selected
as specified.

**v16 failure motivating this work.** user_agent ×2 misclassified as
`jwt` / `docker_ref` on corrected eval. Restoring real distilled data
gives the model a training signal this type was previously missing.

**Implications for prep script (ac-04).** The loader emits ~18k rows; this
is far above the target. The prep script may want to cap or sample-down
to avoid this single type dominating the distilled blend relative to the
other ~230 types. Sampling logic lives in the prep script, not the loader.

---

## identity.medical.loinc

**Path:** `public_dataset`
**Loader:** `output/distillation-v4/loaders/loinc.py`
**Output:** `output/distillation-v4/loinc.csv` — 2,109 unique LOINC codes
**Target:** ≥1,000 unique values — **achieved 2.1×**

**Source.** [NIH National Library of Medicine Clinical Tables LOINC API](https://clinicaltables.nlm.nih.gov/apidoc/loinc_items/v3/doc.html)
— endpoint `https://clinicaltables.nlm.nih.gov/api/loinc_items/v3/search`.
Public, no authentication, no click-through. Paginated via
`&count=500&offset=N`; single-letter term sweep (`a`..`d` suffices for
2k+ codes with name-dedup).

**License.** LOINC codes are © Regenstrief Institute, Inc. Redistribution
is royalty-free under the [LOINC Terms of Use](https://loinc.org/terms-of-use/)
with mandatory attribution.

> **Attribution obligation (must appear in any downstream artefact that
> ships these codes):**
>
> "This material contains content from LOINC (http://loinc.org). LOINC is
> copyright © Regenstrief Institute, Inc."

This obligation propagates to the sherlock-v17 model card on HuggingFace
and any release notes mentioning LOINC training data. **ac-12 must include
this attribution string in the HF model card when sherlock-v17 is
uploaded.**

**Date accessed.** 2026-04-20.

**Deviation from spec sourcing_table.** The spec's primary candidate
phrasing mentioned "LOINC top-N common-code subset (published in LOINC's
free 'essentials' tier or extracted from public clinical datasets like
MIMIC-IV's d_labitems)". The MIMIC-IV path required PhysioNet
credentialing, so the NLM Clinical Tables API was chosen instead —
strictly more authoritative, free, no credentials. This is **not** a
fallback-to-generator path-swap; it's a same-path refinement of the
primary candidate.

**v16 failure motivating this work.** None — LOINC had no v16 eval
failure. This is quality prevention: v16 dropped LOINC entirely because
its distilled rows were mislabeled integers. v17 restores real LOINC
codes, giving the model a genuine signal for future datasets that contain
them.

**Known API behaviour caveats.**
- Offset caps at ~7000 per term (HTTP 400 past that) — the loader treats
  400 as end-of-results, which is correct for this API.
- Name-dedup overlap caps useful rows per single-letter term at
  500–1500. The 4-letter sweep (a–d) saturates the 2k target.
- Responses cached under `loaders/_cache/loinc/` — idempotent re-runs
  are offline.

---

## finance.banking.swift_bic

**Path:** `generator`
**Source location:** `crates/finetype-core/src/generator.rs` (active arm
~line 3393, key `("banking", "swift_bic")`).
**Target:** ≥1,000 unique values — **achieved** (verified 1000/1200
unique over generator run).

**v16 failure motivating this work.** swift_bic distilled rows included
`ROSEDALE`, `FRANCE`, `WINNEBAGO`, `SULLIVAN`, `REG_DWORD` — arbitrary
uppercase strings. The v16 generator also used a 20-country list which
was too narrow.

**Improvements implemented.**
- Country codes: expanded from 20 hand-picked to ~175 ISO 3166-1 alpha-2
  codes, biased 40% toward majors so the output distribution mirrors
  real SWIFT traffic.
- Branch marker: 15% chance of documented `"XXX"` head-office suffix.
- Length mix: 45% emit 11-char form, 55% emit 8-char.
- Structure: 4-alpha bank + 2-alpha country + 2-alnum location + optional
  3-alnum branch, enforced in the generator.

**Dead-code note.** There is a second `("payment", "swift_bic")` arm at
~line 1812 that is unreachable (the live taxonomy key is `finance.banking.*`).
Left untouched — cleanup is out of v17 scope.

**Remains in `_DROP_DISTILLED_TYPES`.** No v3/v4 distilled rows consumed.
Generator is the sole source.

---

## identity.medical.cpt

**Path:** `generator`
**Source location:** `crates/finetype-core/src/generator.rs` (~line 2014,
key `("medical", "cpt")`).
**Target:** ≥1,000 unique values — **achieved**.

**v16 failure motivating this work.** cpt distilled rows were mixed
integers (including `"Early Childhood Education and Teaching"` — clearly
mislabeled). Generator previously emitted non-realistic ranges.

**Improvements implemented.**
- Category I: zero-padded 5-digit `00100..=99999` (previously skipped
  the `00100..09999` prefix).
- Category II: `0001F..9999F`.
- Category III: `0001T..9999T`.
- Rare PLA `U` suffix retained for YAML compatibility.
- Weighting: 85% Category I, 8% III, 6% II, 1% U.

**AMA copyright note.** CPT *codes themselves* are factual — we emit
codes only, never AMA-copyrighted descriptor text. This stays within
fair use.

**Remains in `_DROP_DISTILLED_TYPES`.**

---

## representation.file.excel_format

**Path:** `generator`
**Source location:** `crates/finetype-core/src/generator.rs` (~line 2743,
key `("file", "excel_format")`).
**Target:** ≥500 unique values (smaller legitimate vocabulary than
other types) — **achieved**.

**v16 failure motivating this work.** Distilled rows included CLI
commands, country names, time durations. The generator only produced a
few format literals.

**Improvements implemented.**
- 12 weighted branches covering numeric/currency/percent/scientific/date/
  time/date+time/text/conditional formats.
- Locale-prefixed currency: `[$-409]` prefix.
- Stochastic threshold conditionals: `[>=100]...`, `[=1000]...`.
- 20-unit literal-suffix generator.
- 8-variant multi-section builder (positive;negative;zero;text) with 6
  colour codes (`[Red]`, `[Green]`, `[Cyan]`, etc.).
- Edge values: `"General"`, `"Text"`, `"@"`.

**YAML validation pattern updates (ac-02 corollary).**
- `labels/definitions_representation.yaml` `excel_format`: allowed
  char-class extended with `@ * ! =` (required for text placeholder,
  accounting asterisk, `[=N]` equality conditional). `minLength: 2 → 1`
  (for `@`).

**Remains in `_DROP_DISTILLED_TYPES`.**

---

## identity.government.ssn

**Path:** `generator` — **synthetic-only, privacy constraint binding.**
**Source location:** `crates/finetype-core/src/generator.rs` (~line 2063,
key `("government", "ssn")`).
**Target:** ≥1,000 unique values — **achieved**.

**Privacy constraint.** Per spec constraint and decision 0050: SSN is
synthetic-only. No scraping, no download from any registry, no storage
of any real SSN values. The generator emits numbers within SSA-defined
legal structure but does not consult or reproduce real allocations.

**v16 failure motivating this work.** people_directory column's phone
field was misclassified as ssn. Root cause was under-representation of
SSN format variants in the generator blend.

**Improvements implemented.**
- Area: `001..=899`, excluding `666`.
- Group: `01..=99` (excludes `00`).
- Serial: `0001..=9999` (excludes `0000`).
- Dashed form `NNN-NN-NNNN` (primary, 80%).
- No-dash form `NNNNNNNNN` (secondary, 20%).

**YAML validation pattern updates (ac-02 corollary).**
- `labels/definitions_identity.yaml` `ssn`: pattern now accepts dashed
  OR no-dash form; `minLength: 11 → 9`.

**Remains in `_DROP_DISTILLED_TYPES`.**

---

## technology.internet.http_method

**Path:** `schema_only`
**Source location:** `labels/definitions_technology.yaml` L283-298.

**No distilled rows, no generator changes.** The schema *is* the
training signal for this type.

**v16 failure motivating this work.** http_method distilled rows
included `SAN JOAQUIN`, `GOAT`, `OPERATING`, `IN PROGRESS`, `ENROUTE` —
arbitrary uppercase tokens that taught the model to accept any uppercase
noun as http_method.

**Schema change (ac-06 + ac-07).** `enum` AND `pattern` both enumerate
all 27 case variants explicitly:

- 9 IETF methods: GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS, TRACE,
  CONNECT.
- 3 case conventions per method: UPPER, lower, Title.
- Total: 27 literal strings in the enum; pattern is the regex
  alternation over the same 27.

**Why not `(?i)` case-insensitive regex?** `CompiledValidator`
(`crates/finetype-core/src/validator.rs`) applies `pattern` AND `enum`
**conjunctively**. An `enum: [GET, POST, ...]` with `pattern: (?i)^(GET|POST|...)$`
would still reject `"Get"` because enum is exact-string. Both surfaces
must enumerate the 27 variants. Verified by `ac07_http_method_case_variants`
unit test.

**3-surface cascade (decision 0051).**
1. **YAML** — pattern + enum updated.
2. **Validator** — `CompiledValidator` compiles both; conjunctive
   application verified.
3. **Training pipeline** — http_method REMAINS in
   `_DROP_DISTILLED_TYPES`; no distilled rows consumed. The multi-branch
   validation-branch feature picks up the stronger pass-rate at retrain
   time.

**Language note for future work.** The validation branch in the
multi-branch model is a **learned pass-rate feature**, not a filter or
a gate. Changes to schemas propagate to the validation branch only
after retrain. Do not say "the validator filters input" or "the gate
rejects non-http_method"; say "the validation branch's pass-rate feature
on http_method columns becomes more discriminative after the schema
expansion".

**Remains in `_DROP_DISTILLED_TYPES`.**

---

## Fallback-to-generator invocations

None in v17. Both public-dataset types (user_agent, LOINC) cleared their
targets comfortably.

If a future path-swap is needed, document it here citing which candidate
failed and why. Path-swaps for the same type + same target are fast-path
(this file + progress.md note, no spec bump). Full spec bump only
required if the swap changes a target row count, evaluation gate, or
scope boundary.

---

## Spec reference

`specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3) —
see the `sourcing_table:` top-level field for the authoritative
per-type sourcing decisions. This document MUST stay consistent with
that field; any mismatch is a spec/implementation drift to be resolved
before merge.
