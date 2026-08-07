#!/usr/bin/env python3
"""Gate the SQL and the sample values the taxonomy publishes.

`labels/definitions_*.yaml` is not private data. `finetype taxonomy --full` emits
it, the type-registry pages render `transform` and `decompose` into copy-button
code blocks, and `samples` is what a reader takes as a worked example of the
type. So a `decompose:` naming a function DuckDB does not have is a paste that
returns `Catalog Error`, and a `samples:` entry that fails its own scheme's check
digit is a worked example that is wrong.

Neither was reachable by an existing gate. `scripts/check_duckdb_catalog.py` and
`scripts/check_sql_examples.py` were built for this defect class but enumerate
`README.md`, `docs/*.md` and the `finetype-duckdb` module doc; `labels/` is
outside their file list.

WHAT IS DERIVED (the sources of truth)
    `duckdb_functions()` and `duckdb_types()`, read from a hermetic DuckDB with
    `json` and the LOCAL extension build loaded. The load is provably local:
    `extension_directory` is an empty temp dir, DuckDB must not report finetype
    as installed, and the loaded `extension_version` must equal the workspace
    `Cargo.toml` version.

    `finetype-core`'s `resolve`, reached through the `checksum-samples` binary
    that `make build-extension` already builds. The check-digit maths is not
    reimplemented here; a scheme `resolve` does not know is reported rather than
    passed.

WHAT IS ASSERTED
    1. A call head in a `transform:` or a `decompose:` value resolves in
       `duckdb_functions()`, or is a bare SQL keyword listed in KEYWORDS.
    2. A cast target type in one of those values resolves in `duckdb_types()`.
    3. A `transform_ext:` value may additionally name a symbol from
       EXTENSION_FUNCTIONS or EXTENSION_TYPES — that field means "requires a
       DuckDB extension", and the registry says which one. A `transform:` or
       `decompose:` value may NOT: those two are the plain-DuckDB surface, and
       the split is what mutation `an extension symbol used where no extension
       is loaded` proves.
    4. A leaf carrying a `checksum:` directive names a scheme `resolve` knows,
       carries at least one sample, and each of its samples satisfies that
       scheme's check digit.
    5. Every line the reader's two top-level regexes are responsible for is one
       of them able to read. A line at column zero that `LEAF_RE` misses, or one
       at two spaces that `PROP_RE` misses, is REFUSED rather than dropped —
       the drop is a green verdict on content nothing read.

WHAT IS *NOT* ASSERTED
    That an expression RUNS. Heads and cast targets are checked; argument
    counts, argument types and the `{col}` substitution are not. An expression
    can pass here and still fail to bind.

    That an expression is CORRECT. `decompose:` returning the wrong field for
    the right type is invisible to this.

    Anything about a leaf with no `checksum:` directive. A `samples:` value under
    a type whose substance is a pattern rather than a check digit is out of
    scope; `validation:` is the surface that would settle those.

REGISTERING A SYMBOL
    An entry in KEYWORDS, EXTENSION_FUNCTIONS or EXTENSION_TYPES asserts what it
    says and no more. To reproduce an EXTENSION_* entry, load that extension and
    query the catalog:

        duckdb -c "INSTALL h3 FROM community; LOAD h3;
                   SELECT lower(function_name) FROM duckdb_functions()
                   WHERE lower(function_name) = 'h3_string_to_h3';"

USAGE
    scripts/check_taxonomy_content.py                 # gate the working tree
    scripts/check_taxonomy_content.py --self-test     # prove the gate detects
    scripts/check_taxonomy_content.py --extension PATH
    scripts/check_taxonomy_content.py --checksum-oracle PATH

Needs the duckdb CLI on PATH and `make build-extension` (which produces both the
extension and the `checksum-samples` binary). It does NOT need the model.

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_EXTENSION = "target/release/finetype.duckdb_extension"
DEFAULT_ORACLE = "target/release/checksum-samples"
LABELS_DIR = "labels"
DEFINITION_GLOB = "definitions_*.yaml"

# The three fields carrying SQL. `transform` and `decompose` are the strict pair:
# they are the plain-DuckDB surface and the two the type-registry page renders as
# copyable code. `transform_ext` is documented in every definitions file's own
# header as the form that requires an extension.
STRICT_FIELDS = ("transform", "decompose")
EXT_FIELD = "transform_ext"
SQL_FIELDS = STRICT_FIELDS + (EXT_FIELD,)


class Fatal(Exception):
    """A derivation failed — nothing downstream is meaningful."""


# ══════════════════════════════════════════════════════════════════════════════
# REGISTRIES — a name that is not a function of a loaded extension
# ══════════════════════════════════════════════════════════════════════════════

# Bare SQL keywords that a call-shape regex cannot tell from a function call.
# Each is bound by DuckDB's parser and has no `duckdb_functions()` row.
KEYWORDS = {
    "CAST": "SQL keyword — CAST(x AS T)",
    "TRY_CAST": "SQL keyword — TRY_CAST(x AS T)",
    "COALESCE": "SQL keyword, bound by the parser and absent from duckdb_functions()",
    "IN": "SQL membership operator — x IN (a, b)",
    "THEN": "CASE branch keyword, here followed by a parenthesised expression",
    "DECIMAL": "a parameterised type name inside a cast target — CAST(x AS DECIMAL(18,2))",
}

# Symbols a `transform_ext:` value may name, each with the DuckDB extension that
# registers it. The docstring's REGISTERING A SYMBOL block is the reproduction.
EXTENSION_FUNCTIONS = {
    "st_point": "spatial",
    "st_geomfromtext": "spatial",
    "h3_string_to_h3": "h3 (community)",
    "h3_cell_to_lat": "h3 (community)",
}

EXTENSION_TYPES = {
    "INET": "inet",
}

# A call head: a bare identifier followed by an open paren. Whitespace between
# the two is allowed, which makes the match wider than SQL usually needs — wider
# is the safe direction, because a spurious match has to be registered above
# rather than passing unseen.
HEAD_RE = re.compile(r"(?<![\w.$])([A-Za-z_][A-Za-z0-9_]*)\s*\(")

# A cast target: the leading identifier of `AS <T>` or `::<T>`. Parameters and
# nested types are not read — `DECIMAL(18,2)` is registered by its head, and the
# `DECIMAL(` that follows is a call shape the KEYWORDS entry covers.
CAST_AS_RE = re.compile(r"\bAS\s+([A-Za-z_][A-Za-z0-9_]*)", re.IGNORECASE)
CAST_COLON_RE = re.compile(r"::\s*([A-Za-z_][A-Za-z0-9_]*)")

# A single-quoted SQL literal. Blanked before any of the three regexes above run,
# so a regex pattern or a format string cannot contribute a head or a type.
SQL_STRING_RE = re.compile(r"'(?:''|[^'])*'")


# ══════════════════════════════════════════════════════════════════════════════
# THE READER — a narrow reader for the shapes labels/*.yaml actually uses
# ══════════════════════════════════════════════════════════════════════════════
#
# Deliberately not a YAML parser: every gate CI runs here is stdlib-only at
# module scope, which is what lets `pyright` and the workflow run the same set
# with no dependency install.
#
#   key: "expr"        an inline scalar, quoted or bare, on ONE line
#   key: >             a folded or literal block, continued at deeper indent
#   key:               a nested mapping, one expression per entry
#     name: "expr"
#   key: null          no expression here — `~` reads the same
#   samples:           a block sequence
#     - "value"
#
# `LEAF_RE` and `PROP_RE` are not two patterns among several: between them they
# decide whether ANYTHING under a key is read, so a spelling either one misses
# takes the whole leaf or the whole property out of the run silently. Both are
# therefore refusals rather than skips — a line at column zero, or at exactly
# two spaces, that its regex does not match stops the gate. Deeper lines are
# reached through the key above them and are judged there instead.
#
# The one exemption is the interior of a multi-line flow collection opened by a
# key already read. `labels/definitions_geography.yaml` opens a `locales: [`
# sequence at two spaces and closes it with a `]` at two spaces, and that
# closer is part of a value rather than a property the reader missed.

LEAF_RE = re.compile(r"^([a-z][a-z0-9_.]*):\s*$")
PROP_RE = re.compile(r"^  ([a-z_]+):(.*)$")
NESTED_RE = re.compile(r"^    ([A-Za-z_][A-Za-z0-9_]*):\s*(.*)$")
ITEM_RE = re.compile(r"^    - (.*)$")


@dataclass
class Expr:
    """One SQL expression harvested from a leaf, and where it came from."""

    field_name: str
    shape: str  # inline | block | nested
    text: str
    first_line: int  # 1-based, inclusive
    last_line: int  # 1-based, inclusive


@dataclass
class Sample:
    line: int
    value: str
    escaped: bool  # the scalar carries a backslash, so `value` is not its text


@dataclass
class Leaf:
    path: Path
    rel: str
    line: int
    key: str
    exprs: list[Expr] = field(default_factory=list)
    checksum: str | None = None
    checksum_line: int = 0
    samples: list[Sample] = field(default_factory=list)


def _unquote(raw: str) -> str:
    """The scalar's text, with one layer of surrounding quotes removed.

    Escape sequences are NOT decoded. A backslash therefore means the returned
    text and the scalar's real text differ, which is not tolerable for a sample
    value: it has to be exact. `Sample.escaped` carries the distinction to
    `_check_samples`, which refuses rather than judging a string it did not
    build.
    """
    if len(raw) >= 2 and raw[0] == raw[-1] and raw[0] in "\"'":
        return raw[1:-1]
    return raw


# `_scalar` does not read a value opening with one of these. > and | open a
# block scalar, [ and { a flow collection, & an anchor, * an alias, ! a tag,
# % a directive, # a comment; @ ` , ] and } cannot open a plain scalar.
UNREAD_INDICATORS = ">|[]{}&*!%@`,#"


def _scalar(raw: str) -> str | None:
    """A one-line scalar's text, or None when this reader cannot read the shape.

    Escape sequences are not decoded — see `_unquote`.
    """
    if not raw:
        return None
    if raw[0] in "\"'":
        if len(raw) >= 2 and raw[-1] == raw[0]:
            return raw[1:-1]
        return None
    if raw[0] in UNREAD_INDICATORS:
        return None
    if " #" in raw or "\t#" in raw:
        return None
    return raw


def _refuse(rel: str, lineno: int, what: str, raw: str) -> Fatal:
    """The refusal for a value shape the reader does not read."""
    shown = raw.strip()[:80]
    return Fatal(
        f"{rel}:{lineno}: {what} has a value shape this reader does not read: "
        f"{shown!r}.\n"
        f"    Write the expression as a quoted scalar on one line, or teach "
        f"{Path(__file__).name} this shape."
    )


def _refuse_line(rel: str, lineno: int, what: str, raw: str) -> Fatal:
    """The refusal for a LINE shape the reader does not recognise.

    `_refuse` answers for a value the reader reached and could not read. This
    answers for a line it would never have reached at all: `LEAF_RE` and
    `PROP_RE` are what decide whether a line is read, so a spelling either one
    misses used to be dropped and the file reported as clean.
    """
    shown = raw.strip()[:80]
    return Fatal(
        f"{rel}:{lineno}: {what} has a line shape this reader does not read: "
        f"{shown!r}.\n"
        f"    Spell the key the way its neighbours are spelled, or teach "
        f"{Path(__file__).name} this shape."
    )


# A quoted scalar of either style. Blanked before flow-collection brackets are
# counted, so a bracket inside a value — a character class in a regex, a format
# string — cannot open or close a collection.
QUOTED_RE = re.compile(r"'(?:''|[^'])*'|\"(?:\\.|[^\"\\])*\"")

# A comment, once quoted spans are blanked: `#` at the start of the line or
# after whitespace. YAML reads one inside a flow collection too.
COMMENT_RE = re.compile(r"(?:^|\s)#")


def _flow_delta(text: str) -> int:
    """Net unclosed `[` and `{` in `text`, ignoring quoted spans and comments."""
    blanked = QUOTED_RE.sub("''", text)
    comment = COMMENT_RE.search(blanked)
    if comment is not None:
        blanked = blanked[: comment.start()]
    opened = sum(1 for char in blanked if char in "[{")
    closed = sum(1 for char in blanked if char in "]}")
    return opened - closed


def _refuse_structural(rel: str, lineno: int, line: str) -> None:
    """Refuse a line at an indent one of the two top-level regexes owns.

    Column zero is where `LEAF_RE` reads a leaf key and two spaces is where
    `PROP_RE` reads its properties. Any other indent belongs to a value this
    reader either harvests through the key above it or does not read at all,
    and is judged there rather than here.
    """
    indent = len(line) - len(line.lstrip(" "))
    if indent == 0:
        raise _refuse_line(rel, lineno, "a leaf key at column zero", line)
    if indent == 2:
        raise _refuse_line(rel, lineno, "a property at two spaces of indent", line)


def _refuse_run_on(
    rel: str, lines: list[str], index: int, name: str, *, started: bool = True
) -> None:
    """Refuse a plain scalar that runs on past the line just read.

    A plain scalar continues onto any following line indented deeper than its
    key, across blank lines, and folds into one value. `PROP_RE` sits at two
    spaces and `LEAF_RE` at none, so a deeper line matches neither and is
    dropped — leaving the first line standing for a value it is only part of.

    `started` is whether the key line already carries a scalar. When it does, a
    comment ends that scalar and PyYAML rejects a deeper line after one, so the
    scan stops at the comment. When the key line is bare the value is still
    pending, so a comment is not part of it and does not end it — a bare
    `checksum:`, a `# note` line and then `iban` loads as `checksum: iban`.
    """
    for offset in range(index + 1, len(lines)):
        line = lines[offset]
        if not line.strip():
            continue
        if line.lstrip().startswith("#"):
            if started:
                return
            continue
        if not line.startswith("   "):
            return
        raise _refuse(rel, offset + 1, f"`{name}:`", line)


def read_definitions(labels_dir: Path) -> list[Leaf]:
    """Every leaf in every definitions file, with its SQL, scheme and samples."""
    files = sorted(labels_dir.glob(DEFINITION_GLOB))
    if not files:
        raise Fatal(f"no {DEFINITION_GLOB} under {labels_dir}")

    leaves: list[Leaf] = []
    for path in files:
        before = len(leaves)
        leaves.extend(_read_file(path, labels_dir))
        if len(leaves) == before:
            raise Fatal(
                f"{path.name}: the reader found no leaf definitions in it — the "
                f"file changed shape, or this reader is broken"
            )
    return leaves


def _read_file(path: Path, labels_dir: Path) -> list[Leaf]:
    rel = path.relative_to(labels_dir.parent).as_posix()
    lines = path.read_text(encoding="utf-8").splitlines()
    leaves: list[Leaf] = []
    current: Leaf | None = None
    # Depth of the multi-line flow collection currently open, and the line that
    # opened it. Clamped at zero on both arms: a stray closer ends the
    # collection rather than driving the count negative, which would exempt
    # every line below it from the structural refusal.
    flow_depth = 0
    flow_line = 0

    for index, line in enumerate(lines):
        lineno = index + 1
        if flow_depth:
            flow_depth = max(flow_depth + _flow_delta(line), 0)
            continue
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        leaf_match = LEAF_RE.match(line)
        if leaf_match:
            current = Leaf(path=path, rel=rel, line=lineno, key=leaf_match.group(1))
            leaves.append(current)
            continue
        prop = PROP_RE.match(line)
        if prop is None:
            _refuse_structural(rel, lineno, line)
            continue
        # Set before the value is judged, so that a key whose value opens a
        # collection carries its interior even when the branches below refuse
        # the value itself.
        flow_depth = max(_flow_delta(prop.group(2)), 0)
        if flow_depth:
            flow_line = lineno
        if current is None:
            continue
        name, rest = prop.group(1), prop.group(2).strip()

        if name == "checksum":
            # `checksum:` with nothing after the colon is the scheme written
            # null, unless a deeper line follows — then that line is the value,
            # and a leaf whose scheme this reader does not see takes its
            # samples out of the check-digit run with it.
            _refuse_run_on(rel, lines, index, name, started=rest != "")
            if rest in ("", "null", "~"):
                continue
            current.checksum = _unquote(rest)
            current.checksum_line = lineno
            continue

        if name == "samples" and rest == "":
            # Scanned the way the nested mapping below is: blank and comment
            # lines are skipped, and the block ends at the first line left that
            # is not indented four spaces. The comment test runs BEFORE the
            # indent test, because a comment shallower than the items would
            # otherwise end the block while PyYAML publishes the items on
            # either side of it. `labels/definitions_technology.yaml:233` is a
            # comment at column 0, and `:253` one at two spaces inside a leaf.
            for offset, item in enumerate(lines[index + 1 :], lineno + 1):
                if not item.strip() or item.lstrip().startswith("#"):
                    continue
                if not item.startswith("    "):
                    break
                item_match = ITEM_RE.match(item)
                if not item_match:
                    raise _refuse(rel, offset, f"the value under `{name}:`", item)
                raw = item_match.group(1).strip()
                current.samples.append(
                    Sample(line=offset, value=_unquote(raw), escaped="\\" in raw)
                )
            continue

        if name not in SQL_FIELDS:
            continue

        if rest in ("null", "~"):
            _refuse_run_on(rel, lines, index, name)
            continue

        if rest.startswith(">") or rest.startswith("|"):
            buf: list[str] = []
            last = lineno
            # Not the order the `samples:` and nested loops use: inside a block
            # scalar a `#` is content rather than a comment, and a line
            # shallower than the block is where the scalar ends.
            for offset, cont in enumerate(lines[index + 1 :], lineno + 1):
                if cont.strip() and not cont.startswith("    "):
                    break
                if cont.strip():
                    buf.append(cont.strip())
                    last = offset
            if not buf:
                raise _refuse(rel, lineno, f"`{name}:`", rest)
            current.exprs.append(Expr(name, "block", " ".join(buf), lineno + 1, last))
            continue

        if rest == "":
            entries = 0
            # Comment before indent, for the reason given at `samples:` above.
            for offset, nested in enumerate(lines[index + 1 :], lineno + 1):
                if not nested.strip() or nested.lstrip().startswith("#"):
                    continue
                if not nested.startswith("    "):
                    break
                nested_match = NESTED_RE.match(nested)
                if not nested_match:
                    raise _refuse(rel, offset, f"the entry under `{name}:`", nested)
                entries += 1
                key, value = nested_match.group(1), nested_match.group(2).strip()
                if value in ("null", "~"):
                    continue
                text = _scalar(value)
                if text is None:
                    raise _refuse(rel, offset, f"`{key}:` under `{name}:`", value)
                current.exprs.append(Expr(name, "nested", text, offset, offset))
            if not entries:
                raise _refuse(rel, lineno, f"`{name}:`", rest)
            continue

        text = _scalar(rest)
        if text is None:
            raise _refuse(rel, lineno, f"`{name}:`", rest)
        _refuse_run_on(rel, lines, index, name)
        current.exprs.append(Expr(name, "inline", text, lineno, lineno))

    if flow_depth:
        raise Fatal(
            f"{rel}:{flow_line}: a flow collection opens here and never closes, so "
            f"every line below it went unread.\n"
            f"    {lines[flow_line - 1].strip()[:80]}"
        )

    return leaves


def assert_reader_alive(leaves: list[Leaf]) -> None:
    """Refuse a read that harvested nothing — an empty result is not a clean tree."""
    if not leaves:
        raise Fatal("the reader found no leaf definitions at all")
    if not any(leaf.exprs for leaf in leaves):
        raise Fatal("the reader harvested no SQL expressions from any leaf")
    if not any(leaf.checksum for leaf in leaves):
        raise Fatal("the reader found no `checksum:` directive on any leaf")
    if not any(leaf.samples for leaf in leaves):
        raise Fatal("the reader harvested no `samples:` values from any leaf")


def heads(text: str) -> list[str]:
    return HEAD_RE.findall(SQL_STRING_RE.sub("''", text))


def cast_targets(text: str) -> list[str]:
    blanked = SQL_STRING_RE.sub("''", text)
    return CAST_AS_RE.findall(blanked) + CAST_COLON_RE.findall(blanked)


# ══════════════════════════════════════════════════════════════════════════════
# DERIVATION — the DuckDB catalog, and the check-digit oracle
# ══════════════════════════════════════════════════════════════════════════════

VERSION_SQL = """
SELECT extension_version, loaded, installed
FROM duckdb_extensions() WHERE extension_name = 'finetype';
"""

FUNCTIONS_SQL = "SELECT DISTINCT lower(function_name) AS name FROM duckdb_functions();"
TYPES_SQL = "SELECT DISTINCT upper(type_name) AS name FROM duckdb_types();"


def _duckdb(sql: str, extension: Path, tmp: Path) -> list:
    """Run one statement against a hermetic DuckDB with `json` and the build loaded."""
    if shutil.which("duckdb") is None:
        raise Fatal("duckdb is not on PATH — this gate reads the DuckDB catalog")
    prelude = f"SET extension_directory='{tmp}';\nLOAD json;\nLOAD '{extension}';\n"
    proc = subprocess.run(
        ["duckdb", "-unsigned", "-no-init", "-json", "-c", prelude + sql],
        capture_output=True,
        text=True,
        env={**os.environ, "DUCKDB_EXTENSION_DIRECTORY": str(tmp)},
    )
    if proc.returncode != 0:
        raise Fatal(f"duckdb exited {proc.returncode}:\n{proc.stderr.strip()}")
    out = proc.stdout.strip()
    if not out:
        return []
    try:
        return json.loads(out)
    except json.JSONDecodeError as exc:
        raise Fatal(f"duckdb did not return JSON ({exc}):\n{out[:400]}") from None


def workspace_version(root: Path) -> str:
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not match:
        raise Fatal('no `version = "..."` in Cargo.toml')
    return match.group(1)


def derive_catalog(root: Path, extension: Path) -> tuple[set[str], set[str]]:
    """(function names lowercased, type names uppercased) of the loaded catalog."""
    if not extension.is_file():
        raise Fatal(f"no extension at {extension}\n    Build it first: make build-extension")
    with tempfile.TemporaryDirectory(prefix="taxonomy-content-") as tmp:
        tmp_path = Path(tmp)
        version_rows = _duckdb(VERSION_SQL, extension, tmp_path)
        function_rows = _duckdb(FUNCTIONS_SQL, extension, tmp_path)
        type_rows = _duckdb(TYPES_SQL, extension, tmp_path)

    if not version_rows:
        raise Fatal("duckdb_extensions() has no `finetype` row — the LOAD did nothing")
    info = version_rows[0]
    if not info["loaded"]:
        raise Fatal(f"the extension at {extension} did not load")
    if info["installed"]:
        raise Fatal(
            "DuckDB reports finetype as INSTALLED as well as loaded, so this run "
            "may be reading a community build rather than the local one"
        )
    expected = workspace_version(root)
    if info["extension_version"] != expected:
        raise Fatal(
            f"the loaded extension reports version {info['extension_version']}, "
            f"the tree is {expected} — this is not a build of this tree.\n"
            f"    Rebuild: make build-extension"
        )

    functions = {row["name"] for row in function_rows}
    types = {row["name"] for row in type_rows}
    if not functions:
        raise Fatal("duckdb_functions() returned nothing")
    if not types:
        raise Fatal("duckdb_types() returned nothing")
    return functions, types


def check_digits(oracle: Path, pairs: list[tuple[str, str]]) -> list[str]:
    """One verdict per (scheme, value) pair, from finetype-core's `resolve`."""
    if not pairs:
        return []
    if not oracle.is_file():
        raise Fatal(
            f"no check-digit oracle at {oracle}\n"
            f"    Build it first: make build-extension"
        )
    payload = "".join(f"{scheme}\t{value}\n" for scheme, value in pairs)
    proc = subprocess.run(
        [str(oracle)], input=payload, capture_output=True, text=True
    )
    if proc.returncode != 0:
        raise Fatal(f"{oracle.name} exited {proc.returncode}:\n{proc.stderr.strip()}")
    verdicts = proc.stdout.splitlines()
    if len(verdicts) != len(pairs):
        raise Fatal(
            f"{oracle.name} answered {len(verdicts)} of {len(pairs)} pairs — "
            f"the batch protocol changed"
        )
    return verdicts


# ══════════════════════════════════════════════════════════════════════════════
# THE GATE
# ══════════════════════════════════════════════════════════════════════════════


def check(
    labels_dir: Path, functions: set[str], types: set[str], oracle: Path
) -> list[str]:
    """Return a list of failures. Empty means the published content resolves."""
    leaves = read_definitions(labels_dir)
    assert_reader_alive(leaves)
    failures: list[str] = []

    for leaf in leaves:
        for expr in leaf.exprs:
            strict = expr.field_name in STRICT_FIELDS
            where = f"{leaf.rel}:{expr.first_line}"
            for head in heads(expr.text):
                if head.lower() in functions or head.upper() in KEYWORDS:
                    continue
                provider = EXTENSION_FUNCTIONS.get(head.lower())
                if provider and not strict:
                    continue
                failures.append(
                    _head_failure(where, leaf, expr, head, provider, strict)
                )
            for target in cast_targets(expr.text):
                if target.upper() in types:
                    continue
                provider = EXTENSION_TYPES.get(target.upper())
                if provider and not strict:
                    continue
                failures.append(
                    _type_failure(where, leaf, expr, target, provider, strict)
                )

    failures.extend(_check_samples(leaves, oracle))
    return failures


def _head_failure(
    where: str, leaf: Leaf, expr: Expr, head: str, provider: str | None, strict: bool
) -> str:
    if provider:
        return (
            f"{where}: `{leaf.key}` names `{head}(` in `{expr.field_name}:`, and "
            f"`{head}` comes from the {provider} extension. Only `{EXT_FIELD}:` "
            f"may name an extension symbol."
        )
    return (
        f"{where}: `{leaf.key}` names `{head}(` in `{expr.field_name}:`, which the "
        f"loaded catalog does not have.\n"
        f"    {expr.text[:140]}\n"
        f"    Fix the expression, or register `{head}` in "
        f"{'KEYWORDS' if strict else 'KEYWORDS/EXTENSION_FUNCTIONS'} in "
        f"{Path(__file__).name} with the reason."
    )


def _type_failure(
    where: str, leaf: Leaf, expr: Expr, target: str, provider: str | None, strict: bool
) -> str:
    if provider:
        return (
            f"{where}: `{leaf.key}` casts to `{target}` in `{expr.field_name}:`, and "
            f"`{target}` comes from the {provider} extension. Only `{EXT_FIELD}:` "
            f"may name an extension symbol."
        )
    return (
        f"{where}: `{leaf.key}` casts to `{target}` in `{expr.field_name}:`, which "
        f"the loaded catalog does not have.\n"
        f"    {expr.text[:140]}\n"
        f"    Fix the expression, or register `{target}` in "
        f"{'KEYWORDS' if strict else 'KEYWORDS/EXTENSION_TYPES'} in "
        f"{Path(__file__).name} with the reason."
    )


def _check_samples(leaves: list[Leaf], oracle: Path) -> list[str]:
    failures: list[str] = []
    pairs: list[tuple[str, str]] = []
    origins: list[tuple[Leaf, Sample]] = []
    for leaf in leaves:
        if not leaf.checksum:
            continue
        if not leaf.samples:
            failures.append(
                f"{leaf.rel}:{leaf.checksum_line}: `{leaf.key}` declares "
                f"`checksum: {leaf.checksum}` and carries no `samples:` value, so "
                f"nothing exercises the scheme"
            )
            continue
        for sample in leaf.samples:
            if sample.escaped:
                failures.append(
                    f"{leaf.rel}:{sample.line}: `{leaf.key}` publishes a sample "
                    f"carrying a backslash, and this reader does not decode escape "
                    f"sequences, so it cannot judge the {leaf.checksum} check digit"
                )
                continue
            pairs.append((leaf.checksum, sample.value))
            origins.append((leaf, sample))

    for verdict, (leaf, sample) in zip(check_digits(oracle, pairs), origins):
        if verdict == "ok":
            continue
        if verdict == "no-such-scheme":
            failures.append(
                f"{leaf.rel}:{leaf.checksum_line}: `{leaf.key}` declares "
                f"`checksum: {leaf.checksum}`, which finetype-core's `resolve` "
                f"does not know"
            )
            continue
        failures.append(
            f"{leaf.rel}:{sample.line}: `{leaf.key}` publishes {sample.value!r} as a "
            f"sample, and it fails the {leaf.checksum} check digit"
        )
    return _dedupe(failures)


def _dedupe(items: list[str]) -> list[str]:
    seen: set[str] = set()
    out: list[str] = []
    for item in items:
        if item not in seen:
            seen.add(item)
            out.append(item)
    return out


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def _edit_line(path: Path, lineno: int, old: str, new: str) -> None:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    if old not in lines[lineno - 1]:
        raise Fatal(f"self-test: {old!r} is not on {path.name}:{lineno}")
    lines[lineno - 1] = lines[lineno - 1].replace(old, new, 1)
    path.write_text("".join(lines), encoding="utf-8")


def _first_expr(labels_dir: Path, field_name: str, shape: str) -> tuple[Leaf, Expr]:
    for leaf in read_definitions(labels_dir):
        for expr in leaf.exprs:
            if expr.field_name == field_name and expr.shape == shape:
                if heads(expr.text):
                    return leaf, expr
    raise Fatal(f"self-test: no {shape} `{field_name}:` expression with a call head")


def _find_token(leaf: Leaf, expr: Expr, token: str) -> int:
    lines = leaf.path.read_text(encoding="utf-8").splitlines()
    for lineno in range(expr.first_line, expr.last_line + 1):
        if token in lines[lineno - 1]:
            return lineno
    raise Fatal(f"self-test: {token!r} is not in {leaf.rel}:{expr.first_line}")


def _rename_head(field_name: str, shape: str, replacement: str | None = None):
    """Rewrite the first call head of the first matching expression."""

    def apply(labels_dir: Path) -> None:
        leaf, expr = _first_expr(labels_dir, field_name, shape)
        head = heads(expr.text)[0]
        new = replacement if replacement is not None else f"no_such_{head.lower()}"
        _edit_line(leaf.path, _find_token(leaf, expr, head + "("), head + "(", new + "(")

    return apply


def _rename_cast_target(replacement: str):
    def apply(labels_dir: Path) -> None:
        for leaf in read_definitions(labels_dir):
            for expr in leaf.exprs:
                if expr.field_name not in STRICT_FIELDS:
                    continue
                targets = cast_targets(expr.text)
                if not targets:
                    continue
                target = targets[0]
                _edit_line(
                    leaf.path, _find_token(leaf, expr, target), target, replacement
                )
                return
        raise Fatal("self-test: no strict expression with a cast target")

    return apply


def _fold_nested_entry(labels_dir: Path) -> None:
    """Rewrite the first nested `decompose:` entry as a folded block scalar."""
    leaf, expr = _first_expr(labels_dir, "decompose", "nested")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    match = NESTED_RE.match(lines[expr.first_line - 1].rstrip("\n"))
    if match is None:
        raise Fatal(f"self-test: {leaf.rel}:{expr.first_line} is not a nested entry")
    lines[expr.first_line - 1] = (
        f"    {match.group(1)}: >\n      NO_SUCH_FUNCTION_AT_ALL({{col}})\n"
    )
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _flow_sequence_transform(labels_dir: Path) -> None:
    """Rewrite the first inline `transform:` as a flow sequence of itself.

    The expression is left resolvable, so what has to be caught is the shape.
    """
    leaf, expr = _first_expr(labels_dir, "transform", "inline")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[expr.first_line - 1] = f'  transform: ["{expr.text}"]\n'
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _run_on_inline_transform(labels_dir: Path) -> None:
    """Break the first inline `transform:` across two lines, unquoted.

    The break is at the last space, so PyYAML folds the pair back into the
    expression the leaf started with and what has to be caught is the shape.
    """
    leaf, expr = _first_expr(labels_dir, "transform", "inline")
    head, _, tail = expr.text.rpartition(" ")
    if not head:
        raise Fatal(f"self-test: {expr.text!r} has no space to break at")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[expr.first_line - 1] = f"  transform: {head}\n    {tail}\n"
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _stray_space_before_colon(labels_dir: Path) -> None:
    """Put a space before the colon of the first inline `transform:`.

    PyYAML publishes `  transform : "…"` as the `transform` key, and it is a
    spelling `PROP_RE` misses — `[a-z_]+` cannot reach the colon across the
    space. The value planted is unresolvable, so a reader that drops the line
    is dropping a call to a function the catalog does not have.
    """
    leaf, expr = _first_expr(labels_dir, "transform", "inline")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[expr.first_line - 1] = '  transform : "NO_SUCH_FUNCTION_AT_ALL({col})"\n'
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _first_leaf_key_line(labels_dir: Path) -> tuple[Path, int, str]:
    """The first line of the first definitions file that `LEAF_RE` reads."""
    for path in sorted(labels_dir.glob(DEFINITION_GLOB)):
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if LEAF_RE.match(line):
                return path, lineno, line
    raise Fatal("self-test: no definitions file carries a leaf key")


def _capitalise_leaf_key(labels_dir: Path) -> None:
    """Capitalise the first leaf key of the first definitions file.

    `LEAF_RE` anchors on `[a-z]`, so PyYAML publishes the leaf and this reader
    never sees the key. The FIRST key in the file is the one taken because no
    leaf is open above it: every property below then belongs to nothing and is
    dropped with the key, which is the fail-open in its cleanest form.
    """
    path, lineno, line = _first_leaf_key_line(labels_dir)
    _edit_line(path, lineno, line[0], line[0].upper())


def _orphan_flow_closer(labels_dir: Path) -> None:
    """Put a flow-sequence closer at two spaces with no flow sequence open.

    The exemption `labels/definitions_geography.yaml` needs is for the interior
    of a collection that is OPEN. A rule written instead as "a `]` at two
    spaces is fine" passes that file and passes this line too, so this is what
    separates the two.
    """
    leaf, expr = _first_expr(labels_dir, "transform", "inline")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines.insert(expr.first_line, "  ]\n")
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _unterminated_flow(labels_dir: Path) -> None:
    """Delete the closing line of a multi-line flow collection.

    Everything below the opener then reads as that collection's interior. The
    exemption has to end at the end of the file rather than carrying the rest
    of it, or it is a way to switch the structural refusal off.
    """
    for path in sorted(labels_dir.glob(DEFINITION_GLOB)):
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        depth = 0
        for index, line in enumerate(lines):
            if depth:
                depth = max(depth + _flow_delta(line), 0)
                if not depth:
                    del lines[index]
                    path.write_text("".join(lines), encoding="utf-8")
                    return
                continue
            prop = PROP_RE.match(line.rstrip("\n"))
            if prop is not None:
                depth = max(_flow_delta(prop.group(2)), 0)
    raise Fatal("self-test: no multi-line flow collection to leave open")


def _first_null_sql_prop(labels_dir: Path) -> tuple[Path, int, str]:
    """The first `transform:`, `decompose:` or `transform_ext:` set null or `~`."""
    for path in sorted(labels_dir.glob(DEFINITION_GLOB)):
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            prop = PROP_RE.match(line)
            if prop is None or prop.group(1) not in SQL_FIELDS:
                continue
            if prop.group(2).strip() in ("null", "~"):
                return path, lineno, prop.group(1)
    raise Fatal("self-test: no SQL field is written null")


def _run_on_null_field(labels_dir: Path) -> None:
    """Continue a null SQL field onto a second line.

    PyYAML folds the two into one string, so the field carries an expression
    while the reader reads the leaf as having none there.
    """
    path, lineno, name = _first_null_sql_prop(labels_dir)
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[lineno - 1] = f"  {name}: null\n    NO_SUCH_FUNCTION_AT_ALL({{col}})\n"
    path.write_text("".join(lines), encoding="utf-8")


def _run_on_checksum(labels_dir: Path) -> None:
    """Continue a `checksum:` directive onto a second line."""
    leaf = _first_checksum_leaf(labels_dir)
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[leaf.checksum_line - 1] = f"  checksum: {leaf.checksum}\n    no_such_scheme\n"
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _bare_checksum_key(filler: str):
    """Move a `checksum:` value below its key, with `filler` in between.

    PyYAML folds the two lines into `checksum: <scheme>`, so the leaf still
    declares a scheme. A reader that judges the key line alone sees `checksum:`
    written null and drops the leaf from the check-digit run with its samples.
    """

    def apply(labels_dir: Path) -> None:
        leaf = _first_checksum_leaf(labels_dir)
        lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
        lines[leaf.checksum_line - 1] = f"  checksum:\n{filler}    {leaf.checksum}\n"
        leaf.path.write_text("".join(lines), encoding="utf-8")

    return apply


def _under_indent_block(labels_dir: Path) -> None:
    """Re-indent a folded `transform:` block's continuation to three spaces."""
    leaf, expr = _first_expr(labels_dir, "transform", "block")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    for lineno in range(expr.first_line, expr.last_line + 1):
        lines[lineno - 1] = "   " + lines[lineno - 1].lstrip()
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _under_indent_nested(labels_dir: Path) -> None:
    """Re-indent a `decompose:` mapping's entries to three spaces."""
    leaf, expr = _first_expr(labels_dir, "decompose", "nested")
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    start = expr.first_line - 1
    while start > 0 and NESTED_RE.match(lines[start - 1].rstrip("\n")):
        start -= 1
    while start < len(lines) and NESTED_RE.match(lines[start].rstrip("\n")):
        lines[start] = "   " + lines[start].lstrip()
        start += 1
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _first_checksum_leaf(labels_dir: Path) -> Leaf:
    for leaf in read_definitions(labels_dir):
        if leaf.checksum and leaf.samples:
            return leaf
    raise Fatal("self-test: no leaf carries both a `checksum:` and a sample")


def _bump_digit(value: str) -> str:
    """`value` with its last digit moved on by one, so it fails its own scheme."""
    digits = [index for index, char in enumerate(value) if char.isdigit()]
    if not digits:
        raise Fatal(f"self-test: {value!r} has no digit to bump")
    last = digits[-1]
    return value[:last] + str((int(value[last]) + 1) % 10) + value[last + 1 :]


def _corrupt_sample(labels_dir: Path) -> None:
    """Bump the last digit of the first sample of the first checksum-bearing leaf."""
    leaf = _first_checksum_leaf(labels_dir)
    sample = leaf.samples[0]
    _edit_line(leaf.path, sample.line, sample.value, _bump_digit(sample.value))


def _run_on_sample(labels_dir: Path) -> None:
    """Continue the first sample of the first checksum leaf onto a second line.

    Line one is left whole, so the value a reader that stops there still
    satisfies its check digit. What such a reader loses is the rest of the
    published value, and every sample after this one.
    """
    leaf = _first_checksum_leaf(labels_dir)
    sample = leaf.samples[0]
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    lines[sample.line - 1] = f"    - {sample.value}\n      NO_SUCH_SAMPLE_TAIL\n"
    leaf.path.write_text("".join(lines), encoding="utf-8")


def _interrupt_samples(filler: str):
    """Put `filler` after the first sample and corrupt the last one.

    YAML publishes the items either side of a blank line or a comment, so both
    have to be read past. The corrupted sample is the evidence that they were:
    it sits after the filler, and on its own it is caught.
    """

    def apply(labels_dir: Path) -> None:
        leaf = _first_checksum_leaf(labels_dir)
        if len(leaf.samples) < 2:
            raise Fatal(f"self-test: {leaf.key} carries too few samples to split")
        last = leaf.samples[-1]
        _edit_line(leaf.path, last.line, last.value, _bump_digit(last.value))
        lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
        lines.insert(leaf.samples[0].line, filler)
        leaf.path.write_text("".join(lines), encoding="utf-8")

    return apply


def _interrupt_nested(filler: str):
    """Put `filler` above the last entry of a nested `decompose:`, and break it.

    The same mutation as `_interrupt_samples`, aimed at the other block loop:
    the broken entry sits after the filler, so a reader that stops at the
    filler does not reach it.
    """

    def apply(labels_dir: Path) -> None:
        for leaf in read_definitions(labels_dir):
            entries = [
                expr
                for expr in leaf.exprs
                if expr.field_name == "decompose" and expr.shape == "nested"
            ]
            if len(entries) < 2 or not heads(entries[-1].text):
                continue
            target = entries[-1]
            head = heads(target.text)[0]
            _edit_line(
                leaf.path, target.first_line, head + "(", "NO_SUCH_FUNCTION_AT_ALL("
            )
            lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
            lines.insert(target.first_line - 1, filler)
            leaf.path.write_text("".join(lines), encoding="utf-8")
            return
        raise Fatal("self-test: no `decompose:` mapping has two entries to split")

    return apply


def _rename_scheme(labels_dir: Path) -> None:
    leaf = _first_checksum_leaf(labels_dir)
    _edit_line(leaf.path, leaf.checksum_line, leaf.checksum or "", "no_such_scheme")


def _empty_samples(labels_dir: Path) -> None:
    leaf = _first_checksum_leaf(labels_dir)
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    drop = {sample.line for sample in leaf.samples}
    kept = [line for index, line in enumerate(lines, 1) if index not in drop]
    leaf.path.write_text("".join(kept), encoding="utf-8")


def self_test(root: Path, functions: set[str], types: set[str], oracle: Path) -> int:
    """Mutate a pristine copy of labels/ and require each mutation to be caught."""
    ext_function = sorted(EXTENSION_FUNCTIONS)[0]
    ext_type = sorted(EXTENSION_TYPES)[0]

    cases: list[tuple[str, Callable[[Path], None], str]] = [
        (
            "an inline transform: names a function that does not exist",
            _rename_head("transform", "inline"),
            "which the loaded catalog does not have",
        ),
        (
            "a nested decompose: entry names a function that does not exist",
            _rename_head("decompose", "nested"),
            "in `decompose:`, which the loaded catalog does not have",
        ),
        (
            "an inline decompose: names a function that does not exist",
            _rename_head("decompose", "inline"),
            "in `decompose:`, which the loaded catalog does not have",
        ),
        (
            "a folded block transform: names a function that does not exist",
            _rename_head("transform", "block"),
            "which the loaded catalog does not have",
        ),
        (
            "a transform_ext: names a function no extension in the registry provides",
            _rename_head("transform_ext", "inline"),
            "which the loaded catalog does not have",
        ),
        (
            "an extension function is used where no extension is loaded",
            _rename_head("transform", "inline", replacement=ext_function),
            "may name an extension symbol",
        ),
        (
            "a cast names a type that does not exist",
            _rename_cast_target("no_such_type"),
            "casts to `no_such_type`",
        ),
        (
            "an extension type is used where no extension is loaded",
            _rename_cast_target(ext_type),
            "may name an extension symbol",
        ),
        (
            "a published sample fails its own check digit",
            _corrupt_sample,
            "fails the",
        ),
        (
            "a checksum: names a scheme finetype-core does not know",
            _rename_scheme,
            "does not know",
        ),
        (
            "a checksum leaf loses every sample",
            _empty_samples,
            "carries no `samples:` value",
        ),
        (
            "a nested decompose: entry is folded into a block scalar",
            _fold_nested_entry,
            "under `decompose:` has a value shape this reader does not read",
        ),
        (
            "an inline transform: is written as a flow sequence",
            _flow_sequence_transform,
            "`transform:` has a value shape this reader does not read: '[",
        ),
        (
            "a folded transform: block is continued below the reader's indent",
            _under_indent_block,
            "`transform:` has a value shape this reader does not read: '>'",
        ),
        (
            "a decompose: mapping's entries sit below the reader's indent",
            _under_indent_nested,
            "`decompose:` has a value shape this reader does not read: ''",
        ),
        (
            "an inline transform: is broken across two lines, unquoted",
            _run_on_inline_transform,
            "`transform:` has a value shape this reader does not read",
        ),
        (
            "a null SQL field is continued onto a second line",
            _run_on_null_field,
            "value shape this reader does not read: 'NO_SUCH_FUNCTION_AT_ALL({col})'",
        ),
        (
            "a checksum: directive is continued onto a second line",
            _run_on_checksum,
            "`checksum:` has a value shape this reader does not read: 'no_such_scheme'",
        ),
        (
            "a checksum: key is left bare and its scheme put on the next line",
            _bare_checksum_key(""),
            "`checksum:` has a value shape this reader does not read",
        ),
        (
            "...and again with a comment between the bare key and the scheme",
            _bare_checksum_key("  # a note\n"),
            "`checksum:` has a value shape this reader does not read",
        ),
        (
            "a samples: value is continued onto a second line",
            _run_on_sample,
            "under `samples:` has a value shape this reader does not read: "
            "'NO_SUCH_SAMPLE_TAIL'",
        ),
        # The two top-level regexes, and the one exemption they carry. These
        # mutate the LINE rather than the value: each is a spelling PyYAML
        # publishes and the regex misses, which used to be dropped in silence.
        (
            "a property key carries a stray space before its colon",
            _stray_space_before_colon,
            "a property at two spaces of indent has a line shape this reader "
            "does not read",
        ),
        (
            "a leaf key at column zero is capitalised",
            _capitalise_leaf_key,
            "a leaf key at column zero has a line shape this reader does not read",
        ),
        (
            "a flow-sequence closer sits at two spaces with no sequence open",
            _orphan_flow_closer,
            "a property at two spaces of indent has a line shape this reader "
            "does not read: ']'",
        ),
        (
            "a multi-line flow sequence loses its closer",
            _unterminated_flow,
            "a flow collection opens here and never closes",
        ),
        (
            "a blank line sits between two samples, and a later one is corrupt",
            _interrupt_samples("\n"),
            "fails the",
        ),
        (
            "a blank line sits between two decompose: entries, and the next is broken",
            _interrupt_nested("\n"),
            "in `decompose:`, which the loaded catalog does not have",
        ),
    ]

    # The comment column is swept rather than fixed. A reader that tests the
    # indent before the comment reads past one at four spaces and ends the
    # block at a shallower one, so a case pinned at four alone leaves that
    # reader green. `labels/definitions_technology.yaml:233` is a comment at
    # column 0, and `:253` one at two spaces inside a leaf.
    for column in (0, 2, 3, 4):
        pad = " " * column + "# a note\n"
        cases.append(
            (
                f"a comment at column {column} sits between two samples, "
                f"and a later one is corrupt",
                _interrupt_samples(pad),
                "fails the",
            )
        )
        cases.append(
            (
                f"a comment at column {column} sits between two decompose: entries, "
                f"and the next is broken",
                _interrupt_nested(pad),
                "in `decompose:`, which the loaded catalog does not have",
            )
        )

    print(
        f"self-test: {len(functions)} catalog functions, {len(types)} catalog types, "
        f"oracle {oracle.name}"
    )

    failed = 0
    with tempfile.TemporaryDirectory(prefix="taxonomy-content-selftest-") as tmp:
        pristine = Path(tmp) / "pristine" / LABELS_DIR
        shutil.copytree(root / LABELS_DIR, pristine)

        control = check(pristine, functions, types, oracle)
        if control:
            print("  CONTROL FAILED — the unmutated tree does not pass:")
            for failure in control:
                print(f"      {failure}")
            return 1
        print("  ok   control: unmutated tree passes")

        for name, mutate, expected in cases:
            work = Path(tmp) / "work"
            if work.exists():
                shutil.rmtree(work)
            shutil.copytree(pristine.parent, work)
            try:
                mutate(work / LABELS_DIR)
                found = check(work / LABELS_DIR, functions, types, oracle)
                text = "\n".join(found)
            except Fatal as exc:
                text = str(exc)
                found = [text]
            if not found:
                print(f"  MISS {name}: mutation survived")
                failed += 1
            elif expected not in text:
                print(f"  WRONG {name}: caught, but not for the stated reason")
                print(f"      expected to see: {expected}")
                for line in text.splitlines()[:6]:
                    print(f"      got: {line}")
                failed += 1
            else:
                print(f"  ok   {name}")

    if failed:
        print(f"\nself-test FAILED: {failed} of {len(cases)} mutations not detected")
        return 1
    print(f"\nself-test passed: {len(cases)} mutations detected, control clean")
    return 0


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument(
        "--extension",
        type=Path,
        default=None,
        help=f"the built extension (default: {DEFAULT_EXTENSION})",
    )
    parser.add_argument(
        "--checksum-oracle",
        type=Path,
        default=None,
        help=f"the check-digit binary (default: {DEFAULT_ORACLE})",
    )
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    extension = (args.extension or (root / DEFAULT_EXTENSION)).resolve()
    oracle = (args.checksum_oracle or (root / DEFAULT_ORACLE)).resolve()

    try:
        functions, types = derive_catalog(root, extension)
    except Fatal as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if args.self_test:
        try:
            return self_test(root, functions, types, oracle)
        except Fatal as exc:
            print(f"FATAL: {exc}", file=sys.stderr)
            return 2

    try:
        failures = check(root / LABELS_DIR, functions, types, oracle)
    except Fatal as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if failures:
        print(
            "The taxonomy publishes content the loaded DuckDB cannot resolve:\n",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(f"\n{len(failures)} problem(s).", file=sys.stderr)
        return 1

    leaves = read_definitions(root / LABELS_DIR)
    expressions = sum(len(leaf.exprs) for leaf in leaves)
    scheme_leaves = [leaf for leaf in leaves if leaf.checksum]
    samples = sum(len(leaf.samples) for leaf in scheme_leaves)
    print(
        f"The taxonomy's published content resolves: {expressions} SQL expressions "
        f"over {len(leaves)} leaves, and {samples} samples across "
        f"{len(scheme_leaves)} check-digit leaves."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
