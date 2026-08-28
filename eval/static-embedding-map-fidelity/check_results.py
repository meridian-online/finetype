#!/usr/bin/env python3
"""Refuse a map-fidelity results file whose floors have stopped behaving.

WHY THIS EXISTS
    `map_fidelity.py` compares embedders against two floors: `random-384`, which
    embeds nothing, and `bm25`, which is what an analyst has without a model. A
    result that shows no improvement over MiniLM and a harness that has stopped
    measuring produce the SAME SHAPE of table -- a column of numbers that are
    close together -- and the difference between them is the whole value of the
    file. The floors are what tell them apart, and a floor is only evidence if
    something refuses the file when it drifts.

    That refusal has to be mechanical because the failure is quiet. A tokenizer
    change that maps every input to the unknown token, a projection that
    collapses, a corruption function that stops corrupting: each leaves a
    plausible file. None of them crashes.

WHAT IT REFUSES
    Read the `RULES` list rather than this paragraph -- it is the enumeration and
    this is a summary of it. In outline: a control that has climbed off its
    floor, a ceiling that no longer separates from that floor, a pair
    construction whose positive rate is not what the file says, a corruption
    function that returned the input, a model with no recorded licence, and an
    arm set that differs between corpora, which is how two passes get written
    into one file.

    Each floor is checked against ITS OWN value, which is not zero for two of
    them. AMI and map overlap floor at ~0 because noise has no structure. Pooled
    average precision floors at the positive rate, which the file declares. A
    single "near zero" rule would pass a broken pairwise arm and fail a working
    one.

WHAT IT DOES NOT DO
    It does not re-run the measurement and cannot: that needs five models, a
    corpus download and roughly an hour. It reads the committed artefact. So it
    catches a file that is internally impossible, not a file that is merely
    wrong -- for that, re-run `map_fidelity.py`.

USAGE
    eval/static-embedding-map-fidelity/check_results.py [results.json]
    eval/static-embedding-map-fidelity/check_results.py --self-test

EXIT CODES
    0  clean
    1  findings -- a rule refused the file, or a self-test mutation did not redden
    2  the tool could not run: the file is absent or is not the expected shape
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path
from typing import Any, Callable

DEFAULT_RESULTS = Path(__file__).resolve().parent / "results.json"
DEFAULT_FINDINGS = Path(__file__).resolve().parent / "FINDINGS.md"

# FINDINGS.md carries the table between these markers and nothing hand-edits it.
# Prose that quotes a measurement is a claim nothing reddens when it rots, and
# the specific way it rots here is silent: a re-run writes new numbers into
# results.json, the prose keeps the old ones, and the two read identically.
TABLE_START = "<!-- generated table: check_results.py --emit-table -->"
TABLE_END = "<!-- end generated table -->"

CONTROL = "random-384"
CEILING = "minilm"
LEXICAL = "bm25"

# How far the noise control may sit from a floor before the file is refused.
# AMI and map overlap floor at zero; pooled average precision floors at the
# declared positive rate. Both tolerances are wide enough for sampling noise at
# the sample sizes this harness runs and far narrower than any real signal --
# the smallest real signal in the committed file is an order of magnitude out.
AMI_TOL = 0.06
OVERLAP_MAX = 0.20
AP_TOL = 0.08

# How far the ceiling must sit above the floor for the measure to be said to
# separate anything at all. Not a quality bar: a measure that cannot put MiniLM
# clear of Gaussian noise is not measuring, whatever it reports for the arms in
# between.
SEPARATION = 0.10

# A corruption function that returns its input turns near-duplicate detection
# into string equality, which every arm solves. Above this share of probes the
# arm is not measuring what it is named for.
UNCHANGED_MAX = 0.20


class Findings(list):
    """Findings tagged with the rule that raised them.

    The tag is derived from the rule function's name by `check`, never typed at
    the call site. A tag typed beside each finding is a second name for the same
    thing, and the two drift: the first version of this file had a rule called
    `control_on_its_floor` emitting findings tagged `control_floor`, so the
    coverage check below -- which reads function names -- could not see that the
    rule had a case. Nothing here can now disagree with anything else here.
    """

    def __init__(self) -> None:
        super().__init__()
        self.rule = "?"

    def add(self, message: str) -> None:
        self.append(f"{self.rule}: {message}")


def emit_table(payload: dict) -> str:
    """The whole measurement as one table, generated from the results file.

    Every figure FINDINGS.md states sits here, so a number in that document is
    checked rather than remembered. `rule_findings_table` compares the two, which
    is why this returns a string instead of printing.
    """
    def cell(v: object, places: int = 4) -> str:
        if v is None:
            return "--"
        return f"{v:.{places}f}" if isinstance(v, float) else str(v)

    head = ("| corpus | arm | AMI (map) | retention | kNN overlap | P@k | lift | AP same-class | AP near-dup |\n"
            "|---|---|---|---|---|---|---|---|---|")
    lines = [head]
    for corpus in payload["results"]:
        for arm in corpus["embedders"]:
            lines.append(
                f"| {corpus['corpus']} | `{arm['embedder']}` | "
                f"{cell(arm['ami_map'])} | {cell(arm['retention_vs_minilm'], 3)} | "
                f"{cell(arm['map_overlap_with_minilm'])} | {cell(arm['precision_at_k'])} | "
                f"{cell(arm['lift_over_random'], 3)} | {cell(arm['pairwise_ap_same_class'])} | "
                f"{cell(arm['pairwise_ap_near_duplicate'])} |"
            )
    return "\n".join(lines)


def extract_table(findings: str) -> str | None:
    start = findings.find(TABLE_START)
    end = findings.find(TABLE_END)
    if start < 0 or end < 0 or end < start:
        return None
    return findings[start + len(TABLE_START):end].strip()


def _arms(corpus: dict) -> dict[str, dict]:
    return {a["embedder"]: a for a in corpus["embedders"]}


def rule_required_arms(payload: dict, f: Findings) -> None:
    """Both floors and the ceiling are present in every corpus."""
    for corpus in payload["results"]:
        arms = _arms(corpus)
        for want in (CONTROL, CEILING, LEXICAL):
            if want not in arms:
                f.add(f"{corpus['corpus']} has no {want} arm")


def rule_one_pass(payload: dict, f: Findings) -> None:
    """Every corpus carries the same arms, in the same order.

    Two passes merged into one file show up here first: an arm added or dropped
    between corpora means the corpora were not measured by the same run, and
    every cross-corpus sentence in the findings would be comparing two indexes.
    """
    seen: list[list[str]] = []
    for corpus in payload["results"]:
        seen.append([a["embedder"] for a in corpus["embedders"]])
    for other in seen[1:]:
        if other != seen[0]:
            f.add(f"arm sets differ between corpora: {seen[0]} vs {other}")
            return


def rule_control_on_its_floor(payload: dict, f: Findings) -> None:
    """The noise control scores what an embedder that knows nothing should."""
    rate = payload["pairwise_positive_rate"]
    for corpus in payload["results"]:
        c = _arms(corpus).get(CONTROL)
        if c is None:
            continue
        name = corpus["corpus"]
        for field, tol in (("ami_map", AMI_TOL), ("ami_vectors", AMI_TOL), ("retention_vs_minilm", AMI_TOL)):
            v = c.get(field)
            if v is None or abs(v) > tol:
                f.add(f"{name} {CONTROL}.{field} is {v}, not within {tol} of 0")
        v = c.get("map_overlap_with_minilm")
        if v is None or v > OVERLAP_MAX:
            f.add(f"{name} {CONTROL}.map_overlap_with_minilm is {v}, above {OVERLAP_MAX}")
        for field in ("pairwise_ap_same_class", "pairwise_ap_near_duplicate"):
            v = c.get(field)
            if v is None or abs(v - rate) > AP_TOL:
                f.add(f"{name} {CONTROL}.{field} is {v}, not within {AP_TOL} of {rate}")


def rule_ceiling_separates(payload: dict, f: Findings) -> None:
    """The transformer clears the noise on every measure the file reports.

    This is the rule that fails when the harness has stopped measuring rather
    than when a model is weak: the arms in between can legitimately land
    anywhere, but a measure that puts MiniLM level with Gaussian noise is
    reporting its own breakage.
    """
    rate = payload["pairwise_positive_rate"]
    for corpus in payload["results"]:
        arms = _arms(corpus)
        top, bottom = arms.get(CEILING), arms.get(CONTROL)
        if top is None or bottom is None:
            continue
        name = corpus["corpus"]
        for field, floor in (
            ("ami_map", None),
            ("precision_at_k", None),
            ("pairwise_ap_same_class", rate),
            ("pairwise_ap_near_duplicate", rate),
        ):
            hi = top.get(field)
            lo = bottom.get(field) if floor is None else floor
            if hi is None or lo is None or hi - lo < SEPARATION:
                f.add(f"{name} {CEILING}.{field} is {hi}, under {lo} + {SEPARATION}")


def rule_pair_construction(payload: dict, f: Findings) -> None:
    """The pair counts match the construction the positive rate assumes.

    One positive and one negative per anchor is what makes the declared positive
    rate true, and the declared positive rate is what makes every average
    precision in the file readable. An odd count, or a near-duplicate count that
    is not exactly twice the probes, means the construction moved and the floors
    moved with it.
    """
    for corpus in payload["results"]:
        name, probes = corpus["corpus"], corpus["probe_rows"]
        sc = corpus["pairwise_pairs_same_class"]
        nd = corpus["pairwise_pairs_near_duplicate"]
        if sc % 2 or sc > 2 * probes or sc == 0:
            f.add(f"{name} same-class pairs {sc} is not an even count in (0, {2 * probes}]")
        if nd != 2 * probes:
            f.add(f"{name} near-duplicate pairs {nd} is not 2 x {probes}")


def rule_corruptions_corrupt(payload: dict, f: Findings) -> None:
    """The near-duplicate arm is scoring altered strings, not identical ones."""
    for corpus in payload["results"]:
        name, probes = corpus["corpus"], corpus["probe_rows"]
        unchanged = corpus["near_duplicate_unchanged"]
        if probes and unchanged / probes > UNCHANGED_MAX:
            f.add(
                f"{name} near-duplicate corruption returned the input for {unchanged}/{probes} probes",
            )


def rule_licences_recorded(payload: dict, f: Findings) -> None:
    """Every model that produced a number here has its redistribution terms in the file.

    A positive result makes bundling the obvious next question and a bundled
    model is a redistribution, so the terms are recorded beside the numbers that
    would motivate it rather than looked up afterwards.
    """
    declared = payload.get("licences") or {}
    for corpus in payload["results"]:
        for arm in corpus["embedders"]:
            model = arm.get("model")
            if not model or arm.get("kind") == "lexical":
                continue
            entry = declared.get(model)
            if entry is None:
                f.add(f"no licence recorded for {model}")
            elif not entry.get("licence"):
                f.add(f"licence for {model} is {entry.get('licence')!r}")


def rule_arms_are_distinct(payload: dict, f: Findings) -> None:
    """No two arms produced the same vectors.

    The `model` field records which repository was asked for. This records what
    answered: a fingerprint of a fixed string list, taken in the same pass. They
    are not the same claim, and the gap between them is where the quiet failure
    lives -- a typo in an id, a cache alias, a loader that falls back to a
    default -- because the resulting file has every number in range and simply
    reports that two arms agree. That is the shape of a finding, so nothing
    downstream would question it.
    """
    for corpus in payload["results"]:
        seen: dict[str, str] = {}
        for arm in corpus["embedders"]:
            digest = arm.get("fingerprint")
            if digest is None:
                continue
            if digest in seen:
                f.add(
                    f"{corpus['corpus']} {arm['embedder']} and {seen[digest]} "
                    f"produced identical vectors (fingerprint {digest})"
                )
            seen[digest] = arm["embedder"]


def rule_findings_table(payload: dict, f: Findings) -> None:
    """The write-up's table is the results file's table.

    A number in prose is a claim nothing reddens when it goes stale, and staleness
    here is invisible: a re-run rewrites results.json, the document keeps the
    previous figures, and both read as measured now. So the table is generated and
    this compares it, rather than a reviewer comparing them by eye.
    """
    findings = payload.get("_findings_md")
    if findings is None:
        f.add("FINDINGS.md was not supplied, so its figures are unchecked")
        return
    have = extract_table(findings)
    if have is None:
        f.add(f"FINDINGS.md has no block between {TABLE_START!r} and {TABLE_END!r}")
        return
    want = emit_table(payload)
    if have != want:
        have_rows = have.splitlines()
        want_rows = want.splitlines()
        if len(have_rows) != len(want_rows):
            f.add(f"FINDINGS.md table has {len(have_rows)} lines, results.json generates {len(want_rows)}")
        for h, w in zip(have_rows, want_rows):
            if h != w:
                f.add(f"FINDINGS.md says {h!r}, results.json generates {w!r}")


RULES: list[Callable[[dict, Findings], None]] = [
    rule_required_arms,
    rule_arms_are_distinct,
    rule_one_pass,
    rule_control_on_its_floor,
    rule_ceiling_separates,
    rule_pair_construction,
    rule_corruptions_corrupt,
    rule_licences_recorded,
    rule_findings_table,
]


def rule_name(rule: Callable[[dict, Findings], None]) -> str:
    return rule.__name__.removeprefix("rule_")


def check(payload: dict) -> Findings:
    f = Findings()
    for rule in RULES:
        f.rule = rule_name(rule)
        rule(payload, f)
    f.rule = "?"
    return f


# --------------------------------------------------------------------------
# The self-test. Each case names a rule and a mutation that must make it fire.
# A rule with no case here is a rule nothing proves can fail, and `--self-test`
# refuses that too.
# --------------------------------------------------------------------------


def _arm(name: str, kind: str, model: str | None, **over: Any) -> dict:
    base = {
        "embedder": name,
        "kind": kind,
        "model": model,
        "ami_map": 0.40,
        "ami_vectors": 0.38,
        "retention_vs_minilm": 1.0,
        "map_overlap_with_minilm": 1.0,
        "fingerprint": name,
        "precision_at_k": 0.40,
        "mrr_at_k": 0.60,
        "pairwise_ap_same_class": 0.85,
        "pairwise_ap_near_duplicate": 0.95,
        "lift_over_random": 1.0,
    }
    base.update(over)
    return base


def _fixture() -> dict:
    """A file that every rule passes, built so each mutation has one effect."""
    def corpus(key: str) -> dict:
        return {
            "corpus": key,
            "rows": 3000,
            "classes": 20,
            "probe_rows": 800,
            "retrieval_k": 10,
            "pairwise_pairs_same_class": 1600,
            "pairwise_pairs_near_duplicate": 1600,
            "near_duplicate_unchanged": 3,
            "embedders": [
                _arm(CEILING, "transformer", "sentence-transformers/all-MiniLM-L6-v2"),
                _arm("potion-8m", "static", "minishlab/potion-base-8M",
                     ami_map=0.30, precision_at_k=0.27, retention_vs_minilm=0.71,
                     map_overlap_with_minilm=0.13, pairwise_ap_same_class=0.73,
                     pairwise_ap_near_duplicate=0.94, lift_over_random=0.65),
                _arm(CONTROL, "control", None,
                     ami_map=0.001, ami_vectors=-0.001, retention_vs_minilm=0.0,
                     map_overlap_with_minilm=0.01, precision_at_k=0.05, mrr_at_k=0.09,
                     pairwise_ap_same_class=0.51, pairwise_ap_near_duplicate=0.49,
                     lift_over_random=0.0),
                _arm(LEXICAL, "lexical", "duckdb fts match_bm25",
                     ami_map=None, ami_vectors=None, retention_vs_minilm=None,
                     map_overlap_with_minilm=None, precision_at_k=0.24, mrr_at_k=0.40,
                     pairwise_ap_same_class=0.59, pairwise_ap_near_duplicate=0.99,
                     lift_over_random=0.54),
            ],
        }

    payload = {
        "seed": 42,
        "limit": 3000,
        "pairwise_positive_rate": 0.5,
        "licences": {
            "sentence-transformers/all-MiniLM-L6-v2": {"licence": "apache-2.0"},
            "minishlab/potion-base-8M": {"licence": "mit"},
        },
        "results": [corpus("20news-body"), corpus("finetype-columns")],
    }
    payload["_findings_md"] = f"intro\n{TABLE_START}\n{emit_table(payload)}\n{TABLE_END}\nrest"
    return payload


def _drop_arm(p: dict) -> dict:
    p["results"][0]["embedders"] = [a for a in p["results"][0]["embedders"] if a["embedder"] != CONTROL]
    return p


def _extra_arm_in_one_corpus(p: dict) -> dict:
    p["results"][1]["embedders"].append(_arm("potion-32m", "static", "minishlab/potion-base-32M"))
    p["licences"]["minishlab/potion-base-32M"] = {"licence": "mit"}
    return p


def _control_climbs(p: dict) -> dict:
    _arms(p["results"][0])[CONTROL]["ami_map"] = 0.31
    return p


def _control_pairwise_climbs(p: dict) -> dict:
    _arms(p["results"][0])[CONTROL]["pairwise_ap_same_class"] = 0.84
    return p


def _control_overlaps(p: dict) -> dict:
    _arms(p["results"][0])[CONTROL]["map_overlap_with_minilm"] = 0.55
    return p


def _ceiling_collapses(p: dict) -> dict:
    _arms(p["results"][0])[CEILING]["precision_at_k"] = 0.09
    return p


def _ceiling_pairwise_collapses(p: dict) -> dict:
    _arms(p["results"][0])[CEILING]["pairwise_ap_same_class"] = 0.55
    return p


def _pairs_uneven(p: dict) -> dict:
    p["results"][0]["pairwise_pairs_same_class"] = 1599
    return p


def _pairs_too_many(p: dict) -> dict:
    p["results"][0]["pairwise_pairs_near_duplicate"] = 800
    return p


def _corruption_noop(p: dict) -> dict:
    p["results"][0]["near_duplicate_unchanged"] = 800
    return p


def _two_arms_one_model(p: dict) -> dict:
    _arms(p["results"][0])["potion-8m"]["fingerprint"] = _arms(p["results"][0])[CEILING]["fingerprint"]
    return p


def _table_stale(p: dict) -> dict:
    p["_findings_md"] = p["_findings_md"].replace("0.4000", "0.9999").replace("| 0.40 |", "| 0.99 |")
    return p


def _table_absent(p: dict) -> dict:
    p["_findings_md"] = "no table here at all"
    return p


def _results_moved_under_the_table(p: dict) -> dict:
    _arms(p["results"][0])["potion-8m"]["precision_at_k"] = 0.31
    return p


def _licence_missing(p: dict) -> dict:
    del p["licences"]["minishlab/potion-base-8M"]
    return p


def _licence_blank(p: dict) -> dict:
    p["licences"]["sentence-transformers/all-MiniLM-L6-v2"]["licence"] = None
    return p


CASES: list[tuple[str, str, Callable[[dict], dict]]] = [
    ("required_arms", "the noise control is missing from a corpus", _drop_arm),
    ("one_pass", "one corpus carries an arm the other does not", _extra_arm_in_one_corpus),
    ("arms_are_distinct", "two arms named for different models produced identical vectors", _two_arms_one_model),
    ("control_on_its_floor", "the control's map AMI climbs to a real score", _control_climbs),
    ("control_on_its_floor", "the control's pairwise AP climbs off the positive rate", _control_pairwise_climbs),
    ("control_on_its_floor", "the control's map agrees with MiniLM's", _control_overlaps),
    ("ceiling_separates", "MiniLM's precision falls to the noise floor", _ceiling_collapses),
    ("ceiling_separates", "MiniLM's pairwise AP falls to chance", _ceiling_pairwise_collapses),
    ("pair_construction", "the same-class pair count is odd", _pairs_uneven),
    ("pair_construction", "the near-duplicate pair count is not two per probe", _pairs_too_many),
    ("corruptions_corrupt", "the corruption returned the input for every probe", _corruption_noop),
    ("licences_recorded", "a tested model has no licence entry", _licence_missing),
    ("licences_recorded", "a recorded licence is blank", _licence_blank),
    ("findings_table", "the write-up quotes a figure the results file does not", _table_stale),
    ("findings_table", "the write-up has no generated block", _table_absent),
    ("findings_table", "a re-run moved a number and the write-up kept the old one", _results_moved_under_the_table),
]


def self_test() -> int:
    failures: list[str] = []

    clean = check(_fixture())
    if clean:
        failures.append(f"the fixture is refused before any mutation: {clean}")

    for rule, description, mutate in CASES:
        found = check(mutate(copy.deepcopy(_fixture())))
        hit = [x for x in found if x.startswith(rule + ":")]
        if not hit:
            failures.append(f"{rule}: NOT DETECTED when {description} -- reported {found or 'nothing'}")
        else:
            print(f"  ok  {rule}: refused when {description}")

    # A rule with no case proves nothing. This is the enumeration the docstring
    # points at, so it has to be complete rather than merely non-empty.
    covered = {rule for rule, _, _ in CASES}
    names = {rule_name(rule) for rule in RULES}
    for name in sorted(names - covered):
        failures.append(f"{name}: no self-test case, so nothing proves this rule can fire")
    for name in sorted(covered - names):
        failures.append(f"{name}: a case names a rule that does not exist, so it proves nothing")

    for line in failures:
        print(f"  FAIL  {line}", file=sys.stderr)
    print(f"{len(CASES)} cases, {len(failures)} failures", file=sys.stderr)
    return 1 if failures else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("results", nargs="?", type=Path, default=DEFAULT_RESULTS)
    ap.add_argument("--findings", type=Path, default=DEFAULT_FINDINGS)
    ap.add_argument("--self-test", action="store_true", help="prove every rule can fire")
    ap.add_argument("--emit-table", action="store_true",
                    help="print the table FINDINGS.md must carry, and exit")
    args = ap.parse_args()

    if args.self_test:
        return self_test()

    try:
        payload = json.loads(args.results.read_text())
    except (OSError, ValueError) as exc:
        print(f"cannot read {args.results}: {exc}", file=sys.stderr)
        return 2
    if not isinstance(payload, dict) or "results" not in payload:
        print(f"{args.results} is not a map-fidelity results file", file=sys.stderr)
        return 2

    if args.emit_table:
        print(emit_table(payload))
        return 0

    try:
        payload["_findings_md"] = args.findings.read_text()
    except OSError as exc:
        print(f"cannot read {args.findings}: {exc}", file=sys.stderr)
        return 2

    try:
        found = check(payload)
    except (KeyError, TypeError) as exc:
        print(f"{args.results} is missing a field this check reads: {exc!r}", file=sys.stderr)
        return 2

    for line in found:
        print(f"  {line}", file=sys.stderr)
    if found:
        print(f"{len(found)} findings in {args.results}", file=sys.stderr)
        return 1
    print(f"{args.results}: floors hold, ceiling separates, licences recorded", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
