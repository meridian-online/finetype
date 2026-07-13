#!/usr/bin/env bash
#
# Publish the FineType library crates to crates.io, in dependency order.
#
# Coupled to the Release workflow: runs on every `v*` tag push, so a release and
# its crates.io publish are one action, not two. Properties that make that safe:
#
#   * Idempotent — a version already on crates.io is skipped, so re-running a tag
#     (or a tag where some crates published and a later one failed) is safe.
#   * Inert until enabled — with no CARGO_REGISTRY_TOKEN secret it warns and
#     no-ops, so merging this leaves the release green and unchanged until the
#     author opts in by adding the secret.
#   * Dependency-ordered — core → model → {mcp, train} → cli. cargo publish
#     (Rust >= 1.66) blocks until each crate lands in the index before the next
#     one resolves its internal deps.
#
# finetype-eval is publish=false (never published; version-less path dep on core).
# finetype-duckdb and finetype-build-tools are publish=false (internal).
set -euo pipefail

if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "::warning::CARGO_REGISTRY_TOKEN is not set — skipping crates.io publish."
  echo "Add it under repo Settings → Secrets and variables → Actions to enable automated publishing."
  exit 0
fi

VERSION="${GITHUB_REF_NAME#v}"
UA="finetype-release-ci (+https://github.com/meridian-online/finetype)"

# Dependency order — a crate's internal deps must already be on the index.
CRATES=(finetype-core finetype-model finetype-mcp finetype-train finetype-cli)

for crate in "${CRATES[@]}"; do
  if curl -sf -A "$UA" "https://crates.io/api/v1/crates/${crate}/${VERSION}" -o /dev/null; then
    echo "✓ ${crate} ${VERSION} already on crates.io — skipping."
    continue
  fi
  echo "── Publishing ${crate} ${VERSION} ──"
  # --no-verify: the release binaries are already built and tested by the `build`
  # job, so the verification rebuild is redundant. It would also need the model
  # (gitignored, downloaded via download-model.sh) unpacked into cargo's temp
  # verify dir, which it is not — so verify would fail on model-embedding crates.
  cargo publish -p "$crate" --no-verify
done

echo "crates.io publish complete for ${VERSION}."
