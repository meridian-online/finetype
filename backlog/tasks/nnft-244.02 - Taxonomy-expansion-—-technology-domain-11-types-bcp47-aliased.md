---
id: NNFT-244.02
title: 'Taxonomy expansion — technology domain (+11 types, bcp47 aliased)'
status: Done
assignee:
  - '@nightingale'
created_date: '2026-03-07 05:11'
updated_date: '2026-03-07 05:54'
labels:
  - taxonomy
  - expansion
  - technology
dependencies: []
references:
  - discovery/taxonomy-revision/EXPANSION.md
  - labels/definitions_technology.yaml
parent_task_id: NNFT-244
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add ~10 new technology types from EXPANSION.md Tiers 1-4:

**New categories:** cloud (aws_arn, s3_uri), identifier (ulid, tsid, snowflake_id)
**Existing categories:** internet (cidr, urn, data_uri), development (docker_ref, git_sha), cryptographic (jwt), code (bcp47 — verify vs locale_code first)

Disambiguation concerns:
- TSID vs MD5 (both 32 hex): need timestamp range check tiebreaker
- Snowflake ID vs large integers: need timestamp extraction validation
- git_sha vs SHA-1: header hints differentiate
- bcp47 vs locale_code: dedup check required before adding
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 YAML definitions added for all technology types with validation, format_string, transform, broad_type, tier
- [x] #2 Generators produce valid samples that pass validation for each new type
- [x] #3 `finetype check` passes with all new technology types
- [x] #4 `finetype schema` exports valid JSON Schema for each new type
- [x] #5 transform_ext field populated for ULID (ulid extension)
- [x] #6 Dedup check completed: bcp47 vs locale_code — decision documented
- [x] #7 TSID includes deterministic timestamp-range disambiguation function in validation
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added 11 new technology types across 3 new categories (cloud, identifier) and 2 existing categories (internet, development, cryptographic):

**New types:**
- `technology.identifier.ulid` — Universally Unique Lexicographically Sortable Identifier
- `technology.identifier.tsid` — Time-Sorted ID (timestamp-range validation for MD5 disambiguation)
- `technology.identifier.snowflake_id` — Twitter/Discord Snowflake IDs
- `technology.cloud.aws_arn` — Amazon Resource Names
- `technology.cloud.s3_uri` — S3 bucket/object URIs
- `technology.cryptographic.jwt` — JSON Web Tokens
- `technology.development.docker_ref` — Docker image references
- `technology.development.git_sha` — Git commit SHA hashes
- `technology.internet.cidr` — CIDR notation (IPv4/IPv6)
- `technology.internet.urn` — Uniform Resource Names
- `technology.internet.data_uri` — Data URIs (base64/text)

**Dedup:** bcp47 added as alias to existing `technology.code.locale_code` — no separate type needed.
**Disambiguation:** TSID includes timestamp-range check (2015-2035 epoch window) to distinguish from random hex strings.

Commit: 6f58d45
<!-- SECTION:FINAL_SUMMARY:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
