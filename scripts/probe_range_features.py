#!/usr/bin/env python3
"""ac-01 probe: do numeric-range features separate latitude/longitude/year/postcode/
decimal on REAL gold columns where the gte view collapses them?

Compares a standardized logistic-regression classifier (5-fold CV) on:
  - numeric_range_features (32-dim, static)   vs
  - frozen gte-tiny embed (1536-dim, the floor's encoder).

The decisive task is COORDINATE-vs-DECIMAL: the two-view regression was latitude ->
decimal_number (ac-00), and lat-vs-lon is the Sharpen rule's job, so the model only needs
to call a coordinate column "a coordinate type", not "decimal".
"""
import os
import sys
from pathlib import Path

_S = os.path.dirname(os.path.abspath(__file__))
if _S not in sys.path:
    sys.path.insert(0, _S)

import numpy as np  # noqa: E402
import build_ftmb_v5_gte as B  # noqa: E402
from numeric_range_features import numeric_range_features  # noqa: E402
from score_gold_anchor import SEP, _vendored_values, load_gold  # noqa: E402
from sklearn.linear_model import LogisticRegression  # noqa: E402
from sklearn.model_selection import cross_val_score  # noqa: E402
from sklearn.pipeline import make_pipeline  # noqa: E402
from sklearn.preprocessing import StandardScaler  # noqa: E402

TYPES = {
    "geography.coordinate.latitude": "lat",
    "geography.coordinate.longitude": "lon",
    "datetime.component.year": "year",
    "geography.address.postal_code": "postcode",
    "representation.numeric.decimal_number": "decimal",
}
GOLD = "eval/gold/gold_corpus.tsv"
COLS = "eval/gittables/corpus_pass/columns.parquet"


def main():
    import pyarrow.parquet as pq

    gold = load_gold(Path(GOLD))
    wanted = {(r["file_content_sha256"], r["column_name"]): r for r in gold
              if r["curated_label"] in TYPES}
    tbl = pq.read_table(COLS, columns=["file_content_sha256", "column_name",
                                       "sample_values_truncated"])
    samples = {}
    for r in tbl.to_pylist():
        k = (r.get("file_content_sha256") or "", r.get("column_name") or "")
        if k in wanted and k not in samples:
            samples[k] = [v for v in (r.get("sample_values_truncated") or "").split(SEP) if v]

    B.install_patches()  # frozen gte-tiny (no ckpt) -> 1536 embed via B.gte_embed
    Xr, Xg, y = [], [], []
    for k, r in wanted.items():
        vals = samples.get(k) or _vendored_values(r.get("file_path", ""), r["column_name"])
        if not vals:
            continue
        Xr.append(numeric_range_features(vals))
        Xg.append(B.gte_embed(vals))
        y.append(TYPES[r["curated_label"]])
    Xr, Xg, y = np.array(Xr), np.array(Xg), np.array(y)
    from collections import Counter
    print(f"columns: {len(y)}  per-class: {dict(Counter(y))}")

    def probe(X, name):
        clf = make_pipeline(StandardScaler(), LogisticRegression(max_iter=2000, C=1.0))
        acc = cross_val_score(clf, X, y, cv=5).mean()
        # coordinate-vs-decimal binary
        mask = np.isin(y, ["lat", "lon", "decimal"])
        yb = np.where(np.isin(y[mask], ["lat", "lon"]), "coord", "decimal")
        accb = cross_val_score(clf, X[mask], yb, cv=5).mean()
        print(f"  {name:24} 5-class acc={acc:.3f}   coord-vs-decimal acc={accb:.3f}")
        return acc, accb

    print("\n5-fold CV separability (higher = better):")
    rr = probe(Xr, "range-features (32)")
    gg = probe(Xg, "gte-tiny embed (1536)")
    print(f"\nrange vs gte: 5-class {rr[0]-gg[0]:+.3f} | coord-vs-decimal {rr[1]-gg[1]:+.3f}")


if __name__ == "__main__":
    main()
