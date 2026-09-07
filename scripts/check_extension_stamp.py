#!/usr/bin/env python3
"""Gate the version stamp a built DuckDB extension carries in its metadata trailer.

WHY THIS EXISTS
    The stamp is the only version a `.duckdb_extension` carries in a form a
    machine can read BEFORE loading it. DuckDB surfaces it as
    `duckdb_extensions().extension_version`; a packager reads it straight out of
    the file's last 512 bytes. Anyone who has to tell two builds apart reads
    that field and nothing else.

    Every extension this project published between 2026-06-28, when
    configure/extension_version.txt was committed, and the commit that removed
    it carries `0.6.23`, whatever tag it was built from -- and the file was
    still there when this was written. The cause is that one committed file.
    The build contract

    writes the version through this rule, in
    extension-ci-tools/makefiles/c_api_extensions/base.Makefile:

        configure/extension_version.txt:
        	@ $(VERSION_COMMAND)

    A file target with no prerequisites is never remade once it exists, so a
    tree that ARRIVES with the file already in it never runs VERSION_COMMAND at
    all. The file was tracked in git, so a checkout arrived with it, so the
    autodetection did not run in CI, and the committed literal was stamped onto
    each artefact built after it was committed. Measured on the local build of
    2026-06-04: trailer offset 128 reads `0.6.23` while the tree was at 0.6.58.


    The fix is to stop tracking the file and let the build write it. That fix
    is invisible -- a file that regenerates silently is the same shape of trust
    that produced the defect -- so this gate reads the ARTEFACT'S BYTES and
    refuses a release whose stamp disagrees with the tag it was built from.

WHAT EACH MODE DECIDES, and each is a different defect with a different fix,
which is why each has its own exit code rather than a shared 1:

    --artifact FILE... --tag TAG    (exit 1, exit 3)
        The trailer version of each named artefact equals TAG. Read off the
        file's bytes, never from configure/extension_version.txt: the file is
        what the build wrote, the bytes are what the user receives, and the
        whole defect above is the two disagreeing. A leading `v` is stripped
        from both sides and nothing else is normalised -- see NORMALISATION.

    --artifact FILE... --expect-commit SHA    (exit 1, exit 3)
        The same question asked of a build from an UNTAGGED ref, where the
        stamp must be that commit's abbreviated sha. This is the one that runs
        on a pull request, so the autodetection is observed on every build
        rather than a handful of times a year -- and the committed literal
        fails it, because `0.6.23` abbreviates no commit.

    --artifact FILE --load          (exit 7)
        The version the extension reports AT RUN TIME and the version in its
        trailer, read off ONE artefact in ONE DuckDB session. `ft_version()`
        comes from CARGO_PKG_VERSION compiled into the cdylib;
        `duckdb_extensions().extension_version` is DuckDB's own reading of the
        trailer. Two independent sources, one file, one LOAD.

        THE LOAD IS WHAT ESTABLISHES IDENTITY, and the byte reading above
        cannot: any valid shared library stamped by extension-ci-tools'
        append_extension_metadata script passes a trailer read. Most of the
        refusing is DuckDB's own -- it dlsyms an init symbol derived from the
        FILE NAME, so a re-stamped stranger renamed finetype.duckdb_extension
        does not load at all. The `ft_version()` name check in this file is the
        backstop for what DuckDB would accept, a library that does export that
        symbol, and the self-test reaches it by substituting the session's
        reading because no file on disk produces it. This mode also requires
        DuckDB to report the extension as loaded and NOT installed -- an
        installed community build could otherwise answer for the file on disk.

        THE FILE IS PRESENTED TO DUCKDB AS finetype.duckdb_extension whatever
        the caller called it, because that same filename rule refuses a
        legitimate artefact under a release's name: a published asset is
        `finetype-v0.6.58-<target>.duckdb_extension` and the symbol derived from
        it is `finetype-v0_init_c_api`. See duckdb_readings.

        The artefact must be one this machine can load, and it is refused BY
        NAME when it is not: see require_native.


    --untracked-version-file        (exit 4, exit 5)
        Nothing under configure/ is tracked in git, and
        configure/extension_version.txt is ignored. Both halves, because they
        fail differently: a tracked file disables the rule that writes it, and
        an un-ignored one comes back on the next `git add`.

    --release-wiring                (exit 8)
        This gate is invoked at each point a wrong stamp can escape, unguarded,
        and before the bytes are published. Every mode above refuses something;
        NOTHING MAKES THEM RUN except the two release workflows, and those are
        YAML that no check in this repository read. One `continue-on-error:
        true` turns the refusal into a warning and leaves this file's own
        --self-test green. Read as structure -- jobs, guards, steps, order --
        through the reader in .github/scripts/gate-self-tests.py, so there is
        one parser of a workflow in this repository rather than two that drift.

    --release-rehearsal --artifact F  (exit 9)
        The release job's OWN assemble and stamp commands, executed against a
        tree of real assembled asset names, on a pull request. --release-wiring
        reads those steps as text and reported the release path sound while the
        step that loads an artefact named it under a name DuckDB refuses to
        load -- so on every tag it would have exited 2 and nothing would ever
        have been published. This mode runs the `run:` text of each of those
        steps, with GITHUB_REF_NAME and the job's `env:` supplied, over the
        output of the job's own call to assemble-release-assets.sh. See "the
        rehearsal" below for what it substitutes and what it does not.

    --regenerates-version-file      (exit 6)

        The Makefile forces the version file to be regenerated even when one is
        already on disk. Untracking the file fixes CI, where every checkout is
        fresh; it does not fix a developer's tree, where a file written by the
        previous build persists and upstream's rule still has no prerequisites.
        Asked of make itself with `make -n`, so what is observed is make's
        rebuild decision rather than a string in the Makefile.

NORMALISATION
    A single leading `v` is stripped from both sides of every version
    comparison, and nothing else is. A git tag carries one and a DuckDB
    extension version stamp does not have to; upstream's autodetection writes
    `git tag --points-at HEAD` verbatim, so a tagged build stamps `v0.6.58`
    while the local `make build-extension` path stamps Cargo.toml's `0.6.58`.
    Both are the same release and both must pass. The consumer that decides
    this is brightfield's bundle check, which strips a leading `v` from both
    sides and compares the rest exactly; a tolerant comparison beyond that
    would let `0.6.23` under `v0.6.58` through, which is the entire defect.

UNTAGGED BUILDS
    Upstream's autodetection falls back to `git --no-pager log -1 --format=%h`
    -- a short commit sha -- when no tag points at HEAD. That fallback is kept
    unchanged: a pull-request build stamps its sha, which is a true statement
    about what was built and is what the community CI does for every other
    extension. `--tag` is never passed on an untagged build, and it takes both
    workflows that pass it to say so. release.yml triggers on `tags: ['v*']`, so
    every build it checks has one. MainDistributionPipeline.yml runs on pushes
    to main, on pull requests and on dispatch as well, so the trigger settles
    nothing there -- what does is the `GITHUB_REF_TYPE` test in its stamp step,
    which passes `--tag` only when the ref IS a tag and `--expect-commit`
    otherwise. A tagged build cannot
    pass with a fallback stamp: a short sha does not equal the tag, so the
    comparison refuses and says which of the two it is looking at.

USAGE
    scripts/check_extension_stamp.py --artifact F [--artifact F ...] --tag TAG
    scripts/check_extension_stamp.py --artifact F [--artifact F ...] --expect-commit SHA
    scripts/check_extension_stamp.py --artifact F --load [--tag TAG]
    scripts/check_extension_stamp.py --untracked-version-file
    scripts/check_extension_stamp.py --regenerates-version-file
    scripts/check_extension_stamp.py --release-wiring
    scripts/check_extension_stamp.py --self-test --artifact F


EXIT CODES
    0  clean
    1  a trailer version disagrees with the tag it was built from
    2  the tool could not run: bad usage, an unreadable file, no duckdb, no git
    3  an artefact carries no readable DuckDB metadata trailer
    4  a generated file under configure/ is tracked in git
    5  configure/extension_version.txt is not ignored, so it can be re-added
    6  the Makefile does not force the version file to be regenerated
    7  a loaded extension's run-time version disagrees with its own trailer
    8  the release path no longer invokes this gate where it has to
    9  the release path invokes this gate and does not survive being run
"""


from __future__ import annotations

import argparse
import importlib.util
import itertools
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


# The DuckDB metadata trailer: the last 512 bytes are eight 32-byte NUL-padded
# ASCII fields written LAST-FIRST, then 256 bytes of signature space. Field 1
# (the magic, "4") therefore lands at offset 224, and below it sit the platform
# (192), the DuckDB version (160), the extension version (128) and the ABI type
# (96). extension-ci-tools' append_extension_metadata script writes them and
# brightfield_engine::semantic::read_stamp reads the same offsets.
TRAILER_LEN = 512
FIELD_LEN = 32
OFF_ABI = 96
OFF_EXTENSION_VERSION = 128
OFF_DUCKDB_VERSION = 160
OFF_PLATFORM = 192
OFF_MAGIC = 224
MAGIC = "4"

# The version file, and the directory it lives in. Everything under configure/
# is produced by the build -- a venv, the platform name, the version -- so the
# rule is the directory rather than the one filename: a rename of the file
# would otherwise reintroduce the defect under a name this gate does not know.
CONFIGURE_DIR = "configure"
VERSION_FILE = "configure/extension_version.txt"

# What `make -n extension_version` must be seen to decide. EXTENSION_VERSION is
# set to a probe string on purpose: with it set, upstream's VERSION_COMMAND
# becomes a plain `echo` into the version file, so the probe appears in make's
# dry-run output exactly when make has decided to run the rule. It changes what
# the rule WOULD do and not whether make runs it, which is the only thing being
# observed here, and it removes the venv from a check that has no other need of
# one.
MAKE_PROBE = "STAMP-PROBE-9d41"

EXIT_OK = 0
EXIT_STAMP_DISAGREES = 1
EXIT_CANNOT_RUN = 2
EXIT_NO_TRAILER = 3
EXIT_TRACKED = 4
EXIT_NOT_IGNORED = 5
EXIT_NO_REGENERATION = 6
EXIT_RUNTIME_DISAGREES = 7
EXIT_UNWIRED = 8
EXIT_REHEARSAL = 9

# The name DuckDB has to see. It derives the init symbol it dlsyms from the FILE
# NAME, stemming at the FIRST dot -- so
# `finetype-v0.6.58-x86_64-unknown-linux-gnu.duckdb_extension`, which is exactly
# what a release asset is called, makes it look for `finetype-v0_init_c_api` and
# the LOAD dies. Every artefact this file loads is presented under this name; see
# duckdb_readings.
CANONICAL_NAME = "finetype.duckdb_extension"

SHORT_SHA = re.compile(r"^[0-9a-f]{7,40}$")

# The platforms this project distributes -- the `exclude_archs` input in
# .github/workflows/MainDistributionPipeline.yml is what decides the set. Used
# to pick a platform that is NOT the one under test; nothing here requires the
# list to be complete for that.
DISTRIBUTED_PLATFORMS = (
    "linux_amd64",
    "linux_arm64",
    "osx_amd64",
    "osx_arm64",
    "windows_amd64",
)


class Refused(Exception):
    """A finding. Carries the exit code that says WHICH rung refused."""

    def __init__(self, code: int, message: str) -> None:
        super().__init__(message)
        self.code = code


# ── the trailer ─────────────────────────────────────────────────────────────


def stamp_fields(path: Path) -> dict[str, str]:
    """Every metadata field of a built extension, read off its own bytes."""
    try:
        size = path.stat().st_size
    except OSError as exc:
        raise Refused(EXIT_CANNOT_RUN, f"{path}: {exc}") from None
    if size < TRAILER_LEN:
        raise Refused(
            EXIT_NO_TRAILER,
            f"{path} is {size} bytes -- shorter than the {TRAILER_LEN}-byte metadata "
            "trailer, so it was never stamped and DuckDB will not LOAD it",
        )
    with path.open("rb") as handle:
        handle.seek(-TRAILER_LEN, os.SEEK_END)
        trailer = handle.read(TRAILER_LEN)

    def field(offset: int) -> str:
        raw = trailer[offset : offset + FIELD_LEN]
        return raw.rstrip(b"\x00").decode("utf-8", errors="replace")

    magic = field(OFF_MAGIC)
    if magic != MAGIC:
        raise Refused(
            EXIT_NO_TRAILER,
            f"{path} carries no DuckDB metadata trailer: the magic field at offset "
            f"{OFF_MAGIC} reads {magic!r}, expected {MAGIC!r}. An unstamped shared "
            "library will not LOAD, and a stamp longer than the 32-byte field shifts "
            "every field after it, which looks exactly like this",
        )
    return {
        "abi_type": field(OFF_ABI),
        "extension_version": field(OFF_EXTENSION_VERSION),
        "duckdb_version": field(OFF_DUCKDB_VERSION),
        "platform": field(OFF_PLATFORM),
    }


def core(version: str) -> str:
    """A version with one leading `v` removed. See NORMALISATION in the module doc."""
    return version[1:] if version.startswith("v") else version


def check_against_tag(artifacts: list[Path], tag: str) -> list[str]:
    """AC1/AC3 -- each artefact's trailer version equals the tag it was built from."""
    lines: list[str] = []
    for path in artifacts:
        fields = stamp_fields(path)
        stamped = fields["extension_version"]
        if core(stamped) != core(tag):
            hint = ""
            if "\n" in stamped or "\r" in stamped:
                hint = (
                    "\n    The stamp spans more than one line, which is what "
                    "`git tag --points-at HEAD` writes when SEVERAL tags point at the "
                    "same commit. Exactly one tag may point at a released commit."
                )
            elif len(stamped) == FIELD_LEN and core(tag).endswith(stamped[-8:]):
                hint = (
                    f"\n    The trailer field is {FIELD_LEN} bytes and the tag is longer, "
                    "so the stamp is the tail of it. A tag this long cannot be recorded "
                    "in an artefact at all."
                )
            elif SHORT_SHA.match(stamped):
                hint = (
                    "\n    That is a short commit sha, which is what upstream's "
                    "autodetection writes when NO TAG points at HEAD. This build did "
                    "not see its own tag."
                )
            raise Refused(
                EXIT_STAMP_DISAGREES,
                f"{path} is stamped version {stamped!r} and was built from tag "
                f"{tag!r}.{hint}\n"
                "    Nothing downstream can tell this artefact from any other build: "
                "the trailer is the only version it carries before it is loaded.",
            )
        lines.append(
            f"   {path.name}: {stamped}  ({fields['abi_type']}, {fields['platform']}, "
            f"DuckDB floor {fields['duckdb_version']})"
        )
    return lines


def check_against_commit(artifacts: list[Path], sha: str) -> list[str]:
    """Each artefact's trailer version is the abbreviated sha of the commit built.

    This is what a build from an UNTAGGED ref must produce, and it is the only
    reading available on a pull request -- which makes it the one that turns the
    autodetection from a thing believed into a thing observed, on every run
    rather than a few times a year. The committed literal that caused all of
    this fails here: `0.6.23` is not an abbreviation of any commit.

    The abbreviation length is not fixed. Git scales it with the size of the
    repository, so the stamp is required to be a hex PREFIX of the full sha
    rather than a slice of a chosen length -- with a floor, because a one
    character prefix is also a prefix.
    """
    sha = sha.strip().lower()
    if not SHORT_SHA.match(sha) or len(sha) < 40:
        raise Refused(
            EXIT_CANNOT_RUN, f"--expect-commit needs a full 40-character sha, got {sha!r}"
        )
    lines: list[str] = []
    for path in artifacts:
        stamped = stamp_fields(path)["extension_version"]
        if not (
            len(stamped) >= 7 and SHORT_SHA.match(stamped) and sha.startswith(stamped)
        ):
            raise Refused(
                EXIT_STAMP_DISAGREES,
                f"{path} is stamped {stamped!r} and was built from commit {sha} with no "
                "tag on it, so its stamp must be that commit's abbreviated sha.\n"
                "    A version number here means the build did not derive the stamp at "
                "all -- it read a file that was already in the tree.",
            )
        lines.append(f"   {path.name}: {stamped}, an abbreviation of {sha}")
    return lines


# ── the load ────────────────────────────────────────────────────────────────


def host_platform() -> str:
    """DuckDB's own name for the platform this machine can load extensions for.

    Asked of DuckDB rather than derived from `platform.machine()` and
    `sys.platform`, because the string being compared against is the one DuckDB
    wrote into the trailer and the one it refuses a LOAD over. A derivation of
    ours agreeing with it is a coincidence maintained by hand.
    """
    if shutil.which("duckdb") is None:
        raise Refused(
            EXIT_CANNOT_RUN,
            "duckdb is not on PATH -- this mode loads the artefact and asks it",
        )
    proc = subprocess.run(
        ["duckdb", "-no-init", "-json", "-c", "SELECT platform FROM pragma_platform();"],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"duckdb exited {proc.returncode} reporting its platform:\n{proc.stderr.strip()}",
        )
    try:
        rows = json.loads(proc.stdout.strip() or "[]")
        return str(rows[0]["platform"])
    except (json.JSONDecodeError, IndexError, KeyError) as exc:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"could not read a platform out of `pragma_platform()` ({exc}):\n"
            f"{proc.stdout[:400]}",
        ) from None


def require_native(path: Path, fields: dict[str, str]) -> str:
    """Refuse, by name, an artefact this machine cannot load. Returns the platform.

    DuckDB refuses a foreign binary itself, but it does so from inside the LOAD
    with a message about platforms wearing the exit code this file uses for
    "the tool could not run". Asked here, BEFORE the load, the refusal says
    which platform the artefact is for and which one this is -- and it is a
    refusal rather than a skip, because a mode that quietly passed when it
    could not load the file would read exactly like a mode that checked it.
    """
    here = host_platform()
    built_for = fields["platform"]
    if built_for != here:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"{path} was built for platform {built_for!r} and this machine loads "
            f"{here!r}, so nothing here can make it answer for itself.\n"
            "    This is not a verdict on its version: the byte reading "
            "(--tag / --expect-commit) works on any platform's artefact and is what "
            "the four non-native binaries of a release get. Run this mode on a "
            f"{built_for} runner.",
        )
    return here


LOAD_SQL = """

SELECT ft_version() AS runtime,
       (SELECT extension_version FROM duckdb_extensions()
         WHERE extension_name = 'finetype') AS trailer,
       (SELECT loaded FROM duckdb_extensions()
         WHERE extension_name = 'finetype') AS loaded,
       (SELECT installed FROM duckdb_extensions()
         WHERE extension_name = 'finetype') AS installed;
"""


def duckdb_readings(extension: Path) -> dict:
    """Load one artefact and read its run-time and trailer versions in one session.

    THE BYTES ARE LOADED UNDER `finetype.duckdb_extension`, ALWAYS, whatever the
    caller named the file. DuckDB derives the init symbol it dlsyms from the file
    name, stemming at the FIRST dot, so an artefact called
    `finetype-v0.6.58-x86_64-unknown-linux-gnu.duckdb_extension` -- the name
    .github/scripts/assemble-release-assets.sh gives every published extension
    binary -- makes it look for `finetype-v0_init_c_api` and the LOAD fails with
    `did not contain function`, exit 2, before any version is read.

    That rule cost this repository a release path that could not publish: the
    step in release.yml that loads one asset named it under its assembled name,
    so every tag would have failed there and `Create release` would never have
    run. Making the CHECKER immune is what closes it for every call site rather
    than for the one that was noticed -- a caller cannot hit the rule if the name
    it passes is never the name DuckDB sees.

    A hard link where the filesystem allows one, a copy where it does not; either
    way DuckDB reads the same bytes, and stamp_fields has already read the
    trailer off the original.
    """
    if shutil.which("duckdb") is None:
        raise Refused(
            EXIT_CANNOT_RUN,
            "duckdb is not on PATH -- this mode loads the artefact and asks it",
        )
    with tempfile.TemporaryDirectory(prefix="stamp-load-") as tmp:
        # Separate directories: DuckDB's extension_directory is where an
        # INSTALLED extension would live, and check_loaded refuses a reading
        # that reports finetype as installed. The file being loaded is kept out
        # of it so that refusal keeps meaning what it says.
        ext_dir = Path(tmp) / "extensions"
        ext_dir.mkdir()
        loadable = Path(tmp) / "load" / CANONICAL_NAME
        loadable.parent.mkdir()
        try:
            os.link(extension.resolve(), loadable)
        except OSError:
            shutil.copyfile(extension.resolve(), loadable)
        prelude = f"SET extension_directory='{ext_dir}';\nLOAD '{loadable}';\n"
        proc = subprocess.run(
            ["duckdb", "-unsigned", "-no-init", "-json", "-c", prelude + LOAD_SQL],
            capture_output=True,
            text=True,
            env={**os.environ, "DUCKDB_EXTENSION_DIRECTORY": str(ext_dir)},
            check=False,
        )
    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"duckdb exited {proc.returncode} loading {extension} (presented to it as "
            f"{CANONICAL_NAME}):\n{proc.stderr.strip()}",
        )
    try:
        rows = json.loads(proc.stdout.strip() or "[]")
    except json.JSONDecodeError as exc:
        raise Refused(
            EXIT_CANNOT_RUN, f"duckdb did not return JSON ({exc}):\n{proc.stdout[:400]}"
        ) from None
    if not rows:
        raise Refused(EXIT_CANNOT_RUN, f"duckdb returned no rows for {extension}")
    return rows[0]


def check_loaded(
    extension: Path,
    tag: str | None,
    readings: Callable[[Path], dict] = duckdb_readings,
) -> list[str]:
    """AC5 -- run-time version and trailer version, off one artefact, in one session.

    `readings` is the DuckDB session, and it is a parameter for one reason: two
    of the rungs below answer a reading that no file on disk can produce here.
    A library that is not this extension does not reach them -- DuckDB dlsyms an
    init symbol derived from the FILE NAME and exits first -- so the self-test
    substitutes the reading and leaves everything else real. Production passes
    nothing and gets the real session.
    """
    fields = stamp_fields(extension)
    require_native(extension, fields)
    row = readings(extension)

    if not row.get("loaded"):
        raise Refused(
            EXIT_CANNOT_RUN, f"DuckDB did not report {extension} as loaded"
        )
    if row.get("installed"):
        raise Refused(
            EXIT_CANNOT_RUN,
            "DuckDB reports finetype as INSTALLED as well as loaded, so this reading "
            "may come from a community build on the runner rather than from "
            f"{extension}",
        )

    runtime_raw = str(row["runtime"])
    # `ft_version()` returns `finetype <version>`; the name half is what proves
    # the loaded artefact is THIS extension rather than any stamped library.
    # DuckDB's dlsym of the filename-derived init symbol refuses most strangers
    # before this, which is why the self-test drives this rung with a
    # substituted reading rather than with a file.

    if not runtime_raw.startswith("finetype "):
        raise Refused(
            EXIT_RUNTIME_DISAGREES,
            f"ft_version() returned {runtime_raw!r}, which does not name finetype -- "
            "the loaded artefact is not this extension",
        )
    runtime = runtime_raw[len("finetype ") :].strip()
    duckdbs_reading = str(row["trailer"])

    # DuckDB's own reading of the trailer against this file's reading of the same
    # bytes. It costs nothing and it is the only thing that would notice the
    # offsets at the top of this file drifting from what the stamper writes.
    if duckdbs_reading != fields["extension_version"]:
        raise Refused(
            EXIT_RUNTIME_DISAGREES,
            f"DuckDB reads the trailer of {extension} as {duckdbs_reading!r} and this "
            f"gate reads it as {fields['extension_version']!r} off the same bytes -- "
            "the field offsets in this file no longer match what the stamper writes",
        )

    if core(runtime) != core(duckdbs_reading):
        raise Refused(
            EXIT_RUNTIME_DISAGREES,
            f"{extension} reports {runtime!r} through ft_version() at run time and "
            f"carries {duckdbs_reading!r} in its trailer. One artefact cannot be two "
            "releases: the trailer comes from the build contract and the run-time "
            "string is compiled in from Cargo.toml, so these disagree when the tag "
            "and Cargo.toml disagree.",
        )

    if tag is not None and core(runtime) != core(tag):
        raise Refused(
            EXIT_STAMP_DISAGREES,
            f"{extension} reports {runtime!r} at run time and was built from tag "
            f"{tag!r}. Cargo.toml's version is what ft_version() returns, so the tag "
            "was cut against a tree whose version says something else.",
        )

    return [
        f"   {extension.name}: ft_version() says {runtime}, the trailer says "
        f"{duckdbs_reading}, read off one artefact in one LOAD"
    ]


# ── git and make ────────────────────────────────────────────────────────────


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    try:
        return subprocess.run(
            ["git", "-C", str(root), *args], capture_output=True, text=True, check=False
        )
    except OSError as exc:
        raise Refused(EXIT_CANNOT_RUN, f"git could not be run: {exc}") from None


def check_untracked(root: Path) -> list[str]:
    """AC2 -- nothing under configure/ is tracked, and the version file is ignored."""
    listed = _git(root, "ls-files", "--", CONFIGURE_DIR)
    if listed.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"git ls-files failed in {root}: {listed.stderr.strip()}",
        )
    tracked = [line for line in listed.stdout.splitlines() if line.strip()]
    if tracked:
        raise Refused(
            EXIT_TRACKED,
            "these generated files are tracked in git:\n"
            + "".join(f"      {name}\n" for name in tracked)
            + f"    The build writes {CONFIGURE_DIR}/ on every configure step. A "
            "tracked copy arrives with every checkout, and the rule that would "
            "rewrite it has no prerequisites, so it never runs and every artefact is "
            "stamped with whatever was committed.\n"
            f"    Remove it with: git rm --cached <path>",
        )

    ignored = _git(root, "check-ignore", "-q", "--", VERSION_FILE)
    if ignored.returncode != 0:
        raise Refused(
            EXIT_NOT_IGNORED,
            f"{VERSION_FILE} is not ignored, so the next `git add` puts it back and "
            "the defect returns without anyone choosing it.\n"
            "    Add it to .gitignore.",
        )
    return [f"   nothing under {CONFIGURE_DIR}/ is tracked, and {VERSION_FILE} is ignored"]


def check_regenerates(root: Path, makefile: Path | None = None) -> list[str]:
    """AC3's local half -- make remakes the version file even when one exists."""
    if shutil.which("make") is None:
        raise Refused(EXIT_CANNOT_RUN, "make is not on PATH")
    target = root / VERSION_FILE

    # THE FILE HAS TO BE THERE OR THIS CHECK CANNOT FAIL. The defect is that make
    # leaves an EXISTING file alone; against a tree where the file is absent make
    # remakes it either way and this mode would pass on the broken Makefile too.
    created = False
    if not target.exists():
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("STALE-FROM-A-PREVIOUS-BUILD\n", encoding="utf-8")
        created = True
    try:
        args = ["make", "-n", "extension_version", f"EXTENSION_VERSION={MAKE_PROBE}"]
        if makefile is not None:
            args[1:1] = ["-f", str(makefile)]
        proc = subprocess.run(
            args, cwd=str(root), capture_output=True, text=True, check=False
        )
    finally:
        if created:
            target.unlink(missing_ok=True)

    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"`make -n extension_version` exited {proc.returncode}:\n"
            f"{(proc.stderr or proc.stdout).strip()}",
        )
    if MAKE_PROBE not in proc.stdout:
        raise Refused(
            EXIT_NO_REGENERATION,
            "make would NOT rewrite the version file when one is already on disk. "
            f"`make -n extension_version` said:\n      {proc.stdout.strip() or '(nothing)'}\n"
            "    The build contract declares the file as a target with no "
            "prerequisites, so make treats an existing one as up to date. A second "
            "tag built in the same tree then keeps the FIRST tag's version.\n"
            "    Give the file a phony prerequisite so it is never up to date.",
        )
    return ["   make remakes the version file even when one is already on disk"]


# ── the wiring ──────────────────────────────────────────────────────────────
#
# THE THREE PLACES THIS GATE ACTUALLY REFUSES A RELEASE ARE WORKFLOW YAML, and
# before this mode existed nothing read them. Measured then: with
# `continue-on-error: true` on the release workflow's stamp step, every check in
# this repository exited 0 -- including this file's own --self-test, which never
# opened the workflow. What a user got under that one line: the step went
# red-but-ignored, the release attached the mis-stamped binaries, and the
# consumer refused the bundle it had just downloaded. Deleting both stamp steps,
# or the whole stamp job from the distribution pipeline, bought the same
# nothing. Each of those is a case in the self-test below now.
#
# WHAT THIS MODE STILL CANNOT SEE is whether the steps it finds would RUN. It
# reads text: names, guards, order. --release-rehearsal executes them; the two
# are separate modes because they fail differently and a green reading of a step
# that cannot run is the defect that cost this repository its release path.
#
# So the declarations are read as STRUCTURE -- jobs, their guards, their steps
# and the order of them -- through the reader in .github/scripts/gate-self-tests.py
# rather than a second parser of this file's own. Every job here is identified
# by what it CALLS or RUNS rather than by its name: a renamed job is a rename,
# and a deleted refusal is the defect.

ROUTER_REL = ".github/scripts/gate-self-tests.py"
RELEASE_WORKFLOW = ".github/workflows/release.yml"
DISTRIBUTION_WORKFLOW = ".github/workflows/MainDistributionPipeline.yml"

# The step that publishes. Everything this gate does in release.yml has to
# happen before it, because after it the bytes are on a permanent public URL.
PUBLISH_ACTION = "softprops/action-gh-release"
# The reusable workflow that builds the five extension binaries. The job that
# checks them has to depend on the job that makes them, or it checks nothing.
EXTENSION_BUILD_CALL = "duckdb/extension-ci-tools/.github/workflows/_extension_distribution.yml"
GATE_INVOCATION = "check_extension_stamp.py"
# The script that names the release assets. The rehearsal below runs the release
# job's own call to it, so the names its stamp steps read are the names a
# release actually carries.
ASSEMBLER = "assemble-release-assets.sh"
# What an installation of the DuckDB CLI fetches. The step that LOADs an asset
# needs `duckdb` on PATH, and the runner has none: an install step that goes
# missing turns the load into "duckdb is not on PATH" on a tag and nowhere else.
DUCKDB_CLI_MARK = "duckdb_cli-"
# How it is fetched. The marker alone is a MENTION, and both places that read it
# were reading mentions: a comment naming `duckdb_cli-` satisfied the rung below
# that requires an install above the load, and exempted a step from the
# rehearsal's enumeration. Measured: replacing the install step's `curl` with a
# comment naming the marker and an `echo` left every check in this repository
# green while a tag would exit 2 at the load, and a step whose entire `run:` was
# `# same pin as the duckdb_cli- install above` / `exit 3` was never run by
# anything. Same reason `_invocation_re` exists for the gate itself.
DUCKDB_CLI_FETCH = "curl"

# Every mode this gate offers. `main` builds the parser from this, and
# check_release_wiring requires each one to be invoked by a step in one of the
# workflows below -- a mode CI never runs is a check that rots unheard, and
# reading it back off the parser's own declaration is what keeps the two from
# being separate lists that drift.
MODES = (
    "--load",
    "--untracked-version-file",
    "--regenerates-version-file",
    "--release-wiring",
    "--release-rehearsal",
    "--self-test",
)
CI_WORKFLOW = ".github/workflows/ci.yml"


def _logical_lines(step) -> list[str]:
    """A step's `run:` as shell command lines: comments dropped, continuations joined.

    THE POINT IS THAT A MENTION IS NOT AN INVOCATION. The first cut of this file
    matched a step whose `run:` merely CONTAINED `check_extension_stamp.py`, so
    commenting the call out, or replacing it with
    `echo "check_extension_stamp.py"`, left the step counted and every check in
    this repository green. A `#` line and a quoted argument are the two shapes
    that reads as a call and is not one; dropping comments here and requiring
    command position in _invokes is what separates them.
    """
    out: list[str] = []
    pending = ""
    for raw in step.commands:
        line = raw.strip()
        if not pending and line.startswith("#"):
            continue
        if line.endswith("\\"):
            pending += line[:-1].rstrip() + " "
            continue
        out.append(pending + line)
        pending = ""
    if pending:
        out.append(pending)
    return out


def _invocation_re(script: str) -> re.Pattern[str]:
    """`script` run as a command: at the start of a line or after a shell separator.

    Optionally through an interpreter and behind leading `VAR=value` assignments,
    which is how a call is written when it is not written bare. Anything else --
    inside a quoted string, after `echo`, behind a `#` -- is a mention.
    """
    return re.compile(
        r"(?:^|[;&|]|\bthen\b|\bdo\b|\belse\b)[ \t]*"
        r"(?:[A-Za-z_][A-Za-z0-9_]*=\S*[ \t]+)*"
        r"(?:(?:python3?|bash|sh)[ \t]+)?"
        r"(?:\./)?(?:[\w./-]*/)?" + re.escape(script) + r"(?=[ \t]|$)"
    )


GATE_CALL = _invocation_re(GATE_INVOCATION)
ASSEMBLER_CALL = _invocation_re(ASSEMBLER)
CLI_FETCH_CALL = _invocation_re(DUCKDB_CLI_FETCH)


def _installs_duckdb_cli(step) -> bool:
    """This step FETCHES the DuckDB CLI, rather than mentioning it.

    An invocation of `curl` on a logical line that also names the release asset
    -- so a comment, an echoed string, or prose about the pin is none of it.
    Read on `_logical_lines` for the same reason every other reading in this
    file is: the two questions asked of this predicate are "would the load have
    a duckdb to run" and "is this step the one the rehearsal cannot execute",
    and a comment answers neither.
    """
    return any(
        CLI_FETCH_CALL.search(line) and DUCKDB_CLI_MARK in line for line in _logical_lines(step)
    )


def _workflow_reader():
    """gate-self-tests.py's workflow reader, imported rather than reimplemented."""
    path = ROOT / ROUTER_REL
    spec = importlib.util.spec_from_file_location("gate_self_tests", path)
    if spec is None or spec.loader is None:
        raise Refused(EXIT_CANNOT_RUN, f"{ROUTER_REL} could not be imported")
    module = importlib.util.module_from_spec(spec)
    # Registered BEFORE it executes: `@dataclass` resolves its field types
    # through `sys.modules[cls.__module__]`, which is None for a module that is
    # only half imported.
    sys.modules[spec.name] = module
    try:
        spec.loader.exec_module(module)

    except Exception as exc:  # noqa: BLE001 -- any import failure is "cannot run"
        raise Refused(EXIT_CANNOT_RUN, f"{ROUTER_REL} could not be imported: {exc}") from None
    return module


def _calls(step, pattern: re.Pattern[str]) -> list[str]:
    """The logical lines of this step that INVOKE the command `pattern` names."""
    return [line for line in _logical_lines(step) if pattern.search(line)]


def _gate_steps(steps: list, job_id: str) -> list:
    return [step for step in steps if step.job == job_id and _calls(step, GATE_CALL)]


def _runs(step, needle: str) -> bool:
    """`needle` appears as an argument of a real invocation of the gate.

    Read off the invocation and not off the step, because `--load` in a comment
    or in an echoed string would otherwise decide which of the two release steps
    this is -- and the two are counted separately precisely so that deleting
    either one is named.
    """
    return any(needle in line for line in _calls(step, GATE_CALL))


def _upstream_guards(rel: str, builders: list, verb: str) -> list[str]:
    """A job that is SKIPPED skips everything downstream of it, silently.

    `needs:` is checked elsewhere; this asks whether the job at the other end of
    that edge can be turned off. `if: false` on the build job leaves the stamp
    job present, correctly wired and never run, and GitHub reports the whole
    workflow green -- the skip propagates down the `needs:` chain and a skipped
    required check satisfies branch protection. `continue-on-error` at job level
    is the other half: the build fails, its dependants run, and the stamp step
    reads an artefacts directory that is empty or stale.
    """
    problems: list[str] = []
    for builder in builders:
        if builder.condition:
            problems.append(
                f"{rel}: the job that builds the extension binaries (`{builder.id}`) carries "
                f"`if: {builder.condition}` -- a skipped job skips everything that "
                f"`needs:` it, so the job that {verb} the stamp is skipped with it and the "
                "workflow reports green"
            )
        if builder.continue_on_error:
            problems.append(
                f"{rel}: the job that builds the extension binaries (`{builder.id}`) carries "
                f"`continue-on-error: {builder.continue_on_error}` -- the build fails, its "
                f"dependants run anyway, and the job that {verb} the stamp reads an "
                "artefacts directory that is empty or left over"
            )
    return problems


def _unguarded(step, where: str, problems: list[str]) -> None:
    """The three ways a step that is present still refuses nothing."""
    if step.condition:
        problems.append(
            f"{where} carries `if: {step.condition}` -- a skipped step is a GREEN job, "
            "so a condition here is a switch that turns the refusal off without "
            "deleting it"
        )
    if step.continue_on_error:
        problems.append(
            f"{where} carries `continue-on-error: {step.continue_on_error}` -- the step "
            "reddens, the job stays green, and the release publishes the artefacts the "
            "step refused"
        )
    if step.shell != "bash":
        problems.append(
            f"{where} runs under shell {step.shell or '<the runner default>'!r}, not "
            "`bash`. The default shell on a Linux runner has no `pipefail`, so a "
            "refusal piped anywhere reports the exit code of the last command instead"
        )


def check_release_wiring(root: Path) -> list[str]:
    """This gate is invoked, unguarded, at each point a wrong stamp can escape."""
    reader = _workflow_reader()
    problems: list[str] = []

    try:
        jobs, steps = reader.scan_workflow(root, RELEASE_WORKFLOW)
        dist_jobs, dist_steps = reader.scan_workflow(root, DISTRIBUTION_WORKFLOW)
    except Exception as exc:  # noqa: BLE001 -- Fatal is the reader's, not ours
        raise Refused(EXIT_CANNOT_RUN, f"the workflow could not be read: {exc}") from None

    # ── release.yml: the tag path ────────────────────────────────────────────
    publishing = [step for step in steps if step.uses.startswith(PUBLISH_ACTION)]
    if len(publishing) != 1:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"{RELEASE_WORKFLOW}: expected exactly one step using `{PUBLISH_ACTION}`, "
            f"found {len(publishing)}. This file cannot say what runs before publishing "
            "if it cannot see the publish.",
        )
    publish = publishing[0]
    release_job = jobs[publish.job]

    if release_job.condition:
        problems.append(
            f"{RELEASE_WORKFLOW}: the job that publishes (`{publish.job}`) carries "
            f"`if: {release_job.condition}` -- a job-level condition skips its steps and "
            "every refusal in them together"
        )
    # A JOB-LEVEL `continue-on-error` is not the step-level one and does not fail
    # the same way. The job still reddens; what changes is that every job whose
    # `needs:` names it runs anyway. Three do -- publishing to crates.io, writing
    # the Homebrew formula and updating the install site -- so a refused stamp
    # would stop the GitHub release and nothing else, and a user installing by
    # `brew` would get the formula for a release that was refused.
    if release_job.continue_on_error:
        problems.append(
            f"{RELEASE_WORKFLOW}: the job that publishes (`{publish.job}`) carries "
            f"`continue-on-error: {release_job.continue_on_error}` -- the job reddens and "
            f"every job whose `needs:` names `{publish.job}` "
            f"({', '.join(sorted(j.id for j in jobs.values() if publish.job in j.needs)) or 'none today'}) "
            "runs anyway, off a release the stamp steps refused"
        )
    # NO CONDITION AT ALL on the step that publishes, not merely no `if: false`.
    # `if: always()` and `if: ${{ !cancelled() }}` are the shapes that matter and
    # they read like resilience: every stamp step above stays present, unguarded
    # and before the publish -- which is all the reading above asks -- and the
    # publish runs whatever any of them decided.
    if publish.condition:
        problems.append(
            f"{RELEASE_WORKFLOW}:{publish.lineno}: the step that publishes carries "
            f"`if: {publish.condition}`. A condition here lets the release be created "
            "after a stamp step above it has refused; `always()` and `!cancelled()` do "
            "exactly that. The publish must have no condition, so a refusal above it "
            "stops it"
        )
    builders = [job for job in jobs.values() if EXTENSION_BUILD_CALL in job.uses]
    if not builders:
        problems.append(
            f"{RELEASE_WORKFLOW}: no job calls `{EXTENSION_BUILD_CALL}`, so nothing here "
            "builds the extension binaries this gate reads"
        )
    elif not any(builder.id in release_job.needs for builder in builders):
        problems.append(
            f"{RELEASE_WORKFLOW}: the job that publishes (`{publish.job}`) does not "
            f"`needs:` the job that builds the extension "
            f"({', '.join(b.id for b in builders)}) -- without that edge the stamp steps "
            "read whatever artefacts happen to be there, or none"
        )
    problems += _upstream_guards(RELEASE_WORKFLOW, builders, "reads")

    # The step that names the assets. Every stamp step in this job reads files
    # under names this script writes, so a rehearsal of those steps is only
    # worth something if it assembles the tree the same way -- and the release
    # cannot be checked at all if nothing assembles it.
    assembling = [step for step in steps if step.job == publish.job and _calls(step, ASSEMBLER_CALL)]
    if len(assembling) != 1:
        problems.append(
            f"{RELEASE_WORKFLOW}: expected exactly one step in `{publish.job}` running "
            f"`{ASSEMBLER}`, found {len(assembling)}. That step is what gives every "
            "published binary its name, and --release-rehearsal runs it to produce the "
            "names the stamp steps are then asked about"
        )
    for step in assembling:
        _unguarded(
            step,
            f"{RELEASE_WORKFLOW}:{step.lineno} ({step.name or 'the assembly step'})",
            problems,
        )

    gate = _gate_steps(steps, publish.job)
    byte_reads = [step for step in gate if not _runs(step, "--load")]
    load_reads = [step for step in gate if _runs(step, "--load")]

    if len(byte_reads) != 1:
        problems.append(
            f"{RELEASE_WORKFLOW}: expected exactly one step in `{publish.job}` reading "
            f"the trailers with `{GATE_INVOCATION}` and no `--load`, found "
            f"{len(byte_reads)}. That step is the only thing that reads the four "
            "platforms this runner cannot load"
        )
    if len(load_reads) != 1:
        problems.append(
            f"{RELEASE_WORKFLOW}: expected exactly one step in `{publish.job}` running "
            f"`{GATE_INVOCATION} --load`, found {len(load_reads)}. That step is the only "
            "thing that makes an artefact answer for itself before it is published"
        )
    for step in gate:
        _unguarded(step, f"{RELEASE_WORKFLOW}:{step.lineno} ({step.name or 'the stamp step'})", problems)
        if step.lineno > publish.lineno:
            problems.append(
                f"{RELEASE_WORKFLOW}:{step.lineno} ({step.name or 'the stamp step'}) runs "
                f"AFTER the publish at line {publish.lineno}. A refusal after the release "
                "is created is a red run over bytes the world already has"
            )
    if not gate:
        problems.append(
            f"{RELEASE_WORKFLOW}: job `{publish.job}` runs `{GATE_INVOCATION}` nowhere, so "
            "a tag publishes whatever version the build stamped"
        )

    # `--load` needs `duckdb` on PATH and a GitHub runner has none. The install
    # is a step of its own rather than lines inside the stamp step, so that the
    # stamp step's `run:` is the gate call and nothing else and --release-rehearsal
    # can run it as it stands, comment lines dropped and continuations joined. The cost of that split is this rung: without an install
    # step before it, the load exits 2 with "duckdb is not on PATH" -- on a tag,
    # and nowhere else, which is the shape this whole mode exists to refuse.
    installers = [
        step for step in steps if step.job == publish.job and _installs_duckdb_cli(step)
    ]
    for step in load_reads:
        if not any(inst.lineno < step.lineno for inst in installers):
            problems.append(
                f"{RELEASE_WORKFLOW}:{step.lineno} ({step.name or 'the stamp step'}) runs "
                f"`{GATE_INVOCATION} --load` with no step above it in `{publish.job}` "
                f"fetching `{DUCKDB_CLI_MARK}...`. The runner has no duckdb, so the load "
                "would exit 2 -- the tool could not run -- and the release would fail on "
                "every tag"
            )

    # ── MainDistributionPipeline.yml: the every-commit path ──────────────────
    dist_builders = [job for job in dist_jobs.values() if EXTENSION_BUILD_CALL in job.uses]
    checking = sorted({step.job for step in dist_steps if GATE_INVOCATION in " ".join(step.commands)})
    if len(checking) != 1:
        problems.append(
            f"{DISTRIBUTION_WORKFLOW}: expected exactly one job running "
            f"`{GATE_INVOCATION}`, found {len(checking)}. This is the only place the "
            "stamp is read on a pull request, where the expected answer is the "
            "abbreviated commit -- without it the autodetection is observed a few days "
            "a year, on tags"
        )
    else:
        stamp_job = dist_jobs[checking[0]]
        if stamp_job.condition:
            problems.append(
                f"{DISTRIBUTION_WORKFLOW}: job `{stamp_job.id}` carries "
                f"`if: {stamp_job.condition}` -- a skipped job reports neutral, not red, "
                "so the pull request stays mergeable and the stamp of every build goes "
                "unread"
            )
        if stamp_job.continue_on_error:
            problems.append(
                f"{DISTRIBUTION_WORKFLOW}: job `{stamp_job.id}` carries "
                f"`continue-on-error: {stamp_job.continue_on_error}` -- the job reddens "
                "and the workflow reports success, which is the same nothing as not "
                "running it"
            )
        problems += _upstream_guards(DISTRIBUTION_WORKFLOW, dist_builders, "reads")
        if not dist_builders:
            problems.append(
                f"{DISTRIBUTION_WORKFLOW}: no job calls `{EXTENSION_BUILD_CALL}`"
            )
        elif not any(builder.id in stamp_job.needs for builder in dist_builders):
            problems.append(
                f"{DISTRIBUTION_WORKFLOW}: job `{stamp_job.id}` does not `needs:` the job "
                f"that builds the binaries ({', '.join(b.id for b in dist_builders)}), so "
                "it downloads artefacts that may not exist yet"
            )
        for step in _gate_steps(dist_steps, stamp_job.id):
            _unguarded(
                step,
                f"{DISTRIBUTION_WORKFLOW}:{step.lineno} ({step.name or 'the stamp step'})",
                problems,
            )

    # ── Every mode this gate offers is run by something ──────────────────────
    #
    # A mode nobody invokes is a check that cannot fail, and it reads in a diff
    # exactly like one that runs. The list is the parser's own: `main` builds
    # the flags from MODES, so a mode that exists is a mode this rung asks
    # about. Across all three workflows rather than ci.yml alone, because two
    # of them are only ever run on a tag and one of those is where the byte
    # reading lives.
    invoked: set[str] = set()
    workflow_steps = list(steps) + list(dist_steps)
    try:
        _, ci_steps = reader.scan_workflow(root, CI_WORKFLOW)
    except Exception as exc:  # noqa: BLE001 -- Fatal is the reader's, not ours
        raise Refused(EXIT_CANNOT_RUN, f"{CI_WORKFLOW} could not be read: {exc}") from None
    workflow_steps += ci_steps
    for step in workflow_steps:
        for line in _calls(step, GATE_CALL):
            invoked.update(mode for mode in MODES if mode in line.split())
    for mode in MODES:
        if mode not in invoked:
            problems.append(
                f"`{GATE_INVOCATION} {mode}` is a mode this gate offers and no step in "
                f"{CI_WORKFLOW}, {RELEASE_WORKFLOW} or {DISTRIBUTION_WORKFLOW} invokes. A "
                "check nothing runs cannot fail, and reads in a diff exactly like one that "
                "does"
            )

    if problems:
        raise Refused(
            EXIT_UNWIRED,
            "the release path no longer refuses a wrong stamp:\n"
            + "".join(f"      {line}\n" for line in problems)
            + "    Each of these leaves every other check in this repository green.",
        )
    return [
        f"   {RELEASE_WORKFLOW}: `{publish.job}` assembles the assets, reads the trailers "
        f"and loads one artefact, unguarded, before the publish at line {publish.lineno}",
        f"   {DISTRIBUTION_WORKFLOW}: `{checking[0]}` reads the stamp of every build",
    ]


# ── the rehearsal ───────────────────────────────────────────────────────────
#
# WHAT --release-wiring CANNOT SEE, AND WHAT IT COST. That mode reads the release
# job as structure: which steps exist, what guards them, what order they are in.
# Every one of its answers is about the TEXT of a step. It said the release path
# refused a wrong stamp while the step that loads an artefact could not run at
# all -- it named its file
# `release-assets/finetype-${GITHUB_REF_NAME}-x86_64-unknown-linux-gnu.duckdb_extension`,
# which is exactly what assemble-release-assets.sh writes, and DuckDB derives the
# init symbol it dlsyms from the file name stemming at the first dot. Every tag
# would have exited 2 there, `Create release` would never have run, and nothing
# would have been published: the opposite of the outcome this file exists to make
# safe. release.yml runs only on a pushed tag, so no check in this repository had
# ever executed that line.
#
# So this mode EXECUTES those steps, on a pull request, against an assembled tree
# with the real names. It is not a second reading of the workflow; it is the
# workflow's own `run:` text, run by bash the way Actions runs it, with
# GITHUB_REF_NAME and the job's `env:` supplied. What it cannot bring is the
# runner: the artefacts come from a real local build re-stamped as a tag rather
# than from five platform runners, so the FILE NAMES are a release's and, on a
# host that is not linux_amd64, the bytes under the linux name are this machine's
# own. Nothing on this path reads a platform out of a name -- require_native
# reads the trailer -- so that substitution is invisible to every rung, and the
# name, which is the thing that was wrong, is real.


def _rehearsable(step) -> list[str]:
    """The step's `run:` as a script, or [] if it is not one this can execute."""
    return _logical_lines(step)


def check_release_rehearsal(root: Path, artifact: Path) -> list[str]:
    """The release job's own assemble and stamp commands, run against real names."""
    reader = _workflow_reader()
    try:
        jobs, steps = reader.scan_workflow(root, RELEASE_WORKFLOW)
    except Exception as exc:  # noqa: BLE001 -- Fatal is the reader's, not ours
        raise Refused(EXIT_CANNOT_RUN, f"the workflow could not be read: {exc}") from None

    publishing = [step for step in steps if step.uses.startswith(PUBLISH_ACTION)]
    if len(publishing) != 1:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"{RELEASE_WORKFLOW}: expected exactly one step using `{PUBLISH_ACTION}`, "
            f"found {len(publishing)}; this mode rehearses what runs BEFORE the publish "
            "and cannot say what that is",
        )
    publish = publishing[0]
    release_job = jobs[publish.job]

    # In file order, and only what runs before the publish: the assembly that
    # names the assets, then every step that invokes the gate.
    runnable = [
        step
        for step in steps
        if step.job == publish.job
        and step.lineno < publish.lineno
        and (_calls(step, ASSEMBLER_CALL) or _calls(step, GATE_CALL))
    ]
    assembling = [s for s in runnable if _calls(s, ASSEMBLER_CALL)]
    gate = [s for s in runnable if _calls(s, GATE_CALL)]
    loads = [s for s in gate if _runs(s, "--load")]
    if not assembling or not loads or len(gate) == len(loads):
        raise Refused(
            EXIT_REHEARSAL,
            f"{RELEASE_WORKFLOW}: job `{publish.job}` has "
            f"{len(assembling)} assembly step(s), {len(gate) - len(loads)} step(s) reading "
            f"trailers and {len(loads)} step(s) loading an artefact before the publish. "
            "This mode rehearses all three and would otherwise report success over a "
            "release path it did not exercise -- --release-wiring says which is missing.",
        )
    # EVERY `run:` BEFORE THE PUBLISH IS ACCOUNTED FOR, and this is what makes
    # the mode close a class rather than a path. The defect was a line only a
    # pushed tag executed; a step added to this job tomorrow is another one, and
    # a rehearsal that quietly walked past it would read exactly like a
    # rehearsal that covered it. One kind of step is skipped: any step whose
    # `run:` lines mention `duckdb_cli-`, meant as the DuckDB CLI install, which
    # is a `curl` and a `sudo mv` that no contributor's machine can run. That is
    # matched by INVOCATION -- a `curl` fetching `duckdb_cli-...`, on a logical
    # line -- so a step that merely names the marker in a comment is enumerated
    # like any other and has to be run or explained.
    unexercised = [
        step
        for step in steps
        if step.job == publish.job
        and step.lineno < publish.lineno
        and step.commands
        and step not in runnable
        and not _installs_duckdb_cli(step)
    ]
    if unexercised:
        raise Refused(
            EXIT_REHEARSAL,
            f"{RELEASE_WORKFLOW}: job `{publish.job}` runs step(s) before the publish "
            "that this mode neither executes nor knowingly exempts:\n"
            + "".join(
                f"      line {s.lineno}: {s.name or '(unnamed)'}\n" for s in unexercised
            )
            + "    Only a pushed tag runs them, so nothing in this repository ever "
            "finds out whether they work. Either make the step call the assembler or "
            "this gate, so it is rehearsed, or say here why it cannot be.",
        )
    for step in runnable:
        if any("<<" in line for line in step.commands):
            raise Refused(
                EXIT_CANNOT_RUN,
                f"{RELEASE_WORKFLOW}:{step.lineno} ({step.name or 'a rehearsed step'}) "
                "contains a heredoc. The reader hands this mode the step's lines with "
                "their indentation stripped, which a heredoc body depends on, so running "
                "it would not be running what the release runs.",
            )

    # The artefact has to be loadable HERE: the last rehearsed step loads one.
    base = stamp_fields(artifact)
    here = require_native(artifact, base)
    # The exact shape of a release: `git tag --points-at HEAD` writes the tag
    # verbatim, so the trailer carries a leading `v` that Cargo.toml's
    # compiled-in version does not. Derived from the artefact rather than
    # written here, so it cannot go stale at a version bump.
    tag = f"v{core(base['extension_version'])}"

    env_key = "EXTENSION_DUCKDB_VERSION"
    if env_key not in release_job.env:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"{RELEASE_WORKFLOW}: job `{publish.job}` has no `{env_key}` in its `env:`. "
            "The extension artifact directories are named for it, so this mode cannot "
            "build the tree the assembly step reads.",
        )
    ext_version = release_job.env[env_key]

    assembler = root / ".github" / "scripts" / ASSEMBLER
    if not assembler.is_file():
        raise Refused(EXIT_CANNOT_RUN, f"{assembler} is missing")

    reported: list[str] = []
    with tempfile.TemporaryDirectory(prefix="stamp-rehearsal-") as tmp:
        work = Path(tmp) / "workspace"
        work.mkdir()
        # The workspace a step sees: `scripts/…` and `.github/…` resolve exactly
        # as they do in a checkout, and everything the steps write lands here.
        for rel in ("scripts", ".github"):
            (work / rel).symlink_to((root / rel).resolve(), target_is_directory=True)

        fixture = subprocess.run(
            [
                str(assembler),
                "--make-fixture",
                str(work),
                "--tag",
                tag,
                "--extension-duckdb-version",
                ext_version,
            ],
            capture_output=True,
            text=True,
            check=False,
        )
        if fixture.returncode != 0:
            raise Refused(
                EXIT_CANNOT_RUN,
                f"{ASSEMBLER} --make-fixture exited {fixture.returncode}:\n"
                f"{fixture.stderr.strip()}",
            )

        # Real bytes under the placeholder names. Each of the five is a copy of
        # the artefact re-stamped with `tag` by the real upstream stamper, so
        # what the assembly step copies and the stamp steps then read is a
        # genuine extension carrying a genuine tag stamp.
        plain = Path(tmp) / "unstamped.bin"
        plain.write_bytes(artifact.read_bytes()[:-TRAILER_LEN])
        binaries = sorted((work / "artifacts").glob(f"finetype-{ext_version}-extension-*"))
        if len(binaries) != len(DISTRIBUTED_PLATFORMS):
            raise Refused(
                EXIT_CANNOT_RUN,
                f"the fixture holds {len(binaries)} extension artifact directories, "
                f"expected {len(DISTRIBUTED_PLATFORMS)}",
            )
        for directory in binaries:
            _stamp(plain, directory / CANONICAL_NAME, tag, base)

        env = {
            **os.environ,
            **release_job.env,
            "GITHUB_REF_NAME": tag,
            "GITHUB_REF_TYPE": "tag",
        }
        for step in runnable:
            script = Path(tmp) / f"step-{step.lineno}.sh"
            script.write_text("\n".join(_rehearsable(step)) + "\n", encoding="utf-8")
            # `bash --noprofile --norc -eo pipefail {0}` is what Actions runs a
            # `shell: bash` step with, `-u` deliberately absent. Matched here so
            # a step that survives this survives the runner.
            proc = subprocess.run(
                ["bash", "--noprofile", "--norc", "-eo", "pipefail", str(script)],
                cwd=work,
                env=env,
                capture_output=True,
                text=True,
                check=False,
            )
            label = step.name or f"the step at line {step.lineno}"
            if proc.returncode != 0:
                raise Refused(
                    EXIT_REHEARSAL,
                    f"{RELEASE_WORKFLOW}:{step.lineno} ({label}) exited "
                    f"{proc.returncode} when run against the assets this job assembles, "
                    f"at tag {tag}. On a pushed tag this step would fail and the release "
                    "would not be created.\n"
                    + "".join(
                        f"      {line}\n"
                        for line in (proc.stdout + proc.stderr).strip().splitlines()[-12:]
                    ),
                )
            reported.append(f"   {RELEASE_WORKFLOW}:{step.lineno} ({label}): exit 0")

    return [
        f"   rehearsed the `{publish.job}` job at tag {tag} on {here}, against the asset "
        f"names {ASSEMBLER} writes",
        *reported,
    ]


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST -- a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════


STAMPER = "extension-ci-tools/scripts/append_extension_metadata.py"


def _stamp(
    source: Path,
    out: Path,
    version: str,
    base: dict[str, str],
    platform: str | None = None,
    name: str = "finetype",
) -> None:
    """Stamp a copy of `source` using the REAL upstream stamper.

    Not a local reimplementation: the whole point of a fixture here is that its
    trailer was written by the same code that writes a released artefact's, so a
    change in that code is seen rather than mirrored. It is stdlib-only and
    needs no venv.

    EVERY FIELD BUT THE VERSION COMES OFF THE ARTEFACT UNDER TEST -- that is
    what `base` is, its own `stamp_fields`. A literal here is a constant of the
    machine that WROTE this file, and the proof then only runs where it was
    written: the first cut passed `-p osx_arm64`, and on a linux runner the
    three LOAD cases failed with "built for the platform 'osx_arm64', but we can
    only load extensions built for platform 'linux_amd64'". The ABI type and the
    DuckDB floor are the same shape of constant and are read the same way. Pass
    `platform` only to make a fixture that is DELIBERATELY not this machine's.
    """

    script = ROOT / STAMPER
    if not script.is_file():
        raise Refused(
            EXIT_CANNOT_RUN,
            f"{STAMPER} is missing -- the self-test stamps its fixtures with the real "
            "stamper, so it needs the extension-ci-tools submodule "
            "(git submodule update --init --recursive)",
        )
    proc = subprocess.run(
        [
            sys.executable,
            str(script),
            "-l",
            str(source),
            "-o",
            str(out),
            "-n",
            name,
            "-dv",
            base["duckdb_version"],
            "-p",
            platform or base["platform"],
            "-ev",

            version,
            "--abi-type",
            base["abi_type"],
        ],

        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN, f"the stamper failed for {version!r}:\n{proc.stderr.strip()}"
        )


def _run(argv: list[str], env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    """This script, in a real subprocess, so a case reads the exit code a job reads.

    `env` is how a case substitutes the ENVIRONMENT rather than an argument --
    the one rung below that needs a different answer from `duckdb` gets it by
    putting a different `duckdb` on PATH, which is the only substitution that
    reaches a value this file asks another program for.
    """
    return subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *argv],
        capture_output=True,
        text=True,
        env=env,
        check=False,
    )


def self_test(artifact: Path) -> int:
    failures: list[str] = []

    def case(
        label: str,
        argv: list[str],
        want: int,
        expect_text: str = "",
        env: dict[str, str] | None = None,
    ) -> None:
        proc = _run(argv, env)
        got = proc.returncode
        blob = proc.stdout + proc.stderr
        if got != want:
            failures.append(f"  MISS {label}: exit {got}, expected {want}\n{blob.rstrip()}")
        elif expect_text and expect_text not in blob:
            # WHICH rung fired, not merely that one did. Several of the codes
            # below are one edit apart, and a case that reads only the number
            # passes when a different defect wears this one's result.
            failures.append(
                f"  WRONG {label}: exit {want} without saying {expect_text!r}\n{blob.rstrip()}"
            )
        else:
            print(f"  ok   {label}")

    def rung(label: str, call, want: int, expect_text: str) -> None:
        """A refusal the command line cannot reach, driven in-process.

        Two rungs of check_loaded answer a READING rather than a file, and no
        file on this disk produces either: DuckDB dlsyms an init symbol derived
        from the file name, so a library that is not this extension is refused
        before the reading is taken, and DuckDB's own trailer reading cannot
        disagree with this file's while both use the same offsets. Each is
        driven by substituting exactly one thing -- the session, or the offset
        constant -- and leaving the rest of check_loaded real, including the
        LOAD in the second. Neither is disclosed as covered by the CLI cases
        above, because neither is.
        """
        try:
            call()
        except Refused as refusal:
            blob = str(refusal)
            if refusal.code != want:
                failures.append(f"  MISS {label}: exit {refusal.code}, expected {want}\n{blob}")
            elif expect_text not in blob:
                failures.append(
                    f"  WRONG {label}: exit {want} without saying {expect_text!r}\n{blob}"
                )
            else:
                print(f"  ok   {label}")
            return
        failures.append(f"  MISS {label}: nothing refused")

    with tempfile.TemporaryDirectory(prefix="stamp-selftest-") as tmp:

        tmp_path = Path(tmp)

        # ── Fixtures stamped by the real stamper ───────────────────────────
        # `artifact` is a REAL built extension. Every version fixture below is a
        # copy of it re-stamped, so each one loads wherever the artefact does --
        # the AC5 cases need that, and it keeps the byte-level cases off a shape
        # DuckDB would never accept. Only the trailer is replaced: the 512 bytes
        # stripped here are the eight metadata fields and the signature space,
        # which is the whole of what a reader looks at.
        #
        # `base` is the artefact's own trailer, and it is what every fixture
        # field except the version is taken from -- see _stamp for what a
        # literal there costs. require_native refuses, by name, an artefact this
        # machine cannot load, rather than letting the first LOAD case die of it.
        base = stamp_fields(artifact)
        here = require_native(artifact, base)
        plain = tmp_path / "unstamped.bin"
        plain.write_bytes(artifact.read_bytes()[:-TRAILER_LEN])

        # The version the artefact reports at RUN TIME, compiled into it from
        # Cargo.toml. Read once and used to build the AC5 fixtures, so those
        # cases pin "the trailer agrees with what this artefact says about
        # itself" rather than agreeing with a literal in this file that goes
        # stale on the next version bump -- at which point a control that no
        # longer holds would be indistinguishable from a real refusal.
        runtime = str(duckdb_readings(artifact)["runtime"])[len("finetype ") :].strip()
        if not runtime:
            raise Refused(EXIT_CANNOT_RUN, f"{artifact} reports no version at run time")

        # A version no build can have, for the case that needs the trailer and
        # the compiled-in version to disagree.
        other = "9.9.9"
        if core(runtime) == other:
            raise Refused(EXIT_CANNOT_RUN, f"the mismatch fixture {other} is a real version")

        # EVERY FIXTURE IS NAMED finetype.duckdb_extension, in a directory of its
        # own. DuckDB derives the init symbol it dlsyms from the FILE NAME, not
        # from the extension name in the trailer, so a fixture called
        # `mislabelled.duckdb_extension` fails to load with
        # `did not contain function "mislabelled_init_c_api"` -- exit 2, which
        # every AC5 case would then report instead of the verdict it is for.
        overlong_tag = "v1.4.7-and-a-tag-name-far-past-the-field"

        # A platform this machine is not, for the fixture that has to be
        # unloadable HERE for a reason that is nothing to do with its version.
        foreign_platform = next(p for p in DISTRIBUTED_PLATFORMS if p != here)

        def fixture(key: str, version: str, platform: str | None = None) -> Path:
            out = tmp_path / key / "finetype.duckdb_extension"
            out.parent.mkdir(parents=True, exist_ok=True)
            _stamp(plain, out, version, base, platform)
            return out

        fixtures = {
            # Byte-level cases: the version and the tag are both chosen here, so
            # nothing about them depends on the artefact.
            "bare": "1.4.7",
            "vprefixed": "v1.4.7",
            "committed": "0.6.23",
            "sha": "3f466ff",
            "tooshort": "3f466f",
            "prefix": "1.4",
            "twotags": "v1.4.7\nv1.4.8",
            "overlong": overlong_tag,
            # AC5 cases: stamped from what this artefact actually is.
            "honest": runtime,
            # The exact shape a release has: `git tag --points-at HEAD` writes
            # the tag verbatim, so the trailer carries the `v` and Cargo.toml's
            # compiled-in version does not. Both `core()` calls on the --load
            # path are exercised by the one case that uses this, and removing
            # either reddens it.
            "vhonest": f"v{core(runtime)}",
            "mislabelled": other,
        }
        paths = {key: fixture(key, version) for key, version in fixtures.items()}
        # Correct in every field but the platform, which is deliberately not
        # this machine's. Nothing about its VERSION is wrong.
        paths["foreign"] = fixture("foreign", runtime, foreign_platform)

        def arg(key: str) -> str:
            return str(paths[key])

        # ── THE FIXTURES ARE THE ARTEFACT, in every field but the one under
        #    test. This is the case that fails when a constant of one machine
        #    is written into a proof that has to run on another; the first cut
        #    had three of them and only ran where it was written. It reads each
        #    fixture's trailer BACK, rather than trusting the arguments _stamp
        #    was given, because the stamper is the thing under observation. ───
        for key in sorted(paths):
            if key == "overlong":
                # The one fixture this cannot be asked of, and the reason is the
                # reason it exists: its version is longer than the 32-byte field,
                # so the trailer is longer than 512 bytes and every field read at
                # a fixed offset in the last 512 is shifted by the overflow. What
                # that shift does to the VERSION is the point, and the case
                # `a tag longer than the 32-byte field` below is what pins it.
                continue
            want_platform = foreign_platform if key == "foreign" else base["platform"]

            got = stamp_fields(paths[key])
            wrong = [
                f"{field_name}={got[field_name]!r}, not {want!r}"
                for field_name, want in (
                    ("abi_type", base["abi_type"]),
                    ("duckdb_version", base["duckdb_version"]),
                    ("platform", want_platform),
                )
                if got[field_name] != want
            ]
            label = f"the {key} fixture carries the artefact's own trailer fields"
            if wrong:
                failures.append(f"  MISS {label}: {'; '.join(wrong)}")
            else:
                print(f"  ok   {label}")

        # ── Controls ───────────────────────────────────────────────────────
        case("a tag-stamped artefact under its own tag", [
            "--artifact", arg("vprefixed"), "--tag", "v1.4.7"], EXIT_OK)
        case("a bare-stamped artefact under a v-prefixed tag", [
            "--artifact", arg("bare"), "--tag", "v1.4.7"], EXIT_OK)
        case("several artefacts, all correct", [
            "--artifact", arg("bare"), "--artifact", arg("vprefixed"),
            "--tag", "1.4.7"], EXIT_OK)

        # ── The defect this gate exists for ────────────────────────────────
        case("the committed literal under a real tag", [
            "--artifact", arg("committed"), "--tag", "v1.4.7"],
            EXIT_STAMP_DISAGREES, "'0.6.23'")
        case("one bad artefact among good ones is still refused", [
            "--artifact", arg("bare"), "--artifact", arg("committed"),
            "--tag", "v1.4.7"], EXIT_STAMP_DISAGREES, "'0.6.23'")
        case("the untagged fallback under a real tag", [
            "--artifact", arg("sha"), "--tag", "v1.4.7"],
            EXIT_STAMP_DISAGREES, "short commit sha")
        case("a version that is a PREFIX of the tag", [
            "--artifact", arg("prefix"), "--tag", "v1.4.7"],
            EXIT_STAMP_DISAGREES, "'1.4'")
        case("two tags on one commit", [
            "--artifact", arg("twotags"), "--tag", "v1.4.7"],
            EXIT_STAMP_DISAGREES, "more than one line")

        # ── The untagged expectation, which is what every pull request build
        #    of this repository is checked against. The committed literal is
        #    refused here too, which is what makes the fix observable without
        #    anyone cutting a tag. ──────────────────────────────────────────
        head = "3f466ff3ab8d4d97e2c1b0e6f5a7c9d201234567"
        case("an untagged build stamped with its own commit", [
            "--artifact", arg("sha"), "--expect-commit", head], EXIT_OK)
        case("an untagged build carrying the committed literal", [
            "--artifact", arg("committed"), "--expect-commit", head],
            EXIT_STAMP_DISAGREES, "did not derive the stamp at all")
        case("an untagged build stamped from a different commit", [
            "--artifact", arg("sha"), "--expect-commit", "0" * 40],
            EXIT_STAMP_DISAGREES, "abbreviated sha")
        case("an untagged build stamped too short to be an abbreviation", [
            "--artifact", arg("tooshort"), "--expect-commit", head],
            EXIT_STAMP_DISAGREES, "abbreviated sha")
        case("a tag under the untagged expectation", [
            "--artifact", arg("vprefixed"), "--expect-commit", head],
            EXIT_STAMP_DISAGREES, "abbreviated sha")
        case("both expectations at once", [
            "--artifact", arg("sha"), "--expect-commit", head, "--tag", "v1.4.7"],
            EXIT_CANNOT_RUN, "exactly one of")

        # ── Not a trailer at all: exit 3, never the version rung ───────────
        case("an unstamped shared library", [
            "--artifact", str(plain), "--tag", "v1.4.7"],
            EXIT_NO_TRAILER, "carries no DuckDB metadata trailer")
        # A tag longer than the field is not a corrupt trailer: the fields AFTER
        # the version shift forward by exactly as much as the 512-byte window
        # shifts back, so the magic still reads and only the version is wrong --
        # it keeps the last 32 bytes of the tag. Measured, and the reason this
        # case asserts the version rung rather than the trailer one.
        case("a tag longer than the 32-byte field", [
            "--artifact", arg("overlong"), "--tag", overlong_tag],
            EXIT_STAMP_DISAGREES, "is the tail of it")
        short = tmp_path / "short.bin"
        short.write_bytes(b"\x00" * 64)
        case("a file shorter than the trailer", [
            "--artifact", str(short), "--tag", "v1.4.7"],
            EXIT_NO_TRAILER, "shorter than")

        # ── AC5, driven at the altitude the defect lives: a REAL loadable
        #    artefact whose trailer says one release and whose compiled-in
        #    version says another. A mutation of the comparison itself would
        #    pass without either reading ever being taken off the file. ──────
        case("a real artefact stamped with what it reports at run time", [
            "--artifact", arg("honest"), "--load"], EXIT_OK)
        case("a real artefact re-stamped as another release", [
            "--artifact", arg("mislabelled"), "--load"],
            EXIT_RUNTIME_DISAGREES, "at run time and carries")
        case("a real artefact whose run-time version is not the tag", [
            "--artifact", arg("honest"), "--load", "--tag", f"v{other}"],
            EXIT_STAMP_DISAGREES, "at run time and was built from tag")
        # THE EXACT SHAPE OF A RELEASE, which no case above had: a v-prefixed
        # trailer, a bare compiled-in version, and the v-prefixed tag both are
        # measured against. Removing either `core()` call on this path reddens
        # this and nothing else.
        case("the release shape -- a v-prefixed stamp under its own v-prefixed tag", [
            "--artifact", arg("vhonest"), "--load", "--tag", f"v{core(runtime)}"], EXIT_OK)
        # An artefact this machine cannot load is refused BY NAME and for the
        # right reason -- the platform, which is nothing to do with its version.
        # A mode that skipped instead would read exactly like one that passed,
        # and this is the case that says which happened.
        case("an artefact built for another platform", [
            "--artifact", arg("foreign"), "--load"],
            EXIT_CANNOT_RUN, f"was built for platform {foreign_platform!r}")

        # ── The two rungs no file reaches. See `rung` for why each needs a
        #    substitution and what stays real in it. ────────────────────────
        rung(
            "a loaded library that is not this extension",
            lambda: check_loaded(
                paths["honest"],
                None,
                readings=lambda _path: {
                    "runtime": "notfinetype 1.4.7",
                    "trailer": runtime,
                    "loaded": True,
                    "installed": False,
                },
            ),
            EXIT_RUNTIME_DISAGREES,
            "does not name finetype",
        )

        def drifted_offsets() -> None:
            """A real LOAD of a real fixture, read at an offset that has moved.

            The rung fires when the constants at the top of this file stop
            agreeing with what the stamper writes -- which is the
            extension-ci-tools bump nothing else here would notice. It cannot
            be provoked from a fixture, because DuckDB and this file read the
            same bytes at the same offsets; the only way they disagree is for
            one side to move, and DuckDB's side is not ours to move.
            """
            global OFF_EXTENSION_VERSION
            saved = OFF_EXTENSION_VERSION
            OFF_EXTENSION_VERSION = OFF_ABI
            try:
                check_loaded(paths["honest"], None)
            finally:
                OFF_EXTENSION_VERSION = saved

        rung(
            "this file's trailer offsets drifted from the stamper's",
            drifted_offsets,
            EXIT_RUNTIME_DISAGREES,
            "no longer match what the stamper writes",
        )

        # The two rungs that decide WHOSE reading this is. Both were deletable
        # with every case above green, while the module doc claimed the second
        # of them: a session that reports `installed` may be answering off a
        # community build already on the runner rather than off the file named
        # on the command line, and `loaded` false means the LOAD did not take
        # and the row is about nothing. Neither can be reached from a file --
        # a real LOAD of a real artefact sets loaded true and installed false --
        # so each substitutes the session and leaves the rest of check_loaded
        # real, exactly as the two rungs above do.
        def substituted(**row) -> Callable[[], list[str]]:
            reading = {"runtime": f"finetype {runtime}", "trailer": runtime, **row}
            return lambda: check_loaded(paths["honest"], None, readings=lambda _path: reading)

        rung(
            "a session that did not report the artefact as loaded",
            substituted(loaded=False, installed=False),
            EXIT_CANNOT_RUN,
            "did not report",
        )
        rung(
            "a reading that may have come from an INSTALLED community build",
            substituted(loaded=True, installed=True),
            EXIT_CANNOT_RUN,
            "INSTALLED as well as loaded",
        )

        # ── `here` is DUCKDB'S answer, not a literal ───────────────────────
        # require_native's claim is that the platform it refuses over is the one
        # DuckDB would refuse over. A literal there -- the runner's own platform,
        # written in by hand -- is correct on the machine it names and wrong on
        # every other, and NO FIXTURE CAN SEE IT: a fixture's platform field is
        # compared against whatever `here` holds, so the comparison agrees with
        # itself either way. What is substituted is therefore the ANSWER: a
        # `duckdb` earlier on PATH reporting a platform no artefact carries. The
        # refusal has to name it, which it can only do by having asked.
        stub_bin = tmp_path / "stub-bin"
        stub_bin.mkdir()
        stub = stub_bin / "duckdb"
        stub.write_text(
            '#!/bin/sh\necho \'[{"platform":"STAMP-PROBE-PLATFORM"}]\'\n', encoding="utf-8"
        )
        stub.chmod(0o755)
        case(
            "the platform refused over is what duckdb answers, not a literal",
            ["--artifact", arg("honest"), "--load"],
            EXIT_CANNOT_RUN,
            "this machine loads 'STAMP-PROBE-PLATFORM'",
            env={**os.environ, "PATH": f"{stub_bin}{os.pathsep}{os.environ['PATH']}"},
        )

        # ── AC2, against real git trees. `git ls-files` reads the INDEX, so a
        #    fixture needs `add` and no commit -- which also keeps a repository
        #    hook out of a case that is about neither. ───────────────────────
        for label, add, ignore, want, text in (
            ("a tree with the version file tracked", True, True, EXIT_TRACKED, "tracked in git"),
            ("a tree with it untracked but not ignored", False, False, EXIT_NOT_IGNORED,
             "is not ignored"),
            ("a tree with it untracked and ignored", False, True, EXIT_OK, ""),
        ):
            repo = tmp_path / f"repo-{want}-{int(ignore)}"
            (repo / CONFIGURE_DIR).mkdir(parents=True)
            (repo / VERSION_FILE).write_text("0.6.23\n", encoding="utf-8")
            (repo / ".gitignore").write_text(
                f"/{VERSION_FILE}\n" if ignore else "", encoding="utf-8"
            )
            subprocess.run(["git", "-C", str(repo), "init", "-q"], check=True,
                           capture_output=True)
            if add:
                subprocess.run(["git", "-C", str(repo), "add", "-f", VERSION_FILE],
                               check=True, capture_output=True)
            case(label, ["--untracked-version-file", "--root", str(repo)], want, text)

        # ── AC3's local half: make's rebuild decision, on the real Makefile
        #    and on a copy with the forcing prerequisite taken out. The
        #    mutation is of the Makefile, not of this checker, because the
        #    defect is make's decision and not a string. ───────────────────
        case("the repository's own Makefile", ["--regenerates-version-file"], EXIT_OK)
        mutated = tmp_path / "Makefile.no-force"
        text = (ROOT / "Makefile").read_text(encoding="utf-8")
        marker = f"{VERSION_FILE}: extension_version_is_regenerated"
        if marker not in text:
            failures.append(
                f"  MISS the Makefile mutation: it no longer contains {marker!r}, so the "
                "case below would prove nothing"
            )
        mutated.write_text(text.replace(marker, f"# removed: {marker}"), encoding="utf-8")
        case("a Makefile with the forcing prerequisite removed", [
            "--regenerates-version-file", "--makefile", str(mutated)],
            EXIT_NO_REGENERATION, "would NOT rewrite")

        # ── The wiring, against real copies of the two release workflows and
        #    against one-edit mutations of them. Every mutation below leaves
        #    every other check in this repository at exit 0, including the
        #    cases above: this is the only thing that reads these files. ────
        counter = itertools.count()

        def sub(old: str, new: str):
            """One textual edit, refused unless its anchor is where it says."""

            def edit(text: str) -> str:
                if text.count(old) != 1:
                    raise LookupError(f"{old!r} occurs {text.count(old)} times")
                return text.replace(old, new)

            return edit

        def cut(start: str, end: str):
            """Delete from `start` up to `end`, both anchored exactly once."""

            def edit(text: str) -> str:
                for anchor in (start, end):
                    if text.count(anchor) != 1:
                        raise LookupError(f"{anchor!r} occurs {text.count(anchor)} times")
                return text[: text.index(start)] + text[text.index(end) :]

            return edit

        def swap(first: str, second: str, after: str):
            """Put the `second` block before the `first` one, in file order."""

            def edit(text: str) -> str:
                for anchor in (first, second, after):
                    if text.count(anchor) != 1:
                        raise LookupError(f"{anchor!r} occurs {text.count(anchor)} times")
                a, b, c = (text.index(anchor) for anchor in (first, second, after))
                if not a < b < c:
                    raise LookupError("the three anchors are not in file order")
                return text[:a] + text[b:c] + text[a:b] + text[c:]

            return edit

        def drop_to_end(start: str):
            """Delete from `start` to the end of the file."""

            def edit(text: str) -> str:
                if text.count(start) != 1:
                    raise LookupError(f"{start!r} occurs {text.count(start)} times")
                return text[: text.index(start)]

            return edit

        def wiring_case(
label: str, edits: dict, want: int, expect_text: str = "") -> None:

            root = tmp_path / f"wiring-{next(counter)}"
            (root / ".github" / "workflows").mkdir(parents=True)
            # ci.yml too: the mode reads all three, because the rung asking
            # whether every mode is invoked cannot answer from the two
            # workflows that only ever run on a tag.
            for rel in (RELEASE_WORKFLOW, DISTRIBUTION_WORKFLOW, CI_WORKFLOW):
                text = (ROOT / rel).read_text(encoding="utf-8")
                if rel in edits:
                    try:
                        text = edits[rel](text)
                    except LookupError as gone:
                        # The anchor moved. Say so rather than reporting a case
                        # that mutated nothing as a case that passed.
                        failures.append(
                            f"  MISS {label}: the mutation no longer applies to {rel} "
                            f"({gone}), so this case would prove nothing"
                        )
                        return
                (root / rel).write_text(text, encoding="utf-8")
            case(label, ["--release-wiring", "--root", str(root)], want, expect_text)

        byte_step = "      - name: The extension binaries carry this tag, read off their own bytes\n"
        load_step = "      - name: ...and the one this runner can load agrees with itself and the tag\n"
        publish_step = "      - name: Create release\n"

        wiring_case("the repository's own release workflows", {}, EXIT_OK)
        wiring_case(
            "a stamp step whose failure is ignored",
            {RELEASE_WORKFLOW: sub(byte_step, byte_step + "        continue-on-error: true\n")},
            EXIT_UNWIRED, "continue-on-error: true")
        wiring_case(
            "a stamp step behind a condition",
            {RELEASE_WORKFLOW: sub(load_step, load_step + "        if: github.event_name == 'push'\n")},
            EXIT_UNWIRED, "a skipped step is a GREEN job")
        wiring_case(
            "a stamp step on the runner's default shell",
            {RELEASE_WORKFLOW: sub(byte_step + "        shell: bash\n", byte_step)},
            EXIT_UNWIRED, "`pipefail`")

        # Three rungs, three cases, and the two below are why. Deleting BOTH
        # stamp steps fires all three at once, so that case alone leaves the two
        # counting rungs deletable: a lower rung catches the same input and says
        # something else. Each of these leaves one step standing, so exactly one
        # rung can answer it.
        wiring_case(
            "only the step that loads an artefact deleted",
            {RELEASE_WORKFLOW: cut(load_step, publish_step)},
            EXIT_UNWIRED, "running `check_extension_stamp.py --load`, found 0")
        wiring_case(
            "only the step that reads the five trailers deleted",
            {RELEASE_WORKFLOW: cut(byte_step, load_step)},
            EXIT_UNWIRED, "and no `--load`, found 0")
        wiring_case(
            "both stamp steps deleted from the release",
            {RELEASE_WORKFLOW: cut(byte_step, publish_step)},
            EXIT_UNWIRED, "runs `check_extension_stamp.py` nowhere")

        wiring_case(
            "the stamp steps moved after the publish",
            {RELEASE_WORKFLOW: swap(byte_step, publish_step, "\n  publish-crates:\n")},
            EXIT_UNWIRED, "runs AFTER the publish")
        wiring_case(
            "the release job no longer needing the extension build",

            {RELEASE_WORKFLOW: sub(
                "    needs: [build, build-extension, taxonomy-catalogue]\n",
                "    needs: [build, taxonomy-catalogue]\n")},
            EXIT_UNWIRED, "does not `needs:` the job that builds the extension")
        wiring_case(
            "the whole release job behind a condition",
            {RELEASE_WORKFLOW: sub(
                "  release:\n    name: Create Release\n",
                "  release:\n    if: github.event_name == 'workflow_dispatch'\n    name: Create Release\n")},
            EXIT_UNWIRED, "a job-level condition skips its steps")
        wiring_case(
            "the stamp job deleted from the distribution pipeline",
            {DISTRIBUTION_WORKFLOW: drop_to_end("  stamp:\n")},
            EXIT_UNWIRED, "expected exactly one job running")
        wiring_case(
            "the distribution stamp job behind a condition",
            {DISTRIBUTION_WORKFLOW: sub(
                "  stamp:\n    name:", "  stamp:\n    if: github.ref_type == 'tag'\n    name:")},
            EXIT_UNWIRED, "a skipped job reports neutral, not red")

        wiring_case(
            "the distribution stamp job not needing the build",
            {DISTRIBUTION_WORKFLOW: sub("    needs: duckdb-stable-build\n", "")},
            EXIT_UNWIRED, "does not `needs:` the job that builds the binaries")

        # ── Present, unguarded, before the publish, AND STILL WORTH NOTHING.
        #    Every case below leaves the reading above satisfied. Each was
        #    measured green before its rung existed. ────────────────────────
        assemble_step = "      - name: Assemble release assets\n"
        install_step = "      - name: Install duckdb CLI\n"
        gate_call = '          scripts/check_extension_stamp.py "${args[@]}" --tag "${GITHUB_REF_NAME}"\n'

        # `always()` and `!cancelled()` are the shapes this actually takes, and
        # they read as resilience rather than as a switch: every stamp step
        # stays exactly where it was and the publish stops caring what any of
        # them decided.
        wiring_case(
            "the publish running whatever the stamp steps decided",
            {RELEASE_WORKFLOW: sub(publish_step, publish_step + "        if: always()\n")},
            EXIT_UNWIRED, "the step that publishes carries `if: always()`")
        wiring_case(
            "...and the same one written as `!cancelled()`",
            {RELEASE_WORKFLOW: sub(
                publish_step, publish_step + "        if: ${{ !cancelled() }}\n")},
            EXIT_UNWIRED, "the step that publishes carries `if: ${{ !cancelled() }}`")
        # The job reddens, the release is not created, and the three jobs that
        # `needs:` it publish to crates.io, write the Homebrew formula and
        # update the install site anyway.
        wiring_case(
            "a release job allowed to fail under its dependants",
            {RELEASE_WORKFLOW: sub(
                "  release:\n    name: Create Release\n",
                "  release:\n    continue-on-error: true\n    name: Create Release\n")},
            EXIT_UNWIRED, "every job whose `needs:` names `release`")
        # A skipped job skips its dependants, and GitHub reports the workflow
        # green. The stamp steps stay present, correctly wired, and never run.
        wiring_case(
            "the extension build switched off in the release",
            {RELEASE_WORKFLOW: sub(
                "  build-extension:\n    name: Build DuckDB extension\n",
                "  build-extension:\n    if: false\n    name: Build DuckDB extension\n")},
            EXIT_UNWIRED,
            f"{RELEASE_WORKFLOW}: the job that builds the extension binaries "
            "(`build-extension`) carries `if: false`")
        wiring_case(
            "the extension build switched off in the distribution pipeline",
            {DISTRIBUTION_WORKFLOW: sub(
                "  duckdb-stable-build:\n    name: Build extension binaries\n",
                "  duckdb-stable-build:\n    if: false\n    name: Build extension binaries\n")},
            EXIT_UNWIRED,
            f"{DISTRIBUTION_WORKFLOW}: the job that builds the extension binaries "
            "(`duckdb-stable-build`) carries `if: false`")
        wiring_case(
            "the distribution stamp job allowed to fail",
            {DISTRIBUTION_WORKFLOW: sub(
                "  stamp:\n    name:", "  stamp:\n    continue-on-error: true\n    name:")},
            EXIT_UNWIRED, "the same nothing as not running it")

        # A step whose `run:` MENTIONS this gate is not a step that runs it.
        # Both shapes below leave the step, its name, its shell and its position
        # exactly as they were.
        wiring_case(
            "a stamp step whose call is commented out",
            {RELEASE_WORKFLOW: sub(gate_call, f"          # {gate_call.strip()}\n")},
            EXIT_UNWIRED, "and no `--load`, found 0")
        # And a comment that DESCRIBES the call rather than repeating it. This
        # is the shape the `#` filter in _logical_lines is for and the only one
        # that reaches past the invocation pattern: `then` is a shell separator,
        # so `... then scripts/check_extension_stamp.py exits 1` sits in command
        # position as far as a regex is concerned. Workflow comments in this
        # repository are written exactly like that sentence.
        wiring_case(
            "a stamp step left with a comment ABOUT the call",
            {RELEASE_WORKFLOW: sub(
                gate_call,
                "          # if a trailer disagrees then "
                "scripts/check_extension_stamp.py exits 1\n")},
            EXIT_UNWIRED, "and no `--load`, found 0")
        wiring_case(
            "a stamp step that only echoes the call",
            {RELEASE_WORKFLOW: sub(
                gate_call, '          echo "ran scripts/check_extension_stamp.py"\n')},
            EXIT_UNWIRED, "and no `--load`, found 0")

        # Nothing names the assets, so the stamp steps read an empty directory
        # -- and the rehearsal below has nothing to rehearse against.
        wiring_case(
            "the assembly step deleted",
            {RELEASE_WORKFLOW: cut(assemble_step, byte_step)},
            EXIT_UNWIRED, f"running `{ASSEMBLER}`, found 0")
        # The load needs `duckdb` on PATH and a runner has none. Without this
        # rung the install could be deleted and the load would exit 2 on a tag,
        # which is the same release-only failure this whole mode exists for.
        wiring_case(
            "the duckdb CLI install deleted from the release job",
            {RELEASE_WORKFLOW: cut(install_step, load_step)},
            EXIT_UNWIRED, f"fetching `{DUCKDB_CLI_MARK}...`")
        # ...and the same step LEFT IN PLACE with its fetch replaced by a
        # comment that names the asset. Everything the rung above reads is
        # still true -- the step exists, it is unguarded, it is above the load,
        # and its `run:` contains `duckdb_cli-` -- and there is no duckdb on
        # the runner. Measured green before this rung matched invocations, with
        # the tag exiting 2 at the load and publishing nothing.
        wiring_case(
            "the duckdb CLI fetch replaced by a comment naming the asset",
            {RELEASE_WORKFLOW: sub(
                "          curl -fsSL -o duckdb.zip "
                "https://github.com/duckdb/duckdb/releases/download/v1.5.3/"
                "duckdb_cli-linux-amd64.zip\n",
                "          # the duckdb_cli-linux-amd64.zip pin is shared with ci.yml\n"
                '          echo "installing the duckdb CLI"\n')},
            EXIT_UNWIRED, f"fetching `{DUCKDB_CLI_MARK}...`")

        # A mode of this gate that CI stopped running. The step is present in
        # every other sense -- named, in the right job -- and `true` is what a
        # step left as a placeholder looks like. Nothing else in this
        # repository reads ci.yml for the checks it is supposed to be running.
        wiring_case(
            "a mode of this gate that ci.yml stopped invoking",
            {CI_WORKFLOW: sub(
                "        run: scripts/check_extension_stamp.py --regenerates-version-file\n",
                "        run: true\n")},
            EXIT_UNWIRED,
            "--regenerates-version-file` is a mode this gate offers and no step")

        # ── THE REHEARSAL: the same steps, RUN. Every case above reads the
        #    workflow; these execute it, against a tree assembled by the job's
        #    own assembler, and are the only thing here that would have caught
        #    a stamp step that cannot run at all. ──────────────────────────
        def rehearsal_case(label: str, edits: dict, want: int, expect_text: str = "") -> None:
            root = tmp_path / f"rehearsal-{next(counter)}"
            (root / ".github" / "workflows").mkdir(parents=True)
            # The scripts are the REAL ones: what a case here mutates is the
            # workflow, never the gate it calls.
            (root / "scripts").symlink_to(ROOT / "scripts", target_is_directory=True)
            (root / ".github" / "scripts").symlink_to(
                ROOT / ".github" / "scripts", target_is_directory=True
            )
            for rel in (RELEASE_WORKFLOW, DISTRIBUTION_WORKFLOW):
                text = (ROOT / rel).read_text(encoding="utf-8")
                if rel in edits:
                    try:
                        text = edits[rel](text)
                    except LookupError as gone:
                        failures.append(
                            f"  MISS {label}: the mutation no longer applies to {rel} "
                            f"({gone}), so this case would prove nothing"
                        )
                        return
                (root / rel).write_text(text, encoding="utf-8")
            case(
                label,
                ["--release-rehearsal", "--artifact", str(artifact), "--root", str(root)],
                want,
                expect_text,
            )

        rehearsal_case("the repository's own release job, RUN", {}, EXIT_OK)
        # The defect this mode was written for, in the form it took: the load
        # step naming a file the assembler does not write. Here the name is a
        # target triple that is not one of the five; in the version that
        # shipped it was the right name and DuckDB refused it for its dots.
        rehearsal_case(
            "the load step naming an asset the assembler never writes",
            {RELEASE_WORKFLOW: sub(
                '"release-assets/finetype-${GITHUB_REF_NAME}-x86_64-unknown-linux-gnu.duckdb_extension"',
                '"release-assets/finetype-${GITHUB_REF_NAME}-x86_64-unknown-linux-musl.duckdb_extension"')},
            EXIT_REHEARSAL, "and the one this runner can load agrees with itself and the tag) exited 2")
        # The assembly and the reading pointed at different directories. Both
        # steps are present, unguarded and in the right order; the release
        # publishes nothing because the second finds no files.
        # The class, not the instance: the defect was a line only a pushed tag
        # executed, and a step added to this job tomorrow is another one.
        rehearsal_case(
            "a new step in the release job that nothing here would run",
            {RELEASE_WORKFLOW: sub(
                publish_step,
                "      - name: Something a tag alone would run\n"
                "        shell: bash\n"
                "        run: ./tools/publish-notes.sh\n" + publish_step)},
            EXIT_REHEARSAL, "neither executes nor knowingly exempts")
        # The exemption, from the other side. One kind of step is knowingly not
        # executed here -- the CLI install, a `curl` and a `sudo mv` no
        # contributor's machine can run -- and while that exemption matched a
        # MENTION, any step could claim it by naming the asset in a comment.
        # This one would exit 3 on a tag and was exempted, unrun, before the
        # exemption asked for an invocation.
        rehearsal_case(
            "a step exempting itself by naming the CLI asset in a comment",
            {RELEASE_WORKFLOW: sub(
                publish_step,
                "      - name: Something a tag alone would run\n"
                "        shell: bash\n"
                "        run: |\n"
                "          # same pin as the duckdb_cli- install above\n"
                "          exit 3\n" + publish_step)},
            EXIT_REHEARSAL, "neither executes nor knowingly exempts")
        rehearsal_case(
            "the assembly writing somewhere the stamp steps do not read",
            {RELEASE_WORKFLOW: sub(
                "            --output-dir release-assets \\\n",
                "            --output-dir elsewhere \\\n")},
            EXIT_REHEARSAL, "expected 5 extension binaries in release-assets, found 0")

    if failures:
        print("")
        for line in failures:
            print(line)
        print(f"\nself-test FAILED: {len(failures)} case(s) not detected correctly")
        return 1
    print("\nself-test passed")
    return 0


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(add_help=True, description=__doc__)
    parser.add_argument("--artifact", action="append", default=[], type=Path)
    parser.add_argument("--tag")
    parser.add_argument("--expect-commit")
    # The modes, added from MODES rather than one by one, because
    # --release-wiring requires every entry in it to be invoked by a workflow
    # step. Two lists would let a mode be added to the parser and to nothing
    # else, which is the check-nobody-runs this file exists to refuse.
    for mode in MODES:
        parser.add_argument(mode, action="store_true")

    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--makefile", type=Path)
    args = parser.parse_args(argv)

    try:
        if args.self_test:
            if len(args.artifact) != 1:
                raise Refused(
                    EXIT_CANNOT_RUN,
                    "--self-test needs exactly one --artifact: a real built extension. "
                    "Its LOAD cases cannot be synthesised, and a case that skips itself "
                    "when the artefact is absent reads exactly like a case that passed.\n"
                    "    Build one first: make build-extension",
                )
            return self_test(args.artifact[0])

        reported: list[str] = []
        did_something = False

        if args.untracked_version_file:
            reported += check_untracked(args.root)
            did_something = True
        if args.regenerates_version_file:
            reported += check_regenerates(args.root, args.makefile)
            did_something = True
        if args.release_wiring:
            reported += check_release_wiring(args.root)
            did_something = True
        if args.release_rehearsal:
            if len(args.artifact) != 1:
                raise Refused(
                    EXIT_CANNOT_RUN,
                    "--release-rehearsal needs exactly one --artifact: a real built "
                    "extension this machine can load. It re-stamps five copies of it as "
                    "a tag and runs the release job's own commands over them, and the "
                    "last of those commands LOADs one.\n"
                    "    Build one first: make build-extension",
                )
            reported += check_release_rehearsal(args.root, args.artifact[0])
            did_something = True
            args.artifact = []

        if args.load:
            if len(args.artifact) != 1:
                raise Refused(EXIT_CANNOT_RUN, "--load needs exactly one --artifact")
            reported += check_loaded(args.artifact[0], args.tag)
            did_something = True
        elif args.artifact:
            if bool(args.tag) == bool(args.expect_commit):
                raise Refused(
                    EXIT_CANNOT_RUN,
                    "--artifact needs exactly one of --tag, --expect-commit or --load: "
                    "there is nothing to compare a trailer against on its own, and a "
                    "tagged ref and an untagged one expect different stamps",
                )
            if args.tag:
                reported += check_against_tag(args.artifact, args.tag)
            else:
                reported += check_against_commit(args.artifact, args.expect_commit)
            did_something = True

        if not did_something:
            parser.print_usage(sys.stderr)
            return EXIT_CANNOT_RUN
    except Refused as refusal:
        print(f"check-extension-stamp: {refusal}", file=sys.stderr)
        return refusal.code

    for line in reported:
        print(line)
    print("check-extension-stamp: the stamp is the release it claims to be.")
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
