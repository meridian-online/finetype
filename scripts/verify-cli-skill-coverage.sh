#!/usr/bin/env bash
# verify-cli-skill-coverage.sh — drift check between the FineType CLI skill
# (`.claude/skills/finetype-cli/SKILL.md`) and the live `finetype --help`
# surface.
#
# Iterates over every documented subcommand, extracts the long flag names
# from the binary's per-subcommand `--help`, and diffs them against the
# flags documented in SKILL.md. Hidden subcommands (`generate`, `check`) —
# functional but absent from `finetype --help` — are enumerated via a
# curated allowlist baked in below; treating them as obsolete because
# they're not in the visible command list would silently delete real
# documentation.
#
# Exit codes:
#   0 — skill matches binary surface across every enumerated subcommand
#   1 — at least one drift detected (missing or obsolete flag, or missing
#       subcommand section in the skill)
#   2 — script error (binary not on PATH, SKILL.md missing, etc.)
#
# Wiring into release CI is tracked as a follow-up bead. This script
# ships as an invokable standalone artefact.

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SKILL_PATH="${SKILL_PATH:-$REPO_ROOT/.claude/skills/finetype-cli/SKILL.md}"
FINETYPE_BIN="${FINETYPE_BIN:-finetype}"

# Hidden subcommands that exist in the binary but are absent from
# `finetype --help`. Add to this list (and document the addition in
# SKILL.md) when new hidden subcommands are introduced.
HIDDEN_SUBCOMMANDS=(generate check)

# ---- Pre-flight ----

if ! command -v "$FINETYPE_BIN" >/dev/null 2>&1; then
  echo "verify-cli-skill-coverage.sh: '$FINETYPE_BIN' not on PATH" >&2
  exit 2
fi

if [[ ! -f "$SKILL_PATH" ]]; then
  echo "verify-cli-skill-coverage.sh: SKILL.md not found at $SKILL_PATH" >&2
  exit 2
fi

# ---- Helpers ----

# Enumerate subcommands from `finetype --help`. The output has the form:
#   Commands:
#     infer     ...
#     taxonomy  ...
#     ...
#     help      ...
# We capture indented lines after "Commands:" and stop at the first
# blank line. The synthetic `help` subcommand is excluded — it has no
# user-facing flags and is not documented in SKILL.md.
list_visible_subcommands() {
  "$FINETYPE_BIN" --help 2>/dev/null \
    | awk '
        /^Commands:/ { in_cmds = 1; next }
        in_cmds && /^[[:space:]]*$/ { in_cmds = 0; exit }
        in_cmds && /^[[:space:]]+[a-z]/ { print $1 }
      ' \
    | grep -v '^help$' \
    || true
}

# Long flag names from `finetype <subcommand> --help`. We capture lines
# of the form `      --flag-name`, optionally preceded by `-x, ` for
# short-form aliases. The flag value placeholder (e.g. `<FILE>`) is
# stripped.
binary_flags_for() {
  local subcmd="$1"
  "$FINETYPE_BIN" "$subcmd" --help 2>/dev/null \
    | grep -oE '(^|[[:space:]])(-[a-zA-Z], )?--[a-zA-Z0-9-]+' \
    | grep -oE -- '--[a-zA-Z0-9-]+' \
    | sort -u \
    || true
}

# Documented flags for a subcommand, extracted from SKILL.md. We locate
# the `### \`finetype <subcmd>\`` header, then every flag-table row of
# the form `| \`-x, --flag <X>\` | … |` between that header and the
# next `### ` (or the end-of-file).
documented_flags_for() {
  local subcmd="$1"
  awk -v cmd="$subcmd" '
    BEGIN { in_section = 0 }
    # Match `### `finetype <cmd>`` allowing trailing markdown decorations.
    $0 ~ "^### `finetype " cmd "`" { in_section = 1; next }
    in_section && /^### / { in_section = 0 }
    in_section { print }
  ' "$SKILL_PATH" \
    | grep -oE '`(-[a-zA-Z], )?--[a-zA-Z0-9-]+' \
    | grep -oE -- '--[a-zA-Z0-9-]+' \
    | sort -u \
    || true
}

# ---- Main ----

drift_count=0
drift_messages=()

# Portable equivalent of `mapfile -t visible < <(list_visible_subcommands)` —
# bash 3.2 (macOS default) does not ship mapfile.
visible=()
while IFS= read -r line; do
  visible+=("$line")
done < <(list_visible_subcommands)

# Compose the full subcommand list. If the visible list is empty (unusual
# but possible on a very stripped binary), fall back to the hidden list
# only — `${visible[@]+...}` avoids the unbound-variable error under
# `set -u`.
all_subcommands=(${visible[@]+"${visible[@]}"} "${HIDDEN_SUBCOMMANDS[@]}")

for subcmd in "${all_subcommands[@]}"; do
  # Confirm the subcommand has a section in SKILL.md.
  if ! grep -qE "^### \`finetype $subcmd\`" "$SKILL_PATH"; then
    drift_messages+=("MISSING SECTION: \`finetype $subcmd\` is in the binary but has no \`### \`finetype $subcmd\`\` heading in SKILL.md")
    drift_count=$((drift_count + 1))
    continue
  fi

  # Compare flag sets.
  binary_set=$(binary_flags_for "$subcmd")
  doc_set=$(documented_flags_for "$subcmd")

  # `--help` is always supported by clap and is rarely documented in
  # the per-subcommand table. Filter it out of the binary set so it
  # never produces a false-positive missing-from-skill drift.
  binary_set=$(printf '%s\n' "$binary_set" | grep -vx -- '--help' || true)

  missing_in_skill=$(comm -23 <(echo "$binary_set") <(echo "$doc_set") || true)
  obsolete_in_skill=$(comm -13 <(echo "$binary_set") <(echo "$doc_set") || true)

  if [[ -n "$missing_in_skill" ]]; then
    while IFS= read -r flag; do
      [[ -z "$flag" ]] && continue
      drift_messages+=("MISSING FLAG: \`finetype $subcmd $flag\` is in the binary but not documented in SKILL.md")
      drift_count=$((drift_count + 1))
    done <<< "$missing_in_skill"
  fi

  if [[ -n "$obsolete_in_skill" ]]; then
    while IFS= read -r flag; do
      [[ -z "$flag" ]] && continue
      drift_messages+=("OBSOLETE FLAG: \`finetype $subcmd $flag\` is documented in SKILL.md but not in the binary")
      drift_count=$((drift_count + 1))
    done <<< "$obsolete_in_skill"
  fi
done

# ---- Report ----

if [[ $drift_count -eq 0 ]]; then
  echo "verify-cli-skill-coverage.sh: OK — finetype-cli skill aligned with $($FINETYPE_BIN --version) across $((${#all_subcommands[@]})) subcommands."
  exit 0
fi

echo "verify-cli-skill-coverage.sh: $drift_count drift(s) detected against $($FINETYPE_BIN --version):" >&2
for msg in "${drift_messages[@]}"; do
  echo "  - $msg" >&2
done
exit 1
