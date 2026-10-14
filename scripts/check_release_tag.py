#!/usr/bin/env python3
"""A pushed tag is a release only if it names this tree's version and its changelog entry.

WHAT IT REFUSES
    release.yml publishes whatever tag starts it: five platform binaries, the
    DuckDB extension, crates.io and a Homebrew formula. Its first job,
    `tag-guard`, runs this script, and every other job waits on that one, so a
    refusal here stops the run before anything is built or published.

        exit 3  the tag is not `v` followed by three dot-separated numbers
        exit 4  the tag's number is not the workspace version in Cargo.toml
        exit 5  CHANGELOG.md has no `## [X.Y.Z]` heading for that number
        exit 6  the heading is there with nothing under it

    On success it writes that heading's section of the changelog as the step
    output `notes` (to the file `--github-output` names, which on a runner is
    $GITHUB_OUTPUT). `tag-guard` exposes it as a job output, and `Create
    release` publishes it as the release body in place of GitHub's generated
    notes. A job output rather than an uploaded artifact, because every upload
    in release.yml is a release asset to scripts/check_release_asset_contract.py,
    and the notes are not one.

    The trigger filter `v[0-9]+.[0-9]+.[0-9]+` on release.yml and
    MainDistributionPipeline.yml already keeps a tag like `v19-train-state`
    from starting either workflow (one started a failed release in June 2026).
    Exit 3 is the second line behind that filter, for the day it is loosened.

WHAT ONLY A PUSHED TAG RUNS, AND WHAT STANDS IN FOR IT
    GitHub's tag matching cannot be run anywhere but GitHub. `--wiring` reads
    the two workflows instead: both tag filters are exactly the pattern above,
    the guard step is unconditional and refuses rather than warns, every other
    release job needs it, and the release body is the notes file. That is a
    reading of the workflow text, and it is the honest limit of what a pull
    request can check about a trigger.

    `--release-rehearsal` goes further for the guard itself: it executes the
    `tag-guard` job's own `run:` lines, read out of release.yml, against a copy
    of this tree's Cargo.toml and CHANGELOG.md, once with the tag a release of
    this tree would carry and once for each refusal. Nothing is published.

        exit 7  --wiring found the release path not wired as described above
        exit 8  --release-rehearsal found the guard step not doing its job
        exit 2  the check could not run (unreadable file, unparseable shape)
        exit 1  --self-test: a case did not get the outcome it requires

USAGE
    scripts/check_release_tag.py --tag TAG [--github-output FILE] [--root DIR]
    scripts/check_release_tag.py --wiring [--root DIR]
    scripts/check_release_tag.py --release-rehearsal [--root DIR]
    scripts/check_release_tag.py --self-test
"""

from __future__ import annotations

import argparse
import importlib.util
import os
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ROUTER_REL = ".github/scripts/gate-self-tests.py"
RELEASE_WORKFLOW = ".github/workflows/release.yml"
EXTENSION_WORKFLOW = ".github/workflows/MainDistributionPipeline.yml"
SCRIPT_REL = "scripts/check_release_tag.py"
MANIFEST_REL = "Cargo.toml"
CHANGELOG_REL = "CHANGELOG.md"

GUARD_JOB = "tag-guard"
GUARD_STEP_ID = "guard"
PUBLISH_ACTION = "softprops/action-gh-release"
TAG_FILTER = "v[0-9]+.[0-9]+.[0-9]+"
# The notes travel as step output -> job output -> the publish step's `body:`.
GUARD_ARGS = '--tag "${GITHUB_REF_NAME}" --github-output "${GITHUB_OUTPUT}"'
NOTES_OUTPUT = "notes"
NOTES_JOB_OUTPUT = f"${{{{ steps.{GUARD_STEP_ID}.outputs.{NOTES_OUTPUT} }}}}"
NOTES_BODY = f"${{{{ needs.{GUARD_JOB}.outputs.{NOTES_OUTPUT} }}}}"

TAG_RE = re.compile(r"^v([0-9]+\.[0-9]+\.[0-9]+)$")
HEADING_RE = re.compile(r"^## \[([^\]]+)\]")

EXIT_CANNOT_RUN = 2
EXIT_TAG_FORM = 3
EXIT_MANIFEST = 4
EXIT_NO_HEADING = 5
EXIT_EMPTY_SECTION = 6
EXIT_WIRING = 7
EXIT_REHEARSAL = 8


class Refused(Exception):
    """A finding. Carries the exit code that says which check refused."""

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code


# ── the tag ─────────────────────────────────────────────────────────────────


def workspace_version(manifest: Path) -> str:
    """`version` under `[workspace.package]` in the root Cargo.toml."""
    try:
        text = manifest.read_text(encoding="utf-8")
    except OSError as exc:
        raise Refused(EXIT_CANNOT_RUN, f"{manifest}: {exc}") from None
    section = ""
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            section = stripped
            continue
        if section == "[workspace.package]":
            match = re.match(r'^version\s*=\s*"([^"]+)"\s*$', stripped)
            if match:
                return match.group(1)
    raise Refused(EXIT_CANNOT_RUN, f"{manifest}: no `version` under [workspace.package]")


def changelog_section(changelog: Path, version: str) -> str:
    """The text under `## [version]`, up to the next `## ` heading, trimmed."""
    try:
        lines = changelog.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise Refused(EXIT_CANNOT_RUN, f"{changelog}: {exc}") from None
    start = None
    for i, line in enumerate(lines):
        match = HEADING_RE.match(line)
        if match and match.group(1) == version:
            start = i + 1
            break
    if start is None:
        raise Refused(
            EXIT_NO_HEADING,
            f"{changelog.name} has no `## [{version}]` heading. Write the release's "
            "entry, or move the [Unreleased] items under that heading, before tagging.",
        )
    end = next((j for j in range(start, len(lines)) if lines[j].startswith("## ")), len(lines))
    body = "\n".join(lines[start:end]).strip()
    if not body:
        raise Refused(
            EXIT_EMPTY_SECTION,
            f"{changelog.name}'s `## [{version}]` heading has nothing under it; a "
            "release body would be empty.",
        )
    return body + "\n"


def write_output(github_output: Path, body: str) -> None:
    """Append `notes` to a GitHub step-output file, multi-line form.

    The delimiter is random so no line of the changelog can end the value early.
    """
    delimiter = f"NOTES_{secrets.token_hex(16)}"
    with github_output.open("a", encoding="utf-8") as fh:
        fh.write(f"{NOTES_OUTPUT}<<{delimiter}\n{body}{delimiter}\n")


def read_output(github_output: Path) -> str | None:
    """The `notes` value back out of a step-output file, or None if absent."""
    if not github_output.is_file():
        return None
    match = re.search(
        rf"^{NOTES_OUTPUT}<<(\S+)\n(.*?)^\1$", github_output.read_text(encoding="utf-8"), re.S | re.M
    )
    return match.group(2) if match else None


def check_tag(root: Path, tag: str, github_output: Path | None) -> None:
    match = TAG_RE.match(tag)
    if not match:
        raise Refused(
            EXIT_TAG_FORM,
            f"`{tag}` is not a release tag: a release tag is `v` followed by three "
            "dot-separated numbers, like v0.6.62.",
        )
    version = match.group(1)
    manifest_version = workspace_version(root / MANIFEST_REL)
    if version != manifest_version:
        raise Refused(
            EXIT_MANIFEST,
            f"the tag says {version} and {MANIFEST_REL} says {manifest_version}. "
            "Tag the commit that bumps the version, or bump it first.",
        )
    body = changelog_section(root / CHANGELOG_REL, version)
    if github_output is not None:
        write_output(github_output, body)
    print(f"{tag}: {MANIFEST_REL} and {CHANGELOG_REL} agree; release notes are that section.")


# ── the wiring ──────────────────────────────────────────────────────────────


def _workflow_reader():
    """gate-self-tests.py's workflow reader, imported rather than reimplemented."""
    path = ROOT / ROUTER_REL
    spec = importlib.util.spec_from_file_location("gate_self_tests", path)
    if spec is None or spec.loader is None:
        raise Refused(EXIT_CANNOT_RUN, f"{ROUTER_REL} could not be imported")
    module = importlib.util.module_from_spec(spec)
    # Registered before it executes: `@dataclass` resolves field types through
    # sys.modules[cls.__module__].
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)
    except Exception as exc:  # noqa: BLE001 -- any import failure is "cannot run"
        raise Refused(EXIT_CANNOT_RUN, f"{ROUTER_REL} could not be imported: {exc}") from None
    return module


def _scan(root: Path, rel: str):
    reader = _workflow_reader()
    try:
        return reader.scan_workflow(root, rel)
    except Exception as exc:  # noqa: BLE001 -- the reader's Fatal is "cannot run"
        raise Refused(EXIT_CANNOT_RUN, f"{rel} could not be read: {exc}") from None


def push_tag_filters(root: Path, rel: str) -> list[str]:
    """The `on: push: tags:` entries of a workflow, read line by line.

    The workflow reader starts at `jobs:`, so the trigger block is read here.
    Any shape other than an inline `[...]` list or a block `- item` list under
    `tags:` is refused rather than guessed at.
    """
    try:
        lines = (root / rel).read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise Refused(EXIT_CANNOT_RUN, f"{rel}: {exc}") from None
    i, n = 0, len(lines)
    while i < n and lines[i].rstrip() not in ("on:", '"on":', "'on':"):
        i += 1
    if i == n:
        raise Refused(EXIT_CANNOT_RUN, f"{rel}: no top-level `on:` block")
    i += 1
    in_push = False
    while i < n and (not lines[i].strip() or lines[i].startswith(" ") or lines[i].startswith("#")):
        line = lines[i]
        if re.match(r"^  [A-Za-z_]+:", line):
            in_push = line.strip() == "push:"
        elif in_push and re.match(r"^    tags:", line):
            inline = line.split(":", 1)[1].strip()
            if inline.startswith("["):
                if not inline.endswith("]"):
                    raise Refused(EXIT_CANNOT_RUN, f"{rel}:{i + 1}: cannot read `tags: {inline}`")
                items = [s.strip() for s in inline[1:-1].split(",") if s.strip()]
                return [s.strip("'\"") for s in items]
            if inline:
                raise Refused(EXIT_CANNOT_RUN, f"{rel}:{i + 1}: cannot read `tags: {inline}`")
            items = []
            j = i + 1
            while j < n and re.match(r"^      - ", lines[j]):
                items.append(lines[j].split("- ", 1)[1].strip().strip("'\""))
                j += 1
            return items
        i += 1
    return []


def _publish_with(root: Path, publish_lineno: int) -> dict[str, str]:
    """The single-line `with:` keys of the publish step (block values read as '|')."""
    lines = (root / RELEASE_WORKFLOW).read_text(encoding="utf-8").splitlines()
    out: dict[str, str] = {}
    in_with = False
    for line in lines[publish_lineno:]:
        if line.startswith("      - ") or re.match(r"^ {0,4}[A-Za-z0-9_-]+:", line):
            break
        if re.match(r"^        with:\s*$", line):
            in_with = True
            continue
        if re.match(r"^        [A-Za-z0-9_-]+:", line):
            in_with = False
        if in_with:
            match = re.match(r"^          ([A-Za-z0-9_-]+):\s*(.*)$", line)
            if match:
                out[match.group(1)] = match.group(2).strip()
    return out


def _logical(step) -> list[str]:
    """A step's `run:` as command lines: comments dropped, continuations joined."""
    out: list[str] = []
    pending = ""
    for raw in step.commands:
        line = raw.strip()
        if not pending and (not line or line.startswith("#")):
            continue
        if line.endswith("\\"):
            pending += line[:-1].rstrip() + " "
            continue
        out.append(pending + line)
        pending = ""
    if pending:
        out.append(pending.rstrip())
    return out


def _calls_guard(line: str) -> bool:
    return bool(re.match(rf"^(python3 )?{re.escape(SCRIPT_REL)} ", line))


def check_wiring(root: Path) -> None:
    findings: list[str] = []

    for rel in (RELEASE_WORKFLOW, EXTENSION_WORKFLOW):
        filters = push_tag_filters(root, rel)
        if filters != [TAG_FILTER]:
            findings.append(
                f"{rel}: push tag filter is {filters or 'absent'}, not ['{TAG_FILTER}']; "
                "any other filter lets a tag that is not a version start the workflow."
            )

    jobs, steps = _scan(root, RELEASE_WORKFLOW)
    guard = jobs.get(GUARD_JOB)
    if guard is None:
        findings.append(f"{RELEASE_WORKFLOW}: no `{GUARD_JOB}` job")
    else:
        if guard.condition:
            findings.append(f"{RELEASE_WORKFLOW}: job `{GUARD_JOB}` carries `if: {guard.condition}`")
        if guard.continue_on_error:
            findings.append(f"{RELEASE_WORKFLOW}: job `{GUARD_JOB}` carries continue-on-error")
        calls = [
            (s, line)
            for s in steps
            if s.job == GUARD_JOB
            for line in _logical(s)
            if _calls_guard(line)
        ]
        if len(calls) != 1:
            findings.append(
                f"{RELEASE_WORKFLOW}: job `{GUARD_JOB}` calls {SCRIPT_REL} "
                f"{len(calls)} time(s), not once"
            )
        else:
            step, line = calls[0]
            if step.condition:
                findings.append(f"{RELEASE_WORKFLOW}:{step.lineno}: the guard step carries `if:`")
            if step.continue_on_error:
                findings.append(f"{RELEASE_WORKFLOW}:{step.lineno}: the guard step carries continue-on-error")
            if not line.endswith(GUARD_ARGS):
                findings.append(
                    f"{RELEASE_WORKFLOW}:{step.lineno}: the guard is not called with {GUARD_ARGS}"
                )
            if step.step_id != GUARD_STEP_ID:
                findings.append(
                    f"{RELEASE_WORKFLOW}:{step.lineno}: the guard step's id is "
                    f"{step.step_id!r}, not {GUARD_STEP_ID!r}"
                )
        if guard.outputs.get(NOTES_OUTPUT) != NOTES_JOB_OUTPUT:
            findings.append(
                f"{RELEASE_WORKFLOW}: job `{GUARD_JOB}` does not output "
                f"`{NOTES_OUTPUT}: {NOTES_JOB_OUTPUT}`"
            )

    def needs_guard(job_id: str, seen: frozenset = frozenset()) -> bool:
        job = jobs.get(job_id)
        if job is None or job_id in seen:
            return False
        if GUARD_JOB in job.needs:
            return True
        return any(needs_guard(n, seen | {job_id}) for n in job.needs)

    for job_id in jobs:
        if job_id != GUARD_JOB and not needs_guard(job_id):
            findings.append(
                f"{RELEASE_WORKFLOW}: job `{job_id}` does not wait on `{GUARD_JOB}`, so it "
                "runs for a tag the guard refuses"
            )

    publishing = [s for s in steps if s.uses.startswith(PUBLISH_ACTION)]
    if len(publishing) != 1:
        findings.append(f"{RELEASE_WORKFLOW}: {len(publishing)} step(s) use {PUBLISH_ACTION}, not one")
    else:
        publish = publishing[0]
        if GUARD_JOB not in jobs[publish.job].needs:
            findings.append(
                f"{RELEASE_WORKFLOW}: job `{publish.job}` does not list `{GUARD_JOB}` in "
                "`needs:`, so it cannot read the guard's notes"
            )
        with_keys = _publish_with(root, publish.lineno)
        if with_keys.get("body") != NOTES_BODY:
            findings.append(
                f"{RELEASE_WORKFLOW}:{publish.lineno}: the release body is not "
                f"`body: {NOTES_BODY}` (found {with_keys.get('body')!r})"
            )
        if with_keys.get("generate_release_notes", "false") != "false":
            findings.append(
                f"{RELEASE_WORKFLOW}:{publish.lineno}: generate_release_notes is on, "
                "which adds GitHub's notes to the changelog section"
            )
        if "body_path" in with_keys:
            findings.append(f"{RELEASE_WORKFLOW}:{publish.lineno}: `body_path:` overrides `body:`")

    if findings:
        raise Refused(EXIT_WIRING, "\n".join(findings))
    print("release wiring: tag filters, guard job and release body are as required.")


# ── the rehearsal ───────────────────────────────────────────────────────────


def _guard_lines(root: Path) -> list[str]:
    _, steps = _scan(root, RELEASE_WORKFLOW)
    lines = [line for s in steps if s.job == GUARD_JOB and s.commands for line in _logical(s)]
    if not lines:
        raise Refused(EXIT_REHEARSAL, f"{RELEASE_WORKFLOW}: job `{GUARD_JOB}` runs nothing")
    return lines


def _run_guard(lines: list[str], workdir: Path, tag: str) -> int:
    env = {**os.environ, "GITHUB_REF_NAME": tag, "GITHUB_OUTPUT": str(workdir / "github_output")}
    script = "set -euo pipefail\n" + "\n".join(lines) + "\n"
    proc = subprocess.run(
        ["bash", "-c", script], cwd=workdir, env=env, capture_output=True, text=True
    )
    return proc.returncode


def check_rehearsal(root: Path) -> None:
    lines = _guard_lines(root)
    version = workspace_version(root / MANIFEST_REL)
    major, minor, patch = (int(p) for p in version.split("."))
    findings: list[str] = []

    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)

        def tree(name: str, drop_heading: bool = False) -> Path:
            work = base / name
            (work / "scripts").mkdir(parents=True)
            # The guard as it stands in the tree under check, so a rehearsal of
            # a mutated workflow runs that tree's script, not this one.
            shutil.copy(root / SCRIPT_REL, work / SCRIPT_REL)
            shutil.copy(root / MANIFEST_REL, work / MANIFEST_REL)
            changelog = (root / CHANGELOG_REL).read_text(encoding="utf-8")
            if drop_heading:
                changelog = re.sub(
                    rf"^## \[{re.escape(version)}\].*$", "## [renamed]", changelog, flags=re.M
                )
            (work / CHANGELOG_REL).write_text(changelog, encoding="utf-8")
            return work

        good = tree("good")
        rc = _run_guard(lines, good, f"v{version}")
        notes = read_output(good / "github_output")
        if rc != 0:
            findings.append(f"v{version} (this tree's version) was refused with exit {rc}")
        elif notes is None:
            findings.append(f"v{version} passed but the step set no `{NOTES_OUTPUT}` output")
        elif notes != changelog_section(root / CHANGELOG_REL, version):
            findings.append(f"the `{NOTES_OUTPUT}` output is not the `## [{version}]` section")

        for label, tag, work, want in (
            ("a tag that is not a version", "v19-train-state", tree("form"), EXIT_TAG_FORM),
            ("a version the manifest does not carry", f"v{major}.{minor}.{patch + 1}", tree("manifest"), EXIT_MANIFEST),
            ("a version with no changelog heading", f"v{version}", tree("heading", drop_heading=True), EXIT_NO_HEADING),
        ):
            rc = _run_guard(lines, work, tag)
            if rc != want:
                findings.append(f"{label} ({tag}): the guard step exited {rc}, not {want}")
            if read_output(work / "github_output") is not None:
                findings.append(f"{label} ({tag}): the guard step set `{NOTES_OUTPUT}` anyway")

    if findings:
        raise Refused(EXIT_REHEARSAL, "\n".join(f"{RELEASE_WORKFLOW} `{GUARD_JOB}`: {f}" for f in findings))
    print(f"release rehearsal: `{GUARD_JOB}` passes v{version} and refuses the three bad tags.")


# ── self-test ───────────────────────────────────────────────────────────────


def self_test() -> int:
    passed = 0
    failed: list[str] = []

    def expect(label: str, fn, want: int) -> None:
        nonlocal passed
        try:
            fn()
            got = 0
        except Refused as exc:
            got = exc.code
        except Exception as exc:  # noqa: BLE001 -- a case that raises is a failure
            failed.append(f"{label}: raised {type(exc).__name__}: {exc}")
            return
        if got == want:
            passed += 1
        else:
            failed.append(f"{label}: exit {got}, wanted {want}")

    manifest = '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "1.2.3"\n\n[package]\nversion = "9.9.9"\n'
    changelog = (
        "# Changelog\n\n## [Unreleased]\n\n## [1.2.3] - 2026-10-14\n\n### Fixed\n\n- A thing.\n\n"
        "## [1.2.2] - 2026-10-01\n\n- Older.\n"
    )

    with tempfile.TemporaryDirectory() as tmp:
        base = Path(tmp)

        def tag_tree(name: str, cargo: str = manifest, log: str = changelog) -> Path:
            root = base / name
            root.mkdir()
            (root / MANIFEST_REL).write_text(cargo, encoding="utf-8")
            (root / CHANGELOG_REL).write_text(log, encoding="utf-8")
            return root

        good = tag_tree("good")
        output = base / "github_output"
        output.write_text("earlier=1\n", encoding="utf-8")
        expect("the manifest's version with a changelog entry", lambda: check_tag(good, "v1.2.3", output), 0)
        if read_output(output) == "### Fixed\n\n- A thing.\n" and output.read_text().startswith("earlier=1\n"):
            passed += 1
        else:
            failed.append("notes: not exactly the 1.2.3 section, appended to the output file")
        for bad in ("v19-train-state", "v1.2", "1.2.3", "v1.2.3-rc1", "v1.2.3.4", "vX.Y.Z"):
            expect(f"tag form {bad}", lambda bad=bad: check_tag(good, bad, None), EXIT_TAG_FORM)
        expect("a version the manifest does not carry", lambda: check_tag(good, "v1.2.4", None), EXIT_MANIFEST)
        expect("[package] version is not the workspace's", lambda: check_tag(good, "v9.9.9", None), EXIT_MANIFEST)
        no_heading = tag_tree("no-heading", cargo=manifest.replace("1.2.3", "1.2.4"))
        expect("no changelog heading", lambda: check_tag(no_heading, "v1.2.4", None), EXIT_NO_HEADING)
        empty = tag_tree("empty", log="## [1.2.3] - 2026-10-14\n\n## [1.2.2]\n\n- Older.\n")
        expect("an empty changelog section", lambda: check_tag(empty, "v1.2.3", None), EXIT_EMPTY_SECTION)
        last = tag_tree("last", log="## [1.2.3]\n\n- Only entry.\n")
        expect("the last section, undated", lambda: check_tag(last, "v1.2.3", None), 0)
        no_manifest = base / "no-manifest"
        no_manifest.mkdir()
        expect("no Cargo.toml", lambda: check_tag(no_manifest, "v1.2.3", None), EXIT_CANNOT_RUN)

        # The wiring and the rehearsal, against a copy of this tree's workflows
        # with one defect each. The unmutated copy must pass both, or a
        # mutation reddening proves nothing.
        def wired_tree(name: str, edit=None) -> Path:
            root = base / name
            for rel in (RELEASE_WORKFLOW, EXTENSION_WORKFLOW, ROUTER_REL, SCRIPT_REL, MANIFEST_REL, CHANGELOG_REL):
                (root / rel).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(ROOT / rel, root / rel)
            if edit is not None:
                rel, old, new = edit
                text = (root / rel).read_text(encoding="utf-8")
                if old not in text:
                    raise AssertionError(f"mutation `{name}` found nothing to change in {rel}")
                (root / rel).write_text(text.replace(old, new, 1), encoding="utf-8")
            return root

        control = wired_tree("control")
        expect("wiring: this tree", lambda: check_wiring(control), 0)
        expect("rehearsal: this tree", lambda: check_rehearsal(control), 0)

        guard_call = f"python3 {SCRIPT_REL} {GUARD_ARGS}"
        wiring_cases = (
            ("release.yml filter loosened to v*", (RELEASE_WORKFLOW, f'tags: ["{TAG_FILTER}"]', 'tags: ["v*"]')),
            ("extension filter loosened to v*", (EXTENSION_WORKFLOW, f"tags: ['{TAG_FILTER}']", "tags: ['v*']")),
            ("a second, looser tag filter", (RELEASE_WORKFLOW, f'tags: ["{TAG_FILTER}"]', f'tags: ["{TAG_FILTER}", "v*"]')),
            ("build no longer waits on the guard", (RELEASE_WORKFLOW, f"    needs: {GUARD_JOB}\n    runs-on: ${{{{ matrix.os }}}}", "    runs-on: ${{ matrix.os }}")),
            ("the guard step made conditional", (RELEASE_WORKFLOW, f"        run: {guard_call}", f"        if: false\n        run: {guard_call}")),
            ("the guard step made advisory", (RELEASE_WORKFLOW, f"        run: {guard_call}", f"        continue-on-error: true\n        run: {guard_call}")),
            ("the guard call commented out", (RELEASE_WORKFLOW, f"        run: {guard_call}", f"        run: echo skipped # {guard_call}")),
            ("generated notes switched back on", (RELEASE_WORKFLOW, f"          body: {NOTES_BODY}", "          generate_release_notes: true")),
            ("the release job stops needing the guard", (RELEASE_WORKFLOW, f"    needs: [{GUARD_JOB}, build,", "    needs: [build,")),
            ("the guard's job output renamed", (RELEASE_WORKFLOW, f"      {NOTES_OUTPUT}: {NOTES_JOB_OUTPUT}", f"      body: {NOTES_JOB_OUTPUT}")),
        )
        for label, edit in wiring_cases:
            try:
                tree = wired_tree(label.replace(" ", "-"), edit)
            except AssertionError as exc:
                failed.append(f"wiring: {label}: {exc}")
                continue
            expect(f"wiring: {label}", lambda tree=tree: check_wiring(tree), EXIT_WIRING)

        rehearsal_cases = (
            ("the guard call replaced by `true`", (RELEASE_WORKFLOW, f"        run: {guard_call}", f"        run: true # {guard_call}")),
            ("the guard's notes sent nowhere", (RELEASE_WORKFLOW, '--github-output "${GITHUB_OUTPUT}"', "--github-output /dev/null")),
            ("the guard's manifest check removed", (SCRIPT_REL, "    if version != manifest_version:", "    if False:")),
        )
        for label, edit in rehearsal_cases:
            tree = wired_tree("r-" + label.replace(" ", "-").replace("`", ""), edit)
            expect(f"rehearsal: {label}", lambda tree=tree: check_rehearsal(tree), EXIT_REHEARSAL)

    print(f"self-test: {passed} passed, {len(failed)} failed")
    for f in failed:
        print(f"  FAILED {f}", file=sys.stderr)
    return 1 if failed else 0


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--tag")
    mode.add_argument("--wiring", action="store_true")
    mode.add_argument("--release-rehearsal", action="store_true")
    mode.add_argument("--self-test", action="store_true")
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--root", type=Path, default=None)
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    # --tag runs from the working directory, as the release job runs it; the
    # other two read the repository this script lives in unless told otherwise.
    root = args.root or (Path.cwd() if args.tag is not None else ROOT)
    try:
        if args.tag is not None:
            check_tag(root, args.tag, args.github_output)
        elif args.wiring:
            check_wiring(root)
        else:
            check_rehearsal(root)
    except Refused as exc:
        print(f"error: {exc}", file=sys.stderr)
        return exc.code
    return 0


if __name__ == "__main__":
    sys.exit(main())
