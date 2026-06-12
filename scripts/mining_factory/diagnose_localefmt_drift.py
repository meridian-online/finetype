#!/usr/bin/env python3
"""Diagnose the locale-format proxy NO-GO: which real corpus columns does the proxy
now call container.object.json_array / identity.commerce.isbn, and what did v19 call
them?

Profiles the fixed 1,000-file drift list with BOTH the v19 baseline and the
locale-format proxy, then isolates every column the proxy labels json_array or isbn,
records the v19 label (drift source), the column header, and sample values.

Output: output/mining-factory/locale-format/drift_diagnosis.{json,md}
"""
from __future__ import annotations
import csv
import json
import os
import subprocess
import sys
import tempfile
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BIN = str(REPO / "target/release/finetype")
OUT_DIR = REPO / "output/mining-factory/locale-format"
FILE_LIST = REPO / "output/destination-drift-precheck/sense_dist_v19fx_s42.files.txt"
V19 = "models/sherlock-v19-relu-s42"
PROXY = "models/sherlock-mfg-localefmt-proxy-s42"
TARGETS = {"container.object.json_array", "identity.commerce.isbn"}
ROWS = 1000


def run(cmd, env=None):
    return subprocess.run(cmd, capture_output=True, text=True, env=env)


def profile(list_path: Path, schemadir: Path, model: str) -> None:
    schemadir.mkdir(parents=True, exist_ok=True)
    r = run([BIN, "profile", "--files", str(list_path), "--out-dir", str(schemadir),
             "-o", "json-schema"], env={**os.environ, "FINETYPE_MODEL": model})
    if r.returncode != 0:
        print(f"  profile({model}) non-zero exit; stderr tail:\n{r.stderr[-300:]}",
              file=sys.stderr)


def load_labels(schemadir: Path, idx: int) -> dict[str, str]:
    """idx -> {column_name: label} for one file's schema (named NNNNN.json)."""
    p = schemadir / f"{idx:05d}.json"
    if not p.is_file():
        return {}
    try:
        sj = json.load(open(p))
    except Exception:
        return {}
    return {name: spec.get("x-finetype-label", "")
            for name, spec in sj.get("properties", {}).items()}


def sample_values(csvp: Path, column: str, k: int = 6) -> list[str]:
    out: list[str] = []
    seen: set[str] = set()
    try:
        with open(csvp, newline="", encoding="utf-8", errors="replace") as f:
            r = csv.DictReader(f)
            for row in r:
                v = (row.get(column) or "").strip()
                if v and v not in seen:
                    seen.add(v)
                    out.append(v)
                    if len(out) >= k:
                        break
    except Exception:
        pass
    return out


def main() -> int:
    files = [l.strip() for l in FILE_LIST.read_text().splitlines() if l.strip()]
    print(f"profiling {len(files)} files with v19 + proxy", file=sys.stderr)

    with tempfile.TemporaryDirectory(prefix="lfdrift_") as work:
        work = Path(work)
        csvdir = work / "csv"
        csvdir.mkdir()
        idx_to_csv: dict[int, Path] = {}
        idx_paths: list[str] = []
        conv_err = 0
        for i, f in enumerate(files):
            if not os.path.isfile(f):
                conv_err += 1
                continue
            csvp = csvdir / f"{i:05d}.csv"
            c = run(["duckdb", "-c",
                     f"COPY (SELECT * FROM read_parquet('{os.path.realpath(f)}') "
                     f"LIMIT {ROWS}) TO '{csvp.as_posix()}' (HEADER, FORMAT CSV);"])
            if c.returncode != 0 or not csvp.is_file():
                conv_err += 1
                continue
            idx_to_csv[i] = csvp
            idx_paths.append(str(csvp))
        print(f"converted {len(idx_paths)} files ({conv_err} errors)", file=sys.stderr)

        listfile = work / "list.txt"
        listfile.write_text("\n".join(idx_paths) + "\n")

        # NOTE: profile names each output schema after the input CSV stem (NNNNN.json),
        # so the index maps straight back to the source file.
        v19_dir = work / "schema_v19"
        proxy_dir = work / "schema_proxy"
        print("profiling v19...", file=sys.stderr)
        profile(listfile, v19_dir, V19)
        print("profiling proxy...", file=sys.stderr)
        profile(listfile, proxy_dir, PROXY)

        hits: list[dict] = []
        for i, csvp in idx_to_csv.items():
            proxy_labels = load_labels(proxy_dir, i)
            if not proxy_labels:
                continue
            v19_labels = load_labels(v19_dir, i)
            for col, plabel in proxy_labels.items():
                if plabel in TARGETS:
                    hits.append({
                        "source_file": files[i],
                        "column": col,
                        "proxy_label": plabel,
                        "v19_label": v19_labels.get(col, "<absent>"),
                        "sample_values": sample_values(csvp, col),
                    })

    # Summaries.
    by_target = Counter(h["proxy_label"] for h in hits)
    drift_from = defaultdict(Counter)
    header_tokens = defaultdict(Counter)
    for h in hits:
        drift_from[h["proxy_label"]][h["v19_label"]] += 1
        header_tokens[h["proxy_label"]][h["column"].lower()] += 1

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    (OUT_DIR / "drift_diagnosis.json").write_text(
        json.dumps({"total_hits": len(hits), "by_target": dict(by_target),
                    "hits": hits}, indent=2) + "\n")

    lines = ["# locale-format proxy drift — per-column diagnosis", "",
             f"Profiled the fixed {len(files)}-file drift list with v19 and the "
             "locale-format proxy. Columns the proxy now labels json_array / isbn:",
             ""]
    for tgt in sorted(by_target):
        lines += [f"## `{tgt}` — {by_target[tgt]} columns", "",
                  "**Drifted FROM (what v19 called these columns):**", ""]
        for src, n in drift_from[tgt].most_common():
            lines.append(f"- `{src}` -> `{tgt}`: {n}")
        lines += ["", "**Most common column headers:**", ""]
        for hdr, n in header_tokens[tgt].most_common(15):
            lines.append(f"- `{hdr}`: {n}")
        lines += ["", "**Sample columns (header -> values):**", ""]
        shown = 0
        for h in hits:
            if h["proxy_label"] != tgt:
                continue
            vals = ", ".join(h["sample_values"][:5])
            lines.append(f"- `{h['column']}` (v19=`{h['v19_label']}`): {vals}")
            shown += 1
            if shown >= 20:
                break
        lines.append("")
    (OUT_DIR / "drift_diagnosis.md").write_text("\n".join(lines) + "\n")

    print(f"\n{len(hits)} hits: {dict(by_target)}")
    print(f"wrote {OUT_DIR / 'drift_diagnosis.md'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
