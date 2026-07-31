#!/usr/bin/env python3
"""ac-02 — Compute per-cluster v2 reachability scores via neighbour-label
composition.

Spec: `2026-05-30-reachability-metric-v2`
Design: `spec 2026-05-30-reachability-metric-v2 (metric.md)`

For each (fp_label, correct_label) pair from corroborated_gaps.parquet:
  1. Embed cluster columns + a 50,000-row stratified neighbour pool.
  2. For each cluster column, find k=100 nearest neighbours in the pool
     (excluding the column itself via (file_path, column_name) match).
  3. Bucket neighbours by ydf_prediction:
       absorption_c = fraction of NN with ydf_prediction == correct_label
       risk_c       = fraction of NN with ydf_prediction != correct_label
                      AND sense_prediction != correct_label
  4. absorption = mean(absorption_c), risk = mean(risk_c)
  5. reachability_score = clip(absorption * (1 - risk), 0, 1)

All gap_ids sharing a (fp_label, correct_label) pair get the same score
(the metric is a property of the label-flow direction).

Output:
  output/cluster-reachability/cluster_scores_v2.parquet
    columns: gap_id, criterion, mechanism, fp_label, correct_label,
             affected_column_count, n_cluster_columns,
             n_neighbour_pool, absorption, risk,
             reachability_score, error

Invocation:
  uv run --with duckdb --with numpy scripts/compute_cluster_reachability_v2.py
  uv run --with duckdb --with numpy scripts/compute_cluster_reachability_v2.py \\
      --gap-ids 721b890ea74d,1b858e0d073b,3f2aa8465552,20803deffbad,81b63a52e3ef,cdde5d05b73a
"""
from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_GAPS = REPO / "eval/gittables/corpus_pass/corroborated_gaps.parquet"
DEFAULT_COLS = REPO / "eval/gittables/corpus_pass/columns.parquet"
DEFAULT_OUT = REPO / "output/cluster-reachability/cluster_scores_v2.parquet"

EMBED_NGRAM_DIM = 1024
EMBED_LEN_DIM = 5
EMBED_CHARCLASS_DIM = 4
EMBED_DIM = EMBED_NGRAM_DIM + EMBED_LEN_DIM + EMBED_CHARCLASS_DIM

POOL_SIZE = 50_000
POOL_LABEL_FLOOR = 1_000
K_NN = 100
MIN_CLUSTER = 5
LARGE_CLUSTER_CAP = 5_000
SEED = 44


def _build_embedding(values: list[str], np):
    if not values:
        return np.zeros(EMBED_DIM, dtype=np.float32)
    ngrams = np.zeros(EMBED_NGRAM_DIM, dtype=np.float32)
    for v in values:
        if not v:
            continue
        s = v if len(v) >= 3 else v.ljust(3)
        for i in range(len(s) - 2):
            ngrams[hash(s[i:i + 3]) % EMBED_NGRAM_DIM] += 1.0
    ngram_sum = ngrams.sum()
    if ngram_sum > 0:
        ngrams /= ngram_sum

    lens = np.array([len(v) for v in values if v is not None],
                    dtype=np.float32)
    if lens.size:
        pcts = np.percentile(lens, [10, 25, 50, 75, 90]) / 64.0
    else:
        pcts = np.zeros(5, dtype=np.float32)
    pcts = np.clip(pcts.astype(np.float32), 0.0, 4.0)

    alpha = numer = punct = other = total = 0
    for v in values:
        if not v:
            continue
        for ch in v:
            total += 1
            if ch.isalpha():
                alpha += 1
            elif ch.isdigit():
                numer += 1
            elif ch.isspace():
                other += 1
            elif ch.isprintable():
                punct += 1
            else:
                other += 1
    if total > 0:
        charclass = np.array([alpha, numer, punct, other],
                             dtype=np.float32) / total
    else:
        charclass = np.zeros(4, dtype=np.float32)

    return np.concatenate([ngrams, pcts, charclass]).astype(np.float32)


def _embed_batch(rows: list[str], np):
    out = np.empty((len(rows), EMBED_DIM), dtype=np.float32)
    for i, raw in enumerate(rows):
        if raw is None:
            values: list[str] = []
        else:
            values = [v for v in raw.split("|") if v]
        out[i] = _build_embedding(values, np)
    return out


def _l2_normalise(matrix, np):
    norms = np.linalg.norm(matrix, axis=1, keepdims=True)
    norms = np.where(norms == 0, 1.0, norms)
    return matrix / norms


def _build_neighbour_pool(columns_path, con, np):
    """Build the stratified 50k-row neighbour pool.

    Returns:
      (file_paths, column_names, ydf_preds, sense_preds, embedding_matrix)
    """
    print(f"building stratified neighbour pool ({POOL_SIZE:,} rows)...",
          file=sys.stderr)
    t0 = time.time()

    # Per-label population counts.
    label_counts = con.execute(
        "SELECT ydf_prediction, count(*) AS n FROM read_parquet(?) "
        "WHERE ydf_prediction IS NOT NULL "
        "AND sample_values_truncated IS NOT NULL GROUP BY 1",
        [str(columns_path)],
    ).fetchall()
    if not label_counts:
        raise RuntimeError("no rows with ydf_prediction in columns.parquet")

    labels = [row[0] for row in label_counts]
    counts = {row[0]: row[1] for row in label_counts}
    n_labels = len(labels)
    even_budget = POOL_SIZE // n_labels
    print(f"  n_labels={n_labels}, even_budget={even_budget}, "
          f"total_population={sum(counts.values()):,}", file=sys.stderr)

    # First pass: take min(even_budget, population) from each label.
    per_label_quota: dict[str, int] = {}
    for lbl in labels:
        per_label_quota[lbl] = min(even_budget, counts[lbl])
    print(f"  pass1 sum={sum(per_label_quota.values())}, "
          f"max quota={max(per_label_quota.values())}", file=sys.stderr)

    # Second pass: distribute shortfall proportionally to remaining
    # population across labels with extras, then top up residual rounding.
    shortfall = POOL_SIZE - sum(per_label_quota.values())
    if shortfall > 0:
        extras = [(counts[lbl] - per_label_quota[lbl], lbl) for lbl in labels
                  if counts[lbl] > per_label_quota[lbl]]
        total_extra = sum(e[0] for e in extras)
        if total_extra > 0:
            for extra, lbl in extras:
                share = (extra * shortfall) // total_extra
                take = min(extra, share)
                per_label_quota[lbl] += take
        # Residual top-up (rounding error), largest-extra-first.
        remaining = POOL_SIZE - sum(per_label_quota.values())
        if remaining > 0:
            residual = sorted(
                ((counts[lbl] - per_label_quota[lbl], lbl) for lbl in labels
                 if counts[lbl] > per_label_quota[lbl]),
                reverse=True,
            )
            for extra, lbl in residual:
                if remaining <= 0:
                    break
                take = min(extra, remaining)
                per_label_quota[lbl] += take
                remaining -= take
    print(f"  pass2 final sum={sum(per_label_quota.values())}, "
          f"shortfall remaining={shortfall}", file=sys.stderr)

    # Fetch the strata.
    all_rows: list[tuple] = []
    for lbl in labels:
        quota = per_label_quota[lbl]
        if quota == 0:
            continue
        rows = con.execute(
            f"SELECT file_path, column_name, ydf_prediction, "
            f"sense_prediction, sample_values_truncated "
            f"FROM (SELECT * FROM read_parquet(?) "
            f"WHERE ydf_prediction = ? "
            f"AND sample_values_truncated IS NOT NULL) "
            f"USING SAMPLE {quota} ROWS (reservoir, {SEED})",
            [str(columns_path), lbl],
        ).fetchall()
        all_rows.extend(rows)

    print(f"  fetched {len(all_rows):,} pool rows in {time.time() - t0:.1f}s",
          file=sys.stderr)

    t1 = time.time()
    file_paths = [r[0] for r in all_rows]
    column_names = [r[1] for r in all_rows]
    ydf_preds = [r[2] for r in all_rows]
    sense_preds = [r[3] for r in all_rows]
    raws = [r[4] for r in all_rows]
    embeddings = _l2_normalise(_embed_batch(raws, np), np)
    print(f"  embedded pool in {time.time() - t1:.1f}s", file=sys.stderr)
    return file_paths, column_names, ydf_preds, sense_preds, embeddings


def _score_pair(fp_label, correct_label, columns_path, con, np,
                pool_data, pool_label_counts):
    file_paths, column_names, ydf_preds, sense_preds, pool_emb = pool_data

    n_pool_for_label = pool_label_counts.get(correct_label, 0)
    if n_pool_for_label < POOL_LABEL_FLOOR:
        return {
            "n_cluster_columns": 0,
            "n_neighbour_pool": n_pool_for_label,
            "absorption": None, "risk": None,
            "reachability_score": None,
            "error": (
                f"neighbour pool has {n_pool_for_label} columns for "
                f"correct_label {correct_label} (below {POOL_LABEL_FLOOR} floor)"
            ),
        }

    cluster_rows = con.execute(
        "SELECT file_path, column_name, sample_values_truncated "
        "FROM read_parquet(?) "
        "WHERE sense_prediction = ? AND ydf_prediction = ? "
        "AND sample_values_truncated IS NOT NULL",
        [str(columns_path), fp_label, correct_label],
    ).fetchall()
    n_cluster = len(cluster_rows)
    if n_cluster < MIN_CLUSTER:
        return {
            "n_cluster_columns": n_cluster,
            "n_neighbour_pool": n_pool_for_label,
            "absorption": None, "risk": None,
            "reachability_score": None,
            "error": (
                f"cluster size < {MIN_CLUSTER} columns ({n_cluster}), "
                f"embedding unstable"
            ),
        }

    # Cap cluster size for tractable matmul cost.
    rng = np.random.default_rng(SEED)
    if n_cluster > LARGE_CLUSTER_CAP:
        idx = rng.choice(n_cluster, size=LARGE_CLUSTER_CAP, replace=False)
        cluster_rows = [cluster_rows[i] for i in idx]

    cluster_file_path_set = {(r[0], r[1]) for r in cluster_rows}
    cluster_emb = _l2_normalise(
        _embed_batch([r[2] for r in cluster_rows], np), np
    )

    # Build a boolean mask over the pool excluding the cluster's columns
    # by (file_path, column_name).
    n_pool = pool_emb.shape[0]
    pool_keep = np.array(
        [(file_paths[i], column_names[i]) not in cluster_file_path_set
         for i in range(n_pool)],
        dtype=bool,
    )
    kept_pool_emb = pool_emb[pool_keep]
    if kept_pool_emb.shape[0] < K_NN:
        return {
            "n_cluster_columns": n_cluster,
            "n_neighbour_pool": int(kept_pool_emb.shape[0]),
            "absorption": None, "risk": None,
            "reachability_score": None,
            "error": (
                f"pool after cluster exclusion has {kept_pool_emb.shape[0]} "
                f"rows (< K_NN={K_NN})"
            ),
        }
    # Map kept-pool indices back to original pool indices for label lookup.
    kept_idx = np.where(pool_keep)[0]

    # Cosine similarity (both sides L2-normalised) → distance proxy
    # by negating similarity (argpartition picks smallest distances).
    sims = cluster_emb @ kept_pool_emb.T
    # Find top-K_NN by similarity per cluster column.
    k = min(K_NN, sims.shape[1])
    topk_idx_in_kept = np.argpartition(-sims, k - 1, axis=1)[:, :k]

    # Compute absorption_c and risk_c per cluster row.
    absorption_per_c = np.empty(cluster_emb.shape[0], dtype=np.float64)
    risk_per_c = np.empty(cluster_emb.shape[0], dtype=np.float64)
    for ci in range(cluster_emb.shape[0]):
        nn_kept = topk_idx_in_kept[ci]
        nn_orig = kept_idx[nn_kept]
        absorbed = 0
        risky = 0
        for orig_i in nn_orig:
            ydf = ydf_preds[orig_i]
            sense = sense_preds[orig_i]
            if ydf == correct_label:
                absorbed += 1
            elif sense != correct_label:
                risky += 1
        absorption_per_c[ci] = absorbed / k
        risk_per_c[ci] = risky / k

    absorption = float(absorption_per_c.mean())
    risk = float(risk_per_c.mean())
    reachability = max(0.0, min(1.0, absorption * (1.0 - risk)))

    return {
        "n_cluster_columns": n_cluster,
        "n_neighbour_pool": n_pool_for_label,
        "absorption": absorption, "risk": risk,
        "reachability_score": reachability,
        "error": None,
    }


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--gaps", type=Path, default=DEFAULT_GAPS)
    p.add_argument("--columns", type=Path, default=DEFAULT_COLS)
    p.add_argument("--gap-ids", type=str, default=None,
                   help="Comma-separated gap_id prefixes (default: all gaps)")
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--progress-every", type=int, default=25)
    args = p.parse_args()

    try:
        import duckdb  # type: ignore
        import numpy as np  # type: ignore
    except ImportError as exc:
        print(f"error: missing dep ({exc}); install via uv run --with duckdb --with numpy",
              file=sys.stderr)
        return 2

    if not args.gaps.exists():
        print(f"error: gaps parquet missing at {args.gaps}", file=sys.stderr)
        return 2
    if not args.columns.exists():
        print(f"error: columns parquet missing at {args.columns}", file=sys.stderr)
        return 2

    con = duckdb.connect()

    if args.gap_ids:
        prefixes = [g.strip() for g in args.gap_ids.split(",") if g.strip()]
        where = " OR ".join(["gap_id LIKE ?"] * len(prefixes))
        params = [f"{p}%" for p in prefixes]
        gap_rows = con.execute(
            f"SELECT gap_id, criterion, mechanism, affected_column_count, "
            f"sample_evidence[1].sense_prediction AS fp_label, "
            f"sample_evidence[1].ydf_prediction AS correct_label "
            f"FROM read_parquet(?) WHERE {where} "
            f"ORDER BY affected_column_count DESC",
            [str(args.gaps), *params],
        ).fetchall()
    else:
        gap_rows = con.execute(
            "SELECT gap_id, criterion, mechanism, affected_column_count, "
            "sample_evidence[1].sense_prediction AS fp_label, "
            "sample_evidence[1].ydf_prediction AS correct_label "
            "FROM read_parquet(?) "
            "ORDER BY affected_column_count DESC",
            [str(args.gaps)],
        ).fetchall()

    if not gap_rows:
        print("error: no gap_ids matched", file=sys.stderr)
        return 2

    pair_to_gaps: dict[tuple[str, str], list[tuple]] = {}
    for row in gap_rows:
        gap_id, criterion, mechanism, count, fp, correct = row
        if fp is None or correct is None:
            continue
        pair_to_gaps.setdefault((fp, correct), []).append(row)

    print(f"scoring {len(pair_to_gaps)} distinct (fp, correct) pairs "
          f"across {len(gap_rows)} gap_ids", file=sys.stderr)

    pool_data = _build_neighbour_pool(args.columns, con, np)
    _, _, ydf_preds, _, _ = pool_data
    pool_label_counts: dict[str, int] = {}
    for lbl in ydf_preds:
        pool_label_counts[lbl] = pool_label_counts.get(lbl, 0) + 1

    pair_scores: dict[tuple[str, str], dict] = {}
    t0 = time.time()
    for i, pair in enumerate(pair_to_gaps.keys(), 1):
        fp, correct = pair
        score = _score_pair(fp, correct, args.columns, con, np,
                            pool_data, pool_label_counts)
        pair_scores[pair] = score
        if i % args.progress_every == 0 or i == len(pair_to_gaps):
            elapsed = time.time() - t0
            rate = i / max(elapsed, 1e-6)
            eta = (len(pair_to_gaps) - i) / max(rate, 1e-6)
            reach = score.get("reachability_score")
            absorb = score.get("absorption")
            risk_v = score.get("risk")
            print(
                f"  [{i}/{len(pair_to_gaps)}] {fp[:28]:28s} -> "
                f"{correct[:28]:28s} "
                f"absorb={absorb!s:.6} risk={risk_v!s:.6} "
                f"reach={reach!s:.6} "
                f"(elapsed {elapsed:.0f}s, ETA {eta:.0f}s)",
                file=sys.stderr,
            )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    con.execute("DROP TABLE IF EXISTS scores")
    con.execute(
        "CREATE TABLE scores ("
        "gap_id VARCHAR, criterion VARCHAR, mechanism VARCHAR, "
        "fp_label VARCHAR, correct_label VARCHAR, "
        "affected_column_count BIGINT, "
        "n_cluster_columns BIGINT, n_neighbour_pool BIGINT, "
        "absorption DOUBLE, risk DOUBLE, "
        "reachability_score DOUBLE, error VARCHAR"
        ")"
    )
    inserted = 0
    for gap_id, criterion, mechanism, count, fp, correct in gap_rows:
        if fp is None or correct is None:
            score = {
                "n_cluster_columns": 0, "n_neighbour_pool": 0,
                "absorption": None, "risk": None,
                "reachability_score": None,
                "error": "fp_label or correct_label missing from sample_evidence",
            }
        else:
            score = pair_scores[(fp, correct)]
        con.execute(
            "INSERT INTO scores VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [gap_id, criterion, mechanism, fp, correct, count,
             score["n_cluster_columns"], score["n_neighbour_pool"],
             score["absorption"], score["risk"],
             score["reachability_score"], score["error"]],
        )
        inserted += 1
    con.execute(
        "COPY (SELECT * FROM scores ORDER BY reachability_score DESC NULLS LAST) "
        "TO ? (FORMAT PARQUET)",
        [str(args.out)],
    )
    print(f"wrote {args.out} ({inserted} rows; "
          f"{len(pair_to_gaps)} unique pairs scored)",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
