#!/usr/bin/env python3
"""Round-trip A/B over a corpus sample for one model.

For each table: profile -> self-generated JSON Schema -> validate, then read
per-column rejects from the finetype_reject_errors sidecar. Emits two views of
the same per-column reject data so the fast single-dataset harness can be
reconciled against the gittables corpus criteria:

  non_trivial_pct          recall  (corpus: non_trivial_floor)
  reject_rate_cellwt       precision, cell-weighted  (my roundtrip_metrics.sh)
  reject_rate_ceil_failpct precision, column-weighted (corpus: reject_rate_ceil)
                           = fraction of non-trivial columns whose own reject
                             rate exceeds CEIL (default 0.01)

Model is chosen via FINETYPE_MODEL (path to model dir). Binary via FINETYPE_BIN.

Usage: roundtrip_ab.py --files <list> --label <name> [--limit N] [--ceil 0.01]
"""
import argparse, json, os, subprocess, sys, tempfile, shutil

BIN = os.environ.get("FINETYPE_BIN", "./target/release/finetype")
CATCH_ALL = {"representation.text.plain_text", "unknown", ""}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--files", required=True)
    ap.add_argument("--label", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--ceil", type=float, default=0.01)
    args = ap.parse_args()

    files = [l.strip() for l in open(args.files) if l.strip()]
    if args.limit:
        files = files[: args.limit]
    model = os.environ.get("FINETYPE_MODEL", "models/default")

    work = tempfile.mkdtemp(prefix="rtab_")
    csvdir = os.path.join(work, "csv")
    schemadir = os.path.join(work, "schemas")
    os.makedirs(csvdir)
    os.makedirs(schemadir)

    # profile is CSV-only, so convert each parquet -> CSV (row-capped for speed;
    # reject RATE is unbiased under row sampling). Unique index stems map output
    # back 1:1 to inputs.
    ROW_CAP = 1000
    idx_paths = []
    mapping = []  # (idx, real_path, csv_path)
    convert_err = 0
    for i, f in enumerate(files):
        if not os.path.isfile(f):
            continue
        csvp = os.path.join(csvdir, f"{i:05d}.csv")
        c = run(["duckdb", "-c",
                 f"COPY (SELECT * FROM read_parquet('{os.path.realpath(f)}') "
                 f"LIMIT {ROW_CAP}) TO '{csvp}' (HEADER, FORMAT CSV);"])
        if c.returncode != 0 or not os.path.isfile(csvp):
            convert_err += 1
            continue
        idx_paths.append(csvp)
        mapping.append((i, f, csvp))
    listfile = os.path.join(work, "list.txt")
    open(listfile, "w").write("\n".join(idx_paths) + "\n")

    # One model load for the whole batch.
    r = run([BIN, "profile", "--files", listfile, "--out-dir", schemadir,
             "-o", "json-schema"])
    if r.returncode != 0:
        print(f"batch profile failed: {r.stderr[-500:]}", file=sys.stderr)

    # Per-column accumulation.
    nt_cols = 0
    total_cols = 0
    nt_cells = 0          # Σ rows over non-trivial cols
    nt_rejects = 0        # Σ rejects over non-trivial cols
    nt_ceil_fail = 0      # # non-trivial cols with own reject rate > ceil
    files_ok = 0
    files_err = 0

    for i, _real, csvp in mapping:
        schema = os.path.join(schemadir, f"{i:05d}.json")
        if not os.path.isfile(schema):
            files_err += 1
            continue
        labels = {}
        try:
            sj = json.load(open(schema))
            for col, spec in sj.get("properties", {}).items():
                labels[col] = spec.get("x-finetype-label", "")
        except Exception:
            files_err += 1
            continue

        db = os.path.join(work, "o.db")
        if os.path.exists(db):
            os.remove(db)
        v = run([BIN, "validate", csvp, schema, "--db", db, "--table", "t",
                 "-o", "json"])
        # exit 1 == rejects exist (expected); only treat parse/other as error
        try:
            summary = json.loads(v.stdout)
            rows = summary["total_rows"]
        except Exception:
            files_err += 1
            continue
        files_ok += 1

        # per-column rejects from sidecar
        per_col = {}
        if os.path.exists(db):
            q = run(["duckdb", db, "-noheader", "-csv", "-c",
                     "SELECT column_name, COUNT(*) FROM finetype_reject_errors "
                     "GROUP BY 1;"])
            for line in q.stdout.splitlines():
                if "," in line:
                    name, n = line.rsplit(",", 1)
                    name = name.strip().strip('"')
                    try:
                        per_col[name] = int(n)
                    except ValueError:
                        pass

        for col, label in labels.items():
            total_cols += 1
            if label in CATCH_ALL:
                continue
            nt_cols += 1
            rej = min(per_col.get(col, 0), rows)
            nt_cells += rows
            nt_rejects += rej
            if rows > 0 and (rej / rows) > args.ceil:
                nt_ceil_fail += 1

    shutil.rmtree(work, ignore_errors=True)

    non_trivial_pct = nt_cols / total_cols if total_cols else 0.0
    reject_rate_cellwt = nt_rejects / nt_cells if nt_cells else 0.0
    reject_rate_ceil_failpct = nt_ceil_fail / nt_cols if nt_cols else 0.0

    out = {
        "label": args.label,
        "model": model,
        "files_ok": files_ok,
        "files_err": files_err,
        "convert_err": convert_err,
        "total_cols": total_cols,
        "non_trivial_cols": nt_cols,
        "non_trivial_pct": round(non_trivial_pct, 4),
        "reject_rate_cellwt": round(reject_rate_cellwt, 4),
        "reject_rate_ceil_failpct": round(reject_rate_ceil_failpct, 4),
        "ceil": args.ceil,
    }
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
