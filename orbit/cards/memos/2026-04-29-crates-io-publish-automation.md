# crates.io publishing not in release pipeline

**Found during:** v0.6.20 release flow (post-PR #61, validate-corpus iter-4).
The release workflow ran cleanly across 5 platforms, GitHub Releases +
Homebrew tap auto-bumped, but no workspace crate landed on crates.io.

## What

`.github/workflows/release.yml` (228 lines) has **no `cargo publish`
step**. Confirmed via `grep -E "crates|cargo publish" .github/workflows/release.yml`
returning empty. The release pipeline has four jobs:

1. `build` — 5-platform binary matrix
2. `release` — softprops/action-gh-release publishes assets
3. `update-homebrew` — auto-edits the formula in the tap repo
4. `update-install-site` — dispatches to install.meridian.online

None of them touch crates.io.

## Drift evidence

```
| Crate                  | crates.io max_version | Workspace version |
|------------------------|-----------------------|-------------------|
| finetype-core          | 0.5.0                 | 0.6.20            |
| finetype               | 404 (never published) | 0.6.20            |
| finetype-cli           | (not checked)         | 0.6.20            |
| finetype-model         | (not checked)         | 0.6.20            |
| finetype-mcp           | (not checked)         | 0.6.20            |
| finetype-duckdb        | (not checked)         | 0.6.20            |
| finetype-eval          | (not checked)         | 0.6.20            |
| finetype-train         | (not checked)         | 0.6.20            |
| finetype-build-tools   | (not checked)         | 0.6.20            |
```

`finetype-core` is **15 patch versions behind** the live workspace.
The 0.5.0 publish was a manual one-off; nothing has shipped to
crates.io since.

## Where

- Pipeline locus: `.github/workflows/release.yml`
- Workspace crates needing a publish decision:
  `crates/finetype-{core,model,cli,mcp,duckdb,eval,train,build-tools,candle-spike}/Cargo.toml`
- Most workspace crates currently default to `publish = true` (no
  `publish = false` in any leaf `Cargo.toml`), so a `cargo publish`
  in dependency order would push every leaf — that's almost
  certainly not the intent. `finetype-eval`, `finetype-train`, and
  `finetype-candle-spike` are clearly internal-only.

## Open questions for the publish manifest

Which crates should land on crates.io? Three plausible scopes:

- **Minimal:** `finetype-core` only (the library that downstream
  type-inference users would `cargo add`). Ship the binary via
  Homebrew + GitHub releases as today.
- **Standard:** `finetype-core` + `finetype-model` + `finetype-cli`
  (so `cargo install finetype-cli` works as an install path
  alongside Homebrew). MCP, DuckDB, eval, train stay internal.
- **Maximal:** every leaf except the spike. Less defensible —
  `finetype-eval` carries dataset-fetching + benchmark tooling
  that isn't a library.

A `publish = false` audit on internal crates is a precondition
for any of these. Add it before automating the publish step or the
first run will spray internal crates to the registry.

## Provenance

- v0.6.20 release session, 2026-04-29
- Triggered by Hugh's "should we cut a release?" decision
- Memo filed at Hugh's request after the gap was surfaced in the
  release-channel summary
