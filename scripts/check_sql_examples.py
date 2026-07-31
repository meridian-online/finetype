#!/usr/bin/env python3
"""Run every ```sql example in the docs against a LOCAL build, and check its SHAPE.

A SQL example in a README is a promise that the query runs. Nothing in CI ran
one, so the promises drifted: a fence loads
`target/release/finetype_duckdb.duckdb_extension`, an artifact no build produces;
another calls `parse_fr_month(...)`, a function that does not exist in DuckDB or
in this extension; a third puts a shell pipeline inside a ```sql fence. Each of
those is a copy-paste that fails on first contact with a terminal.

WHAT IS RUN
    Every ```sql fence in README.md and docs/*.md (dated snapshots excluded).
    Each fence gets: a hermetic DuckDB, a LOAD of the LOCAL artifact, and its
    document's fixture — the tables the examples name — prepended. Then every
    statement in the fence runs. A DuckDB error is a failure.

WHAT IS ASSERTED — SHAPE ONLY
    Whatever the fence's own trailing comment shows about the RESULT:

        an ASCII table  -> the result's column NAMES, in order, must equal the
                           header cells of that box
        `-- {'a': …}`   -> the statement must return a STRUCT whose FIELD NAMES,
                           in order, are the literal's keys
        `-- → '{json}'` -> the returned JSON's KEY SET must equal the literal's
        `-- → 'text'`   -> the statement must return exactly one VARCHAR column

    Plus a shape contract per table macro (MACRO_SHAPES), checked with DESCRIBE:
    column names AND column types.

WHAT IS NEVER ASSERTED — and why the gate stays green when the model moves
    No confidence digit, no predicted label, no score, no row ordering, no row
    count. Those are model output: they move with a retrain, with a locale, with
    the sample DuckDB happens to take. Pinning one would make this gate flake and
    then be ignored, which is worse than not having it. The parser is built so it
    CANNOT assert them — it reads keys out of a literal and throws the values
    away — and the self-test proves it, by moving a confidence digit and a
    predicted label and requiring the gate to stay green.

    A documented LOAD path IS asserted (LOCAL_LOAD): it must name the artifact
    the build produces, or the bare community extension name. That is a fact
    about this repo, not about the model.

USAGE
    scripts/check_sql_examples.py                  # gate the working tree
    scripts/check_sql_examples.py --list           # list the fences and fixtures
    scripts/check_sql_examples.py --self-test      # prove the gate detects
    scripts/check_sql_examples.py --extension PATH

Needs the duckdb CLI, a built extension (`make build-extension`) and a model
(`FINETYPE_MODEL_DIR`, or the extension downloads one on first classify).

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
from pathlib import Path

DEFAULT_EXTENSION = "target/release/finetype.duckdb_extension"

# ══════════════════════════════════════════════════════════════════════════════
# SCOPE — which fences run, and the data they run against
# ══════════════════════════════════════════════════════════════════════════════

# Dated reports: snapshots of a past run, not instructions to a reader.
SNAPSHOT_FILES = {
    "docs/release-notes-0.6.54.md",
    "docs/branch-ablation-m2v8m-s43.md",
    "docs/compact-dmy-gate-coverage.md",
    "docs/compact-dmy-mutation-matrix.md",
    "docs/mechanism-attribution.md",
}

# One fixture per document: the tables its examples name, with enough rows for a
# column-level classifier to have something to work with. Values are chosen to be
# obviously typed (an email is an email) — but nothing here asserts what the
# model calls them.
FIXTURES: dict[str, str] = {
    "README.md": """
CREATE TABLE my_table (value VARCHAR, json_col VARCHAR);
INSERT INTO my_table VALUES
  ('01/15/2024', '{"host":"192.168.1.1","seen":"2024-01-15"}'),
  ('02/20/2024', '{"host":"10.0.0.8","seen":"2024-02-20"}'),
  ('03/25/2024', '{"host":"172.16.0.3","seen":"2024-03-25"}'),
  ('04/30/2024', '{"host":"192.168.1.9","seen":"2024-04-30"}');
""",
    "docs/DEVELOPMENT.md": """
CREATE TABLE people (age INTEGER, email VARCHAR, phone VARCHAR);
INSERT INTO people VALUES
  (34, 'jane.doe@company.co.uk', '+44 20 7946 0958'),
  (41, 'sam.patel@example.com',  '+1 202 555 0100'),
  (29, 'not-an-email',           'not-a-phone');
""",
    "docs/LOCALE_GUIDE.md": """
CREATE TABLE customers (customer_id INTEGER, phone_number VARCHAR);
INSERT INTO customers VALUES
  (1, '+1 202 555 0100'), (2, '+49 30 12345678'),
  (3, '+33 1 42 68 53 00'), (4, '+44 20 7946 0958');
CREATE TABLE french_dates (month_col VARCHAR);
INSERT INTO french_dates VALUES
  ('janvier'), ('février'), ('mars'), ('avril'), ('mai'), ('juin');
CREATE TABLE orders (region VARCHAR, postal_code VARCHAR);
INSERT INTO orders VALUES
  ('EN_US', '90210'), ('EN_US', '10001'),
  ('EN_CA', 'M5V 3A8'), ('EN_GB', 'SW1A 1AA');
""",
}

# Files a document's examples open by path. Written into the run directory.
FILE_ASSETS: dict[str, dict[str, str]] = {
    "docs/DEVELOPMENT.md": {
        "schema.json": json.dumps(
            {
                "properties": {
                    "email": {
                        "type": "string",
                        "pattern": "^[^@]+@[^@]+\\.[^@]+$",
                    },
                    "phone": {"type": "string", "pattern": "^[+0-9 ()-]+$"},
                }
            },
            indent=2,
        )
    },
}

# The output columns of each table macro, as the docs describe them. Checked with
# DESCRIBE: names AND types. This is the one place types are pinned, and they are
# the extension's own contract, not a model prediction.
MACRO_SHAPES: dict[str, tuple[str, list[tuple[str, str]]]] = {
    "ft_profile('people')": (
        "docs/DEVELOPMENT.md",
        [
            ("column_name", "VARCHAR"),
            ("type", "VARCHAR"),
            ("confidence", "DOUBLE"),
            ("duckdb_type", "VARCHAR"),
        ],
    ),
    "ft_validate('people', 'schema.json')": (
        "docs/DEVELOPMENT.md",
        [
            ("column_name", "VARCHAR"),
            ("total", "BIGINT"),
            ("rejects", "BIGINT"),
            ("sample_message", "VARCHAR"),
        ],
    ),
}


class Fatal(Exception):
    """The extension or duckdb could not be used — nothing downstream is meaningful."""


# ══════════════════════════════════════════════════════════════════════════════
# EXTRACTION
# ══════════════════════════════════════════════════════════════════════════════

FENCE_OPEN_RE = re.compile(r"^```sql\s*$")
FENCE_CLOSE_RE = re.compile(r"^```\s*$")
LOAD_RE = re.compile(r"^\s*LOAD\s+'(?P<path>[^']+)'\s*;?\s*$", re.IGNORECASE)

# A shell pipeline hiding in a ```sql fence — it is not SQL and never was.
SHELL_RE = re.compile(r"^\s*(finetype|duckdb|jq|cat|echo|curl)\s+[^(]*$")


class Fence:
    def __init__(self, rel: str, start: int, lines: list[str]):
        self.rel = rel
        self.start = start  # 1-based line of the ```sql marker
        self.lines = lines

    @property
    def where(self) -> str:
        return f"{self.rel}:{self.start}"


def find_fences(root: Path) -> list[Fence]:
    fences: list[Fence] = []
    docs = [root / "README.md"] + sorted((root / "docs").glob("*.md"))
    for path in docs:
        if not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        if rel in SNAPSHOT_FILES:
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        index = 0
        while index < len(lines):
            if FENCE_OPEN_RE.match(lines[index]):
                body: list[str] = []
                cursor = index + 1
                while cursor < len(lines) and not FENCE_CLOSE_RE.match(lines[cursor]):
                    body.append(lines[cursor])
                    cursor += 1
                fences.append(Fence(rel, index + 1, body))
                index = cursor
            index += 1
    return fences


class Statement:
    def __init__(self, sql: str, line: int, comments: list[str]):
        self.sql = sql
        self.line = line
        self.comments = comments


def _scan(line: str, in_string: bool) -> tuple[str, str, bool, bool]:
    """Split one line into (code, trailing comment, ends-a-statement, in string).

    Quote-aware, so a `--` or `;` inside a string literal is not a terminator.
    Two statements on one line is not a shape these docs use and is not handled:
    the remainder of a line after `;` stays with that statement.
    """
    out: list[str] = []
    ends = False
    index = 0
    while index < len(line):
        char = line[index]
        if in_string:
            out.append(char)
            if char == "'":
                in_string = False
            index += 1
            continue
        if char == "'":
            in_string = True
            out.append(char)
            index += 1
            continue
        if line.startswith("--", index):
            return "".join(out), line[index:].strip(), ends, in_string
        out.append(char)
        if char == ";":
            ends = True
        index += 1
    return "".join(out), "", ends, in_string


def split_statements(fence: Fence) -> list[Statement]:
    """Statements plus the comment block that FOLLOWS each one.

    A comment block before a statement describes it; a comment block after it
    shows its result. Only the latter is an expectation, so collection stops at
    the blank line that separates the two — otherwise the box drawn under one
    statement gets attributed to the statement before it.
    """
    statements: list[Statement] = []
    buffer: list[str] = []
    buffer_line = 0
    in_string = False
    collecting = False

    def flush():
        nonlocal buffer, collecting
        statements.append(Statement("\n".join(buffer).strip(), buffer_line, []))
        buffer = []
        collecting = True

    for offset, raw in enumerate(fence.lines):
        line_no = fence.start + 1 + offset
        stripped = raw.strip()
        if not buffer:
            if not stripped:
                collecting = False
                continue
            if stripped.startswith("--"):
                if collecting and statements:
                    statements[-1].comments.append(stripped)
                continue
            buffer_line = line_no
            collecting = False
        code, comment, ends, in_string = _scan(raw, in_string)
        buffer.append(code.rstrip())
        if ends and not in_string:
            flush()
            if comment:
                statements[-1].comments.append(comment)
        elif comment and not buffer[-1].strip():
            # a comment line in the middle of a multi-line statement
            buffer.pop()
    if buffer and "\n".join(buffer).strip():
        flush()
    return statements


# ══════════════════════════════════════════════════════════════════════════════
# EXPECTATIONS — read out of the fence's own comments, keys only
# ══════════════════════════════════════════════════════════════════════════════

BOX_ROW_RE = re.compile(r"^--\s*│(?P<cells>.*)│\s*$")
BOX_TOP_RE = re.compile(r"^--\s*[┌├└]")
STRUCT_LITERAL_RE = re.compile(r"^--\s*\{(?P<body>'.*)\}\s*$")
ARROW_RE = re.compile(r"^--\s*(?:→|->)\s*(?P<body>.+?)\s*$")
STRUCT_KEY_RE = re.compile(r"'(?P<key>[A-Za-z_][A-Za-z0-9_]*)'\s*:")


def read_expectation(comments: list[str]) -> tuple[str, object] | None:
    """The shape a comment block claims. Values are read and discarded."""
    for index, comment in enumerate(comments):
        if BOX_TOP_RE.match(comment):
            for row in comments[index:]:
                match = BOX_ROW_RE.match(row)
                if match:
                    cells = [cell.strip() for cell in match.group("cells").split("│")]
                    return ("columns", [cell for cell in cells if cell])
            return None
    for comment in comments:
        match = STRUCT_LITERAL_RE.match(comment)
        if match:
            return ("struct_fields", STRUCT_KEY_RE.findall(match.group("body")))
        match = ARROW_RE.match(comment)
        if match:
            body = match.group("body").strip().strip("'")
            if body.startswith("{") or body.startswith("["):
                # Fail closed: a documented result that LOOKS like JSON but does
                # not parse used to be skipped silently, which quietly took a
                # README example out of this gate.
                try:
                    return ("json_keys", sorted(json.loads(body)))
                except json.JSONDecodeError:
                    return ("unreadable", body)
            return ("one_varchar", body)
    return None


# ══════════════════════════════════════════════════════════════════════════════
# RUNNING
# ══════════════════════════════════════════════════════════════════════════════

MARKER = "__ft_doc_marker__"


def prepare_run_dir(run_dir: Path, root: Path, rel: str) -> None:
    """A directory the examples can run in: their file assets, and the model.

    The `models` symlink is load-bearing, not tidiness. The Model2Vec branch
    resolves `model_dir/model2vec/` and then the CWD-RELATIVE `models/model2vec/`
    (crates/finetype-model/src/multi_branch/mod.rs `load_model2vec`), and the
    downloaded layout puts model2vec beside the model rather than inside it. A
    run from anywhere other than the repo root panics without this.
    """
    for name, body in FILE_ASSETS.get(rel, {}).items():
        (run_dir / name).write_text(body, encoding="utf-8")
    models = root / "models"
    if models.is_dir():
        (run_dir / "models").symlink_to(models, target_is_directory=True)


def model_env(root: Path) -> dict[str, str]:
    """Point at the local model unless the caller already chose one."""
    if os.environ.get("FINETYPE_MODEL_DIR"):
        return {}
    default = root / "models" / "default"
    if (default / "model.safetensors").exists():
        return {"FINETYPE_MODEL_DIR": str(default.resolve())}
    return {}  # the extension will fetch from HuggingFace


def model_preflight(root: Path) -> str | None:
    """Why the model would fail to load, in words rather than as a Rust panic.

    Both halves have to be there. The multi-branch loader takes the model
    directory from FINETYPE_MODEL_DIR (or downloads one), then resolves the
    Model2Vec branch at `model_dir/model2vec/` and falls back to the CWD-relative
    `models/model2vec/` (crates/finetype-model/src/multi_branch/mod.rs
    `load_model2vec`). Missing the second half aborts the process, and eight
    fences then report the same backtrace.
    """
    chosen = os.environ.get("FINETYPE_MODEL_DIR") or model_env(root).get(
        "FINETYPE_MODEL_DIR"
    )
    if not chosen:
        return None  # nothing local — the extension will fetch from HuggingFace
    model_dir = Path(chosen)
    if not (model_dir / "model.safetensors").is_file():
        return (
            f"FINETYPE_MODEL_DIR={model_dir} has no model.safetensors.\n"
            f"       Fetch one: .github/scripts/download-model.sh"
        )
    local_m2v = model_dir / "model2vec" / "model.safetensors"
    shared_m2v = root / "models" / "model2vec" / "model.safetensors"
    if not local_m2v.is_file() and not shared_m2v.is_file():
        return (
            f"the Model2Vec branch is missing. Checked {local_m2v} and "
            f"{shared_m2v}.\n"
            f"       Fetch it: .github/scripts/download-model.sh"
        )
    return None


def _duckdb(
    script: str, cwd: Path, extension: Path, tmp: Path, extra_env: dict | None = None
) -> tuple[int, str, str]:
    prelude = f"SET extension_directory='{tmp}';\nLOAD '{extension}';\n"
    proc = subprocess.run(
        ["duckdb", "-unsigned", "-no-init", "-json", "-c", prelude + script],
        capture_output=True,
        text=True,
        cwd=cwd,
        env={
            **os.environ,
            "DUCKDB_EXTENSION_DIRECTORY": str(tmp),
            **(extra_env or {}),
        },
    )
    return proc.returncode, proc.stdout, proc.stderr


def _arrays(stdout: str) -> list[list]:
    """Successive JSON arrays, one per statement that returned a result set."""
    decoder = json.JSONDecoder()
    out: list[list] = []
    index = 0
    text = stdout.strip()
    while index < len(text):
        while index < len(text) and text[index] in " \n\r\t":
            index += 1
        if index >= len(text):
            break
        value, index = decoder.raw_decode(text, index)
        out.append(value)
    return out


def run_fence(
    fence: Fence, statements: list[Statement], root: Path, extension: Path
) -> tuple[list[str], dict[int, list]]:
    """Run one fence. Returns (failures, {statement index: its result rows})."""
    failures: list[str] = []
    with tempfile.TemporaryDirectory(prefix="ft-sql-examples-") as tmp:
        tmp_path = Path(tmp)
        run_dir = tmp_path / "run"
        run_dir.mkdir()
        ext_dir = tmp_path / "ext"
        ext_dir.mkdir()
        prepare_run_dir(run_dir, root, fence.rel)

        pieces = [FIXTURES.get(fence.rel, "")]
        emitted: list[int] = []
        for index, statement in enumerate(statements):
            if LOAD_RE.match(statement.sql):
                continue  # the local LOAD is injected by _duckdb
            pieces.append(statement.sql)
            pieces.append(f"SELECT '{MARKER}' AS m;")
            emitted.append(index)

        code, stdout, stderr = _duckdb(
            "\n".join(pieces), run_dir, extension, ext_dir, model_env(root)
        )
        if code != 0:
            failed = _blame(stderr, statements, emitted, stdout)
            failures.append(
                f"{fence.where}: the example does not run — DuckDB exited {code}\n"
                f"    {failed}\n"
                f"    {stderr.strip().splitlines()[0] if stderr.strip() else ''}"
            )
            return failures, {}

        arrays = _arrays(stdout)
        results: dict[int, list] = {}
        pending: list[list] = []
        cursor = 0
        for array in arrays:
            if array and isinstance(array[0], dict) and array[0].get("m") == MARKER:
                if cursor < len(emitted):
                    results[emitted[cursor]] = pending[-1] if pending else []
                cursor += 1
                pending = []
            else:
                pending.append(array)
        if cursor != len(emitted):
            failures.append(
                f"{fence.where}: {cursor} of {len(emitted)} statements reported a "
                f"result — this gate cannot line results up with statements here"
            )
            return failures, {}
    return failures, results


def _blame(stderr: str, statements: list[Statement], emitted: list[int], stdout: str) -> str:
    """Name the statement that failed, by counting the markers that got through."""
    seen = sum(
        1
        for array in _arrays(stdout)
        if array and isinstance(array[0], dict) and array[0].get("m") == MARKER
    )
    if seen < len(emitted):
        statement = statements[emitted[seen]]
        first = statement.sql.strip().splitlines()[0]
        return f"line {statement.line}: {first[:120]}"
    return "in the fixture or the trailing statements"


# ══════════════════════════════════════════════════════════════════════════════
# THE GATE
# ══════════════════════════════════════════════════════════════════════════════


def check(root: Path, extension: Path, macro_shapes: dict | None = None) -> list[str]:
    failures: list[str] = []
    fences = find_fences(root)
    if not fences:
        return ["no ```sql fences found at all — this gate has stopped reading the docs"]

    artifact = Path(DEFAULT_EXTENSION)
    for fence in fences:
        statements = split_statements(fence)
        if not statements:
            failures.append(f"{fence.where}: an empty ```sql fence")
            continue

        # ── a shell pipeline is not SQL ──────────────────────────────────────
        for statement in statements:
            first = statement.sql.strip().splitlines()[0]
            if SHELL_RE.match(first) and not first.rstrip().endswith(";"):
                failures.append(
                    f"{fence.rel}:{statement.line}: a shell command inside a ```sql "
                    f"fence — move it to a ```bash fence\n    {first[:120]}"
                )

        # ── the documented LOAD must name an artifact the build produces ─────
        for statement in statements:
            match = LOAD_RE.match(statement.sql)
            if not match:
                continue
            documented = match.group("path")
            if documented == "finetype":
                continue  # the community-install form
            normalised = Path(documented)
            if normalised.parts and normalised.parts[0] == ".":
                normalised = Path(*normalised.parts[1:])
            if normalised != artifact:
                failures.append(
                    f"{fence.rel}:{statement.line}: `LOAD '{documented}'` names an "
                    f"artifact no build produces. `make build-extension` writes "
                    f"`{artifact}`; `LOAD 'finetype'` is the community form."
                )

        # A wrong LOAD path does not stop the fence being run — the injected
        # LOAD is the local build either way, and a second independent problem
        # in the same fence is worth reporting in the same pass.
        run_failures, results = run_fence(fence, statements, root, extension)
        failures.extend(run_failures)
        if run_failures:
            continue

        # ── shape, from the fence's own trailing comments ────────────────────
        for index, statement in enumerate(statements):
            expectation = read_expectation(statement.comments)
            if expectation is None:
                continue
            kind, want = expectation
            where = f"{fence.rel}:{statement.line}"
            described = _describe(statement.sql, fence, root, extension)
            if described is None:
                failures.append(f"{where}: DESCRIBE of this statement failed")
                continue
            names = [row["column_name"] for row in described]
            types = [row["column_type"] for row in described]

            if kind == "columns":
                if names != want:
                    failures.append(
                        f"{where}: the documented result box shows columns {want}, "
                        f"the query returns {names}"
                    )
            elif kind == "struct_fields":
                if len(types) != 1 or not types[0].startswith("STRUCT("):
                    failures.append(
                        f"{where}: documented as returning a STRUCT {want}, the "
                        f"query returns {types}"
                    )
                else:
                    fields = _struct_fields(types[0])
                    if fields != want:
                        failures.append(
                            f"{where}: documented STRUCT fields {want}, the query "
                            f"returns {fields}"
                        )
            elif kind == "json_keys":
                rows = results.get(index) or []
                if not rows:
                    failures.append(
                        f"{where}: documented as returning JSON, the query returned "
                        f"no rows against this document's fixture"
                    )
                elif len(names) != 1:
                    failures.append(
                        f"{where}: documented as returning one JSON column, the "
                        f"query returns {names}"
                    )
                else:
                    value = rows[0][names[0]]
                    try:
                        got = sorted(json.loads(value))
                    except (json.JSONDecodeError, TypeError):
                        failures.append(
                            f"{where}: documented as returning JSON, the query "
                            f"returned {str(value)[:80]!r}"
                        )
                        continue
                    # SUBSET, not equality. Some keys are conditional —
                    # `disambiguation` appears only when a rule fired
                    # (column_fn.rs `format_column_result_json`) — so requiring
                    # equality would make this gate depend on model behaviour,
                    # which is exactly what it must not do. Naming a key that is
                    # never returned is the defect; showing fewer keys than a
                    # full result is an abridged example.
                    invented = [key for key in want if key not in got]
                    if invented:
                        failures.append(
                            f"{where}: the documented JSON names {invented}, which "
                            f"the query never returns (it returns {got})"
                        )
            elif kind == "unreadable":
                failures.append(
                    f"{where}: the documented result looks like JSON but does not "
                    f"parse, so nothing checks it — write it as valid JSON or drop "
                    f"the `→` marker\n    {str(want)[:120]}"
                )
            elif kind == "one_varchar":
                if len(types) != 1 or types[0] != "VARCHAR":
                    failures.append(
                        f"{where}: documented as returning a single text value, "
                        f"the query returns {types}"
                    )

        # ── the table macros' own column contract ────────────────────────────
        for call, (owner, want) in (macro_shapes or MACRO_SHAPES).items():
            if owner != fence.rel:
                continue
            if not any(call in statement.sql for statement in statements):
                continue
            described = _describe(f"SELECT * FROM {call};", fence, root, extension)
            if described is None:
                failures.append(f"{fence.where}: DESCRIBE of `{call}` failed")
                continue
            got = [(row["column_name"], row["column_type"]) for row in described]
            if got != want:
                failures.append(
                    f"{fence.where}: `{call}` is documented as yielding {want}, "
                    f"DESCRIBE says {got}"
                )

    return failures


def _describe(sql: str, fence: Fence, root: Path, extension: Path) -> list | None:
    """DESCRIBE one statement, with its document's fixture in place."""
    body = sql.strip().rstrip(";")
    if not body.lower().startswith(("select", "with", "from")):
        return None
    with tempfile.TemporaryDirectory(prefix="ft-sql-describe-") as tmp:
        tmp_path = Path(tmp)
        run_dir = tmp_path / "run"
        run_dir.mkdir()
        ext_dir = tmp_path / "ext"
        ext_dir.mkdir()
        prepare_run_dir(run_dir, root, fence.rel)
        script = FIXTURES.get(fence.rel, "") + f"\nDESCRIBE {body};"
        code, stdout, _ = _duckdb(script, run_dir, extension, ext_dir, model_env(root))
        if code != 0:
            return None
        arrays = _arrays(stdout)
        return arrays[-1] if arrays else None


def _struct_fields(struct_type: str) -> list[str]:
    inner = struct_type[len("STRUCT(") : struct_type.rindex(")")]
    fields, depth, current = [], 0, ""
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
        token = field.strip().split()[0]
        names.append(token.strip('"'))
    return names


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST
# ══════════════════════════════════════════════════════════════════════════════


def _sub(path_rel: str, old: str, new: str):
    """Mutate a documented snippet, tolerating how it happens to be spaced.

    These targets are fragments of prose, and matching them LITERALLY couples
    the self-test to the document's exact formatting. Two mutations matched on
    exact spacing and lost their target when the surrounding prose was rewritten.
    The gate stayed green; its proof that it can fail went red.

    The causal explanation that stood here was DELETED rather than rewritten, on
    the second failed attempt to state it. It blamed a specific commit for
    inserting the spaces; independent checks placed the loss two commits earlier
    and showed that commit expanding the JSON rather than collapsing it. Three
    accounts of this one sentence have now been wrong. The count is measured and
    kept; the causality is not reliably known from this tree, so it is not
    asserted. What matters operationally is the rule below, which does not
    depend on knowing which commit did it.

    So whitespace is optional wherever JSON punctuation allows it. The mutation
    is a claim about the document's MEANING; the spacing is not part of it.
    """

    pattern = re.compile(
        re.escape(old).replace(":", r":\s*").replace(",", r",\s*")
    )

    def apply(root: Path):
        path = root / path_rel
        text = path.read_text(encoding="utf-8")
        if not pattern.search(text):
            raise Fatal(f"self-test: mutation target not found in {path_rel}: {old!r}")
        path.write_text(pattern.sub(lambda _: new, text, count=1), encoding="utf-8")

    return apply


def _nothing(root: Path) -> None:
    """Change no document — for a case that varies the expectation instead."""


def self_test(root: Path, extension: Path) -> int:
    # (name, doc mutation, expected substring — None means "must stay green",
    #  MACRO_SHAPES override)
    wrong_macro_shape = {
        "ft_profile('people')": (
            "docs/DEVELOPMENT.md",
            [
                ("column_name", "VARCHAR"),
                ("type", "VARCHAR"),
                ("confidence", "VARCHAR"),  # it is a DOUBLE
                ("duckdb_type", "VARCHAR"),
            ],
        )
    }

    cases: list[tuple[str, object, str | None, dict | None]] = [
        (
            "a documented query stops running",
            # Breaks the TABLE, not one named example. Naming a specific
            # query couples this to which functions the README happens to
            # teach: it named a `finetype_cast` example, the docs moved to
            # the `ft_` surface, and the mutation lost its target while the
            # gate stayed green. Any documented query over the fixture will
            # do, and there is always at least one.
            _sub("README.md", "FROM my_table;", "FROM no_such_table;"),
            "the example does not run",
            None,
        ),
        (
            "a documented function does not exist",
            _sub("docs/DEVELOPMENT.md", "SELECT ft_infer(", "SELECT ft_inferr("),
            "the example does not run",
            None,
        ),
        (
            "a documented result box renames a column",
            _sub("docs/DEVELOPMENT.md", "│ column_name │ type ", "│ col_name    │ type "),
            "the documented result box shows columns",
            None,
        ),
        (
            "a documented STRUCT renames a field",
            _sub("docs/DEVELOPMENT.md", "-- {'valid': false, 'constraint': pattern,",
                 "-- {'valid': false, 'rule': pattern,"),
            "documented STRUCT fields",
            None,
        ),
        (
            "a documented JSON result names a key the query never returns",
            _sub("README.md", '"duckdb_type":"DATE"', '"broad_type":"DATE"'),
            "which the query never returns",
            None,
        ),
        (
            "the documented LOAD names an artifact no build produces",
            _sub("README.md", "LOAD './target/release/finetype.duckdb_extension';",
                 "LOAD './target/release/finetype_duckdb.duckdb_extension';"),
            "names an artifact no build produces",
            None,
        ),
        (
            "a shell pipeline is put back into a ```sql fence",
            _sub("docs/LOCALE_GUIDE.md",
                 "SELECT * FROM customers\nWHERE json_extract_string",
                 "finetype infer -f phones.txt --output json\n\n"
                 "SELECT * FROM customers\nWHERE json_extract_string"),
            "a shell command inside a ```sql fence",
            None,
        ),
        (
            "a table macro's documented column TYPE drifts",
            _nothing,
            "DESCRIBE says",
            wrong_macro_shape,
        ),

        # ── the two that must NOT fire ───────────────────────────────────────
        (
            "STAYS GREEN: a confidence digit moves",
            _sub("docs/DEVELOPMENT.md", "│      0.956 │", "│      0.412 │"),
            None,
            None,
        ),
        (
            "STAYS GREEN: a predicted label changes",
            _sub("README.md", '"type":"datetime.date.mdy_slash"',
                 '"type":"datetime.date.dmy_slash"'),
            None,
            None,
        ),
    ]

    failed = 0
    with tempfile.TemporaryDirectory(prefix="ft-sql-selftest-") as tmp:
        pristine = Path(tmp) / "pristine"
        _stage(root, pristine)

        control = check(pristine, extension)
        if control:
            print("  CONTROL FAILED — the unmutated tree does not pass:")
            for failure in control:
                print(f"      {failure}")
            return 1
        print("  ok   control: unmutated tree passes")

        for name, mutate, expected, macro_shapes in cases:
            work = Path(tmp) / "work"
            if work.exists():
                shutil.rmtree(work)
            shutil.copytree(pristine, work)
            try:
                mutate(work)
                found = check(work, extension, macro_shapes)
                text = "\n".join(found)
            except Fatal as exc:
                text = str(exc)
                found = [text]
            if expected is None:
                if found:
                    print(f"  FLAKY {name}: the gate fired on model output")
                    for line in text.splitlines()[:4]:
                        print(f"      got: {line}")
                    failed += 1
                else:
                    print(f"  ok   {name}")
                continue
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
        print(f"\nself-test FAILED: {failed} of {len(cases)} cases wrong")
        return 1
    print(f"\nself-test passed: {len(cases)} cases, control clean")
    return 0


def _stage(root: Path, dest: Path) -> None:
    """A scratch tree of just the documents, plus the real model.

    The `models` symlink matters: without it `model_env` finds no
    `models/default` under the scratch root, every mutated run falls back to
    fetching the model from HuggingFace, and the self-test does eleven downloads
    on a cold CI cache instead of none.
    """
    for src in [root / "README.md"] + sorted((root / "docs").glob("*.md")):
        target = dest / src.relative_to(root)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, target)
    models = root / "models"
    if models.is_dir():
        (dest / "models").symlink_to(models.resolve(), target_is_directory=True)


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--extension", type=Path, default=None)
    parser.add_argument("--list", action="store_true", help="list the fences")
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    extension = (args.extension or (root / DEFAULT_EXTENSION)).resolve()

    if args.list:
        for fence in find_fences(root):
            statements = split_statements(fence)
            fixture = "fixture" if fence.rel in FIXTURES else "no fixture"
            print(f"{fence.where}  {len(statements)} statements  ({fixture})")
        return 0

    if shutil.which("duckdb") is None:
        print("FATAL: duckdb is not on PATH", file=sys.stderr)
        return 2
    if not extension.is_file():
        print(
            f"FATAL: no extension at {extension}\n"
            f"       Build it first: make build-extension",
            file=sys.stderr,
        )
        return 2
    reason = model_preflight(root)
    if reason:
        print(f"FATAL: {reason}", file=sys.stderr)
        return 2

    if args.self_test:
        try:
            return self_test(root, extension)
        except Fatal as exc:
            print(f"FATAL: {exc}", file=sys.stderr)
            return 2

    failures = check(root, extension)
    if failures:
        print("The documented SQL examples do not run, or do not have the shape the docs claim:\n", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(f"\n{len(failures)} problem(s).", file=sys.stderr)
        return 1

    fences = find_fences(root)
    total = sum(len(split_statements(fence)) for fence in fences)
    print(
        f"Every documented SQL example runs against the local build and has the "
        f"shape the docs claim: {len(fences)} fences, {total} statements."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
