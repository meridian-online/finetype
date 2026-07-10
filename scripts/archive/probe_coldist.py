#!/usr/bin/env python3
"""ac-02 probe: does adding column-distribution features to the EXISTING 27-dim stats
branch lift separability on the confusions the rules currently handle?

Compares (5-fold CV logistic regression, standardized):
  stats-27 (current)   vs   stats-27 ++ coldist (proposed)
on coord-vs-decimal (the decimal attractor) and increment-vs-integer (sequentiality).
The 27 stats come from `finetype extract-features --json`; coldist from Python.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

_S = os.path.dirname(os.path.abspath(__file__))
if _S not in sys.path:
    sys.path.insert(0, _S)

import numpy as np  # noqa: E402
from column_distribution_features import column_distribution_features  # noqa: E402
from score_gold_anchor import SEP, _vendored_values, load_gold  # noqa: E402
from sklearn.linear_model import LogisticRegression  # noqa: E402
from sklearn.model_selection import cross_val_score  # noqa: E402
from sklearn.pipeline import make_pipeline  # noqa: E402
from sklearn.preprocessing import StandardScaler  # noqa: E402

BIN = "./target/release/finetype"
TYPES = {
    "geography.coordinate.latitude": "coord",
    "geography.coordinate.longitude": "coord",
    "representation.numeric.decimal_number": "decimal",
    "datetime.component.year": "year",
    "representation.numeric.integer_number": "integer",
    "representation.identifier.increment": "increment",
}


def col_stats(values):
    out = subprocess.run([BIN, "extract-features", "--json"], input=json.dumps(values),
                         capture_output=True, text=True, timeout=60)
    return json.loads(out.stdout)["stats"]


def main():
    import pyarrow.parquet as pq
    gold = load_gold(Path("eval/gold/gold_corpus.tsv"))
    wanted = {(r["file_content_sha256"], r["column_name"]): r for r in gold
              if r["curated_label"] in TYPES}
    tbl = pq.read_table("eval/gittables/corpus_pass/columns.parquet",
                        columns=["file_content_sha256", "column_name", "sample_values_truncated"])
    samples = {}
    for r in tbl.to_pylist():
        k = (r.get("file_content_sha256") or "", r.get("column_name") or "")
        if k in wanted and k not in samples:
            samples[k] = [v for v in (r.get("sample_values_truncated") or "").split(SEP) if v]

    Xs, Xc, y = [], [], []
    for k, r in wanted.items():
        vals = samples.get(k) or _vendored_values(r.get("file_path", ""), r["column_name"])
        if not vals:
            continue
        Xs.append(col_stats(vals))
        Xc.append(column_distribution_features(vals))
        y.append(TYPES[r["curated_label"]])
    Xs, Xc, y = np.array(Xs), np.array(Xc), np.array(y)
    Xsc = np.concatenate([Xs, Xc], axis=1)
    from collections import Counter
    print(f"columns={len(y)} stats_dim={Xs.shape[1]} coldist_dim={Xc.shape[1]} per-class={dict(Counter(y))}")

    def cv(X, mask=None):
        clf = make_pipeline(StandardScaler(), LogisticRegression(max_iter=3000))
        if mask is None:
            return cross_val_score(clf, X, y, cv=5).mean()
        return cross_val_score(clf, X[mask], y[mask], cv=5).mean()

    def task(name, classes):
        m = np.isin(y, classes)
        if m.sum() < 20:
            print(f"  {name}: too few ({m.sum()})"); return
        ys = y.copy()
        s = cv(Xs[m][:, :], None) if False else cross_val_score(
            make_pipeline(StandardScaler(), LogisticRegression(max_iter=3000)), Xs[m], y[m], cv=5).mean()
        sc = cross_val_score(make_pipeline(StandardScaler(), LogisticRegression(max_iter=3000)),
                             Xsc[m], y[m], cv=5).mean()
        print(f"  {name:24} stats={s:.3f}  stats+coldist={sc:.3f}  lift={sc - s:+.3f}")

    print("\n5-fold CV separability (stats-27 vs stats-27+coldist):")
    print(f"  {'all-classes':24} stats={cv(Xs):.3f}  stats+coldist={cv(Xsc):.3f}  lift={cv(Xsc) - cv(Xs):+.3f}")
    task("coord-vs-decimal", ["coord", "decimal"])
    task("coord/dec/year/int", ["coord", "decimal", "year", "integer"])
    task("increment-vs-integer", ["increment", "integer"])


if __name__ == "__main__":
    main()
