# Certainty roadmap — harvestable closed-set / checksum / structural rules

**Date:** 2026-07-07 · **Direction:** [[membership-sets-and-certainties-direction]] — the Sense
layer is at its ceiling, so squeeze accuracy from deterministic **certainties** (a value that
provably *is or isn't* the type), never rules that simulate semantics.

> **STATUS 2026-07-08:**
> - **Tier-1 checksums SHIPPED** (commit 8b88845) — imei, issn, orcid, cas, iso6346, dea. Gate GO /
>   gold flat / 17 transitions verified. `output/certainty-checksums/findings.md`, [[checksum-batch-shipped]].
> - **Tier-2 JWT SHIPPED** (commit 3348b57) — `is_jwt` structural check + `jwt_substance_guard`
>   (demote→unknown). Gate GO / gold flat / 23-of-23 full-pipeline demotes verified. `output/certainty-jwt/findings.md`.
> - **Tier-2 XML / JSON / json_array: NO-OP, do NOT build.** The current pipeline emits ZERO of them
>   (xml 0/80 stale-xml files; json/json_array 0 in-sample). The roadmap's 2104-xml/7920-jwt counts
>   were **STALE** — `columns.parquet` predates current url/windows_path/text detection, which already
>   relabels those columns correctly. A guard would be dead code.
> - **PROCESS LESSON (load-bearing):** `columns.parquet` emission counts are stale — the shipped
>   pipeline has improved since it was generated. Re-verify a roadmap candidate's emission against the
>   CURRENT binary (profile the stale-labelled files) BEFORE investing; emission is the reprioritiser.
>   The re-check is now a tool: **`scripts/emission_recheck.py <label>`** (profiles a deterministic
>   sample of the stale-labelled files with the current binary, reports live keep-rate + projected
>   live count + destinations, writes a provenance-stamped JSON under
>   `output/certainty-roadmap/emission_recheck/`). Run it before sourcing any list.

> **TIER-3 LIVE RE-CHECK (2026-07-09, binary 0.6.42 @ e58bba3, 400-file samples):**
> | Candidate | Stale | Live now | Keep | Verdict |
> |---|---:|---:|---:|---|
> | `technology.code.locale_code` | 4,246 | **~1,933** | 45% | **SURVIVOR** — largest live surface; but 2-letter ISO-639 space is collision-dense (destinations incl. `region`/`country_code`/`word`) — the build's calibration must clear collision before shipping |
> | `representation.file.mime_type` | 3,214 | **~1,372** | 43% | **SURVIVOR** — cleaner set (IANA media types are distinctive multi-part strings, low collision); recommended FIRST build |
> | `identity.medical.icd10` | 904 | ~368 | 41% | **BORDERLINE** — moderate residual; destinations are `alphanumeric_id`/`postal_code` (a recovery play, not demote-only); defer behind the two survivors |
> | `identity.medical.hcpcs` | 1,215 | **~149** | 12% | **DROP** — already handled (287/407 sampled → `alphanumeric_id`); below the ~1,000 volume bar. Don't source a CMS list for ~149 (the dead-XML-guard trap). |
>
> Survivor order: **mime_type first** (lower collision), then locale_code (larger but 2-letter-dense — calibrate collision). icd10 deferred; hcpcs dropped.

## What counts as a certainty, ranked by strength

1. **Checksum** — a mathematical check digit. Self-validating, NO list, NO staleness, curbs
   numeric/code-attractor over-emission (the npi/upc/isbn pattern). *The best kind.*
2. **Structural parse** — does the value actually parse (JSON/XML/JWT/WKT)? Deterministic, no
   list. Stronger than the current regex (regex is fooled by content — the s_expression precedent).
3. **Closed published set** — membership in an authoritative enumerable list (naics/tld/unlocode
   pattern). Strength depends on collision density + list shippability + churn.
4. **Set-vs-set vote** — dominant-member reconciliation of two closed sets (the shipped geo vote).

## Prioritisation axes

**Corpus emission** (how often the model emits it — attractor-shaped high-emitters are the prize) ×
**self-precision** (checksum > parse > dense set) × **list shippability** (public+stable+authoritative) ÷
**effort** (wire-existing-algo ≪ implement-algo ≪ source-and-ship-list). Emission counts below are
from the full `columns.parquet`.

---

## Master priority list

| # | Type | Mechanism | Emit | Algo/list status | Notes |
|---|------|-----------|-----:|------------------|-------|
| 1 | `technology.code.imei` | checksum **luhn** | 3563 | **algo EXISTS** | 15-digit attractor; wire `checksum: luhn` — near-zero effort, top ROI |
| 2 | `technology.cryptographic.jwt` | structural parse | 7920 | implement (split `.`, base64url-decode header→JSON) | highest emitter; regex-only today |
| 3 | `representation.file.mime_type` | closed set (IANA media types ~2k) | 3214 | source list (public, like tld) | IANA registry, stable |
| 4 | `technology.code.locale_code` | closed set (ISO 639 lang subtags) | 4246 | source list (IANA language-subtag-registry) | region part already ISO-3166 |
| 5 | `container.object.xml` / `.json` / `.json_array` | structural parse | 2104 (xml) | implement real parse vs regex | regex fooled by content (s_expression precedent) |
| 6 | `identity.medical.loinc` | checksum (mod-10) **+** set | 1631 | implement mod-10; LOINC set is large | checksum alone is a cheap first cut |
| 7 | `identity.medical.hcpcs` | closed set (CMS, public) | 1215 | source list (public domain) | A-V + 4 digits; collides with plate-ish codes |
| 8 | `identity.commerce.issn` | checksum (mod-11) | 1158 | implement mod-11 | 8-digit; distinct from isbn |
| 9 | `identity.medical.icd10` | closed set (WHO, public) | 904 | source list | gold already has icd10 support |
| 10 | `finance.crypto.bitcoin_address` | checksum (Base58Check / bech32) | 469 | implement | self-precise, no list |
| 11 | `representation.scientific.cas_number` | checksum | 298 | implement (weighted mod-10) | chemical registry |
| 12 | `identity.academic.orcid` | checksum (ISO-7064 mod-11-2) | 196 | implement | 16-digit |
| 13 | `geography.transportation.iso6346` | checksum | 183 | implement (container check digit) | shipping container |
| 14 | `identity.medical.dea_number` | checksum (DEA formula) | 171 | implement | US prescriber |
| 15 | `identity.government.ein` | valid-prefix set (~90 IRS campus prefixes) | 1548 | source (public) | weaker certainty — prefix only, not full checksum |
| 16 | `finance.banking.swift_bic` | substructure: chars 5-6 = ISO-3166 country | 1002 | reuse country enum | partial certainty; corroborates, not confirms |
| 17 | `finance.crypto.ethereum_address` | EIP-55 mixed-case checksum | 1800 | implement (optional — many are all-lowercase, uncheckable) | weak: only mixed-case addresses are checkable |

## Excluded, with the reason (honest scope)

- `identity.government.vin` (checksum) & `identity.government.ssn` — **emit 7 each.** Checksummable
  but negligible corpus impact; not worth the algo. Revisit only if an external eval flags them.
- `identity.medical.cpt` (922) — the code list is **AMA-proprietary/licensed; we cannot ship it.**
  Structural (`^\d{5}$`) only, and it's a known 5-digit attractor (banked-retrain finding). Leave.
- `identity.medical.ndc` — FDA directory is **large and changes constantly** (naics-like but worse
  churn). Set is possible but low-priority vs the stable lists above.
- `technology.identifier.snowflake_id` (1102) — **no checksum** (17-20 digit; only a loose embedded
  timestamp range). Would need a heuristic, not a certainty. Skip per the principle.
- Currency `amount_*` family (18 formats) — these are format-structural and already validated; not
  closed-set certainties.
- Datetime (84 types) — format-structural, already covered by the strptime validators.

## Recommended sequence

1. **`imei → checksum: luhn`** — one directive line, algorithm already exists, 3,563 emissions, a
   classic 15-digit numeric attractor. Same gate cycle as npi/upc; likely the same "checksum-blind
   gate false-alarm → adjudicate GO" story. **Do this first.**
2. **A checksum batch** (issn, orcid, cas, iso6346, dea, loinc-mod10) — each is self-precise and
   demote-only; implement the algorithms in `finetype_core::checksum`, wire the directives, gate
   together. ~2,000 combined emissions of attractor over-emission to curb.
3. **JWT + XML/JSON structural substance checks** — deterministic parse, highest emitters (7,920 +
   2,104), no list to source. Mirror the `s_expression` `is_s_expression` substance-guard pattern.
4. **Closed-set sources** (mime_type, locale_code/ISO-639, hcpcs, icd10) — naics/tld/unlocode
   pattern; each needs a sourced, provenance-headed set file. Batch by list availability.

## Method note

Every rule ships the same way as unlocode/geo-vote: gold (no-regression) → corpus-honest gate
(blocking) → **per-column transition trace** (the load-bearing check — it caught the geo-vote
Canadian bug the gate and gold both missed). Checksums are demote-only and self-precise, so they
gate like npi/upc; closed sets need a collision check (dense sets like a 2-letter space stay
demote-only with a keep bar).
