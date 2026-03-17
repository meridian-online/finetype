# Information Architecture Audit — FineType Repo

**Date:** 2026-03-12
**Auditor:** Nightingale
**Scope:** README.md + top-level user-facing markdown (9 files)
**Reference:** delta / bat / fd README pattern (install → use → configure)

---

## Executive Summary

The repo has good content but the **wrong structure for its audience**. Architecture detail dominates user-facing documentation. Several docs are severely stale (referencing 159–172 types when the current taxonomy has 250). Two brand references still say "Noon" instead of "Meridian". The `backlog/` directory was deleted but is still linked.

**Key recommendation:** Halve the README by moving architecture/internals to `docs/ARCHITECTURE.md`, and retire or consolidate 4 of the 6 `docs/` files.

---

## 1. README.md — Structure Analysis

### Current structure (389 lines)

| Section | Lines | Audience | Assessment |
|---|---|---|---|
| Hero examples | 1–16 | End user | **Good** — 3 clear examples |
| Features | 18–29 | End user | **Good** — but bullet count is high (10 items) |
| Installation | 31–52 | End user | **Good** — 3 methods, clear |
| Usage: CLI | 54–88 | End user | **Good** — but 11 examples may overwhelm |
| Usage: DuckDB | 90–115 | End user | **Good** — relevant, concise |
| Usage: MCP | 117–134 | End user | **OK** — niche audience |
| Usage: Library | 136–146 | Developer | **Needs verification** — API example may be stale |
| Taxonomy | 148–166 | End user | **Good** — clean summary table |
| Performance | 168–184 | Mixed | **OK** — useful but could be shorter |
| Architecture | 186–304 | Contributor | **Move out** — 118 lines of mermaid + pipeline stages |
| Crates | 254–266 | Contributor | **Move out** — internal workspace detail |
| Repo structure | 268–288 | Contributor | **Move out** — `tree` output |
| Why Sense→Sharpen? | 290–300 | Contributor | **Move out** — design rationale |
| Why Candle? | 302–304 | Contributor | **Move out** — 3 lines, niche |
| Development | 306–336 | Contributor | **Move to DEVELOPMENT.md or CONTRIBUTING.md** |
| Taxonomy definitions | 338–358 | Contributor | **Move out** — YAML schema details |
| Known Limitations | 360–370 | End user | **Keep** — useful |
| License/Contributing/Credits | 372–389 | Both | **Keep** — standard |

### Recommendation

**Target: ~200 lines.** Move lines 186–358 (architecture, crates, repo structure, Sense→Sharpen, Candle, development, taxonomy definitions) to `docs/ARCHITECTURE.md` and link from README with one line: "See [Architecture](docs/ARCHITECTURE.md) for internals."

### Exemplar comparison (delta/bat/fd pattern)

| Section | bat | fd | FineType (current) | FineType (recommended) |
|---|---|---|---|---|
| Hero / screenshot | Yes | Yes | Yes (code examples) | Keep |
| Features list | 8 bullets | 6 bullets | 10 bullets | Trim to 6–8 |
| Installation | 3 methods | 3 methods | 3 methods | Keep |
| Usage examples | 5–6 | 4–5 | 11 | Trim to 6 |
| Architecture | No | No | 118 lines | Move to docs/ |
| Performance | No (separate) | No | 16 lines | Keep (brief) |
| Contributing | 1 line + link | 1 line + link | 2 lines | Keep |

---

## 2. Stale Content

### Critical — wrong type counts

| File | Says | Should say | Severity |
|---|---|---|---|
| `TAXONOMY_QUICK_REFERENCE.md` | "172 Type Definitions" in 6 domains, 29 categories | 250 types, 7 domains, 43 categories | **High** — entire file is stale |
| `docs/TAXONOMY_COMPARISON.md` | "159 types" (throughout) | 250 types | **High** — multiple instances |
| `docs/SENSE_AND_SHARPEN_PIPELINE.md` | "163 classes", "21 datasets", "98.3% accuracy" | 250 classes, 30 datasets, 97.7% accuracy | **High** — core stats wrong |
| `docs/LOCALE_DETECTION_ARCHITECTURE.md` | "5 types" with locale support | 5+ types (expanded locale coverage) | **Medium** — locale counts outdated |
| `docs/ENTITY_CLASSIFIER.md` | Unchecked implementation checklist | Implementation is complete | **Medium** — misleading |
| `README.md` Performance section | "95.7% label, 97.3% domain (178/186)" | 97.7% label, 98.9% domain (170/174) | **Medium** — accuracy stats outdated |
| `README.md` Features | "32 deterministic features" | 36 deterministic features | **Low** |

### Critical — broken references

| File | Reference | Issue |
|---|---|---|
| `README.md` line 337 | `[backlog/](backlog/)` | Directory deleted — **broken link** |
| `README.md` line 337 | `[Backlog.md](https://backlog.md)` | External product link — likely not intended as a hyperlink |
| `docs/LOCALE_DETECTION_ARCHITECTURE.md` line 160 | `backlog/decisions/decision-002` | Directory deleted — **broken reference** |
| `README.md` line 387 | `[rmcp](https://github.com/anthropics/rmcp)` | URL may be incorrect — rmcp moved to `modelcontextprotocol/rust-sdk` |

### Brand references — "Noon" → "Meridian"

| File | Line | Text |
|---|---|---|
| `docs/LOCALE_DETECTION_ARCHITECTURE.md` | 107 | "Noon's second pillar" |
| `specs/architectural-pivot/REVIEW.md` | 81 | "Noon pillars" |

### Stale version references

| File | Says | Current |
|---|---|---|
| `docs/LOCALE_GUIDE.md` | "v0.5.3" (multiple) | v0.6.10 |
| `docs/SENSE_AND_SHARPEN_PIPELINE.md` | "v0.5.3+" | v0.6.10 |
| `TAXONOMY_QUICK_REFERENCE.md` footer | "Last Updated: 2026-02-09" | 2026-03-12 |

---

## 3. File Placement — Root Clutter

### Current root markdown files

| File | Lines | Purpose | Recommendation |
|---|---|---|---|
| `README.md` | 389 | User-facing intro | **Keep** — trim to ~200 lines |
| `CLAUDE.md` | ~300 | AI agent instructions | **Keep** — standard for Claude Code |
| `CHANGELOG.md` | ~530 | Release history | **Keep** — standard Rust convention |
| `DEVELOPMENT.md` | 129 | Training + DuckDB build guide | **Rename → `docs/DEVELOPMENT.md`** |
| `TAXONOMY_QUICK_REFERENCE.md` | 324 | Stale taxonomy overview | **Delete or replace** — content is wrong |

### Current `docs/` files

| File | Lines | Last meaningful update | Recommendation |
|---|---|---|---|
| `SENSE_AND_SHARPEN_PIPELINE.md` | 218 | Pre-v0.6 (stale stats) | **Update or consolidate** into ARCHITECTURE.md |
| `ENTITY_CLASSIFIER.md` | 135 | Pre-implementation (checklist stale) | **Delete** — implementation is done, spec is historical |
| `LOCALE_GUIDE.md` | 419 | Pre-v0.6 (stale version refs) | **Update** — good user-facing content |
| `LOCALE_DETECTION_ARCHITECTURE.md` | 165 | Pre-v0.6 (stale locale counts) | **Consolidate** into ARCHITECTURE.md |
| `TAXONOMY_COMPARISON.md` | 229 | Pre-v0.6 (says 159 types) | **Archive to specs/** — research document, not living docs |
| `plans/` (2 files) | ~200 | 2026-03-04/05 | **Move to specs/** — planning docs, not user-facing |

### Recommended structure

```
finetype/
├── README.md              # ~200 lines: hero → install → usage → taxonomy → perf → limitations
├── CHANGELOG.md           # Standard — keep as-is
├── CLAUDE.md              # Agent instructions — keep as-is
├── docs/
│   ├── ARCHITECTURE.md    # NEW: merged from README architecture + Sense→Sharpen + locale arch
│   ├── DEVELOPMENT.md     # Moved from root
│   └── LOCALE_GUIDE.md    # Updated with current version/stats
└── specs/                 # Archive TAXONOMY_COMPARISON.md, ENTITY_CLASSIFIER.md, plans/ here
```

**Net result:** Root goes from 5 markdown files to 3. docs/ goes from 7 files to 3.

---

## 4. Rust Best Practices Compliance

| Convention | Status | Notes |
|---|---|---|
| `README.md` at root | **Yes** | Present |
| `CHANGELOG.md` | **Yes** | Keep a Changelog format |
| `LICENSE` file | **Yes** | MIT |
| `Cargo.toml` workspace | **Yes** | Clean workspace with default-members |
| Per-crate READMEs | **No** | None of the 9 crates have their own README — minor, not blocking |
| `CONTRIBUTING.md` | **No** | README says "Contributions welcome!" but no guide |
| `.github/ISSUE_TEMPLATE` | **Unknown** | Not checked |
| `examples/` directory | **No** | No runnable examples — the "As a Library" README section is the only guidance |
| Rustdoc on public API | **Partial** | Core types are documented, but no `#![doc = include_str!("../README.md")]` in lib.rs |
| `cargo doc` builds clean | **Not tested** | Would need verification |

### Recommendations
- Add a minimal `CONTRIBUTING.md` (build instructions, test commands, PR process)
- Consider adding `examples/` with 1-2 runnable Rust examples if crates.io usage is a goal
- Per-crate READMEs are nice-to-have but not blocking

---

## 5. Link Verification (Mechanical)

### Broken URLs (3)

| File | Link | Issue |
|---|---|---|
| README.md (line 3, 382) | `https://meridian.online/projects/finetype/` | **404** — project page doesn't exist yet |
| README.md (line 387) | `https://github.com/anthropics/rmcp` | **404** — repo moved to `modelcontextprotocol/rust-sdk` |
| README.md (line 337) | `[backlog/](backlog/)` | **Missing** — directory deleted |

### Stale text references (not clickable, but inaccurate)

| File | Reference | Fix |
|---|---|---|
| TAXONOMY_QUICK_REFERENCE.md (252–257) | `definitions_v2_*.yaml` (6 files) | Files are `definitions_*.yaml` (7 files, no `_v2_` prefix) |
| docs/LOCALE_DETECTION_ARCHITECTURE.md (160) | `backlog/decisions/decision-002` | Should be `decisions/0002-locale-detection-post-hoc-validation.md` |

### Working links verified

- `https://meridian.online` — OK
- `https://modelcontextprotocol.io/` — OK
- `https://github.com/huggingface/candle` — OK
- `https://duckdb.org` — OK
- `https://serde.rs` — OK
- `https://keepachangelog.com/` — OK
- `https://semver.org/` — OK
- `https://zenodo.org/record/5706316` — OK (redirects)
- `https://schema.org/docs/full.html` — OK
- All relative file paths (`labels/`, `LICENSE`, `docs/LOCALE_GUIDE.md`, `../labels/`, `../eval/gittables/REPORT.md`) — OK

---

## 6. CLI Examples Verification (Mechanical)

**All 15 tested commands pass. 1 skipped (train — requires pre-generated data).**

### Hero examples (README lines 8–15)

| Command | Expected | Actual | Status |
|---|---|---|---|
| `finetype infer -i "192.168.1.1"` | `technology.internet.ip_v4` | `technology.internet.ip_v4` | **PASS** |
| `finetype infer -i "2024-01-15T10:30:00Z"` | `datetime.timestamp.iso_8601` | `datetime.timestamp.iso_8601` | **PASS** |
| `finetype infer -i "hello@example.com"` | `identity.person.email` | `identity.person.email` | **PASS** |

### Usage section examples

| # | Command | Status | Notes |
|---|---|---|---|
| 1 | `finetype infer -i "bc89:..."` (IPv6) | **PASS** | Returns `technology.internet.ip_v6` |
| 2 | `finetype infer -f ... --mode column` | **PASS** | Column mode works with test file |
| 3 | `finetype profile -f data.csv` | **PASS** | Tested with `tests/fixtures/categoricals.csv` |
| 4 | `finetype load -f data.csv` | **PASS** | Generates valid CREATE TABLE DDL |
| 5 | `finetype mcp` | **PASS** | Starts, prints ready message, exits cleanly |
| 6 | `finetype train ...` | **SKIP** | `--help` verified; flags match docs |
| 7 | `finetype generate --samples 10` | **PASS** | Generates 2480 samples (10/label) |
| 8 | `finetype check` | **PASS** | 250/250 definitions, ALL CHECKS PASSED |
| 9 | `finetype taxonomy --domain datetime` | **PASS** | 84 datetime definitions listed |
| 10 | `finetype schema "datetime.date.*" --pretty` | **PASS** | 42 schemas emitted |

### Development section commands

| Command | Status | Notes |
|---|---|---|
| `cargo build --release` | **PASS** | 4m03s |
| `cargo test --all` | **PASS** | 185 tests, 0 failed, 14 ignored |
| `cargo run --release -- check` | **PASS** | ALL CHECKS PASSED |

**Verdict:** All CLI examples are current and accurate. No stale commands.

---

## 7. Prioritised Recommendations

### P0 — Fix immediately (broken/misleading)

1. **Remove `backlog/` references** from README.md (line 337) and LOCALE_DETECTION_ARCHITECTURE.md (line 160)
2. **Fix broken URLs** — `meridian.online/projects/finetype/` returns 404 (badge + credits), `anthropics/rmcp` moved to `modelcontextprotocol/rust-sdk`
3. **Update README accuracy stats** — Performance section shows outdated numbers
4. **Fix "Noon" → "Meridian"** in docs/LOCALE_DETECTION_ARCHITECTURE.md

### P1 — Address in this PR (structural)

4. **Trim README to ~200 lines** — move Architecture, Crates, Repo Structure, Why Sense→Sharpen, Why Candle, Development, Taxonomy Definitions to `docs/ARCHITECTURE.md`
5. **Delete or replace TAXONOMY_QUICK_REFERENCE.md** — content is deeply stale (172 vs 250 types, wrong domains, wrong categories)
6. **Move DEVELOPMENT.md** from root to `docs/`
7. **Update version references** in LOCALE_GUIDE.md and SENSE_AND_SHARPEN_PIPELINE.md

### P2 — Follow-up (nice-to-have)

8. **Consolidate docs/** — merge SENSE_AND_SHARPEN_PIPELINE.md + LOCALE_DETECTION_ARCHITECTURE.md into a single ARCHITECTURE.md
9. **Archive stale docs** — move TAXONOMY_COMPARISON.md, ENTITY_CLASSIFIER.md, plans/ to specs/
10. **Add CONTRIBUTING.md** with build/test/PR instructions
11. **Fix rmcp link** — verify current GitHub URL
12. **Update LOCALE_GUIDE.md locale counts** to match current coverage
