# Reservoir-mining sweep — roadmap (2026-07-14)

Mined the **live-Sense cache** (`output/qualified-name-recovery/sense_cache.tsv`, 837k columns,
sibling-aware labels) for shippable self-precise structural recovery guards, across the full sense
distribution (not just residual — the qualified_name lesson: 68% of a reservoir hides in the
confident-mislabel OVERRIDE tier). 12 candidate types, each mined + adversarially FP-verified.

**5 of 12 were staleness mirages** — `json`, `email`, `ip_address`, `coordinate_wkt` are already
recovered live (by `ceded_leaf_recovery` or the model head); `hex_color`'s `#` signal was stripped
from the cached sample. The real sweep is ~5 guards.

## Ranked

| # | Type | Leaf | Stranded | Verdict | Notes |
|---|------|------|---------:|---------|-------|
| 1 | **url** | technology.internet.url | ~291 | **SHIPPED 2026-07-14** | fire-on widen to override tier; validator self-precise; 0 FP. `output/url-override/` |
| 2 | filename | technology.filesystem.filename | 851 | **SHIPPED 2026-07-14** | leaf minted (sibling of windows_path) + filename_recovery guard; gate GO, gold +1 (fixed a defect it surfaced), 104 promotions 0 FP. `output/filename-recovery/` |
| 3 | delimited_array | container.array.comma_separated (+pipe/semicolon) | ~590→**130** | **SHIPPED 2026-07-14** | `delimited_array_recovery` guard: self-precise bracket/pipe/semicolon only (bare comma REJECTED — indistinguishable from `City,Region`/money/date), per-column delimiter voting. The 590 was a mirage; clean core is 130 (brackets→json_array via ceded, so this uniquely lands paren-tuples). Gate GO zero bands, gold flat 0-regression, repr flat, 130 recoveries 0 FP. `output/delimited-array/finding.md` |
| 4 | version_string | technology.development.version | ~100–130 distinct | **needs-header-gate** | bare MAJOR.MINOR.PATCH is a date/clock magnet (~15% value-only FP); header gate `version\|firmware\|ver\|rev\|release\|build` MINUS `date\|time\|year\|month`, residual-only, never override a confident dmy_dot/ymd_dot; veto any 1900–2099 4-digit component. 72% is one replicated `ver=1.6.1` dump — weigh distinct reach. |
| 5 | windows_path | technology.filesystem.windows_path | ~12 distinct | **needs-gate** | widen existing reader's fire-on to override labels; DRIVE branch (`C:\`) self-precise, UNC branch needs a hex-blob veto (reject `^\\+x?[0-9a-f]+$`, require interior `\`). ~90% is one `architectureSmells` dump — thin distinct reach. |
| 6 | hostname | technology.internet.hostname | 62 | **defer** | genuine FQDNs in DNS-log/ECR data; below the volume bar (single dataset); FP class = BigCloneBench `<num>.java` (java is a brand TLD) → needs code-ext + `.pw` vetoes. Revisit if a network-log corpus lifts reach ~100×. |
| 7 | unix_path | `technology.filesystem.unix_path` (NEW) | 44 | **defer** | ~85% FP — path-absolute URLs are structurally identical to POSIX paths (186/399 named `href`, 182/399 carry query strings). Real leaf gap but not a real reservoir. |

## Next actions

- **Author decision (the one real ask):** mint `technology.filesystem.filename` (sibling of the existing windows_path; representation.file.* is file *properties*, not the file entity)? It unlocks the 851-column
  filename reservoir — a common column type sprayed across five confident-wrong buckets. Worth it; the
  corpus-honest gate literally cannot score it until the leaf exists.
- **Buildable next (no author input):** `delimited_array` — the best buildable-today gated guard (~590
  clean after the three value vetoes). Then `version_string` (header-gated) and `windows_path` (override
  widen + hex-blob veto), both dataset-concentrated so weigh distinct-source reach, not raw counts.
- **Honest scope:** gold has ~0 rows in these residual/text slices, so every recovery gates on the
  corpus-honest cand-vs-base diff + a distinct-cardinality spot-check — not the gold headline — until
  gold expands. And the offline sense_cache is stale on any type `ceded_leaf_recovery` already touches;
  real behaviour is only the live cand-vs-base diff.

Substrate: this file; miner/verifier journal in
`subagents/workflows/wf_c96c5f35-ada/journal.jsonl`; url ship `output/url-override/finding.md`.
