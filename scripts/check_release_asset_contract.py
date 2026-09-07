#!/usr/bin/env python3
"""Gate the third link: what `release.yml` uploads against what the assembler makes.

There are three places a release asset can be lost, and each of them is a
DECLARATION IN release.yml that some other file has to be read against.

    1. the artifacts tree     -- assemble-release-assets.sh exits 1/3/4/5 when
                                 a platform, an arch, the catalogue or a
                                 checksum is absent from what it was handed
    2. the artifact uploads   -- `if-no-files-found: error` fires only when the
                                 UNION of a step's globs comes up empty, so
                                 dropping one glob of three is silent. F below
                                 is what reads that
    3. the release step's
       `files:` list          -- guarded by NOTHING before this file existed

Deleting `release-assets/taxonomy-schemas.json` from that list leaves the
assembler at exit 0 and every other gate green, and the tag publishes five
extension binaries and no type source. `fail_on_unmatched_files: true` does not
help: it fires on an entry that is PRESENT and matches nothing, and says
nothing about an entry that is gone. The same shape applies to the
`--threshold` the release workflow hands `check_model_coverage.py` -- set it to
0.0 and the coverage gate is a no-op at release time with nothing in this
repository noticing, and `continue-on-error: true` on that same step reaches
the identical outcome by a different route (G).

WHAT IS CHECKED
    A  Every file `assemble-release-assets.sh` actually produces is matched by
       at least one glob in the release step's `files:` list.
    B  Every glob in that list matches at least one produced file. A glob that
       can never match is what `fail_on_unmatched_files: true` turns into a
       failed release, so it is a failure here, on a pull request, instead.
    C  `fail_on_unmatched_files: true` is set, so B's property is enforced at
       release time as well as here.
    D  The `--threshold` the release workflow passes `check_model_coverage.py`
       equals that script's own `MIN_COVERAGE`, and behaves: it refuses a
       catalogue covering half the model's labels and accepts one covering all
       of them. Equality is deliberate rather than a range. A release that
       wants a stricter bar moves `MIN_COVERAGE`, where the reasoning for the
       number already lives, and both places move together.
    E  Every asset the release publishes carries its `.sha256` and every
       published `.sha256` carries its asset -- measured on the files the
       `files:` globs actually match, so a sidecar that is assembled and not
       published is caught as well as one that is never assembled.
    F  Every glob in an upload step's `path:` list is LOAD-BEARING: with that
       one glob removed the release path refuses. This is the rung that reads
       the build job's uploads. Drop `finetype-*.sha256` from them and nothing
       else in this repository notices -- the union is non-empty so the upload
       is silent, all five archives assemble, and the tag publishes them with
       no checksum. It also enumerates the taxonomy upload's claim that its
       four globs are exactly the assembler's exit-4 list: each one, removed,
       has to produce that refusal.
    G  No job and no step in the release workflow carries `continue-on-error`.
       A refusal that leaves the job green is the same nothing as a deleted
       refusal, and the coverage step is only one of the places it would land.
    H  The tap formula's asset check runs, unguarded, before the step that
       pushes the formula -- the ordering that comment claims for it.
    I  Every artifact the assembler expects to download is uploaded by a step
       here, and the Rust target triples it names extension binaries for are
       exactly the ones the build matrix ships a CLI archive for -- so no
       platform ends up with one half of a release and a 404 for the other.

HOW, AND WHY IT IS NOT A SOURCE SCAN
    A, B, E, F and I RUN the assembler against a synthetic artifacts tree and
    glob the real directory it writes; D IMPORTS `check_model_coverage` and
    runs it as a subprocess. Nothing here parses the assembler's source or
    reads a number out of it, because the defect this whole card kept producing
    is a check that reads the shape of some code instead of asking what the
    code does.

    THE TREE IS NOT WRITTEN HERE EITHER. It comes from the assembler's own
    `--make-fixture`, the same builder its self-test and the stamp gate's
    release rehearsal use, and is then FILTERED THROUGH THE UPLOAD GLOBS the
    workflow declares: a file no `path:` entry matches is a file the artifact
    does not carry, so deleting a glob deletes those files here exactly as it
    would on a tag. A fixture that stated its own sidecars would have made F
    unfalsifiable.

    What IS read as text is `release.yml`, which is unavoidable: it is the
    declaration under test. Its jobs, steps, guards and `continue-on-error`
    keys come from `.github/scripts/gate-self-tests.py`'s `scan_workflow`,
    imported rather than reimplemented -- the same reader
    `scripts/check_extension_stamp.py` asks its own questions of. The `with:`
    mappings (`path:`, `name:`, `files:`, `fail_on_unmatched_files:`) and the
    build matrix are read here, because that reader does not model them, and
    every shape they cannot read exactly is REFUSED with exit 2.

USAGE
    scripts/check_release_asset_contract.py
    scripts/check_release_asset_contract.py --workflow <release.yml> \\
        --assembler <assemble-release-assets.sh>
    scripts/check_release_asset_contract.py --self-test

EXIT CODES
    0  the release step ships exactly what the assembler assembles, and the
       release workflow's coverage threshold is the reviewed one
    1  the contract is broken -- a produced asset no glob matches, a glob
       nothing matches, a missing or wrong threshold
    2  the check could not run: the workflow could not be read unambiguously,
       or the assembler did not assemble

Stdlib only.
"""

from __future__ import annotations

import argparse
import fnmatch
import glob
import importlib.util
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_REL = ".github/workflows/release.yml"
ASSEMBLER_REL = ".github/scripts/assemble-release-assets.sh"
COVERAGE_REL = "scripts/check_model_coverage.py"
ROUTER_REL = ".github/scripts/gate-self-tests.py"

RELEASE_ACTION = "softprops/action-gh-release"
UPLOAD_ACTION = "actions/upload-artifact"
# The check the release runs against the formula it has just written, and the
# step that publishes that formula. H asserts the order, which is the whole of
# what makes the check worth running.
FORMULA_CHECK = "check-formula-asset.sh"
FORMULA_PUSH = "git push"
FIXTURE_TAG = "vCONTRACT"
FIXTURE_EXT_VERSION = "vTEST"
# The in-artifact filename every extension build produces, and the directory
# shape the reusable workflow uploads them under. Those artifacts come from
# `duckdb/extension-ci-tools`, not from an upload step in this workflow, so
# they are the one part of the tree no `path:` list here explains.
EXTENSION_ARTIFACT_DIR = f"finetype-{FIXTURE_EXT_VERSION}-extension-*"


class Unreadable(Exception):
    """The workflow could not be read unambiguously. Exit 2, never a verdict."""


@dataclass(frozen=True)
class Upload:
    """One `actions/upload-artifact` step, resolved for one matrix leg."""

    job: str
    lineno: int
    artifact: str
    globs: tuple[str, ...]
    if_no_files_found: str
    # The Rust target triple this leg builds for, or "" for an unmatrixed
    # upload. Read off the matrix leg rather than parsed back out of the
    # artifact name, which would agree with whatever the name got wrong.
    target: str = ""


# ── reading the declaration under test ──────────────────────────────────────


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def _is_blank_or_comment(line: str) -> bool:
    stripped = line.strip()
    return stripped == "" or stripped.startswith("#")


def release_step(lines: list[str]) -> tuple[list[str], str]:
    """Return (files globs, fail_on_unmatched_files value) from the release step.

    Refuses unless exactly one step in the workflow uses the release action:
    two of them, or none, means this file is reading something other than what
    it thinks it is.
    """
    hits = [
        i
        for i, line in enumerate(lines)
        if not line.lstrip().startswith("#") and RELEASE_ACTION in line and "uses:" in line
    ]
    if len(hits) != 1:
        raise Unreadable(
            f"expected exactly one step using `{RELEASE_ACTION}`, found {len(hits)}"
            + (f" at lines {[i + 1 for i in hits]}" if hits else "")
        )
    start = hits[0]
    # The step's own indent: `- uses:` or `uses:` under a `- name:`.
    step_indent = _indent(lines[start].replace("- ", "  ", 1) if lines[start].lstrip().startswith("- ") else lines[start])

    files: list[str] | None = None
    unmatched_setting: str | None = None
    i = start + 1
    while i < len(lines):
        line = lines[i]
        if not _is_blank_or_comment(line) and _indent(line) < step_indent:
            break
        if _is_blank_or_comment(line):
            i += 1
            continue
        stripped = line.strip()
        if stripped.startswith("fail_on_unmatched_files:"):
            unmatched_setting = stripped.split(":", 1)[1].strip()
        if re.fullmatch(r"files:\s*\|-?", stripped):
            block_indent = _indent(line)
            files = []
            j = i + 1
            while j < len(lines):
                entry = lines[j]
                if entry.strip() == "":
                    j += 1
                    continue
                if _indent(entry) <= block_indent:
                    break
                files.append(entry.strip())
                j += 1
            i = j
            continue
        if stripped.startswith("files:"):
            raise Unreadable(
                f"{WORKFLOW_REL}:{i + 1}: `files:` is not a `|` block scalar; this reader "
                "handles only the block form the release step uses"
            )
        i += 1

    if files is None:
        raise Unreadable(f"{WORKFLOW_REL}: the `{RELEASE_ACTION}` step has no `files: |` block")
    if not files:
        raise Unreadable(f"{WORKFLOW_REL}: the `{RELEASE_ACTION}` step's `files:` block is empty")
    if unmatched_setting is None:
        return files, "<absent>"
    return files, unmatched_setting


def coverage_threshold(lines: list[str]) -> str:
    """Return the `--threshold` value the workflow hands check_model_coverage.py."""
    hits = [
        i
        for i, line in enumerate(lines)
        if "check_model_coverage.py" in line and not line.lstrip().startswith("#")
    ]
    if len(hits) != 1:
        raise Unreadable(
            f"expected exactly one invocation of check_model_coverage.py in {WORKFLOW_REL}, "
            f"found {len(hits)}"
        )
    command = ""
    i = hits[0]
    while i < len(lines):
        piece = lines[i].strip()
        command += " " + piece.rstrip("\\")
        if not piece.endswith("\\"):
            break
        i += 1
    found = re.findall(r"--threshold[=\s]+(\S+)", command)
    if len(found) != 1:
        raise Unreadable(
            f"expected exactly one `--threshold` in the check_model_coverage.py invocation, "
            f"found {len(found)}"
        )
    return found[0]


def _workflow_reader():
    """gate-self-tests.py's workflow reader, imported rather than reimplemented.

    Jobs, steps, guards, `uses:` and `continue-on-error:` come from there.
    `scripts/check_extension_stamp.py` asks the same reader its own questions,
    so a workflow shape none of us can parse is refused once rather than
    answered three different ways by three parsers that have drifted.
    """
    path = REPO_ROOT / ROUTER_REL
    spec = importlib.util.spec_from_file_location("_finetype_gate_router", path)
    if spec is None or spec.loader is None:
        raise Unreadable(f"{ROUTER_REL} could not be imported")
    module = importlib.util.module_from_spec(spec)
    # Registered BEFORE it executes: `@dataclass` resolves its field types
    # through `sys.modules[cls.__module__]`, which is None for a module that is
    # only half imported.
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 -- any import failure is "cannot run"
        raise Unreadable(f"{ROUTER_REL} could not be imported: {exc}") from None
    return module


def structure(workflow: Path):
    """(jobs, steps) for any workflow path, including a mutated copy in /tmp."""
    reader = _workflow_reader()
    try:
        return reader.scan_workflow(workflow.parent, workflow.name)
    except Exception as exc:  # noqa: BLE001 -- Fatal is the reader's, not ours
        raise Unreadable(f"{workflow.name} could not be read: {exc}") from None


_SCALAR_BLOCK = {"|", "|-", "|+", ">", ">-", ">+"}
_KEY = re.compile(r"([A-Za-z0-9_.-]+):(?:[ ](.*))?$")
# `${{ … }}`, the only expression syntax any of the values this file reads use.
_EXPRESSION = re.compile(r"\$\{\{\s*([A-Za-z0-9_.-]+)\s*\}\}")


def _scalar_block(lines: list[str], start: int, key_indent: int) -> tuple[list[str], int]:
    out: list[str] = []
    i = start
    while i < len(lines):
        if lines[i].strip() == "":
            i += 1
            continue
        if _indent(lines[i]) <= key_indent:
            break
        out.append(lines[i].strip())
        i += 1
    return out, i


def _mapping(lines: list[str], start: int, key_indent: int, where: str) -> dict[str, list[str]]:
    """`key: value` entries at exactly `key_indent`; block scalars kept as lists."""
    out: dict[str, list[str]] = {}
    i = start
    while i < len(lines):
        line = lines[i]
        if _is_blank_or_comment(line):
            i += 1
            continue
        depth = _indent(line)
        if depth < key_indent:
            break
        if depth > key_indent:
            raise Unreadable(
                f"{where}:{i + 1}: indented deeper than the mapping it is in; this reader "
                "handles scalars and `|` blocks, not nested mappings"
            )
        match = _KEY.fullmatch(line.strip())
        if not match:
            raise Unreadable(f"{where}:{i + 1}: cannot read `{line.strip()}` as `key: value`")
        key, value = match.group(1), (match.group(2) or "").strip()
        if value in _SCALAR_BLOCK:
            out[key], i = _scalar_block(lines, i + 1, depth)
            continue
        if not value:
            raise Unreadable(
                f"{where}:{i + 1}: `{key}:` has a value this reader cannot take as a scalar"
            )
        out[key] = [value]
        i += 1
    return out


def step_with(lines: list[str], step_lineno: int, where: str) -> dict[str, list[str]]:
    """The `with:` mapping of the step beginning at `step_lineno` (1-based)."""
    start = step_lineno - 1
    if not lines[start].startswith("      - "):
        raise Unreadable(f"{where}:{step_lineno}: not the first line of a step")
    i = start + 1
    while i < len(lines):
        line = lines[i]
        if _is_blank_or_comment(line):
            i += 1
            continue
        if _indent(line) < 8:
            break
        if _indent(line) == 8:
            key, _, inline = line.strip().partition(":")
            if key.strip() == "with":
                if inline.strip():
                    raise Unreadable(f"{where}:{i + 1}: `with:` is not written as a mapping")
                return _mapping(lines, i + 1, 10, where)
        i += 1
    return {}


def matrix_include(lines: list[str], job_lineno: int, where: str) -> list[dict[str, str]]:
    """A job's `strategy: matrix: include:` legs, or [] when it has no matrix.

    Only the `include:` form is read. A matrix written as bare axes multiplies
    out, and a reader that guessed at the product would name artifact
    directories that do not exist -- so it is refused instead.
    """
    i = job_lineno  # the line after the `  <job>:` header
    strategy = matrix = None
    while i < len(lines):
        line = lines[i]
        if _is_blank_or_comment(line):
            i += 1
            continue
        depth = _indent(line)
        if depth <= 2:
            break
        if depth == 4 and line.strip() == "strategy:":
            strategy = i
        elif strategy is not None and depth == 6 and line.strip() == "matrix:":
            matrix = i
        elif matrix is not None and depth == 8:
            if line.strip() != "include:":
                raise Unreadable(
                    f"{where}:{i + 1}: this job's matrix is written as `{line.strip()}`; only "
                    "`include:` is read, because a bare axis product would have to be guessed"
                )
            return _include_entries(lines, i + 1, where)
        i += 1
    return []


def _include_entries(lines: list[str], start: int, where: str) -> list[dict[str, str]]:
    entries: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    item_indent: int | None = None
    i = start
    while i < len(lines):
        line = lines[i]
        if _is_blank_or_comment(line):
            i += 1
            continue
        depth = _indent(line)
        if depth <= 8:
            break
        stripped = line.strip()
        if stripped.startswith("- "):
            if item_indent is None:
                item_indent = depth
            current = {}
            entries.append(current)
            stripped = stripped[2:]
        elif current is None or depth != (item_indent or 0) + 2:
            raise Unreadable(f"{where}:{i + 1}: cannot read `{stripped}` as a matrix entry")
        match = _KEY.fullmatch(stripped)
        if not match or not (match.group(2) or "").strip():
            raise Unreadable(f"{where}:{i + 1}: cannot read `{stripped}` as `key: value`")
        current[match.group(1)] = (match.group(2) or "").strip()
        i += 1
    if not entries:
        raise Unreadable(f"{where}:{start + 1}: an `include:` with no entries")
    return entries


def _resolve(text: str, leg: dict[str, str], where: str, lineno: int) -> str:
    def substitute(match: re.Match[str]) -> str:
        reference = match.group(1)
        if reference.startswith("matrix."):
            key = reference[len("matrix.") :]
            if key not in leg:
                raise Unreadable(
                    f"{where}:{lineno}: `${{{{ {reference} }}}}` names a matrix key this "
                    f"leg does not set ({', '.join(sorted(leg)) or 'no keys'})"
                )
            return leg[key]
        if reference == "github.ref_name":
            return FIXTURE_TAG
        raise Unreadable(
            f"{where}:{lineno}: `${{{{ {reference} }}}}` is an expression this reader "
            "cannot resolve, so it cannot say what this artifact is called"
        )

    return _EXPRESSION.sub(substitute, text)


def uploads(lines: list[str], jobs: dict, steps: list, where: str) -> list[Upload]:
    """Every `actions/upload-artifact` step, one row per matrix leg it runs for.

    The artifact NAME is what ties a step to the directory
    `actions/download-artifact` leaves behind, which is what the assembler
    reads; the `path:` globs are what decides which files are in it.
    """
    found: list[Upload] = []
    for step in steps:
        if not step.uses.startswith(UPLOAD_ACTION):
            continue
        block = step_with(lines, step.lineno, where)
        for required in ("name", "path"):
            if required not in block:
                raise Unreadable(
                    f"{where}:{step.lineno}: the upload step has no `{required}:`; without it "
                    "this file cannot say which downloaded directory it becomes"
                )
        patterns = tuple(entry for entry in block["path"] if entry)
        if not patterns:
            raise Unreadable(f"{where}:{step.lineno}: the upload step's `path:` is empty")
        for pattern in patterns:
            if "/" in pattern:
                raise Unreadable(
                    f"{where}:{step.lineno}: `{pattern}` uploads from a subdirectory; the "
                    "artifact layout that produces is not one this file can reproduce"
                )
        setting = block.get("if-no-files-found", ["<absent>"])[0]
        legs = matrix_include(lines, jobs[step.job].lineno, where) or [{}]
        for leg in legs:
            artifact = _resolve(" ".join(block["name"]), leg, where, step.lineno)
            found.append(
                Upload(step.job, step.lineno, artifact, patterns, setting, leg.get("target", ""))
            )
    if not found:
        raise Unreadable(
            f"{where}: no step uses `{UPLOAD_ACTION}`, so nothing here delivers the files "
            "the release job assembles"
        )
    return found


# ── asking the code what it does ────────────────────────────────────────────


def make_fixture(assembler: Path, root: Path) -> None:
    """The artifacts tree, built by the assembler's own `--make-fixture`.

    ONE BUILDER. This file used to write its own copy of what
    `actions/download-artifact` leaves behind, which meant the sidecars in it
    were a statement of this file's beliefs rather than of the workflow's: F
    could not fail, because the tree carried the files whether or not any
    upload delivered them. The assembler's fixture is the same one its
    self-test and `check_extension_stamp.py --release-rehearsal` run against.
    """
    proc = subprocess.run(
        [
            "bash",
            str(assembler),
            "--make-fixture", str(root),
            "--tag", FIXTURE_TAG,
            "--extension-duckdb-version", FIXTURE_EXT_VERSION,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise Unreadable(
            f"{assembler} --make-fixture exited {proc.returncode}: "
            f"{(proc.stdout + proc.stderr).strip()}"
        )
    if not (root / "artifacts").is_dir():
        raise Unreadable(f"{assembler} --make-fixture wrote no artifacts/ directory")


def deliver(root: Path, ups: list[Upload], drop: tuple[int, str] | None, where: str) -> list[str]:
    """Reduce the fixture to what the upload globs actually deliver.

    A file no `path:` entry matches is a file the artifact does not carry, so
    it is removed here exactly as it would be absent on a tag. `drop` removes
    one declared glob first, which is how F asks whether that glob is
    load-bearing.

    Returns failure strings for globs that match nothing ANYWHERE. Per-leg is
    the wrong question: `finetype-*.zip` matches in the windows leg and nothing
    else, and both are correct.
    """
    by_artifact: dict[str, Upload] = {}
    for upload in ups:
        if upload.artifact in by_artifact:
            raise Unreadable(
                f"{where}: two upload steps both produce an artifact named "
                f"`{upload.artifact}`; the downloaded tree would hold one directory and "
                "this file cannot say whose"
            )
        by_artifact[upload.artifact] = upload

    live: set[tuple[int, str]] = set()
    for directory in sorted((root / "artifacts").iterdir()):
        if not directory.is_dir():
            continue
        upload = by_artifact.get(directory.name)
        if upload is None:
            # An extension build artifact, or a divergence between the fixture
            # and the matrix. Not refused here: `platforms` below owns that
            # comparison and can say which side is short, where this could only
            # say that something did not line up.
            continue
        patterns = [g for g in upload.globs if (upload.lineno, g) != drop]
        for entry in sorted(directory.iterdir()):
            if not entry.is_file():
                continue
            hits = [g for g in patterns if fnmatch.fnmatch(entry.name, g)]
            if not hits:
                entry.unlink()
                continue
            live.update((upload.lineno, g) for g in hits)

    failures: list[str] = []
    for upload in ups:
        for pattern in upload.globs:
            if (upload.lineno, pattern) == drop:
                continue
            if (upload.lineno, pattern) not in live:
                failures.append(
                    f"{where}:{upload.lineno}: the upload step's `{pattern}` matches nothing "
                    f"the `{upload.job}` job produces, so it delivers nothing. "
                    "`if-no-files-found` cannot see this: it fires only when EVERY glob on "
                    "the step comes up empty"
                )
    return sorted(set(failures))


def platforms(ups: list[Upload], pristine: Path, produced: list[str], where: str) -> list[str]:
    """I: both halves of a release cover the same platforms, and every artifact is uploaded.

    Two independent lists that nothing ties together. `ARCH_TARGET_PAIRS` in
    the assembler names the Rust target triple each extension binary is
    published under; the `build` matrix here names the target triple each CLI
    archive is published under; and the assembler's fixture states which
    artifact directories the download step will find. A platform present in one
    and not the others ships half a release -- a 404 for the half nobody
    noticed -- while every step reports success.

    Read off the directories the fixture BUILDS and the filenames the assembler
    WROTE, never off either script's source.
    """
    failures: list[str] = []
    built = {
        entry.name
        for entry in (pristine / "artifacts").iterdir()
        if entry.is_dir() and not fnmatch.fnmatch(entry.name, EXTENSION_ARTIFACT_DIR)
    }
    declared = {upload.artifact for upload in ups}
    if built != declared:
        failures.append(
            f"the artifacts the release job downloads and the artifacts {where} uploads are "
            f"not the same set: {sorted(declared - built) or 'nothing'} uploaded here that the "
            f"assembler's fixture does not build, {sorted(built - declared) or 'nothing'} built "
            "that no upload step delivers"
        )
    extension_targets = {
        Path(name).name[len(f"finetype-{FIXTURE_TAG}-") : -len(".duckdb_extension")]
        for name in produced
        if name.endswith(".duckdb_extension")
    }
    archive_targets = {upload.target for upload in ups if upload.target}
    unpaired_cli = archive_targets - extension_targets
    unpaired_ext = extension_targets - archive_targets
    if unpaired_cli or unpaired_ext:
        failures.append(
            "the CLI archives and the extension binaries would ship for different platforms: "
            f"{sorted(unpaired_cli) or 'nothing'} gets a CLI archive and no extension, "
            f"{sorted(unpaired_ext) or 'nothing'} gets an extension and no CLI archive"
        )
    return failures


def assemble(assembler: Path, root: Path) -> subprocess.CompletedProcess[str]:
    """Run the real assembler over the tree in `root`. The caller reads the code."""
    return subprocess.run(
        [
            "bash",
            str(assembler),
            "--artifacts-dir", "artifacts",
            "--output-dir", "release-assets",
            "--tag", FIXTURE_TAG,
            "--extension-duckdb-version", FIXTURE_EXT_VERSION,
        ],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )


def assembled(root: Path) -> list[str]:
    return sorted(
        str(path.relative_to(root))
        for path in (root / "release-assets").rglob("*")
        if path.is_file()
    )


def min_coverage(coverage_script: Path) -> float:
    """Read MIN_COVERAGE by importing the module, not by scanning its text."""
    spec = importlib.util.spec_from_file_location("_finetype_coverage_gate", coverage_script)
    if spec is None or spec.loader is None:
        raise Unreadable(f"could not import {coverage_script}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    value = getattr(module, "MIN_COVERAGE", None)
    if not isinstance(value, float):
        raise Unreadable(f"{coverage_script} has no float MIN_COVERAGE to compare against")
    return value


def coverage_exit(coverage_script: Path, root: Path, threshold: str, covered: int, total: int) -> int:
    """Exit code of the coverage gate at `threshold` for a fixture of known coverage."""
    labels = [f"label.{n:03d}" for n in range(total)]
    catalogue = [{"x-finetype-label": label, "pattern": "x"} for label in labels[:covered]]
    catalogue_path = root / f"catalogue-{covered}-{total}.json"
    catalogue_path.write_text(json.dumps(catalogue), encoding="utf-8")
    labels_path = root / f"labels-{total}.json"
    labels_path.write_text(json.dumps(labels), encoding="utf-8")
    proc = subprocess.run(
        [
            sys.executable, str(coverage_script),
            "--catalogue", str(catalogue_path),
            "--label-map", str(labels_path),
            "--threshold", threshold,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode


# ── the contract ────────────────────────────────────────────────────────────


def advisory(jobs: dict, steps: list, where: str) -> list[str]:
    """G: nothing in the release workflow may be allowed to fail quietly."""
    failures = []
    for job in sorted(jobs.values(), key=lambda j: j.lineno):
        if job.continue_on_error:
            failures.append(
                f"{where}:{job.lineno}: job `{job.id}` carries "
                f"`continue-on-error: {job.continue_on_error}` -- it reddens and every job "
                "whose `needs:` names it runs anyway, off a release its own checks refused"
            )
    for step in steps:
        if step.continue_on_error:
            failures.append(
                f"{where}:{step.lineno} ({step.name or 'unnamed step'}) carries "
                f"`continue-on-error: {step.continue_on_error}` -- the step reddens and the "
                "job stays green. On the coverage step that is the same release "
                "`--threshold 0.0` produces, by a route no threshold check can see"
            )
    return failures


def formula_order(steps: list, where: str) -> list[str]:
    """H: the formula's asset check runs, unguarded, before the formula is pushed."""
    checks = [s for s in steps if any(FORMULA_CHECK in line for line in s.commands)]
    if len(checks) != 1:
        return [
            f"{where}: expected exactly one step running `{FORMULA_CHECK}`, found "
            f"{len(checks)}. It is the only thing between a formula naming an asset that is "
            "not there and a stranger's `brew install`, and the tap has no CI of its own"
        ]
    check_step = checks[0]
    failures = []
    if check_step.condition:
        failures.append(
            f"{where}:{check_step.lineno}: the formula asset check carries "
            f"`if: {check_step.condition}` -- a skipped step is a green job, so the formula "
            "is pushed unchecked"
        )
    pushes = [
        s
        for s in steps
        if s.job == check_step.job and any(FORMULA_PUSH in line for line in s.commands)
    ]
    if not pushes:
        failures.append(
            f"{where}: job `{check_step.job}` runs `{FORMULA_CHECK}` and never pushes; this "
            "file cannot say the check runs before a publish it cannot find"
        )
    for push in pushes:
        if push.lineno < check_step.lineno:
            failures.append(
                f"{where}:{check_step.lineno}: the formula asset check runs AFTER the push at "
                f"line {push.lineno}. A broken pair then lands in the tap and surfaces at "
                "someone else's `brew install` instead of stopping here"
            )
    return failures


def check(workflow: Path, assembler: Path, coverage_script: Path) -> list[str]:
    """Return failure strings (empty = the contract holds). Raises Unreadable for exit 2."""
    lines = workflow.read_text(encoding="utf-8").splitlines()
    where = workflow.name
    jobs, steps = structure(workflow)
    globs, unmatched_setting = release_step(lines)
    threshold = coverage_threshold(lines)
    ups = uploads(lines, jobs, steps, where)
    failures: list[str] = []

    failures += advisory(jobs, steps, where)
    failures += formula_order(steps, where)
    for upload in {u.lineno: u for u in ups}.values():
        if upload.if_no_files_found != "error":
            failures.append(
                f"{where}:{upload.lineno}: the upload step's if-no-files-found is "
                f"`{upload.if_no_files_found}`, not `error`, so an upload matching nothing at "
                "all is a warning and a green job"
            )

    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        pristine = root / "pristine"
        make_fixture(assembler, pristine)

        # The control tree: everything the fixture builds, reduced to what the
        # declared upload globs deliver. If the assembler refuses THIS, the
        # workflow's own uploads do not deliver what the release needs -- which
        # is the failure `if-no-files-found: error` cannot see.
        work = root / "control"
        shutil.copytree(pristine, work)
        failures += deliver(work, ups, None, where)
        run = assemble(assembler, work)
        if run.returncode != 0:
            failures.append(
                f"the upload paths {where} declares do not deliver what the release needs: "
                f"the assembler exited {run.returncode} over the artifacts they produce -- "
                f"{(run.stdout + run.stderr).strip().splitlines()[-1] if (run.stdout + run.stderr).strip() else 'no output'}"
            )
            return failures
        produced = assembled(work)
        if not produced:
            raise Unreadable(f"{assembler} exited 0 and wrote nothing")
        failures += platforms(ups, pristine, produced, where)

        matched: set[str] = set()
        for pattern in globs:
            hits = {hit for hit in glob.glob(pattern, root_dir=work) if (work / hit).is_file()}
            if not hits:
                failures.append(
                    f"the release step's `{pattern}` matches nothing the assembler produces -- "
                    "with fail_on_unmatched_files: true that is a failed release"
                )
            matched |= hits

        for asset in produced:
            if asset not in matched:
                failures.append(
                    f"the assembler produces `{asset}` and no glob in the release step's "
                    "`files:` list matches it, so the tag would not carry it"
                )

        # E, over what is PUBLISHED rather than what is assembled. The
        # assembler refuses an unpaired asset in the directory it writes; this
        # asks the same question of the subset the `files:` globs carry to the
        # release, which is a different set the moment one of those globs
        # changes.
        published = {Path(name).name for name in matched}
        for name in sorted(published):
            if name.endswith(".sha256"):
                if name[: -len(".sha256")] not in published:
                    failures.append(
                        f"the release publishes `{name}` and not the asset it names, so the "
                        "tag carries a checksum for a file it does not have"
                    )
            elif f"{name}.sha256" not in published:
                failures.append(
                    f"the release publishes `{name}` with no `{name}.sha256` beside it, so a "
                    "downloader has nothing to verify it against"
                )

        if unmatched_setting != "true":
            failures.append(
                f"the release step's fail_on_unmatched_files is `{unmatched_setting}`, not `true`, "
                "so a glob that stops matching is skipped with a warning at release time"
            )

        # F, one glob at a time. Removing an entry from a `path:` list is
        # exactly what `if-no-files-found: error` cannot see, and it is the way
        # the CLI sidecars went unrequired: the union stays non-empty, the
        # upload is silent, and the release path has to be the thing that says
        # no.
        for upload in ups:
            for pattern in upload.globs:
                variant = root / f"drop-{upload.lineno}-{pattern.replace('*', 'STAR').replace('.', '_')}"
                if variant.exists():
                    continue
                shutil.copytree(pristine, variant)
                deliver(variant, ups, (upload.lineno, pattern), where)
                dropped = assemble(assembler, variant)
                if dropped.returncode == 0:
                    failures.append(
                        f"{where}:{upload.lineno}: removing `{pattern}` from the upload step's "
                        f"`path:` list changes nothing the release path refuses -- the "
                        f"assembler still exits 0, over {len(assembled(variant))} files instead "
                        f"of {len(produced)}. Either nothing requires what it delivers, or "
                        "another entry on the same step already delivers it"
                    )

        floor = min_coverage(coverage_script)
        if abs(float(threshold) - floor) > 1e-9:
            failures.append(
                f"the release workflow passes --threshold {threshold} but {COVERAGE_REL}'s "
                f"MIN_COVERAGE is {floor}; move MIN_COVERAGE, where the reasoning for the "
                "number lives, rather than diverging from it here"
            )

        half = coverage_exit(coverage_script, root, threshold, 50, 100)
        if half != 1:
            failures.append(
                f"at --threshold {threshold} a catalogue covering 50 of 100 model labels exits "
                f"{half}, not 1 -- the release-time coverage gate is a no-op"
            )
        full = coverage_exit(coverage_script, root, threshold, 100, 100)
        if full != 0:
            failures.append(
                f"at --threshold {threshold} a catalogue covering every model label exits "
                f"{full}, not 0 -- the release-time coverage gate refuses a correct release"
            )

    return failures


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


def _run_cli(argv: list[str]) -> tuple[int, str]:
    """Run THIS file as a real subprocess and return (exit code, combined output).

    The cases below prove `check` returns the right failure strings. They say
    nothing about whether the PROGRAM acts on them: changing `main`'s
    `return 1` to `return 0` leaves every one of them green while the gate
    prints its complaint to stderr and exits 0 over a release.yml that has
    dropped an asset, and both steps of this gate's CI job pass. The
    `Unreadable -> return 2` path is the more plausible edit of the two, since
    this reader deliberately refuses a wide family of legitimate YAML and
    softening that refusal into a warning is exactly the change someone would
    make. Neither is visible from inside the process, so these cases go
    through argv, `main` and `sys.exit`, and assert the exact code.

    Same pattern, and for the same reason, as `_run_cli` in
    scripts/check_model_coverage.py and scripts/check_schema_catalogue.py.
    """
    proc = subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *argv],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, proc.stdout + proc.stderr


def _exit_code_cases(original: str, tmpdir: Path) -> int:
    """Return the number of exit-code cases that did not behave as stated."""
    failed = 0

    dropped = tmpdir / "dropped.yml"
    dropped.write_text(original.replace("            release-assets/taxonomy-schemas.json\n", "", 1), encoding="utf-8")
    renamed = tmpdir / "renamed.yml"
    renamed.write_text(original.replace(f"uses: {RELEASE_ACTION}@v2", "uses: other/release@v9", 1), encoding="utf-8")
    not_a_workflow = tmpdir / "not-a-workflow.yml"
    not_a_workflow.write_text("name: something else\n", encoding="utf-8")
    intact = tmpdir / "intact.yml"
    intact.write_text(original, encoding="utf-8")

    # Exact codes, never "non-zero": exit 1 means the contract was read and
    # broken, exit 2 means it could not be read. A defect that swaps one for
    # the other refuses the right input for the wrong reason.
    cases: list[tuple[str, list[str], int]] = [
        ("the real release workflow exits 0", ["--workflow", str(intact)], 0),
        (
            "a release step that has dropped the catalogue exits 1",
            ["--workflow", str(dropped)],
            1,
        ),
        (
            "a workflow whose release step cannot be found exits 2, not 1 and not 0",
            ["--workflow", str(renamed)],
            2,
        ),
        (
            "a file that is not the release workflow exits 2, not 0",
            ["--workflow", str(not_a_workflow)],
            2,
        ),
        (
            "a --workflow path that does not exist exits 2, not 0",
            ["--workflow", str(tmpdir / "absent.yml")],
            2,
        ),
        (
            "an --assembler path that does not exist exits 2, not 0",
            ["--workflow", str(intact), "--assembler", str(tmpdir / "absent.sh")],
            2,
        ),
    ]
    for name, argv, expected in cases:
        code, output = _run_cli(argv)
        if code != expected:
            print(f"  MISS {name}: exited {code}")
            last = output.strip().splitlines()
            print(f"      last line of output: {last[-1] if last else '<none>'}")
            failed += 1
        else:
            print(f"  ok   {name}")
    return failed


def self_test() -> int:
    # Mutations are applied to a COPY in a temp directory, never to the tracked
    # workflow. A self-test that edits a file another job may be reading, and
    # relies on a `finally` to put it back, is one interrupted process away
    # from leaving a mutated release workflow on disk.
    workflow = REPO_ROOT / WORKFLOW_REL
    assembler = REPO_ROOT / ASSEMBLER_REL
    coverage_script = REPO_ROOT / COVERAGE_REL
    original = workflow.read_text(encoding="utf-8")

    try:
        control = check(workflow, assembler, coverage_script)
    except Unreadable as exc:
        print(f"  CONTROL FAILED — the real workflow could not be read: {exc}")
        return 1
    if control:
        print("  CONTROL FAILED — the real release workflow does not satisfy the contract:")
        for failure in control:
            print(f"      {failure}")
        return 1
    print("  ok   control: the real release step ships exactly what the assembler assembles")

    # (name, edit applied to release.yml's text, substring the failure must name)
    cases: list[tuple[str, str, str, str]] = [
        (
            "the catalogue is dropped from the release step's files: list",
            "            release-assets/taxonomy-schemas.json\n",
            "",
            "release-assets/taxonomy-schemas.json",
        ),
        (
            "the model manifest is dropped from the release step's files: list",
            "            release-assets/finetype-model.json\n",
            "",
            "release-assets/finetype-model.json",
        ),
        (
            "the extension binaries are dropped from the release step's files: list",
            "            release-assets/finetype-*.duckdb_extension\n",
            "",
            ".duckdb_extension",
        ),
        (
            "the sha256 sidecars are dropped from the release step's files: list",
            "            release-assets/*.sha256\n",
            "",
            ".sha256",
        ),
        (
            "a glob is added that nothing the assembler produces matches",
            "            release-assets/taxonomy-schemas.json\n",
            "            release-assets/taxonomy-schemas.json\n            release-assets/finetype-*.wasm\n",
            "matches nothing",
        ),
        (
            "fail_on_unmatched_files is turned off",
            "fail_on_unmatched_files: true",
            "fail_on_unmatched_files: false",
            "not `true`",
        ),
        (
            "the release-time coverage threshold is loosened to 0.0",
            "--threshold 0.95",
            "--threshold 0.0",
            "no-op",
        ),
        (
            "the release-time coverage threshold is tightened away from MIN_COVERAGE",
            "--threshold 0.95",
            "--threshold 0.99",
            "MIN_COVERAGE",
        ),
        # ── the upload declarations, which nothing here read until this card ──
        (
            "the build upload stops delivering the CLI checksums",
            "            finetype-*.sha256\n",
            "",
            "assembled asset with no .sha256 beside it",
        ),
        (
            "the build upload stops delivering the CLI archives",
            "            finetype-*.tar.gz\n",
            "",
            ".sha256 naming an asset that did not arrive",
        ),
        (
            "the taxonomy upload stops delivering the model manifest's checksum",
            "            finetype-model.json.sha256\n",
            "",
            "did not deliver: finetype-model.json.sha256",
        ),
        (
            "a second glob covers the same files, so neither is load-bearing",
            "            finetype-*.sha256\n",
            "            finetype-*.sha256\n            finetype-*.sha*\n",
            "changes nothing the release path refuses",
        ),
        (
            "an upload downgraded to `if-no-files-found: warn`",
            "          if-no-files-found: error\n          path: |\n            finetype-*.tar.gz\n",
            "          if-no-files-found: warn\n          path: |\n            finetype-*.tar.gz\n",
            "not `error`",
        ),
        # AC4's route: the same silently-skipped gate `--threshold 0.0` gives,
        # reached without touching a threshold. The case names the COVERAGE step
        # rather than any step, because that is the one the threshold rung above
        # is otherwise the only reader of.
        (
            "the release-time coverage gate is made advisory instead of loosened",
            "      - name: AC4 — the catalogue still describes most of the shipped model's labels\n",
            "      - name: AC4 — the catalogue still describes most of the shipped model's labels\n"
            "        continue-on-error: true\n",
            "carries `continue-on-error: true`",
        ),
        (
            "a sixth platform builds a CLI archive with no extension behind it",
            "          - target: x86_64-pc-windows-msvc\n            os: windows-latest\n            archive: zip\n",
            "          - target: x86_64-pc-windows-msvc\n            os: windows-latest\n            archive: zip\n"
            "          - target: riscv64gc-unknown-linux-gnu\n            os: ubuntu-latest\n            archive: tar.gz\n",
            "gets a CLI archive and no extension",
        ),
    ]

    # H's ordering rung takes two whole steps swapping places, which is not a
    # one-line substitution. Built from the file rather than written out here,
    # and the loop below refuses an anchor that does not appear exactly once --
    # so a case that has quietly stopped mutating anything is reported rather
    # than passing on an unmutated tree.
    formula_check = original[
        original.index("      - name: Prove the formula's") : original.index(
            "      - name: Commit and push"
        )
    ]
    formula_push = original[
        original.index("      - name: Commit and push") : original.index("  update-install-site:")
    ]
    cases.append(
        (
            "the formula's asset check is moved after the push that publishes it",
            formula_check + formula_push,
            formula_push + formula_check,
            "runs AFTER the push at line",
        )
    )
    cases.append(
        (
            "the formula's asset check is turned off with an `if:` rather than deleted",
            "      - name: Prove the formula's assets exist and match their checksums\n",
            "      - name: Prove the formula's assets exist and match their checksums\n"
            "        if: github.event_name == 'push'\n",
            "a skipped step is a green job",
        )
    )

    # Ambiguity is exit 2, not a verdict: a reader that cannot find the step it
    # is checking must refuse rather than report a clean contract.
    #
    # EACH REFUSAL NAMES ITS REASON, and that is not decoration. Three readers
    # here refuse unless they find EXACTLY ONE of something, and every one of
    # those refusals used to be pinned in one direction only: relaxing
    # `len(hits) != 1` to `len(hits) < 1` left this list entirely green,
    # because the duplicate-step case then refused for a different reason
    # further down -- "the step has no `files:` block" -- and a case that
    # accepts any Unreadable cannot tell the two apart. The second and third
    # cases below are the ones with no zero-direction twin at all: a second
    # `--threshold` is READ as the first while argparse takes the last, so the
    # gate would report the reviewed 0.95 over a release running at 0.0.
    refusals: list[tuple[str, str, str, str]] = [
        (
            "the release step's action is renamed so no step matches",
            f"uses: {RELEASE_ACTION}@v2",
            "uses: some-other-org/some-other-release@v9",
            f"expected exactly one step using `{RELEASE_ACTION}`, found 0",
        ),
        (
            "a second step uses the release action",
            f"      - name: Create release\n        uses: {RELEASE_ACTION}@v2\n",
            f"      - name: Create release again\n        uses: {RELEASE_ACTION}@v2\n"
            f"      - name: Create release\n        uses: {RELEASE_ACTION}@v2\n",
            f"expected exactly one step using `{RELEASE_ACTION}`, found 2",
        ),
        (
            "the coverage invocation loses its --threshold",
            "            --threshold 0.95 \\\n",
            "",
            "expected exactly one `--threshold`",
        ),
        (
            "a second --threshold, which argparse would take instead of the first",
            "            --threshold 0.95 \\\n",
            "            --threshold 0.95 \\\n            --threshold 0.0 \\\n",
            "expected exactly one `--threshold` in the check_model_coverage.py invocation, found 2",
        ),
        (
            "a second invocation of the coverage gate",
            "          python3 scripts/check_model_coverage.py \\\n",
            "          python3 scripts/check_model_coverage.py --catalogue x --label-map y\n"
            "          python3 scripts/check_model_coverage.py \\\n",
            "expected exactly one invocation of check_model_coverage.py",
        ),
    ]

    failed = 0
    with tempfile.TemporaryDirectory() as tmpdir:
        mutated = Path(tmpdir) / "release.yml"

        for name, old, new, expected in cases:
            if original.count(old) != 1:
                print(f"  WRONG {name}: its anchor {old.strip()!r} appears {original.count(old)} times")
                failed += 1
                continue
            mutated.write_text(original.replace(old, new, 1), encoding="utf-8")
            try:
                found = check(mutated, assembler, coverage_script)
            except Unreadable as exc:
                found = [f"unreadable: {exc}"]
            text = "\n".join(found)
            if not found:
                print(f"  MISS {name}: mutation survived")
                failed += 1
            elif expected not in text:
                print(f"  WRONG {name}: caught, but not for the stated reason")
                print(f"      expected to see: {expected}")
                print(f"      got: {text}")
                failed += 1
            else:
                print(f"  ok   {name}")

        for name, old, new, expected in refusals:
            if original.count(old) != 1:
                print(f"  WRONG {name}: its anchor appears {original.count(old)} times")
                failed += 1
                continue
            mutated.write_text(original.replace(old, new, 1), encoding="utf-8")
            try:
                found = check(mutated, assembler, coverage_script)
            except Unreadable as exc:
                if expected in str(exc):
                    print(f"  ok   {name}: refused as unreadable, naming the ambiguity")
                else:
                    print(f"  WRONG {name}: refused, but not for the stated reason")
                    print(f"      expected to see: {expected}")
                    print(f"      got: {exc}")
                    failed += 1
                continue
            print(f"  MISS {name}: returned a verdict {found} instead of refusing")
            failed += 1

        # The verdict cases above end here; what follows drives the same
        # workflows through the process boundary CI actually reads.
        failed += _exit_code_cases(original, Path(tmpdir))

    if failed:
        print(f"\nself-test FAILED: {failed} case(s) not detected correctly")
        return 1
    print(
        f"\nself-test passed: {len(cases)} contract mutations detected, "
        f"{len(refusals)} ambiguities refused, exit codes pinned"
    )
    return 0


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--workflow", type=Path, default=REPO_ROOT / WORKFLOW_REL)
    parser.add_argument("--assembler", type=Path, default=REPO_ROOT / ASSEMBLER_REL)
    parser.add_argument("--coverage-script", type=Path, default=REPO_ROOT / COVERAGE_REL)
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()

    if shutil.which("bash") is None:
        print("error: bash is required to run the assembler", file=sys.stderr)
        return 2

    for path in (args.workflow, args.assembler, args.coverage_script):
        if not path.is_file():
            print(f"error: not a file: {path}", file=sys.stderr)
            return 2

    try:
        failures = check(args.workflow, args.assembler, args.coverage_script)
    except Unreadable as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    if failures:
        print(f"the release step in {args.workflow} does not ship what the assembler assembles:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(f"release asset contract holds: {args.workflow} ships every file {args.assembler} assembles")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
