#!/usr/bin/env python3
"""Whole-record A/B for a model-artifact change, through `finetype profile -o csv`.

Why this exists
---------------
`scripts/score_gold_anchor.py predict` writes one row per column carrying the
**label** and a `confidence` column it declares in its header and then fills with
the empty string for every row (line 284 declares it, line 300 writes `""`).  Two
of its prediction files matching by sha256 therefore establishes label-invariance
and *nothing else*: confidence, quality band, runner-up, disambiguation rule,
format string, transform and detected locale can all move without shifting that
file's digest.  Two separate pull requests were refused for citing that digest as
whole-record evidence.

`finetype profile -o csv` emits the whole record.  This script drives it once per
gold column, one single-column CSV per invocation — the same reconstruction
`score_gold_anchor._profile_column` uses, so the comparison runs through the path
a user actually runs — and then diffs two such runs field by field.

`detected_locale` is not run-to-run stable
------------------------------------------
It is nondeterministic on a *fixed* binary and a *fixed* model directory; see
`crates/finetype-cli/src/cmd_run.rs:107`, which declines to emit it for that
reason.  `diff` reports it like any other field but also reports it separately as
`unstable_fields`, and `--noise-floor` names the fields a same-binary/same-model
repeat already showed moving, so a reader is never handed locale churn as if it
were an effect of the change under test.

Usage
-----
    encoder_dtype_record_diff.py prepare --gold G --columns C.parquet --out S.tsv
    encoder_dtype_record_diff.py emit --binary B --model M --samples S.tsv --out R.tsv
    encoder_dtype_record_diff.py diff --a R1.tsv --b R2.tsv [--json OUT.json]
    encoder_dtype_record_diff.py self-test

`emit` must be run with the repository root as the working directory: `profile`
resolves `models/model2vec` and `labels/` relative to it, so a different cwd
silently changes the shared header encoder and the taxonomy under test.
"""

from __future__ import annotations

import argparse
import csv
import io
import json
import os
import subprocess
import sys
import tempfile
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

# The corpus parquet joins truncated sample values with this separator.
SEP = "│"

# The columns `profile -o csv` prints, in order.  Kept as a literal so a change
# to the CLI's header reddens `parse_profile_csv` loudly instead of silently
# shifting every field one to the left.
PROFILE_FIELDS = [
    "column",
    "type",
    "confidence",
    "quality_band",
    "runner_up",
    "broad_type",
    "format_string",
    "transform",
    "is_generic",
    "samples_used",
    "non_null",
    "null",
    "disambiguation",
    "locale",
]

# Every emitted field is compared, `column` included.  The row key comes from the
# samples file (sha + the gold column name), never from the tool's own output:
# DuckDB's CSV sniffer renames an all-numeric header to `column0`, so `column` is
# something the tool decides and therefore something the diff has to watch.
COMPARED_FIELDS = list(PROFILE_FIELDS)

# Fields whose difference is a numeric magnitude rather than a flip.
NUMERIC_FIELDS = {"confidence"}

RECORD_FIELDS = ["file_content_sha256", "column_name"] + PROFILE_FIELDS


# ── sample preparation ──────────────────────────────────────────────────────


def load_tsv(path: Path) -> list[dict[str, str]]:
    with path.open(newline="") as fh:
        return [dict(r) for r in csv.DictReader(fh, delimiter="\t")]


def write_tsv(path: Path, rows: list[dict[str, str]], fields: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=fields, delimiter="\t", lineterminator="\n")
        w.writeheader()
        for r in rows:
            w.writerow({k: r.get(k, "") for k in fields})


def cmd_prepare(args: argparse.Namespace) -> int:
    """Resolve each gold column to its truncated sample values.

    Reads the corpus parquet through the `duckdb` CLI rather than pyarrow: the
    CLI is already a hard runtime dependency of `finetype profile` (choice 0100),
    so this adds nothing to what a reader must install to reproduce the numbers.
    The intermediate is newline-delimited JSON, not TSV, because sample values
    are real corpus text and may contain tabs or newlines that a TSV would eat.
    """
    gold = load_tsv(args.gold)
    wanted = sorted({(r["file_content_sha256"], r["column_name"]) for r in gold})
    print(f"{len(gold)} gold rows, {len(wanted)} distinct columns", file=sys.stderr)

    samples: dict[tuple[str, str], str] = {}
    with tempfile.TemporaryDirectory() as td:
        want_csv = Path(td) / "wanted.csv"
        with want_csv.open("w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow(["sha", "col"])
            w.writerows(wanted)
        out_json = Path(td) / "resolved.json"
        # ANY_VALUE would be scan-order dependent.  min() is deterministic, and
        # (sha, column_name) is unique in the parquet for these keys anyway —
        # 843 matching rows, 843 distinct keys — so the two agree here.
        sql = (
            "COPY (SELECT c.file_content_sha256 AS sha, c.column_name AS col, "
            "min(c.sample_values_truncated) AS vals "
            f"FROM read_parquet({str(args.columns)!r}) c "
            f"JOIN read_csv({str(want_csv)!r}, header=true) w "
            "ON c.file_content_sha256 = w.sha AND c.column_name = w.col "
            "GROUP BY 1, 2) "
            f"TO {str(out_json)!r} (FORMAT JSON);"
        )
        proc = subprocess.run(
            ["duckdb", "-c", sql], capture_output=True, text=True, timeout=1800
        )
        if proc.returncode != 0:
            print(f"duckdb failed: {proc.stderr.strip()[-500:]}", file=sys.stderr)
            return 2
        for line in out_json.read_text().splitlines():
            if not line.strip():
                continue
            rec = json.loads(line)
            vals = [v for v in (rec.get("vals") or "").split(SEP) if v != ""]
            if vals:
                samples[(rec["sha"], rec["col"])] = SEP.join(vals)

    rows: list[dict[str, str]] = []
    for r in gold:
        key = (r["file_content_sha256"], r["column_name"])
        joined = samples.get(key)
        if joined is None:
            continue
        rows.append(
            {
                "file_content_sha256": key[0],
                "column_name": key[1],
                "n_values": str(joined.count(SEP) + 1),
                "values": joined,
            }
        )
    write_tsv(args.out, rows, ["file_content_sha256", "column_name", "n_values", "values"])
    print(
        f"resolved {len(rows)}/{len(gold)} gold rows from {args.columns} -> {args.out}",
        file=sys.stderr,
    )
    return 0


# ── record emission ─────────────────────────────────────────────────────────


class ProfileError(RuntimeError):
    pass


def parse_profile_csv(stdout: str, column_name: str) -> dict[str, str]:
    """Pull the row for `column_name` out of `profile -o csv` stdout.

    Uses the csv module rather than `str.split(",")` because several emitted
    fields are free text that may contain commas (a runner-up label, a format
    string, a transform expression).  A naive split silently shifts every field
    after the first comma-bearing one.
    """
    reader = csv.reader(io.StringIO(stdout))
    rows = [r for r in reader if r]
    if not rows:
        raise ProfileError("profile emitted no CSV rows")
    header = rows[0]
    if header != PROFILE_FIELDS:
        raise ProfileError(
            f"profile CSV header changed: expected {PROFILE_FIELDS}, got {header}"
        )
    data: list[dict[str, str]] = []
    for row in rows[1:]:
        if len(row) != len(PROFILE_FIELDS):
            raise ProfileError(f"malformed CSV row ({len(row)} fields): {row!r}")
        data.append(dict(zip(PROFILE_FIELDS, row)))
    for rec in data:
        if rec["column"] == column_name:
            return rec
    # DuckDB's CSV sniffer treats an all-numeric first line as data, not a
    # header, and names the column `column0`.  Ten gold columns have names like
    # `1001` and `92.14`, so an exact-name lookup drops them.  A single-column
    # CSV has exactly one profile row and there is no ambiguity about which one
    # it is; the name the tool chose is kept in the `column` field and compared
    # like every other, so the rename is visible rather than papered over.
    if len(data) == 1:
        return data[0]
    raise ProfileError(
        f"no row for column {column_name!r} and {len(data)} rows to choose from"
    )


def profile_one(
    binary: Path, column_name: str, values: list[str], env: dict[str, str], cwd: Path
) -> dict[str, str]:
    with tempfile.TemporaryDirectory() as td:
        csv_path = Path(td) / "col.csv"
        with csv_path.open("w", newline="") as fh:
            w = csv.writer(fh, lineterminator="\n")
            w.writerow([column_name])
            for v in values:
                w.writerow([v])
        proc = subprocess.run(
            [str(binary), "profile", "-f", str(csv_path), "-o", "csv"],
            capture_output=True,
            text=True,
            timeout=300,
            env=env,
            cwd=str(cwd),
        )
    if proc.returncode != 0:
        raise ProfileError(f"exit {proc.returncode}: {proc.stderr.strip()[-300:]}")
    return parse_profile_csv(proc.stdout, column_name)


def cmd_emit(args: argparse.Namespace) -> int:
    samples = load_tsv(args.samples)
    env = dict(os.environ)
    env["FINETYPE_MODEL"] = str(args.model)
    cwd = args.cwd.resolve()
    binary = args.binary.resolve()
    if not binary.exists():
        print(f"binary not found: {binary}", file=sys.stderr)
        return 2
    if not (cwd / "labels").is_dir() or not (cwd / "models" / "model2vec").is_dir():
        print(
            f"--cwd {cwd} is not a finetype checkout (needs labels/ and "
            "models/model2vec/); profile resolves both relative to cwd",
            file=sys.stderr,
        )
        return 2

    total = len(samples)
    out: list[dict[str, str] | None] = [None] * total
    errors: list[str] = []

    def work(i: int) -> None:
        r = samples[i]
        vals = [v for v in r["values"].split(SEP) if v != ""]
        try:
            rec = profile_one(binary, r["column_name"], vals, env, cwd)
        except Exception as e:  # noqa: BLE001 - one bad column must not lose the run
            errors.append(f"{r['file_content_sha256'][:12]}/{r['column_name']}: {e}")
            return
        rec = dict(rec)
        rec["file_content_sha256"] = r["file_content_sha256"]
        rec["column_name"] = r["column_name"]
        out[i] = rec

    with ThreadPoolExecutor(max_workers=args.jobs) as pool:
        for n, _ in enumerate(pool.map(work, range(total)), 1):
            if n % 100 == 0:
                print(f"  ... {n}/{total}", file=sys.stderr)

    rows = [r for r in out if r is not None]
    write_tsv(args.out, rows, RECORD_FIELDS)
    print(f"emitted {len(rows)}/{total} records -> {args.out}", file=sys.stderr)
    for e in errors[:20]:
        print(f"  ERR {e}", file=sys.stderr)
    if errors:
        print(f"  ({len(errors)} errors total)", file=sys.stderr)
    # A partial run is not a measurement: fail loudly rather than let a caller
    # diff two files that silently cover different column sets.
    return 0 if not errors else 1


# ── record diff ─────────────────────────────────────────────────────────────


def _key(r: dict[str, str]) -> tuple[str, str]:
    return (r.get("file_content_sha256", ""), r.get("column_name", ""))


@dataclass
class DiffResult:
    rows_a: int
    rows_b: int
    only_a: list[tuple[str, str]]
    only_b: list[tuple[str, str]]
    common: list[tuple[str, str]]
    differ: dict[str, int]
    examples: dict[str, list[dict[str, str]]]
    max_abs_delta: dict[str, float]
    max_abs_delta_at: dict[str, str]
    whole_record_differ: int

    def to_json(self, unstable: list[str]) -> dict[str, object]:
        return {
            "rows_a": self.rows_a,
            "rows_b": self.rows_b,
            "common": len(self.common),
            "only_a": [list(k) for k in self.only_a],
            "only_b": [list(k) for k in self.only_b],
            "per_field": {
                f: {
                    "differ": self.differ[f],
                    "of": len(self.common),
                    "examples": self.examples[f],
                }
                for f in COMPARED_FIELDS
            },
            "max_abs_delta": self.max_abs_delta,
            "max_abs_delta_at": self.max_abs_delta_at,
            "whole_record_differ": self.whole_record_differ,
            "declared_unstable_fields": sorted(unstable),
        }


def diff_records(
    a_rows: list[dict[str, str]], b_rows: list[dict[str, str]]
) -> DiffResult:
    """Field-by-field comparison of two emitted record sets, keyed by identity.

    Rows are matched on (file_content_sha256, column_name), never on position:
    if either side dropped a column the positional zip would compare unrelated
    rows and report a large fake difference (or, worse, a small true-looking one).
    """
    a_by = {_key(r): r for r in a_rows}
    b_by = {_key(r): r for r in b_rows}
    only_a = sorted(k for k in a_by if k not in b_by)
    only_b = sorted(k for k in b_by if k not in a_by)
    common = sorted(k for k in a_by if k in b_by)

    differ: dict[str, int] = {f: 0 for f in COMPARED_FIELDS}
    examples: dict[str, list[dict[str, str]]] = {f: [] for f in COMPARED_FIELDS}
    max_abs: dict[str, float] = {f: 0.0 for f in NUMERIC_FIELDS}
    max_abs_at: dict[str, str] = {f: "" for f in NUMERIC_FIELDS}
    whole_record_differ = 0

    for k in common:
        ra, rb = a_by[k], b_by[k]
        row_moved = False
        for f in COMPARED_FIELDS:
            va, vb = ra.get(f, ""), rb.get(f, "")
            if va == vb:
                continue
            row_moved = True
            differ[f] += 1
            if len(examples[f]) < 25:
                examples[f].append(
                    {"column": k[1], "sha": k[0][:12], "a": va, "b": vb}
                )
            if f in NUMERIC_FIELDS:
                try:
                    d = abs(float(va) - float(vb))
                except ValueError:
                    d = float("inf")
                if d > max_abs[f]:
                    max_abs[f] = d
                    max_abs_at[f] = f"{k[1]} ({va} -> {vb})"
        if row_moved:
            whole_record_differ += 1

    return DiffResult(
        rows_a=len(a_rows),
        rows_b=len(b_rows),
        only_a=only_a,
        only_b=only_b,
        common=common,
        differ=differ,
        examples=examples,
        max_abs_delta=max_abs,
        max_abs_delta_at=max_abs_at,
        whole_record_differ=whole_record_differ,
    )


def _render(res: DiffResult, noise: set[str]) -> str:
    n = len(res.common)
    lines: list[str] = [f"rows: a={res.rows_a} b={res.rows_b} common={n}"]
    if res.only_a or res.only_b:
        lines.append(f"UNMATCHED: only_a={len(res.only_a)} only_b={len(res.only_b)}")
    lines.append("")
    lines.append(f"{'field':<16}{'differ':>10}  note")
    for f in COMPARED_FIELDS:
        note = ""
        if f in NUMERIC_FIELDS:
            at = res.max_abs_delta_at[f]
            note = f"max |delta| {res.max_abs_delta[f]:.6g}" + (
                f" at {at}" if at else ""
            )
        if f in noise:
            note = (note + "  " if note else "") + "[NOT run-to-run stable]"
        lines.append(f"{f:<16}{res.differ[f]:>6}/{n:<4}  {note}")
    lines.append("")
    lines.append(f"{'whole record':<16}{res.whole_record_differ:>6}/{n:<4}")
    return "\n".join(lines)


def cmd_diff(args: argparse.Namespace) -> int:
    a_rows = load_tsv(args.a)
    b_rows = load_tsv(args.b)
    res = diff_records(a_rows, b_rows)
    noise = set(args.noise_floor)
    print(_render(res, noise))
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        payload = res.to_json(list(noise))
        payload["a"] = str(args.a)
        payload["b"] = str(args.b)
        args.json.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
        print(f"\n-> {args.json}", file=sys.stderr)
    return 0


# ── self-test ───────────────────────────────────────────────────────────────

_FAILURES: list[str] = []


def check(name: str, cond: bool) -> None:
    print(f"  {'ok  ' if cond else 'FAIL'} {name}")
    if not cond:
        _FAILURES.append(name)


def _rec(**kw: str) -> dict[str, str]:
    base = {f: "" for f in RECORD_FIELDS}
    base["file_content_sha256"] = "aa" * 32
    base["column_name"] = "col"
    base["column"] = "col"
    base["type"] = "representation.numeric.integer_number"
    base["confidence"] = "0.5000"
    base["quality_band"] = "high"
    base["is_generic"] = "false"
    base["samples_used"] = "5"
    base["non_null"] = "5"
    base["null"] = "0"
    base.update(kw)
    return base


def cmd_self_test(_args: argparse.Namespace) -> int:
    print("encoder_dtype_record_diff self-test")

    # -- the defect this whole script exists to prevent -----------------------
    # A comparison that only reads the label passes here.  Each case below moves
    # exactly one non-label field and requires the diff to name it.
    for field, other in [
        ("confidence", "0.4370"),
        ("quality_band", "low"),
        ("runner_up", "geography.address.postal_code"),
        ("disambiguation", "numeric_postal_code_detection"),
        ("locale", "EN_NZ"),
        ("format_string", "%Y-%m-%d"),
        ("transform", "CAST({col} AS VARCHAR)"),
        ("broad_type", "VARCHAR"),
        ("is_generic", "true"),
        ("samples_used", "4"),
        ("non_null", "4"),
        ("null", "1"),
    ]:
        a = [_rec()]
        b = [_rec(**{field: other})]
        res = diff_records(a, b)
        moved = {f for f in COMPARED_FIELDS if res.differ[f] > 0}
        check(
            f"a change to {field} alone is reported as {field} and nothing else",
            moved == {field},
        )
        check(
            f"a change to {field} alone counts as one whole-record difference",
            res.whole_record_differ == 1,
        )

    # Identical records must report zero, or every case above proves nothing.
    res = diff_records([_rec()], [_rec()])
    check(
        "identical records report no difference in any field",
        all(res.differ[f] == 0 for f in COMPARED_FIELDS)
        and res.whole_record_differ == 0,
    )

    # -- numeric magnitude ----------------------------------------------------
    res = diff_records([_rec(confidence="0.4376")], [_rec(confidence="0.4370")])
    check(
        "the confidence delta is a magnitude, not a flag (0.4376 -> 0.4370 = 0.0006)",
        abs(res.max_abs_delta["confidence"] - 0.0006) < 1e-9,
    )
    res = diff_records(
        [_rec(confidence="0.4376"), _rec(column_name="c2", column="c2", confidence="0.9000")],
        [_rec(confidence="0.4370"), _rec(column_name="c2", column="c2", confidence="0.8000")],
    )
    check(
        "max |delta| is the maximum over rows, not the first or the last",
        abs(res.max_abs_delta["confidence"] - 0.1) < 1e-9,
    )

    # -- identity matching, not position -------------------------------------
    # The realistic wrong implementation is `zip(a_rows, b_rows)`.  It passes on
    # aligned inputs; it silently compares unrelated columns when one side drops
    # a row, which is exactly what a partially-failed emit produces.
    a = [
        _rec(column_name="c1", column="c1", type="t1"),
        _rec(column_name="c2", column="c2", type="t2"),
        _rec(column_name="c3", column="c3", type="t3"),
    ]
    b = [
        _rec(column_name="c1", column="c1", type="t1"),
        _rec(column_name="c3", column="c3", type="t3"),
    ]
    res = diff_records(a, b)
    check(
        "a row missing from one side is reported as unmatched, not silently realigned",
        len(res.common) == 2 and len(res.only_a) == 1 and res.only_a[0][1] == "c2",
    )
    check(
        "the rows that DO match are still compared correctly around the gap",
        res.differ["type"] == 0,
    )

    # -- empty vs non-empty ---------------------------------------------------
    # A comparison that skips empty fields ("nothing to compare") passes on a
    # regression that stops emitting a field at all.
    res = diff_records([_rec(runner_up="")], [_rec(runner_up="x")])
    check(
        "a field going from empty to non-empty is a difference",
        res.differ["runner_up"] == 1,
    )
    res = diff_records([_rec(disambiguation="x")], [_rec(disambiguation="")])
    check(
        "a field going from non-empty to empty is a difference",
        res.differ["disambiguation"] == 1,
    )

    # -- CSV parsing ----------------------------------------------------------
    header = ",".join(PROFILE_FIELDS)
    body = (
        '"amt","finance.money.amount",0.9000,"high","representation.numeric.decimal_number",'
        '"DOUBLE","#,##0.00","CAST({col} AS DOUBLE)",false,5,5,0,"currency_symbol","EN_US"'
    )
    rec = parse_profile_csv(header + "\n" + body + "\n", "amt")
    check(
        "a quoted field containing a comma does not shift the parse (format_string)",
        rec["format_string"] == "#,##0.00",
    )
    check(
        "the fields after a comma-bearing one are still correct (locale)",
        rec["locale"] == "EN_US" and rec["disambiguation"] == "currency_symbol",
    )

    two_cols = (
        header
        + "\n"
        + '"a","representation.text.RESIDUAL",0.1000,"low","","VARCHAR","","",true,5,5,0,"",""\n'
        + '"b","finance.money.amount",0.9000,"high","","DOUBLE","","",false,5,5,0,"",""\n'
    )
    rec = parse_profile_csv(two_cols, "b")
    check(
        "the row for the requested column is selected, not the first row",
        rec["type"] == "finance.money.amount",
    )

    try:
        parse_profile_csv("column,type\n\"a\",\"b\"\n", "a")
        ok = False
    except ProfileError:
        ok = True
    check("a changed CSV header is refused, not parsed into shifted fields", ok)

    try:
        parse_profile_csv(header + "\n", "a")
        ok = False
    except ProfileError:
        ok = True
    check("a missing column row raises rather than returning an empty record", ok)

    # -- the sniffer rename ---------------------------------------------------
    # An all-numeric header ("1001") is treated as data by DuckDB's CSV sniffer
    # and the column comes back as `column0`.  The single row is accepted, and
    # the name the tool chose is what lands in the record.
    renamed = (
        header
        + "\n"
        + '"column0","representation.numeric.integer_number",0.5786,"low",'
        + '"datetime.component.year","BIGINT","","CAST({col} AS BIGINT)",true,4,4,0,'
        + '"increment_substance_veto",""\n'
    )
    rec = parse_profile_csv(renamed, "1001")
    check(
        "a single-row profile is accepted when the sniffer renamed the column",
        rec["type"] == "representation.numeric.integer_number",
    )
    check(
        "the renamed column keeps the name the tool emitted, not the one asked for",
        rec["column"] == "column0",
    )
    try:
        parse_profile_csv(two_cols, "zzz")
        ok = False
    except ProfileError:
        ok = True
    check(
        "the single-row fallback does NOT fire when there is more than one row",
        ok,
    )
    res = diff_records([_rec(column="column0")], [_rec(column="col")])
    check(
        "the emitted column name is itself compared, so a rename is a difference",
        res.differ["column"] == 1,
    )

    # -- round trip through the TSV writer ------------------------------------
    # The diff reads what emit wrote; a writer that drops a field would make
    # every comparison of that field trivially equal.
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "r.tsv"
        rows = [_rec(transform="CAST({col} AS VARCHAR)", format_string="#,##0.00")]
        write_tsv(p, rows, RECORD_FIELDS)
        back = load_tsv(p)
        check(
            "every compared field survives the TSV round trip",
            len(back) == 1
            and all(back[0][f] == rows[0][f] for f in COMPARED_FIELDS),
        )
        res = diff_records(back, [_rec(transform="", format_string="#,##0.00")])
        check(
            "a field that only differs after the round trip is still detected",
            res.differ["transform"] == 1,
        )

    # -- the rendered report actually carries the numbers ---------------------
    res = diff_records([_rec(confidence="0.4376")], [_rec(confidence="0.4370")])
    text = _render(res, {"locale"})
    check("the report prints the differing count for confidence", "1/1" in text)
    check("the report prints the max |delta|", "0.0006" in text)
    check(
        "the report marks a declared-unstable field so locale churn is not read as effect",
        "[NOT run-to-run stable]" in text
        and text.splitlines()[[i for i, l in enumerate(text.splitlines())
                               if l.startswith("locale")][0]].endswith(
            "[NOT run-to-run stable]"),
    )

    print()
    if _FAILURES:
        print(f"{len(_FAILURES)} FAILED:")
        for f in _FAILURES:
            print(f"  - {f}")
        return 1
    print("all cases pass")
    return 0


# ── cli ─────────────────────────────────────────────────────────────────────


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("prepare", help="resolve gold columns to sample values")
    p.add_argument("--gold", type=Path, default=Path("eval/gold/gold_corpus.tsv"))
    p.add_argument(
        "--columns", type=Path, default=Path("eval/gittables/corpus_pass/columns.parquet")
    )
    p.add_argument("--out", type=Path, required=True)
    p.set_defaults(func=cmd_prepare)

    p = sub.add_parser("emit", help="profile every prepared column and record the whole row")
    p.add_argument("--binary", type=Path, required=True)
    p.add_argument("--model", type=Path, required=True, help="FINETYPE_MODEL directory")
    p.add_argument("--samples", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True)
    p.add_argument("--jobs", type=int, default=4)
    p.add_argument("--cwd", type=Path, default=Path("."))
    p.set_defaults(func=cmd_emit)

    p = sub.add_parser("diff", help="field-by-field diff of two emitted record sets")
    p.add_argument("--a", type=Path, required=True)
    p.add_argument("--b", type=Path, required=True)
    p.add_argument("--json", type=Path)
    p.add_argument(
        "--noise-floor",
        nargs="*",
        default=[],
        help="fields already shown to move on a same-binary/same-model repeat",
    )
    p.set_defaults(func=cmd_diff)

    p = sub.add_parser("self-test", help="run the built-in cases")
    p.set_defaults(func=cmd_self_test)

    args = ap.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
