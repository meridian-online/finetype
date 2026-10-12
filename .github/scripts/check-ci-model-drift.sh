#!/usr/bin/env bash
# Fail when the five places that name the default model disagree.
#
#   models/default                 — the symlink crates/finetype-cli/build.rs follows
#                                    in a checkout; the reference the rest are held to
#   .github/workflows/ci.yml       — FINETYPE_CI_MODEL, the model CI downloads
#   .github/workflows/release.yml  — FINETYPE_CI_MODEL, the model the release builds
#   crates/finetype-cli/build.rs   — DEFAULT_MODEL, the model a `cargo install` binary
#                                    embeds, where no symlink ships
#   .gitignore                     — the `models/<name>/` line recording which model
#                                    directory download-model.sh fills (the weight
#                                    files themselves are ignored by the globs above it)
#
# The build-script constant drifted from the symlink for a week in June 2026
# with CI green, because this script read the environment and the symlink and
# nothing else. Every pin is read from the tree now, so a local run with no
# environment checks the same five things CI does. FINETYPE_CI_MODEL in the
# environment, when set, must agree too.
#
# Names are compared as written. build.rs joins the link target onto models/
# without normalising it and download-model.sh uses the variable's value the
# same way, so `models/foo` and `foo` address different directories for both.
#
# `readlink` first, `cat` second: git checks the symlink out as a plain text
# file on Windows, the fallback download-model.sh and build.rs each carry.
#
# --self-test runs this script over scratch trees, asserting the exit status
# and the message of each planted drift and of the aligned cases.
set -uo pipefail

SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"

PASS=0
FAIL=0

# Strip the CR a Windows checkout leaves on a plain-text symlink, and surrounding
# whitespace, exactly as download-model.sh does before it uses the name.
trim() {
  printf '%s' "$1" | tr -d '\r' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//'
}

# The FINETYPE_CI_MODEL value in a workflow's env block, or empty. A trailing
# YAML comment is not part of the name.
workflow_pin() {
  sed -n -e 's/^[[:space:]]*FINETYPE_CI_MODEL:[[:space:]]*//p' "$1" 2>/dev/null | head -1 | sed -e 's/[[:space:]]*#.*$//' | tr -d '"'"'"
}

# The DEFAULT_MODEL constant in build.rs, whatever cargo fmt did to the spacing.
build_rs_pin() {
  sed -n 's/^[[:space:]]*const[[:space:]]*DEFAULT_MODEL[[:space:]]*:[[:space:]]*&str[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1" 2>/dev/null | head -1
}

check_drift() {
  local target pin value problems=()

  target="$(readlink models/default 2>/dev/null || cat models/default 2>/dev/null || true)"
  target="$(trim "${target}")"
  if [ -z "${target}" ]; then
    echo "CI model drift: models/default resolves to no name. Nothing says which model the binary embeds." >&2
    return 1
  fi

  for pin in .github/workflows/ci.yml .github/workflows/release.yml; do
    value="$(trim "$(workflow_pin "${pin}")")"
    if [ -z "${value}" ]; then
      problems+=("${pin} names no model (no FINETYPE_CI_MODEL line)")
    elif [ "${value}" != "${target}" ]; then
      problems+=("${pin}=${value}")
    fi
  done

  pin=crates/finetype-cli/build.rs
  value="$(trim "$(build_rs_pin "${pin}")")"
  if [ -z "${value}" ]; then
    problems+=("${pin} names no model (no DEFAULT_MODEL constant)")
  elif [ "${value}" != "${target}" ]; then
    problems+=("${pin}=${value}")
  fi

  if ! grep -qxF -- "models/${target}/" .gitignore 2>/dev/null; then
    problems+=(".gitignore has no line models/${target}/ — the tree does not record which model directory CI fills")
  fi

  local raw="${FINETYPE_CI_MODEL:-}"
  if [ -n "${raw}" ]; then
    local ci_model
    ci_model="$(trim "${raw}")"
    if [ -z "${ci_model}" ]; then
      echo "CI model drift: FINETYPE_CI_MODEL is whitespace, which download-model.sh refuses to resolve." >&2
      return 1
    fi
    if [ "${ci_model}" != "${target}" ]; then
      problems+=("FINETYPE_CI_MODEL=${ci_model}")
    fi
  fi

  if [ "${#problems[@]}" -gt 0 ]; then
    echo "CI model drift: models/default=${target}, but:" >&2
    local p
    for p in "${problems[@]}"; do
      echo "  ${p}" >&2
    done
    echo "Point every pin at one model." >&2
    return 1
  fi

  echo "CI model aligned with models/default (${target}): ci.yml, release.yml, build.rs, .gitignore."
  return 0
}

# ──────────────────────────────────────────────────────────────────────────
# Self-test. Each case builds a scratch tree, runs THIS file in it, and holds
# the run to a status and a message — a status alone cannot tell a refusal
# apart from a crash.
# ──────────────────────────────────────────────────────────────────────────

# Materialise the five pins in the scratch tree, all naming $1. Each case
# starts aligned and then breaks the one pin it is about. Invoked through
# each case's setup string, which run_case evals.
# shellcheck disable=SC2329
plant() {
  local name="$1"
  mkdir -p models .github/workflows crates/finetype-cli
  ln -sfn "${name}" models/default
  printf 'env:\n  FINETYPE_CI_MODEL: %s\n' "${name}" > .github/workflows/ci.yml
  printf 'env:\n  FINETYPE_CI_MODEL: %s\n' "${name}" > .github/workflows/release.yml
  printf '#[cfg(feature = "embed-models")]\nconst DEFAULT_MODEL: &str = "%s";\n' "${name}" > crates/finetype-cli/build.rs
  printf 'models/%s/\n' "${name}" > .gitignore
}

run_case() {
  # <name> <want status> <want message pattern> <setup, run inside the scratch tree>
  # [<VAR=value> ...] — the environment the script sees, which is otherwise empty.
  local name="$1" want_status="$2" want_pattern="$3" setup="$4"
  shift 4

  local dir out rc=0 ok=true
  dir="$(mktemp -d)"
  (cd "${dir}" && eval "${setup}")
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
    "1" "models/default=a" \
    "plant b; ln -sfn a models/default"

  # The Windows checkout shape: git writes the symlink as a text file, CRLF and all.
  run_case "drift_plain_text_symlink" \
    "1" "models/default=a" \
    "plant b; rm models/default; printf 'a\r\n' > models/default"

  # A name that differs only by a path component. build.rs would read models/models/a
  # here, so treating this as aligned — what a basename comparison does — is drift.
  run_case "drift_by_path_component" \
    "1" "models/default=models/a" \
    "plant a; ln -sfn models/a models/default"

  run_case "drift_ci_workflow" \
    "1" "\.github/workflows/ci\.yml=b" \
    "plant a; printf 'env:\n  FINETYPE_CI_MODEL: b\n' > .github/workflows/ci.yml"

  run_case "drift_release_workflow" \
    "1" "\.github/workflows/release\.yml=b" \
    "plant a; printf 'env:\n  FINETYPE_CI_MODEL: b\n' > .github/workflows/release.yml"

  run_case "drift_build_rs" \
    "1" "crates/finetype-cli/build\.rs=b" \
    "plant a; printf 'const DEFAULT_MODEL: &str = \"b\";\n' > crates/finetype-cli/build.rs"

  run_case "drift_gitignore" \
    "1" "\.gitignore has no line models/a/" \
    "plant a; printf 'models/b/\n' > .gitignore"

  # The name is a literal, not a pattern: `a.b` must not be satisfied by `axb`.
  run_case "drift_gitignore_name_is_literal" \
    "1" "\.gitignore has no line models/a\.b/" \
    "plant a.b; printf 'models/axb/\n' > .gitignore"

  # The environment is a sixth voice when set, as before.
  run_case "drift_environment" \
    "1" "FINETYPE_CI_MODEL=b" \
    "plant a" \
    FINETYPE_CI_MODEL=b

  run_case "default_missing" \
    "1" "models/default resolves to no name" \
    "plant a; rm models/default"

  run_case "default_empty" \
    "1" "models/default resolves to no name" \
    "plant a; rm models/default; : > models/default"

  run_case "ci_workflow_missing_variable" \
    "1" "\.github/workflows/ci\.yml names no model" \
    "plant a; printf 'env:\n  OTHER: x\n' > .github/workflows/ci.yml"

  run_case "build_rs_missing_constant" \
    "1" "crates/finetype-cli/build\.rs names no model" \
    "plant a; printf 'fn main() {}\n' > crates/finetype-cli/build.rs"

  run_case "ci_model_whitespace" \
    "1" "FINETYPE_CI_MODEL is whitespace" \
    "plant a" \
    "FINETYPE_CI_MODEL=   "

  echo
  echo "Alignment, which must stay green — otherwise the catches above are noise:"

  run_case "aligned_all" \
    "0" "aligned with models/default \(a\)" \
    "plant a"

  run_case "aligned_plain_text_symlink" \
    "0" "aligned with models/default \(a\)" \
    "plant a; rm models/default; printf 'a\r\n' > models/default"

  # cargo fmt may space the constant differently; the name is what matters.
  run_case "aligned_build_rs_spacing" \
    "0" "aligned with models/default \(a\)" \
    "plant a; printf 'const DEFAULT_MODEL : &str=\"a\" ;\n' > crates/finetype-cli/build.rs"

  # The gitignore line may sit among others; it is matched whole.
  run_case "aligned_gitignore_among_others" \
    "0" "aligned with models/default \(a\)" \
    "plant a; printf 'models/z/\nmodels/a/\nmodels/y/\n' > .gitignore"

  # A workflow pin may carry a trailing YAML comment; the name ends before it.
  run_case "aligned_workflow_pin_with_comment" \
    "0" "aligned with models/default \(a\)" \
    "plant a; printf 'env:\n  FINETYPE_CI_MODEL: a  # bump with models/default\n' > .github/workflows/ci.yml"

  run_case "aligned_environment_matches" \
    "0" "aligned with models/default \(a\)" \
    "plant a" \
    FINETYPE_CI_MODEL=a

  run_case "ci_model_unset" \
    "0" "aligned with models/default \(a\)" \
    "plant a"

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
