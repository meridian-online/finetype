#!/usr/bin/env python3
"""Run every expression the registry publishes on its own type's samples.

`labels/definitions_*.yaml` is published: `finetype taxonomy --full` emits it,
and the type-registry pages render `transform` and each `decompose` part as a
copy-button code block beside the leaf's `samples`. A reader takes the two
together as a worked example. `scripts/check_taxonomy_content.py` checks that
the function names and cast targets in those expressions resolve; it says, in
its own WHAT IS *NOT* ASSERTED, that nothing checks an expression runs or
returns the field it names. This does.

WHAT IS DERIVED
    The expressions and samples, from the `registry-expressions` binary that
    `make build-extension` builds. It reads `labels/` through finetype-core's
    own `Taxonomy::from_directory`, so what runs here is the string the product
    publishes, escapes and block scalars included. A transform written as a
    folded block keeps a `\\u4EE4` escape as six literal characters; decoding it
    here would run an expression nobody ships and pass it.

    The results, from the duckdb CLI with no extension loaded and autoloading
    off: `transform` and `decompose` are the plain-DuckDB surface.

WHAT IS ASSERTED
    1. Every `transform` and every `decompose` part, run on every one of its
       leaf's `samples` with the sample standing in for `{col}`, returns a
       value: not NULL, and not an error.
    2. No `REGEXP_EXTRACT` call in either field, in any case, passes two
       arguments with a pattern that has a capturing group. Without a group
       index DuckDB returns the whole match, so `'^arn:([^:]+):'` reads
       `arn:aws:` where the part says partition. Pass the index, or drop the
       group if the whole match is the part.
    3. A `transform` is a string, and a `decompose` is a string or a mapping of
       part name to string. Any other shape is refused rather than skipped.

WHAT IS *NOT* ASSERTED
    That the value is the RIGHT value. A part that returns the wrong non-null
    field on every sample passes 1; assertion 2 closes the shape of that defect
    this registry has had, and nothing closes the rest.

    An expression that calls an `ft_` function. Those need the extension, so
    they are skipped and the count is printed; the registry has none today.

    `transform_ext`, which requires an extension by definition.

USAGE
    scripts/check_registry_expressions.py                  # gate labels/
    scripts/check_registry_expressions.py --self-test      # prove it detects
    scripts/check_registry_expressions.py --labels DIR --oracle PATH

Needs the duckdb CLI on PATH and `make build-extension`. Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

DEFAULT_ORACLE = "target/release/registry-expressions"
LABELS_DIR = "labels"
COLUMN = "registry_sample"
# Error messages are fetched one statement at a time, so a tree where
# everything fails does not cost one process per run.
MAX_ERROR_DETAILS = 200


class Fatal(Exception):
    """A derivation failed — nothing downstream is meaningful."""


@dataclass
class Expr:
    label: str
    where: str  # "transform" or "decompose.<part>"
    sql: str


# ══════════════════════════════════════════════════════════════════════════════
# READING THE ORACLE
# ══════════════════════════════════════════════════════════════════════════════


def read_oracle(oracle: Path, labels: Path) -> list[dict]:
    if not oracle.is_file():
        raise Fatal(f"{oracle} not found — run `make build-extension` first")
    proc = subprocess.run(
        [str(oracle), "--labels", str(labels)],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise Fatal(f"{oracle} exited {proc.returncode}: {proc.stderr.strip()}")
    records = [json.loads(line) for line in proc.stdout.splitlines() if line]
    if not records:
        raise Fatal(f"{oracle} emitted no leaves for {labels}")
    return records


def harvest(records: list[dict]) -> tuple[list[Expr], dict[str, list[str]], list[str]]:
    """The expressions, each leaf's samples, and the shapes refused."""
    exprs: list[Expr] = []
    samples: dict[str, list[str]] = {}
    refused: list[str] = []
    for record in records:
        label = record["label"]
        samples[label] = record["samples"]
        transform = record.get("transform")
        if transform is not None:
            if isinstance(transform, str):
                exprs.append(Expr(label, "transform", transform))
            else:
                refused.append(f"{label}: transform is not a string: {transform!r}")
        decompose = record.get("decompose")
        if decompose is None:
            continue
        if isinstance(decompose, str):
            exprs.append(Expr(label, "decompose", decompose))
        elif isinstance(decompose, dict):
            for part, sql in decompose.items():
                if isinstance(sql, str):
                    exprs.append(Expr(label, f"decompose.{part}", sql))
                else:
                    refused.append(
                        f"{label}: decompose.{part} is not a string: {sql!r}"
                    )
        else:
            refused.append(
                f"{label}: decompose is neither a string nor a mapping: {decompose!r}"
            )
    for label, values in samples.items():
        for index, value in enumerate(values):
            if not isinstance(value, str):
                refused.append(f"{label}: samples[{index}] is not a string: {value!r}")
    return exprs, samples, refused


# ══════════════════════════════════════════════════════════════════════════════
# ASSERTION 2 — a two-argument REGEXP_EXTRACT whose pattern captures
# ══════════════════════════════════════════════════════════════════════════════

REGEXP_EXTRACT_RE = re.compile(r"(?<![\w.$])regexp_extract\s*\(", re.IGNORECASE)
FT_CALL_RE = re.compile(r"(?<![\w.$])ft_[A-Za-z0-9_]*\s*\(", re.IGNORECASE)


def call_arguments(sql: str, open_paren: int) -> list[str] | None:
    """The top-level arguments of the call whose `(` is at `open_paren`.

    SQL string literals are skipped whole, `''` included, so a comma or a
    bracket inside a pattern neither splits an argument nor closes the call.
    None when the call is not closed.
    """
    args: list[str] = []
    depth = 0
    start = open_paren + 1
    i = start
    while i < len(sql):
        ch = sql[i]
        if ch == "'":
            i += 1
            while i < len(sql):
                if sql[i] == "'":
                    if i + 1 < len(sql) and sql[i + 1] == "'":
                        i += 2
                        continue
                    break
                i += 1
        elif ch == "(":
            depth += 1
        elif ch == ")":
            if depth == 0:
                args.append(sql[start:i].strip())
                return args
            depth -= 1
        elif ch == "," and depth == 0:
            args.append(sql[start:i].strip())
            start = i + 1
        i += 1
    return None


def string_literal(arg: str) -> str | None:
    """The text of `arg` if it is exactly one SQL string literal."""
    if len(arg) >= 2 and arg[0] == "'" and arg[-1] == "'":
        body = arg[1:-1]
        if "'" not in body.replace("''", ""):
            return body.replace("''", "'")
    return None


def capturing_groups(pattern: str) -> int:
    """How many capturing groups an RE2 pattern opens.

    An escaped `(` and a `(` inside a character class are literals. `(?:`,
    `(?i)`, `(?i:` and lookaround open no group; `(?P<name>` and `(?<name>`
    capture.
    """
    count = 0
    i = 0
    while i < len(pattern):
        ch = pattern[i]
        if ch == "\\":
            i += 2
            continue
        if ch == "[":
            i += 1
            if i < len(pattern) and pattern[i] == "^":
                i += 1
            if i < len(pattern) and pattern[i] == "]":
                i += 1
            while i < len(pattern) and pattern[i] != "]":
                i += 2 if pattern[i] == "\\" else 1
            i += 1
            continue
        if ch == "(":
            rest = pattern[i + 1 :]
            if not rest.startswith("?"):
                count += 1
            elif rest.startswith(("?P<", "?<")) and not rest.startswith(("?<=", "?<!")):
                count += 1
        i += 1
    return count


def uncaptured_extracts(expr: Expr) -> list[str]:
    problems = []
    for match in REGEXP_EXTRACT_RE.finditer(expr.sql):
        args = call_arguments(expr.sql, match.end() - 1)
        call = expr.sql[match.start() : match.end() + 40].split("\n")[0]
        if args is None:
            problems.append(f"{expr.label} {expr.where}: unclosed call {call!r}")
            continue
        if len(args) != 2:
            continue
        pattern = string_literal(args[1])
        if pattern is None:
            problems.append(
                f"{expr.label} {expr.where}: two-argument REGEXP_EXTRACT whose "
                f"pattern is not one string literal, so whether it captures "
                f"cannot be read: {args[1]!r}"
            )
        elif capturing_groups(pattern):
            problems.append(
                f"{expr.label} {expr.where}: two-argument REGEXP_EXTRACT with a "
                f"capturing group returns the whole match, not the group: "
                f"'{pattern}' — pass the group index, or drop the group"
            )
    return problems


# ══════════════════════════════════════════════════════════════════════════════
# ASSERTION 1 — every expression returns a value on every sample
# ══════════════════════════════════════════════════════════════════════════════

PRELUDE = (
    ".bail off\n"
    "SET autoinstall_known_extensions = false;\n"
    "SET autoload_known_extensions = false;\n"
    "SET TimeZone = 'UTC';\n"
)


def sql_literal(value: str) -> str:
    return "'" + value.replace("'", "''") + "'"


def statement(ident: int, expr: str, sample: str) -> str:
    # The expression sits on lines of its own so a `--` comment in it cannot
    # swallow the closing parenthesis.
    body = expr.replace("{col}", COLUMN)
    return (
        f"SELECT {ident} AS id, CAST((\n{body}\n) AS VARCHAR) AS out "
        f"FROM (SELECT {sql_literal(sample)}::VARCHAR AS {COLUMN});\n"
    )


def duckdb(script: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["duckdb", "-no-init", "-jsonlines"],
        input=script,
        capture_output=True,
        text=True,
        check=False,
    )


def run_all(runs: list[tuple[Expr, int, str]]) -> dict[int, str | None]:
    """Each run's text result; a run that errored is absent."""
    script = PRELUDE + "".join(
        statement(ident, expr.sql, sample) for ident, (expr, _, sample) in enumerate(runs)
    )
    proc = duckdb(script)
    results: dict[int, str | None] = {}
    for line in proc.stdout.splitlines():
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        results[int(row["id"])] = row["out"]
    return results


def error_detail(expr: Expr, sample: str) -> str:
    proc = duckdb(PRELUDE + statement(0, expr.sql, sample))
    lines = [line for line in proc.stderr.splitlines() if line.strip()]
    return lines[0] if lines else f"no row returned (duckdb exit {proc.returncode})"


# ══════════════════════════════════════════════════════════════════════════════
# THE GATE
# ══════════════════════════════════════════════════════════════════════════════


def check(labels: Path, oracle: Path) -> tuple[int, list[str]]:
    """The exit status and the report for one labels directory."""
    if shutil.which("duckdb") is None:
        raise Fatal("duckdb CLI not on PATH")
    records = read_oracle(oracle, labels)
    exprs, samples, problems = harvest(records)

    skipped = [expr for expr in exprs if FT_CALL_RE.search(expr.sql)]
    runnable = [expr for expr in exprs if not FT_CALL_RE.search(expr.sql)]

    for expr in exprs:
        problems.extend(uncaptured_extracts(expr))

    runs = [
        (expr, index, sample)
        for expr in runnable
        for index, sample in enumerate(samples.get(expr.label, []))
        if isinstance(sample, str)
    ]
    results = run_all(runs)
    details = 0
    for ident, (expr, index, sample) in enumerate(runs):
        if ident not in results:
            if details < MAX_ERROR_DETAILS:
                why = error_detail(expr, sample)
                details += 1
            else:
                why = "error (detail not fetched past the cap)"
            problems.append(
                f"{expr.label} {expr.where}: samples[{index}] {sample!r} errors: {why}"
            )
        elif results[ident] is None:
            problems.append(
                f"{expr.label} {expr.where}: samples[{index}] {sample!r} returns NULL"
            )

    report = [
        f"{len(records)} leaves, {len(exprs)} expressions, {len(runs)} runs "
        f"on samples, {len(skipped)} skipped for calling an ft_ function"
    ]
    report += [f"  skipped: {expr.label} {expr.where}" for expr in skipped]
    if problems:
        report.append(f"✗ {len(problems)} problem(s):")
        report += [f"  {problem}" for problem in problems]
        return 1, report
    report.append("✓ every expression returns a value on every sample of its leaf")
    return 0, report


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST
# ══════════════════════════════════════════════════════════════════════════════

CLEAN = r"""
fixture.clean.leaf:
  title: "Clean"
  transform: "CAST(REGEXP_EXTRACT({col}, '^([a-z]+)-', 1) AS VARCHAR)"
  decompose:
    word: "REGEXP_EXTRACT({col}, '^([a-z]+)-', 1)"
    whole: "REGEXP_EXTRACT({col}, '[a-z]+-[0-9]+')"
    noncapturing: "REGEXP_EXTRACT({col}, '(?:[a-z]+)-')"
    escaped_paren: "COALESCE(NULLIF(REGEXP_EXTRACT({col}, '\\(x\\)'), ''), 'none')"
    class_paren: "COALESCE(NULLIF(REGEXP_EXTRACT({col}, '[(]x'), ''), 'none')"
    comma_in_quantifier: "REGEXP_EXTRACT(TRIM({col}), '([a-z]{1,9})-', 1)"
    flags: "REGEXP_EXTRACT({col}, '(?i)[A-Z]+')"
    extension_only: "ft_something({col})"
  samples:
    - "abc-1"
    - "de-22"
    - "fgh-333"
"""

# Each case: (name, leaf YAML, fragment the report must carry). Every broken
# leaf fails on its LAST sample only, or statically, so a gate that tried one
# sample, or stopped at the first leaf, passes it.
CASES = [
    (
        "a decompose part returns NULL on one sample",
        """
fixture.null.part:
  title: "Null part"
  decompose:
    number: "TRY_CAST({col} AS INT)"
  samples:
    - "1"
    - "2"
    - "x"
""",
        "fixture.null.part decompose.number: samples[2] 'x' returns NULL",
    ),
    (
        "a transform errors on one sample",
        """
fixture.error.transform:
  title: "Error transform"
  transform: "CAST({col} AS INT)"
  samples:
    - "1"
    - "2"
    - "x"
""",
        "fixture.error.transform transform: samples[2] 'x' errors: Conversion Error",
    ),
    (
        "a pattern RE2 rejects",
        r"""
fixture.error.lookahead:
  title: "Lookahead"
  transform: "REGEXP_REPLACE({col}, '[.](?=[0-9])', '')"
  samples:
    - "1.5"
""",
        "fixture.error.lookahead transform: samples[0] '1.5' errors: Invalid Input Error",
    ),
    (
        "a folded transform whose escapes YAML never decodes",
        # The backslash is spliced in: a backslash-u escape written here is
        # exactly what an editor or a heredoc may decode on the way in.
        """
fixture.null.folded:
  title: "Folded"
  transform: >-
    CASE WHEN {col} LIKE '<BACKSLASH>u4EE4%' THEN 'reiwa' END
  samples:
    - "令和"
""".replace("<BACKSLASH>", chr(92)),
        "fixture.null.folded transform: samples[0] '令和' returns NULL",
    ),
    (
        "a two-argument REGEXP_EXTRACT that captures",
        """
fixture.capture.upper:
  title: "Capture"
  decompose:
    partition: "REGEXP_EXTRACT({col}, '^arn:([^:]+):')"
  samples:
    - "arn:aws:s3"
""",
        "fixture.capture.upper decompose.partition: two-argument REGEXP_EXTRACT "
        "with a capturing group",
    ),
    (
        "the same in lower case, nested in a cast",
        r"""
fixture.capture.lower:
  title: "Capture lower"
  decompose:
    group: "CAST(regexp_extract({col}, '-(\\d{2})-') AS VARCHAR)"
  samples:
    - "078-05-1120"
""",
        "fixture.capture.lower decompose.group: two-argument REGEXP_EXTRACT",
    ),
    (
        "a named group",
        """
fixture.capture.named:
  title: "Named"
  decompose:
    host: "REGEXP_EXTRACT({col}, '//(?P<host>[^/]+)')"
  samples:
    - "https://example.com/x"
""",
        "fixture.capture.named decompose.host: two-argument REGEXP_EXTRACT",
    ),
    (
        "a capture in a transform, with a comma inside the pattern",
        """
fixture.capture.transform:
  title: "Capture transform"
  transform: "REGEXP_EXTRACT({col}, '(a,b)')"
  samples:
    - "a,b"
""",
        "fixture.capture.transform transform: two-argument REGEXP_EXTRACT",
    ),
    (
        "a decompose shape that is not a string or a mapping",
        """
fixture.shape.list:
  title: "List"
  decompose:
    - "UPPER({col})"
  samples:
    - "a"
""",
        "fixture.shape.list: decompose is neither a string nor a mapping",
    ),
]


def self_test(oracle: Path) -> int:
    failures = []
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)

        def run(name: str, yaml_text: str) -> tuple[int, str]:
            labels = root / name.replace(" ", "_")
            labels.mkdir()
            (labels / "definitions_fixture.yaml").write_text(yaml_text, encoding="utf-8")
            status, report = check(labels, oracle)
            return status, "\n".join(report)

        status, text = run("clean", CLEAN)
        if status != 0:
            failures.append(f"clean fixture refused:\n{text}")
        elif "1 skipped for calling an ft_ function" not in text:
            failures.append(f"clean fixture did not count its ft_ call:\n{text}")
        else:
            print("  ✓ clean fixture passes, its ft_ call skipped and counted")

        for name, leaf, fragment in CASES:
            status, text = run(name, CLEAN + leaf)
            if status == 0 or fragment not in text:
                failures.append(
                    f"{name}: exit {status}, expected {fragment!r} in:\n{text}"
                )
            else:
                print(f"  ✓ refused: {name}")

    if failures:
        print("✗ self-test FAILED:")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print(f"✓ self-test: clean fixture passes and {len(CASES)} refusals fire")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
    parser.add_argument("--labels", default=LABELS_DIR)
    parser.add_argument("--oracle", default=DEFAULT_ORACLE)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    try:
        if args.self_test:
            return self_test(Path(args.oracle))
        status, report = check(Path(args.labels), Path(args.oracle))
    except Fatal as err:
        print(f"✗ {err}", file=sys.stderr)
        return 2
    print("\n".join(report))
    return status


if __name__ == "__main__":
    sys.exit(main())
