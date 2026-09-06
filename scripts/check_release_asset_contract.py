#!/usr/bin/env python3
"""Gate the third link: what `release.yml` uploads against what the assembler makes.

There are three places a release asset can be lost, and closing two of them is
what makes the third dangerous.

    1. the artifacts tree     -- guarded by assemble-release-assets.sh, exits
                                 1/3/4 when a platform, an arch or the
                                 catalogue is absent
    2. the artifact uploads   -- guarded by `if-no-files-found: error`
    3. the release step's
       `files:` list          -- guarded by NOTHING before this file existed

Deleting `release-assets/taxonomy-schemas.json` from that list leaves the
assembler at exit 0 and every other gate green, and the tag publishes five
extension binaries and no type source. `fail_on_unmatched_files: true` does not
help: it fires on an entry that is PRESENT and matches nothing, and says
nothing about an entry that is gone. The same shape applies to the
`--threshold` the release workflow hands `check_model_coverage.py` -- set it to
0.0 and the coverage gate is a no-op at release time with nothing in this
repository noticing.

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

HOW, AND WHY IT IS NOT A SOURCE SCAN
    A and B RUN the assembler against a synthetic artifacts tree and glob the
    real directory it writes; D IMPORTS `check_model_coverage` and runs it as a
    subprocess. Nothing here parses the assembler's source or reads a number
    out of it, because the defect this whole card kept producing is a check
    that reads the shape of some code instead of asking what the code does.
    The one thing that IS read as text is `release.yml`, which is unavoidable:
    it is the declaration under test. It is read line-structured and stdlib
    only, in the same style and for the same reason as
    `.github/scripts/gate-self-tests.py`'s `scan_workflow` -- every shape it
    cannot read exactly is REFUSED with exit 2 rather than guessed at.

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
import glob
import hashlib
import importlib.util
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
WORKFLOW_REL = ".github/workflows/release.yml"
ASSEMBLER_REL = ".github/scripts/assemble-release-assets.sh"
COVERAGE_REL = "scripts/check_model_coverage.py"

RELEASE_ACTION = "softprops/action-gh-release"
FIXTURE_TAG = "vCONTRACT"
FIXTURE_EXT_VERSION = "vTEST"
FIXTURE_ARCHS = ("linux_amd64", "linux_arm64", "osx_amd64", "osx_arm64", "windows_amd64")
FIXTURE_TARGETS = (
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
)


class Unreadable(Exception):
    """The workflow could not be read unambiguously. Exit 2, never a verdict."""


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


# ── asking the code what it does ────────────────────────────────────────────


def _sha256_sidecar(path: Path) -> None:
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    path.with_name(path.name + ".sha256").write_text(f"{digest}  {path.name}\n", encoding="utf-8")


def build_artifacts_tree(root: Path) -> None:
    """A synthetic `artifacts/` standing in for what download-artifact produces."""
    artifacts = root / "artifacts"
    for target in FIXTURE_TARGETS:
        directory = artifacts / f"finetype-{target}"
        directory.mkdir(parents=True, exist_ok=True)
        suffix = "zip" if "windows" in target else "tar.gz"
        archive = directory / f"finetype-{FIXTURE_TAG}-{target}.{suffix}"
        archive.write_text(f"cli-binary-for-{target}\n", encoding="utf-8")
        _sha256_sidecar(archive)
    for arch in FIXTURE_ARCHS:
        directory = artifacts / f"finetype-{FIXTURE_EXT_VERSION}-extension-{arch}"
        directory.mkdir(parents=True, exist_ok=True)
        (directory / "finetype.duckdb_extension").write_text(f"EXTENSION-{arch}\n", encoding="utf-8")
    catalogue_dir = artifacts / "finetype-taxonomy-catalogue"
    catalogue_dir.mkdir(parents=True, exist_ok=True)
    catalogue = catalogue_dir / "taxonomy-schemas.json"
    catalogue.write_text('[{"x-finetype-label":"a.b.c","pattern":"x"}]\n', encoding="utf-8")
    _sha256_sidecar(catalogue)
    manifest = catalogue_dir / "finetype-model.json"
    manifest.write_text('{"model":"m2v8m-s43"}\n', encoding="utf-8")
    _sha256_sidecar(manifest)


def assemble(assembler: Path, root: Path) -> list[str]:
    """Run the real assembler and return the paths it wrote, relative to `root`."""
    build_artifacts_tree(root)
    proc = subprocess.run(
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
    if proc.returncode != 0:
        raise Unreadable(
            f"{assembler} exited {proc.returncode} on a complete synthetic tree, so there is "
            f"nothing to compare the release step against: {(proc.stdout + proc.stderr).strip()}"
        )
    produced = sorted(
        str(path.relative_to(root)) for path in (root / "release-assets").rglob("*") if path.is_file()
    )
    if not produced:
        raise Unreadable(f"{assembler} exited 0 and wrote nothing")
    return produced


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


def check(workflow: Path, assembler: Path, coverage_script: Path) -> list[str]:
    """Return failure strings (empty = the contract holds). Raises Unreadable for exit 2."""
    lines = workflow.read_text(encoding="utf-8").splitlines()
    globs, unmatched_setting = release_step(lines)
    threshold = coverage_threshold(lines)
    failures: list[str] = []

    with tempfile.TemporaryDirectory() as tmpdir:
        root = Path(tmpdir)
        produced = assemble(assembler, root)

        matched: set[str] = set()
        for pattern in globs:
            hits = {hit for hit in glob.glob(pattern, root_dir=root) if (root / hit).is_file()}
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

        if unmatched_setting != "true":
            failures.append(
                f"the release step's fail_on_unmatched_files is `{unmatched_setting}`, not `true`, "
                "so a glob that stops matching is skipped with a warning at release time"
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
    ]

    # Ambiguity is exit 2, not a verdict: a reader that cannot find the step it
    # is checking must refuse rather than report a clean contract.
    refusals: list[tuple[str, str, str]] = [
        (
            "the release step's action is renamed so no step matches",
            f"uses: {RELEASE_ACTION}@v2",
            "uses: some-other-org/some-other-release@v9",
        ),
        (
            "a second step uses the release action",
            f"      - name: Create release\n        uses: {RELEASE_ACTION}@v2\n",
            f"      - name: Create release again\n        uses: {RELEASE_ACTION}@v2\n"
            f"      - name: Create release\n        uses: {RELEASE_ACTION}@v2\n",
        ),
        (
            "the coverage invocation loses its --threshold",
            "            --threshold 0.95 \\\n",
            "",
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

        for name, old, new in refusals:
            if original.count(old) != 1:
                print(f"  WRONG {name}: its anchor appears {original.count(old)} times")
                failed += 1
                continue
            mutated.write_text(original.replace(old, new, 1), encoding="utf-8")
            try:
                found = check(mutated, assembler, coverage_script)
            except Unreadable:
                print(f"  ok   {name}: refused as unreadable rather than scored")
                continue
            print(f"  MISS {name}: returned a verdict {found} instead of refusing")
            failed += 1

    if failed:
        print(f"\nself-test FAILED: {failed} case(s) not detected correctly")
        return 1
    print(f"\nself-test passed: {len(cases)} contract mutations detected, {len(refusals)} ambiguities refused")
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
