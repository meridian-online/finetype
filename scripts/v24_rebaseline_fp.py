#!/usr/bin/env python3
"""v24 ac-00 — re-baseline the four numeric-target clusters' FP rate on the
SHIPPED default (v19), not on the v22 run the diagnostic recorded.

The diagnostic's `sense_prediction` in columns.parquet is a v22 prediction.
v22 is the campaign head, NOT what ships — models/default -> v19. So before
extracting v22 false positives as hard negatives we must confirm v19 still
makes the same mistake. This script:

  1. For each cluster (sense_fp_label, ydf_correct_label), samples N member
     columns from columns.parquet (members = v22 said sense_fp AND ydf said
     the numeric correct label).
  2. Pulls each real column from its source parquet -> single-column CSV.
  3. Profiles the batch ONCE under FINETYPE_MODEL (default the v19 dir) and
     reads the shipped pipeline label (x-finetype-label) per column.
  4. Reports, per cluster: how often v19 STILL fires the FP label (the v19 FP
     rate), and the histogram of what v19 assigns instead.

A cluster whose v19 FP rate is ~0 is already fixed on what ships and should be
dropped from the v24 retrain scope (ac-00).

Usage:
  FINETYPE_MODEL=models/sherlock-v19-relu-s42 \
    scripts/v24_rebaseline_fp.py [--n 300] [--rows 1000] \
    [--columns eval/gittables/corpus_pass/columns.parquet] [--seed 42]
"""
import argparse, json, os, subprocess, sys, tempfile, shutil, collections

BIN = os.environ.get("FINETYPE_BIN", "./target/release/finetype")
MODEL = os.environ.get("FINETYPE_MODEL", "models/sherlock-v19-relu-s42")

# (key, sense_fp_label, ydf_correct_label)
CLUSTERS = [
    ("utc->int",     "datetime.offset.utc",                   "representation.numeric.integer_number"),
    ("bool->int",    "representation.boolean.binary",         "representation.numeric.integer_number"),
    ("url->int",     "technology.internet.url",               "representation.numeric.integer_number"),
    ("int->dec",     "representation.numeric.integer_number", "representation.numeric.decimal_number"),
]


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def sample_members(columns_pq, sense_fp, ydf_correct, n, seed):
    """Return up to n (file_path, column_name) members, deterministic by seed."""
    # The WHERE must be applied BEFORE the sample — a bare `USING SAMPLE` on a
    # filtered read samples the whole parquet first, then filters to ~nothing.
    # Wrap the filter in a subquery so the reservoir sees only cluster members.
    q = run(["duckdb", "-noheader", "-csv", "-c",
             f"SELECT file_path, column_name FROM ("
             f"  SELECT file_path, column_name "
             f"  FROM read_parquet('{columns_pq}') "
             f"  WHERE sense_prediction='{sense_fp}' "
             f"    AND ydf_prediction='{ydf_correct}'"
             f") USING SAMPLE {n} ROWS (reservoir, {seed});"])
    out = []
    for line in q.stdout.splitlines():
        if "," not in line:
            continue
        fp, col = line.split(",", 1)
        out.append((fp.strip().strip('"'), col.strip().strip('"')))
    return out


def extract_column(file_path, column_name, csv_out, rows):
    """One source column -> single-column CSV. Doubles quotes to be safe."""
    safe_col = column_name.replace('"', '""')
    safe_path = file_path.replace("'", "''")
    c = run(["duckdb", "-c",
             f'COPY (SELECT "{safe_col}" FROM read_parquet(\'{safe_path}\') '
             f"LIMIT {rows}) TO '{csv_out}' (HEADER, FORMAT CSV);"])
    return c.returncode == 0 and os.path.isfile(csv_out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=300, help="members sampled per cluster")
    ap.add_argument("--rows", type=int, default=1000, help="row cap per column")
    ap.add_argument("--columns", default="eval/gittables/corpus_pass/columns.parquet")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    if not os.path.isdir(MODEL):
        print(f"error: model dir not found: {MODEL}", file=sys.stderr)
        sys.exit(3)

    work = tempfile.mkdtemp(prefix="v24rb_")
    csvdir = os.path.join(work, "csv")
    schemadir = os.path.join(work, "schema")
    os.makedirs(csvdir)
    os.makedirs(schemadir)

    # index -> (cluster_key, sense_fp). Stems keep profile output 1:1 with input.
    meta = {}
    idx_paths = []
    extract_err = collections.Counter()
    i = 0
    for key, sense_fp, ydf_correct in CLUSTERS:
        members = sample_members(args.columns, sense_fp, ydf_correct, args.n, args.seed)
        for fp, col in members:
            if not os.path.isfile(fp):
                extract_err[key] += 1
                continue
            csvp = os.path.join(csvdir, f"{i:06d}.csv")
            if not extract_column(fp, col, csvp, args.rows):
                extract_err[key] += 1
                i += 1
                continue
            meta[i] = (key, sense_fp)
            idx_paths.append(csvp)
            i += 1

    listfile = os.path.join(work, "list.txt")
    open(listfile, "w").write("\n".join(idx_paths) + "\n")

    # One model load for the whole batch -> shipped pipeline label per column.
    r = run([BIN, "profile", "--files", listfile, "--out-dir", schemadir,
             "-o", "json-schema"], env={**os.environ, "FINETYPE_MODEL": MODEL})
    if r.returncode != 0:
        print(f"batch profile non-zero exit; stderr tail:\n{r.stderr[-400:]}",
              file=sys.stderr)

    # Per-cluster tally of the shipped label.
    stats = {key: {"sampled": 0, "profiled": 0, "v19_fp": 0,
                   "reassigned": collections.Counter()}
             for key, _, _ in CLUSTERS}
    for idx, (key, sense_fp) in meta.items():
        stats[key]["sampled"] += 1
        schema = os.path.join(schemadir, f"{idx:06d}.json")
        if not os.path.isfile(schema):
            continue
        try:
            sj = json.load(open(schema))
            props = sj.get("properties", {})
            # single-column CSV -> exactly one property
            label = next(iter(props.values())).get("x-finetype-label", "")
        except Exception:
            continue
        stats[key]["profiled"] += 1
        if label == sense_fp:
            stats[key]["v19_fp"] += 1
        else:
            stats[key]["reassigned"][label] += 1

    shutil.rmtree(work, ignore_errors=True)

    print(f"# v24 ac-00 re-baseline — model={MODEL}")
    print(f"# n={args.n}/cluster rows={args.rows} seed={args.seed}\n")
    out_rows = []
    for key, sense_fp, ydf_correct in CLUSTERS:
        s = stats[key]
        prof = s["profiled"]
        rate = s["v19_fp"] / prof if prof else 0.0
        verdict = "DROP (already ~0 on v19)" if rate <= 0.01 else "KEEP (v19 still mistypes)"
        top = s["reassigned"].most_common(4)
        out_rows.append({
            "cluster": key, "sense_fp": sense_fp, "ydf_correct": ydf_correct,
            "sampled": s["sampled"], "profiled": prof,
            "v19_fp_count": s["v19_fp"], "v19_fp_rate": round(rate, 4),
            "verdict": verdict,
            "v19_reassigns_to": [{"label": l, "n": n} for l, n in top],
            "extract_err": extract_err[key],
        })
        print(f"## {key}: {sense_fp} (ydf says {ydf_correct})")
        print(f"   sampled={s['sampled']} profiled={prof} "
              f"v19_fp={s['v19_fp']} v19_fp_rate={rate:.4f}  -> {verdict}")
        if top:
            print("   v19 instead assigns: " +
                  ", ".join(f"{l}={n}" for l, n in top))
        print()

    print("=== JSON ===")
    print(json.dumps(out_rows, indent=2))


if __name__ == "__main__":
    main()
