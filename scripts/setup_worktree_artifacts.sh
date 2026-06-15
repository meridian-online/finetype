#!/usr/bin/env bash
#
# setup_worktree_artifacts.sh — make a git worktree buildable + eval-ready.
#
# WHY THIS EXISTS
# ---------------
# Git worktrees share commits but NOT gitignored/untracked files. FineType
# keeps its model weights and eval data out of git (see .gitignore:
# models/**/*.safetensors, models/**/config.json, the venv pattern,
# eval/gittables/corpus_pass/columns.parquet, ...). So a fresh worktree is
# missing every artifact the build and the eval gates depend on:
#
#   1. crates/finetype-cli/build.rs follows the (tracked) `models/default`
#      symlink and, for the multi-branch default, calls include_bytes! on
#      <target>/model.safetensors + config.json + label_map.json. Those
#      files are gitignored, so in a worktree the symlink dangles and the
#      build PANICS ("Flat model not found" / cannot resolve models/default
#      / include_bytes! on a missing path). It ALSO embeds models/model2vec
#      and models/entity-classifier the same way.
#   2. The gold + corpus-honest eval gates need the Python venv
#      (eval/gittables/.venv) and the corpus parquet
#      (eval/gittables/corpus_pass/columns.parquet) — both gitignored.
#
# Copying these would waste gigabytes per worktree, so we SYMLINK them back
# to the main checkout. Idempotent: only creates a link when the target is
# missing, and refuses to clobber a real (non-symlink) file.
#
# USAGE
#   scripts/setup_worktree_artifacts.sh [MAIN_CHECKOUT] [WORKTREE]
#
#   MAIN_CHECKOUT  Path to the primary (non-worktree) checkout. Auto-detected
#                  from `git rev-parse --git-common-dir` if omitted.
#   WORKTREE       Path to the worktree to populate. Defaults to the current
#                  worktree's top level.
#
# Run from the main checkout it is a harmless no-op (artifacts already exist).

set -euo pipefail

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# ── Must be inside a git worktree ────────────────────────────────────────
git rev-parse --is-inside-work-tree >/dev/null 2>&1 \
  || die "not inside a git work tree"

# ── Resolve the main checkout ────────────────────────────────────────────
# `git rev-parse --git-common-dir` points at the SHARED .git dir, which lives
# inside the main checkout (its parent). In a linked worktree --git-dir
# differs from --git-common-dir; in the main checkout they match.
common_dir="$(cd "$(git rev-parse --git-common-dir)" && pwd)"

if [[ $# -ge 1 && -n "${1:-}" ]]; then
  main_checkout="$1"
else
  main_checkout="$(dirname "$common_dir")"
fi
[[ -d "$main_checkout" ]] || die "main checkout not found: $main_checkout"
main_checkout="$(cd "$main_checkout" && pwd)"

# ── Resolve the target worktree ──────────────────────────────────────────
if [[ $# -ge 2 && -n "${2:-}" ]]; then
  worktree="$2"
else
  worktree="$(git rev-parse --show-toplevel)"
fi
[[ -d "$worktree" ]] || die "worktree not found: $worktree"
worktree="$(cd "$worktree" && pwd)"

[[ -e "$main_checkout/Cargo.toml" ]] \
  || die "main checkout has no Cargo.toml — wrong path? $main_checkout"

printf 'main checkout : %s\n' "$main_checkout"
printf 'worktree      : %s\n\n' "$worktree"

if [[ "$main_checkout" == "$worktree" ]]; then
  printf 'worktree IS the main checkout — nothing to link (no-op).\n'
  exit 0
fi

# ── Artifacts to bridge ──────────────────────────────────────────────────
# Relative paths, resolved against both checkouts.
#
# NOTE on partially-tracked dirs: models/model2vec and models/entity-classifier
# ship tracked metadata (tokenizer.json, label_index.json, config.json) so the
# DIRECTORY exists in a worktree — but their *.safetensors weights are
# gitignored and absent. build.rs include_bytes!s those weights, so a dir-level
# "it's already present" check is NOT enough; the missing FILES inside must be
# linked. So each model dir lists the exact gitignored files build.rs needs.
#
# The `models/default` symlink is tracked (present in the worktree) but its
# target dir is gitignored in full, so we link the whole resolved target dir.
default_target="$(readlink "$main_checkout/models/default" 2>/dev/null || true)"
[[ -n "$default_target" ]] \
  || die "models/default is not a symlink in main checkout"

# Each entry is one path relative to the checkout root. Listing individual
# weight files (not just their parent dir) covers the partially-tracked case.
artifacts=(
  "models/$default_target"                              # multi-branch default dir (fully gitignored)
  "models/model2vec/model.safetensors"                 # header-branch embeddings (build.rs)
  "models/model2vec/type_embeddings.safetensors"       # header-branch type embeddings (build.rs)
  "models/entity-classifier/model.safetensors"         # full_name demotion weights (build.rs)
  "eval/gittables/.venv"                                # eval gate python env
  "eval/gittables/corpus_pass/columns.parquet"         # corpus-honest gate input
)

linked=0
skipped=0
for rel in "${artifacts[@]}"; do
  src="$main_checkout/$rel"
  dst="$worktree/$rel"

  if [[ ! -e "$src" && ! -L "$src" ]]; then
    printf 'skip   %-50s (absent in main checkout)\n' "$rel"
    skipped=$((skipped + 1))
    continue
  fi

  if [[ -L "$dst" ]]; then
    printf 'ok     %-50s (already symlinked)\n' "$rel"
    skipped=$((skipped + 1))
    continue
  fi

  if [[ -e "$dst" ]]; then
    # A real file/dir is present — never clobber it.
    printf 'keep   %-50s (real file present, left untouched)\n' "$rel"
    skipped=$((skipped + 1))
    continue
  fi

  mkdir -p "$(dirname "$dst")"
  ln -s "$src" "$dst"
  printf 'link   %-50s -> %s\n' "$rel" "$src"
  linked=$((linked + 1))
done

printf '\nlinked %d, skipped %d.\n' "$linked" "$skipped"
