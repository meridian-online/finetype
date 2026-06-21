#!/usr/bin/env python3
"""Build a GOLD-column FTMB with the potion embed slot, for offline scoring (ac-02).

Potion sibling of build_gold_ftmb.py. One record per gold column as a singleton group;
char/stats/header/validation come from `finetype extract-features` (Rust, unaffected), and
the embed slot from build_ftmb_v5_potion's Python potion aggregation (full width — NOT the
truncated Rust path). Scored offline by `predict_multibranch --model <m> --data <this>`.

Usage:
  eval/gittables/.venv/bin/python scripts/build_gold_ftmb_potion.py \
      --gold eval/gold/gold_corpus.tsv --columns eval/gittables/corpus_pass/columns.parquet \
      --binary ./target/release/finetype --potion minishlab/potion-base-8M \
      --out output/embed-frontier/gold_m2v8m.ftmb
  # two-view:  --potion minishlab/potion-base-4M --two-view minishlab/potion-base-32M
"""
import argparse
import sys
from pathlib import Path

sys.path.insert(0, "scripts")
import build_ftmb_v5_potion as B       # noqa: E402
import prepare_multibranch_data as P   # noqa: E402
from score_gold_anchor import SEP, _vendored_values, load_gold  # noqa: E402

JOIN = "\x1f"  # unit separator inside the FTMB label: <sha>\x1f<column_name>


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--gold", required=True)
    ap.add_argument("--columns", required=True, help="corpus parquet with sample_values_truncated")
    ap.add_argument("--binary", default="./target/release/finetype")
    ap.add_argument("--potion", default="minishlab/potion-base-8M")
    ap.add_argument("--two-view", default=None)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()

    B._POTIONS = [a.potion] + ([a.two_view] if a.two_view else [])
    B.install_patches()  # patches P.extract_features (potion embed), EMBED_DIM, VALID_DIM, v5
    print(f"[gold-ftmb] EMBED_DIM={P.EMBED_DIM} VALID_DIM={P.VALID_DIM} VERSION={P.VERSION_V4} "
          f"potion={' ++ '.join(B._POTIONS)}")

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
