#!/usr/bin/env python3
"""evidence.py — the fixture manifest behind every accuracy number this repo quotes.

An accuracy bar is a property of a (model, fixture, binary) triple, never a constant
in a script. Ground truth moves: rows get re-adjudicated, columns get added, and a
float copied into a shell script silently becomes a false-rejection generator the
moment the fixture underneath it changes.

This tool is the registry. Fixture *versions* are content-addressed by sha256, so a
run resolves which version of the gold corpus it is about to score against and prints
that id next to every number it emits. Baselines are recorded per fixture version, so
a comparison is only ever offered when the bar and the candidate were measured on the
same ground truth.

Stdlib only — it runs under any python3, no venv required.

Manifest: evidence/fixtures.json

    resolve-fixture   --path P                      which registered version is this file?
    register-fixture  --id ID (--path P | --sha256 H --rows N)
    list                                            what is registered
    get-baseline      --fixture ID --key K          a recorded bar, or exit 4
    record-baseline   --fixture ID --key K --correct N --scored N ...
    write-headline    --out F --set k=v ...         a fixture-stamped score record
    verify                                          manifest self-consistency

Exit codes: 0 ok · 2 usage · 3 fixture not registered · 4 baseline not recorded ·
            5 manifest inconsistent
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import sys
from pathlib import Path

SCHEMA = "finetype/evidence-fixtures@1"
DEFAULT_MANIFEST = Path(__file__).resolve().parent.parent / "evidence" / "fixtures.json"

EXIT_USAGE = 2
EXIT_NO_FIXTURE = 3
EXIT_NO_BASELINE = 4
EXIT_INCONSISTENT = 5

BASELINE_FIELDS = ("model", "binary", "pipeline", "source", "note")


def die(code: int, *lines: str) -> None:
    for line in lines:
        print(line, file=sys.stderr)
    sys.exit(code)


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def data_rows_of(path: Path) -> int:
    """Data rows in a headered TSV: total lines minus the header line."""
    n = 0
    with path.open("rb") as fh:
        for _ in fh:
            n += 1
    return max(0, n - 1)


def load(manifest: Path) -> dict:
    if not manifest.exists():
        die(
            EXIT_INCONSISTENT,
            f"FAIL: no fixture manifest at {manifest}.",
            "      Every published score names the fixture version it was measured on;",
            "      without the manifest there is nothing to name. Create it with",
            "      register-fixture.",
        )
    try:
        doc = json.loads(manifest.read_text())
    except json.JSONDecodeError as exc:
        die(EXIT_INCONSISTENT, f"FAIL: {manifest} is not valid JSON: {exc}")
    if doc.get("schema") != SCHEMA:
        die(
            EXIT_INCONSISTENT,
            f"FAIL: {manifest} declares schema {doc.get('schema')!r}, expected {SCHEMA!r}.",
        )
    if not isinstance(doc.get("fixtures"), dict):
        die(EXIT_INCONSISTENT, f"FAIL: {manifest} has no 'fixtures' object.")
    return doc


def save(manifest: Path, doc: dict) -> None:
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")


def find_by_sha(doc: dict, sha: str) -> tuple[str, dict] | None:
    for fid, fx in doc["fixtures"].items():
        if fx.get("sha256") == sha:
            return fid, fx
    return None


def cmd_resolve_fixture(args) -> int:
    path = Path(args.path)
    if not path.exists():
        die(EXIT_USAGE, f"FAIL: no such file: {path}")
    sha = sha256_of(path)
    rows = data_rows_of(path)
    doc = load(args.manifest)
    hit = find_by_sha(doc, sha)
    if hit is None:
        known = ", ".join(doc["fixtures"]) or "(none)"
        die(
            EXIT_NO_FIXTURE,
            f"FAIL: {path} is not a registered fixture version.",
            f"      sha256={sha}  rows={rows}",
            f"      registered versions: {known}",
            "      A score measured on an unregistered fixture cannot be falsified later,",
            "      because nobody can tell which ground truth produced it. Register this",
            "      version first, naming what changed:",
            f"        scripts/evidence.py register-fixture --path {path} \\",
            f"            --id gold-{dt.date.today().isoformat()[2:]} --note '<what changed>'",
        )
    fid, fx = hit
    if fx.get("rows") != rows:
        die(
            EXIT_INCONSISTENT,
            f"FAIL: {path} matches fixture {fid} by sha256 but has {rows} data rows,",
            f"      while the manifest records {fx.get('rows')}. One of the two is wrong.",
        )
    if args.format == "tsv":
        print(f"{fid}\t{sha}\t{rows}\t{fx.get('path', str(path))}")
    else:
        print(fid)
    return 0


def cmd_register_fixture(args) -> int:
    if args.path:
        path = Path(args.path)
        if not path.exists():
            die(EXIT_USAGE, f"FAIL: no such file: {path}")
        sha = sha256_of(path)
        rows = data_rows_of(path)
        rel = args.record_path or str(path)
    else:
        if not (args.sha256 and args.rows is not None):
            die(EXIT_USAGE, "FAIL: give --path, or both --sha256 and --rows.")
        sha, rows, rel = args.sha256, args.rows, (args.record_path or "")
        if not rel:
            die(EXIT_USAGE, "FAIL: --record-path is required when registering by hash.")

    manifest = args.manifest
    doc = (
        load(manifest)
        if manifest.exists()
        else {
            "schema": SCHEMA,
            "description": (
                "Fixture versions this repo measures against, content-addressed by sha256, "
                "with the baselines recorded on each. A bar belongs to a "
                "(model, fixture, binary) triple; it is never a constant in a script."
            ),
            "fixtures": {},
        }
    )

    clash = find_by_sha(doc, sha)
    if clash and clash[0] != args.id:
        die(
            EXIT_USAGE,
            f"FAIL: sha256 {sha} is already registered as fixture {clash[0]!r}.",
            "      One content hash, one fixture id.",
        )
    if args.id in doc["fixtures"] and not args.force:
        die(
            EXIT_USAGE,
            f"FAIL: fixture id {args.id!r} already exists (sha256 "
            f"{doc['fixtures'][args.id].get('sha256')}).",
            "      Fixture ids are immutable — pick a new id, or pass --force to correct a "
            "mistake.",
        )

    entry = doc["fixtures"].get(args.id, {})
    entry.update(
        {
            "path": rel,
            "sha256": sha,
            "rows": rows,
            "registered": args.date or dt.date.today().isoformat(),
            "note": args.note or entry.get("note", ""),
        }
    )
    entry.setdefault("baselines", {})
    doc["fixtures"][args.id] = entry
    save(manifest, doc)
    print(f"registered fixture {args.id}  sha256={sha}  rows={rows}")
    return 0


def cmd_list(args) -> int:
    doc = load(args.manifest)
    for fid, fx in doc["fixtures"].items():
        print(f"{fid}\t{fx['sha256'][:12]}…\trows={fx['rows']}\t{fx.get('path', '')}")
        for key, b in fx.get("baselines", {}).items():
            print(f"    {key}\t{b['correct']}/{b['scored']} = {b['score']:.3f}\t{b.get('source', '')}")
    return 0


def _get_fixture(doc: dict, fid: str) -> dict:
    fx = doc["fixtures"].get(fid)
    if fx is None:
        die(
            EXIT_NO_FIXTURE,
            f"FAIL: no fixture {fid!r} in the manifest.",
            f"      registered versions: {', '.join(doc['fixtures']) or '(none)'}",
        )
    return fx


def cmd_get_baseline(args) -> int:
    doc = load(args.manifest)
    fx = _get_fixture(doc, args.fixture)
    b = fx.get("baselines", {}).get(args.key)
    if b is None:
        keys = ", ".join(fx.get("baselines", {})) or "(none recorded)"
        die(
            EXIT_NO_BASELINE,
            f"FAIL: baseline {args.key!r} has no recorded score on fixture {args.fixture!r}.",
            f"      baselines recorded on {args.fixture}: {keys}",
            "      A bar measured on one fixture version says nothing about another. Measure",
            "      this baseline on this fixture and record it with record-baseline.",
        )
    if args.format == "tsv":
        fields = [f"{b['score']:.3f}"]
        fields += [str(b.get(k, "")) for k in ("correct", "scored", "model", "binary", "pipeline", "source")]
        print("\t".join(fields))
    else:
        print(f"{b['score']:.3f}")
    return 0


def cmd_record_baseline(args) -> int:
    doc = load(args.manifest)
    fx = _get_fixture(doc, args.fixture)
    if args.scored <= 0:
        die(EXIT_USAGE, "FAIL: --scored must be positive.")
    if not 0 <= args.correct <= args.scored:
        die(EXIT_USAGE, f"FAIL: --correct {args.correct} is not in 0..{args.scored}.")
    baselines = fx.setdefault("baselines", {})
    if args.key in baselines and not args.force:
        prev = baselines[args.key]
        die(
            EXIT_USAGE,
            f"FAIL: baseline {args.key!r} is already recorded on {args.fixture!r} as "
            f"{prev['correct']}/{prev['scored']} = {prev['score']:.3f}.",
            "      A recorded measurement is history. Use a new key, or --force to correct a "
            "transcription error.",
        )
    entry = {
        "correct": args.correct,
        "scored": args.scored,
        "score": round(args.correct / args.scored, 3),
        "measured": args.date or dt.date.today().isoformat(),
    }
    for field in BASELINE_FIELDS:
        val = getattr(args, field, None)
        if val:
            entry[field] = val
    baselines[args.key] = entry
    save(args.manifest, doc)
    print(
        f"recorded {args.key} on {args.fixture}: "
        f"{entry['correct']}/{entry['scored']} = {entry['score']:.3f}"
    )
    return 0


def _coerce(text: str):
    try:
        return int(text)
    except ValueError:
        pass
    try:
        return float(text)
    except ValueError:
        return text


def cmd_write_headline(args) -> int:
    doc: dict = {}
    for pair in args.set:
        if "=" not in pair:
            die(EXIT_USAGE, f"FAIL: --set expects key=value, got {pair!r}")
        key, _, value = pair.partition("=")
        cursor = doc
        parts = key.split(".")
        for part in parts[:-1]:
            cursor = cursor.setdefault(part, {})
            if not isinstance(cursor, dict):
                die(EXIT_USAGE, f"FAIL: --set key {key!r} collides with a scalar")
        cursor[parts[-1]] = _coerce(value)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
    return 0


def cmd_verify(args) -> int:
    doc = load(args.manifest)
    problems: list[str] = []
    seen_sha: dict[str, str] = {}
    for fid, fx in doc["fixtures"].items():
        for field in ("path", "sha256", "rows", "registered"):
            if field not in fx:
                problems.append(f"{fid}: missing {field!r}")
        sha = fx.get("sha256", "")
        if len(sha) != 64 or any(c not in "0123456789abcdef" for c in sha):
            problems.append(f"{fid}: sha256 {sha!r} is not a 64-char lowercase hex digest")
        if sha in seen_sha:
            problems.append(f"{fid}: shares sha256 with {seen_sha[sha]}")
        seen_sha[sha] = fid
        if not isinstance(fx.get("rows"), int) or fx.get("rows", 0) <= 0:
            problems.append(f"{fid}: rows must be a positive integer")
        for key, b in fx.get("baselines", {}).items():
            for field in ("correct", "scored", "score"):
                if field not in b:
                    problems.append(f"{fid}/{key}: missing {field!r}")
            if not all(f in b for f in ("correct", "scored", "score")):
                continue
            if not 0 <= b["correct"] <= b["scored"]:
                problems.append(f"{fid}/{key}: correct {b['correct']} outside 0..{b['scored']}")
                continue
            expect = round(b["correct"] / b["scored"], 3)
            if abs(expect - b["score"]) > 5e-4:
                problems.append(
                    f"{fid}/{key}: score {b['score']} != {b['correct']}/{b['scored']} = {expect}"
                )
            if b["scored"] > fx.get("rows", 0):
                problems.append(
                    f"{fid}/{key}: scored {b['scored']} exceeds the fixture's {fx.get('rows')} rows"
                )
            if not b.get("source"):
                problems.append(f"{fid}/{key}: no 'source' — an unsourced bar is not evidence")

    # Several fixture versions legitimately share one path — they are the same file at
    # different points in time. What must hold is that whatever is on that path *now*
    # is registered under some id: an unregistered working copy is precisely how a
    # score stops being attributable to a known ground truth.
    for path_str in sorted({fx.get("path", "") for fx in doc["fixtures"].values() if fx.get("path")}):
        path = Path(path_str)
        if not path.is_file():
            continue
        live = sha256_of(path)
        match = [fid for fid, fx in doc["fixtures"].items() if fx.get("sha256") == live]
        if not match:
            problems.append(
                f"{path}: the file on disk (sha256 {live[:12]}…, {data_rows_of(path)} data rows) "
                "is not registered under any fixture id — register this version before "
                "measuring anything on it"
            )
        elif len(match) > 1:
            problems.append(f"{path}: sha256 {live[:12]}… is registered twice: {', '.join(match)}")
        else:
            rows = data_rows_of(path)
            if doc["fixtures"][match[0]].get("rows") != rows:
                problems.append(
                    f"{match[0]}: recorded rows={doc['fixtures'][match[0]].get('rows')} but "
                    f"{path} has {rows} data rows"
                )

    if problems:
        die(
            EXIT_INCONSISTENT,
            f"FAIL: {args.manifest} is inconsistent:",
            *(f"  - {p}" for p in problems),
        )
    n_base = sum(len(fx.get("baselines", {})) for fx in doc["fixtures"].values())
    print(f"ok: {len(doc['fixtures'])} fixture versions, {n_base} recorded baselines")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("resolve-fixture", help="which registered version is this file?")
    p.add_argument("--path", required=True)
    p.add_argument("--format", choices=("id", "tsv"), default="id")
    p.set_defaults(fn=cmd_resolve_fixture)

    p = sub.add_parser("register-fixture", help="register a new fixture version")
    p.add_argument("--id", required=True)
    p.add_argument("--path")
    p.add_argument("--sha256")
    p.add_argument("--rows", type=int)
    p.add_argument("--record-path", help="path to record when registering by hash")
    p.add_argument("--note", default="")
    p.add_argument("--date")
    p.add_argument("--force", action="store_true")
    p.set_defaults(fn=cmd_register_fixture)

    p = sub.add_parser("list", help="what is registered")
    p.set_defaults(fn=cmd_list)

    p = sub.add_parser("get-baseline", help="a recorded bar for a fixture version")
    p.add_argument("--fixture", required=True)
    p.add_argument("--key", required=True)
    p.add_argument("--format", choices=("score", "tsv"), default="score")
    p.set_defaults(fn=cmd_get_baseline)

    p = sub.add_parser("record-baseline", help="record a measurement against a fixture version")
    p.add_argument("--fixture", required=True)
    p.add_argument("--key", required=True)
    p.add_argument("--correct", type=int, required=True)
    p.add_argument("--scored", type=int, required=True)
    for field in BASELINE_FIELDS:
        p.add_argument(f"--{field}", default="")
    p.add_argument("--date")
    p.add_argument("--force", action="store_true")
    p.set_defaults(fn=cmd_record_baseline)

    p = sub.add_parser("write-headline", help="write a fixture-stamped score record")
    p.add_argument("--out", required=True)
    p.add_argument("--set", action="append", default=[], metavar="KEY=VALUE")
    p.set_defaults(fn=cmd_write_headline)

    p = sub.add_parser("verify", help="manifest self-consistency")
    p.set_defaults(fn=cmd_verify)

    args = ap.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
