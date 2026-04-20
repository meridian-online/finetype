#!/usr/bin/env bash
# Tests for download-model.sh model-name resolution logic.
#
# Covers AC-1 of specs/2026-04-20-ci-decouple-default-symlink/spec.yaml:
#   (a) FINETYPE_CI_MODEL set + non-empty → resolves to env var value
#   (b) FINETYPE_CI_MODEL="" (empty)      → falls back to models/default
#   (c) env var unset + models/default missing → exits non-zero with error
#   (d) FINETYPE_CI_MODEL whitespace-only  → treated as unresolvable
#
# Cases (a) and (b) stub curl to avoid network hits — the point is to exercise
# the resolution block, not to actually download a model. The test extracts
# the resolution block from download-model.sh and runs it in isolation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOWNLOAD_SCRIPT="${SCRIPT_DIR}/download-model.sh"

if [ ! -f "${DOWNLOAD_SCRIPT}" ]; then
  echo "ERROR: download-model.sh not found at ${DOWNLOAD_SCRIPT}" >&2
  exit 1
fi

# ──────────────────────────────────────────────────────────────────────────
# Test harness — runs the resolution block in a temp dir, captures stdout,
# stderr, and exit code.
# ──────────────────────────────────────────────────────────────────────────

PASS=0
FAIL=0

# Extract just the resolution block — everything from `set -euo pipefail` down
# to the `echo "Active model: ..."` line. This avoids actually downloading
# anything while exercising the real script text.
extract_resolution_block() {
  awk '
    /^set -euo pipefail$/ { found=1 }
    found { print }
    /^echo "Active model:/ { print "exit 0"; exit }
  ' "${DOWNLOAD_SCRIPT}"
}

RESOLUTION_BLOCK=$(extract_resolution_block)
if [ -z "${RESOLUTION_BLOCK}" ]; then
  echo "ERROR: could not extract resolution block from download-model.sh" >&2
  exit 1
fi

run_case() {
  local name="$1"
  local expected_status="$2"
  local expected_stdout_pattern="$3"
  local expected_stderr_pattern="$4"
  # Remaining args are env assignments, passed to `env`.
  shift 4

  local workdir
  workdir=$(mktemp -d)
  # Optional setup hook: if first remaining arg starts with "SETUP:", it's a
  # shell command to run inside the workdir before the resolution block.
  if [ $# -gt 0 ] && [[ "$1" == SETUP:* ]]; then
    (cd "${workdir}" && bash -c "${1#SETUP:}")
    shift
  fi

  local stdout_file="${workdir}/stdout"
  local stderr_file="${workdir}/stderr"
  local status=0
  # Run the extracted resolution block in the workdir with the given env.
  (cd "${workdir}" && env -i HOME="${HOME}" PATH="${PATH}" "$@" bash -c "${RESOLUTION_BLOCK}") \
    >"${stdout_file}" 2>"${stderr_file}" || status=$?

  local stdout_content stderr_content
  stdout_content=$(cat "${stdout_file}")
  stderr_content=$(cat "${stderr_file}")

  local ok=true
  if [ "${status}" != "${expected_status}" ]; then
    ok=false
    echo "  [FAIL] ${name}: expected exit ${expected_status}, got ${status}"
  fi
  if [ -n "${expected_stdout_pattern}" ] && ! grep -qE "${expected_stdout_pattern}" <<<"${stdout_content}"; then
    ok=false
    echo "  [FAIL] ${name}: stdout did not match /${expected_stdout_pattern}/"
    echo "    stdout: ${stdout_content}"
  fi
  if [ -n "${expected_stderr_pattern}" ] && ! grep -qE "${expected_stderr_pattern}" <<<"${stderr_content}"; then
    ok=false
    echo "  [FAIL] ${name}: stderr did not match /${expected_stderr_pattern}/"
    echo "    stderr: ${stderr_content}"
  fi

  if ${ok}; then
    echo "  [PASS] ${name}"
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
  fi

  rm -rf "${workdir}"
}

echo "Running download-model.sh resolution tests..."
echo

# Case (a): FINETYPE_CI_MODEL set, models/default missing → uses env var.
run_case "a_env_var_wins" \
  "0" "^Active model: sherlock-v16$" "" \
  FINETYPE_CI_MODEL=sherlock-v16

# Case (b): FINETYPE_CI_MODEL empty, models/default symlink exists → falls back.
run_case "b_empty_env_falls_back" \
  "0" "^Active model: sherlock-v14$" "" \
  "SETUP:mkdir -p sherlock-v14 && ln -sfn sherlock-v14 models-default-tmp && mkdir -p models && mv models-default-tmp models/default" \
  FINETYPE_CI_MODEL=""

# Case (c): env var unset, models/default missing → loud error.
run_case "c_unresolvable_errors_loudly" \
  "1" "" "cannot resolve model"

# Case (d): FINETYPE_CI_MODEL whitespace → treated as unresolvable.
run_case "d_whitespace_is_unresolvable" \
  "1" "" "cannot resolve model" \
  FINETYPE_CI_MODEL="   "

echo
echo "Results: ${PASS} passed, ${FAIL} failed"
if [ "${FAIL}" -ne 0 ]; then
  exit 1
fi
