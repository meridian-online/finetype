#!/usr/bin/env python3
"""Re-baseline gold by feeding the model a PROPER sample read from each column's
source file, instead of the ~8-value `sample_values_truncated` stored in
columns.parquet (which understates production accuracy — production reads the
full column and samples up to 100).

For each gold column, read up to --cap non-null values from the source file via
the duckdb CLI (csv or parquet), reconstruct a single-column CSV, and profile it
with the real binary — the same path score_gold_anchor.cmd_predict uses, but with
production-scale values. Emits the same predictions TSV for `score_gold_anchor.py
score`.

Run: eval/gittables/.venv/bin/python scripts/predict_from_source.py \
       --gold eval/gold/gold_corpus.tsv --binary ./target/release/finetype \
       --out output/categorical-alnum-recall/predictions_fullsample.tsv [--cap 5000]
"""
import argparse
import csv
import os
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from score_gold_anchor import _profile_column  # noqa: E402
from sweep_decisive_stats import resolve, con  # noqa: E402


def source_values(path: str, col: str, cap: int) -> list[str]:
    """Up to `cap` non-null values for `col`, read from the source file."""
    p = resolve(path)
    if not p:
        return []
    src = (f"read_parquet('{p}')" if p.endswith(".parquet")
           else f"read_csv_auto('{p}', SAMPLE_SIZE=-1, ignore_errors=true)")
    q = col.replace('"', '""')
    for name in (q, None):
        try:
            target = f'"{q}"' if name is not None else None
            if target is None:
                # resolve exact column name case-insensitively
                schema = [c[0] for c in con.execute(f"DESCRIBE SELECT * FROM {src}").fetchall()]
                m = {str(c).strip().lower(): c for c in schema}
                cn = m.get(col.strip().lower())
                if not cn:
                    return []
                target = '"' + cn.replace('"', '""') + '"'
            rows = con.execute(
                f"SELECT {target} FROM {src} WHERE {target} IS NOT NULL LIMIT {cap}"
            ).fetchall()
            return [str(r[0]) for r in rows]
        except Exception:
            continue
    return []


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gold", required=True, type=Path)
    ap.add_argument("--binary", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--cap", type=int, default=5000)
    args = ap.parse_args()

    gold = list(csv.DictReader(args.gold.open(), delimiter="\t"))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    n_ok = n_err = 0
    with args.out.open("w") as fh:
        w = csv.writer(fh, delimiter="\t", lineterminator="\n")
        w.writerow(["file_content_sha256", "column_name", "predicted_label", "confidence"])
        for i, r in enumerate(gold, 1):
            vals = source_values(r.get("file_path", ""), r["column_name"], args.cap)
            if not vals:
                n_err += 1
                continue
            try:
                label = _profile_column(args.binary, r["column_name"], vals)
            except Exception as e:  # noqa: BLE001
                print(f"  row {i}: {e}", file=sys.stderr)
                n_err += 1
                continue
            w.writerow([r["file_content_sha256"], r["column_name"], label, ""])
            n_ok += 1
            if i % 100 == 0:
                print(f"  ... {i}/{len(gold)}", file=sys.stderr)
    print(f"predicted {n_ok} ({n_err} missing/err) -> {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
