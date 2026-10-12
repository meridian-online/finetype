#!/usr/bin/env python3
"""Re-measure the README's DuckDB version table against the community registry.

README.md carries a table: per DuckDB version, what the community build of
finetype answered to SELECT ft_version() on osx_arm64, on a dated run. Nothing
re-measured it after the registry republished, so a cell could go stale with
no one noticing. This does the measuring, on a schedule, and reports.

WHAT IT DOES
    1. Parse the table: one row per DuckDB version, each cell read into an
       outcome: a version string, "not published" (INSTALL fails with HTTP
       404), or "refused" (INSTALL serves a file the version refuses to load).
    2. For each row, download that DuckDB CLI release for this platform and
       run the README's two lines, INSTALL finetype FROM community; and
       LOAD finetype; then SELECT ft_version(), falling back to
       SELECT finetype_version() when the first name does not exist (the
       build that predates the ft_ names). Classify the result the same way
       and compare.
    3. List every stable DuckDB release at or above the table's lowest
       version that has no row.

A stale cell or a missing row is reported and the exit is 1. The README is
never edited: a cell is a measurement with a date, and the fix is a new
measurement a person commits.

It is a signal, not a gate. The workflow that runs it is scheduled and never
triggered by a pull request.

USAGE
    scripts/check_readme_version_table.py --self-test   # offline: parse, classify, compare
    scripts/check_readme_version_table.py --measure     # network: the real thing
    scripts/check_readme_version_table.py --measure --report report.md

Stdlib only.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
RELEASES_URL = "https://api.github.com/repos/duckdb/duckdb/releases?per_page=100"

ROW_RE = re.compile(r"^> \| (\d+\.\d+\.\d+) \| (.*) \|\s*$")
VERSION_RE = re.compile(r"`finetype (\d+\.\d+\.\d+)`")


@dataclass(frozen=True)
class Outcome:
    kind: str          # "version" | "not_published" | "refused" | "unreadable" | "error"
    detail: str = ""   # the version for "version", the message otherwise

    def __str__(self) -> str:
        return f"{self.kind}:{self.detail}" if self.detail else self.kind


@dataclass(frozen=True)
class Row:
    duckdb: str
    cell: str
    outcome: Outcome


def version_key(v: str) -> tuple[int, ...]:
    return tuple(int(p) for p in v.split("."))


def classify_cell(cell: str) -> Outcome:
    m = VERSION_RE.search(cell)
    if m:
        return Outcome("version", m.group(1))
    if "HTTP 404" in cell:
        return Outcome("not_published")
    if "refuses to load" in cell:
        return Outcome("refused")
    return Outcome("unreadable", cell)


def parse_table(text: str) -> list[Row]:
    rows = []
    for line in text.splitlines():
        m = ROW_RE.match(line)
        if m:
            rows.append(Row(m.group(1), m.group(2), classify_cell(m.group(2))))
    if not rows:
        raise SystemExit("no version-table rows found; the README table shape changed")
    return rows


def classify_probe(install_out: str, load_out: str, version_out: str) -> Outcome:
    """Classify one DuckDB release's probe from the three statements' output.

    A refused build surfaces at INSTALL on some releases and at LOAD on others,
    so both outputs are read the same way: a 404 means no build is published,
    a version complaint means the served build is for another DuckDB.
    """
    for out in (install_out, load_out):
        if "Error" not in out:
            continue
        if "404" in out:
            return Outcome("not_published")
        if "version" in out.lower():
            return Outcome("refused")
        return Outcome("error", out.strip().splitlines()[-1])
    m = re.search(r"finetype (\d+\.\d+\.\d+)", version_out)
    if m:
        return Outcome("version", m.group(1))
    return Outcome("error", version_out.strip().splitlines()[-1] if version_out.strip() else "no output")


def missing_rows(rows: list[Row], releases: list[dict]) -> list[str]:
    """Stable releases at or above the table's lowest version with no row."""
    have = {r.duckdb for r in rows}
    floor = min(version_key(r.duckdb) for r in rows)
    out = []
    for rel in releases:
        if rel.get("prerelease") or rel.get("draft"):
            continue
        tag = rel.get("tag_name", "")
        if not re.fullmatch(r"v\d+\.\d+\.\d+", tag):
            continue
        v = tag[1:]
        if version_key(v) >= floor and v not in have:
            out.append(v)
    return sorted(out, key=version_key)


def compare(rows: list[Row], measured: dict[str, Outcome]) -> list[str]:
    problems = []
    for r in rows:
        got = measured[r.duckdb]
        if r.outcome.kind == "unreadable":
            problems.append(f"{r.duckdb}: cell is not readable as a version or a known failure: {r.cell!r}")
        elif got != r.outcome:
            problems.append(f"{r.duckdb}: README says {r.outcome}, measured {got}")
    return problems


# ── measurement ─────────────────────────────────────────────────────────────

def cli_asset() -> str:
    system, machine = platform.system(), platform.machine()
    if system == "Darwin":
        return "duckdb_cli-osx-universal.zip"
    if system == "Linux":
        return "duckdb_cli-linux-aarch64.zip" if machine in ("arm64", "aarch64") else "duckdb_cli-linux-amd64.zip"
    raise SystemExit(f"unsupported platform {system}/{machine}")


def fetch_cli(version: str, into: Path) -> Path:
    url = f"https://github.com/duckdb/duckdb/releases/download/v{version}/{cli_asset()}"
    into.mkdir(parents=True, exist_ok=True)
    zip_path = into / f"duckdb-{version}.zip"
    with urllib.request.urlopen(url, timeout=120) as resp, open(zip_path, "wb") as fh:
        fh.write(resp.read())
    with zipfile.ZipFile(zip_path) as zf:
        zf.extract("duckdb", into)
    exe = into / "duckdb"
    exe.chmod(0o755)
    return exe


def run_sql(exe: Path, ext_dir: Path, sql: str) -> str:
    proc = subprocess.run(
        [str(exe), "-noheader", "-csv", "-c", f"SET extension_directory='{ext_dir}'; {sql}"],
        capture_output=True, text=True, timeout=300,
    )
    return proc.stdout + proc.stderr


def probe(version: str, work: Path) -> Outcome:
    exe = fetch_cli(version, work / version)
    ext_dir = work / version / "ext"
    ext_dir.mkdir(parents=True, exist_ok=True)
    install_out = run_sql(exe, ext_dir, "INSTALL finetype FROM community;")
    if "Error" in install_out:
        return classify_probe(install_out, "", "")
    load_out = run_sql(exe, ext_dir, "LOAD finetype;")
    if "Error" in load_out:
        return classify_probe(install_out, load_out, "")
    version_out = run_sql(exe, ext_dir, "LOAD finetype; SELECT ft_version();")
    if "Catalog Error" in version_out:
        # The build predates the ft_ names (the 1.5.0 row); its old name answers.
        version_out = run_sql(exe, ext_dir, "LOAD finetype; SELECT finetype_version();")
    return classify_probe(install_out, load_out, version_out)


def fetch_releases() -> list[dict]:
    req = urllib.request.Request(RELEASES_URL, headers={"Accept": "application/vnd.github+json"})
    token = os.environ.get("GITHUB_TOKEN")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req, timeout=60) as resp:
        return json.load(resp)


def measure(readme: Path, report: Path | None) -> int:
    rows = parse_table(readme.read_text(encoding="utf-8"))
    measured: dict[str, Outcome] = {}
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        for r in rows:
            measured[r.duckdb] = probe(r.duckdb, work)
            print(f"  {r.duckdb}: README {r.outcome}, measured {measured[r.duckdb]}")
    problems = compare(rows, measured)
    missing = missing_rows(rows, fetch_releases())
    lines = [f"Re-measured {len(rows)} rows on {platform.system()}/{platform.machine()}.", ""]
    if problems:
        lines += ["Stale cells:"] + [f"- {p}" for p in problems] + [""]
    if missing:
        lines += ["DuckDB releases with no row:"] + [f"- {v}" for v in missing] + [""]
    if not problems and not missing:
        lines.append("Every cell matches and every release has a row.")
    text = "\n".join(lines)
    print(text)
    if report:
        report.write_text(text + "\n", encoding="utf-8")
    return 1 if problems or missing else 0


# ── self-test ───────────────────────────────────────────────────────────────

FIXTURE = """\
> | DuckDB | `SELECT ft_version()` |
> |---|---|
> | 1.5.6 | `finetype 0.6.57` |
> | 1.5.2 | no build published — `INSTALL` fails with HTTP 404 |
> | 1.5.1 | `INSTALL` fails — the file served is the 1.5.0 build, which 1.5.1 refuses to load |
> | 1.5.0 | `ft_version()` does not exist — the build is `finetype 0.2.0`; `SELECT finetype_version()` answers |
"""


def self_test() -> int:
    passed = failed = 0

    def case(name: str, ok: bool, detail: str = "") -> None:
        nonlocal passed, failed
        if ok:
            passed += 1
            print(f"  [PASS] {name}")
        else:
            failed += 1
            print(f"  [FAIL] {name} {detail}")

    rows = parse_table(FIXTURE)
    case("parse_four_rows", [r.duckdb for r in rows] == ["1.5.6", "1.5.2", "1.5.1", "1.5.0"], str(rows))
    case("cell_version", rows[0].outcome == Outcome("version", "0.6.57"))
    case("cell_not_published", rows[1].outcome == Outcome("not_published"))
    case("cell_refused", rows[2].outcome == Outcome("refused"))
    case("cell_old_build_reads_its_version", rows[3].outcome == Outcome("version", "0.2.0"))
    case("cell_unreadable", classify_cell("something else entirely").kind == "unreadable")

    case("probe_version", classify_probe("", "", "finetype 0.6.57\n") == Outcome("version", "0.6.57"))
    case("probe_404", classify_probe("IO Error: Failed to download extension ... HTTP 404", "", "").kind == "not_published")
    case("probe_refused_at_load", classify_probe("", "Invalid Input Error: ... built for DuckDB version 'v1.5.0' ...", "").kind == "refused")
    case("probe_refused_at_install", classify_probe("IO Error: ... built for DuckDB version 'v1.5.0' ...", "", "").kind == "refused")
    case("probe_other_error", classify_probe("", "", "Catalog Error: ft_version does not exist").kind == "error")

    measured = {
        "1.5.6": Outcome("version", "0.6.57"),
        "1.5.2": Outcome("not_published"),
        "1.5.1": Outcome("refused"),
        "1.5.0": Outcome("version", "0.2.0"),
    }
    case("compare_all_match", compare(rows, measured) == [])
    stale = dict(measured, **{"1.5.6": Outcome("version", "0.6.61")})
    case("compare_stale_version", any("1.5.6" in p and "0.6.61" in p for p in compare(rows, stale)))
    unreadable = parse_table(FIXTURE + "> | 1.4.4 | served, probably |\n")
    case("compare_reports_unreadable", any("1.4.4" in p and "not readable" in p for p in compare(unreadable, dict(measured, **{"1.4.4": Outcome("version", "0.6.0")}))))

    releases = [
        {"tag_name": "v1.5.7", "prerelease": False},
        {"tag_name": "v1.5.6", "prerelease": False},
        {"tag_name": "v2.0.0", "prerelease": False},
        {"tag_name": "v2.0.0-dev", "prerelease": True},
        {"tag_name": "v1.4.4", "prerelease": False},
    ]
    case("missing_rows_above_floor", missing_rows(rows, releases) == ["1.5.7", "2.0.0"])
    case("prerelease_is_not_missing", "2.0.0-dev" not in missing_rows(rows, releases))

    print(f"\nResults: {passed} passed, {failed} failed")
    return 0 if failed == 0 else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").split("\n")[0])
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--measure", action="store_true")
    parser.add_argument("--readme", type=Path, default=README)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if args.measure:
        return measure(args.readme, args.report)
    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
