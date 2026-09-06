#!/usr/bin/env python3
"""Gate the version stamp a built DuckDB extension carries in its metadata trailer.

WHY THIS EXISTS
    The stamp is the only version a `.duckdb_extension` carries in a form a
    machine can read BEFORE loading it. DuckDB surfaces it as
    `duckdb_extensions().extension_version`; a packager reads it straight out of
    the file's last 512 bytes. Anyone who has to tell two builds apart reads
    that field and nothing else.

    Every extension this project has ever published carries `0.6.23`, whatever
    tag it was built from. The cause is one committed file. The build contract
    writes the version through this rule, in
    extension-ci-tools/makefiles/c_api_extensions/base.Makefile:

        configure/extension_version.txt:
        	@ $(VERSION_COMMAND)

    A file target with no prerequisites is never remade once it exists, so a
    tree that ARRIVES with the file already in it never runs VERSION_COMMAND at
    all. The file was tracked in git, so every checkout arrived with it, so the
    autodetection has never run in CI, and the committed literal has been
    stamped onto every artefact since it was committed. Measured on the local
    build of 2026-06-04: trailer offset 128 reads `0.6.23` while the tree was
    at 0.6.58.

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

        THE LOAD IS THE ONLY THING HERE THAT ESTABLISHES IDENTITY. Any valid
        shared library stamped by extension-ci-tools' append_extension_metadata
        script passes a trailer read; only running it proves the artefact is
        this extension. That is why this mode exists as well as the one above,
        and why it also requires DuckDB to report the extension as loaded and
        NOT installed -- an installed community build could otherwise answer
        for the file on disk.

    --untracked-version-file        (exit 4, exit 5)
        Nothing under configure/ is tracked in git, and
        configure/extension_version.txt is ignored. Both halves, because they
        fail differently: a tracked file disables the rule that writes it, and
        an un-ignored one comes back on the next `git add`.

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
    extension. `--tag` is never passed on an untagged build, because the only
    workflow that passes it triggers on `tags: ['v*']`. A tagged build cannot
    pass with a fallback stamp: a short sha does not equal the tag, so the
    comparison refuses and says which of the two it is looking at.

USAGE
    scripts/check_extension_stamp.py --artifact F [--artifact F ...] --tag TAG
    scripts/check_extension_stamp.py --artifact F [--artifact F ...] --expect-commit SHA
    scripts/check_extension_stamp.py --artifact F --load [--tag TAG]
    scripts/check_extension_stamp.py --untracked-version-file
    scripts/check_extension_stamp.py --regenerates-version-file
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

SHORT_SHA = re.compile(r"^[0-9a-f]{7,40}$")


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
    """Load one artefact and read its run-time and trailer versions in one session."""
    if shutil.which("duckdb") is None:
        raise Refused(
            EXIT_CANNOT_RUN,
            "duckdb is not on PATH -- this mode loads the artefact and asks it",
        )
    with tempfile.TemporaryDirectory(prefix="stamp-load-") as tmp:
        prelude = f"SET extension_directory='{tmp}';\nLOAD '{extension.resolve()}';\n"
        proc = subprocess.run(
            ["duckdb", "-unsigned", "-no-init", "-json", "-c", prelude + LOAD_SQL],
            capture_output=True,
            text=True,
            env={**os.environ, "DUCKDB_EXTENSION_DIRECTORY": tmp},
            check=False,
        )
    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN,
            f"duckdb exited {proc.returncode} loading {extension}:\n{proc.stderr.strip()}",
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


def check_loaded(extension: Path, tag: str | None) -> list[str]:
    """AC5 -- run-time version and trailer version, off one artefact, in one session."""
    fields = stamp_fields(extension)
    row = duckdb_readings(extension)

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


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST -- a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════

STAMPER = "extension-ci-tools/scripts/append_extension_metadata.py"


def _stamp(source: Path, out: Path, version: str, name: str = "finetype") -> None:
    """Stamp a copy of `source` using the REAL upstream stamper.

    Not a local reimplementation: the whole point of a fixture here is that its
    trailer was written by the same code that writes a released artefact's, so a
    change in that code is seen rather than mirrored. It is stdlib-only and
    needs no venv.
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
            "v1.2.0",
            "-p",
            "osx_arm64",
            "-ev",
            version,
            "--abi-type",
            "C_STRUCT",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise Refused(
            EXIT_CANNOT_RUN, f"the stamper failed for {version!r}:\n{proc.stderr.strip()}"
        )


def _run(argv: list[str]) -> subprocess.CompletedProcess[str]:
    """This script, in a real subprocess, so a case reads the exit code a job reads."""
    return subprocess.run(
        [sys.executable, str(Path(__file__).resolve()), *argv],
        capture_output=True,
        text=True,
        check=False,
    )


def self_test(artifact: Path) -> int:
    failures: list[str] = []

    def case(label: str, argv: list[str], want: int, expect_text: str = "") -> None:
        proc = _run(argv)
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

    with tempfile.TemporaryDirectory(prefix="stamp-selftest-") as tmp:
        tmp_path = Path(tmp)

        # ── Fixtures stamped by the real stamper ───────────────────────────
        # `artifact` is a REAL built extension. Every version fixture below is a
        # copy of it re-stamped, so each one still LOADS -- the AC5 cases need
        # that, and it keeps the byte-level cases off a shape DuckDB would never
        # accept. Only the trailer is replaced: the 512 bytes stripped here are
        # the eight metadata fields and the signature space, which is the whole
        # of what a reader looks at.
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

        def fixture(key: str, version: str) -> Path:
            out = tmp_path / key / "finetype.duckdb_extension"
            out.parent.mkdir(parents=True, exist_ok=True)
            _stamp(plain, out, version)
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
            "mislabelled": other,
        }
        paths = {key: fixture(key, version) for key, version in fixtures.items()}

        def arg(key: str) -> str:
            return str(paths[key])

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
    parser.add_argument("--load", action="store_true")
    parser.add_argument("--untracked-version-file", action="store_true")
    parser.add_argument("--regenerates-version-file", action="store_true")
    parser.add_argument("--self-test", action="store_true")
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
