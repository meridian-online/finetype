#!/usr/bin/env python3
"""ac-02 — Compute per-cluster reachability scores for corroborated gaps.

Spec: `2026-05-29-cluster-reachability-scoring`
Design: `spec 2026-05-29-cluster-reachability-scoring (metric.md)`

For each gap_id in `eval/gittables/corpus_pass/corroborated_gaps.parquet`,
build a value-shape embedding for every column in the cluster (defined
as columns in `columns.parquet` where `sense_prediction == fp_label`
AND `ydf_prediction == correct_label`), embed a 1000-sample baseline
of correct_label columns outside the cluster, and emit two cluster-
level scores:

  tightness    = 1 - mean intra-cluster pairwise cosine distance
                 (epsilon-smoothed for n < 50)
  specificity  = mean of per-column k=10 NN cosine distance to baseline

  reachability_score = clip(tightness * specificity, 0.0, 1.0)

The diagnostic clusters multiple gap_ids under a single (fp_label,
correct_label) pair (the label-flow direction). The metric is a
property of the label-flow direction, so all gap_ids sharing a
(fp_label, correct_label) get the same score. The full-gap output
satisfies ac-02's "one row per gap_id" close evidence while only
doing 878 distinct scoring runs (not 64k).

Output:
  output/cluster-reachability/cluster_scores.parquet
    columns: gap_id, criterion, mechanism, fp_label, correct_label,
             affected_column_count, n_cluster_columns,
             n_baseline_samples, tightness, specificity,
             reachability_score, error

Invocation (duckdb + numpy required — install via uv run --with):
  uv run --with duckdb --with numpy scripts/compute_cluster_reachability.py
  uv run --with duckdb --with numpy scripts/compute_cluster_reachability.py \\
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
DEFAULT_OUT = REPO / "output/cluster-reachability/cluster_scores.parquet"

EMBED_NGRAM_DIM = 1024
EMBED_LEN_DIM = 5
EMBED_CHARCLASS_DIM = 4
EMBED_DIM = EMBED_NGRAM_DIM + EMBED_LEN_DIM + EMBED_CHARCLASS_DIM

BASELINE_SIZE = 1000
BASELINE_MIN = 200
BASELINE_POOL_SIZE = 5000  # cached pool per correct_label, filtered per fp_label
K_NN = 10
PAIR_SAMPLE_CAP = 2000
SMOOTH_FLOOR = 50
SMOOTH_PSEUDO = 10
SEED = 44
MIN_CLUSTER = 5
LARGE_CLUSTER_CAP = 5000  # sample-cap for very large clusters in tightness


def _build_embedding(values: list[str], np):
    """Return EMBED_DIM-vector for one column's sampled values."""
    if not values:
        return np.zeros(EMBED_DIM, dtype=np.float32)

    ngrams = np.zeros(EMBED_NGRAM_DIM, dtype=np.float32)
    for v in values:
        if not v:
            continue
        s = v
        if len(s) < 3:
            s = s.ljust(3)
        for i in range(len(s) - 2):
            tri = s[i:i + 3]
            ngrams[hash(tri) % EMBED_NGRAM_DIM] += 1.0
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


def _compute_tightness(cluster_emb, np, rng):
    n = cluster_emb.shape[0]
    if n < 2:
        return 0.0

    # Down-sample very large clusters for the pairwise computation.
    if n > LARGE_CLUSTER_CAP:
        idx = rng.choice(n, size=LARGE_CLUSTER_CAP, replace=False)
        emb = cluster_emb[idx]
        n_pair = LARGE_CLUSTER_CAP
    else:
        emb = cluster_emb
        n_pair = n

    if n_pair * (n_pair - 1) // 2 > PAIR_SAMPLE_CAP:
        idx_a = rng.integers(0, n_pair, size=PAIR_SAMPLE_CAP * 2)
        idx_b = rng.integers(0, n_pair, size=PAIR_SAMPLE_CAP * 2)
        mask = idx_a != idx_b
        idx_a, idx_b = idx_a[mask][:PAIR_SAMPLE_CAP], idx_b[mask][:PAIR_SAMPLE_CAP]
        sims = (emb[idx_a] * emb[idx_b]).sum(axis=1)
        dists = np.clip(1.0 - sims, 0.0, 2.0) * 0.5
        mean_dist = float(dists.mean())
    else:
        sims = emb @ emb.T
        dists = np.clip(1.0 - sims, 0.0, 2.0) * 0.5
        triu = np.triu_indices(n_pair, k=1)
        mean_dist = float(dists[triu].mean())

    tightness_raw = 1.0 - mean_dist
    if n < SMOOTH_FLOOR:
        return tightness_raw * n / (n + SMOOTH_PSEUDO)
    return tightness_raw


def _compute_specificity(cluster_emb, baseline_emb, np, rng):
    if baseline_emb.shape[0] == 0:
        return None
    # Down-sample cluster if huge — k-NN cost is cluster_size × baseline_size.
    n = cluster_emb.shape[0]
    if n > LARGE_CLUSTER_CAP:
        idx = rng.choice(n, size=LARGE_CLUSTER_CAP, replace=False)
        emb = cluster_emb[idx]
    else:
        emb = cluster_emb
    sims = emb @ baseline_emb.T
    dist = np.clip(1.0 - sims, 0.0, 2.0) * 0.5
    k = min(K_NN, dist.shape[1])
    partitioned = np.partition(dist, k - 1, axis=1)[:, :k]
    per_col_mean = partitioned.mean(axis=1)
    return float(per_col_mean.mean())


def _load_baseline_pool(correct_label, columns_path, con, np):
    """Return (sample_values_truncated, sense_prediction, embedding_matrix)."""
    rows = con.execute(
        f"SELECT sample_values_truncated, sense_prediction FROM read_parquet(?) "
        f"WHERE ydf_prediction = ? AND sample_values_truncated IS NOT NULL "
        f"USING SAMPLE {BASELINE_POOL_SIZE} ROWS (reservoir, {SEED})",
        [str(columns_path), correct_label],
    ).fetchall()
    if not rows:
        return [], np.zeros((0, EMBED_DIM), dtype=np.float32)
    senses = [r[1] for r in rows]
    raws = [r[0] for r in rows]
    emb = _l2_normalise(_embed_batch(raws, np), np)
    return senses, emb


def _score_pair(fp_label, correct_label, columns_path, con, np,
                baseline_cache):
    rng = np.random.default_rng(SEED)

    # Cluster columns.
    cluster_rows = con.execute(
        "SELECT sample_values_truncated FROM read_parquet(?) "
        "WHERE sense_prediction = ? AND ydf_prediction = ? "
        "AND sample_values_truncated IS NOT NULL",
        [str(columns_path), fp_label, correct_label],
    ).fetchall()
    n_cluster = len(cluster_rows)
    if n_cluster < MIN_CLUSTER:
        return {
            "n_cluster_columns": n_cluster, "n_baseline_samples": 0,
            "tightness": None, "specificity": None,
            "reachability_score": None,
            "error": f"cluster size < {MIN_CLUSTER} columns ({n_cluster}), embedding unstable",
        }

    cluster_emb = _l2_normalise(
        _embed_batch([r[0] for r in cluster_rows], np), np
    )

    # Baseline: filter cached pool by sense != fp_label.
    if correct_label not in baseline_cache:
        baseline_cache[correct_label] = _load_baseline_pool(
            correct_label, columns_path, con, np
        )
    senses_pool, baseline_pool_emb = baseline_cache[correct_label]

    # Filter pool to exclude cluster (sense_prediction == fp_label).
    if not senses_pool:
        filtered_emb = baseline_pool_emb  # already zero-row
    else:
        mask = np.array([s != fp_label for s in senses_pool], dtype=bool)
        filtered_emb = baseline_pool_emb[mask]
    n_pool_filtered = filtered_emb.shape[0]

    if n_pool_filtered < BASELINE_MIN:
        tightness = _compute_tightness(cluster_emb, np, rng)
        return {
            "n_cluster_columns": n_cluster,
            "n_baseline_samples": n_pool_filtered,
            "tightness": tightness, "specificity": None,
            "reachability_score": None,
            "error": (
                f"baseline < {BASELINE_MIN} columns for correct_label "
                f"{correct_label} outside cluster (n={n_pool_filtered})"
            ),
        }

    # Take up to BASELINE_SIZE from the filtered pool.
    n_sample = min(BASELINE_SIZE, n_pool_filtered)
    if n_pool_filtered > BASELINE_SIZE:
        idx = rng.choice(n_pool_filtered, size=BASELINE_SIZE, replace=False)
        baseline_emb = filtered_emb[idx]
    else:
        baseline_emb = filtered_emb

    tightness = _compute_tightness(cluster_emb, np, rng)
    specificity = _compute_specificity(cluster_emb, baseline_emb, np, rng)

    if specificity is None:
        reachability = None
    else:
        reachability = max(0.0, min(1.0, tightness * specificity))

    return {
        "n_cluster_columns": n_cluster, "n_baseline_samples": n_sample,
        "tightness": tightness, "specificity": specificity,
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

    # Identify unique (fp_label, correct_label) pairs.
    pair_to_gaps: dict[tuple[str, str], list[tuple]] = {}
    for row in gap_rows:
        gap_id, criterion, mechanism, count, fp, correct = row
        if fp is None or correct is None:
            continue
        pair_to_gaps.setdefault((fp, correct), []).append(row)

    print(f"scoring {len(pair_to_gaps)} distinct (fp, correct) pairs "
          f"across {len(gap_rows)} gap_ids", file=sys.stderr)

    pair_scores: dict[tuple[str, str], dict] = {}
    baseline_cache: dict = {}
    t0 = time.time()
    for i, pair in enumerate(pair_to_gaps.keys(), 1):
        fp, correct = pair
        score = _score_pair(fp, correct, args.columns, con, np, baseline_cache)
        pair_scores[pair] = score
        if i % args.progress_every == 0 or i == len(pair_to_gaps):
            elapsed = time.time() - t0
            rate = i / max(elapsed, 1e-6)
            eta = (len(pair_to_gaps) - i) / max(rate, 1e-6)
            print(f"  [{i}/{len(pair_to_gaps)}] {fp[:30]:30s} -> "
                  f"{correct[:30]:30s} "
                  f"reach={score.get('reachability_score')} "
                  f"(elapsed {elapsed:.0f}s, ETA {eta:.0f}s)",
                  file=sys.stderr)

    args.out.parent.mkdir(parents=True, exist_ok=True)

    con.execute("DROP TABLE IF EXISTS scores")
    con.execute(
        "CREATE TABLE scores ("
        "gap_id VARCHAR, criterion VARCHAR, mechanism VARCHAR, "
        "fp_label VARCHAR, correct_label VARCHAR, "
        "affected_column_count BIGINT, "
        "n_cluster_columns BIGINT, n_baseline_samples BIGINT, "
        "tightness DOUBLE, specificity DOUBLE, "
        "reachability_score DOUBLE, error VARCHAR"
        ")"
    )

    inserted = 0
    for gap_id, criterion, mechanism, count, fp, correct in gap_rows:
        if fp is None or correct is None:
            score = {
                "n_cluster_columns": 0, "n_baseline_samples": 0,
                "tightness": None, "specificity": None,
                "reachability_score": None,
                "error": "fp_label or correct_label missing from sample_evidence",
            }
        else:
            score = pair_scores[(fp, correct)]
        con.execute(
            "INSERT INTO scores VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            [gap_id, criterion, mechanism, fp, correct, count,
             score["n_cluster_columns"], score["n_baseline_samples"],
             score["tightness"], score["specificity"],
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
