#!/usr/bin/env python3
"""Gate every taxonomy-size number quoted in prose against the shipped taxonomy.

Sixteen green CI checks say nothing about whether a documented claim is true,
because nothing in CI reads documentation. This does, for one narrow but
recurring defect family: a headline count and a per-domain table that disagree
with each other, and both disagreeing with the data.

WHAT IS DERIVED (the source of truth)
    `labels/definitions_*.yaml` — the seven domain files that
    `crates/finetype-core/src/taxonomy.rs` embeds with `include_str!`, i.e. the
    taxonomy the CLI and the DuckDB extension actually ship. The file list on
    disk is compared against that `include_str!` list, so a domain file that is
    added but not embedded (or embedded but deleted) fails here rather than
    being silently counted or silently shipped.

        total ............ number of `domain.category.type` definitions
        domains .......... number of distinct domains
        domain_files ..... number of definitions_*.yaml files
        per-domain ....... the breakdown
        locale_specific .. definitions with `designation: locale_specific`
        validation_by_locale ... definitions carrying per-locale patterns
        locales:<key> .... locales listed under one definition's
                           `validation_by_locale` block

WHAT IS ASSERTED
    Each entry in CLAIMS names a file and a regex over its prose, and binds the
    captured number to one derived fact. A registered claim that no longer
    matches is a failure too — reworded prose must re-register, it must not fall
    silently out of the gate. The README domain table, the ARCHITECTURE inline
    breakdown and the LOCALE_DETECTION coverage table are checked row by row AND
    against their headline, so headline, table and data cannot drift apart.

WHAT IS *NOT* ASSERTED (scope, stated plainly)
    Model-side counts — CharCNN classes, "the shipped model predicts N labels",
    evaluation coverage ("N tested types") — are NOT taxonomy sizes and have a
    different source of truth (a model artifact, an eval run). This gate does not
    check them and must not be read as evidence that they are right. They are
    listed in ACKNOWLEDGED with the reason they are out of scope.

    Dated evidence reports under docs/ are snapshots of a past measurement and
    are exempt (SNAPSHOT_FILES); rewriting them would falsify the record.

KNOWN BLIND SPOTS (so nobody mistakes this for a general prose checker)
    - The fail-closed scan recognises "N types", "N definitions" and "N domains".
      It does NOT recognise "N formats", "N labels" or "N classes" — those read
      as model-side vocabulary here, and admitting them would fill ACKNOWLEDGED
      with entries this gate cannot check anyway. A NEW sentence phrased
      "35 technology formats" would pass unseen unless it is registered.
    - It checks numbers, nothing else. Wrong return types, wrong attributions
      and wrong prose of every other kind are outside it entirely.

FAIL-CLOSED
    Any count-shaped phrase ("N types", "N definitions", "N domains") found in a
    scanned, non-exempt file that is neither registered nor acknowledged is a
    failure. A new wrong number cannot be added without CI noticing.

USAGE
    scripts/check_doc_taxonomy_counts.py            # gate the working tree
    scripts/check_doc_taxonomy_counts.py --facts    # print the derived numbers
    scripts/check_doc_taxonomy_counts.py --self-test  # prove the gate detects

Stdlib only — no PyYAML (CI's system python is not guaranteed to have it; see
the `import yaml` guards in scripts/test_evidence.sh).
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
import tempfile
from pathlib import Path

# ══════════════════════════════════════════════════════════════════════════════
# DERIVATION — read the shipped taxonomy
# ══════════════════════════════════════════════════════════════════════════════

KEY_RE = re.compile(r"^(?P<key>[a-z][a-z0-9_]*\.[a-z0-9_]+\.[a-z0-9_]+):\s*$")
INCLUDE_RE = re.compile(r'include_str!\("(?:\.\./)+labels/(definitions_[a-z_]+\.yaml)"\)')
LOCALE_SPECIFIC_RE = re.compile(r"""^ {2}designation:\s*["']?locale_specific["']?\s*$""")
BY_LOCALE_RE = re.compile(r"^ {2}validation_by_locale:\s*$")
# Quoted keys are real: YAML 1.1 reads a bare NO as a boolean, so `"NO":` is how
# Norway is spelled here. An unquoted-NO regex silently undercounts.
LOCALE_KEY_RE = re.compile(r"""^ {4}(?P<q>["']?)(?P<locale>[A-Z][A-Z0-9_]*)(?P=q):\s*$""")

TAXONOMY_RS = "crates/finetype-core/src/taxonomy.rs"

WORD_NUMBERS = {
    "six": 6,
    "seven": 7,
    "eight": 8,
    "nine": 9,
    "ten": 10,
}


class Fatal(Exception):
    """The taxonomy itself could not be read — nothing downstream is meaningful."""


def derive(root: Path) -> dict:
    """Derive the taxonomy facts from labels/definitions_*.yaml.

    The scan is deliberately strict rather than tolerant: a top-level line that
    is not a `domain.category.type:` key aborts instead of being skipped, so the
    count can never quietly diverge from what serde_yaml would parse.
    """
    labels_dir = root / "labels"
    on_disk = sorted(p.name for p in labels_dir.glob("definitions_*.yaml"))
    if not on_disk:
        raise Fatal(f"no definitions_*.yaml under {labels_dir}")

    tax_rs = root / TAXONOMY_RS
    if not tax_rs.is_file():
        raise Fatal(f"missing {TAXONOMY_RS} — cannot confirm what the build embeds")
    embedded = sorted(set(INCLUDE_RE.findall(tax_rs.read_text(encoding="utf-8"))))
    if not embedded:
        raise Fatal(
            f"no include_str! of labels/definitions_*.yaml found in {TAXONOMY_RS}; "
            "this gate counts what the build embeds and can no longer tell what that is"
        )
    if embedded != on_disk:
        raise Fatal(
            "labels/ and the embedded taxonomy disagree — the numbers in the docs "
            "would describe files the build does not ship.\n"
            f"    on disk : {', '.join(on_disk)}\n"
            f"    embedded: {', '.join(embedded)}"
        )

    per_domain: dict[str, int] = {}
    seen: dict[str, str] = {}
    locale_specific = 0
    validation_by_locale = 0
    locales: dict[str, int] = {}

    for name in on_disk:
        path = labels_dir / name
        file_domain = name[len("definitions_") : -len(".yaml")]
        current = None
        in_locale_block = False
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if not line.strip() or line.lstrip().startswith("#"):
                continue
            if line[0] not in " \t":
                in_locale_block = False
                if line.rstrip() in ("---", "..."):
                    continue
                match = KEY_RE.match(line)
                if not match:
                    raise Fatal(
                        f"{name}:{lineno}: top-level line is not a "
                        f"`domain.category.type:` key — refusing to guess a count "
                        f"from a file this scanner does not fully understand:\n"
                        f"    {line.rstrip()}"
                    )
                key = match.group("key")
                if key in seen:
                    raise Fatal(f"duplicate definition {key}: {seen[key]} and {name}:{lineno}")
                seen[key] = f"{name}:{lineno}"
                domain = key.split(".", 1)[0]
                if domain != file_domain:
                    raise Fatal(
                        f"{name}:{lineno}: {key} is a `{domain}` label in the "
                        f"`{file_domain}` domain file"
                    )
                per_domain[domain] = per_domain.get(domain, 0) + 1
                current = key
            elif current is not None:
                if LOCALE_SPECIFIC_RE.match(line):
                    locale_specific += 1
                    in_locale_block = False
                elif BY_LOCALE_RE.match(line):
                    validation_by_locale += 1
                    locales[current] = 0
                    in_locale_block = True
                elif line.startswith("  ") and not line.startswith("   "):
                    # a sibling property of the definition — the block has ended
                    in_locale_block = False
                elif in_locale_block and line.startswith("    ") and not line.startswith("     "):
                    match = LOCALE_KEY_RE.match(line)
                    if not match:
                        raise Fatal(
                            f"{name}:{lineno}: `validation_by_locale` entry for "
                            f"{current} is not an uppercase locale key — refusing "
                            f"to count locales from a block this scanner does not "
                            f"fully understand:\n    {line.rstrip()}"
                        )
                    locales[current] += 1

    facts = {
        "total": sum(per_domain.values()),
        "domains": len(per_domain),
        "domain_files": len(on_disk),
        "per_domain": per_domain,
        "locale_specific": locale_specific,
        "validation_by_locale": validation_by_locale,
        "locale_coverage": locales,
    }
    facts.update({f"locales:{key}": count for key, count in locales.items()})
    facts.update({f"per_domain:{key}": count for key, count in per_domain.items()})
    return facts


# ══════════════════════════════════════════════════════════════════════════════
# THE CLAIM REGISTRY — every documented taxonomy-size number, and what it must equal
# ══════════════════════════════════════════════════════════════════════════════
#
# (path, regex, {capture group: derived fact})
#
# Every regex must match at least once. Reword the prose and the gate fails with
# "registered claim not found" — that is the point: a claim may not leave the
# gate quietly.

CLAIMS: list[tuple[str, str, dict[str, str]]] = [
    (
        "README.md",
        r"taxonomy of (?P<n>\d+) semantic types",
        {"n": "total"},
    ),
    (
        "README.md",
        r"\*\*(?P<n>\d+) semantic types\*\* across (?P<d>\d+) domains",
        {"n": "total", "d": "domains"},
    ),
    (
        "README.md",
        r"recognizes \*\*(?P<n>\d+) types\*\* across \*\*(?P<d>\d+) domains\*\*",
        {"n": "total", "d": "domains"},
    ),
    (
        "docs/ARCHITECTURE.md",
        r"Taxonomy definitions \((?P<n>\d+) types, (?P<d>\d+) domains, YAML\)",
        {"n": "total", "d": "domains"},
    ),
    (
        "docs/ARCHITECTURE.md",
        r"Each of the (?P<n>\d+) types is defined in YAML",
        {"n": "total"},
    ),
    (
        "docs/ARCHITECTURE.md",
        r"`labels/definitions_\*\.yaml` \((?P<n>\d+) domain files\)",
        {"n": "domain_files"},
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        r"taxonomy of (?P<n>\d+) types",
        {"n": "total"},
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        r"\*\*Granularity\*\* \| (?P<n>\d+) leaf types in (?P<d>\d+) domains",
        {"n": "total", "d": "domains"},
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        r"Full YAML definitions for all (?P<n>\d+) types",
        {"n": "total"},
    ),
    (
        "docs/LOCALE_DETECTION_ARCHITECTURE.md",
        r"(?P<n>\d+) types are designated `locale_specific` in the taxonomy\. "
        r"Of these, (?P<v>\d+) now have",
        {"n": "locale_specific", "v": "validation_by_locale"},
    ),
    (
        "docs/LOCALE_DETECTION_ARCHITECTURE.md",
        r"`validation_by_locale` patterns exist for (?P<n>\d+) types",
        {"n": "validation_by_locale"},
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        r"(?P<n>\d+) types in the `datetime` domain",
        {"n": "per_domain:datetime"},
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        r"the (?P<dt>\d+) datetime, (?P<tech>\d+) technology and (?P<cont>\d+) container types",
        {
            "dt": "per_domain:datetime",
            "tech": "per_domain:technology",
            "cont": "per_domain:container",
        },
    ),
    (
        "README.md",
        r"validates (?P<postal>\d+) locales for postal codes, (?P<phone>\d+) for "
        r"phone numbers, (?P<monthday>\d+) for month/day names",
        {
            "postal": "locales:geography.address.postal_code",
            "phone": "locales:identity.person.phone_number",
            "monthday": "locales:datetime.component.month_name",
        },
    ),
    # The same three locale counts appear again inside the README domain table's
    # Examples column. Registered separately so the bullet above and the table
    # cannot say different things.
    (
        "README.md",
        r"month/day names \((?P<n>\d+) locales\)",
        {"n": "locales:datetime.component.month_name"},
    ),
    (
        "README.md",
        r"phone numbers \((?P<n>\d+) locales\)",
        {"n": "locales:identity.person.phone_number"},
    ),
    (
        "README.md",
        r"postal codes \((?P<n>\d+) locales\)",
        {"n": "locales:geography.address.postal_code"},
    ),
    (
        "crates/finetype-core/README.md",
        r"(?P<n>\d+) semantic type definitions across (?P<d>[a-z]+) domains",
        {"n": "total", "d": "domains"},
    ),
    (
        "crates/finetype-core/src/lib.rs",
        r"synthetic data generation for all (?P<n>\d+) types",
        # `finetype check` (CI job "Taxonomy Check") exits non-zero unless every
        # taxonomy definition has a working generator, so generator coverage is
        # the taxonomy total by construction.
        {"n": "total"},
    ),
]

# The README domain table: | Domain | Types | Examples |
TABLE_FILE = "README.md"
TABLE_ROW_RE = re.compile(r"^\|\s*`(?P<domain>[a-z_]+)`\s*\|\s*(?P<n>\d+)\s*\|")
TABLE_HEADER_RE = re.compile(r"^\|\s*Domain\s*\|\s*Types\s*\|", re.IGNORECASE)

# The ARCHITECTURE inline breakdown: "N definitions across D domains (a: 1, b: 2, ...)"
INLINE_FILE = "docs/ARCHITECTURE.md"
INLINE_RE = re.compile(
    r"(?P<n>\d+) definitions across (?P<d>\d+) domains \((?P<breakdown>[^)]*)\)"
)
INLINE_PAIR_RE = re.compile(r"([a-z_]+)\s*:\s*(\d+)")

# The LOCALE_DETECTION coverage table: | <leaf type> | N (sample locales) | Source |
LOCALE_TABLE_FILE = "docs/LOCALE_DETECTION_ARCHITECTURE.md"
LOCALE_TABLE_HEADER_RE = re.compile(r"^\|\s*Type\s*\|\s*Locales\s*\|", re.IGNORECASE)
LOCALE_TABLE_ROW_RE = re.compile(r"^\|\s*(?P<leaf>[a-z_]+)\s*\|\s*(?P<n>\d+)\b")


# ══════════════════════════════════════════════════════════════════════════════
# SCOPE — what is scanned, what is exempt, what is knowingly out of scope
# ══════════════════════════════════════════════════════════════════════════════

def scanned_files(root: Path) -> list[Path]:
    """Living documentation: prose a reader takes as current fact."""
    files: list[Path] = []
    readme = root / "README.md"
    if readme.is_file():
        files.append(readme)
    files.extend(sorted((root / "docs").glob("*.md")))
    files.extend(sorted(root.glob("crates/*/README.md")))
    files.extend(sorted(root.glob("crates/*/src/lib.rs")))
    return [f for f in files if f.is_file()]


# Snapshots of a past measurement. Rewriting them would falsify the record, so
# their numbers are deliberately not gated.
SNAPSHOT_FILES = {
    "docs/release-notes-0.6.54.md": "release notes — a snapshot of one release",
    "docs/branch-ablation-m2v8m-s43.md": "ablation report — a snapshot of one run",
    "docs/compact-dmy-gate-coverage.md": "gate coverage report — a snapshot of one run",
    "docs/compact-dmy-mutation-matrix.md": "mutation matrix — a snapshot of one run",
    "docs/mechanism-attribution.md": "attribution report — a snapshot of one run",
    "docs/RELEASE.md": "release runbook — quotes numbers from specific past releases",
    "docs/plans": "dated plans — snapshots of intent at a date",
}

# Count-shaped phrases in scanned files that are NOT taxonomy sizes. Each entry
# is (path, substring found on the line, why it is out of scope). Being listed
# here asserts only that the number is not a taxonomy size — NOT that it is right.
ACKNOWLEDGED: list[tuple[str, str, str]] = [
    (
        "README.md",
        "99.9% actionability across 120 tested types",
        "actionability evaluation coverage — an eval run, not the taxonomy size",
    ),
    (
        "README.md",
        "values transformed across 120 types",
        "actionability evaluation coverage — an eval run, not the taxonomy size",
    ),
    (
        "docs/SENSE_AND_SHARPEN_PIPELINE.md",
        "trained to recognize 250 semantic types",
        "CharCNN output width — a model artifact, not the taxonomy",
    ),
    (
        "docs/SENSE_AND_SHARPEN_PIPELINE.md",
        "trying to predict 250 types from scratch",
        "CharCNN output width — a model artifact, not the taxonomy",
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        "schema.org (59 types) and DBpedia (122 types)",
        "external label spaces (GitTables annotations), not FineType's taxonomy",
    ),
    (
        "docs/TAXONOMY_COMPARISON.md",
        "~800 types + ~1,400 properties",
        "external type systems' own sizes, not FineType's taxonomy",
    ),
]


DISCOVERY_RES = [
    re.compile(
        r"(?<![\w.\-])\*{0,2}(?P<n>\d[\d,]*)\*{0,2}\s*\+?\s*"
        r"(?:\*\*)?(?:semantic |leaf |distinct |taxonomy |tested )?"
        r"(?:types?|definitions?)\b",
        re.IGNORECASE,
    ),
    re.compile(
        r"(?<![\w.\-])\*{0,2}(?P<n>\d[\d,]*|six|seven|eight|nine|ten)\*{0,2}\s+domains?\b",
        re.IGNORECASE,
    ),
]


def discovery_lines(path: Path, rel: str) -> list[tuple[int, str]]:
    """Lines carrying a count-shaped phrase. .rs files: module doc (`//!`) only."""
    hits = []
    for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if rel.endswith(".rs") and not line.lstrip().startswith("//!"):
            continue
        if any(rx.search(line) for rx in DISCOVERY_RES):
            hits.append((lineno, line))
    return hits


# ══════════════════════════════════════════════════════════════════════════════
# THE GATE
# ══════════════════════════════════════════════════════════════════════════════


def _expect(value: str, fact_key: str, facts: dict) -> tuple[bool, int, int]:
    got = WORD_NUMBERS.get(value.lower()) if not value.isdigit() else int(value)
    want = facts[fact_key]
    return (got == want, got if got is not None else -1, want)


def check(root: Path) -> list[str]:
    """Return a list of failures. Empty means the docs match the taxonomy."""
    facts = derive(root)
    failures: list[str] = []
    covered: set[tuple[str, int]] = set()

    def cover(rel: str, text: str, start: int) -> int:
        lineno = text.count("\n", 0, start) + 1
        covered.add((rel, lineno))
        return lineno

    # ── registered claims ────────────────────────────────────────────────────
    for rel, pattern, expects in CLAIMS:
        path = root / rel
        if not path.is_file():
            failures.append(f"{rel}: registered claim file is missing")
            continue
        text = path.read_text(encoding="utf-8")
        matches = list(re.finditer(pattern, text))
        if not matches:
            failures.append(
                f"{rel}: registered claim no longer matches — /{pattern}/\n"
                f"    The prose moved. Re-register it in {Path(__file__).name} "
                f"(a claim must not leave this gate silently)."
            )
            continue
        for match in matches:
            lineno = cover(rel, text, match.start())
            for group, fact_key in expects.items():
                ok, got, want = _expect(match.group(group), fact_key, facts)
                if not ok:
                    failures.append(
                        f"{rel}:{lineno}: documented {fact_key} is {got}, "
                        f"derived from labels/ it is {want}\n"
                        f"    {match.group(0).strip()}"
                    )

    # ── README per-domain table(s) ───────────────────────────────────────────
    table_path = root / TABLE_FILE
    if not table_path.is_file():
        failures.append(f"{TABLE_FILE}: missing — the per-domain table cannot be checked")
    else:
        tables = _find_tables(table_path, TABLE_HEADER_RE, TABLE_ROW_RE, "domain", covered, TABLE_FILE)
        if not tables:
            failures.append(
                f"{TABLE_FILE}: the per-domain table (a `| Domain | Types |` header) "
                f"is gone — restore it or drop it from this gate deliberately"
            )
        # Every such table, not just the first: a second one would otherwise be
        # a place to put a wrong breakdown that nothing reads.
        for header_line, rows in tables:
            failures.extend(
                _compare_breakdown(
                    f"{TABLE_FILE}:{header_line} per-domain table", rows, facts
                )
            )

    # ── ARCHITECTURE inline breakdown ────────────────────────────────────────
    inline_path = root / INLINE_FILE
    if not inline_path.is_file():
        failures.append(f"{INLINE_FILE}: missing — the inline breakdown cannot be checked")
    else:
        text = inline_path.read_text(encoding="utf-8")
        matches = list(INLINE_RE.finditer(text))
        if not matches:
            failures.append(
                f"{INLINE_FILE}: registered claim no longer matches — "
                f"/{INLINE_RE.pattern}/\n"
                f"    The prose moved. Re-register it in {Path(__file__).name}."
            )
        for match in matches:
            lineno = cover(INLINE_FILE, text, match.start())
            for group, fact_key in (("n", "total"), ("d", "domains")):
                ok, got, want = _expect(match.group(group), fact_key, facts)
                if not ok:
                    failures.append(
                        f"{INLINE_FILE}:{lineno}: documented {fact_key} is {got}, "
                        f"derived from labels/ it is {want}"
                    )
            rows = {d: int(n) for d, n in INLINE_PAIR_RE.findall(match.group("breakdown"))}
            failures.extend(
                _compare_breakdown(f"{INLINE_FILE}:{lineno} inline breakdown", rows, facts)
            )

    # ── LOCALE_DETECTION per-type coverage table ─────────────────────────────
    locale_path = root / LOCALE_TABLE_FILE
    if not locale_path.is_file():
        failures.append(f"{LOCALE_TABLE_FILE}: missing — the locale table cannot be checked")
    else:
        tables = _find_tables(
            locale_path, LOCALE_TABLE_HEADER_RE, LOCALE_TABLE_ROW_RE, "leaf",
            covered, LOCALE_TABLE_FILE,
        )
        if not tables:
            failures.append(
                f"{LOCALE_TABLE_FILE}: the coverage table (a `| Type | Locales |` "
                f"header) is gone — restore it or drop it from this gate deliberately"
            )
        derived_leaves = {key.rsplit(".", 1)[-1]: n for key, n in facts["locale_coverage"].items()}
        if len(derived_leaves) != len(facts["locale_coverage"]):
            failures.append(
                "two definitions with per-locale patterns share a leaf name; "
                "the locale table can no longer be matched by leaf"
            )
        for header_line, rows in tables:
            where = f"{LOCALE_TABLE_FILE}:{header_line} locale coverage table"
            missing = sorted(set(derived_leaves) - set(rows))
            extra = sorted(set(rows) - set(derived_leaves))
            if missing:
                failures.append(
                    f"{where}: no row for {', '.join(missing)} — "
                    f"it carries `validation_by_locale` patterns"
                )
            if extra:
                failures.append(
                    f"{where}: row for {', '.join(extra)}, which has no "
                    f"`validation_by_locale` block"
                )
            for leaf in sorted(set(rows) & set(derived_leaves)):
                if rows[leaf] != derived_leaves[leaf]:
                    failures.append(
                        f"{where}: `{leaf}` documented as {rows[leaf]} locales, "
                        f"derived from labels/ it is {derived_leaves[leaf]}"
                    )

    # ── fail-closed: no unregistered count claims ────────────────────────────
    for path in scanned_files(root):
        rel = path.relative_to(root).as_posix()
        if any(rel == s or rel.startswith(s + "/") for s in SNAPSHOT_FILES):
            continue
        for lineno, line in discovery_lines(path, rel):
            if (rel, lineno) in covered:
                continue
            if any(r == rel and sub in line for r, sub, _ in ACKNOWLEDGED):
                continue
            failures.append(
                f"{rel}:{lineno}: count-shaped claim is neither gated nor "
                f"acknowledged — is it a taxonomy size?\n"
                f"    {line.strip()[:160]}\n"
                f"    If yes, add it to CLAIMS in {Path(__file__).name}. If no "
                f"(a model class count, an eval figure), add it to ACKNOWLEDGED "
                f"with the reason."
            )

    return failures


def _find_tables(
    path: Path,
    header_re: re.Pattern,
    row_re: re.Pattern,
    key_group: str,
    covered: set,
    rel: str,
) -> list[tuple[int, dict[str, int]]]:
    """Every markdown table in `path` whose header matches, not just the first.

    Checking only the first would leave a second table as a place to put a wrong
    breakdown that nothing reads.
    """
    lines = path.read_text(encoding="utf-8").splitlines()
    tables: list[tuple[int, dict[str, int]]] = []
    for index, line in enumerate(lines):
        if not header_re.match(line):
            continue
        rows: dict[str, int] = {}
        for offset, row in enumerate(lines[index + 1 :], index + 2):
            if not row.startswith("|"):
                break
            match = row_re.match(row)
            if match:
                rows[match.group(key_group)] = int(match.group("n"))
                covered.add((rel, offset))
        tables.append((index + 1, rows))
    return tables


def _compare_breakdown(where: str, rows: dict[str, int], facts: dict) -> list[str]:
    """Compare a documented per-domain breakdown against the derived one.

    Row-by-row, not by total: two domains with swapped counts still sum to the
    right headline, and that must not pass.
    """
    failures = []
    derived = facts["per_domain"]
    missing = sorted(set(derived) - set(rows))
    extra = sorted(set(rows) - set(derived))
    if missing:
        failures.append(f"{where}: no row for {', '.join(missing)}")
    if extra:
        failures.append(f"{where}: row for {', '.join(extra)}, which is not a domain")
    for domain in sorted(set(rows) & set(derived)):
        if rows[domain] != derived[domain]:
            failures.append(
                f"{where}: `{domain}` documented as {rows[domain]}, "
                f"derived from labels/ it is {derived[domain]}"
            )
    if not missing and not extra:
        documented_total = sum(rows.values())
        if documented_total != facts["total"]:
            failures.append(
                f"{where}: rows sum to {documented_total}, "
                f"derived total is {facts['total']}"
            )
    return failures


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════

def _sub(path_rel: str, old: str, new: str, count: int = 1):
    def apply(root: Path):
        path = root / path_rel
        text = path.read_text(encoding="utf-8")
        if old not in text:
            raise Fatal(f"self-test: mutation target not found in {path_rel}: {old!r}")
        path.write_text(text.replace(old, new, count), encoding="utf-8")

    return apply


def _append(path_rel: str, extra: str):
    def apply(root: Path):
        path = root / path_rel
        path.write_text(path.read_text(encoding="utf-8") + extra, encoding="utf-8")

    return apply


def _drop_definition(path_rel: str, key: str):
    def apply(root: Path):
        path = root / path_rel
        lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
        out, dropping, found = [], False, False
        for line in lines:
            if line.startswith(key + ":"):
                dropping, found = True, True
                continue
            if dropping:
                if line.strip() and not line[0].isspace():
                    dropping = False
                else:
                    continue
            out.append(line)
        if not found:
            raise Fatal(f"self-test: {key} not found in {path_rel}")
        path.write_text("".join(out), encoding="utf-8")

    return apply


def self_test(root: Path) -> int:
    """Mutate a scratch copy of the tree and require each mutation to be caught.

    Each case names the substring its failure must contain, so a mutation caught
    for an unrelated reason does not count as detection.
    """
    facts = derive(root)
    per = facts["per_domain"]
    a, b = sorted(per)[0], sorted(per)[1]

    cases: list[tuple[str, object, str]] = [
        (
            "headline drifts from the data",
            _sub("README.md", f"taxonomy of {facts['total']} semantic types",
                 f"taxonomy of {facts['total'] - 1} semantic types"),
            "documented total is",
        ),
        (
            "one table row drifts",
            _sub("README.md", f"| `{a}` | {per[a]} |", f"| `{a}` | {per[a] + 3} |"),
            f"`{a}` documented as",
        ),
        (
            "two table rows swapped — the sum still matches the headline",
            lambda r: (
                _sub("README.md", f"| `{a}` | {per[a]} |", f"| `{a}` | {per[b]} |")(r),
                _sub("README.md", f"| `{b}` | {per[b]} |", f"| `{b}` | {per[a]} |")(r),
            ),
            f"`{a}` documented as {per[b]}",
        ),
        (
            "the inline breakdown drifts",
            _sub("docs/ARCHITECTURE.md", f"{a}: {per[a]}", f"{a}: {per[a] + 1}"),
            "inline breakdown",
        ),
        (
            "a definition is deleted and the docs are not updated",
            _drop_definition("labels/definitions_container.yaml", "container.key_value.query_string"),
            "documented total is",
        ),
        (
            "a definition is added and the docs are not updated",
            _append("labels/definitions_container.yaml",
                    "\ncontainer.object.selftest_probe:\n  designation: universal\n"),
            "documented total is",
        ),
        (
            "a locale-coverage table row drifts",
            _sub("docs/LOCALE_DETECTION_ARCHITECTURE.md",
                 f"| state_code | {facts['locales:geography.location.state_code']} ",
                 "| state_code | 9 "),
            "`state_code` documented as 9 locales",
        ),
        (
            "a locale is added to a definition and the docs are not updated",
            _sub("labels/definitions_geography.yaml",
                 "  validation_by_locale:\n    EN_US:\n",
                 "  validation_by_locale:\n    ZZ_SELFTEST:\n      type: string\n    EN_US:\n"),
            "locales, derived from labels/ it is",
        ),
        (
            "the README locale headline drifts from the table",
            _sub("README.md",
                 f"validates {facts['locales:geography.address.postal_code']} locales for postal codes",
                 "validates 12 locales for postal codes"),
            "documented locales:geography.address.postal_code is 12",
        ),
        (
            "a registered claim is reworded out of the gate",
            _sub("README.md", f"taxonomy of {facts['total']} semantic types",
                 "taxonomy of many semantic types"),
            "registered claim no longer matches",
        ),
        (
            "a SECOND per-domain table appears with wrong numbers",
            _append(
                "README.md",
                "\n| Domain | Types | Examples |\n|---|---|---|\n"
                + "".join(f"| `{d}` | {n + 1} | x |\n" for d, n in sorted(per.items())),
            ),
            "per-domain table",
        ),
        (
            "a brand-new unregistered claim appears",
            _append("docs/ARCHITECTURE.md", "\nFineType now ships 999 types.\n"),
            "neither gated nor acknowledged",
        ),
        (
            "a domain file is dropped from the embedded list",
            _sub(TAXONOMY_RS,
                 '        include_str!("../../../labels/definitions_container.yaml"),\n', ""),
            "labels/ and the embedded taxonomy disagree",
        ),
        (
            "a domain file is added but never embedded",
            lambda r: (r / "labels" / "definitions_selftest.yaml").write_text(
                "selftest.object.probe:\n  designation: universal\n", encoding="utf-8"
            ),
            "labels/ and the embedded taxonomy disagree",
        ),
        (
            "a definitions file grows a line this scanner cannot parse",
            _append("labels/definitions_container.yaml", "\nnot a definition key\n"),
            "refusing to guess a count",
        ),
        (
            "the same definition is declared twice",
            _append("labels/definitions_container.yaml",
                    "\ncontainer.key_value.query_string:\n  designation: universal\n"),
            "duplicate definition",
        ),
    ]

    print(f"self-test: derived total={facts['total']} domains={facts['domains']}")

    failed = 0
    with tempfile.TemporaryDirectory(prefix="doc-taxonomy-counts-") as tmp:
        pristine = Path(tmp) / "pristine"
        _stage(root, pristine)

        # Control: the unmutated tree must pass, or every "caught" below is noise.
        control = check(pristine)
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
                found = check(work)
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
    """Copy just the files this gate reads into a scratch tree."""
    wanted = [root / TAXONOMY_RS]
    wanted += sorted((root / "labels").glob("definitions_*.yaml"))
    wanted += scanned_files(root)
    for src in wanted:
        rel = src.relative_to(root)
        target = dest / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, target)


# ══════════════════════════════════════════════════════════════════════════════

def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--facts", action="store_true", help="print the derived numbers")
    parser.add_argument("--self-test", action="store_true", help="prove the gate detects")
    args = parser.parse_args(argv)
    root = args.root.resolve()

    if args.self_test:
        try:
            return self_test(root)
        except Fatal as exc:
            print(f"FATAL: {exc}", file=sys.stderr)
            return 2

    try:
        facts = derive(root)
    except Fatal as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if args.facts:
        print(f"total                {facts['total']}")
        print(f"domains              {facts['domains']}")
        print(f"domain_files         {facts['domain_files']}")
        print(f"locale_specific      {facts['locale_specific']}")
        print(f"validation_by_locale {facts['validation_by_locale']}")
        for domain in sorted(facts["per_domain"]):
            print(f"  {domain:<16} {facts['per_domain'][domain]}")
        return 0

    try:
        failures = check(root)
    except Fatal as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if failures:
        print(
            f"Documented taxonomy counts disagree with labels/definitions_*.yaml "
            f"({facts['total']} definitions across {facts['domains']} domains):\n",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(
            f"\n{len(failures)} problem(s). Derived numbers: "
            f"scripts/{Path(__file__).name} --facts",
            file=sys.stderr,
        )
        return 1

    print(
        f"Taxonomy counts in the docs match labels/definitions_*.yaml: "
        f"{facts['total']} definitions across {facts['domains']} domains "
        f"({', '.join(f'{d} {n}' for d, n in sorted(facts['per_domain'].items()))})."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
