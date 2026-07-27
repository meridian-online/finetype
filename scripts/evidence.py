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
    taxonomy-version  [--root R]                    the label vocabulary's content version
    render-release    --binary V [--check]          the readable report, rendered from the manifest
    verify                                          manifest self-consistency

Exit codes: 0 ok · 2 usage · 3 fixture not registered · 4 baseline not recorded ·
            5 manifest inconsistent
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
import re
import sys
from pathlib import Path

SCHEMA = "finetype/evidence-fixtures@1"
REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_EVIDENCE_DIR = REPO_ROOT / "evidence"
DEFAULT_MANIFEST = DEFAULT_EVIDENCE_DIR / "fixtures.json"

EXIT_USAGE = 2
EXIT_NO_FIXTURE = 3
EXIT_NO_BASELINE = 4
EXIT_INCONSISTENT = 5

BASELINE_FIELDS = ("model", "binary", "pipeline", "source", "note")

# The taxonomy is not a numbered artefact — it is the seven tracked
# labels/definitions_*.yaml files, compiled into the binary by include_str!. So its
# "version" is its content: a digest over those files, which cannot be typed in wrong
# because the printed id is derived from the digest.
TAXONOMY_DIR = "labels"
TAXONOMY_GLOB = "definitions_*.yaml"
# A taxonomy label is a top-level YAML key: dotted, at column zero, nothing after the colon.
TAXONOMY_KEY_RE = re.compile(r"^([A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)+):$")

# evidence/ is the tracked record, not a second output directory. These are the standing
# limits, enforced by `verify` rather than left to judgement: the moment evidence/ holds
# megabytes the split is in the wrong place, and the fix is to move artefacts back to
# output/ and keep the report.
EVIDENCE_TEXT_SUFFIXES = (".md", ".json", ".tsv", ".csv", ".txt")
EVIDENCE_MAX_FILE_BYTES = 200 * 1024
EVIDENCE_MAX_TOTAL_BYTES = 1024 * 1024


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


def taxonomy_files(root: Path) -> list[Path]:
    return sorted((root / TAXONOMY_DIR).glob(TAXONOMY_GLOB), key=lambda p: p.name)


def taxonomy_labels(path: Path) -> list[str]:
    """Top-level keys of one definitions file — one per taxonomy type."""
    out = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = TAXONOMY_KEY_RE.match(line)
        if m:
            out.append(m.group(1))
    return out


def taxonomy_digest(root: Path) -> dict:
    """Content-address the label vocabulary.

    The digest covers each file's name, byte length and bytes, in name order, so a
    rename, a truncation or an edit all move it. The version id is derived from the
    digest — there is no separate number anyone can forget to bump, or type in wrong.
    """
    files = taxonomy_files(root)
    if not files:
        die(
            EXIT_USAGE,
            f"FAIL: no {TAXONOMY_DIR}/{TAXONOMY_GLOB} under {root}.",
            "      The taxonomy is those files; without them there is no version to record.",
        )
    h = hashlib.sha256()
    labels: set[str] = set()
    for path in files:
        data = path.read_bytes()
        h.update(f"{path.name}\n{len(data)}\n".encode())
        h.update(data)
        labels.update(taxonomy_labels(path))
    sha = h.hexdigest()
    return {
        "version": "tax-" + sha[:12],
        "sha256": sha,
        "types": len(labels),
        "sources": [f"{TAXONOMY_DIR}/{p.name}" for p in files],
        "labels": sorted(labels),
    }


def taxonomy_block(root: Path, commit: str = "") -> dict:
    """The subset of the digest that gets recorded on a fixture (no label list).

    `commit` names where the vocabulary was read from, so a historical stamp taken from
    a git blob is distinguishable from one taken off the working tree.
    """
    d = taxonomy_digest(root)
    block = {k: d[k] for k in ("version", "sha256", "types", "sources")}
    if commit:
        block["commit"] = commit
    return block


def fixture_labels(path: Path, column: str) -> list[str]:
    """Distinct curated labels in a headered TSV fixture, in sorted order."""
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        if reader.fieldnames is None or column not in reader.fieldnames:
            return []
        return sorted({row[column] for row in reader if row.get(column)})


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

    prev = doc["fixtures"].get(args.id, {})
    # Rebuilt in a fixed key order rather than updated in place, so re-registering a
    # fixture produces the same bytes as registering it fresh — a manifest that reorders
    # itself makes every later diff unreadable.
    entry = {
        "version": args.id,
        "path": rel,
        "sha256": sha,
        "rows": rows,
        "registered": args.date or dt.date.today().isoformat(),
        "taxonomy": taxonomy_block(Path(args.taxonomy_root), args.taxonomy_commit),
        "note": args.note or prev.get("note", ""),
    }
    column = args.label_column or prev.get("label_column")
    if column:
        entry["label_column"] = column
        # Only countable when the fixture is the file on disk; registering a historical
        # version by hash cannot count labels in a blob it does not have.
        if args.path and Path(args.path).is_file() and sha256_of(Path(args.path)) == sha:
            entry["labels_used"] = len(fixture_labels(Path(args.path), column))
    entry["baselines"] = prev.get("baselines", {})
    doc["fixtures"][args.id] = entry
    save(manifest, doc)
    tax = entry["taxonomy"]
    print(
        f"registered fixture {args.id}  sha256={sha}  rows={rows}  "
        f"taxonomy={tax['version']} ({tax['types']} types)"
    )
    return 0


def cmd_taxonomy_version(args) -> int:
    d = taxonomy_digest(Path(args.root))
    if args.format == "json":
        print(json.dumps({k: v for k, v in d.items() if k != "labels"}, indent=2))
    elif args.format == "tsv":
        print(f"{d['version']}\t{d['sha256']}\t{d['types']}")
    else:
        print(d["version"])
    return 0


def cmd_list(args) -> int:
    doc = load(args.manifest)
    for fid, fx in doc["fixtures"].items():
        tax = fx.get("taxonomy", {})
        print(
            f"{fid}\t{fx['sha256'][:12]}…\trows={fx['rows']}\t"
            f"{tax.get('version', 'taxonomy=?')}\t{fx.get('path', '')}"
        )
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


def release_report_path(evidence_dir: Path, binary: str) -> Path:
    return evidence_dir / f"release-{binary}.md"


def render_release(doc: dict, binary: str) -> str:
    """The readable report, derived entirely from the manifest.

    Nothing here is typed by hand, which is the point: a number in the prose that the
    manifest does not carry is exactly the transcription error this directory exists to
    prevent. `verify` re-renders and diffs, so editing one without the other fails CI.
    """
    rows = []  # (fixture id, fixture, key, baseline)
    for fid in sorted(doc["fixtures"]):
        fx = doc["fixtures"][fid]
        for key in sorted(fx.get("baselines", {})):
            b = fx["baselines"][key]
            if b.get("binary") == binary:
                rows.append((fid, fx, key, b))

    out: list[str] = []
    w = out.append
    w(
        f"> Generated from `evidence/fixtures.json` by "
        f"`scripts/evidence.py render-release --binary {binary}`. Do not hand-edit — "
        f"`scripts/evidence.py verify` re-renders this file and fails if it has drifted "
        f"from the manifest."
    )
    w("")
    w(f"# finetype {binary} — what it measured at")
    w("")
    if not rows:
        w(f"No baseline in the manifest was measured on binary `{binary}`.")
        w("")
        return "\n".join(out)

    w(
        "**Every score below names the gold fixture version it was measured on.** A score "
        "without a fixture version is not evidence: ground truth moves under a bar, and a "
        "float remembered without its fixture becomes a false-rejection generator. Two "
        "scores measured on different fixture versions are not comparable, and this report "
        "refuses to subtract them rather than presenting the difference as a result."
    )
    w("")
    w("## Headline")
    w("")
    w("| Fixture version | Taxonomy | Model | Pipeline | Correct / scored | Accuracy | Measured |")
    w("|---|---|---|---|---:|---:|---|")
    for fid, fx, _key, b in rows:
        tax = fx.get("taxonomy", {}).get("version", "—")
        w(
            f"| `{fid}` | `{tax}` | `{b.get('model', '—')}` | {b.get('pipeline', '—')} | "
            f"{b['correct']} / {b['scored']} | **{b['score']:.3f}** | {b.get('measured', '—')} |"
        )
    w("")
    for fid, fx, key, b in rows:
        if b.get("note"):
            w(f"- `{fid}` · `{key}` — {b['note']}")
    w("")

    w("## Same-fixture comparison")
    w("")
    w(
        "A delta is only meaningful when the fixture version and the pipeline are held "
        "fixed and the binary is the only thing that moved. These are the pairs in the "
        "manifest that satisfy that."
    )
    w("")
    pairs = []
    for fid, fx, _key, b in rows:
        for okey in sorted(fx.get("baselines", {})):
            o = fx["baselines"][okey]
            if o.get("binary") == binary or o.get("pipeline") != b.get("pipeline"):
                continue
            if o.get("scored") != b.get("scored"):
                continue
            pairs.append((fid, o, b))
    if pairs:
        w("| Fixture version | Against | Then | Now | Δ columns | Δ accuracy |")
        w("|---|---|---:|---:|---:|---:|")
        for fid, o, b in pairs:
            dc = b["correct"] - o["correct"]
            da = b["score"] - o["score"]
            w(
                f"| `{fid}` | `{o.get('binary', '—')}` | {o['correct']}/{o['scored']} = "
                f"{o['score']:.3f} | {b['correct']}/{b['scored']} = {b['score']:.3f} | "
                f"{dc:+d} | {da:+.3f} |"
            )
    else:
        w(
            f"None. No other binary has a baseline recorded on the same fixture version and "
            f"pipeline as `{binary}`, so there is no honest delta to state."
        )
    w("")

    refused = []
    refused_seen: set[frozenset] = set()
    seen_fixtures = {fid for fid, _fx, _k, _b in rows}
    for fid, _fx, key, b in rows:
        for ofid in sorted(doc["fixtures"]):
            if ofid == fid:
                continue
            for okey in sorted(doc["fixtures"][ofid].get("baselines", {})):
                o = doc["fixtures"][ofid]["baselines"][okey]
                if o.get("pipeline") != b.get("pipeline"):
                    continue
                # One pair, stated once — A-vs-B and B-vs-A are the same refusal.
                token = frozenset({(fid, key), (ofid, okey)})
                if token in refused_seen:
                    continue
                refused_seen.add(token)
                refused.append((fid, b, ofid, o, okey))
    if refused:
        w("### Refused comparisons")
        w("")
        w(
            "Same pipeline, different ground truth. The difference between these numbers is "
            "not a measurement of anything, so it is not stated as one."
        )
        w("")
        for fid, b, ofid, o, okey in refused:
            w(
                f"- `{fid}` {b['correct']}/{b['scored']} = {b['score']:.3f} vs `{ofid}` "
                f"{o['correct']}/{o['scored']} = {o['score']:.3f} (`{okey}`) — different "
                f"fixture versions."
            )
        w("")

    w("## Fixture versions cited")
    w("")
    w("| Version | Rows | Taxonomy | Types | Content hash (sha256) | Path |")
    w("|---|---:|---|---:|---|---|")
    for fid in sorted(seen_fixtures):
        fx = doc["fixtures"][fid]
        tax = fx.get("taxonomy", {})
        w(
            f"| `{fid}` | {fx.get('rows', '—')} | `{tax.get('version', '—')}` | "
            f"{tax.get('types', '—')} | `{fx.get('sha256', '')}` | `{fx.get('path', '')}` |"
        )
    w("")
    for fid in sorted(seen_fixtures):
        note = doc["fixtures"][fid].get("note")
        if note:
            w(f"- `{fid}` — {note}")
    w("")
    w("## Reproducing this")
    w("")
    w("Every score above cites its `source` in the manifest:")
    w("")
    for fid, _fx, key, b in rows:
        w(f"- `{fid}` · `{key}` — {b.get('source', '(none)')}")
    w("")
    w(
        "The fixture is content-addressed, so a checkout can confirm it is holding the same "
        "ground truth before it measures anything:"
    )
    w("")
    w("```sh")
    w("scripts/evidence.py resolve-fixture --path eval/gold/gold_corpus.tsv")
    w("scripts/evidence.py taxonomy-version")
    w("scripts/evidence.py list")
    w("```")
    w("")
    w(
        "The run artefacts behind these numbers — FTMBs, prediction TSVs, per-column report "
        "tables — are regenerable and live in the ignored `output/`. What is tracked here is "
        "the part that cannot be regenerated: which ground truth this was, and what it came "
        "out as."
    )
    w("")
    w("## What this does not record")
    w("")
    w(
        "The taxonomy column above is the vocabulary **the fixture was adjudicated under**, "
        "read from the commit in each fixture's `taxonomy.commit`. It is not the vocabulary "
        "the measuring binary was built with. The taxonomy is compiled into the binary by "
        "`include_str!`, so those are separable and a run should stamp both — but these "
        "scores predate the manifest, and the taxonomy of the binary that produced them was "
        "not captured at the time. It is not reconstructed here, because a stamp inferred "
        "after the fact is a guess wearing a hash. Measurements recorded from now on carry "
        "the taxonomy version they were measured under."
    )
    w("")
    return "\n".join(out)


def cmd_render_release(args) -> int:
    doc = load(args.manifest)
    text = render_release(doc, args.binary)
    out = Path(args.out) if args.out else release_report_path(args.manifest.parent, args.binary)
    if args.check:
        if not out.exists():
            die(EXIT_INCONSISTENT, f"FAIL: {out} does not exist; run render-release to create it.")
        if out.read_text(encoding="utf-8") != text:
            die(
                EXIT_INCONSISTENT,
                f"FAIL: {out} has drifted from evidence/fixtures.json.",
                "      The report is generated, not written: a number in it that the manifest",
                "      does not carry is the transcription error this directory exists to stop.",
                f"      Re-render it:  scripts/evidence.py render-release --binary {args.binary}",
            )
        print(f"ok: {out} matches the manifest")
        return 0
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    print(f"wrote {out} ({len(text)} bytes)")
    return 0


def check_evidence_dir(evidence_dir: Path) -> list[str]:
    """evidence/ holds text and small tabular files only. Enforced, not trusted."""
    problems: list[str] = []
    if not evidence_dir.is_dir():
        return [f"{evidence_dir}: no evidence directory"]
    total = 0
    for path in sorted(p for p in evidence_dir.rglob("*") if p.is_file()):
        rel = path.relative_to(evidence_dir.parent)
        size = path.stat().st_size
        total += size
        if path.suffix not in EVIDENCE_TEXT_SUFFIXES:
            problems.append(
                f"{rel}: suffix {path.suffix or '(none)'!r} is not one of "
                f"{', '.join(EVIDENCE_TEXT_SUFFIXES)} — evidence/ holds headline reports and the "
                f"manifest; regenerable artefacts belong in the ignored output/"
            )
            continue
        try:
            path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            problems.append(
                f"{rel}: is not UTF-8 text despite its {path.suffix} suffix — a binary artefact "
                f"renamed is still a binary artefact"
            )
            continue
        if size > EVIDENCE_MAX_FILE_BYTES:
            problems.append(
                f"{rel}: {size} bytes exceeds the {EVIDENCE_MAX_FILE_BYTES}-byte per-file limit — "
                f"keep the headline here and move the table it summarises to output/"
            )
    if total > EVIDENCE_MAX_TOTAL_BYTES:
        problems.append(
            f"{evidence_dir}: {total} bytes total exceeds the {EVIDENCE_MAX_TOTAL_BYTES}-byte "
            f"budget — if evidence/ is accumulating megabytes the split is in the wrong place, "
            f"and the fix is to move artefacts back to output/, not to raise the limit"
        )
    return problems


def cmd_verify(args) -> int:
    doc = load(args.manifest)
    problems: list[str] = []
    seen_sha: dict[str, str] = {}
    labels_checked: set[str] = set()
    for fid, fx in doc["fixtures"].items():
        for field in ("version", "path", "sha256", "rows", "registered", "taxonomy"):
            if field not in fx:
                problems.append(f"{fid}: missing {field!r}")
        if "version" in fx and fx["version"] != fid:
            problems.append(
                f"{fid}: records version {fx['version']!r} — a fixture's id is its version"
            )
        tax = fx.get("taxonomy")
        if isinstance(tax, dict):
            tsha = tax.get("sha256", "")
            if len(tsha) != 64 or any(c not in "0123456789abcdef" for c in tsha):
                problems.append(
                    f"{fid}: taxonomy sha256 {tsha!r} is not a 64-char lowercase hex digest"
                )
            elif tax.get("version") != "tax-" + tsha[:12]:
                problems.append(
                    f"{fid}: taxonomy version {tax.get('version')!r} is not derived from its own "
                    f"digest (expected {'tax-' + tsha[:12]!r}) — a hand-written version can name "
                    f"a vocabulary that was never measured"
                )
            if not isinstance(tax.get("types"), int) or tax.get("types", 0) <= 0:
                problems.append(f"{fid}: taxonomy 'types' must be a positive integer")
        elif tax is not None:
            problems.append(f"{fid}: 'taxonomy' must be an object")
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
            fx = doc["fixtures"][match[0]]
            rows = data_rows_of(path)
            if fx.get("rows") != rows:
                problems.append(
                    f"{match[0]}: recorded rows={fx.get('rows')} but "
                    f"{path} has {rows} data rows"
                )
            found = check_labels_resolve(match[0], fx, path, Path(args.taxonomy_root))
            problems += found
            if not found and fx.get("label_column"):
                labels_checked.add(match[0])

    # The evidence directory is a property of the repo, not of whichever manifest you
    # happened to point at: a scratch manifest in a temp dir is a fixture registry, not
    # an evidence directory, and scanning its parent flags every scratch file in it.
    evidence_dir = args.evidence_dir or DEFAULT_EVIDENCE_DIR
    problems += check_evidence_dir(evidence_dir)

    # A release named in `reports` must have a current, generated report on disk. This is
    # what makes "the baseline is committed" a checked fact rather than a claim: delete
    # evidence/release-<v>.md, or edit a number in it, and this reddens.
    for binary in doc.get("reports", []):
        out = release_report_path(evidence_dir, binary)
        if not out.is_file():
            problems.append(
                f"{out}: the manifest lists {binary!r} under 'reports' but the report is not "
                f"on disk — render it with: scripts/evidence.py render-release --binary {binary}"
            )
        elif out.read_text(encoding="utf-8") != render_release(doc, binary):
            problems.append(
                f"{out}: has drifted from the manifest. The report is generated, not written — "
                f"re-render it: scripts/evidence.py render-release --binary {binary}"
            )

    if problems:
        die(
            EXIT_INCONSISTENT,
            f"FAIL: {args.manifest} is inconsistent:",
            *(f"  - {p}" for p in problems),
        )
    n_base = sum(len(fx.get("baselines", {})) for fx in doc["fixtures"].values())
    n_reports = len(doc.get("reports", []))
    tax = taxonomy_digest(Path(args.taxonomy_root))
    print(
        f"ok: {len(doc['fixtures'])} fixture versions, {n_base} recorded baselines, "
        f"{n_reports} release report{'' if n_reports == 1 else 's'} current"
    )
    print(f"    checkout taxonomy: {tax['version']} ({tax['types']} types)")
    for fid in sorted(doc["fixtures"]):
        recorded = doc["fixtures"][fid].get("taxonomy", {}).get("version")
        if not recorded or recorded == tax["version"]:
            continue
        if fid in labels_checked:
            tail = (
                "every label it uses is still a type the checkout defines, so its scores stay "
                "attributable"
            )
        else:
            tail = (
                "its labels were not checked against the checkout — the file on disk is a "
                "different version, so this fixture's blob is not here to read"
            )
        print(
            f"    note: {fid} was adjudicated under {recorded}, the checkout is at "
            f"{tax['version']}; {tail}."
        )
    return 0


def check_labels_resolve(fid: str, fx: dict, path: Path, tax_root: Path) -> list[str]:
    """Every label the fixture asserts must still be a type the taxonomy defines.

    This is the check that makes the recorded taxonomy version load-bearing rather than
    decorative. Taxonomy drift on its own is fine and reported as a note — a fixture is
    not invalidated because a type was added elsewhere. What is not fine is a fixture
    whose ground truth names a label the taxonomy no longer has: a score against it is
    attributed to a type that does not exist.
    """
    column = fx.get("label_column")
    if not column:
        return []
    used = fixture_labels(path, column)
    if not used:
        return [
            f"{fid}: label_column {column!r} is recorded but {path} has no such column, or it "
            f"is empty — the recorded taxonomy version is then unchecked"
        ]
    if fx.get("labels_used") is not None and fx["labels_used"] != len(used):
        return [
            f"{fid}: records labels_used={fx['labels_used']} but {path} uses {len(used)} distinct "
            f"labels in {column!r}"
        ]
    known = set(taxonomy_digest(tax_root)["labels"])
    missing = [lbl for lbl in used if lbl not in known]
    if missing:
        return [
            f"{fid}: {len(missing)} label(s) in {path} are not types in the checked-out taxonomy: "
            f"{', '.join(missing[:5])}{'…' if len(missing) > 5 else ''} — a score against a label "
            f"the taxonomy does not define cannot be attributed to anything"
        ]
    return []


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
    p.add_argument("--taxonomy-root", default=str(REPO_ROOT))
    p.add_argument("--taxonomy-commit", default="",
                   help="commit the taxonomy root was extracted from, for a historical stamp")
    p.add_argument("--label-column", help="the fixture's ground-truth label column")
    p.add_argument("--force", action="store_true")
    p.set_defaults(fn=cmd_register_fixture)

    p = sub.add_parser("taxonomy-version", help="the label vocabulary's content version")
    p.add_argument("--root", default=str(REPO_ROOT))
    p.add_argument("--format", choices=("id", "tsv", "json"), default="id")
    p.set_defaults(fn=cmd_taxonomy_version)

    p = sub.add_parser("render-release", help="render the readable report from the manifest")
    p.add_argument("--binary", required=True)
    p.add_argument("--out")
    p.add_argument("--check", action="store_true", help="fail if the report has drifted")
    p.set_defaults(fn=cmd_render_release)

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
    p.add_argument("--taxonomy-root", default=str(REPO_ROOT))
    p.add_argument("--evidence-dir", type=Path,
                   help=f"directory held to the size/text budget (default {DEFAULT_EVIDENCE_DIR})")
    p.set_defaults(fn=cmd_verify)

    args = ap.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(main())
