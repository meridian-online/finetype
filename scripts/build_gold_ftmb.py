#!/usr/bin/env python3
"""Build an FTMB v5 for the gold (or representative) eval columns so a gte model can
be scored offline (ac-03, spec 2026-06-20-gte-tiny-embed-branch-swap).

One record per gold column as a SINGLETON table group (singletons skip sibling-context
in predict_multibranch, matching the single-column `profile` path v19 was scored on).
Each record's label is the join key  "<sha>\\x1f<column_name>"  so predictions map back
to (file_content_sha256, column_name) for score_gold_anchor.

Features are computed exactly as the training binary: char/stats/header(Model2Vec
128)/validation via `finetype extract-features`, and the embed slot via the SAME gte
4-stat L2-normed aggregation as build_ftmb_v5_gte (frozen base, or --encoder-checkpoint
for the fine-tuned encoder). Values come from the corpus parquet (the small per-column
sample the labeller saw), vendored-CSV fallback — identical to score_gold_anchor predict.

Usage:
  python3 scripts/build_gold_ftmb.py --gold eval/gold/gold_corpus.tsv \\
      --columns eval/gittables/corpus_pass/columns.parquet \\
      --binary ./target/release/finetype --out output/gte-tiny-embed-swap/gold_floor.ftmb \\
      [--encoder-checkpoint output/gte-tiny-embed-swap/gte_small_ft.pt]
"""
import argparse
import os
import sys

_SCRIPTS = os.path.dirname(os.path.abspath(__file__))
if _SCRIPTS not in sys.path:
    sys.path.insert(0, _SCRIPTS)

import build_ftmb_v5_gte as B       # noqa: E402
import prepare_multibranch_data as P  # noqa: E402
from score_gold_anchor import SEP, _vendored_values, load_gold  # noqa: E402

JOIN = "\x1f"  # unit separator inside the FTMB label: <sha>\x1f<column_name>


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gold", required=True)
    ap.add_argument("--columns", required=True, help="corpus parquet with sample_values_truncated")
    ap.add_argument("--binary", default="./target/release/finetype")
    ap.add_argument("--encoder-checkpoint", default=None)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    B._ENCODER_CKPT = a.encoder_checkpoint
    B.install_patches()  # patches P.extract_features (gte 1536), EMBED_DIM, VALID_DIM, write_ftmb_v4->v5
    print(f"[gold-ftmb] EMBED_DIM={P.EMBED_DIM} VALID_DIM={P.VALID_DIM} VERSION={P.VERSION_V4} "
          f"encoder={'ft:'+a.encoder_checkpoint if a.encoder_checkpoint else 'frozen-base'}")

    from pathlib import Path
    gold = load_gold(Path(a.gold))
    wanted = {(r["file_content_sha256"], r["column_name"]) for r in gold}
    print(f"[gold-ftmb] {len(gold)} gold columns")

    import pyarrow.parquet as pq
    tbl = pq.read_table(
        a.columns, columns=["file_content_sha256", "column_name", "sample_values_truncated"])
    samples = {}
    for r in tbl.to_pylist():
        key = (r.get("file_content_sha256") or "", r.get("column_name") or "")
        if key in wanted and key not in samples:
            raw = r.get("sample_values_truncated") or ""
            samples[key] = [v for v in raw.split(SEP) if v != ""]

    groups = []
    n_novals = n_featfail = 0
    for r in gold:
        key = (r["file_content_sha256"], r["column_name"])
        vals = samples.get(key) or _vendored_values(r.get("file_path", ""), r["column_name"])
        if not vals:
            n_novals += 1
            continue
        feats = P.extract_features(a.binary, vals, header=r["column_name"], include_validation=True)
        if feats is None:
            n_featfail += 1
            continue
        dims = [("char", feats.get("char"), P.CHAR_DIM), ("embed", feats.get("embed"), P.EMBED_DIM),
                ("stats", feats.get("stats"), P.STATS_DIM),
                ("header", feats.get("header_features"), P.HEADER_DIM),
                ("validation", feats.get("validation"), P.VALID_DIM)]
        if any(f is None or len(f) != d for _, f, d in dims):
            n_featfail += 1
            continue
        rec = {
            "label": f"{r['file_content_sha256']}{JOIN}{r['column_name']}",
            "column_index": 0,
            "char": feats["char"], "embed": feats["embed"], "stats": feats["stats"],
            "header": feats["header_features"], "validation": feats["validation"],
        }
        groups.append({"sibling_headers": [r["column_name"]], "records": [rec]})

    P.write_ftmb_v4(a.out, groups)  # -> v5 (patched)
    print(f"[gold-ftmb] wrote {len(groups)} records -> {a.out} "
          f"({n_novals} no-values, {n_featfail} feature-fail)")


if __name__ == "__main__":
    main()
