#!/usr/bin/env python3
"""Gate every published `samples:` value against its own leaf's `validation.pattern`.

`samples:` is a publication surface. `crates/finetype-mcp/src/json_schema.rs`,
`crates/finetype-mcp/src/resources.rs` and `crates/finetype-cli/src/cmd_taxonomy.rs`
all read it, which is how a sample reaches the type registry, the MCP resources
and the published taxonomy. `validation.pattern` is published beside it and
describes the same type. When the two contradict each other, one of them is a
wrong claim about a data type, shipped to a stranger.

Nothing looked at that. `finetype check` validates the values the GENERATOR
produces, never the static list — 251 definitions x the 50-value `--samples`
default is the `12550/12550` it reports, and none of those 12,550 values comes
from `labels/`. `scripts/check_taxonomy_content.py` reads `samples:` but declines
non-checksum leaves by documented scope. So a sample of `"ABC"` under a leaf
whose own pattern is `^[0-9]+$` left every gate green.

WHAT IS DERIVED (the source of truth)
    `target/release/validate-samples`, the oracle in `finetype-build-tools` that
    `make build-extension` already builds. It reads `labels/` with the product's
    own `Taxonomy::from_directory` and applies the pattern with the product's own
    `CompiledValidator`, so YAML escape decoding and the regex dialect are the
    shipped ones. Neither is reimplemented here — a Python `re` reimplementation
    would refuse the ten patterns in `labels/` that use `\\p{...}`, which `re`
    cannot compile, and would answer for its own dialect on the two that use
    lookaround.

    `scripts/check_taxonomy_content.py`'s reader, for the file and line each
    sample sits on. The oracle answers WHAT is wrong; that reader answers WHERE.

WHAT IS ASSERTED
    1. Every string sample under a leaf carrying `validation.pattern` satisfies
       that pattern, unless the pair is listed in PUBLISHED_CONTRADICTIONS below.
    2. A `validation.pattern` compiles. `Taxonomy::compile_validators` drops a
       validator that does not with `.ok()`, so an uncompilable pattern silently
       validates nothing rather than failing.
    3. A leaf carrying `validation.pattern` publishes at least one sample. A
       pattern with nothing to exercise it passes this gate vacuously.
    4. A PUBLISHED_CONTRADICTIONS entry still contradicts. An entry whose sample
       now matches, or whose sample is gone, fails: a quarantine that has stopped
       being true is a false statement in the tree.
    5. The two readings of `labels/` agree — same leaf keys, same sample count
       per leaf. serde_yaml's `HashMap` silently keeps the last of two leaves
       with the same key; the line reader would see both.

WHAT IS *NOT* ASSERTED, AND THE CROSS-LEAF DECISION
    That a sample belongs to the leaf it sits under. This gate reports a sample
    its OWN leaf rejects. It deliberately does NOT report a sample that a
    DIFFERENT leaf's pattern would also accept, because that is a judgement and
    not a violation: patterns here are shape, and a shape-only pattern "confirms
    90% of random input" (`Validation::is_precise`, crates/finetype-core/src/
    taxonomy.rs). Eight patterns in `labels/` are shared BYTE-FOR-BYTE by two or
    three leaves each — `^[0-9]+$` by `representation.identifier.increment` and
    `representation.identifier.numeric_code`, `^\\d{2}/\\d{2}/\\d{4}$` by
    `datetime.date.mdy_slash` and `datetime.date.dmy_slash` — so for those pairs
    no pattern check can tell the two apart even in principle. The self-test pins
    that decision rather than leaving it to prose: `a sample is swapped for one
    another leaf with the identical pattern publishes` is a case that must stay
    GREEN.

    What does answer the cross-leaf question: the `checksum:` and `membership:`
    directives and their substance guards (a value of the right shape but the
    wrong check digit or outside the published code list is not that type), the
    check-digit half of `scripts/check_taxonomy_content.py`, and human review of
    the leaf. Not this.

    That `minLength`, `maxLength`, `enum`, `minimum` or `maximum` hold. The
    oracle strips them before building the validator so a verdict is
    attributable to the pattern alone.

    Anything about `validation_by_locale`. The base `validation` is the universal
    fallback `Taxonomy::get_validator` reaches for, and on the six leaves that
    carry per-locale schemas it is the superset.

USAGE
    scripts/check_sample_patterns.py                # gate the working tree
    scripts/check_sample_patterns.py --self-test    # prove the gate detects
    scripts/check_sample_patterns.py --oracle PATH

Needs `make build-extension` (or `cargo build -p finetype-build-tools --release`)
for the oracle. It needs no model, no duckdb and no network.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_taxonomy_content as reader  # noqa: E402

DEFAULT_ORACLE = "target/release/validate-samples"
LABELS_DIR = "labels"

# ── The contradictions this gate found on its first run ───────────────────────
#
# Each entry is a leaf, a sample it publishes, and the pattern that leaf
# publishes beside it. The two contradict each other. WHICH SIDE IS WRONG IS A
# TAXONOMY JUDGEMENT AND THIS GATE DOES NOT MAKE IT: `::1` is a real IPv6 address
# and the pattern that rejects it is the defect, while `1A1z7agoat2GPFH7pPPPP...`
# is a truncated placeholder and the sample is the defect. Both need a decision
# about the type, made by someone who owns the type.
#
# The list is a ratchet, not an exemption:
#   - a contradiction NOT on this list fails the gate;
#   - an entry on this list that has stopped contradicting fails the gate, so a
#     fix must delete its line in the same change;
#   - the match is on the exact (leaf, value) pair, so editing a quarantined
#     sample to a different wrong value fails the gate.
#
# Found 2026-08-06 by this gate's first run over 954 samples across 251 leaves.
PUBLISHED_CONTRADICTIONS: tuple[tuple[str, str, str], ...] = (
    (
        "finance.crypto.bitcoin_address",
        "1A1z7agoat2GPFH7pPPPP...",
        "a truncated placeholder: the trailing `...` cannot match any address "
        "pattern, and the value is not a real address either",
    ),
    (
        "finance.currency.amount_apostrophe",
        "-CHF 999.99",
        "the pattern puts the sign after the currency code (`(CHF\\s?)?-?`), so "
        "a leading minus is unmatched",
    ),
    (
        "finance.currency.amount_code_prefix",
        "EUR 1.234,56",
        "a European-format decimal under a pattern whose decimal separator is "
        "`\\.` only",
    ),
    (
        "geography.coordinate.plus_code",
        "7FG49W00+2V",
        "`0` is the Open Location Code padding character and is not in the "
        "pattern's 20-character alphabet",
    ),
    (
        "identity.person.weight",
        "170 pounds",
        "the pattern's unit alternation is `kg|lbs|lb|g|oz|stones` and does not "
        "include `pounds`",
    ),
    (
        "representation.format.color_rgb",
        "rgba(0, 0, 255, 0.5)",
        "the pattern has no `rgba` head and no alpha component",
    ),
    (
        "technology.internet.ip_v6",
        "2001:db8:85a3::8a2e:370:7334",
        "`::` zero compression, under a pattern that requires all eight groups "
        "written out",
    ),
    (
        "technology.internet.ip_v6",
        "::1",
        "the loopback address in its only legal form, under the same "
        "all-eight-groups pattern",
    ),
    (
        "technology.internet.ip_v6",
        "fe80::1",
        "a link-local address in compressed form, under the same "
        "all-eight-groups pattern",
    ),
)


class Fatal(Exception):
    """The gate cannot answer, which is not the same as answering `pass`."""


@dataclass
class Record:
    """One verdict from the oracle."""

    leaf: str
    index: int
    verdict: str
    value: str


def run_oracle(oracle: Path, labels_dir: Path) -> list[Record]:
    """Every (leaf, sample) verdict the oracle has, in its order."""
    if not oracle.exists():
        raise Fatal(
            f"{oracle} not found. Build it with `make build-extension`, or "
            f"`cargo build -p finetype-build-tools --release`."
        )
    try:
        result = subprocess.run(
            [str(oracle), "--labels", str(labels_dir)],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        raise Fatal(f"{oracle} could not be run: {exc}") from exc
    if result.returncode != 0:
        raise Fatal(
            f"{oracle.name} exited {result.returncode} over {labels_dir}: "
            f"{result.stderr.strip()}"
        )

    records: list[Record] = []
    for lineno, line in enumerate(result.stdout.splitlines(), 1):
        fields = line.split("\t")
        if len(fields) != 4:
            raise Fatal(
                f"{oracle.name} line {lineno} is not `leaf<TAB>index<TAB>"
                f"verdict<TAB>json`: {line!r}"
            )
        leaf, index, verdict, encoded = fields
        try:
            value = json.loads(encoded)
        except json.JSONDecodeError as exc:
            raise Fatal(
                f"{oracle.name} line {lineno} carries an unreadable value: {exc}"
            ) from exc
        if not isinstance(value, str):
            raise Fatal(f"{oracle.name} line {lineno} carries a non-string value")
        records.append(Record(leaf=leaf, index=int(index), verdict=verdict, value=value))
    if not records:
        raise Fatal(
            f"{oracle.name} reported no samples at all over {labels_dir} — the "
            f"taxonomy is empty, or the oracle is broken"
        )
    return records


def _cross_check(records: list[Record], leaves: list[reader.Leaf]) -> None:
    """Refuse when the oracle and the line reader disagree about `labels/`.

    Two independent readings of the same files. `serde_yaml` collects into a
    `HashMap`, so two leaves with the same key leave only the last one; the line
    reader would see both. A disagreement means one of them is not reading what
    ships, and neither verdict can be trusted until it is settled.
    """
    from_oracle: dict[str, int] = {}
    for record in records:
        # Index -1 is the leaf-level record a leaf with no samples gets. It
        # carries the leaf into the key comparison and counts as no samples.
        from_oracle[record.leaf] = from_oracle.get(record.leaf, 0) + (
            1 if record.index >= 0 else 0
        )
    from_lines: dict[str, int] = {}
    for leaf in leaves:
        from_lines[leaf.key] = from_lines.get(leaf.key, 0) + len(leaf.samples)

    only_oracle = sorted(set(from_oracle) - set(from_lines))
    only_lines = sorted(set(from_lines) - set(from_oracle))
    if only_oracle or only_lines:
        raise Fatal(
            f"the two readings of {LABELS_DIR}/ disagree about which leaves "
            f"exist: {len(only_oracle)} only the oracle sees "
            f"({', '.join(only_oracle[:3]) or '-'}), {len(only_lines)} only the "
            f"line reader sees ({', '.join(only_lines[:3]) or '-'}). A leaf key "
            f"defined twice is the usual cause: serde_yaml keeps the last one."
        )
    for key in sorted(from_oracle):
        if from_oracle[key] != from_lines[key]:
            raise Fatal(
                f"the two readings of {LABELS_DIR}/ disagree about `{key}`: the "
                f"oracle sees {from_oracle[key]} samples, the line reader sees "
                f"{from_lines[key]}. A leaf key defined twice is the usual "
                f"cause: serde_yaml keeps the last one, the line reader keeps "
                f"both."
            )


def check(labels_dir: Path, oracle: Path) -> list[str]:
    """Every way the published samples and the published patterns contradict."""
    records = run_oracle(oracle, labels_dir)
    leaves = reader.read_definitions(labels_dir)
    _cross_check(records, leaves)

    where: dict[tuple[str, int], tuple[str, int]] = {}
    leaf_site: dict[str, tuple[str, int]] = {}
    for leaf in leaves:
        leaf_site[leaf.key] = (leaf.rel, leaf.line)
        for index, sample in enumerate(leaf.samples):
            where[(leaf.key, index)] = (leaf.rel, sample.line)
    patterns = _patterns(leaves)

    quarantined = {(leaf, value) for leaf, value, _ in PUBLISHED_CONTRADICTIONS}
    still_contradicting: set[tuple[str, str]] = set()
    failures: list[str] = []
    patterned: set[str] = set()

    for record in records:
        fallback = leaf_site.get(record.leaf, (f"{LABELS_DIR}/?", 0))
        rel, line = where.get((record.leaf, record.index), fallback)
        site = f"{rel}:{line}"
        if record.verdict in ("no-pattern", "no-samples-no-pattern"):
            continue
        patterned.add(record.leaf)
        if record.verdict == "no-samples":
            failures.append(
                f"{site}: `{record.leaf}` carries a `validation.pattern` and "
                f"publishes no `samples:` value, so nothing exercises it"
            )
            continue
        if record.verdict == "ok":
            continue
        if record.verdict == "bad-pattern":
            failures.append(
                f"{site}: `{record.leaf}` publishes a `validation.pattern` that "
                f"does not compile, so the product drops the validator and "
                f"checks nothing: {patterns.get(record.leaf, '?')}"
            )
            continue
        if record.verdict == "non-string":
            failures.append(
                f"{site}: `{record.leaf}` publishes a sample that is not a "
                f"string, and a pattern applies to strings. Quote it."
            )
            continue
        if record.verdict != "fail":
            raise Fatal(
                f"{oracle.name} returned an unknown verdict {record.verdict!r} "
                f"for `{record.leaf}`"
            )
        if (record.leaf, record.value) in quarantined:
            still_contradicting.add((record.leaf, record.value))
            continue
        failures.append(
            f"{site}: `{record.leaf}` publishes {record.value!r} as a sample, "
            f"and its own `validation.pattern` rejects it: "
            f"{patterns.get(record.leaf, '?')}"
        )

    for leaf, value, _ in PUBLISHED_CONTRADICTIONS:
        if (leaf, value) not in still_contradicting:
            failures.append(
                f"scripts/{Path(__file__).name}: PUBLISHED_CONTRADICTIONS lists "
                f"{value!r} under `{leaf}` as contradicting its own pattern, and "
                f"it no longer does — the sample was fixed, the pattern was "
                f"widened, or the sample is gone. Delete the entry."
            )

    if not patterned:
        raise Fatal(
            f"not one leaf in {LABELS_DIR}/ carries a `validation.pattern`, so "
            f"this gate asserted nothing"
        )
    return failures


def _patterns(leaves: list[reader.Leaf]) -> dict[str, str]:
    """Each leaf's `validation.pattern` as it is WRITTEN, for the message.

    Read off the line, not decoded: this is quoted back to a reader who is about
    to open the file, so the text they will see there is the useful text. The
    oracle decides validity; nothing here does.
    """
    found: dict[str, str] = {}
    for leaf in leaves:
        lines = leaf.path.read_text(encoding="utf-8").splitlines()
        in_validation = False
        for line in lines[leaf.line :]:
            if line and not line.startswith(" "):
                break
            if line.startswith("  ") and not line.startswith("   "):
                in_validation = line.strip() == "validation:"
                continue
            if in_validation and line.startswith("    pattern:"):
                found[leaf.key] = line.split(":", 1)[1].strip()
        # A leaf whose `validation:` block sits below another leaf's lines is not
        # reachable here; `leaf.line` is where its own key is.
    return found


# ══════════════════════════════════════════════════════════════════════════════
# SELF-TEST — a gate that is only known to pass is not known to detect
# ══════════════════════════════════════════════════════════════════════════════

# A value chosen to be outside every pattern in `labels/`: it opens with an
# underscore, carries a character no class in the tree names, and no pattern is
# unanchored. The self-test does not TRUST that — it requires the gate to redden,
# so a taxonomy that grew a pattern accepting this reports a MISS rather than
# quietly weakening the case.
SENTINEL = "__NO_LEAF_ACCEPTS_THIS_☃__"


def _edit_line(path: Path, lineno: int, old: str, new: str) -> None:
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    if old not in lines[lineno - 1]:
        raise Fatal(f"self-test: {old!r} is not on {path.name}:{lineno}")
    lines[lineno - 1] = lines[lineno - 1].replace(old, new, 1)
    path.write_text("".join(lines), encoding="utf-8")


def _patterned_leaves(labels_dir: Path) -> list[reader.Leaf]:
    """Leaves carrying a `validation.pattern`, in file order."""
    leaves = reader.read_definitions(labels_dir)
    patterns = _patterns(leaves)
    found = [leaf for leaf in leaves if leaf.key in patterns and leaf.samples]
    if not found:
        raise Fatal("self-test: no leaf carries both a pattern and a sample")
    return found


def _pattern_line(leaf: reader.Leaf) -> int:
    lines = leaf.path.read_text(encoding="utf-8").splitlines()
    in_validation = False
    for offset, line in enumerate(lines[leaf.line :], leaf.line + 1):
        if line and not line.startswith(" "):
            break
        if line.startswith("  ") and not line.startswith("   "):
            in_validation = line.strip() == "validation:"
            continue
        if in_validation and line.startswith("    pattern:"):
            return offset
    raise Fatal(f"self-test: `{leaf.key}` has no pattern line")


def _quarantined_sample(labels_dir: Path) -> tuple[reader.Leaf, reader.Sample, str]:
    """A quarantined (leaf, value) pair, with a sibling sample that passes.

    Derived rather than named, so the case follows the list rather than pinning
    the taxonomy as it stood when the list was written.
    """
    quarantined = {(leaf, value) for leaf, value, _ in PUBLISHED_CONTRADICTIONS}
    for leaf in reader.read_definitions(labels_dir):
        hits = [s for s in leaf.samples if (leaf.key, s.value) in quarantined]
        clean = [s for s in leaf.samples if (leaf.key, s.value) not in quarantined]
        if hits and clean:
            return leaf, hits[0], clean[0].value
    raise Fatal("self-test: no quarantined sample sits beside a passing one")


def _replace_sample(picker: Callable[[list[reader.Leaf]], reader.Leaf], value: str):
    """Rewrite one sample of the picked leaf."""

    def apply(labels_dir: Path) -> None:
        leaf = picker(_patterned_leaves(labels_dir))
        sample = leaf.samples[0]
        _edit_line(leaf.path, sample.line, sample.value, value)

    return apply


def _replace_pattern(replacement: str):
    def apply(labels_dir: Path) -> None:
        leaf = _patterned_leaves(labels_dir)[0]
        lineno = _pattern_line(leaf)
        lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
        line = lines[lineno - 1]
        indent = line[: len(line) - len(line.lstrip())]
        lines[lineno - 1] = f'{indent}pattern: "{replacement}"\n'
        leaf.path.write_text("".join(lines), encoding="utf-8")

    return apply


def _drop_samples(labels_dir: Path) -> None:
    leaf = _patterned_leaves(labels_dir)[0]
    lines = leaf.path.read_text(encoding="utf-8").splitlines(keepends=True)
    drop = {sample.line for sample in leaf.samples}
    kept = [line for index, line in enumerate(lines, 1) if index not in drop]
    leaf.path.write_text("".join(kept), encoding="utf-8")


def _edit_quarantined(labels_dir: Path) -> None:
    leaf, sample, _ = _quarantined_sample(labels_dir)
    _edit_line(leaf.path, sample.line, sample.value, sample.value + "_EDITED")


def _fix_quarantined(labels_dir: Path) -> None:
    leaf, sample, replacement = _quarantined_sample(labels_dir)
    _edit_line(leaf.path, sample.line, sample.value, replacement)


def _duplicate_leaf_key(labels_dir: Path) -> None:
    """Give one leaf the key of another, so serde_yaml keeps only one of them."""
    leaves = reader.read_definitions(labels_dir)
    by_file: dict[Path, list[reader.Leaf]] = {}
    for leaf in leaves:
        by_file.setdefault(leaf.path, []).append(leaf)
    for path, group in sorted(by_file.items()):
        if len(group) >= 2:
            _edit_line(path, group[1].line, group[1].key + ":", group[0].key + ":")
            return
    raise Fatal("self-test: no definitions file holds two leaves")


def _twin_leaves(labels_dir: Path) -> tuple[reader.Leaf, reader.Leaf]:
    """Two leaves whose `validation.pattern` is byte-for-byte the same.

    The cross-leaf case: no pattern check can tell these two apart, which is why
    the swap below has to stay GREEN.
    """
    leaves = reader.read_definitions(labels_dir)
    patterns = _patterns(leaves)
    by_pattern: dict[str, list[reader.Leaf]] = {}
    for leaf in leaves:
        if leaf.key in patterns and leaf.samples:
            by_pattern.setdefault(patterns[leaf.key], []).append(leaf)
    for _, group in sorted(by_pattern.items(), key=lambda item: item[0]):
        if len(group) >= 2:
            return group[0], group[1]
    raise Fatal("self-test: no two leaves share a pattern")


def _swap_across_twins(labels_dir: Path) -> None:
    donor, recipient = _twin_leaves(labels_dir)
    _edit_line(
        recipient.path,
        recipient.samples[0].line,
        recipient.samples[0].value,
        donor.samples[0].value,
    )


def _spoil_unpatterned_sample(labels_dir: Path) -> None:
    leaves = reader.read_definitions(labels_dir)
    patterns = _patterns(leaves)
    for leaf in leaves:
        if leaf.key not in patterns and leaf.samples:
            _edit_line(leaf.path, leaf.samples[0].line, leaf.samples[0].value, SENTINEL)
            return
    raise Fatal("self-test: every leaf with a sample carries a pattern")


def self_test(root: Path, oracle: Path) -> int:
    """Mutate a pristine copy of labels/ and require each case to land."""
    must_redden: list[tuple[str, Callable[[Path], None], str]] = [
        (
            "the first leaf's sample is replaced with a value its own pattern rejects",
            _replace_sample(lambda leaves: leaves[0], SENTINEL),
            "its own `validation.pattern` rejects it",
        ),
        (
            "...and the middle leaf's",
            _replace_sample(lambda leaves: leaves[len(leaves) // 2], SENTINEL),
            "its own `validation.pattern` rejects it",
        ),
        (
            "...and the last leaf's",
            _replace_sample(lambda leaves: leaves[-1], SENTINEL),
            "its own `validation.pattern` rejects it",
        ),
        (
            "a pattern is tightened until its own published samples fail it",
            _replace_pattern("^__NOTHING_MATCHES_THIS__$"),
            "its own `validation.pattern` rejects it",
        ),
        (
            "a pattern no longer compiles",
            _replace_pattern("^[0-9"),
            "does not compile, so the product drops the validator",
        ),
        (
            "a leaf with a pattern loses every sample",
            _drop_samples,
            "publishes no `samples:` value, so nothing exercises it",
        ),
        (
            "a quarantined sample is edited to a different wrong value",
            _edit_quarantined,
            "its own `validation.pattern` rejects it",
        ),
        (
            "a quarantined sample is corrected and its entry left in place",
            _fix_quarantined,
            "it no longer does",
        ),
        (
            "a leaf key is duplicated, so serde_yaml keeps only one of the two",
            _duplicate_leaf_key,
            f"the two readings of {LABELS_DIR}/ disagree about",
        ),
    ]

    # Two cases that must NOT redden. Without them this gate could be reporting
    # every sample it does not recognise and nobody would find out until the
    # first honest one was refused.
    must_stay_green: list[tuple[str, Callable[[Path], None]]] = [
        (
            "a sample is swapped for one another leaf with the IDENTICAL pattern "
            "publishes — the cross-leaf case, which is a judgement and not a "
            "violation",
            _swap_across_twins,
        ),
        (
            "a leaf with no `validation.pattern` publishes a nonsense sample — "
            "out of this gate's declared scope",
            _spoil_unpatterned_sample,
        ),
    ]

    print(f"self-test: oracle {oracle.name}, {len(PUBLISHED_CONTRADICTIONS)} quarantined")

    failed = 0
    with tempfile.TemporaryDirectory(prefix="sample-patterns-selftest-") as tmp:
        pristine = Path(tmp) / "pristine" / LABELS_DIR
        shutil.copytree(root / LABELS_DIR, pristine)

        control = check(pristine, oracle)
        if control:
            print("  CONTROL FAILED — the unmutated tree does not pass:")
            for failure in control:
                print(f"      {failure}")
            return 1
        print("  ok   control: unmutated tree passes")

        for name, mutate, expected in must_redden:
            found, text = _run_case(tmp, pristine, oracle, mutate)
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

        for name, mutate in must_stay_green:
            found, text = _run_case(tmp, pristine, oracle, mutate)
            if found:
                print(f"  FALSE POSITIVE {name}")
                for line in text.splitlines()[:6]:
                    print(f"      got: {line}")
                failed += 1
            else:
                print(f"  ok   stays green: {name}")

    total = len(must_redden) + len(must_stay_green)
    if failed:
        print(f"\nself-test FAILED: {failed} of {total} cases did not land")
        return 1
    print(
        f"\nself-test passed: {len(must_redden)} mutations detected, "
        f"{len(must_stay_green)} left green, control clean"
    )
    return 0


def _run_case(
    tmp: str, pristine: Path, oracle: Path, mutate: Callable[[Path], None]
) -> tuple[list[str], str]:
    work = Path(tmp) / "work"
    if work.exists():
        shutil.rmtree(work)
    shutil.copytree(pristine.parent, work)
    try:
        mutate(work / LABELS_DIR)
        found = check(work / LABELS_DIR, oracle)
    except (Fatal, reader.Fatal) as exc:
        found = [str(exc)]
    return found, "\n".join(found)


# ══════════════════════════════════════════════════════════════════════════════


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parent.parent
    )
    parser.add_argument(
        "--oracle",
        type=Path,
        default=None,
        help=f"the pattern oracle (default: {DEFAULT_ORACLE})",
    )
    parser.add_argument(
        "--self-test", action="store_true", help="prove the gate detects"
    )
    args = parser.parse_args(argv)
    root = args.root.resolve()
    oracle = (args.oracle or (root / DEFAULT_ORACLE)).resolve()

    if args.self_test:
        try:
            return self_test(root, oracle)
        except (Fatal, reader.Fatal) as exc:
            print(f"FATAL: {exc}", file=sys.stderr)
            return 2

    try:
        failures = check(root / LABELS_DIR, oracle)
    except (Fatal, reader.Fatal) as exc:
        print(f"FATAL: {exc}", file=sys.stderr)
        return 2

    if failures:
        print(
            "The taxonomy publishes samples its own patterns reject:\n",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        print(f"\n{len(failures)} problem(s).", file=sys.stderr)
        return 1

    records = run_oracle(oracle, root / LABELS_DIR)
    samples = [r for r in records if r.index >= 0]
    judged = [r for r in samples if r.verdict != "no-pattern"]
    leaves_judged = {r.leaf for r in judged}
    print(
        f"Every published sample satisfies its own leaf's pattern: "
        f"{len(judged)} samples across {len(leaves_judged)} leaves with a "
        f"`validation.pattern`, {len(samples) - len(judged)} samples under "
        f"leaves with none, and {len(PUBLISHED_CONTRADICTIONS)} known "
        f"contradictions quarantined."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
