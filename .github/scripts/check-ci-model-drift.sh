#!/usr/bin/env bash
# Non-blocking drift check: warns when FINETYPE_CI_MODEL and models/default
# disagree. Visible as a GitHub Actions `::warning::` annotation — does NOT
# fail the build. Legitimate divergence during promotion PRs is expected;
# the warning just ensures someone sees the split.
#
# Covers AC-7 of specs/2026-04-20-ci-decouple-default-symlink/spec.yaml.
#
# Exit code: always 0 (non-blocking).
set -uo pipefail

ci_model="${FINETYPE_CI_MODEL:-}"
symlink_target=$(readlink models/default 2>/dev/null || cat models/default 2>/dev/null || true)
symlink_target=$(printf '%s' "${symlink_target}" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')

if [ -z "${ci_model}" ]; then
  echo "FINETYPE_CI_MODEL is unset — no drift check (CI is using models/default fallback)."
  exit 0
fi

if [ -z "${symlink_target}" ]; then
  echo "::warning title=CI model drift::FINETYPE_CI_MODEL=${ci_model} but models/default is missing. Runtime users have no default model."
  exit 0
fi

if [ "${ci_model}" != "${symlink_target}" ]; then
  echo "::warning title=CI model drift::FINETYPE_CI_MODEL=${ci_model} differs from models/default→${symlink_target}. CI is testing against a model not promoted for runtime users. If this is a promotion PR, flip models/default in the same PR (or the next one) to reconcile."
else
  echo "CI model aligned with models/default (${ci_model})."
fi

exit 0
