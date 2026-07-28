#!/usr/bin/env bash
# bench_profile_ab.sh — wall-clock A/B of two finetype binaries over the same files.
#
# Why this exists: the speedup a release note quotes has to be re-derivable by someone
# who was not in the room. A figure that lives only in a pull-request body is a number
# nobody can check, and it stops being checkable the moment the branch is deleted.
#
# What it measures is the whole `profile` invocation — model load, duckdb shell-out,
# classification, serialisation — because that is the thing a user waits for. It does
# NOT attribute the difference to any one change; two binaries differ by everything
# that landed between them.
#
# Runs are ALTERNATED (A B A B ...) rather than blocked (A A A B B B). A blocked layout
# hands the whole of any thermal or cache drift to whichever binary ran second, which is
# how a machine that warms up produces a speedup that is not there.
#
# Usage:
#   scripts/bench_profile_ab.sh --a <binA> --b <binB> --files <list.txt> [--repeats N] [--label-a X] [--label-b Y]
#
#   --files  a file of input paths, one per line (finetype's own --files batch format)
#
# Output: one TSV stream on stdout — `#` provenance header, one row per (binary,
# repeat), then a `#` summary block carrying median, mean, range and the two speedups.
# All of it on stdout, and all of it in one file, deliberately: a release note quotes the
# SUMMARY, so the summary has to be inside the artefact the note points at. Split the
# derived figures onto stderr and the committed file no longer contains the number being
# quoted, which is the transcription gap this script exists to close. The provenance
# header records each binary's sha256, so a reader can tell whether the "before" side was
# the published release or a local rebuild of it.
# Exit: 0 both binaries completed every repeat · 1 a run failed
set -euo pipefail
cd "$(cd "$(dirname "$0")/.." && pwd)"

BIN_A=""; BIN_B=""; FILES=""; REPEATS=5; LABEL_A=""; LABEL_B=""
while [ $# -gt 0 ]; do
  case "$1" in
    --a)       BIN_A="$2"; shift 2;;
    --b)       BIN_B="$2"; shift 2;;
    --files)   FILES="$2"; shift 2;;
    --repeats) REPEATS="$2"; shift 2;;
    --label-a) LABEL_A="$2"; shift 2;;
    --label-b) LABEL_B="$2"; shift 2;;
    -h|--help) sed -n '2,22p' "$0" >&2; exit 0;;
    *) echo "unknown argument: $1" >&2; exit 2;;
  esac
done
[ -x "$BIN_A" ] || { echo "FAIL: --a is not an executable: $BIN_A" >&2; exit 2; }
[ -x "$BIN_B" ] || { echo "FAIL: --b is not an executable: $BIN_B" >&2; exit 2; }
[ -f "$FILES" ] || { echo "FAIL: --files is not a file: $FILES" >&2; exit 2; }
[ -n "$LABEL_A" ] || LABEL_A="$("$BIN_A" --version 2>/dev/null | tr ' ' '-')"
[ -n "$LABEL_B" ] || LABEL_B="$("$BIN_B" --version 2>/dev/null | tr ' ' '-')"

N_FILES=$(grep -cve '^[[:space:]]*$' "$FILES")
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# One untimed pass per binary so neither pays the cold page-cache cost of the corpus.
for b in "$BIN_A" "$BIN_B"; do
  "$b" profile --files "$FILES" --out-dir "$TMP/warm" -o json-schema >/dev/null 2>&1 \
    || { echo "FAIL: warm-up run of $b failed" >&2; exit 1; }
done

# time_one <binary> <outdir> -> seconds, as a decimal
time_one() {
  local bin="$1" out="$2" start end
  start=$(python3 -c 'import time;print(time.perf_counter())')
  "$bin" profile --files "$FILES" --out-dir "$out" -o json-schema >/dev/null 2>&1 \
    || { echo "FAIL: timed run of $bin failed" >&2; exit 1; }
  end=$(python3 -c 'import time;print(time.perf_counter())')
  python3 -c "print(f'{$end - $start:.4f}')"
}

sha_of() { shasum -a 256 "$1" 2>/dev/null | cut -d' ' -f1; }

printf '# finetype profile A/B — wall clock, alternated runs\n'
printf '# date\t%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
printf '# files\t%s\t(%s inputs)\n' "$FILES" "$N_FILES"
printf '# repeats\t%s\n' "$REPEATS"
printf '# a\t%s\t%s\t%s\t%s\n' "$LABEL_A" "$BIN_A" "$("$BIN_A" --version 2>/dev/null)" "$(sha_of "$BIN_A")"
printf '# b\t%s\t%s\t%s\t%s\n' "$LABEL_B" "$BIN_B" "$("$BIN_B" --version 2>/dev/null)" "$(sha_of "$BIN_B")"
printf '# the whole `profile` invocation is timed — model load, duckdb shell-out,\n'
printf '# classification, serialisation — and the two binaries differ by everything that\n'
printf '# landed between them, not by any one change.\n'
printf 'binary\trepeat\tseconds\n'
: > "$TMP/a.times"; : > "$TMP/b.times"
for i in $(seq 1 "$REPEATS"); do
  ta="$(time_one "$BIN_A" "$TMP/a$i")"; echo "$ta" >> "$TMP/a.times"
  printf '%s\t%s\t%s\n' "$LABEL_A" "$i" "$ta"
  tb="$(time_one "$BIN_B" "$TMP/b$i")"; echo "$tb" >> "$TMP/b.times"
  printf '%s\t%s\t%s\n' "$LABEL_B" "$i" "$tb"
  rm -rf "$TMP/a$i" "$TMP/b$i"
done

# The summary is derived here rather than by whoever reads the file, because a median
# recomputed by hand in a release note is exactly the transcription this repo keeps
# catching. Median AND mean are both reported: they disagree when one repeat is an
# outlier, and a reader who can see both can tell that from a clean result.
python3 - "$TMP/a.times" "$TMP/b.times" "$LABEL_A" "$LABEL_B" <<'PY'
import statistics as st, sys
a = [float(x) for x in open(sys.argv[1])]
b = [float(x) for x in open(sys.argv[2])]
la, lb = sys.argv[3], sys.argv[4]
print("# summary\tmedian_s\tmean_s\tmin_s\tmax_s")
for lab, v in ((la, a), (lb, b)):
    print(f"# {lab}\t{st.median(v):.4f}\t{st.mean(v):.4f}\t{min(v):.4f}\t{max(v):.4f}")
print(f"# speedup\tmedian\t{st.median(a)/st.median(b):.4f}")
print(f"# speedup\tmean\t{st.mean(a)/st.mean(b):.4f}")
PY
