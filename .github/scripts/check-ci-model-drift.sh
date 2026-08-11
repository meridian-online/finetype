#!/usr/bin/env bash
# Fail when FINETYPE_CI_MODEL and the models/default target name different models.
#
#   FINETYPE_CI_MODEL — the model `.github/scripts/download-model.sh` fetches, into
#                       models/<name>/.
#   models/default    — the model `crates/finetype-cli/build.rs` embeds under the
#                       default `embed-models` feature.
#
# Resolution mirrors download-model.sh, because that script decides which directory
# CI ends up with: the variable wins while it is set and non-empty, and an unset or
# empty variable sends the download to models/default, leaving no second name to
# compare.
#
# Names are compared as written. build.rs joins the link target onto models/ without
# normalising it and download-model.sh uses the variable's value the same way, so
# `models/foo` and `foo` address different directories for both of them.
#
# `readlink` first, `cat` second: git checks the symlink out as a plain text file on
# Windows, the fallback download-model.sh and build.rs each carry.
#
# --self-test runs this script over scratch trees, asserting the exit status and the
# message of a planted drift and of the aligned cases.
set -uo pipefail

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"

PASS=0
FAIL=0

# Strip the CR a Windows checkout leaves on a plain-text symlink, and surrounding
# whitespace, exactly as download-model.sh does before it uses the name.
trim() {
  printf '%s' "$1" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}

check_drift() {
  local raw ci_model target

  raw="${FINETYPE_CI_MODEL:-}"
  if [ -z "${raw}" ]; then
    echo "FINETYPE_CI_MODEL is unset or empty — download-model.sh falls back to models/default, so there is no second name to compare."
    return 0
  fi

  ci_model="$(trim "${raw}")"
  if [ -z "${ci_model}" ]; then
    echo "CI model drift: FINETYPE_CI_MODEL is whitespace, which download-model.sh refuses to resolve." >&2
    return 1
  fi

  target="$(readlink models/default 2>/dev/null || true)"
  target="$(trim "${target}")"
  if [ -z "${target}" ]; then
    echo "CI model drift: FINETYPE_CI_MODEL=${ci_model}, models/default resolves to no name. Nothing says which model the binary embeds." >&2
    return 1
  fi

  if [ "${ci_model}" != "${target}" ]; then
    echo "CI model drift: FINETYPE_CI_MODEL=${ci_model}, models/default=${target}. CI downloads models/${ci_model} while the binary embeds models/${target}. Point both at one model." >&2
    return 1
  fi

  echo "CI model aligned with models/default (${ci_model})."
  return 0
}

# ──────────────────────────────────────────────────────────────────────────
# Self-test. Each case builds a scratch tree, runs THIS file in it, and holds
# the run to a status and a message — a status alone cannot tell a refusal
# apart from a crash.
# ──────────────────────────────────────────────────────────────────────────

run_case() {
  # <name> <want status> <want message pattern> <setup, run inside the scratch tree>
  # [<VAR=value> ...] — the environment the script sees, which is otherwise empty.
  local name="$1" want_status="$2" want_pattern="$3" setup="$4"
  shift 4

  local dir out rc=0 ok=true
  dir="$(mktemp -d)"
  (cd "${dir}" && mkdir -p models && eval "${setup}")
  # Command substitution, not a pipe: rc is the script's own status.
  out="$(cd "${dir}" && env -i HOME="${HOME}" PATH="${PATH}" "$@" bash "${SELF}" 2>&1)" || rc=$?
  rm -rf "${dir}"

  if [ "${rc}" != "${want_status}" ]; then
    ok=false
    echo "  [FAIL] ${name}: wanted exit ${want_status}, got ${rc}"
  fi
  if ! grep -qE "${want_pattern}" <<<"${out}"; then
    ok=false
    echo "  [FAIL] ${name}: message did not match /${want_pattern}/"
  fi

  if ${ok}; then
    echo "  [PASS] ${name}"
    PASS=$((PASS + 1))
  else
    echo "         got: ${out}"
    FAIL=$((FAIL + 1))
  fi
}

self_test() {
  echo "Running check-ci-model-drift.sh self-test..."
  echo
  echo "Drift, which must be caught:"

  run_case "drift_symlink" \
    "1" "FINETYPE_CI_MODEL=b, models/default=a" \
    "ln -sfn a models/default" \
    FINETYPE_CI_MODEL=b

  # The Windows checkout shape: git writes the symlink as a text file, CRLF and all.
  run_case "drift_plain_text_symlink" \
    "1" "FINETYPE_CI_MODEL=b, models/default=a" \
    "printf 'a\r\n' > models/default" \
    FINETYPE_CI_MODEL=b

  # A name that differs only by a path component. build.rs would read models/models/a
  # here, so treating this as aligned — what a basename comparison does — is drift.
  run_case "drift_by_path_component" \
    "1" "models/default=models/a" \
    "ln -sfn models/a models/default" \
    FINETYPE_CI_MODEL=a

  run_case "default_missing" \
    "1" "models/default resolves to no name" \
    "" \
    FINETYPE_CI_MODEL=a

  run_case "default_empty" \
    "1" "models/default resolves to no name" \
    ": > models/default" \
    FINETYPE_CI_MODEL=a

  run_case "ci_model_whitespace" \
    "1" "FINETYPE_CI_MODEL is whitespace" \
    "ln -sfn a models/default" \
    "FINETYPE_CI_MODEL=   "

  echo
  echo "Alignment, which must stay green — otherwise the catches above are noise:"

  run_case "aligned_symlink" \
    "0" "aligned with models/default \(a\)" \
    "ln -sfn a models/default" \
    FINETYPE_CI_MODEL=a

  run_case "aligned_plain_text_symlink" \
    "0" "aligned with models/default \(a\)" \
    "printf 'a\r\n' > models/default" \
    FINETYPE_CI_MODEL=a

  run_case "ci_model_unset" \
    "0" "FINETYPE_CI_MODEL is unset or empty" \
    "ln -sfn a models/default"

  run_case "ci_model_empty" \
    "0" "FINETYPE_CI_MODEL is unset or empty" \
    "ln -sfn a models/default" \
    FINETYPE_CI_MODEL=

  echo
  echo "Results: ${PASS} passed, ${FAIL} failed"
  [ "${FAIL}" -eq 0 ]
}

case "${1:-}" in
  --self-test)
    self_test
    exit $?
    ;;
  "")
    check_drift
    exit $?
    ;;
  *)
    echo "usage: check-ci-model-drift.sh [--self-test]" >&2
    exit 64
    ;;
esac
