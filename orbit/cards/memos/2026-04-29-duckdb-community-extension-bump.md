# DuckDB community extension not auto-bumped on release

**Found during:** v0.6.20 release flow (post-PR #61, validate-corpus iter-4).
GitHub Releases + Homebrew shipped automatically; the DuckDB community
extension stayed at v0.2.0.

## What

The DuckDB community extension is shipped via PR to
https://github.com/duckdb/community-extensions — Hugh has done this
manually in the past. There is **no automation** in
`.github/workflows/release.yml` for opening that PR.

Workspace ships v0.2.0 of the extension (per CLAUDE.md "Distribution"
section: *"DuckDB community extension (v0.2.0 merged)"*). Workspace
package version is now 0.6.20. The extension binary in
`crates/finetype-duckdb/Cargo.toml` inherits `version.workspace = true`
so the local crate would build at 0.6.20, but no publish step exists.

## Drift evidence

```
| Channel                    | Version | Cadence                       |
|----------------------------|---------|-------------------------------|
| GitHub Release binaries    | 0.6.20  | auto on tag push              |
| Homebrew tap formula       | 0.6.20  | auto on release-job needs:    |
| install.meridian.online    | 0.6.20  | auto via dispatch             |
| crates.io                  | 0.5.0   | (separate memo — not auto)    |
| DuckDB community extension | 0.2.0   | manual PR to community-       |
|                            |         | extensions, no automation     |
```

## Why this matters

DuckDB users discovering FineType via `INSTALL finetype FROM community`
get a build that is **18 minor versions stale**. The extension at v0.2.0
predates the validator widening, the Sharpen rule audit, the v19-relu
model, the validate-corpus harness — essentially everything in CLAUDE.md
"Recent work."

If a user reads the docs site and sees the v0.6.20 feature surface but
installs via DuckDB, they get a 2024-era binary that doesn't match. This
is a documentation honesty issue as much as a versioning one.

## Where

- Extension build target: `crates/finetype-duckdb/`
  (`finetype_duckdb.duckdb_extension` produced by
  `cargo build -p finetype_duckdb --release`)
- Build orchestration: top-level `Makefile` rule `EXTENSION` at line 6
- Community-extensions repo: https://github.com/duckdb/community-extensions
  — descriptions live in
  `community-extensions/extensions/finetype/description.yml`
- No FineType release-pipeline locus exists yet

## Fix paths (sketch — for design when this becomes a card)

- **Manual checklist** — document the steps in `docs/RELEASE.md` or a
  similar runbook. Lowest engineering cost; acknowledges the cadence
  is human-paced. Status quo, just written down.
- **Semi-automated** — release.yml job that builds the extension
  binary, attaches it to the GitHub release, and runs a `gh pr create`
  against `duckdb/community-extensions` with the bump. Requires a PAT
  for the cross-repo push and review by DuckDB maintainers (so still
  not "auto-merged" — they gate-keep).
- **Skip the community channel** — accept that GitHub Release +
  Homebrew are the canonical channels and document the community
  extension as a "best-effort, periodically refreshed" track. Update
  CLAUDE.md to remove the "v0.2.0 merged" line and replace with a
  pointer to the canonical channels.

## Provenance

- v0.6.20 release session, 2026-04-29
- Triggered by Hugh's "should we cut a release?" decision
- Memo filed at Hugh's request after the gap was surfaced in the
  release-channel summary
- Companion memo: `2026-04-29-crates-io-publish-automation.md`
