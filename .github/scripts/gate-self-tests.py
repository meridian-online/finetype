#!/usr/bin/env python3
"""Route each gate's self-test to the diffs that change that gate, and prove the routing.

WHY THIS EXISTS
    A gate earns its exit code from a self-test: a harness that mutates the gate,
    or the tree the gate reads, and requires the gate to redden. Fourteen of them
    ship here. Every one ran on every pull request, whether or not the diff went
    anywhere near the gate it covers -- so the cheapest change in the repository
    paid for the most expensive proof, and the rule "a changed gate re-proves
    itself" was held by a reviewer remembering it rather than by anything that
    checks.

    Two things have to be true at once and only one of them is obvious:

      the self-test RUNS when its gate changes  -- otherwise the proof is
          optional, which is the same as absent;
      the self-test DOES NOT RUN when it does not  -- otherwise it is a tax, and
          a tax gets trimmed by whoever is next in a hurry.

    An always-on step satisfies the first and fails the second. A step someone
    remembers to add satisfies neither for long.

WHAT THIS DOES
    `plan`  reads `.github/gate-self-tests.tsv`, diffs the pull request against
            its base, and emits one boolean per gate into `$GITHUB_OUTPUT`. Each
            self-test step in `.github/workflows/ci.yml` is guarded by its own
            boolean, so the workflow runs exactly the proofs the diff invalidated.

    `audit` is the half that keeps `plan` honest, and it runs unconditionally
            because it is the cheap one. It reddens when the manifest, the tree
            and the workflow stop agreeing -- a gate nothing routes, a guard
            naming a job that does not exist, a routing output nobody declared,
            a guard written `== 'false'` so the proof runs only when the gate did
            not change. Three of those are SILENT in Actions: an expression
            referencing a missing output is the empty string, the step is skipped,
            and the job is green. So the audit refuses rather than reports.

            It is a list of refused shapes, not a proof of exhaustion. Each shape
            it covers has a named case in `--self-test`; a shape nobody thought of
            has neither, and the four that this file's first version missed were
            all found by someone reading it rather than by running it. Read the
            case list before trusting the coverage.

    `--self-test` is this file's own instance of the thing it routes. It builds
            scratch repositories and requires each check above to redden against
            a tree that violates it, and to stay quiet against one that does not.

FAIL-SAFE DIRECTION
    Every uncertainty routes MORE work, never less. No base commit, an
    unfetchable base, a diff that will not run, a change to the manifest, to this
    file, or to the workflow -- each selects every gate and says so in the log.
    A router that cannot tell what changed must never answer "nothing".

USAGE
    .github/scripts/gate-self-tests.py plan --base <sha>   # emit the booleans
    .github/scripts/gate-self-tests.py audit               # manifest vs tree vs workflow
    .github/scripts/gate-self-tests.py --self-test         # prove all of the above

EXIT CODES
    0  clean
    1  findings -- the audit disagrees with the tree, or a self-test case did not
       redden
    2  the tool could not run correctly: a malformed manifest, an unreadable
       workflow, no routing job. Always a hard failure, never a quiet pass.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

MANIFEST_REL = ".github/gate-self-tests.tsv"
ROUTER_REL = ".github/scripts/gate-self-tests.py"
WORKFLOW_REL = ".github/workflows/ci.yml"

# A change to any of these invalidates the routing itself, so all of it re-runs.
# The workflow is in the set because the guards live there: an edit that moves a
# self-test between jobs, or rewrites a condition, has to be answered by the
# self-tests rather than by the reader of the diff.
ROOT_TRIGGERS = (MANIFEST_REL, ROUTER_REL, WORKFLOW_REL)

# How the workflow names the planning step. The audit finds the routing job by
# looking for this, rather than being told a job name it could not verify.
PLAN_MARK = "gate-self-tests.py plan"

# Where a gate self-test may live. Non-recursive on purpose: `scripts/` also
# holds the training and mining tree, which is research code rather than gates.
GATE_DIRS = ("scripts", ".github/scripts")

# A file that exposes a self-test entry point of its own.
SELFTEST_ENTRY = re.compile(
    r"""(["']--self-test["'])|(^[ \t]*--self-test\))|(add_parser\(\s*["']self-test["'])""",
    re.MULTILINE,
)

# A file that IS somebody's self-test, by name. `scripts/check-public-hygiene.sh`
# and `scripts/check-formula-asset.sh` take no `--self-test` flag; their proofs
# are separate scripts, and this is how those are found.
SELFTEST_NAME = re.compile(r"(-selftest\.(?:sh|py)$)|([-_]mutations\.(?:sh|py)$)")

# A `run:` line that looks like a gate self-test being invoked. Used in the
# opposite direction from the manifest: anything matching this that the manifest
# does not know about is an unrouted proof running on every diff.
SELFTEST_INVOCATION = re.compile(
    r"(--self-test\b)|(\sself-test\b)|(-selftest\.(?:sh|py)\b)|([-_]mutations\.(?:sh|py)\b)"
)

# An id becomes a job output name and a property in `needs.<job>.outputs.<id>`.
# A hyphen there parses as subtraction, so the character set is narrowed here
# rather than discovered when a guard silently evaluates to the empty string.
ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")


def guard_expression(routing_job: str, gate_id: str) -> str:
    """The ONE condition a routed self-test may carry.

    Compared whole, never searched for. A containment test accepts every
    expression that merely mentions the output, and four of those invert or
    disable the routing while reading almost identically:

        needs.<job>.outputs.<id> == 'false'      the proof runs only when the
                                                 gate did NOT change
        needs.<job>.outputs.<id> != 'true'       the same, one character
        … == 'true' && github.event_name == 'push'
                                                 the proof never runs on a pull
                                                 request, which is the only
                                                 event this file gates
        … == 'true' || <anything>                the guard stops deciding

    A mechanism that guarantees a gate's self-test runs, and that can be
    switched off by a one-character edit nothing refuses, is not the guarantee.
    """
    return f"needs.{routing_job}.outputs.{gate_id} == 'true'"


def job_guard_gates(condition: str, routing_job: str, gate_ids: set[str]) -> set[str] | None:
    """The gates a job-level `if:` admits, or None if it is not purely routing.

    A job condition made only of guards joined by `||` can widen which diffs run
    the job and can never narrow it below its own steps' guards, so it is safe.
    Anything else -- a `&&`, a negation, an event test, a bare term -- is
    REFUSED rather than interpreted. Reading it wrongly means certifying a job
    that skips the proof its steps ask for.
    """
    if not condition.strip():
        return set()
    admitted: set[str] = set()
    for term in (t.strip() for t in condition.split("||")):
        match = next((g for g in gate_ids if term == guard_expression(routing_job, g)), None)
        if match is None:
            return None
        admitted.add(match)
    return admitted

BLOCK_SCALARS = {"|", ">", "|-", ">-", "|+", ">+"}


class Fatal(Exception):
    """The tool cannot answer. Exit 2, never a verdict."""


# ── the manifest ────────────────────────────────────────────────────────────


@dataclass(frozen=True)
class Gate:
    id: str
    commands: tuple[str, ...]
    paths: tuple[str, ...]
    lineno: int


def load_manifest(root: Path) -> list[Gate]:
    """Parse the routing manifest, refusing anything it cannot read exactly."""
    path = root / MANIFEST_REL
    if not path.is_file():
        raise Fatal(f"{MANIFEST_REL}: not found under {root}")

    gates: list[Gate] = []
    first_seen: dict[str, int] = {}
    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        fields = [f.strip() for f in raw.split("\t")]
        # Padded before the count is checked, so a malformed row is REFUSED by the
        # line below rather than crashing the reader on an unpack. A traceback and
        # a refusal are not the same signal and only one of them names the line.
        gate_id, commands_field, paths_field = (fields + ["", ""])[:3]
        if len(fields) != 3:
            raise Fatal(
                f"{MANIFEST_REL}:{lineno}: expected 3 tab-separated columns, found {len(fields)}"
            )

        if not ID_RE.match(gate_id):
            raise Fatal(
                f"{MANIFEST_REL}:{lineno}: id {gate_id!r} must match {ID_RE.pattern} -- "
                "it is dereferenced as needs.<job>.outputs.<id> in the workflow"
            )
        if gate_id in first_seen:
            raise Fatal(
                f"{MANIFEST_REL}:{lineno}: duplicate id {gate_id!r}, "
                f"first defined on line {first_seen[gate_id]}"
            )

        commands = tuple(c.strip() for c in commands_field.split(",") if c.strip())
        paths = tuple(p.strip() for p in paths_field.split(",") if p.strip())
        if not commands:
            raise Fatal(f"{MANIFEST_REL}:{lineno}: {gate_id!r} names no self-test command")
        if not paths:
            raise Fatal(f"{MANIFEST_REL}:{lineno}: {gate_id!r} watches no path")

        first_seen[gate_id] = lineno
        gates.append(Gate(gate_id, commands, paths, lineno))

    if not gates:
        raise Fatal(f"{MANIFEST_REL}: no rows -- an empty manifest routes nothing and says so")
    return gates


# ── the workflow ────────────────────────────────────────────────────────────


@dataclass
class Job:
    id: str
    lineno: int
    condition: str = ""
    needs: tuple[str, ...] = ()
    outputs: dict[str, str] = field(default_factory=dict)


@dataclass
class Step:
    job: str
    lineno: int
    name: str = ""
    step_id: str = ""
    condition: str = ""
    commands: tuple[str, ...] = ()


def _read_value(key: str, inline: str, lines: list[str], index: int, key_indent: int) -> tuple[list[str], int]:
    """Read one mapping value, block scalar or not. Returns (lines, next index)."""
    if inline in BLOCK_SCALARS:
        out: list[str] = []
        j = index + 1
        while j < len(lines):
            line = lines[j]
            if not line.strip():
                out.append("")
                j += 1
                continue
            if len(line) - len(line.lstrip()) <= key_indent:
                break
            out.append(line.strip())
            j += 1
        while out and not out[-1]:
            out.pop()
        return out, j
    if inline.startswith(("|", ">")):
        raise Fatal(f"{WORKFLOW_REL}:{index + 1}: cannot read the block scalar `{key}: {inline}`")
    return ([inline] if inline else []), index + 1


_JOB_RE = re.compile(r"^  ([A-Za-z0-9_-]+):\s*$")
_JOB_KEY_RE = re.compile(r"^    ([A-Za-z0-9_-]+):[ ]?(.*)$")
_JOB_OUTPUT_RE = re.compile(r"^      ([A-Za-z0-9_-]+):[ ]?(.*)$")
_STEP_START = "      - "
_STEP_KEY_RE = re.compile(r"^        ([A-Za-z0-9_-]+):[ ]?(.*)$")


def scan_workflow(root: Path) -> tuple[dict[str, Job], list[Step]]:
    """Read the jobs, their guards and their `run:` commands out of the workflow.

    Line-structured rather than YAML-parsed, because the standard library has no
    YAML reader and this file is the only consumer. Every shape it cannot read
    exactly is REFUSED: a routing decision made from a half-read workflow is the
    failure mode, not a missing dependency.
    """
    path = root / WORKFLOW_REL
    if not path.is_file():
        raise Fatal(f"{WORKFLOW_REL}: not found under {root}")
    lines = path.read_text(encoding="utf-8").splitlines()

    jobs: dict[str, Job] = {}
    steps: list[Step] = []
    in_jobs = False
    in_steps = False
    job: Job | None = None
    step: Step | None = None

    def close_step() -> None:
        nonlocal step
        if step is not None:
            steps.append(step)
            step = None

    i = 0
    while i < len(lines):
        line = lines[i]

        if line.rstrip() == "jobs:":
            in_jobs = True
            i += 1
            continue
        if not in_jobs:
            i += 1
            continue

        job_match = _JOB_RE.match(line)
        if job_match:
            close_step()
            job = Job(id=job_match.group(1), lineno=i + 1)
            jobs[job.id] = job
            in_steps = False
            i += 1
            continue

        if job is None:
            i += 1
            continue

        if in_steps and line.startswith(_STEP_START):
            close_step()
            step = Step(job=job.id, lineno=i + 1)
            remainder = line[len(_STEP_START) :]
            key, _, inline = remainder.partition(":")
            key, inline = key.strip(), inline.strip()
            values, i = _read_value(key, inline, lines, i, 8)
            _apply_step_key(step, key, values)
            continue

        if step is not None:
            step_key = _STEP_KEY_RE.match(line)
            if step_key:
                key, inline = step_key.group(1), step_key.group(2).strip()
                values, i = _read_value(key, inline, lines, i, 8)
                _apply_step_key(step, key, values)
                continue

        job_key = _JOB_KEY_RE.match(line)
        if job_key:
            close_step()
            key, inline = job_key.group(1), job_key.group(2).strip()
            if key == "steps":
                in_steps = True
                i += 1
                continue
            in_steps = False
            if key == "if":
                values, i = _read_value(key, inline, lines, i, 4)
                job.condition = " ".join(values)
                continue
            if key == "needs":
                if not inline:
                    raise Fatal(
                        f"{WORKFLOW_REL}:{i + 1}: job `{job.id}` writes `needs:` as a block "
                        "sequence. Write it inline -- this reader refuses a shape it would "
                        "have to guess at, because guessing here skips a self-test silently."
                    )
                job.needs = tuple(
                    n.strip() for n in inline.strip("[]").split(",") if n.strip()
                )
                i += 1
                continue
            if key == "outputs":
                j = i + 1
                while j < len(lines):
                    nxt = lines[j]
                    if not nxt.strip():
                        j += 1
                        continue
                    if len(nxt) - len(nxt.lstrip()) <= 4:
                        break
                    out_match = _JOB_OUTPUT_RE.match(nxt)
                    if not out_match:
                        raise Fatal(
                            f"{WORKFLOW_REL}:{j + 1}: cannot read this line as an output of "
                            f"job `{job.id}`"
                        )
                    job.outputs[out_match.group(1)] = out_match.group(2).strip()
                    j += 1
                i = j
                continue
            i += 1
            continue

        i += 1

    close_step()
    if not jobs:
        raise Fatal(f"{WORKFLOW_REL}: no jobs found -- the reader is looking at the wrong shape")
    return jobs, steps


def _apply_step_key(step: Step, key: str, values: list[str]) -> None:
    if key == "name":
        step.name = " ".join(values)
    elif key == "id":
        step.step_id = " ".join(values)
    elif key == "if":
        step.condition = " ".join(values)
    elif key == "run":
        step.commands = tuple(v for v in values if v)


# ── discovery ───────────────────────────────────────────────────────────────


def discover_gate_files(root: Path) -> list[str]:
    """Every file in the tree that is, or carries, a gate self-test."""
    found: list[str] = []
    for rel_dir in GATE_DIRS:
        directory = root / rel_dir
        if not directory.is_dir():
            continue
        for entry in sorted(directory.iterdir()):
            if not entry.is_file():
                continue
            rel = f"{rel_dir}/{entry.name}"
            if SELFTEST_NAME.search(entry.name):
                found.append(rel)
                continue
            try:
                text = entry.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            if SELFTEST_ENTRY.search(text):
                found.append(rel)
    return found


# ── the audit ───────────────────────────────────────────────────────────────


def routing_job_id(steps: list[Step]) -> str:
    """The job that runs `plan`, found in the workflow rather than assumed."""
    owners = sorted({s.job for s in steps if any(PLAN_MARK in c for c in s.commands)})
    if not owners:
        raise Fatal(
            f"{WORKFLOW_REL}: no step runs `{PLAN_MARK}`, so nothing sets the outputs every "
            "guard reads. Each guard would evaluate to the empty string and each self-test "
            "would be skipped in a green job."
        )
    if len(owners) > 1:
        raise Fatal(
            f"{WORKFLOW_REL}: `{PLAN_MARK}` runs in more than one job ({', '.join(owners)}); "
            "the guards can only name one"
        )
    return owners[0]


def plan_step_id(steps: list[Step], routing_job: str) -> str:
    """The `id:` of the planning step, which the job outputs must reference."""
    for step in steps:
        if step.job == routing_job and any(PLAN_MARK in c for c in step.commands):
            if not step.step_id:
                raise Fatal(
                    f"{WORKFLOW_REL}:{step.lineno}: the step running `{PLAN_MARK}` has no "
                    "`id:`, so the job cannot map its outputs"
                )
            return step.step_id
    raise Fatal(f"{WORKFLOW_REL}: lost the planning step in job `{routing_job}`")


def audit(root: Path) -> list[str]:
    """Everything that must be true for a guard in the workflow to mean anything."""
    gates = load_manifest(root)
    jobs, steps = scan_workflow(root)
    routing_job = routing_job_id(steps)
    step_id = plan_step_id(steps, routing_job)
    problems: list[str] = []

    watched = {p for g in gates for p in g.paths}
    registered = {c for g in gates for c in g.commands}
    every_id = {g.id for g in gates}

    # The routing job answers for all of them, so it may not be conditional. A
    # single `if:` here -- `github.event_name == 'workflow_dispatch'` is the
    # one-line version -- skips this job, skips every job that needs it, and
    # skips every proof they carry, in a run with no red anywhere.
    if jobs[routing_job].condition:
        problems.append(
            f"{WORKFLOW_REL}:{jobs[routing_job].lineno}: routing job `{routing_job}` carries "
            f"`if: {jobs[routing_job].condition}`. It must be unconditional: skipping it skips "
            "every job that needs it and every proof they carry, with nothing red"
        )

    for trigger in (MANIFEST_REL, ROUTER_REL):
        if trigger not in watched:
            problems.append(
                f"{MANIFEST_REL}: `{trigger}` re-routes every gate when it changes and no row "
                "watches it, so its own self-test would never be selected"
            )

    for gate in gates:
        for rel in gate.paths:
            if not (root / rel).is_file():
                problems.append(
                    f"{MANIFEST_REL}:{gate.lineno}: `{gate.id}` watches `{rel}`, which is not a "
                    "file in the tree -- a path that cannot change routes nothing"
                )

        reference = f"needs.{routing_job}.outputs.{gate.id}"
        guard = guard_expression(routing_job, gate.id)
        declared = jobs[routing_job].outputs.get(gate.id, "")
        expected = f"steps.{step_id}.outputs.{gate.id}"
        if not declared:
            problems.append(
                f"{WORKFLOW_REL}:{jobs[routing_job].lineno}: job `{routing_job}` declares no "
                f"output `{gate.id}`, so `{reference}` is the empty string and every step it "
                "guards is skipped in a green job"
            )
        if declared and expected not in declared:
            problems.append(
                f"{WORKFLOW_REL}:{jobs[routing_job].lineno}: output `{gate.id}` of job "
                f"`{routing_job}` does not read `{expected}`, so it carries another gate's "
                "verdict"
            )

        for command in gate.commands:
            hits = [s for s in steps if command in s.commands]
            if not hits:
                problems.append(
                    f"{MANIFEST_REL}:{gate.lineno}: `{gate.id}` registers `{command}`, which no "
                    f"step in {WORKFLOW_REL} runs -- a self-test nothing invokes"
                )
                continue
            for hit in hits:
                owner = jobs[hit.job]
                # Whole-string, both sides. `guard in condition` accepts
                # `== 'false'`, `!= 'true'` and `… && github.event_name == 'push'`,
                # each of which reads like routing and disables it.
                if hit.condition != guard and owner.condition != guard:
                    carried = hit.condition or owner.condition
                    if not carried:
                        problems.append(
                            f"{WORKFLOW_REL}:{hit.lineno}: `{command}` runs unguarded, so it "
                            f"runs on every diff. It must carry exactly `{guard}`"
                        )
                    else:
                        problems.append(
                            f"{WORKFLOW_REL}:{hit.lineno}: `{command}` is guarded by "
                            f"`{carried}`, not by `{guard}`. Only the exact comparison routes "
                            "it: `== 'false'`, `!= 'true'` and an added `&&` each read like "
                            "routing and disable it"
                        )
                # A job condition may only WIDEN. Any term that is not one of these
                # guards can narrow the job away while its gate has changed, and the
                # step inside is then skipped in a green job whatever its own `if:`
                # says.
                admitted = job_guard_gates(owner.condition, routing_job, every_id)
                if admitted is None:
                    problems.append(
                        f"{WORKFLOW_REL}:{owner.lineno}: job `{hit.job}` carries a routed "
                        f"self-test under `if: {owner.condition}`, which is not a disjunction "
                        "of routing guards, so it can skip a proof its steps ask for"
                    )
                elif admitted and gate.id not in admitted:
                    problems.append(
                        f"{WORKFLOW_REL}:{owner.lineno}: job `{hit.job}` runs `{command}` but "
                        f"its own `if:` never admits `{gate.id}`, so a diff changing that gate "
                        "skips the job and the proof with it"
                    )
                if hit.job != routing_job and routing_job not in owner.needs:
                    problems.append(
                        f"{WORKFLOW_REL}:{owner.lineno}: job `{hit.job}` reads "
                        f"`needs.{routing_job}.…` without `needs: {routing_job}`, so the guard "
                        "is the empty string and the step is skipped in a green job"
                    )

    for step in steps:
        for command in step.commands:
            if SELFTEST_INVOCATION.search(command) and command not in registered:
                problems.append(
                    f"{WORKFLOW_REL}:{step.lineno}: `{command}` is a gate self-test that "
                    f"{MANIFEST_REL} does not register, so nothing routes it and it runs on "
                    "every diff"
                )

    for rel in discover_gate_files(root):
        if rel not in watched:
            problems.append(
                f"{MANIFEST_REL}: `{rel}` carries a gate self-test and no row watches it, so a "
                "diff that changes it re-proves nothing"
            )

    return problems


# ── planning ────────────────────────────────────────────────────────────────


def _git(root: Path, *args: str) -> tuple[int, str]:
    proc = subprocess.run(
        ["git", "-C", str(root), *args],
        capture_output=True,
        text=True,
        check=False,
    )
    return proc.returncode, proc.stdout


def resolve_base(root: Path, base: str) -> str | None:
    """Make the base commit readable, fetching it shallowly if it is absent."""
    base = base.strip()
    if not base or set(base) <= {"0"}:
        return None
    if _git(root, "cat-file", "-e", f"{base}^{{commit}}")[0] == 0:
        return base
    if _git(root, "fetch", "--no-tags", "--depth=1", "origin", base)[0] != 0:
        return None
    if _git(root, "cat-file", "-e", f"{base}^{{commit}}")[0] != 0:
        return None
    return base


def changed_paths(root: Path, base: str) -> list[str] | None:
    code, out = _git(root, "diff", "--name-only", "--no-renames", base, "HEAD")
    if code != 0:
        return None
    return [line.strip() for line in out.splitlines() if line.strip()]


@dataclass
class Plan:
    gates: list[Gate]
    selected: set[str]
    changed: list[str]
    forced: str


def route(root: Path, base: str) -> Plan:
    """Which gates this diff invalidates. Every uncertainty selects all of them."""
    gates = load_manifest(root)
    every = {g.id for g in gates}

    resolved = resolve_base(root, base)
    if resolved is None:
        return Plan(gates, every, [], f"no readable base commit (--base {base!r})")

    changed = changed_paths(root, resolved)
    if changed is None:
        return Plan(gates, every, [], f"`git diff {resolved} HEAD` did not run")

    triggered = [p for p in changed if p in ROOT_TRIGGERS]
    if triggered:
        return Plan(gates, every, changed, f"the routing itself changed: {', '.join(triggered)}")

    changed_set = set(changed)
    selected = {g.id for g in gates if changed_set.intersection(g.paths)}
    return Plan(gates, selected, changed, "")


def emit(plan: Plan) -> None:
    """Print the decision, and hand it to Actions if Actions is asking."""
    if plan.forced:
        print(f"routing every gate self-test: {plan.forced}")
    else:
        print(f"{len(plan.changed)} changed path(s) against the base")
    for gate in plan.gates:
        state = "RUN " if gate.id in plan.selected else "skip"
        print(f"  {state}  {gate.id:<24} {', '.join(gate.commands)}")

    destination = os.environ.get("GITHUB_OUTPUT")
    if not destination:
        return
    with open(destination, "a", encoding="utf-8") as handle:
        handle.write(f"any={'true' if plan.selected else 'false'}\n")
        for gate in plan.gates:
            handle.write(f"{gate.id}={'true' if gate.id in plan.selected else 'false'}\n")


# ── self-test ───────────────────────────────────────────────────────────────

SCRATCH_MANIFEST = """\
# scratch manifest
alpha\tscripts/alpha_gate.py --self-test\tscripts/alpha_gate.py
beta\tscripts/beta-gate-selftest.sh\tscripts/beta_gate.sh, scripts/beta-gate-selftest.sh
gate_routing\t.github/scripts/gate-self-tests.py --self-test\t.github/scripts/gate-self-tests.py, .github/gate-self-tests.tsv
"""

SCRATCH_WORKFLOW = """\
name: CI

on:
  pull_request:
    branches: [main]

jobs:
  gate-routing:
    name: Routing
    runs-on: ubuntu-latest
    outputs:
      any: ${{ steps.route.outputs.any }}
      alpha: ${{ steps.route.outputs.alpha }}
      beta: ${{ steps.route.outputs.beta }}
      gate_routing: ${{ steps.route.outputs.gate_routing }}
    steps:
      - uses: actions/checkout@v5
      - name: Audit
        run: .github/scripts/gate-self-tests.py audit
      - name: Route
        id: route
        run: .github/scripts/gate-self-tests.py plan --base "abc"

  alpha:
    name: Alpha
    needs: [gate-routing]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: the real gate
        run: scripts/alpha_gate.py
      - name: ...and it detects
        if: needs.gate-routing.outputs.alpha == 'true'
        run: scripts/alpha_gate.py --self-test

  beta:
    name: Beta
    needs: [gate-routing]
    if: needs.gate-routing.outputs.beta == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: the beta proof
        run: |
          echo running
          scripts/beta-gate-selftest.sh

  router-self-test:
    name: Router
    needs: [gate-routing]
    if: needs.gate-routing.outputs.gate_routing == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: the router proves itself
        run: .github/scripts/gate-self-tests.py --self-test
"""


def _write(root: Path, rel: str, text: str) -> None:
    target = root / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text, encoding="utf-8")


def _scratch_tree(root: Path) -> None:
    _write(root, MANIFEST_REL, SCRATCH_MANIFEST)
    _write(root, WORKFLOW_REL, SCRATCH_WORKFLOW)
    (root / ROUTER_REL).parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / ROUTER_REL, root / ROUTER_REL)
    _write(root, "scripts/alpha_gate.py", 'import sys\nif "--self-test" in sys.argv:\n    pass\n')
    _write(root, "scripts/beta_gate.sh", "#!/usr/bin/env bash\nexit 0\n")
    _write(root, "scripts/beta-gate-selftest.sh", "#!/usr/bin/env bash\nexit 0\n")
    _write(root, "docs/unrelated.md", "prose\n")


def _scratch_repo(root: Path) -> str:
    """A committed scratch tree. Returns the base commit."""
    _scratch_tree(root)
    for args in (
        ("init", "-q"),
        ("config", "user.email", "self-test@example.invalid"),
        ("config", "user.name", "self test"),
        ("add", "-A"),
        ("-c", "commit.gpgsign=false", "commit", "-qm", "base"),
    ):
        code, _ = _git(root, *args)
        if code != 0:
            raise Fatal(f"self-test: scratch `git {' '.join(args)}` failed")
    code, out = _git(root, "rev-parse", "HEAD")
    if code != 0:
        raise Fatal("self-test: scratch repo has no HEAD")
    return out.strip()


class Recorder:
    """Case bookkeeping: a case that raises is a failure, never a silent skip."""

    def __init__(self) -> None:
        self.passed = 0
        self.failed: list[str] = []

    def check(self, name: str, condition: bool, detail: str = "") -> None:
        if condition:
            self.passed += 1
            print(f"  ok    {name}")
        else:
            self.failed.append(name)
            print(f"  FAIL  {name}{(' -- ' + detail) if detail else ''}")

    def reddens(self, name: str, run: Callable[[], list[str]], expect: str) -> None:
        """`run` must report the NAMED problem, or raise Fatal naming it.

        The substring is not decoration. A case that accepts any non-empty result
        passes when the mutation trips an unrelated rule, or when the scratch
        setup breaks — which is a case that cannot distinguish the rule it was
        written for from its own scaffolding falling over.
        """
        try:
            reported = "\n".join(run())
        except Fatal as exc:
            reported = str(exc)
        if not reported:
            self.check(name, False, "nothing was reported")
            return
        self.check(name, expect in reported, f"expected {expect!r}, got: {reported}")


def _commit_change(root: Path, edits: dict[str, str], message: str) -> None:
    for rel, text in edits.items():
        _write(root, rel, text)
    _git(root, "add", "-A")
    code, _ = _git(root, "-c", "commit.gpgsign=false", "commit", "-qm", message)
    if code != 0:
        raise Fatal(f"self-test: scratch commit {message!r} failed")


def self_test() -> int:
    print("gate self-test routing -- self-test")
    rec = Recorder()

    # ── routing, against a real scratch git repository ───────────────────────
    print("\nrouting")
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "repo"
        root.mkdir()
        base = _scratch_repo(root)

        rec.check("an unchanged tree selects nothing", route(root, base).selected == set())

        _commit_change(root, {"scripts/alpha_gate.py": "# edited\n"}, "touch alpha")
        plan = route(root, base)
        rec.check("a changed gate selects its own self-test", plan.selected == {"alpha"}, str(plan.selected))
        _git(root, "reset", "-q", "--hard", base)

        _commit_change(root, {"scripts/beta-gate-selftest.sh": "# edited\n"}, "touch beta proof")
        rec.check(
            "changing a gate's PROOF selects it too", route(root, base).selected == {"beta"}
        )
        _git(root, "reset", "-q", "--hard", base)

        _commit_change(root, {"docs/unrelated.md": "more prose\n"}, "touch prose")
        plan = route(root, base)
        rec.check("a diff touching no gate selects nothing", plan.selected == set(), str(plan.selected))
        _git(root, "reset", "-q", "--hard", base)

        every = {"alpha", "beta", "gate_routing"}
        for rel, label in (
            (MANIFEST_REL, "manifest"),
            (ROUTER_REL, "router"),
            (WORKFLOW_REL, "workflow"),
        ):
            body = (root / rel).read_text(encoding="utf-8") + "\n# touched\n"
            _commit_change(root, {rel: body}, f"touch {label}")
            plan = route(root, base)
            rec.check(f"changing the {label} selects every gate", plan.selected == every, str(plan.selected))
            rec.check(f"...and says why the {label} did it", bool(plan.forced), plan.forced)
            _git(root, "reset", "-q", "--hard", base)

        # The documented "a diff that will not run selects every gate" branch,
        # exercised by making `git diff` genuinely fail rather than by injecting a
        # stub: an orphan HEAD leaves the base resolvable and HEAD unnameable, so
        # resolve_base succeeds and changed_paths returns None. Without this the
        # branch could be inverted to "selects nothing" -- the opposite of the
        # documented property -- with every other case still green.
        _git(root, "checkout", "-q", "--orphan", "unborn")
        plan = route(root, base)
        rec.check(
            "a diff that will not run selects every gate", plan.selected == every, str(plan.selected)
        )
        rec.check("...and names the diff as the reason", "git diff" in plan.forced, plan.forced)
        _git(root, "checkout", "-q", "-f", "-B", "main", base)

        absent = "0" * 40
        plan = route(root, absent)
        rec.check("an all-zero base selects every gate", plan.selected == every)
        plan = route(root, "")
        rec.check("an empty base selects every gate", plan.selected == every)
        plan = route(root, "deadbeef" * 5)
        rec.check("an unfetchable base selects every gate", plan.selected == every)

        out_file = Path(tmp) / "outputs.txt"
        os.environ["GITHUB_OUTPUT"] = str(out_file)
        try:
            emit(route(root, base))
            written = out_file.read_text(encoding="utf-8")
        finally:
            del os.environ["GITHUB_OUTPUT"]
        rec.check("an empty plan writes any=false", "any=false\n" in written, written)
        rec.check("...and a line per gate", written.count("=") == 4, written)

    # ── the manifest reader refuses what it cannot read ──────────────────────
    # Each case pins the MESSAGE, not just the refusal. Six rows can each be
    # refused for six reasons and an untargeted `except Fatal` accepts any of
    # them -- which is how a deleted column-count check hides behind the
    # "names no path" refusal that a short row also triggers.
    print("\nmanifest")
    for name, body, expect in (
        (
            "a duplicate id is fatal",
            SCRATCH_MANIFEST + "alpha\tx --self-test\tscripts/alpha_gate.py\n",
            "duplicate id 'alpha'",
        ),
        (
            "a hyphenated id is fatal",
            "gate-a\tx --self-test\tscripts/alpha_gate.py\n",
            "id 'gate-a' must match",
        ),
        (
            "a two-column row is fatal",
            "alpha\tx --self-test\n",
            "expected 3 tab-separated columns, found 2",
        ),
        (
            "a row with no command is fatal",
            "alpha\t\tscripts/alpha_gate.py\n",
            "'alpha' names no self-test command",
        ),
        ("a row with no path is fatal", "alpha\tx --self-test\t\n", "'alpha' watches no path"),
        ("an empty manifest is fatal", "# nothing but a comment\n", "no rows"),
    ):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            _scratch_tree(root)
            _write(root, MANIFEST_REL, body)
            try:
                load_manifest(root)
                rec.check(name, False, "it parsed")
            except Fatal as exc:
                rec.check(name, expect in str(exc), f"expected {expect!r}, got: {exc}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _scratch_tree(root)
        rec.check("the scratch manifest itself parses", len(load_manifest(root)) == 3)

    # ── the audit ────────────────────────────────────────────────────────────
    print("\naudit")

    def with_tree(mutate: Callable[[Path], None]) -> Callable[[], list[str]]:
        """Run the audit over a scratch tree that `mutate` has broken."""

        def run() -> list[str]:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                _scratch_tree(root)
                mutate(root)
                return audit(root)

        return run

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _scratch_tree(root)
        clean = audit(root)
        rec.check("the unbroken scratch tree audits clean", clean == [], "; ".join(clean))

    # The positive half of the job-condition rule, and the shape `ci.yml`'s
    # encoder job actually uses: a job carrying proofs for two gates says so on
    # the job, and each step still carries its own exact guard. Without this case
    # the rule could be tightened to "no job condition at all" and nothing would
    # say it had stopped admitting a legitimate shape.
    #
    # The step guard is not optional here. Whole-job routing -- a bare step under
    # a job condition -- is sound only while that condition is exactly one guard;
    # widen it and the bare step runs whenever EITHER gate changes, which is the
    # cost this routing exists to avoid. That shape is refused below.
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _scratch_tree(root)
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        text = text.replace(
            "    if: needs.gate-routing.outputs.beta == 'true'",
            "    if: needs.gate-routing.outputs.beta == 'true' || "
            "needs.gate-routing.outputs.alpha == 'true'",
        ).replace(
            "      - name: the beta proof\n        run: |",
            "      - name: the beta proof\n"
            "        if: needs.gate-routing.outputs.beta == 'true'\n        run: |",
        )
        _write(root, WORKFLOW_REL, text)
        widened = audit(root)
        rec.check(
            "a job condition widened to a second gate, with per-step guards, stays clean",
            widened == [],
            "; ".join(widened),
        )

    def drop_guard(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "        if: needs.gate-routing.outputs.alpha == 'true'\n", ""
        ))

    def misname_job(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "needs.gate-routing.outputs.alpha == 'true'",
            "needs.gate-routng.outputs.alpha == 'true'",
        ))

    def wrong_id(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "        if: needs.gate-routing.outputs.alpha == 'true'",
            "        if: needs.gate-routing.outputs.beta == 'true'",
        ))

    def drop_needs(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "  alpha:\n    name: Alpha\n    needs: [gate-routing]\n",
            "  alpha:\n    name: Alpha\n",
        ))

    def drop_output(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "      alpha: ${{ steps.route.outputs.alpha }}\n", ""
        ))

    def crossed_output(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "      alpha: ${{ steps.route.outputs.alpha }}",
            "      alpha: ${{ steps.route.outputs.beta }}",
        ))

    def drop_step(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "        run: scripts/alpha_gate.py --self-test\n", "        run: true\n"
        ))

    def unregistered_selftest(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "      - name: the real gate\n        run: scripts/alpha_gate.py\n",
            "      - name: the real gate\n        run: scripts/alpha_gate.py\n"
            "      - name: a stowaway\n        run: scripts/gamma_gate.py --self-test\n",
        ))

    def unwatched_gate(root: Path) -> None:
        _write(root, "scripts/gamma_gate.py", 'import sys\nif "--self-test" in sys.argv:\n    pass\n')

    def unwatched_proof(root: Path) -> None:
        _write(root, "scripts/gamma-gate-selftest.sh", "#!/usr/bin/env bash\nexit 0\n")

    def missing_watched_path(root: Path) -> None:
        (root / "scripts/beta_gate.sh").unlink()

    def no_routing_job(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "        run: .github/scripts/gate-self-tests.py plan --base \"abc\"",
            "        run: true",
        ))

    def two_routing_jobs(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text + """
  second-router:
    name: Second
    runs-on: ubuntu-latest
    steps:
      - name: Route again
        id: route2
        run: .github/scripts/gate-self-tests.py plan --base "abc"
""")

    def no_plan_step_id(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace("        id: route\n", ""))

    def block_needs(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "    needs: [gate-routing]\n", "    needs:\n      - gate-routing\n", 1
        ))

    def unrouted_root_trigger(root: Path) -> None:
        text = (root / MANIFEST_REL).read_text(encoding="utf-8")
        _write(root, MANIFEST_REL, text.replace(
            ", .github/gate-self-tests.tsv", ""
        ))

    # The four expressions that read like routing and are not. Each mentions the
    # right output, so a containment test accepts all four; the first two invert
    # the routing and the third confines it to an event this workflow does not
    # gate self-tests on.
    def rewrite_alpha_guard(replacement: str) -> Callable[[Path], None]:
        def mutate(root: Path) -> None:
            text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
            _write(root, WORKFLOW_REL, text.replace(
                "        if: needs.gate-routing.outputs.alpha == 'true'",
                f"        if: {replacement}",
            ))

        return mutate

    def conditional_routing_job(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "  gate-routing:\n    name: Routing\n",
            "  gate-routing:\n    name: Routing\n"
            "    if: github.event_name == 'workflow_dispatch'\n",
        ))

    def narrowing_job_condition(root: Path) -> None:
        # The beta job is routed whole. Point its condition at another gate: beta
        # changes, the job is skipped, and beta's proof never runs.
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "    if: needs.gate-routing.outputs.beta == 'true'",
            "    if: needs.gate-routing.outputs.alpha == 'true'",
        ))

    def widened_whole_job_routing(root: Path) -> None:
        # The beta step relies on whole-job routing and carries no `if:` of its
        # own. Widening the job condition makes it run whenever ALPHA changes.
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "    if: needs.gate-routing.outputs.beta == 'true'",
            "    if: needs.gate-routing.outputs.beta == 'true' || "
            "needs.gate-routing.outputs.alpha == 'true'",
        ))

    def uninterpretable_job_condition(root: Path) -> None:
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        _write(root, WORKFLOW_REL, text.replace(
            "    if: needs.gate-routing.outputs.beta == 'true'",
            "    if: needs.gate-routing.outputs.beta == 'true' && github.ref != 'refs/heads/main'",
        ))

    for name, mutate, expect in (
        (
            "an unguarded self-test step reddens",
            drop_guard,
            "runs unguarded, so it runs on every diff",
        ),
        (
            "a guard naming a job that does not exist reddens",
            misname_job,
            "is guarded by `needs.gate-routng.outputs.alpha == 'true'`",
        ),
        (
            "a guard carrying another gate's id reddens",
            wrong_id,
            "is guarded by `needs.gate-routing.outputs.beta == 'true'`",
        ),
        (
            "a guard INVERTED to == 'false' reddens",
            rewrite_alpha_guard("needs.gate-routing.outputs.alpha == 'false'"),
            "is guarded by `needs.gate-routing.outputs.alpha == 'false'`",
        ),
        (
            "a guard inverted to != 'true' reddens",
            rewrite_alpha_guard("needs.gate-routing.outputs.alpha != 'true'"),
            "is guarded by `needs.gate-routing.outputs.alpha != 'true'`",
        ),
        (
            "a guard narrowed by an event test reddens",
            rewrite_alpha_guard(
                "needs.gate-routing.outputs.alpha == 'true' && github.event_name == 'push'"
            ),
            "&& github.event_name == 'push'`, not by",
        ),
        (
            "a guard widened by a disjunction with a non-guard reddens",
            rewrite_alpha_guard(
                "needs.gate-routing.outputs.alpha == 'true' || github.event_name == 'push'"
            ),
            "|| github.event_name == 'push'`, not by",
        ),
        (
            "a CONDITIONAL routing job reddens",
            conditional_routing_job,
            "routing job `gate-routing` carries `if: github.event_name == 'workflow_dispatch'`",
        ),
        (
            "a job whose own `if:` never admits the gate it proves reddens",
            narrowing_job_condition,
            "its own `if:` never admits `beta`",
        ),
        (
            "a job condition that is not a disjunction of guards reddens",
            uninterpretable_job_condition,
            "is not a disjunction of routing guards",
        ),
        (
            "whole-job routing widened under a bare step reddens",
            widened_whole_job_routing,
            "|| needs.gate-routing.outputs.alpha == 'true'`, not by",
        ),
        (
            "a job reading the routing outputs without `needs:` reddens",
            drop_needs,
            "without `needs: gate-routing`",
        ),
        ("a routing output nobody declared reddens", drop_output, "declares no output `alpha`"),
        (
            "an output wired to the wrong gate reddens",
            crossed_output,
            "does not read `steps.route.outputs.alpha`",
        ),
        (
            "a registered self-test no step runs reddens",
            drop_step,
            f"which no step in {WORKFLOW_REL} runs",
        ),
        (
            "a self-test the manifest does not register reddens",
            unregistered_selftest,
            f"`scripts/gamma_gate.py --self-test` is a gate self-test that {MANIFEST_REL}",
        ),
        (
            "a gate script no row watches reddens",
            unwatched_gate,
            "`scripts/gamma_gate.py` carries a gate self-test and no row watches it",
        ),
        (
            "a proof script no row watches reddens",
            unwatched_proof,
            "`scripts/gamma-gate-selftest.sh` carries a gate self-test",
        ),
        (
            "a watched path that is not a file reddens",
            missing_watched_path,
            "watches `scripts/beta_gate.sh`, which is not a file",
        ),
        (
            "a workflow with no routing job reddens",
            no_routing_job,
            f"no step runs `{PLAN_MARK}`",
        ),
        ("a workflow with two routing jobs reddens", two_routing_jobs, "runs in more than one job"),
        ("a planning step with no `id:` reddens", no_plan_step_id, "has no `id:`"),
        (
            "a block-sequence `needs:` is refused rather than guessed",
            block_needs,
            "writes `needs:` as a block sequence",
        ),
        (
            "the manifest not watching itself reddens",
            unrouted_root_trigger,
            f"`{MANIFEST_REL}` re-routes every gate when it changes and no row watches it",
        ),
    ):
        rec.reddens(name, with_tree(mutate), expect)

    # ── the real tree ────────────────────────────────────────────────────────
    print("\nthis repository")
    real = audit(ROOT)
    rec.check("the real tree audits clean", real == [], "; ".join(real))

    print(f"\n{rec.passed} passed, {len(rec.failed)} failed")
    if rec.failed:
        for name in rec.failed:
            print(f"  FAILED: {name}")
        return 1
    return 0


# ── entry point ─────────────────────────────────────────────────────────────


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--self-test", action="store_true", help="prove the routing and the audit")
    sub = parser.add_subparsers(dest="command")

    plan_parser = sub.add_parser("plan", help="emit one boolean per gate for this diff")
    plan_parser.add_argument("--base", default="", help="the commit this diff is measured against")

    sub.add_parser("audit", help="manifest vs tree vs workflow")

    args = parser.parse_args(argv)

    if args.self_test:
        return self_test()
    if args.command == "plan":
        emit(route(ROOT, args.base))
        return 0
    if args.command == "audit":
        problems = audit(ROOT)
        for problem in problems:
            print(f"FAIL  {problem}")
        if problems:
            print(f"\n{len(problems)} problem(s): a guarded self-test is only worth its guard")
            return 1
        print("gate self-test routing: every gate registered, guarded and reachable")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except Fatal as fatal:
        print(f"gate-self-tests: {fatal}", file=sys.stderr)
        sys.exit(2)
