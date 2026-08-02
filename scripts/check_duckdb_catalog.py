#!/usr/bin/env python3
"""Gate the documented DuckDB surface against the catalog of a LOADED local build.

The extension's surface — which functions exist, what kind each is, what each
returns — is asserted in prose in README.md, docs/ARCHITECTURE.md,
docs/DEVELOPMENT.md and the `finetype-duckdb` module doc, and prose is where it
has gone wrong: a crate table said "5 scalar functions" when the entrypoint
registers 13 scalars, 2 table macros and 1 table function, and a return type was
described as a STRUCT where the catalog says VARCHAR.

DuckDB settles all of it mechanically. `duckdb_functions()` reports every
function a loaded extension registered, with its kind, its overloads and its
return type — including the STRUCT field names. This loads the local build and
compares.

WHAT IS DERIVED (the source of truth)
    `duckdb_functions()` after `LOAD <the local artifact>`, taken as:

        scalars ......... name → return type + the set of parameter-type tuples
        aggregates ...... name → return type + the set of parameter-type tuples
        table functions . name → parameter-type tuples
        table macros .... name → parameter names
        struct fields ... for a scalar or aggregate returning STRUCT, its field
                          names in order

    The load is hermetic and provably local: `extension_directory` is pointed at
    an empty temp dir, so an installed community build cannot be picked up, and
    the loaded `extension_version` must equal the workspace `Cargo.toml` version.
    A machine with community finetype installed is the normal case (this one had
    0.6.23 installed while the tree built 0.6.54), so this is not paranoia.

WHAT IS ASSERTED
    1. The ARCHITECTURE function table is the registry: every function in the
       catalog must have a row, and every row must name a function in the
       catalog. A renamed, removed or newly-added function reddens CI.
    2. That table's Returns column must equal the catalog's return type
       VERBATIM — the whole STRUCT signature, field names included.
    3. The `finetype-duckdb` module doc must list the same function names.
    4. Prose counts ("N scalar functions", "N table macros") must equal the
       derived counts.
    5. Documented STRUCT literals must have exactly the catalog's field names.
    6. Fail-closed: any `name(` call shape in a scanned doc whose name looks like
       part of this surface must be a function the catalog has — or be listed in
       NOT_FUNCTIONS with the reason it is not one.

WHAT IS *NOT* ASSERTED
    Behaviour. This says a function exists and returns what the docs claim; it
    says nothing about the value coming back. Whether the documented EXAMPLES run
    and produce the documented shape is scripts/check_sql_examples.py.

    Nothing about the community build. This deliberately never touches it — the
    question here is whether the docs match the code in this tree.

    finetype_spike is registered but is a spike artefact the docs deliberately do
    not teach; it is listed in UNDOCUMENTED with that reason rather than being
    silently skipped.

USAGE
    scripts/check_duckdb_catalog.py                  # gate the working tree
    scripts/check_duckdb_catalog.py --catalog        # print the derived surface
    scripts/check_duckdb_catalog.py --self-test      # prove the gate detects
    scripts/check_duckdb_catalog.py --extension PATH # a build elsewhere

Needs the duckdb CLI on PATH and a built extension (`make build-extension`). It
does NOT need the model: LOAD populates the catalog on its own.

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
from pathlib import Path

DEFAULT_EXTENSION = "target/release/finetype.duckdb_extension"

# Every function name this surface uses starts with one of these.
SURFACE_PREFIXES = ("ft_", "finetype")


class Fatal(Exception):
    """The catalog could not be read — nothing downstream is meaningful."""


# ══════════════════════════════════════════════════════════════════════════════
# DERIVATION — load the local build, read duckdb_functions()
# ══════════════════════════════════════════════════════════════════════════════

CATALOG_SQL = """
SELECT function_name, function_type, return_type, parameters, parameter_types
FROM duckdb_functions()
WHERE function_name LIKE 'ft\\_%' ESCAPE '\\'
   OR function_name LIKE 'finetype%'
ORDER BY function_name, parameter_types;
"""

VERSION_SQL = """
SELECT extension_version, loaded, installed
FROM duckdb_extensions() WHERE extension_name = 'finetype';
"""


def workspace_version(root: Path) -> str:
    """The version the local build stamps — `[workspace.package] version`."""
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'^version\s*=\s*"([^"]+)"', text, re.MULTILINE)
    if not match:
        raise Fatal("no `version = \"...\"` in Cargo.toml")
    return match.group(1)


def _duckdb(sql: str, extension: Path, tmp: Path) -> list:
    """Run one statement against a hermetic DuckDB with the local build loaded."""
    if shutil.which("duckdb") is None:
        raise Fatal("duckdb is not on PATH — this gate reads the extension catalog")
    prelude = (
        f"SET extension_directory='{tmp}';\n"
        f"LOAD '{extension}';\n"
    )
    proc = subprocess.run(
        ["duckdb", "-unsigned", "-no-init", "-json", "-c", prelude + sql],
        capture_output=True,
        text=True,
        # An empty HOME cannot resolve `~/.duckdb`; extension_directory above
        # already redirects it, this is belt and braces against a stray config.
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


STRUCT_FIELD_RE = re.compile(r'(?:"(?P<quoted>[^"]+)"|(?P<bare>[A-Za-z_][A-Za-z0-9_]*))\s+')


def struct_fields(return_type: str) -> list[str] | None:
    """Field names of a `STRUCT(a INT, "b" VARCHAR)` return type, in order."""
    if not return_type.startswith("STRUCT("):
        return None
    inner = return_type[len("STRUCT(") : return_type.rindex(")")]
    fields = []
    depth = 0
    current = ""
    for char in inner:
        if char == "," and depth == 0:
            fields.append(current)
            current = ""
            continue
        if char in "(<":
            depth += 1
        elif char in ")>":
            depth -= 1
        current += char
    fields.append(current)
    names = []
    for field in fields:
        match = STRUCT_FIELD_RE.match(field.strip())
        if not match:
            raise Fatal(f"cannot read a field name out of {field.strip()!r}")
        names.append(match.group("quoted") or match.group("bare"))
    return names


def derive(root: Path, extension: Path) -> dict:
    """Load the local build and read its registered surface out of the catalog."""
    if not extension.is_file():
        raise Fatal(
            f"no extension at {extension}\n"
            f"    Build it first: make build-extension"
        )
    with tempfile.TemporaryDirectory(prefix="duckdb-catalog-") as tmp:
        tmp_path = Path(tmp)
        version_rows = _duckdb(VERSION_SQL, extension, tmp_path)
        rows = _duckdb(CATALOG_SQL, extension, tmp_path)

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

    scalars: dict[str, dict] = {}
    aggregates: dict[str, dict] = {}
    table_functions: dict[str, list] = {}
    table_macros: dict[str, list] = {}
    for row in rows:
        name = row["function_name"]
        params = tuple(row["parameter_types"] or [])
        kind = row["function_type"]
        if kind in ("scalar", "aggregate"):
            bucket = scalars if kind == "scalar" else aggregates
            entry = bucket.setdefault(
                name, {"return_type": row["return_type"], "overloads": []}
            )
            if entry["return_type"] != row["return_type"]:
                raise Fatal(
                    f"{name} has overloads with different return types "
                    f"({entry['return_type']} and {row['return_type']}); this gate "
                    f"documents one return type per function"
                )
            entry["overloads"].append(params)
        elif kind == "table_macro":
            table_macros[name] = list(row["parameters"] or [])
        elif kind == "table":
            table_functions.setdefault(name, []).append(params)
        else:
            raise Fatal(f"{name}: unhandled function_type {kind!r}")

    if not scalars:
        raise Fatal("the catalog lists no finetype scalar functions at all")

    for entry in list(scalars.values()) + list(aggregates.values()):
        entry["overloads"].sort()
        entry["struct_fields"] = struct_fields(entry["return_type"])

    return {
        "version": info["extension_version"],
        "scalars": scalars,
        "aggregates": aggregates,
        "table_functions": table_functions,
        "table_macros": table_macros,
    }


def all_names(catalog: dict) -> set[str]:
    return (
        set(catalog["scalars"])
        | set(catalog["aggregates"])
        | set(catalog["table_functions"])
        | set(catalog["table_macros"])
    )


def registrations(catalog: dict) -> set[tuple[str, str, str]]:
    """Every (name, kind, return type) the loaded extension registers.

    A name can appear twice: `ft_profile` is an aggregate over a column *and* a
    table macro, and the docs have to be able to say both.
    """
    out = {
        (name, SCALAR, entry["return_type"])
        for name, entry in catalog["scalars"].items()
    }
    out |= {
        (name, AGGREGATE, entry["return_type"])
        for name, entry in catalog["aggregates"].items()
    }
    out |= {(name, TABLE_MACRO, "TABLE") for name in catalog["table_macros"]}
    out |= {(name, TABLE_FUNCTION, "TABLE") for name in catalog["table_functions"]}
    return out


# ══════════════════════════════════════════════════════════════════════════════
# THE DOCUMENTED SURFACE
# ══════════════════════════════════════════════════════════════════════════════

# The registry table. Each row names a function, its kind and its return type;
# the (name, kind, returns) triples must be exactly the catalog's. Three columns
# rather than two because `ft_profile` is registered TWICE — as an aggregate over
# a column and as a table macro — and a one-row-per-name table cannot say that.
SURFACE_FILE = "docs/ARCHITECTURE.md"
SURFACE_HEADER_RE = re.compile(
    r"^\|\s*Function\s*\|\s*Kind\s*\|\s*Returns\s*\|", re.IGNORECASE
)
SURFACE_ROW_RE = re.compile(
    r"^\|\s*`(?P<name>[a-z_]+)\((?P<args>[^)]*)\)`[^|]*\|\s*(?P<kind>[^|]+?)\s*\|"
    r"\s*(?P<returns>[^|]+?)\s*\|"
)

SCALAR, AGGREGATE = "scalar", "aggregate"
TABLE_MACRO, TABLE_FUNCTION = "table macro", "table function"

# The module doc's bullet list of the surface.
MODULE_DOC_FILE = "crates/finetype-duckdb/src/lib.rs"
MODULE_DOC_RE = re.compile(r"^//! - `(?P<name>[a-z_]+)\(")

# Registered but deliberately untaught.
UNDOCUMENTED = {
    "finetype_spike": (
        "a spike artefact kept for the compile-time evidence that vtab is active "
        "under loadable-extension; the entrypoint says so and no doc teaches it"
    ),
}

# Prose counts of the surface.
COUNT_CLAIMS: list[tuple[str, str, dict[str, str]]] = [
    (
        "docs/ARCHITECTURE.md",
        r"DuckDB extension: (?P<s>\d+) scalar functions \+ (?P<a>\d+) aggregate"
        r" \+ (?P<m>\d+) table macros",
        {"s": "scalars", "a": "aggregates", "m": "table_macros"},
    ),
    (
        "README.md",
        r"(?P<s>\d+) scalar functions, (?P<a>\d+) aggregate and (?P<m>\d+) table macros",
        {"s": "scalars", "a": "aggregates", "m": "table_macros"},
    ),
]

# Documented STRUCT literals: the key set must be the catalog's field names.
# Values are never asserted — a confidence digit is not reproducible.
STRUCT_CLAIMS: list[tuple[str, str, str]] = [
    (
        "docs/DEVELOPMENT.md",
        r"-- \{(?P<fields>'valid'[^}]*)\}",
        "ft_validate_text",
    ),
]
STRUCT_KEY_RE = re.compile(r"'(?P<key>[a-z_]+)'\s*:")

# Files whose prose is scanned for call shapes.
def scanned_files(root: Path) -> list[Path]:
    files = [root / "README.md", root / MODULE_DOC_FILE]
    files.extend(sorted((root / "docs").glob("*.md")))
    return [f for f in files if f.is_file()]


CALL_SHAPE_RE = re.compile(r"(?<![\w.])(?P<name>ft_[a-z_]+|finetype[a-z_]*)\s*\(")

# Names that match the call shape but are not functions of this extension. Being
# listed asserts only that — not that the surrounding prose is right.
NOT_FUNCTIONS = {
    "finetype_core": "a crate",
    "finetype_duckdb": "a crate (and the cargo package name)",
    "finetype_model": "a crate",
    "finetype_init_c_api": "the C entrypoint symbol baked by the entrypoint macro",
    "finetype_reject_errors": "the sidecar TABLE the CLI writes, not a function",
}


# ══════════════════════════════════════════════════════════════════════════════
# THE GATE
# ══════════════════════════════════════════════════════════════════════════════


def check(root: Path, catalog: dict) -> list[str]:
    """Return a list of failures. Empty means the docs match the catalog."""
    failures: list[str] = []
    derived_rows = registrations(catalog)
    expected = {row for row in derived_rows if row[0] not in UNDOCUMENTED}

    # ── the registry table ───────────────────────────────────────────────────
    surface_path = root / SURFACE_FILE
    documented: list[tuple[str, str, str, int]] = []
    if not surface_path.is_file():
        failures.append(f"{SURFACE_FILE}: missing — the surface table cannot be checked")
    else:
        documented = _read_surface_table(surface_path)
        if not documented:
            failures.append(
                f"{SURFACE_FILE}: no `| Function | Kind | Returns |` table — restore "
                f"it or drop it from this gate deliberately"
            )

    documented_rows = {(name, kind, _normalise(ret)) for name, kind, ret, _ in documented}
    normalised_derived = {(name, kind, _normalise(ret)) for name, kind, ret in derived_rows}
    normalised_expected = {(name, kind, _normalise(ret)) for name, kind, ret in expected}

    for name, kind, ret in sorted(normalised_expected - documented_rows):
        also_named = [row for row in documented_rows if row[0] == name and row[1] == kind]
        if also_named:
            stated = also_named[0][2]
            lineno = next(ln for n, k, _, ln in documented if n == name and k == kind)
            failures.append(
                f"{SURFACE_FILE}:{lineno}: `{name}` ({kind}) is documented as "
                f"returning {stated}, the loaded extension returns {ret}"
            )
        else:
            failures.append(
                f"{SURFACE_FILE}: no row for `{name}` as a {kind} — the loaded "
                f"extension registers one, returning {ret}"
            )
    for name, kind, ret in sorted(documented_rows - normalised_derived):
        if any(row[0] == name and row[1] == kind for row in normalised_derived):
            continue  # already reported above as a return-type drift
        lineno = next(
            ln for n, k, r, ln in documented if (n, k, _normalise(r)) == (name, kind, ret)
        )
        failures.append(
            f"{SURFACE_FILE}:{lineno}: a row for `{name}` as a {kind}, which the "
            f"loaded extension does not register"
        )
    for name, _, _, lineno in documented:
        if name in UNDOCUMENTED:
            failures.append(
                f"{SURFACE_FILE}:{lineno}: a row for `{name}`, which is listed as "
                f"deliberately untaught ({UNDOCUMENTED[name]})"
            )

    # ── the module doc's list ────────────────────────────────────────────────
    module_path = root / MODULE_DOC_FILE
    if not module_path.is_file():
        failures.append(f"{MODULE_DOC_FILE}: missing")
    else:
        listed = set()
        for line in module_path.read_text(encoding="utf-8").splitlines():
            match = MODULE_DOC_RE.match(line.strip())
            if match:
                listed.add(match.group("name"))
        expected_names = {name for name, _, _ in expected}
        for name in sorted(expected_names - listed):
            failures.append(
                f"{MODULE_DOC_FILE}: the module doc does not list `{name}`, which "
                f"the extension registers"
            )
        for name in sorted(listed - all_names(catalog)):
            failures.append(
                f"{MODULE_DOC_FILE}: the module doc lists `{name}`, which the "
                f"extension does not register"
            )

    # ── prose counts ─────────────────────────────────────────────────────────
    for rel, pattern, expects in COUNT_CLAIMS:
        path = root / rel
        if not path.is_file():
            failures.append(f"{rel}: registered count claim file is missing")
            continue
        text = path.read_text(encoding="utf-8")
        matches = list(re.finditer(pattern, text))
        if not matches:
            failures.append(
                f"{rel}: registered count claim no longer matches — /{pattern}/\n"
                f"    The prose moved. Re-register it in {Path(__file__).name}."
            )
            continue
        for match in matches:
            lineno = text.count("\n", 0, match.start()) + 1
            for group, bucket in expects.items():
                stated = int(match.group(group))
                derived = len(catalog[bucket])
                if stated != derived:
                    failures.append(
                        f"{rel}:{lineno}: documented {stated} {bucket}, the loaded "
                        f"extension registers {derived}"
                    )

    # ── documented STRUCT field sets ─────────────────────────────────────────
    for rel, pattern, name in STRUCT_CLAIMS:
        path = root / rel
        if not path.is_file():
            failures.append(f"{rel}: registered struct claim file is missing")
            continue
        text = path.read_text(encoding="utf-8")
        matches = list(re.finditer(pattern, text))
        if not matches:
            failures.append(
                f"{rel}: registered struct claim no longer matches — /{pattern}/\n"
                f"    The prose moved. Re-register it in {Path(__file__).name}."
            )
            continue
        derived = catalog["scalars"].get(name, {}).get("struct_fields")
        if derived is None:
            failures.append(
                f"{rel}: `{name}` is documented as returning a STRUCT, the loaded "
                f"extension returns {_return_of(catalog, name)}"
            )
            continue
        for match in matches:
            lineno = text.count("\n", 0, match.start()) + 1
            keys = STRUCT_KEY_RE.findall(match.group("fields"))
            if keys != derived:
                failures.append(
                    f"{rel}:{lineno}: `{name}` is documented with STRUCT fields "
                    f"{keys}, the loaded extension returns {derived}"
                )

    # ── fail-closed: every documented call shape must exist ──────────────────
    known = all_names(catalog)
    for path in scanned_files(root):
        rel = path.relative_to(root).as_posix()
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in CALL_SHAPE_RE.finditer(line):
                name = match.group("name")
                if name in known or name in NOT_FUNCTIONS:
                    continue
                failures.append(
                    f"{rel}:{lineno}: `{name}(` is called here, but the loaded "
                    f"extension registers no such function\n"
                    f"    {line.strip()[:140]}\n"
                    f"    If it is not a function of this extension, add it to "
                    f"NOT_FUNCTIONS in {Path(__file__).name} with the reason."
                )

    return failures


def _return_of(catalog: dict, name: str) -> str:
    if name in catalog["scalars"]:
        return catalog["scalars"][name]["return_type"]
    return "TABLE"


def _normalise(text: str) -> str:
    return re.sub(r"\s+", " ", text.replace("`", "").strip())


def _read_surface_table(path: Path) -> list[tuple[str, str, str, int]]:
    """(name, kind, documented return type, line number) for every row."""
    rows: list[tuple[str, str, str, int]] = []
    lines = path.read_text(encoding="utf-8").splitlines()
    for index, line in enumerate(lines):
        if not SURFACE_HEADER_RE.match(line):
            continue
        for offset, row in enumerate(lines[index + 1 :], index + 2):
            if not row.startswith("|"):
                break
            match = SURFACE_ROW_RE.match(row)
            if match:
                rows.append(
                    (
                        match.group("name"),
                        match.group("kind").strip().lower(),
                        match.group("returns"),
                        offset,
                    )
                )
    return rows


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def _sub(path_rel: str, old: str, new: str):
    def apply(root: Path):
        path = root / path_rel
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise Fatal(f"self-test: mutation target not found in {path_rel}: {old!r}")
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    return apply


def _drop_line(path_rel: str, needle: str):
    def apply(root: Path):
        path = root / path_rel
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        kept = [line for line in lines if needle not in line]
        if len(kept) == len(lines):
            raise Fatal(f"self-test: no line containing {needle!r} in {path_rel}")
        path.write_text("".join(kept), encoding="utf-8")

    return apply


def _append(path_rel: str, extra: str):
    def apply(root: Path):
        path = root / path_rel
        path.write_text(path.read_text(encoding="utf-8") + extra, encoding="utf-8")

    return apply


def self_test(root: Path, catalog: dict) -> int:
    """Mutate a scratch copy of the docs and require each mutation to be caught."""
    scalars = len(catalog["scalars"])
    aggregates = len(catalog["aggregates"])
    macros = len(catalog["table_macros"])
    profile_returns = catalog["aggregates"]["ft_profile"]["return_type"]

    cases: list[tuple[str, Callable[[Path], None], str]] = [
        (
            "a function loses its row from the registry table",
            _drop_line("docs/ARCHITECTURE.md", "| `ft_cast(value)`"),
            "no row for `ft_cast`",
        ),
        (
            "the registry table names a function that does not exist",
            _sub("docs/ARCHITECTURE.md", "| `ft_cast(value)`", "| `ft_casts(value)`"),
            "which the loaded extension does not register",
        ),
        (
            "a documented return type drifts (the STRUCT-vs-VARCHAR defect)",
            _sub("docs/ARCHITECTURE.md", f"| {profile_returns} |", "| VARCHAR |"),
            "documented as returning VARCHAR",
        ),
        (
            "a documented STRUCT gains a field the extension does not return",
            _sub(
                "docs/ARCHITECTURE.md",
                'STRUCT("type" VARCHAR, confidence DOUBLE, duckdb_type VARCHAR)',
                'STRUCT("type" VARCHAR, confidence DOUBLE, duckdb_type VARCHAR, locale VARCHAR)',
            ),
            "documented as returning",
        ),
        (
            "the scalar count drifts",
            _sub(
                "docs/ARCHITECTURE.md",
                f"DuckDB extension: {scalars} scalar functions + {aggregates} aggregate"
                f" + {macros} table macros",
                f"DuckDB extension: 5 scalar functions + {aggregates} aggregate"
                f" + {macros} table macros",
            ),
            f"documented 5 scalars, the loaded extension registers {scalars}",
        ),
        (
            "the aggregate count drifts",
            _sub(
                "docs/ARCHITECTURE.md",
                f"DuckDB extension: {scalars} scalar functions + {aggregates} aggregate"
                f" + {macros} table macros",
                f"DuckDB extension: {scalars} scalar functions + 4 aggregate"
                f" + {macros} table macros",
            ),
            f"documented 4 aggregates, the loaded extension registers {aggregates}",
        ),
        (
            "the table-macro count drifts",
            _sub(
                "README.md",
                f"{scalars} scalar functions, {aggregates} aggregate and {macros} table macros",
                f"{scalars} scalar functions, {aggregates} aggregate and 9 table macros",
            ),
            f"documented 9 table_macros, the loaded extension registers {macros}",
        ),
        (
            "the aggregate is documented as a scalar",
            _sub(
                "docs/ARCHITECTURE.md",
                "| `ft_profile(col, header?)` | aggregate |",
                "| `ft_profile(col, header?)` | scalar |",
            ),
            "no row for `ft_profile` as a aggregate",
        ),
        (
            "the module doc stops listing a function",
            _drop_line(MODULE_DOC_FILE, "//! - `ft_cast("),
            "does not list `ft_cast`",
        ),
        (
            "the module doc lists a function that does not exist",
            _sub(MODULE_DOC_FILE, "//! - `ft_cast(", "//! - `ft_casts("),
            "lists `ft_casts`",
        ),
        (
            "a documented STRUCT literal drifts from the catalog's field names",
            _sub(
                "docs/DEVELOPMENT.md",
                "-- {'valid': false, 'constraint': pattern,",
                "-- {'valid': false, 'rule': pattern,",
            ),
            "documented with STRUCT fields",
        ),
        (
            "the docs call a function that was never registered",
            _append("docs/ARCHITECTURE.md", "\nUse `ft_classify(col)` for this.\n"),
            "`ft_classify(` is called here",
        ),
        (
            "the spike artefact is taught as production surface",
            _sub(
                "docs/ARCHITECTURE.md",
                "| `ft_cast(value)`",
                "| `finetype_spike(n)` | TABLE | x |\n| `ft_cast(value)`",
            ),
            "listed as deliberately untaught",
        ),
    ]

    print(
        f"self-test: catalog v{catalog['version']} — {scalars} scalars, "
        f"{aggregates} aggregates, {len(catalog['table_functions'])} table "
        f"functions, {macros} table macros"
    )

    failed = 0
    with tempfile.TemporaryDirectory(prefix="duckdb-catalog-selftest-") as tmp:
        pristine = Path(tmp) / "pristine"
        _stage(root, pristine)

        control = check(pristine, catalog)
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
            shutil.copytree(pristine, work)
            try:
                mutate(work)
                found = check(work, catalog)
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


def _stage(root: Path, dest: Path) -> None:
    for src in scanned_files(root) + [root / SURFACE_FILE, root / MODULE_DOC_FILE]:
        rel = src.relative_to(root)
        target = dest / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            shutil.copyfile(src, target)


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
    parser.add_argument("--catalog", action="store_true", help="print the derived surface")
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    extension = (args.extension or (root / DEFAULT_EXTENSION)).resolve()

    try:
        catalog = derive(root, extension)
    except Fatal as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if args.catalog:
        print(f"extension version {catalog['version']}  ({extension})")
        for name in sorted(catalog["scalars"]):
            entry = catalog["scalars"][name]
            print(f"  scalar       {name} -> {entry['return_type']}")
            for overload in entry["overloads"]:
                print(f"                 ({', '.join(overload) or ''})")
        for name in sorted(catalog["aggregates"]):
            entry = catalog["aggregates"][name]
            print(f"  aggregate    {name} -> {entry['return_type']}")
            for overload in entry["overloads"]:
                print(f"                 ({', '.join(overload) or ''})")
        for name in sorted(catalog["table_functions"]):
            print(f"  table fn     {name}")
        for name in sorted(catalog["table_macros"]):
            print(f"  table macro  {name}({', '.join(catalog['table_macros'][name])})")
        return 0

    if args.self_test:
        try:
            return self_test(root, catalog)
        except Fatal as exc:
            print(f"FATAL: {exc}", file=sys.stderr)
            return 2

    failures = check(root, catalog)
    if failures:
        print(
            f"The documented DuckDB surface disagrees with the loaded extension "
            f"(v{catalog['version']}: {len(catalog['scalars'])} scalars, "
            f"{len(catalog['aggregates'])} aggregates, "
            f"{len(catalog['table_functions'])} table functions, "
            f"{len(catalog['table_macros'])} table macros):\n",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(
            f"\n{len(failures)} problem(s). Derived surface: "
            f"scripts/{Path(__file__).name} --catalog",
            file=sys.stderr,
        )
        return 1

    print(
        f"The documented DuckDB surface matches the loaded extension: v"
        f"{catalog['version']}, {len(catalog['scalars'])} scalar functions, "
        f"{len(catalog['aggregates'])} aggregate, "
        f"{len(catalog['table_macros'])} table macros, "
        f"{len(catalog['table_functions'])} table function."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
