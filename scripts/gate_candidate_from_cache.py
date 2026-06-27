#!/usr/bin/env python3
"""Build corpus-honest-gate CANDIDATE predictions on the stratified sample by
reconstructing tables from the cached corpus columns.parquet values and profiling
them sibling-aware with the current binary — no source-file re-read.

Output: a candidate parquet (file_path, column_name, sense_prediction=composed) ready
for `scripts/corpus_honest_gate.py --candidate`. Robust to batch-halt: tables are
profiled in small chunks via `profile --files -o json-schema`, max N concurrent; a
chunk that errors is logged and skipped (its columns are simply absent from the
candidate, which the gate's inner join tolerates).

Spec 2026-06-27-composed-accuracy-roadmap. Usage:
  gate_candidate_from_cache.py --workdir <dir> --out <candidate.parquet> [--limit-files K]
"""
import argparse
import collections
import csv
import json
import os
import subprocess
import sys
import time

import pyarrow as pa
import pyarrow.parquet as pq

SEP = "│"


def clean(s):
    return (s or "").replace("\t", " ").replace("\r", " ").replace("\n", " ")[:300]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--columns", default="eval/gittables/corpus_pass/columns.parquet")
    ap.add_argument("--sample", default="output/corpus-honest-gate/stratified_sample.files.txt")
    ap.add_argument("--bin", default="./target/release/finetype")
    ap.add_argument("--workdir", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--chunk", type=int, default=200)
    ap.add_argument("--limit-files", type=int, default=0)
    ap.add_argument("--raw-sense-tsv", default="",
                    help="Emit a raw-Sense resharpen-input TSV (adds --raw-model to profile) "
                         "instead of the composed candidate parquet. Reusable cache: resharpen "
                         "it with any binary to get composed in seconds.")
    a = ap.parse_args()
    tdir = os.path.join(a.workdir, "tables")
    jdir = os.path.join(a.workdir, "json")
    os.makedirs(tdir, exist_ok=True)
    os.makedirs(jdir, exist_ok=True)

    sample = set(l.strip() for l in open(a.sample) if l.strip())
    t = pq.read_table(a.columns, columns=["file_path", "column_name", "sample_values_truncated"])
    d = t.to_pydict()
    by_file = collections.defaultdict(list)
    for i in range(len(d["column_name"])):
        fp = d["file_path"][i]
        if fp in sample:
            by_file[fp].append((d["column_name"][i], d["sample_values_truncated"][i] or ""))
    files = sorted(by_file)
    if a.limit_files:
        files = files[: a.limit_files]
    print(f"{len(files)} sample tables (of {len(sample)} sample files)", file=sys.stderr)

    # Reconstruct one CSV per table; record stem -> (file_path, {header_in_csv: orig_col})
    stems = {}
    flist = []
    vals_by_id = {}  # (file_path, orig_col) -> raw SEP-joined values (for the resharpen TSV)
    for idx, fp in enumerate(files):
        cols = by_file[fp]
        valcols = [[clean(v) for v in c[1].split(SEP) if v != ""] for c in cols]
        maxlen = max((len(v) for v in valcols), default=0)
        if maxlen == 0:
            continue
        # Unique CSV headers (dedup collisions) mapped back to original column names.
        seen, headers, namemap = {}, [], {}
        for j, c in enumerate(cols):
            base = clean(c[0]) or f"col{j}"
            if base in seen:
                seen[base] += 1
                h = f"{base}__dup{seen[base]}"
            else:
                seen[base] = 0
                h = base
            headers.append(h)
            namemap[h] = c[0]
            vals_by_id[(fp, c[0])] = c[1]
        stem = f"t{idx}"
        path = os.path.join(tdir, stem + ".csv")
        with open(path, "w", newline="") as fh:
            w = csv.writer(fh, quoting=csv.QUOTE_ALL)
            w.writerow(headers)
            for r in range(maxlen):
                w.writerow([(valcols[c][r] if r < len(valcols[c]) else "") for c in range(len(cols))])
        stems[stem] = (fp, namemap)
        flist.append(path)

    # Chunk the table list; run `profile --files` (one model load/chunk), max `jobs` concurrent.
    chunks = [flist[i : i + a.chunk] for i in range(0, len(flist), a.chunk)]
    print(f"{len(chunks)} chunks of <= {a.chunk}, {a.jobs} concurrent", file=sys.stderr)
    t0 = time.time()
    running = {}  # proc -> (k, outdir)
    failed = []
    done = 0

    def launch(k):
        ch = chunks[k]
        lp = os.path.join(a.workdir, f"chunk_{k}.txt")
        with open(lp, "w") as fh:
            fh.write("\n".join(ch) + "\n")
        outk = os.path.join(jdir, f"out_{k}")
        os.makedirs(outk, exist_ok=True)
        cmd = [a.bin, "profile"]
        if a.raw_sense_tsv:
            cmd.append("--raw-model")
        cmd += ["--files", lp, "--out-dir", outk, "-o", "json-schema"]
        p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        running[p] = (k, outk)

    nxt = 0
    while nxt < len(chunks) and len(running) < a.jobs:
        launch(nxt)
        nxt += 1
    while running:
        for p in list(running):
            rc = p.poll()
            if rc is None:
                continue
            k, _ = running.pop(p)
            done += 1
            if rc != 0:
                failed.append(k)
            if done % 20 == 0:
                el = time.time() - t0
                print(f"  {done}/{len(chunks)} chunks ({el:.0f}s, {len(failed)} failed)", file=sys.stderr)
            if nxt < len(chunks):
                launch(nxt)
                nxt += 1
        time.sleep(0.3)
    print(f"profiling done in {time.time()-t0:.0f}s; {len(failed)} chunks failed", file=sys.stderr)

    # Collect composed labels (x-finetype-label) per (file_path, original column_name).
    fps, cns, preds = [], [], []
    for k in range(len(chunks)):
        outk = os.path.join(jdir, f"out_{k}")
        if not os.path.isdir(outk):
            continue
        for fn in os.listdir(outk):
            stem = fn.rsplit(".", 1)[0]
            if stem not in stems:
                continue
            fp, namemap = stems[stem]
            try:
                js = json.load(open(os.path.join(outk, fn)))
            except Exception:
                continue
            for prop, v in js.get("properties", {}).items():
                orig = namemap.get(prop)
                if orig is None:
                    continue
                fps.append(fp)
                cns.append(orig)
                preds.append(v.get("x-finetype-label", ""))
    if a.raw_sense_tsv:
        # Emit the resharpen-input TSV: id<TAB>header<TAB>raw_sense<TAB>conf<TAB>values(0x1f).
        US = "\x1f"
        n = 0
        with open(a.raw_sense_tsv, "w") as fh:
            for fp, cn, pred in zip(fps, cns, preds):
                raw = vals_by_id.get((fp, cn), "")
                vstr = US.join(clean(v) for v in raw.split(SEP) if v != "")
                fh.write(f"{fp}{US}{cn}\t{clean(cn)}\t{pred}\t1.0\t{vstr}\n")
                n += 1
        print(f"raw-sense cache: {n} columns -> {a.raw_sense_tsv}", file=sys.stderr)
    else:
        pq.write_table(pa.table({"file_path": fps, "column_name": cns, "sense_prediction": preds}), a.out)
        print(f"candidate: {len(fps)} columns -> {a.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
