#!/usr/bin/env python3
"""Gate: a label fix is a measurement, not a lucky draw.

WHY THIS EXISTS
    A free-text column does not type the same way twice. Profile the same pool
    of values twice, with a different 100-value sample each time, and the label
    comes back different often enough that "it types correctly now" and "this
    draw typed correctly" are indistinguishable at review. Every label fix in
    this repository was verified by running `profile` once and reading the
    answer, so a fix and a draw looked the same, and the analyst downstream
    trusted a label that was one sample.

    This gate replaces the single draw with a distribution. It profiles each
    pool over many independent windows, takes the MODAL label, and requires it
    to be the one `tests/fixtures/label_stability/BASELINE.md` records.

WHAT IS ASSERTED, AND WHAT IS ONLY RECORDED
    Asserted, and a failure of either reddens this gate:

    R1  MODAL LABEL — the label a pool returns on more windows than any other
        is the one the baseline records for it.
    R2  THE `unknown` RULE — `unknown` is the modal label only of a pool the
        baseline marks `undecided`. `unknown` is written by a demotion guard,
        never by the model, so a pool that goes unknown has lost its label
        rather than changed it.

    Recorded, and NOT asserted: two figures, both written by `--remeasure`.

    The AGREEMENT — the modal label's share of the windows that read. It
    measures how unstable a column still is, and it moves with the model, the
    read path and the sample size, so asserting it would turn every legitimate
    improvement red.

    The WINDOWS READ — how many of the windows drawn came back as one column.
    It is not always all of them: measured on this branch with duckdb v1.5.5,
    one of `naics_description`'s sixty windows sniffs as three semicolon-
    delimited columns, which is the defect #124 narrowed rather than closed. A
    100-value sample of a published column can still widen where the whole file
    does not. This figure is RECORDED and not asserted because it moves with the
    duckdb version — this repository's CI installs v1.5.3 and a developer's
    machine may hold any later one — and a gate that reddens on a dependency's
    patch release gets switched off. What IS refused is a pool where no window
    read at all: a measurement that did not happen is never a pass.

    R2 IS NOT REDUNDANT WITH R1, and the case that separates them is the reason
    it is written out. The two NAICS pools are perfectly stable and perfectly
    wrong: every window returns `unknown`, so an agreement check scores them
    healthy and R1 — which compares against a baseline that records `unknown`
    for them — passes. R2 is what refuses a pool whose label has been demoted
    away, and it fires on a pool the baseline marks anything other than
    `undecided`.

EXIT CODES
    0  every pool's modal label is the one the baseline records
    1  a verdict: R1 or R2 failed
    2  the gate could not answer -- a missing binary, a fixture with no baseline
       row, a pool too small to draw a window from, a pool no window of which
       read as one column. A gate that cannot measure must not report a pass.

USAGE
    python3 scripts/check_label_stability.py              # gate the tree
    python3 scripts/check_label_stability.py --remeasure  # rewrite the figures
    python3 scripts/check_label_stability.py --self-test  # prove it can fail
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import random
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURE_DIR_REL = "tests/fixtures/label_stability"
BASELINE_REL = f"{FIXTURE_DIR_REL}/BASELINE.md"
DEFAULT_BINARY_REL = "target/release/finetype"

# The label a demotion guard writes when it takes a prediction away. It is never
# a model output, which is why R2 can treat it as a distinct outcome rather than
# as one label among the taxonomy's.
UNKNOWN = "unknown"

# The one status under which R2 permits `unknown` to be modal. A pool is
# `undecided` while no label exists for what it holds; every other status
# asserts that a label does exist, so `unknown` there is a loss.
UNDECIDED = "undecided"

# `profile --files` writes one output per input and only wires json-schema and
# datapackage through the per-file writer. json-schema is the cheaper of the two
# and carries the taxonomy label as `x-finetype-label`.
LABEL_KEY = "x-finetype-label"

# Batch mode exists so the model and the taxonomy load ONCE for every window in
# a run rather than once per window. Measured on this branch: 60 windows of one
# pool in 9 seconds batched, against about 1.5 seconds each spawned singly.
BATCH_FLAGS = ("--files", "--out-dir")


class Fatal(Exception):
    """The gate cannot answer. Exit 2, never a verdict."""


# ── the baseline ────────────────────────────────────────────────────────────

# `| `fixture` | 0.917 | 60/60 | `label` | status |` -- the five columns of the
# table in BASELINE.md. The fixture and the label are backticked in the file so
# a reader can tell a name from prose; the regex requires the backticks so that
# a row whose formatting has drifted is refused rather than half-parsed.
ROW_RE = re.compile(
    r"^\|\s*`(?P<fixture>[a-z0-9_]+)`"
    r"\s*\|\s*(?P<agreement>[0-9]\.[0-9]{3})"
    r"\s*\|\s*(?P<read>[0-9]+)/(?P<drawn>[0-9]+)"
    r"\s*\|\s*`(?P<label>[A-Za-z0-9_.]+)`"
    r"\s*\|\s*(?P<status>[a-z]+)\s*\|\s*$"
)

FRONTMATTER_RE = re.compile(r"^(?P<key>[a-z_]+):\s*(?P<value>.*?)\s*$")

# Every frontmatter key the gate reads, and the two AC1 requires the file to
# carry: the build that produced the figures and the read path it used.
REQUIRED_KEYS = ("measured", "binary", "build", "read_path", "draws", "window", "seed")


@dataclass(frozen=True)
class Row:
    """One pool's recorded expectation."""

    fixture: str
    agreement: float
    read: int
    drawn: int
    label: str
    status: str
    lineno: int


@dataclass(frozen=True)
class Baseline:
    front: dict[str, str]
    rows: tuple[Row, ...]

    @property
    def draws(self) -> int:
        return int(self.front["draws"])

    @property
    def window(self) -> int:
        return int(self.front["window"])

    @property
    def seed(self) -> str:
        return self.front["seed"]


def load_baseline(path: Path) -> Baseline:
    """Parse BASELINE.md, refusing anything it cannot read exactly."""
    if not path.is_file():
        raise Fatal(f"{path}: not found")
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        raise Fatal(f"{path}: no YAML frontmatter -- line 1 is not `---`")
    try:
        end = lines.index("---", 1)
    except ValueError:
        raise Fatal(f"{path}: frontmatter is never closed by a `---` line") from None

    front: dict[str, str] = {}
    for lineno, line in enumerate(lines[1:end], start=2):
        if not line.strip():
            continue
        m = FRONTMATTER_RE.match(line)
        if not m:
            raise Fatal(f"{path}:{lineno}: frontmatter line is not `key: value`")
        front[m.group("key")] = m.group("value")

    missing = [k for k in REQUIRED_KEYS if k not in front]
    if missing:
        raise Fatal(
            f"{path}: frontmatter is missing {', '.join(missing)}. "
            "A figure whose build and read path are not named is not a measurement."
        )
    for key in ("draws", "window"):
        if not front[key].isdigit() or int(front[key]) < 1:
            raise Fatal(f"{path}: frontmatter `{key}` is `{front[key]}`, not a positive integer")

    rows: list[Row] = []
    seen: set[str] = set()
    for lineno, line in enumerate(lines[end + 1 :], start=end + 2):
        if not line.startswith("| `"):
            continue
        m = ROW_RE.match(line)
        if not m:
            raise Fatal(
                f"{path}:{lineno}: table row does not parse. Expected a backticked "
                f"fixture, a three-decimal agreement, a read/drawn window count, a "
                f"backticked label and a status. Got: {line.strip()}"
            )
        fixture = m.group("fixture")
        if fixture in seen:
            raise Fatal(f"{path}:{lineno}: duplicate row for `{fixture}`")
        seen.add(fixture)
        rows.append(
            Row(
                fixture=fixture,
                agreement=float(m.group("agreement")),
                read=int(m.group("read")),
                drawn=int(m.group("drawn")),
                label=m.group("label"),
                status=m.group("status"),
                lineno=lineno,
            )
        )

    if not rows:
        raise Fatal(f"{path}: no table rows -- an empty baseline asserts nothing")
    return Baseline(front=front, rows=tuple(rows))


# ── drawing the windows ─────────────────────────────────────────────────────


def read_pool(path: Path) -> tuple[str, list[str]]:
    """The header name and every non-empty value of a single-column fixture.

    THE HEADER TRAVELS WITH THE WINDOW. `profile` takes the column name as a
    hint unless `--no-header-hint` is given, so a window written under a
    different header is not a sample of the same column -- it is a sample of a
    column nobody publishes. Each window CSV therefore carries the pool's own
    header, which is also why the windows cannot be packed into one wide CSV:
    sixty columns of the same name would be de-duplicated into sixty different
    hints.
    """
    with path.open(newline="", encoding="utf-8") as handle:
        reader = csv.reader(handle)
        try:
            header_row = next(reader)
        except StopIteration:
            raise Fatal(f"{path}: empty fixture") from None
        if len(header_row) != 1:
            raise Fatal(
                f"{path}: header has {len(header_row)} columns; a pool is one column"
            )
        values = [row[0] for row in reader if row and row[0] != ""]
    return header_row[0], values


def draw_windows(values: list[str], draws: int, window: int, seed: str, fixture: str) -> list[list[str]]:
    """`draws` independent samples of `window` values, seeded per fixture.

    Seeded per fixture rather than once for the run, so that adding a pool or
    changing the order of the table does not move every other pool's windows.
    Sampling is without replacement WITHIN a window and independent BETWEEN
    windows, which is what makes the spread of labels a property of the column
    rather than of one sample.
    """
    if len(values) < window:
        raise Fatal(
            f"{fixture}: pool holds {len(values)} values, fewer than the "
            f"{window}-value window the baseline draws"
        )
    rng = random.Random(f"{seed}:{fixture}")
    return [rng.sample(values, window) for _ in range(draws)]


def write_window(path: Path, header: str, values: list[str]) -> None:
    with path.open("w", newline="", encoding="utf-8") as handle:
        # `\n`, not the csv module's default `\r\n`: the fixtures are written
        # by duckdb's own COPY with bare newlines, and a window that differs
        # from its pool in line terminator is one more variable between a label
        # that moved and a reader that did.
        writer = csv.writer(handle, lineterminator="\n")
        writer.writerow([header])
        for value in values:
            writer.writerow([value])


# ── profiling ───────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class Measurement:
    fixture: str
    labels: tuple[str, ...]
    widened: tuple[str, ...]

    @property
    def modal(self) -> str:
        return Counter(self.labels).most_common(1)[0][0]

    @property
    def agreement(self) -> float:
        return Counter(self.labels).most_common(1)[0][1] / len(self.labels)

    @property
    def spread(self) -> str:
        return ", ".join(
            f"`{label}` on {count} of {len(self.labels)}"
            for label, count in Counter(self.labels).most_common()
        )


def profile_windows(
    binary: Path, fixtures: dict[str, Path], baseline: Baseline, workdir: Path
) -> dict[str, Measurement]:
    """Draw every pool's windows, profile them in one batch, collect the labels."""
    windows_dir = workdir / "windows"
    out_dir = workdir / "out"
    windows_dir.mkdir(parents=True, exist_ok=True)
    out_dir.mkdir(parents=True, exist_ok=True)

    planned: dict[str, list[Path]] = {}
    paths: list[Path] = []
    for fixture, csv_path in fixtures.items():
        header, values = read_pool(csv_path)
        windows = draw_windows(values, baseline.draws, baseline.window, baseline.seed, fixture)
        planned[fixture] = []
        for index, window in enumerate(windows):
            # The stem is the join between an input and its output file, so it
            # has to be unique across the whole batch, not within a pool.
            target = windows_dir / f"{fixture}__{index:04d}.csv"
            write_window(target, header, window)
            planned[fixture].append(target)
            paths.append(target)

    listing = workdir / "paths.txt"
    listing.write_text("\n".join(str(p) for p in paths) + "\n", encoding="utf-8")

    log = workdir / "profile.log"
    with log.open("w", encoding="utf-8") as handle:
        completed = subprocess.run(
            [
                str(binary),
                "profile",
                BATCH_FLAGS[0],
                str(listing),
                BATCH_FLAGS[1],
                str(out_dir),
                "-o",
                "json-schema",
            ],
            stdout=handle,
            stderr=subprocess.STDOUT,
            check=False,
        )
    if completed.returncode != 0:
        tail = "\n".join(log.read_text(encoding="utf-8").splitlines()[-20:])
        raise Fatal(f"`profile --files` exited {completed.returncode}:\n{tail}")

    measurements: dict[str, Measurement] = {}
    for fixture, window_paths in planned.items():
        labels: list[str] = []
        widened: list[str] = []
        for window_path in window_paths:
            out_path = out_dir / f"{window_path.stem}.json"
            if not out_path.is_file():
                raise Fatal(
                    f"{fixture}: `profile --files` wrote no output for {window_path.name}. "
                    "A window that was not profiled is not a window that agreed."
                )
            document = json.loads(out_path.read_text(encoding="utf-8"))
            properties = document.get("properties", {})
            if len(properties) != 1:
                # The read path widened the schema: #124 made `profile` sniff the
                # shape first and read with the column list the sniff pinned, and
                # `tests/smoke.sh` asserts it on this fixture family by name. A
                # window read as several columns is a regression of that, and the
                # labels collected from it are labels of columns nobody published.
                widened.append(f"{window_path.name} read as {len(properties)} columns")
                continue
            column = next(iter(properties.values()))
            label = column.get(LABEL_KEY)
            if not isinstance(label, str):
                raise Fatal(
                    f"{fixture}: {out_path.name} carries no `{LABEL_KEY}`. "
                    "The output shape moved and this gate is reading the wrong key."
                )
            labels.append(label)
        if not labels:
            raise Fatal(
                f"{fixture}: not one of the {len(window_paths)} windows drawn read as a "
                f"single column ({len(widened)} widened). There is nothing here to take "
                "a modal label of, so this is a refusal to answer and not a pass."
            )
        measurements[fixture] = Measurement(
            fixture=fixture, labels=tuple(labels), widened=tuple(widened)
        )
    return measurements


# ── the verdict ─────────────────────────────────────────────────────────────


def resolve_fixtures(fixture_dir: Path, baseline: Baseline, only: list[str]) -> dict[str, Path]:
    """Pair every baseline row with its CSV, refusing drift in either direction."""
    if not fixture_dir.is_dir():
        raise Fatal(f"{fixture_dir}: not a directory")
    on_disk = {p.stem: p for p in sorted(fixture_dir.glob("*.csv"))}
    recorded = {row.fixture for row in baseline.rows}

    missing = sorted(recorded - set(on_disk))
    if missing:
        raise Fatal(
            f"{fixture_dir}: the baseline records {', '.join(missing)} and no such CSV exists. "
            "A pool that cannot be drawn from cannot be reported stable."
        )
    unrecorded = sorted(set(on_disk) - recorded)
    if unrecorded:
        raise Fatal(
            f"{fixture_dir}: {', '.join(unrecorded)} has no row in the baseline. "
            "A fixture nothing records is a pool nothing asserts."
        )

    if not only:
        return {row.fixture: on_disk[row.fixture] for row in baseline.rows}
    unknown_names = sorted(set(only) - recorded)
    if unknown_names:
        raise Fatal(f"--only names {', '.join(unknown_names)}, which the baseline does not record")
    return {row.fixture: on_disk[row.fixture] for row in baseline.rows if row.fixture in only}


def verdict(baseline: Baseline, measurements: dict[str, Measurement]) -> list[str]:
    """Every violation of R1 and R2, named so a reader can act without a rerun."""
    violations: list[str] = []
    for row in baseline.rows:
        measured = measurements.get(row.fixture)
        if measured is None:
            continue
        if measured.modal != row.label:
            violations.append(
                f"{row.fixture}: modal label moved. BASELINE.md records "
                f"`{row.label}`; this build returns `{measured.modal}` "
                f"({measured.spread})."
            )
        if measured.modal == UNKNOWN and row.status != UNDECIDED:
            violations.append(
                f"{row.fixture}: `{UNKNOWN}` is the modal label of a pool the baseline "
                f"marks `{row.status}`. `{UNKNOWN}` is written by the demotion guard, "
                f"never by the model, so this pool has lost its label rather than "
                f"changed it. Only an `{UNDECIDED}` pool may be modally `{UNKNOWN}`."
            )
    return violations


def report(baseline: Baseline, measurements: dict[str, Measurement]) -> None:
    print(
        f"label stability: {len(measurements)} pools, {baseline.draws} windows of "
        f"{baseline.window} values, seed {baseline.seed}"
    )
    for row in baseline.rows:
        measured = measurements.get(row.fixture)
        if measured is None:
            continue
        print(
            f"  {row.fixture:<22} modal {measured.modal:<38} "
            f"agreement {measured.agreement:.3f}  "
            f"read {len(measured.labels)}/{baseline.draws}  "
            f"(baseline {row.agreement:.3f} {row.read}/{row.drawn} {row.status})"
        )
        for widened in measured.widened:
            # Not a verdict -- see the module docstring. It is printed every run
            # because a figure that only lives in a table nobody re-reads is how
            # a narrowed defect becomes a forgotten one.
            print(f"    read path widened: {widened}")


# ── re-measuring ────────────────────────────────────────────────────────────


def remeasure(path: Path, baseline: Baseline, measurements: dict[str, Measurement], binary: Path) -> None:
    """Write the observed agreement figures and the provenance back into the file.

    IT REFUSES TO MOVE A MODAL LABEL, and that refusal is the point. Agreement
    is a recorded measurement and re-measuring it is bookkeeping; the modal
    label is the thing this gate asserts, and a check that rewrites its own
    expectation to match what it just observed asserts nothing at all.
    """
    moved = [
        f"{row.fixture} (`{row.label}` -> `{measurements[row.fixture].modal}`)"
        for row in baseline.rows
        if row.fixture in measurements and measurements[row.fixture].modal != row.label
    ]
    if moved:
        raise Fatal(
            "--remeasure refuses: the modal label moved on " + ", ".join(moved) + ". "
            "That is the assertion, not the measurement. Decide whether the move is a "
            "fix or a regression and edit the row by hand, in the commit that caused it."
        )

    text = path.read_text(encoding="utf-8")
    version = subprocess.run(
        [str(binary), "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    commit = subprocess.run(
        ["git", "-C", str(ROOT), "rev-parse", "--short", "HEAD"],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()

    replacements = {
        "binary": f"{version} (built from this tree)",
        "build": commit,
    }
    lines = text.splitlines()
    end = lines.index("---", 1)
    for index, line in enumerate(lines[1:end], start=1):
        m = FRONTMATTER_RE.match(line)
        if m and m.group("key") in replacements:
            lines[index] = f"{m.group('key')}: {replacements[m.group('key')]}"

    for row in baseline.rows:
        measured = measurements.get(row.fixture)
        if measured is None:
            continue
        old = lines[row.lineno - 1]
        rewritten = re.sub(
            r"(\|\s*)[0-9]\.[0-9]{3}(\s*\|)",
            rf"\g<1>{measured.agreement:.3f}\g<2>",
            old,
            count=1,
        )
        lines[row.lineno - 1] = re.sub(
            r"(\|\s*)[0-9]+/[0-9]+(\s*\|)",
            rf"\g<1>{len(measured.labels)}/{baseline.draws}\g<2>",
            rewritten,
            count=1,
        )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{path}: agreement figures and provenance rewritten from this run")


# ── the self-test ───────────────────────────────────────────────────────────

SELFTEST_DOC = """Prove this gate can fail, by breaking what it reads and requiring it to redden.

A gate that is only known to pass is not known to detect. Each case below
mutates one thing and names the message the gate has to produce. The control
case runs the same command against an unmutated copy and requires a pass --
without it, a gate that failed unconditionally would clear every other case.
"""


def _mutate(source: Path, target: Path, pattern: str, replacement: str) -> None:
    """Rewrite one line of a copied file, REFUSING a pattern that matched nothing.

    A mutation that silently failed to apply leaves the gate looking at an
    unmutated tree, and a gate that then passes is indistinguishable from a gate
    that cannot detect. This self-test lost a case to exactly that: the table
    grew a column, the substitution stopped matching, and the case reported the
    gate had not detected a label it had never been shown. The refusal is the
    fix, because the harness now cannot report on a mutation it did not make.
    """
    text = source.read_text(encoding="utf-8")
    mutated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise Fatal(
            f"self-test: the mutation {pattern!r} matched {count} lines of {source}, not 1. "
            "The file's shape has moved and this case is not testing what it says."
        )
    target.write_text(mutated, encoding="utf-8")


def _run_gate(args: list[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def _case(name: str, args: list[str], expect_exit: int, expect_text: list[str], forbid: list[str]) -> bool:
    done = _run_gate(args)
    output = done.stdout + done.stderr
    problems: list[str] = []
    if done.returncode != expect_exit:
        problems.append(f"exited {done.returncode}, expected {expect_exit}")
    for needle in expect_text:
        if needle not in output:
            problems.append(f"output does not name {needle!r}")
    for needle in forbid:
        if needle in output:
            problems.append(f"output names {needle!r}, which this case must NOT trip")
    if problems:
        print(f"  FAIL  {name}")
        for problem in problems:
            print(f"        {problem}")
        print("        ---- gate output ----")
        for line in output.splitlines()[-25:]:
            print(f"        {line}")
        return False
    print(f"  ok    {name}")
    return True


def self_test(binary: Path) -> int:
    print(SELFTEST_DOC)
    real_fixtures = ROOT / FIXTURE_DIR_REL
    real_baseline = ROOT / BASELINE_REL
    passed = True

    with tempfile.TemporaryDirectory(prefix="label-stability-selftest-") as tmp:
        scratch = Path(tmp)

        # The two pools every case below is built on, and why they are these two.
        # `gleif_corpus` is a pool the baseline marks with a real label, so it is
        # where a moved label and a lost label can both be planted.
        # `naics_corpus` is the pool the baseline marks `undecided`, so it is the
        # only one on which R2 can be tripped WITHOUT R1 firing alongside it.
        stable = "gleif_corpus"
        undecided = "naics_corpus"

        baseline = load_baseline(real_baseline)
        recorded = {row.fixture: row for row in baseline.rows}
        stable_label = recorded[stable].label

        # ── control ─────────────────────────────────────────────────────────
        passed &= _case(
            "unmutated tree passes",
            ["--binary", str(binary), "--only", f"{stable},{undecided}"],
            expect_exit=0,
            expect_text=[stable, undecided],
            forbid=["modal label moved", "is the modal label of a pool"],
        )

        # ── R1: the recorded label is what the gate compares against ─────────
        moved = scratch / "moved-baseline.md"
        wrong_label = "representation.text.entity_name"
        _mutate(
            real_baseline,
            moved,
            rf"(^\|\s*`{stable}`\s*\|[^|]*\|[^|]*\|\s*)`[^`]+`",
            rf"\g<1>`{wrong_label}`",
        )
        passed &= _case(
            "a moved modal label is refused, naming the pool and both labels",
            ["--binary", str(binary), "--baseline", str(moved), "--only", stable],
            expect_exit=1,
            expect_text=["modal label moved", stable, wrong_label, stable_label],
            forbid=[],
        )

        # ── R2: `unknown` on a pool the baseline does not mark undecided ─────
        demoted = scratch / "demoted-baseline.md"
        _mutate(
            real_baseline,
            demoted,
            rf"(^\|\s*`{undecided}`\s*\|.*\|\s*)undecided(\s*\|)$",
            r"\g<1>unstable\g<2>",
        )
        passed &= _case(
            "`unknown` modal on a pool marked `unstable` is refused, naming the pool",
            ["--binary", str(binary), "--baseline", str(demoted), "--only", undecided],
            expect_exit=1,
            expect_text=[
                f"{undecided}: `{UNKNOWN}` is the modal label of a pool",
                "unstable",
            ],
            # R1 must NOT fire here: the recorded label is still `unknown` and so
            # is the observed one. If it does, R2 is not a rule of its own and
            # the NAICS case it exists for is uncovered.
            forbid=["modal label moved"],
        )

        # ── the observation is measured, not assumed ─────────────────────────
        # The two cases above mutate what the gate EXPECTS. A gate that never
        # profiled anything and simply disagreed with its own baseline would
        # pass both. This one mutates what the gate SEES: a pool the baseline
        # marks with a real label is filled with the prose that profiles as
        # `unknown`, under its own header, and both rules have to fire.
        swapped = scratch / "fixtures"
        shutil.copytree(real_fixtures, swapped)
        swap_header, _ = read_pool(real_fixtures / f"{stable}.csv")
        _, prose = read_pool(real_fixtures / f"{undecided}.csv")
        write_window(swapped / f"{stable}.csv", swap_header, prose)
        passed &= _case(
            "a pool whose values have gone unknown is refused by both rules",
            ["--binary", str(binary), "--fixtures", str(swapped), "--only", stable],
            expect_exit=1,
            expect_text=[
                "modal label moved",
                f"{stable}: `{UNKNOWN}` is the modal label of a pool",
            ],
            forbid=[],
        )

        # ── silence is not a pass ────────────────────────────────────────────
        thinned = scratch / "thinned"
        shutil.copytree(real_fixtures, thinned)
        (thinned / f"{stable}.csv").unlink()
        passed &= _case(
            "a fixture the baseline records but the tree does not hold is refused",
            ["--binary", str(binary), "--fixtures", str(thinned)],
            expect_exit=2,
            expect_text=["and no such CSV exists", stable],
            forbid=[],
        )

        no_binary = scratch / "not-a-finetype"
        no_binary.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        no_binary.chmod(0o755)
        passed &= _case(
            "a binary that profiles nothing is refused, not reported green",
            ["--binary", str(no_binary), "--only", undecided],
            expect_exit=2,
            expect_text=["wrote no output"],
            forbid=[],
        )

    print()
    if passed:
        print("self-test: every case reddened the gate as required")
        return 0
    print("self-test: the gate did not detect something it must detect")
    return 1


# ── entry point ─────────────────────────────────────────────────────────────


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", default=None, help=f"finetype binary (default {DEFAULT_BINARY_REL}, or $FINETYPE)")
    parser.add_argument("--baseline", default=None, help=f"baseline file (default {BASELINE_REL})")
    parser.add_argument("--fixtures", default=None, help=f"fixture directory (default {FIXTURE_DIR_REL})")
    parser.add_argument("--only", default="", help="comma-separated pools to measure (default: every recorded pool)")
    parser.add_argument("--remeasure", action="store_true", help="rewrite the agreement figures and provenance from this run")
    parser.add_argument("--self-test", action="store_true", help="prove this gate can fail")
    args = parser.parse_args(argv)

    binary = Path(args.binary or os.environ.get("FINETYPE") or ROOT / DEFAULT_BINARY_REL).resolve()

    try:
        if args.self_test:
            if not binary.is_file():
                raise Fatal(
                    f"{binary}: not found. Build it first: cargo build --release -p finetype-cli"
                )
            return self_test(binary)

        if not binary.is_file():
            raise Fatal(
                f"{binary}: not found. Build it first: cargo build --release -p finetype-cli"
            )
        baseline_path = Path(args.baseline) if args.baseline else ROOT / BASELINE_REL
        fixture_dir = Path(args.fixtures) if args.fixtures else ROOT / FIXTURE_DIR_REL
        only = [name for name in args.only.split(",") if name]

        baseline = load_baseline(baseline_path)
        fixtures = resolve_fixtures(fixture_dir, baseline, only)
        with tempfile.TemporaryDirectory(prefix="label-stability-") as tmp:
            measurements = profile_windows(binary, fixtures, baseline, Path(tmp))
            report(baseline, measurements)
            if args.remeasure:
                remeasure(baseline_path, baseline, measurements, binary)
                return 0
            violations = verdict(baseline, measurements)
    except Fatal as exc:
        print(f"label stability: cannot answer -- {exc}", file=sys.stderr)
        return 2

    if violations:
        print()
        for violation in violations:
            print(f"REFUSED: {violation}")
        return 1
    print("every pool's modal label is the one the baseline records")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
