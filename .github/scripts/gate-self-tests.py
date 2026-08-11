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
            written `== 'false'` so the proof runs only when the gate did not
            change, a condition on the step that does the routing.

            THE WORST SHAPE IS THE LAST ONE, and it is why this paragraph does
            not lead with the guards. Skip the routing step and every output in
            that job is empty, so every proof skips AND the audit meant to refuse
            the tree never runs -- in a job that reports green and complete. A
            missing guard costs one proof; a skipped planner costs the proofs and
            the check on them together.

            IT IS A LIST OF REFUSED SHAPES, NOT A PROOF OF EXHAUSTION. Every shape
            it covers has a named case in `--self-test` and the case list is the
            enumeration -- read it rather than trusting a count in prose. Six
            shapes have been added by people reading this file rather than running
            it, the last two after the audit was already refusing four, so the
            honest prior is that more exist. When you find the seventh, add the
            rule and its case; do not tighten this sentence.

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
       workflow, no routing step. Always a hard failure, never a quiet pass.
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

# How the workflow names the planning step. The audit finds each job's router by
# looking for this, rather than being told a step id it could not verify.
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

# An id becomes a step output name and a property in `steps.<step>.outputs.<id>`.
# A hyphen there parses as subtraction, so the character set is narrowed here
# rather than discovered when a guard silently evaluates to the empty string.
ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")


def guard_expression(plan_step: str, gate_id: str) -> str:
    """The ONE condition a routed self-test may carry.

    Compared whole, never searched for. A containment test accepts every
    expression that merely mentions the output, and four of those invert or
    disable the routing while reading almost identically:

        steps.<plan>.outputs.<id> == 'false'   the proof runs only when the gate
                                               did NOT change
        steps.<plan>.outputs.<id> != 'true'    the same, one character
        … == 'true' && github.event_name == 'push'
                                               the proof never runs on a pull
                                               request, which is the only event
                                               this file gates
        … == 'true' || <anything>              the guard stops deciding

    A mechanism that guarantees a gate's self-test runs, and that can be
    switched off by a one-character edit nothing refuses, is not the guarantee.

    IT READS A STEP, NOT A JOB, AND THAT IS THE LOAD-BEARING PART. The first cut
    published the booleans as outputs of one routing job and had every other job
    `needs:` it. Measured on a probe pull request: when that job failed, the jobs
    depending on it were SKIPPED, and a required status check that is skipped
    SATISFIES branch protection -- the pull request reported UNSTABLE, which is
    mergeable, not BLOCKED. Two of this repository's five required contexts were
    among them. Routing inside the job removes the dependency, so a required
    check can no longer be satisfied by not running.
    """
    return f"steps.{plan_step}.outputs.{gate_id} == 'true'"


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
                "it is read as steps.<step>.outputs.<id> in the workflow"
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
    # `continue-on-error: true` turns a red proof into a green job. It is read
    # here because a key this reader ignores is a key the audit cannot refuse.
    continue_on_error: str = ""


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
    elif key == "continue-on-error":
        step.continue_on_error = " ".join(values)


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


def plan_steps_by_job(steps: list[Step]) -> dict[str, Step]:
    """The `plan` step each job runs, keyed by job.

    Every job carrying a routed self-test runs its own. There is no shared
    routing job to depend on, by design -- see `guard_expression`.
    """
    found: dict[str, Step] = {}
    for step in steps:
        if not any(PLAN_MARK in c for c in step.commands):
            continue
        if step.job in found:
            raise Fatal(
                f"{WORKFLOW_REL}:{step.lineno}: job `{step.job}` runs `{PLAN_MARK}` twice; "
                "the guards can only name one step"
            )
        if not step.step_id:
            raise Fatal(
                f"{WORKFLOW_REL}:{step.lineno}: the step running `{PLAN_MARK}` in job "
                f"`{step.job}` has no `id:`, so no guard can reference its outputs"
            )
        found[step.job] = step
    if not found:
        raise Fatal(
            f"{WORKFLOW_REL}: no step runs `{PLAN_MARK}`, so nothing sets the outputs every "
            "guard reads. Each guard would evaluate to the empty string and each self-test "
            "would be skipped in a green job."
        )
    return found


def audit(root: Path) -> list[str]:
    """Everything that must be true for a guard in the workflow to mean anything."""
    gates = load_manifest(root)
    jobs, steps = scan_workflow(root)
    plan_steps = plan_steps_by_job(steps)
    problems: list[str] = []

    watched = {p for g in gates for p in g.paths}
    registered = {c for g in gates for c in g.commands}

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
                planner = plan_steps.get(hit.job)

                # The job has to do its own routing. Depending on another job's
                # outputs is what let a skipped required check satisfy branch
                # protection; see `guard_expression`.
                if planner is None:
                    problems.append(
                        f"{WORKFLOW_REL}:{hit.lineno}: job `{hit.job}` runs `{command}` but no "
                        f"step in it runs `{PLAN_MARK}`, so nothing in this job sets the "
                        "outputs its guards read"
                    )
                    continue

                # Ordering, because Actions resolves `steps.<id>.outputs` against
                # what has ALREADY run. A guard above its planner is the empty
                # string, so the proof is skipped and the job is green.
                if planner.lineno > hit.lineno:
                    problems.append(
                        f"{WORKFLOW_REL}:{hit.lineno}: `{command}` is guarded by step "
                        f"`{planner.step_id}`, which does not run until line {planner.lineno}. "
                        "Its outputs are empty here, so the proof is skipped in a green job"
                    )

                # THREE conditions sit in this neighbourhood and all three have to
                # be looked at. The guard below is one. The job is the second. The
                # PLANNER is the third, and it is the worst of them: skip the step
                # that routes and every output is empty, so every proof in the job
                # skips AND the audit that would have refused the tree never runs,
                # in a job that reports green and complete. The first version of
                # this file read the other two and not this one.
                if planner.condition:
                    problems.append(
                        f"{WORKFLOW_REL}:{planner.lineno}: the routing step `{planner.step_id}` "
                        f"in job `{hit.job}` carries `if: {planner.condition}`. It must be "
                        "unconditional: skipping it empties every output in this job, skips "
                        "every proof, and skips the audit meant to refuse that"
                    )

                # A job-level `if:` can skip the whole job, and it cannot read the
                # `steps` context, so it can never be the guard -- only a way to
                # lose one.
                if owner.condition:
                    problems.append(
                        f"{WORKFLOW_REL}:{owner.lineno}: job `{hit.job}` carries a routed "
                        f"self-test under `if: {owner.condition}`. A job holding a proof must "
                        "be unconditional; a job-level condition cannot read `steps` and can "
                        "only skip the proof"
                    )

                # A proof allowed to fail is not a proof. This is the same shape as
                # a missing guard reached from the other side: the step runs, goes
                # red, and the job reports success anyway.
                for offender, role in ((planner, "routing step"), (hit, "proof")):
                    if offender.continue_on_error:
                        problems.append(
                            f"{WORKFLOW_REL}:{offender.lineno}: the {role} in job `{hit.job}` "
                            f"carries `continue-on-error: {offender.continue_on_error}`, so it "
                            "can fail and leave the job green"
                        )

                guard = guard_expression(planner.step_id, gate.id)
                if hit.condition != guard:
                    if not hit.condition:
                        problems.append(
                            f"{WORKFLOW_REL}:{hit.lineno}: `{command}` runs unguarded, so it "
                            f"runs on every diff. It must carry exactly `{guard}`"
                        )
                    else:
                        problems.append(
                            f"{WORKFLOW_REL}:{hit.lineno}: `{command}` is guarded by "
                            f"`{hit.condition}`, not by `{guard}`. Only the exact comparison "
                            "routes it: `== 'false'`, `!= 'true'` and an added `&&` each read "
                            "like routing and disable it"
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
  alpha:
    name: Alpha
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Route
        id: route
        run: .github/scripts/gate-self-tests.py plan --base "abc"
      - name: the real gate
        run: scripts/alpha_gate.py
      - name: ...and it detects
        if: steps.route.outputs.alpha == 'true'
        run: scripts/alpha_gate.py --self-test

  beta:
    name: Beta
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Route
        id: route
        run: .github/scripts/gate-self-tests.py plan --base "abc"
      - name: the beta proof
        if: steps.route.outputs.beta == 'true'
        run: |
          echo running
          scripts/beta-gate-selftest.sh

  router-self-test:
    name: Router
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Route
        id: route
        run: .github/scripts/gate-self-tests.py plan --base "abc"
      - name: the router proves itself
        if: steps.route.outputs.gate_routing == 'true'
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

    # The positive half, and it has to bite or it is decoration. A job may carry
    # UNROUTED steps beside routed ones -- `evidence` and `doc-surface` both do,
    # and their real gates must keep running on every diff -- so adding one must
    # stay clean. The assertion below is written against a workflow the mutation
    # actually changed: `_scratch_tree` is re-read afterwards and the inserted
    # text asserted present, because the two replacements this case used to make
    # had silently become no-ops when the fixture changed shape, and a no-op
    # mutation is a case that passes on an unmutated tree.
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _scratch_tree(root)
        text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
        extra = "      - name: an unrouted check\n        run: scripts/alpha_gate.py --verify\n"
        mutated = text.replace("      - name: the real gate\n", extra + "      - name: the real gate\n", 1)
        rec.check("...the widening case actually mutates its fixture", mutated != text)
        _write(root, WORKFLOW_REL, mutated)
        widened = audit(root)
        rec.check(
            "an unrouted step beside a routed one stays clean",
            widened == [],
            "; ".join(widened),
        )

    def rewrite(old: str, new_text: str, count: int = -1) -> Callable[[Path], None]:
        """Mutate the scratch workflow by substitution.

        `count` matters: all three scratch jobs carry an identical routing step,
        so a mutation meant for one job has to say so, or it removes the routing
        from the whole file and trips a different rule.
        """

        def mutate(root: Path) -> None:
            text = (root / WORKFLOW_REL).read_text(encoding="utf-8")
            assert text.count(old) >= 1, f"self-test: {old!r} is not in the scratch workflow"
            _write(root, WORKFLOW_REL, text.replace(old, new_text, count))

        return mutate

    ALPHA_PROOF = (
        "      - name: ...and it detects\n"
        "        if: steps.route.outputs.alpha == 'true'\n"
        "        run: scripts/alpha_gate.py --self-test\n"
    )

    ALPHA_GUARD = "        if: steps.route.outputs.alpha == 'true'\n"
    ALPHA_PLAN = (
        "      - name: Route\n        id: route\n"
        '        run: .github/scripts/gate-self-tests.py plan --base "abc"\n'
    )

    def unwatched_gate(root: Path) -> None:
        _write(root, "scripts/gamma_gate.py", 'import sys\nif "--self-test" in sys.argv:\n    pass\n')

    def unwatched_proof(root: Path) -> None:
        _write(root, "scripts/gamma-gate-selftest.sh", "#!/usr/bin/env bash\nexit 0\n")

    def missing_watched_path(root: Path) -> None:
        (root / "scripts/beta_gate.sh").unlink()

    def unrouted_root_trigger(root: Path) -> None:
        text = (root / MANIFEST_REL).read_text(encoding="utf-8")
        _write(root, MANIFEST_REL, text.replace(", .github/gate-self-tests.tsv", ""))

    guard_shapes = (
        ("INVERTED to == 'false'", "steps.route.outputs.alpha == 'false'"),
        ("inverted to != 'true'", "steps.route.outputs.alpha != 'true'"),
        (
            "narrowed by an event test",
            "steps.route.outputs.alpha == 'true' && github.event_name == 'push'",
        ),
        (
            "widened by a disjunction",
            "steps.route.outputs.alpha == 'true' || github.event_name == 'push'",
        ),
        ("pointed at another gate", "steps.route.outputs.beta == 'true'"),
        ("pointed at a step that does not exist", "steps.rout.outputs.alpha == 'true'"),
    )

    cases: list[tuple[str, Callable[[Path], None], str]] = [
        (
            "an unguarded self-test step reddens",
            rewrite(ALPHA_GUARD, ""),
            "runs unguarded, so it runs on every diff",
        ),
    ]
    for label, expression in guard_shapes:
        cases.append((
            f"a guard {label} reddens",
            rewrite(ALPHA_GUARD, f"        if: {expression}\n"),
            f"is guarded by `{expression}`, not by",
        ))

    cases += [
        (
            "a job carrying a proof with NO routing step of its own reddens",
            rewrite(ALPHA_PLAN, "", 1),
            "but no step in it runs `gate-self-tests.py plan`",
        ),
        (
            "a guard sitting ABOVE the step that sets it reddens",
            # Actions resolves `steps.<id>.outputs` against what has already run,
            # so a routing step moved BELOW the proof it guards leaves the guard
            # empty and the proof skipped, in a green job. Moving it one step down
            # is not enough -- it has to end up past the guarded step.
            lambda root: _write(
                root,
                WORKFLOW_REL,
                (root / WORKFLOW_REL)
                .read_text(encoding="utf-8")
                .replace(ALPHA_PLAN, "", 1)
                .replace(ALPHA_PROOF, ALPHA_PROOF + ALPHA_PLAN, 1),
            ),
            "which does not run until line",
        ),
        (
            "an `if:` on the ROUTING STEP reddens",
            # The worst of the three conditions in this neighbourhood, and the one
            # the first version of this file did not read: skipping the planner
            # empties every output in the job, skips every proof, and skips the
            # audit that exists to refuse the tree -- all of it green.
            rewrite(ALPHA_PLAN, ALPHA_PLAN.replace(
                "        id: route\n",
                "        id: route\n        if: github.event_name == 'push'\n",
            ), 1),
            "the routing step `route` in job `alpha` carries `if: github.event_name == 'push'`",
        ),
        (
            "`continue-on-error` on a routed proof reddens",
            rewrite(ALPHA_GUARD, ALPHA_GUARD + "        continue-on-error: true\n", 1),
            "the proof in job `alpha` carries `continue-on-error: true`",
        ),
        (
            "`continue-on-error` on the routing step reddens",
            rewrite(ALPHA_PLAN, ALPHA_PLAN.replace(
                "        id: route\n", "        id: route\n        continue-on-error: true\n",
            ), 1),
            "the routing step in job `alpha` carries `continue-on-error: true`",
        ),
        (
            "a job-level `if:` on a job holding a proof reddens",
            rewrite(
                "  alpha:\n    name: Alpha\n",
                "  alpha:\n    name: Alpha\n    if: github.event_name == 'push'\n",
            ),
            "A job holding a proof must be unconditional",
        ),
        (
            "a routing step with no `id:` is refused",
            rewrite("      - name: Route\n        id: route\n", "      - name: Route\n"),
            "has no `id:`",
        ),
        (
            "two routing steps in one job are refused",
            rewrite(ALPHA_PLAN, ALPHA_PLAN + ALPHA_PLAN),
            "runs `gate-self-tests.py plan` twice",
        ),
        (
            "a workflow with no routing step at all is refused",
            rewrite('        run: .github/scripts/gate-self-tests.py plan --base "abc"', "        run: true"),
            "no step runs `gate-self-tests.py plan`",
        ),
        (
            "a registered self-test no step runs reddens",
            rewrite("        run: scripts/alpha_gate.py --self-test\n", "        run: true\n"),
            f"which no step in {WORKFLOW_REL} runs",
        ),
        (
            "a self-test the manifest does not register reddens",
            rewrite(
                "      - name: the real gate\n",
                "      - name: a stowaway\n        run: scripts/gamma_gate.py --self-test\n"
                "      - name: the real gate\n",
            ),
            f"`scripts/gamma_gate.py --self-test` is a gate self-test that {MANIFEST_REL}",
        ),
        (
            "a block-sequence `needs:` is refused rather than guessed",
            rewrite("    name: Alpha\n", "    name: Alpha\n    needs:\n      - beta\n"),
            "writes `needs:` as a block sequence",
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
            "the manifest not watching itself reddens",
            unrouted_root_trigger,
            f"`{MANIFEST_REL}` re-routes every gate when it changes and no row watches it",
        ),
    ]

    for name, mutate, expect in cases:
        rec.reddens(name, with_tree(mutate), expect)

    # `plan` must REFUSE on a tree the audit rejects, not route it. Without this
    # the audit could be dropped from the planning path entirely -- leaving it
    # only in the standalone job, which is not a required context and therefore
    # not blocking -- with every other case still green.
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp) / "repo"
        root.mkdir()
        clean_base = _scratch_repo(root)
        rec.check("`plan` routes a clean tree", cmd_plan(root, clean_base) == 0)
        broken = (root / MANIFEST_REL).read_text(encoding="utf-8")
        _write(root, MANIFEST_REL, broken + "stowaway\tscripts/nope.py --self-test\tscripts/nope.py\n")
        rec.check(
            "`plan` REFUSES a tree the audit rejects, rather than routing it",
            cmd_plan(root, clean_base) == 1,
        )

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


def cmd_plan(root: Path, base: str) -> int:
    """Audit, then route. The order is the point and the self-test pins it.

    The audit runs INSIDE the job that is about to route, which is what makes it
    blocking: two of this repository's required contexts route, so a routing
    defect reddens a check branch protection reads. A dedicated audit job does
    not -- it is not a required context, and a pull request whose only red job is
    unrequired reports UNSTABLE, which is mergeable.
    """
    if report_audit(root) != 0:
        return 1
    emit(route(root, base))
    return 0


def report_audit(root: Path) -> int:
    """Print the audit's findings and return the exit code they imply."""
    problems = audit(root)
    for problem in problems:
        print(f"FAIL  {problem}")
    if problems:
        print(f"\n{len(problems)} problem(s): a guarded self-test is only worth its guard")
        return 1
    print("gate self-test routing: every gate registered, guarded and reachable")
    return 0


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
        return cmd_plan(ROOT, args.base)
    if args.command == "audit":
        return report_audit(ROOT)

    parser.print_help()
    return 2


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except Fatal as fatal:
        print(f"gate-self-tests: {fatal}", file=sys.stderr)
        sys.exit(2)
