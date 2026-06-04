#!/usr/bin/env python3
"""CharCNN-v13 collateral probe vs v19 baseline (and v24 for reference).

Runs the same three lenses used on v24:
  1. Sampled Sense-distribution snapshot (800 files) — categorical/geo/latitude/utc.
  2. Full-file collateral profiling (150 files) — count cols flipping into utc/latitude.
  3. Earthquake round-trip (non_trivial_pct / reject_rate).

Compares CharCNN against v19 (in-domain multi-branch baseline). NOTE the confound:
CharCNN here is trained on SYNTHETIC per-type data, v19 on the gittables blend — so a
clean CharCNN footprint is encouraging-but-confounded, not the cascade verdict.
"""
import subprocess, collections, os, tempfile, sys, csv, io
import pandas as pd

REPO = "/Users/hugh/github/meridian-online/finetype"
BIN = f"{REPO}/target/release/finetype"
CHARCNN = "models/char-cnn-v14-feat"
V19 = "models/sherlock-v19-relu-s42"
COLS = f"{REPO}/eval/gittables/corpus_pass/columns.parquet"
UTC, LAT = "datetime.offset.utc", "geography.coordinate.latitude"
CAT = "representation.discrete.categorical"

def profile_file(model, csvpath, model_type=None):
    cmd = [BIN, "profile", "-f", csvpath, "-o", "csv"]
    if model_type:
        cmd += ["--model-type", model_type]
    out = subprocess.run(cmd,
        env={"FINETYPE_MODEL": model, "PATH": "/usr/bin:/bin"},
        capture_output=True, text=True).stdout
    res = {}
    rdr = csv.reader(io.StringIO(out))
    rows = list(rdr)
    for row in rows[1:]:  # skip header
        if len(row) >= 2:
            res[row[0]] = row[1]
    return res

def lens2_collateral(n_files=150):
    allfp = pd.read_parquet(COLS, columns=["file_path"])["file_path"].drop_duplicates()
    files = allfp.sample(n_files, random_state=42).tolist()
    coll = []
    ncols = 0
    charcnn_hist = collections.Counter()
    for f in files:
        try:
            d = pd.read_parquet(f)
        except Exception:
            continue
        if d.shape[1] == 0 or len(d) == 0:
            continue
        with tempfile.NamedTemporaryFile("w", suffix=".csv", delete=False) as tf:
            d.to_csv(tf.name, index=False); csvp = tf.name
        a = profile_file(V19, csvp)
        b = profile_file(CHARCNN, csvp, model_type="char-cnn")
        os.unlink(csvp)
        for c in b:
            ncols += 1
            charcnn_hist[b[c]] += 1
            if b[c] in (UTC, LAT) and a.get(c) != b[c]:
                vals = [str(x) for x in d[c].dropna().tolist()[:5]]
                coll.append((b[c], a.get(c, "?"), c, vals))
    return ncols, coll, charcnn_hist

def main():
    if not os.path.isdir(f"{REPO}/{CHARCNN}"):
        print(f"FATAL: {CHARCNN} not found — did training succeed?"); sys.exit(1)
    print(f"=== CharCNN probe eval: {CHARCNN} vs {V19} ===\n")

    print("LENS 2 — full-file collateral (150 files), CharCNN vs v19:")
    ncols, coll, hist = lens2_collateral()
    nutc = sum(1 for b, *_ in coll if b == UTC)
    nlat = sum(1 for b, *_ in coll if b == LAT)
    print(f"  profiled {ncols} cols")
    print(f"  NEW utc collateral:      {nutc}")
    print(f"  NEW latitude collateral: {nlat}")
    print(f"  (v24 multi-branch on same lens was: 26 utc + 3 latitude = 29/1880)")
    print(f"\n  CharCNN absolute corpus footprint (key labels):")
    tot = sum(hist.values()) or 1
    for L in [UTC, LAT, CAT, "geography.location.city", "geography.location.region",
              "representation.numeric.integer_number", "representation.numeric.decimal_number"]:
        print(f"    {L:46} {hist.get(L,0):5}  ({100*hist.get(L,0)/tot:.2f}%)")
    print(f"\n  collateral source labels (what v19 called them):")
    for lab, n in collections.Counter(a for _, a, *_ in coll).most_common(8):
        print(f"    {n:3}  {lab}")
    print(f"\n  examples:")
    for b, a, c, vals in coll[:12]:
        sb = "utc" if b.endswith("utc") else "latitude"
        print(f"    ->{sb:8} (was {a.split('.')[-1][:18]:18}) {c[:18]:18} {vals[:4]}")

if __name__ == "__main__":
    main()
