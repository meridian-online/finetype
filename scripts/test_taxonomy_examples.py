#!/usr/bin/env python3
"""Round-trip test for the taxonomy `examples` arrays.

Every type definition in the taxonomy carries an `examples` array (surfaced by
`finetype taxonomy -o json-schema`). Those examples are meant to be *representative
values* of the type — so a column made of them should classify back as that type.

This test builds one isolated single-column CSV per type (the type's examples,
cycled to a fixed row count), runs the REAL product pipeline (`finetype profile`,
which applies Sense + the full Sharpen recovery layer — NOT `finetype infer`, which
is single-value-weak and skips the column-context recovery guards), and checks that
the inferred label round-trips to the type's own label.

Why `profile`, not `infer`:
  `finetype infer -i "#FF0000" --mode column` classifies a *1-row column* and
  returns `integer_number` — an artifact of the column-level model seeing one value
  with no distribution and no recovery context. The same value inside a real column
  (`profile`) round-trips to `color_hex`. `profile` is what analysts actually run,
  so it is the honest surface for an examples-integrity check.

Engine: `profile --files <list> --out-dir <dir> -o json-schema` loads the model +
taxonomy ONCE and profiles each file in turn (fast, and each column is isolated so
there is no cross-column sibling contamination).

Header policy: by default each column is headed with the type's own leaf name
(e.g. `color_hex`) — the most-favourable, realistic header. A failure *even with the
ideal header* is the strongest signal that an example does not represent its type.
`--neutral-header` heads every column `value` instead, isolating the pure
value-signal (many types legitimately lean on header context — that is a feature,
not a bug, so neutral failures are advisory only).

Regression contract (mirrors the gold/gate baseline pattern):
  A committed baseline (output/taxonomy-examples/baseline.json) records the current
  pass set and the acknowledged-gap set. The test FAILS (exit 1) when a type that
  used to round-trip regresses, or a NEW type fails without being acknowledged in
  the baseline. Acknowledged gaps that still fail are green. Run with
  --update-baseline to re-lock after an intentional change; the report always lists
  gaps that have started passing (candidates to graduate out of the baseline).

Usage:
  python3 scripts/test_taxonomy_examples.py                 # score vs baseline, exit 1 on regression
  python3 scripts/test_taxonomy_examples.py --update-baseline
  python3 scripts/test_taxonomy_examples.py --neutral-header --no-baseline   # ad-hoc, value-only
  FINETYPE_BIN=target/release/finetype python3 scripts/test_taxonomy_examples.py
"""
import argparse
import csv
import json
import os
import subprocess
import sys
import tempfile

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def resolve_bin() -> str:
    """FINETYPE_BIN override > repo release build > PATH `finetype`.

    Preferring the repo's target/release binary over a PATH install avoids the
    stale-binary footgun: a globally-installed `finetype` can lag the working
    tree by several versions, silently scoring examples against an old taxonomy
    (e.g. a leaf added this week would spuriously fail to round-trip)."""
    override = os.environ.get("FINETYPE_BIN")
    if override:
        return override
    release = os.path.join(REPO, "target", "release", "finetype")
    return release if os.path.exists(release) else "finetype"


BIN = resolve_bin()


def bin_version() -> str:
    try:
        return subprocess.run([BIN, "--version"], capture_output=True, text=True).stdout.strip()
    except Exception:
        return "?"



OUT_DIR = os.path.join(REPO, "output", "taxonomy-examples")
BASELINE = os.path.join(OUT_DIR, "baseline.json")
REPORT = os.path.join(OUT_DIR, "report.md")


def leaf(label: str) -> str:
    return label.split(".")[-1]


def load_taxonomy_examples() -> list[tuple[str, list[str]]]:
    """Return [(label, examples)] from `finetype taxonomy -o json-schema`."""
    p = subprocess.run(
        [BIN, "taxonomy", "-o", "json-schema"],
        capture_output=True, text=True,
    )
    if p.returncode != 0:
        sys.exit(f"`{BIN} taxonomy -o json-schema` failed:\n{p.stderr}")
    schema = json.loads(p.stdout)
    out = []
    for s in schema:
        label = s.get("x-finetype-label")
        examples = s.get("examples") or []
        if label and examples:
            out.append((label, examples))
    return out


def build_columns(types, workdir, n_rows, neutral) -> dict[str, str]:
    """Write one isolated single-column CSV per type. Return {filestem: label}."""
    stem_to_label = {}
    paths = []
    for label, examples in types:
        stem = label.replace(".", "__")
        header = "value" if neutral else leaf(label)
        rows = [examples[i % len(examples)] for i in range(n_rows)]
        path = os.path.join(workdir, f"{stem}.csv")
        with open(path, "w", newline="") as f:
            # QUOTE_ALL keeps comma/quote-bearing values (user_agent, csv, wkt…)
            # from confusing the CSV sniffer into splitting extra columns.
            w = csv.writer(f, quoting=csv.QUOTE_ALL)
            w.writerow([header])
            for v in rows:
                w.writerow([v])
        paths.append(path)
        stem_to_label[stem] = label
    with open(os.path.join(workdir, "filelist.txt"), "w") as f:
        f.write("\n".join(paths) + "\n")
    return stem_to_label


def run_profile(workdir) -> str:
    out = os.path.join(workdir, "out")
    os.makedirs(out, exist_ok=True)
    p = subprocess.run(
        [BIN, "profile", "--files", os.path.join(workdir, "filelist.txt"),
         "--out-dir", out, "-o", "json-schema"],
        capture_output=True, text=True,
    )
    # non-zero exit can come from a single sniffer edge case; press on if outputs exist
    if not os.listdir(out):
        sys.exit(f"`{BIN} profile --files` produced no output:\n{p.stderr}")
    return out


def score(stem_to_label, out) -> dict[str, str]:
    """Return {label: inferred_label}. Missing output -> '<no-output>'."""
    got = {}
    for stem, label in stem_to_label.items():
        op = os.path.join(out, f"{stem}.json")
        inferred = "<no-output>"
        if os.path.exists(op):
            doc = json.load(open(op))
            props = doc.get("properties", {})
            if props:
                # the target column is the first (and only) property
                inferred = next(iter(props.values())).get("x-finetype-label", "unknown")
        got[label] = inferred
    return got


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--neutral-header", action="store_true",
                    help="head every column 'value' (isolate value-signal; advisory)")
    ap.add_argument("--n-rows", type=int, default=20,
                    help="rows per synthetic column (examples cycled); default 20")
    ap.add_argument("--update-baseline", action="store_true",
                    help="rewrite the committed baseline from the current run")
    ap.add_argument("--no-baseline", action="store_true",
                    help="score + report only; do not diff or gate on the baseline")
    args = ap.parse_args()

    os.makedirs(OUT_DIR, exist_ok=True)
    types = load_taxonomy_examples()

    with tempfile.TemporaryDirectory(prefix="ft-examples-") as workdir:
        stem_to_label = build_columns(types, workdir, args.n_rows, args.neutral_header)
        out = run_profile(workdir)
        got = score(stem_to_label, out)

    passed = sorted(l for l, g in got.items() if g == l)
    failed = {l: g for l, g in got.items() if g != l}
    header_mode = "neutral('value')" if args.neutral_header else "leaf-name"
    rate = 100.0 * len(passed) / len(types)

    print(f"Binary: {BIN} ({bin_version()})")
    print(f"Round-trip: {len(passed)}/{len(types)} ({rate:.1f}%)  "
          f"[profile, isolated, header={header_mode}, n_rows={args.n_rows}]")

    # --- baseline diff / regression gate ---
    exit_code = 0
    regressions, new_fails, recovered = [], [], []
    baseline = None
    if os.path.exists(BASELINE) and not args.no_baseline:
        baseline = json.load(open(BASELINE))
        base_pass = set(baseline.get("passing", []))
        base_gaps = set(baseline.get("acknowledged_gaps", {}))
        for l in base_pass:
            if l in failed:
                regressions.append((l, failed[l]))
        for l in failed:
            if l not in base_pass and l not in base_gaps:
                new_fails.append((l, failed[l]))
        for l in base_gaps:
            if l not in failed:
                recovered.append(l)

    write_report(types, passed, failed, header_mode, args.n_rows,
                 baseline, regressions, new_fails, recovered)

    if args.update_baseline:
        json.dump({
            "engine": "profile --files -o json-schema, isolated single-column",
            "header_mode": header_mode,
            "n_rows": args.n_rows,
            "round_trip": f"{len(passed)}/{len(types)}",
            "passing": passed,
            "acknowledged_gaps": failed,
        }, open(BASELINE, "w"), indent=2, sort_keys=True)
        print(f"Baseline updated: {BASELINE} "
              f"({len(passed)} passing, {len(failed)} acknowledged gaps)")
        return

    if baseline is not None:
        if regressions:
            print(f"\nREGRESSION ({len(regressions)}): types that used to round-trip now fail:")
            for l, g in sorted(regressions):
                print(f"  {l} -> {g}")
            exit_code = 1
        if new_fails:
            print(f"\nNEW UNACKNOWLEDGED FAILS ({len(new_fails)}): "
                  f"new/changed examples that don't round-trip:")
            for l, g in sorted(new_fails):
                print(f"  {l} -> {g}")
            exit_code = 1
        if recovered:
            print(f"\nGRADUATED ({len(recovered)}): acknowledged gaps now pass — "
                  f"rerun with --update-baseline to lock in:")
            for l in sorted(recovered):
                print(f"  {l}")
        if exit_code == 0 and not recovered:
            print("OK — no regressions vs baseline.")
    else:
        print(f"\n{len(failed)} type(s) do not round-trip (no baseline gate). See {REPORT}")

    print(f"Report: {REPORT}")
    sys.exit(exit_code)


def write_report(types, passed, failed, header_mode, n_rows,
                 baseline, regressions, new_fails, recovered):
    lines = []
    lines.append("# Taxonomy examples round-trip report\n")
    lines.append(f"Engine: `finetype profile` (isolated single-column, full Sense+Sharpen). "
                 f"Header: {header_mode}. Rows/column: {n_rows}. "
                 f"Binary: `{BIN}` ({bin_version()}).\n")
    lines.append(f"**Round-trip: {len(passed)}/{len(types)} "
                 f"({100.0*len(passed)/len(types):.1f}%).**\n")
    lines.append("A type *round-trips* when a column of its own taxonomy examples profiles "
                 "back to that type. Failures are grouped below; not all are bugs — many are "
                 "types the 244-dim model cannot predict and that the Sharpen recovery guards "
                 "only fire for on real column context (membership thresholds, anchored "
                 "patterns), which a small synthetic example column need not satisfy.\n")
    if baseline is not None:
        lines.append("## Baseline diff\n")
        lines.append(f"- Regressions (were passing, now fail): **{len(regressions)}**")
        lines.append(f"- New unacknowledged fails: **{len(new_fails)}**")
        lines.append(f"- Graduated (acknowledged gap now passes): **{len(recovered)}**\n")
        for title, items in [("Regressions", regressions), ("New unacknowledged fails", new_fails)]:
            if items:
                lines.append(f"### {title}")
                for l, g in sorted(items):
                    lines.append(f"- `{l}` → `{g}`")
                lines.append("")
        if recovered:
            lines.append("### Graduated (rerun --update-baseline to lock)")
            for l in sorted(recovered):
                lines.append(f"- `{l}`")
            lines.append("")
    lines.append("## All non-round-tripping types\n")
    lines.append("| type | profiled as |")
    lines.append("| --- | --- |")
    for l, g in sorted(failed.items()):
        lines.append(f"| `{l}` | `{g}` |")
    lines.append("")
    open(REPORT, "w").write("\n".join(lines))


if __name__ == "__main__":
    main()
