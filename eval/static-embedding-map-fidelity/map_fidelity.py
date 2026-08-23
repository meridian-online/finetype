# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "model2vec==0.9.0",
#   "sentence-transformers==6.0.0",
#   "umap-learn==0.5.12",
#   "scikit-learn==1.9.0",
#   "numpy==2.5.2",
# ]
# ///
"""Does a static embedding change what a 2D map looks like?

The goal a private planning repo records for this work is that an analyst can
embed a text column locally, with no API key and no per-row bill, and still get
a 2D map that "shows the cluster structure a hosted transformer would have
shown". The published quality gap for static embeddings is a RETRIEVAL gap.
This harness measures whether that gap reaches the picture, which is the
question the decision to build actually turns on.

WHAT IS COMPARED. Each corpus is embedded by every embedder in EMBEDDERS, then
projected with identical, seeded UMAP settings mirroring the reference stack --
`umap.UMAP(metric="cosine", n_neighbors=15, random_state=SEED)` over a
precomputed kNN graph, which is what
`embedding_atlas/projection.py:_run_umap` does. Identical projection settings
are what let a difference between two maps be attributed to the embedder.

THE CONTROL IS THE POINT. `random-384` embeds nothing: it returns seeded
Gaussian noise. Every score it earns is the score this measurement gives to an
embedder that knows nothing, so it is the floor a real number has to clear. A
measure that cannot separate MiniLM from noise is not measuring cluster
structure, and running without the control would hide that. `retention` below is
defined against that floor for the same reason -- a raw AMI ratio flatters a
weak embedder whenever the task is easy.

WHAT IS NOT MEASURED. Speed. It is already measured elsewhere, it is not the
argument for the milestone, and mixing it in invites the embedding-only figure
into a product claim it does not support.

  uv run --script eval/static-embedding-map-fidelity/map_fidelity.py --out results.json
"""

from __future__ import annotations

import argparse
import json
import platform
import sys
from dataclasses import dataclass, asdict
from pathlib import Path

import numpy as np

# The default, and the ONLY seed the module reads directly. Everything downstream takes
# `args.seed` as an argument -- the corpus shuffle and the noise control included -- so
# `--seed 7` reseeds the whole run rather than half of it. It used to reseed UMAP and
# k-means while leaving the control and the document order pinned at 42, which is the
# shape of bug that makes a seed sweep look like a real effect.
SEED = 42

# Mirrors embedding_atlas/projection.py:_run_umap defaults, which is what makes
# "the same seeded projection settings" (AC1) true rather than merely claimed.
UMAP_METRIC = "cosine"
UMAP_N_NEIGHBORS = 15

# How many neighbours the map-to-map agreement measure compares. Independent of
# UMAP_N_NEIGHBORS: this asks whether two PICTURES place the same points
# together, not how either was built.
AGREEMENT_K = 20


@dataclass(frozen=True)
class Corpus:
    key: str
    shape: str  # "long-form" | "short" | "very-short"
    description: str


CORPORA = [
    Corpus("20news-body", "long-form", "20 Newsgroups posts, headers/footers/quotes stripped"),
    Corpus("20news-subject", "short", "The Subject: line of the same posts, same labels"),
    Corpus("finetype-columns", "very-short", "Column names from finetype's representative corpus, labelled by semantic type"),
]

EMBEDDERS = [
    ("minilm", "sentence-transformers/all-MiniLM-L6-v2", "transformer"),
    ("potion-8m", "minishlab/potion-base-8M", "static"),
    ("potion-4m", "minishlab/potion-base-4M", "static"),
    ("random-384", None, "control"),
]


def load_corpus(corpus: Corpus, limit: int, repo_root: Path, seed: int) -> tuple[list[str], np.ndarray]:
    """Return (texts, labels). Labels are integer class ids."""
    if corpus.key.startswith("20news"):
        from sklearn.datasets import fetch_20newsgroups

        bunch = fetch_20newsgroups(
            subset="train",
            remove=("headers", "footers", "quotes"),
            random_state=seed,
            shuffle=True,
        )
        texts_raw = list(bunch.data)
        labels_raw = np.asarray(bunch.target)

        if corpus.key == "20news-subject":
            # The Subject: line is stripped by remove=("headers",), so re-read it
            # from the unstripped copy. Same documents, same order, same labels.
            full = fetch_20newsgroups(subset="train", random_state=seed, shuffle=True)
            texts_raw = [_subject_of(d) for d in full.data]

        keep = [i for i, t in enumerate(texts_raw) if len(t.strip()) >= 8]
        keep = keep[:limit]
        return [texts_raw[i].strip() for i in keep], labels_raw[keep]

    if corpus.key == "finetype-columns":
        import csv

        path = repo_root / "eval" / "repr" / "representative_corpus.tsv"
        rows = list(csv.DictReader(path.open(), delimiter="\t"))
        counts: dict[str, int] = {}
        for r in rows:
            counts[r["curated_label"]] = counts.get(r["curated_label"], 0) + 1
        # A class with a handful of members cannot be found by any clustering at
        # this scale, so keeping it only adds noise to the score. Stated, not silent.
        keep_labels = sorted(l for l, c in counts.items() if c >= 5)
        idx = {l: i for i, l in enumerate(keep_labels)}
        texts, labels = [], []
        for r in rows:
            if r["curated_label"] in idx and r["column_name"].strip():
                texts.append(r["column_name"].strip())
                labels.append(idx[r["curated_label"]])
        return texts[:limit], np.asarray(labels[:limit])

    raise ValueError(f"unknown corpus {corpus.key}")


def _subject_of(document: str) -> str:
    for line in document.splitlines():
        if line.startswith("Subject:"):
            return line[len("Subject:"):].replace("Re:", " ").strip()
    return ""


def embed(key: str, model_name: str | None, texts: list[str], seed: int) -> np.ndarray:
    if key == "random-384":
        rng = np.random.default_rng(seed)
        return rng.normal(size=(len(texts), 384)).astype(np.float32)
    if model_name is not None and model_name.startswith("minishlab/"):
        from model2vec import StaticModel

        return StaticModel.from_pretrained(model_name).encode(texts).astype(np.float32)
    from sentence_transformers import SentenceTransformer

    return SentenceTransformer(model_name).encode(texts, show_progress_bar=False).astype(np.float32)


def project(vectors: np.ndarray, seed: int) -> np.ndarray:
    """UMAP to 2D, with the reference stack's settings and a fixed random_state."""
    import umap
    from umap.umap_ import nearest_neighbors

    n_neighbors = min(UMAP_N_NEIGHBORS, max(2, len(vectors) - 1))
    knn = nearest_neighbors(
        vectors,
        n_neighbors=n_neighbors,
        metric=UMAP_METRIC,
        metric_kwds=None,
        angular=False,
        random_state=seed,
    )
    reducer = umap.UMAP(
        n_neighbors=n_neighbors,
        metric=UMAP_METRIC,
        random_state=seed,
        precomputed_knn=knn,
    )
    return np.asarray(reducer.fit_transform(vectors))


def cluster_agreement(points: np.ndarray, labels: np.ndarray, seed: int) -> float:
    """AMI between a k-means clustering of the map and the ground-truth labels.

    AMI rather than ARI because the classes are unbalanced in every corpus here
    and AMI is the one corrected for that. k is the true class count: this asks
    whether the STRUCTURE is legible, not whether an analyst could guess k.
    """
    from sklearn.cluster import KMeans
    from sklearn.metrics import adjusted_mutual_info_score

    k = int(len(np.unique(labels)))
    assignment = KMeans(n_clusters=k, random_state=seed, n_init=10).fit_predict(points)
    return float(adjusted_mutual_info_score(labels, assignment))


def map_overlap(a: np.ndarray, b: np.ndarray, k: int) -> float:
    """Mean fraction of each point's k nearest neighbours shared between two maps.

    Label-free, so it answers a different question from AMI: do the two PICTURES
    put the same points next to each other, whether or not either is any good?
    """
    from sklearn.neighbors import NearestNeighbors

    k = min(k, len(a) - 1)
    na = NearestNeighbors(n_neighbors=k + 1).fit(a).kneighbors(a, return_distance=False)[:, 1:]
    nb = NearestNeighbors(n_neighbors=k + 1).fit(b).kneighbors(b, return_distance=False)[:, 1:]
    return float(np.mean([len(set(x) & set(y)) / k for x, y in zip(na, nb)]))


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, required=True, help="results JSON to write")
    ap.add_argument("--limit", type=int, default=3000, help="max rows per corpus")
    ap.add_argument("--seed", type=int, default=SEED)
    ap.add_argument("--corpus", action="append", help="restrict to these corpus keys")
    args = ap.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    corpora = [c for c in CORPORA if not args.corpus or c.key in args.corpus]

    results: list[dict] = []
    for corpus in corpora:
        texts, labels = load_corpus(corpus, args.limit, repo_root, args.seed)
        print(f"[{corpus.key}] {len(texts)} rows, {len(np.unique(labels))} classes", file=sys.stderr)

        maps: dict[str, np.ndarray] = {}
        rows: dict[str, dict] = {}
        for key, model_name, kind in EMBEDDERS:
            vectors = embed(key, model_name, texts, args.seed)
            points = project(vectors, args.seed)
            maps[key] = points
            rows[key] = {
                "embedder": key,
                "kind": kind,
                "model": model_name,
                "dim": int(vectors.shape[1]),
                "ami_map": cluster_agreement(points, labels, args.seed),
                "ami_vectors": cluster_agreement(vectors, labels, args.seed),
            }
            print(f"  {key}: AMI(map)={rows[key]['ami_map']:.4f} AMI(vectors)={rows[key]['ami_vectors']:.4f}", file=sys.stderr)

        floor = rows["random-384"]["ami_map"]
        ceiling = rows["minilm"]["ami_map"]
        for key, row in rows.items():
            # Retention against the noise floor, not against zero: on an easy
            # corpus a raw ratio makes a weak embedder look close to a good one.
            span = ceiling - floor
            row["retention_vs_minilm"] = None if span <= 0 else (row["ami_map"] - floor) / span
            row["map_overlap_with_minilm"] = map_overlap(maps[key], maps["minilm"], AGREEMENT_K)

        results.append({
            "corpus": corpus.key,
            "shape": corpus.shape,
            "description": corpus.description,
            "rows": len(texts),
            "classes": int(len(np.unique(labels))),
            "embedders": list(rows.values()),
        })

    payload = {
        "seed": args.seed,
        "limit": args.limit,
        "umap": {"metric": UMAP_METRIC, "n_neighbors": UMAP_N_NEIGHBORS, "random_state": args.seed},
        "agreement_k": AGREEMENT_K,
        "platform": f"{platform.system()} {platform.machine()} python{platform.python_version()}",
        "corpora": [asdict(c) for c in corpora],
        "results": results,
    }
    args.out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
