#!/usr/bin/env bash
# The extension loads on a database opened read-only.
#
# LOAD runs the extension's entry point, which registers the ft_validate and
# ft_profile table macros with CREATE OR REPLACE MACRO on the database being
# opened. On a read-only database that CREATE fails. It used to panic inside
# a function that cannot unwind, and the process aborted (exit 134). The entry
# point now skips a macro DuckDB refuses, so this checks three things:
#
#   1. a read-only open loads the extension and a scalar runs (ft_version);
#   2. on that open the two table macros are absent, as expected;
#   3. a read-write open still registers both macros.
#
# Usage: scripts/check_extension_readonly.sh [path/to/finetype.duckdb_extension]
# Needs the duckdb CLI on PATH. No model: LOAD alone exercises the entry point.
set -euo pipefail

ext="${1:-target/release/finetype.duckdb_extension}"
if [[ ! -f "$ext" ]]; then
  echo "error: no extension at $ext; run make build-extension first" >&2
  exit 2
fi
ext="$(cd "$(dirname "$ext")" && pwd)/$(basename "$ext")"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
db="$work/analyst.duckdb"

# -no-agent keeps the CLI's agent-mode output off; -init /dev/null skips any
# user rc file, so the output is only what the query prints.
duck() { duckdb -no-agent -init /dev/null -noheader -csv "$@"; }

duck "$db" -c "CREATE TABLE t AS SELECT 'a@b.com' AS email;" >/dev/null

macros="SELECT count(*) FROM duckdb_functions() WHERE function_type = 'table_macro' AND function_name IN ('ft_validate', 'ft_profile');"

fail=0

# 1 + 2: read-only open.
set +e
ro_out="$(duck -unsigned -readonly "$db" -c "LOAD '$ext'; SELECT ft_version() IS NOT NULL; $macros" 2>&1)"
ro_status=$?
set -e
if [[ $ro_status -ne 0 ]]; then
  echo "FAIL: read-only open with LOAD exited $ro_status" >&2
  echo "$ro_out" >&2
  fail=1
else
  expected=$'true\n0'
  if [[ "$ro_out" != "$expected" ]]; then
    echo "FAIL: read-only open printed:" >&2
    echo "$ro_out" >&2
    echo "expected ft_version() to run (true) and no table macros (0)" >&2
    fail=1
  else
    echo "ok: read-only open loads; ft_version runs; table macros skipped"
  fi
fi

# 3: read-write open still registers both macros.
set +e
rw_out="$(duck -unsigned "$db" -c "LOAD '$ext'; $macros" 2>&1)"
rw_status=$?
set -e
if [[ $rw_status -ne 0 || "$rw_out" != "2" ]]; then
  echo "FAIL: read-write open exited $rw_status and printed:" >&2
  echo "$rw_out" >&2
  echo "expected both table macros registered (2)" >&2
  fail=1
else
  echo "ok: read-write open registers ft_validate and ft_profile"
fi

exit $fail
