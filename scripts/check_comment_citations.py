#!/usr/bin/env python3
"""Gate the citations in changed comments: a named symbol or path must resolve.

WHY THIS EXISTS
    Over two days this repo shipped ~20 false statements in comments across two
    PRs, and every blocking review finding was prose rather than logic. They
    share one shape: a claim about code the author was NOT looking at, plausible
    because it was almost true, or true of a neighbour, or true last week.

    Two of them were the same sentence, twice: a const attributed to
    one crate that lives in another, then "corrected" into a universal claim
    that was also false. Both were one grep from being refuted.
    The grep is cheap; performing it is what did not happen. So it happens here.

WHAT IS CHECKED (changed comment lines only, `origin/main...HEAD`)
    A  ATTRIBUTION — "`<crate>`'s `<SYMBOL>`" or "`<SYMBOL>` in `<crate>`"
       asserts that a symbol is DEFINED in a named crate. Verified by looking
       for an actual definition (`const`/`static`/`fn`/`struct`/`enum`/`trait`/
       `type`/`macro_rules!`) under that crate. A mere mention does not count —
       the defect this was written for was a const attributed to a crate that
       only ever *reads* the value.
    B  PATH — a backticked repo-relative path (optionally `:LINE`) must exist.

WHERE IT LOOKS
    Comments in the file types SYNTAX names — Rust `//`, `///`, `//!`, and `#`
    in Python, shell, YAML and TOML — plus the names in SYNTAX_BY_NAME, which
    is how the Makefile is reached. A workflow file is in scope
    because it is an instruction file: an agent reads `.github/workflows/ci.yml`
    and acts on it, so a present-tense pointer at a path that has moved is
    indistinguishable from an instruction to use it.

WHAT IS *NOT* CHECKED (scope, stated so nobody reads this as more than it is)
    - ADDED comment lines in the diff, and no more. Pre-existing debt is not
      blocking; this stops new claims, it does not audit old ones. `--all`
      audits the tree.
    - A `/* … */` block is not read, in any language that has one.
    - A file absent from both tables is not read; Markdown is one such.
    - Bare `a::b` Rust paths are deliberately NOT resolved. Precision matters
      more than recall here — a gate that cries wolf gets disabled, and this
      repo has three defused guards already. Adding path resolution means
      teaching it modules, re-exports and glob imports; until then a `::` path
      is out of scope rather than half-checked.
    - It checks that a cited thing EXISTS where claimed. It cannot check what
      the code DOES. "merging under-reports every group but one" was false and
      no citation gate would ever have caught it — that needs a reviewer.

ESCAPE HATCH
    A claim about code outside this repo (upstream libraries, a sibling repo)
    cannot resolve here and must be registered in ACKNOWLEDGED with a reason,
    not silently skipped. Writing the reason is the point.

USAGE
    scripts/check_comment_citations.py             # gate the diff vs origin/main
    scripts/check_comment_citations.py --all       # audit every comment in tree
    scripts/check_comment_citations.py --self-test # prove the gate can fail
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Symbols named in comments that are NOT defined in this repo. Each needs a
# reason: the point of the registry is that "it's external" gets written down
# rather than assumed.
ACKNOWLEDGED: dict[str, str] = {
    "duckdb_functions": "DuckDB's own catalog function, not defined in this tree",
    "duckdb_create_aggregate_function": "DuckDB C API, from libduckdb-sys",
    "duckdb_register_aggregate_function": "DuckDB C API, from libduckdb-sys",
    "include_str": "std macro",
}

DEFN = r"(?:const|static|fn|struct|enum|trait|type|union|mod)\s+{sym}\b|macro_rules!\s+{sym}\b"

# What a comment looks like, per file type: the markers that open one, and the
# quote characters that can hide a marker from being one. This table IS the file
# filter — a path it cannot answer for is a path the gate does not read — so
# widening the set of files and widening the comment matcher are one edit, and
# cannot come apart. `--self-test` asserts a workflow file resolves here.
#
# Rust tracks `"` and not `'`: an apostrophe there is a lifetime or a char
# literal far more often than a string delimiter.
_HASH = (("#",), "\"'")
SYNTAX: dict[str, tuple[tuple[str, ...], str]] = {
    ".rs": (("//!", "///", "//"), '"'),
    ".py": _HASH,
    ".sh": _HASH,
    ".yml": _HASH,
    ".yaml": _HASH,
    ".toml": _HASH,
}
SYNTAX_BY_NAME: dict[str, tuple[tuple[str, ...], str]] = {"Makefile": _HASH}

# `some-crate`'s `SYMBOL`   /   `SYMBOL` in `some-crate`
ATTR_POSSESSIVE = re.compile(r"`([a-z][a-z0-9-]*)`'s\s+`([A-Za-z_][A-Za-z0-9_]*)`")
ATTR_IN = re.compile(r"`([A-Za-z_][A-Za-z0-9_]*)`\s+in\s+`([a-z][a-z0-9-]*)`")

# A backticked repo-relative path, optionally with :LINE
PATH_REF = re.compile(r"`((?:crates|scripts|examples|vendor|\.github)/[A-Za-z0-9_./-]+?)(?::\d+)?`")


def syntax_for(path: Path) -> tuple[tuple[str, ...], str] | None:
    """How comments are written in this file, or None if the gate cannot read it."""
    return SYNTAX.get(path.suffix) or SYNTAX_BY_NAME.get(path.name)


def comment_body(line: str, markers: tuple[str, ...], quotes: str) -> str | None:
    """The comment text on this line, or None if the line carries no comment.

    A marker opens a comment where it starts the line or follows whitespace and
    is not inside a quoted run. Both halves are load-bearing: `${VAR#name}` is
    not a comment, `sed 's/#.*//'` is not a comment, and a citation reported out
    of either is the kind of noise that gets a gate switched off.

    A quote character is tracked only where the line carries an EVEN number of
    it. An unbalanced one is prose — "don't", "it's" — and tracking it would
    swallow the rest of the line, hiding a real comment behind an apostrophe.
    The tie goes to reading the line: under-reading a quoted argument costs a
    false positive somebody sees and fixes, while over-reading one costs a
    citation nobody checks, silently.
    """
    live = {q for q in quotes if line.count(q) % 2 == 0}
    open_q = ""
    for i, c in enumerate(line):
        if open_q:
            if c == open_q:
                open_q = ""
            continue
        if c in live:
            open_q = c
            continue
        if i and not line[i - 1].isspace():
            continue
        for mark in markers:
            if line.startswith(mark, i):
                return line[i + len(mark):].strip()
    return None


def comment_on(path: Path, line: str) -> str | None:
    syn = syntax_for(path)
    return None if syn is None else comment_body(line, *syn)


def tracked_files() -> list[Path]:
    """The tracked paths the gate can read comments in.

    `git ls-files` rather than a glob so the audit covers what the repository
    ships — a submodule is one gitlink entry here, not a second tree walked at
    this repo's expense.
    """
    out = subprocess.run(
        ["git", "-C", str(ROOT), "ls-files", "-z"], capture_output=True, text=True
    )
    files = [ROOT / f for f in (out.stdout or "").split("\0") if f]
    return sorted(f for f in files if syntax_for(f) is not None and f.is_file())


def crates() -> set[str]:
    d = ROOT / "crates"
    return {p.name for p in d.iterdir() if p.is_dir()} if d.is_dir() else set()


_CRATE_TEXT: dict[str, str] = {}


def crate_text(crate: str) -> str:
    """Every .rs byte in a crate, read once and cached.

    Pure Python on purpose. This used to shell out to `grep -E`, where `\\s`
    and `\\b` are GNU extensions: the self-test passed on macOS and failed on
    the Linux runner, for the same tree. A gate whose verdict depends on which
    grep is installed is worse than no gate, because it is only trustworthy
    where nobody is looking.
    """
    if crate not in _CRATE_TEXT:
        d = ROOT / "crates" / crate
        parts = []
        for f in sorted(d.glob("**/*.rs")):
            if "target" in f.parts:
                continue
            try:
                parts.append(f.read_text(errors="replace"))
            except OSError:
                pass
        _CRATE_TEXT[crate] = "\n".join(parts)
    return _CRATE_TEXT[crate]


def defines(crate: str, symbol: str) -> bool:
    """Is `symbol` really a thing in `crate`?

    Three ways to be one, all found by measuring the false positives this gate
    produced on its first whole-tree run (5 of 7 hits were wrong):

      1. an item definition — `const`/`fn`/`struct`/…  (including in `build.rs`,
         which is where the fixture that exposed the grep divergence lives)
      2. a MODULE or TEST FILE — "`X`'s `sampling` tests" names
         `crates/X/tests/sampling.rs`, not an item
      3. a reserved STRING LITERAL — "`X`'s `__some_alias` alias"

    A bare mention in prose still does not count, which is what keeps the
    original defect — a const attributed to a crate that only *reads* the
    value — detectable.
    """
    d = ROOT / "crates" / crate
    if not d.is_dir():
        return False
    if any(p for p in d.glob(f"**/{symbol}.rs") if "target" not in p.parts):
        return True
    text = crate_text(crate)
    if re.search(DEFN.format(sym=re.escape(symbol)), text):
        return True
    return f'"{symbol}"' in text


def path_exists(ref: str) -> bool:
    """Repo-relative, or crate-relative — comments write both and mean the same.

    A path written as `vendor/x/` may be real at `crates/<crate>/vendor/x/`.
    Flagging that as missing is noise; a path that resolves nowhere is not.
    """
    if (ROOT / ref).exists():
        return True
    return any((c / ref).exists() for c in (ROOT / "crates").iterdir() if c.is_dir())


def changed_files() -> list[Path]:
    """Files this branch touched, or a LOUD failure.

    The one thing this gate must never do is pass vacuously. A shallow CI
    checkout has no merge base with origin/main, `git diff` then yields nothing,
    and "citations resolve" would be printed over an unread diff — which is
    precisely how a checker becomes decorative. So an unresolvable base is an
    error, not an empty list.
    """
    subprocess.run(["git", "-C", str(ROOT), "fetch", "-q", "--no-tags", "origin", "main"],
                   capture_output=True)
    base = subprocess.run(
        ["git", "-C", str(ROOT), "merge-base", "origin/main", "HEAD"],
        capture_output=True, text=True,
    )
    if base.returncode != 0 or not (base.stdout or "").strip():
        sys.exit(
            "check-comment-citations: no merge base with origin/main — refusing to\n"
            "report success over a diff it could not read. In CI this means the\n"
            "checkout is too shallow; set `fetch-depth: 0`."
        )
    out = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "--name-only", "--diff-filter=d", "origin/main...HEAD"],
        capture_output=True,
        text=True,
    )
    touched = [ROOT / f for f in (out.stdout or "").split()]
    return [f for f in touched if syntax_for(f) is not None]


def added_comment_lines(path: Path) -> list[tuple[int, str]]:
    """(line-number, comment-body) for lines this branch ADDED."""
    rel = path.relative_to(ROOT)
    out = subprocess.run(
        ["git", "-C", str(ROOT), "diff", "-U0", "origin/main...HEAD", "--", str(rel)],
        capture_output=True,
        text=True,
    )
    hits: list[tuple[int, str]] = []
    lineno = 0
    for line in (out.stdout or "").splitlines():
        hunk = re.match(r"^@@ -\d+(?:,\d+)? \+(\d+)", line)
        if hunk:
            lineno = int(hunk.group(1))
            continue
        if line.startswith("+") and not line.startswith("+++"):
            body = comment_on(path, line[1:])
            if body is not None:
                hits.append((lineno, body))
            lineno += 1
    return hits


def comment_lines(path: Path, text: str) -> list[tuple[int, str]]:
    """(line-number, comment-body) for every comment in `text`, read as `path`."""
    hits: list[tuple[int, str]] = []
    for i, line in enumerate(text.splitlines(), 1):
        body = comment_on(path, line)
        if body is not None:
            hits.append((i, body))
    return hits


def all_comment_lines(path: Path) -> list[tuple[int, str]]:
    return comment_lines(path, path.read_text(errors="replace"))


def check(pairs: list[tuple[Path, list[tuple[int, str]]]]) -> list[str]:
    known = crates()
    failures: list[str] = []
    for path, lines in pairs:
        rel = path.relative_to(ROOT)
        for lineno, body in lines:
            for crate, symbol in (
                [(c, s) for c, s in ATTR_POSSESSIVE.findall(body)]
                + [(c, s) for s, c in ATTR_IN.findall(body)]
            ):
                if crate not in known or symbol in ACKNOWLEDGED:
                    continue
                if not defines(crate, symbol):
                    where = [k for k in known if defines(k, symbol)]
                    hint = f" — it is defined in {', '.join(where)}" if where else " — no crate defines it"
                    failures.append(f"{rel}:{lineno} attributes `{symbol}` to `{crate}`{hint}")
            for ref in PATH_REF.findall(body):
                if not path_exists(ref):
                    failures.append(f"{rel}:{lineno} cites `{ref}`, which does not exist")
    return failures


def self_test() -> int:
    """Prove the gate detects — on fixtures DERIVED from this repo, not named.

    The first port of this script to a second repo failed 6 of 10 cases purely
    because the fixtures named the first repo's crates. Hardcoded fixtures do
    not travel; derived ones do. If the repo is too small to derive a case, that
    is a loud failure rather than a silent skip.
    """
    known = sorted(crates())
    if len(known) < 2:
        print("self-test: need >= 2 crates to derive an attribution case")
        return 1

    # A symbol one crate defines and another does not — the real defect shape.
    sym = home = other = None
    for c in known:
        for f in sorted((ROOT / "crates" / c).glob("**/*.rs")):
            for m in re.finditer(r"\bconst\s+([A-Z][A-Z0-9_]{4,})\b", f.read_text(errors="replace")):
                cand = m.group(1)
                # The fixture must be valid under the SAME resolver the test
                # exercises, not merely under the regex that found it. When
                # those two disagreed, the self-test passed locally and failed
                # in CI, and the derivation had no way to notice.
                if not defines(c, cand):
                    continue
                elsewhere = [k for k in known if k != c and defines(k, cand)]
                if len(elsewhere) < len(known) - 1:
                    wrong = next((k for k in known if k != c and not defines(k, cand)), None)
                    if wrong:
                        sym, home, other = cand, c, wrong
                        break
            if sym:
                break
        if sym:
            break
    if not sym or not home or not other:
        print("self-test: could not derive a crate-exclusive symbol")
        return 1

    real_file = next(iter(sorted((ROOT / "crates" / home).glob("**/*.rs")))).relative_to(ROOT)
    missing = f"crates/{home}/src/definitely_not_here_{abs(hash(home)) % 9973}.rs"

    cases = [
        (f"`{other}`'s `{sym}` holds it", False,
         f"a const attributed to a crate that does not define it ({sym} -> {other})"),
        (f"`{sym}` in `{other}` holds it", False,
         "the same claim in the other word order"),
        (f"`{home}`'s `{sym}` holds it", True,
         f"STAYS GREEN: the true attribution ({sym} -> {home})"),
        (f"see `{real_file}:1` for why", True,
         "STAYS GREEN: a real path with a line number"),
        (f"see `{missing}` for why", False,
         "a cited file that is not there"),
        (f"`{home}`'s `TotallyMadeUpThing_{abs(hash(sym)) % 97}` is used", False,
         "a symbol no crate defines at all"),
    ]
    # --- the three PRECISION fixtures -------------------------------------
    # Each was a false positive on this gate's first whole-tree run, when it
    # scored 7 hits of which 5 were wrong. They are regression tests for the
    # noise, and a gate that cries wolf gets switched off — so they are derived
    # here too rather than named, which is how they went missing when this
    # script was first ported to a second repo.
    test_file = next(iter(sorted(ROOT.glob("crates/*/tests/*.rs"))), None)
    if test_file is None:
        print("self-test: no crates/*/tests/*.rs to derive the test-file case from")
        return 1
    test_crate, test_stem = test_file.parts[-3], test_file.stem

    literal = lit_crate = None
    for c in known:
        for f in sorted((ROOT / "crates" / c).glob("**/*.rs")):
            text = f.read_text(errors="replace")
            m = re.search(r'"([a-z_][a-z0-9_]{4,})"', text)
            # Wanted: a token present as a QUOTED STRING but not as an item.
            # `defines()` resolves it through the literal branch, which is the
            # behaviour under test — so check the item pattern directly here,
            # not `defines()`, or the condition is circular and never holds.
            if m and not re.search(
                DEFN.format(sym=re.escape(m.group(1))),
                text,
            ):
                literal, lit_crate = m.group(1), c
                break
        if literal:
            break
    if literal is None:
        print("self-test: no crate-local string literal to derive the literal case from")
        return 1

    rel_path = None
    for c in known:
        for cand in ("src/lib.rs", "src/main.rs", "tests"):
            if (ROOT / "crates" / c / cand).exists() and not (ROOT / cand).exists():
                rel_path = cand
                break
        if rel_path:
            break
    if rel_path is None:
        print("self-test: no crate-relative path to derive the path case from")
        return 1

    cases += [
        (f"its own guard lives in `{test_crate}`'s `{test_stem}` tests", True,
         f"STAYS GREEN: a test file, not an item ({test_crate}/tests/{test_stem}.rs)"),
        (f"must match `{lit_crate}`'s `{literal}` alias", True,
         f"STAYS GREEN: a string literal, not a const (\"{literal}\")"),
        (f"read from `{rel_path}`", True,
         f"STAYS GREEN: a crate-relative path, real under crates/*/{rel_path}"),
    ]

    for name, why in list(ACKNOWLEDGED.items())[:1]:
        cases.append((f"`{name}` is called here", True,
                      f"STAYS GREEN: ACKNOWLEDGED as external ({why})"))

    # --- PAST RUST: the file filter and the comment matcher, together ------
    # A file filter widened without the comment matcher yields a gate that reads
    # more files and finds nothing in them — coverage in appearance only. The
    # cases above hand `check()` a comment body directly, so they cannot tell
    # the difference. These hand it a RAW LINE and let the gate's own reader
    # find the comment, so a `#` line it cannot parse is a MISS here.
    workflow = next(iter(sorted(ROOT.glob(".github/workflows/*.yml"))), None)
    if workflow is None:
        print("self-test: no .github/workflows/*.yml to derive the workflow case from")
        return 1

    # Which real files the gate will open, asserted before anything is planted
    # in one. The line cases below plant into a probe path; these say the tree's
    # own workflow, shell and Python files are paths the gate reaches at all.
    # The Markdown control is the stated scope limit, held to rather than
    # assumed: widening to it is a decision, not a side effect.
    set_cases: list[tuple[Path, bool, str]] = []
    for pattern in (".github/workflows/*.yml", "scripts/*.sh", "scripts/*.py"):
        found = next(iter(sorted(ROOT.glob(pattern))), None)
        if found is None:
            print(f"self-test: no {pattern} to derive a file-set case from")
            return 1
        set_cases.append((found, True, f"IN SCOPE: {found.relative_to(ROOT)}"))
    doc = next(iter(sorted(ROOT.glob("*.md"))), None)
    if doc is None:
        print("self-test: no *.md to derive the out-of-scope case from")
        return 1
    set_cases.append((doc, False, f"OUT OF SCOPE, and stated so: {doc.relative_to(ROOT)}"))

    probe_rs = ROOT / "crates" / home / "src" / "probe.rs"
    probe_yml = workflow.parent / "probe.yml"
    probe_sh = ROOT / "scripts" / "probe.sh"

    line_cases: list[tuple[Path, str, bool, str]] = [
        (probe_yml, f"      # rebuilt from `{missing}`", False,
         "a dead path cited in a workflow comment"),
        (probe_yml, f"      # rebuilt from `{real_file}:1`", True,
         "STAYS GREEN: a real path cited in a workflow comment"),
        (probe_yml, f"      - run: make build  # rebuilt from `{missing}`", False,
         "a dead path in a TRAILING workflow comment"),
        (probe_yml, f"      - run: echo 'rebuilt from `{missing}`'", True,
         "STAYS GREEN: a quoted YAML value is not a comment"),
        (probe_yml, f"      - run: curl https://example.invalid/x#`{missing}`", True,
         "STAYS GREEN: a `#` fragment mid-token is not a comment"),
        (probe_sh, f"sed 's/#.*//' in.txt  # rebuilt from `{missing}`", False,
         "a `#` inside a quoted argument does not end the scan early"),
        (probe_sh, f"grep -- '# rebuilt from `{missing}`' in.txt", True,
         "STAYS GREEN: a `#` inside a quoted argument opens no comment"),
        (probe_sh, f"dest=${{src#`{missing}`}}", True,
         "STAYS GREEN: `${VAR#…}` is a parameter expansion, not a comment"),
        (probe_sh, f"echo \"it's fine\"  # rebuilt from `{missing}`", False,
         "an apostrophe in prose does not hide the comment behind it"),
        (probe_rs, f"let n = 1; // rebuilt from `{missing}`", False,
         "a dead path in a trailing Rust comment"),
        (probe_rs, f"let p = \"rebuilt from `{missing}`\";", True,
         "STAYS GREEN: a Rust string literal is not a comment"),
    ]

    bad = 0
    for path, reachable, label in set_cases:
        ok = (syntax_for(path) is not None) == reachable
        print(f"  {'ok  ' if ok else 'MISS'} {label}")
        if not ok:
            bad += 1
    for body, should_pass, label in cases:
        got = not check([(probe_rs, [(1, body)])])
        ok = got == should_pass
        print(f"  {'ok  ' if ok else 'MISS'} {label}")
        if not ok:
            bad += 1
    for path, line, should_pass, label in line_cases:
        got = not check([(path, comment_lines(path, line))])
        ok = got == should_pass
        print(f"  {'ok  ' if ok else 'MISS'} [{path.suffix}] {label}")
        if not ok:
            bad += 1
    total = len(set_cases) + len(cases) + len(line_cases)
    if bad:
        print(f"\nself-test FAILED: {bad} of {total} cases wrong")
        return 1
    print(f"\nself-test passed: {total} derived cases, detection and control both correct")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if "--self-test" in args:
        return self_test()
    if "--all" in args:
        pairs = [(f, all_comment_lines(f)) for f in tracked_files()]
    else:
        pairs = [(f, added_comment_lines(f)) for f in changed_files() if f.exists()]

    failures = check(pairs)
    if failures:
        print("Comment citations that do not resolve:\n")
        for f in failures:
            print(f"  {f}")
        print(
            "\nEach names a symbol or path that is not where the comment says it is.\n"
            "Fix the comment, or — if it is about code outside this repo — register\n"
            "it in ACKNOWLEDGED with the reason."
        )
        return 1
    checked = sum(len(v) for _, v in pairs)
    print(f"Comment citations resolve ({checked} comment lines checked).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
