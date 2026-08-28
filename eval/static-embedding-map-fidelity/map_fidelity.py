# /// script
# requires-python = ">=3.11"
# dependencies = [
#   "model2vec==0.9.0",
#   "sentence-transformers==6.0.0",
#   "umap-learn==0.5.12",
#   "scikit-learn==1.9.0",
#   "numpy==2.5.2",
#   "duckdb==1.5.5",
# ]
# ///
"""What does a static embedding cost, and which property of the model is charging?

The goal a private planning repo records for this work is that an analyst can
embed a text column locally, with no API key and no per-row bill, and still get
a 2D map that "shows the cluster structure a hosted transformer would have
shown". The published quality gap for static embeddings is a RETRIEVAL gap.
This harness measures whether that gap reaches the picture, whether it reaches
ranked retrieval on our own corpora, and whether it reaches the pairwise
question -- and it varies model size and training objective so that a gap can be
attributed to one of them rather than to "static embeddings".

WHAT IS COMPARED. Each corpus is embedded by every embedder in EMBEDDERS, then
projected with identical, seeded UMAP settings mirroring the reference stack --
`umap.UMAP(metric="cosine", n_neighbors=15, random_state=SEED)` over a
precomputed kNN graph, which is what
`embedding_atlas/projection.py:_run_umap` does. Identical projection settings
are what let a difference between two maps be attributed to the embedder.

THE MODEL LADDER IS THE EXPERIMENT. `potion-base-4M`, `-8M` and `-32M` vary
SIZE (and, between 8M and 32M, vocabulary: 29,528 tokens against 63,091).
`potion-retrieval-32M` is `potion-base-32M` fine-tuned on a retrieval
objective -- same base, same tokenizer, same 63,091-token vocabulary, same
on-disk format -- so the 32M pair varies OBJECTIVE and nothing else. A gap that
closes across the size ladder is a size or vocabulary gap; a gap that closes
only at the retrieval arm is an objective gap; a gap that survives both is a
property of the architecture. `vocab_type_coverage` and `vocab_token_coverage`
are recorded per corpus per model so the vocabulary reading is a measured
number rather than an inference from the size.

THREE QUESTIONS, NOT ONE. They are separated because a model class can be
strong at one and weak at another, and quoting either alone misdescribes it.

  map      -- `ami_map`, `retention_vs_minilm`, `map_overlap_with_minilm`.
              What the 2D PICTURE shows, and whether two pictures put the same
              points together.
  ranked   -- `precision_at_k`, `mrr_at_k`. Given one document as a query, are
              the top-k nearest in the FULL-DIMENSION vector space the same
              class? This is the question the published static-embedding gap is
              about, and it is measured on the vectors, not on the map.
  pairwise -- `pairwise_ap_same_class`, `pairwise_ap_near_duplicate`. Given
              exactly two texts and ONE global threshold, are they the same
              class / near-duplicates of each other? Pooled average precision,
              so the score ordering is global: this is deliberately not a
              per-query ranking, because per-query normalisation would turn it
              back into the ranked question.

TWO FLOORS, AND THEY ARE THE POINT. `random-384` embeds nothing: it returns
seeded Gaussian noise. Every score it earns is the score this measurement gives
to an embedder that knows nothing, so it is the floor a real number has to
clear. A measure that cannot separate MiniLM from noise is not measuring
anything, and running without the control would hide that. `bm25` is the second
floor and a different kind: DuckDB's `fts` extension over the same corpora,
which is what an analyst already has without any model at all. A dense arm that
does not beat BM25 has not earned its download.

The floors are not all at zero and the numbers are not comparable until you
know where each one sits. AMI and map overlap floor at ~0 because noise has no
structure. `precision_at_k` floors at the CLASS PRIOR -- a uniform random
retriever returns the query's class at its base rate -- so read
`lift_over_random`, which normalises against the measured `random-384` arm the
way `retention_vs_minilm` does. Pooled AP floors at the POSITIVE RATE, which
this harness fixes at 0.5 by construction (one positive and one negative pair
per anchor), so an arm at 0.5 is scoring at chance.

ONE PASS, ONE INDEX. Every figure in the output file comes from one invocation
over one seeded sample of one corpus. The retrieval queries and the pairwise
anchors are the SAME sampled row set, and every arm sees it. Two figures
measured against two samples read exactly like two figures measured against
one, which is why this is a property of the harness rather than of how it
happens to be called.

WHAT IS NOT MEASURED. Speed. It is already measured elsewhere, it is not the
argument for the milestone, and mixing it in invites the embedding-only figure
into a product claim it does not support.

  uv run --script eval/static-embedding-map-fidelity/map_fidelity.py --out results.json

`check_results.py` reads the file this writes and refuses one whose floors have
stopped behaving. It is stdlib-only and runs in CI; this script does not.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass, asdict
from datetime import datetime, timezone
from pathlib import Path

import numpy as np

# The default, and the ONLY seed the module reads directly. Everything downstream takes
# `args.seed` as an argument -- the corpus shuffle, the noise control, the probe sample,
# the pair construction and the near-duplicate corruptions included -- so `--seed 7`
# reseeds the whole run rather than half of it. A run that reseeds the projection while
# pinning the sample is the shape of bug that makes a seed sweep look like a real effect.
SEED = 42

# Mirrors embedding_atlas/projection.py:_run_umap defaults, which is what makes
# "the same seeded projection settings" true rather than merely claimed.
UMAP_METRIC = "cosine"
UMAP_N_NEIGHBORS = 15

# How many neighbours the map-to-map agreement measure compares. Independent of
# UMAP_N_NEIGHBORS: this asks whether two PICTURES place the same points
# together, not how either was built.
AGREEMENT_K = 20

# How many neighbours the RANKED arm retrieves, in the full-dimension vector
# space rather than on the map. Ten because that is the depth the published
# static-embedding figures use (NDCG@10), so a reader can put ours beside theirs
# without rescaling.
RETRIEVAL_K = 10

# One sampled row set per corpus, used as the retrieval queries AND as the
# pairwise anchors. Shared on purpose: it is what makes the ranked and pairwise
# numbers two readings of one sample rather than two samples. Capped because the
# lexical arm costs one full-text query per probe and a 20 Newsgroups body is a
# several-hundred-term query.
PROBE_ROWS = 800

# Each anchor contributes exactly one positive and one negative pair, so the
# pooled positive rate is 0.5 and an arm scoring at chance scores 0.5. Stated
# rather than derived, because average precision is meaningless without it.
PAIRWISE_POSITIVE_RATE = 0.5

# Fraction of whitespace tokens a long-text near-duplicate drops. Short texts
# (identifiers) take the identifier corruptions instead; see near_duplicate().
NEAR_DUP_DROP = 0.30

# Redistribution terms, read from each model's Hugging Face card metadata at run
# time and written into the output. These are the fallbacks used when the live
# read fails, and the output says which of the two it used. A bundled model is a
# redistribution, so this is recorded whether or not anything is bundled today.
LICENCE_FALLBACK = {
    "sentence-transformers/all-MiniLM-L6-v2": "apache-2.0",
    "minishlab/potion-base-4M": "mit",
    "minishlab/potion-base-8M": "mit",
    "minishlab/potion-base-32M": "mit",
    "minishlab/potion-retrieval-32M": "mit",
}

WORD = re.compile(r"[a-z0-9]+")

# Fixed strings every model encodes, so each arm carries a fingerprint of what it
# actually produced. The name in the output says which model was ASKED for; this
# says which one answered. Two arms with one fingerprint is the failure that
# leaves the most plausible file -- a typo in a repository id, a cache alias, a
# loader that falls back -- because every number stays in range and the columns
# just agree with each other, which is what this harness would otherwise report
# as a finding. Two of the five arms share a dimension count, so shape alone would
# not have caught the pair that matters.
FINGERPRINT_TEXTS = [
    "customer_id",
    "the quick brown fox jumps over the lazy dog",
    "consolidated statement of cash flows",
]


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

# Ordered as the experiment reads: the ceiling, then the size ladder, then the
# one model that differs from its neighbour only in training objective, then the
# noise floor.
EMBEDDERS = [
    ("minilm", "sentence-transformers/all-MiniLM-L6-v2", "transformer"),
    ("potion-4m", "minishlab/potion-base-4M", "static"),
    ("potion-8m", "minishlab/potion-base-8M", "static"),
    ("potion-32m", "minishlab/potion-base-32M", "static"),
    ("potion-retrieval-32m", "minishlab/potion-retrieval-32M", "static-retrieval"),
    ("random-384", None, "control"),
]

# The lexical floor. Not an embedder: it has no vectors, so it has no map, no
# AMI and no map overlap, and those fields are null for it in the output. It
# answers the ranked and pairwise questions and nothing else.
LEXICAL_ARM = "bm25"


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


def load_model(model_name: str):
    """The three potion models and MiniLM, each through its own loader.

    Every `minishlab/` model here is Model2Vec format, which is the property that
    lets the size ladder and the retrieval arm run through one code path: no
    branch below distinguishes 4M from 32M from the retrieval fine-tune.
    """
    if model_name.startswith("minishlab/"):
        from model2vec import StaticModel

        return StaticModel.from_pretrained(model_name)
    from sentence_transformers import SentenceTransformer

    return SentenceTransformer(model_name)


def embed(key: str, model_name: str | None, model, texts: list[str], seed: int) -> np.ndarray:
    """Dispatch on the model NAME, not on the object.

    `StaticModel` and `SentenceTransformer` both expose `tokenizer` and both
    expose `encode`, so any hasattr probe that distinguishes them is one release
    away from picking the wrong branch silently.
    """
    if key == "random-384":
        rng = np.random.default_rng(seed)
        return rng.normal(size=(len(texts), 384)).astype(np.float32)
    if model_name is not None and model_name.startswith("minishlab/"):
        return model.encode(texts).astype(np.float32)
    return model.encode(texts, show_progress_bar=False).astype(np.float32)


def vocabulary(model) -> tuple[int, set[str]] | None:
    """(total vocabulary size, whole-word subset), or None for an arm with none.

    Both numbers are kept because they answer different questions. The total is
    what a model card quotes. The whole-word subset is what decides whether a
    word gets its own vector: everything else is reconstructed from word pieces
    and is whatever those pieces average to. The vocabulary reading of this
    vocabulary reading of the question above needs the second, so it has to be
    measured rather than inferred from the first.
    """
    tok = getattr(model, "tokenizer", None)
    if tok is None or not hasattr(tok, "get_vocab"):
        return None
    vocab = tok.get_vocab()
    return len(vocab), {t.lower() for t in vocab if not t.startswith("##")}


def vocab_coverage(entry: tuple[int, set[str]] | None, texts: list[str]) -> tuple[float | None, float | None]:
    """(type coverage, token coverage) of a corpus against a model's whole-word vocabulary."""
    if entry is None:
        return None, None
    vocab = entry[1]
    counts: dict[str, int] = {}
    for t in texts:
        for w in WORD.findall(t.lower()):
            counts[w] = counts.get(w, 0) + 1
    if not counts:
        return None, None
    types_in = sum(1 for w in counts if w in vocab)
    tokens_in = sum(c for w, c in counts.items() if w in vocab)
    return types_in / len(counts), tokens_in / sum(counts.values())


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


# --------------------------------------------------------------------------
# The ranked question: given one document, what are the top-k nearest, and are
# they the same class? Measured on the FULL-DIMENSION vectors, not on the map --
# the map's neighbourhoods are `map_overlap_with_minilm`, and conflating the two
# would answer a question about a picture with a number about retrieval.
# --------------------------------------------------------------------------


def ranked_from_ranking(ranked: list[list[int]], probe: np.ndarray, labels: np.ndarray, k: int) -> dict:
    """precision@k and MRR@k from an already-ranked candidate list per query.

    `ranked[i]` is the retrieved document ids for `probe[i]`, best first, self
    already removed. Shorter than k is not padded and not forgiven: the
    denominator stays k, so a lexical arm that matches nothing on a query is
    scored as having returned nothing, which is what it did.
    """
    precisions, rr = [], []
    for row, q in zip(ranked, probe):
        want = labels[q]
        hits = [1 if labels[d] == want else 0 for d in row[:k]]
        precisions.append(sum(hits) / k)
        first = next((r for r, h in enumerate(hits, 1) if h), 0)
        rr.append(1.0 / first if first else 0.0)
    return {
        "precision_at_k": float(np.mean(precisions)),
        "mrr_at_k": float(np.mean(rr)),
    }


def dense_ranking(vectors: np.ndarray, probe: np.ndarray, k: int) -> list[list[int]]:
    """Top-k by cosine over the whole corpus, self excluded."""
    norms = vectors / np.clip(np.linalg.norm(vectors, axis=1, keepdims=True), 1e-12, None)
    sims = norms[probe] @ norms.T
    sims[np.arange(len(probe)), probe] = -np.inf
    top = np.argpartition(-sims, kth=k, axis=1)[:, :k]
    out = []
    for r, row in enumerate(top):
        out.append([int(d) for d in row[np.argsort(-sims[r, row], kind="stable")]])
    return out


def _unit(vectors: np.ndarray) -> np.ndarray:
    return vectors / np.clip(np.linalg.norm(vectors, axis=1, keepdims=True), 1e-12, None)


def dense_pair_scores(
    left: np.ndarray, right: np.ndarray, pairs: list[tuple[int, int]]
) -> list[float]:
    """Cosine for each (left index, right index) pair.

    Two arrays rather than one because the near-duplicate arm scores a corpus row
    against a CORRUPTION, which lives in its own array. Passing the same array
    twice gives the same-class arm.
    """
    a = _unit(left)[[i for i, _ in pairs]]
    b = _unit(right)[[j for _, j in pairs]]
    return [float(x) for x in np.sum(a * b, axis=1)]


# --------------------------------------------------------------------------
# The lexical floor. DuckDB's `fts` extension, which is the BM25 an analyst
# already has in the database the rest of this product runs in.
# --------------------------------------------------------------------------


def bm25_scores(texts: list[str], queries: list[str], table: str) -> list[dict[int, float]]:
    """One dict of {doc id: BM25 score} per query, over an index of `texts`.

    Absent from the dict means the document shares no stemmed, non-stopword term
    with the query, which BM25 scores as no match. Callers treat absent as 0.0
    rather than dropping the pair, so an arm that retrieves nothing is scored
    rather than excused.
    """
    import duckdb

    con = duckdb.connect()
    con.execute("INSTALL fts; LOAD fts;")
    con.execute(f"CREATE TABLE {table}(id BIGINT, txt VARCHAR)")
    con.executemany(f"INSERT INTO {table} VALUES (?, ?)", list(enumerate(texts)))
    con.execute(f"PRAGMA create_fts_index('{table}', 'id', 'txt', overwrite=1)")
    sql = (
        f"SELECT id, fts_main_{table}.match_bm25(id, ?) AS score "
        f"FROM {table} WHERE score IS NOT NULL"
    )
    out: list[dict[int, float]] = []
    for q in queries:
        try:
            rows = con.execute(sql, [q]).fetchall()
        except Exception as exc:  # a query with no indexable term, or an fts parse refusal
            print(f"    bm25 query returned no result set: {type(exc).__name__}", file=sys.stderr)
            rows = []
        out.append({int(i): float(s) for i, s in rows})
    con.close()
    return out


def bm25_ranking(scored: list[dict[int, float]], probe: np.ndarray, k: int, tiebreak: np.ndarray) -> list[list[int]]:
    """Top-k per query from BM25 score dicts, self excluded.

    Ties are broken by a seeded permutation rather than by document id. BM25
    produces genuine ties -- every unmatched document is absent, and short
    identifiers collide on a single shared term -- and breaking those by id would
    let corpus order, which correlates with label order, decide the score.
    """
    out = []
    for row, q in zip(scored, probe):
        cand = [(s, tiebreak[d], d) for d, s in row.items() if d != int(q)]
        cand.sort(key=lambda t: (-t[0], t[1]))
        out.append([int(d) for _, _, d in cand[:k]])
    return out


# --------------------------------------------------------------------------
# The pairwise question: given exactly two texts and one GLOBAL threshold, are
# they the same class / near-duplicates? Pooled average precision. Per-query
# normalisation is deliberately not applied -- it would turn this back into the
# ranked question, which is the confusion this arm exists to avoid.
# --------------------------------------------------------------------------


def pooled_ap(scores: list[float], positive: list[int]) -> float:
    from sklearn.metrics import average_precision_score

    return float(average_precision_score(np.asarray(positive), np.asarray(scores)))


def same_class_pairs(probe: np.ndarray, labels: np.ndarray, rng) -> tuple[list[tuple[int, int]], list[int]]:
    """One same-class pair and one different-class pair per anchor.

    Fixing the positive rate at PAIRWISE_POSITIVE_RATE is what makes the pooled
    AP readable: an arm at 0.5 is at chance. An anchor whose class has no other
    member, or a corpus of one class, contributes nothing rather than a
    degenerate pair.
    """
    by_label: dict[int, list[int]] = {}
    for i, l in enumerate(labels):
        by_label.setdefault(int(l), []).append(i)
    all_labels = sorted(by_label)
    pairs: list[tuple[int, int]] = []
    positive: list[int] = []
    for a in probe:
        a = int(a)
        same = [i for i in by_label[int(labels[a])] if i != a]
        others = [l for l in all_labels if l != int(labels[a])]
        if not same or not others:
            continue
        pairs.append((a, same[int(rng.integers(len(same)))]))
        positive.append(1)
        pool = by_label[others[int(rng.integers(len(others)))]]
        pairs.append((a, pool[int(rng.integers(len(pool)))]))
        positive.append(0)
    return pairs, positive


def near_duplicate(text: str, rng) -> str:
    """A seeded near-duplicate of one text: the same thing, written differently.

    Two regimes, because the two shapes of near-duplicate in this product are not
    the same edit. A column name is an identifier and its duplicates are
    `customer_id` / `customerId` / `Customer Id` / `cust_id` / `cstmr_id`, which
    is a spelling change over a two-word string. A post is prose and its
    duplicates are quotations, reposts and excerpts, which is a deletion.
    """
    words = text.split()
    if len(words) <= 3:
        parts = [p for p in re.split(r"[_\-\s]+|(?<=[a-z0-9])(?=[A-Z])", text) if p]
        if not parts:
            return text
        longest = max(range(len(parts)), key=lambda i: len(parts[i]))
        start = int(rng.integers(4))
        for attempt in range(4):
            choice = (start + attempt) % 4
            p = list(parts)
            if choice == 0:
                out = p[0].lower() + "".join(w.capitalize() for w in p[1:])
            elif choice == 1:
                out = " ".join(w.capitalize() for w in p)
            elif choice == 2:
                p[longest] = p[longest][:4]
                out = "_".join(p)
            else:
                head = p[longest][:1]
                p[longest] = head + re.sub(r"[aeiouAEIOU]", "", p[longest][1:])
                out = "_".join(p)
            if out and out != text:
                return out
        return text
    keep = [w for w in words if rng.random() >= NEAR_DUP_DROP]
    return " ".join(keep) if keep else text


def near_duplicate_pairs(probe: np.ndarray, rng) -> tuple[list[tuple[int, int]], list[int]]:
    """One (anchor, own corruption) pair and one (anchor, other corruption) pair.

    Indices on the right refer to positions in the CORRUPTION list, not to corpus
    rows, so the negative partner is another anchor's corruption rather than
    another anchor. Both sides of the decision are then the same kind of string.
    """
    n = len(probe)
    pairs: list[tuple[int, int]] = []
    positive: list[int] = []
    for a in range(n):
        pairs.append((a, a))
        positive.append(1)
        other = int(rng.integers(n - 1)) if n > 1 else 0
        if other >= a:
            other += 1
        pairs.append((a, other))
        positive.append(0)
    return pairs, positive


def licences(models: list[str]) -> dict:
    """Redistribution terms, read live from each model's Hugging Face card.

    A positive result makes bundling the obvious next question and a bundled
    model is a redistribution, so this is recorded in the same pass as the
    numbers that would motivate it. The output says whether each value was read
    or fell back, because a pinned string in this file is a claim nothing checks.
    """
    out = {}
    for name in models:
        url = f"https://huggingface.co/api/models/{name}"
        try:
            with urllib.request.urlopen(url, timeout=30) as fh:
                meta = json.load(fh)
            card = meta.get("cardData") or {}
            value = card.get("license") or card.get("license_name")
            out[name] = {
                "licence": value,
                "source": "huggingface model card metadata",
                "base_model": card.get("base_model"),
                "read_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
            }
        except (urllib.error.URLError, TimeoutError, ValueError, OSError) as exc:
            out[name] = {
                "licence": LICENCE_FALLBACK.get(name),
                "source": f"fallback in map_fidelity.py; live read failed: {type(exc).__name__}",
                "base_model": None,
                "read_at": None,
            }
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, required=True, help="results JSON to write")
    ap.add_argument("--limit", type=int, default=3000, help="max rows per corpus")
    ap.add_argument("--seed", type=int, default=SEED)
    ap.add_argument("--corpus", action="append", help="restrict to these corpus keys")
    ap.add_argument("--probe-rows", type=int, default=PROBE_ROWS,
                    help="retrieval queries and pairwise anchors, sampled once and shared")
    args = ap.parse_args()

    repo_root = Path(__file__).resolve().parents[2]
    corpora = [c for c in CORPORA if not args.corpus or c.key in args.corpus]

    # Loaded once, outside the corpus loop, so every corpus is measured by the
    # same model objects in the same process. Reloading per corpus would be a
    # second index by another name.
    models = {}
    for key, name, _ in EMBEDDERS:
        if name is not None:
            print(f"loading {name}", file=sys.stderr)
            models[key] = load_model(name)
    vocabs = {k: vocabulary(m) for k, m in models.items()}
    fingerprints = {
        key: hashlib.sha256(
            np.round(embed(key, name, models.get(key), FINGERPRINT_TEXTS, args.seed), 4).tobytes()
        ).hexdigest()[:16]
        for key, name, _ in EMBEDDERS
    }
    for key, digest in fingerprints.items():
        print(f"  {key} fingerprint {digest}", file=sys.stderr)

    results: list[dict] = []
    for corpus in corpora:
        texts, labels = load_corpus(corpus, args.limit, repo_root, args.seed)
        n = len(texts)
        rng = np.random.default_rng(args.seed)
        probe = np.sort(rng.choice(n, size=min(args.probe_rows, n), replace=False))
        tiebreak = rng.permutation(n)
        k = min(RETRIEVAL_K, n - 1)
        print(f"[{corpus.key}] {n} rows, {len(np.unique(labels))} classes, {len(probe)} probes", file=sys.stderr)

        # Every pair set is built ONCE, from the shared probe sample, and handed
        # to every arm. Building them per arm would give each arm its own index.
        sc_pairs, sc_positive = same_class_pairs(probe, labels, np.random.default_rng(args.seed + 1))
        dup_rng = np.random.default_rng(args.seed + 2)
        corruptions = [near_duplicate(texts[int(i)], dup_rng) for i in probe]
        unchanged = sum(1 for i, c in zip(probe, corruptions) if c == texts[int(i)])
        nd_pairs, nd_positive = near_duplicate_pairs(probe, np.random.default_rng(args.seed + 3))

        # The near-duplicate pairs index the probe sample on the left and the
        # corruption list on the right, so translate the left side to corpus rows
        # once here rather than inside each arm.
        nd_pairs_rows = [(int(probe[i]), j) for i, j in nd_pairs]

        maps: dict[str, np.ndarray] = {}
        # The full-dimension vectors kept alongside the maps, so the SAME overlap
        # metric can be taken at both stages. Without this only `maps` survives the
        # arm loop, and the only full-dimension number available for comparison is
        # `ami_vectors` — which answers a different question, and comparing the two
        # is what made the first draft blame the projection.
        vecs: dict[str, np.ndarray] = {}
        rows: dict[str, dict] = {}
        for key, model_name, kind in EMBEDDERS:
            vectors = embed(key, model_name, models.get(key), texts, args.seed)
            # A different seed for the control's corruption vectors: with the
            # same one, `random-384` would score a corruption against noise drawn
            # from the same stream, and a positive pair that collided on an index
            # would earn a cosine of exactly 1. The control has to be independent
            # of the thing it is a control for.
            dup_vectors = embed(key, model_name, models.get(key), corruptions, args.seed + 4)
            points = project(vectors, args.seed)
            maps[key] = points
            vecs[key] = vectors
            v_types, v_tokens = vocab_coverage(vocabs.get(key), texts)
            entry = vocabs.get(key)
            row = {
                "embedder": key,
                "kind": kind,
                "model": model_name,
                "dim": int(vectors.shape[1]),
                "fingerprint": fingerprints[key],
                "vocab_size": None if entry is None else entry[0],
                "vocab_whole_word_size": None if entry is None else len(entry[1]),
                "vocab_type_coverage": v_types,
                "vocab_token_coverage": v_tokens,
                "ami_map": cluster_agreement(points, labels, args.seed),
                "ami_vectors": cluster_agreement(vectors, labels, args.seed),
            }
            row.update(ranked_from_ranking(dense_ranking(vectors, probe, k), probe, labels, k))
            row["pairwise_ap_same_class"] = pooled_ap(
                dense_pair_scores(vectors, vectors, sc_pairs), sc_positive
            )
            row["pairwise_ap_near_duplicate"] = pooled_ap(
                dense_pair_scores(vectors, dup_vectors, nd_pairs_rows), nd_positive
            )
            rows[key] = row
            print(
                f"  {key}: AMI(map)={row['ami_map']:.4f} P@{k}={row['precision_at_k']:.4f} "
                f"AP(class)={row['pairwise_ap_same_class']:.4f} AP(dup)={row['pairwise_ap_near_duplicate']:.4f}",
                file=sys.stderr,
            )

        # The lexical floor, over the same corpus, the same probes and the same
        # pairs. Two indexes: one over the corpus, which answers the ranked
        # question and the same-class pairs, and one over the corruptions, which
        # is the only place a near-duplicate can be looked up.
        corpus_scored = bm25_scores(texts, [texts[int(i)] for i in probe], "corpus_docs")
        dup_scored = bm25_scores(corruptions, [texts[int(i)] for i in probe], "dup_docs")
        pos_of = {int(p): r for r, p in enumerate(probe)}
        lex = {
            "embedder": LEXICAL_ARM,
            "kind": "lexical",
            "model": "duckdb fts match_bm25 (porter stemmer, english stopwords)",
            "fingerprint": None,
            "dim": None,
            "vocab_size": None,
            "vocab_type_coverage": None,
            "vocab_token_coverage": None,
            "ami_map": None,
            "ami_vectors": None,
        }
        lex.update(ranked_from_ranking(bm25_ranking(corpus_scored, probe, k, tiebreak), probe, labels, k))
        lex["pairwise_ap_same_class"] = pooled_ap(
            [corpus_scored[pos_of[i]].get(j, 0.0) for i, j in sc_pairs], sc_positive
        )
        lex["pairwise_ap_near_duplicate"] = pooled_ap(
            [dup_scored[i].get(j, 0.0) for i, j in nd_pairs], nd_positive
        )
        lex["vocab_whole_word_size"] = None
        # BM25 scores are not calibrated across queries, so a POOLED average
        # precision over pairs from different queries reads a real ordering plus
        # a per-query offset. Recorded rather than hidden: the ranked columns
        # above are unaffected, because each is a ranking within one query.
        lex["pairwise_score_calibration"] = "uncalibrated across queries"
        rows[LEXICAL_ARM] = lex
        print(
            f"  {LEXICAL_ARM}: P@{k}={lex['precision_at_k']:.4f} "
            f"AP(class)={lex['pairwise_ap_same_class']:.4f} AP(dup)={lex['pairwise_ap_near_duplicate']:.4f}",
            file=sys.stderr,
        )

        floor = rows["random-384"]["ami_map"]
        ceiling = rows["minilm"]["ami_map"]
        p_floor = rows["random-384"]["precision_at_k"]
        p_ceiling = rows["minilm"]["precision_at_k"]
        for key, row in rows.items():
            # Retention against the noise floor, not against zero: on an easy
            # corpus a raw ratio makes a weak embedder look close to a good one.
            span = ceiling - floor
            if row["ami_map"] is None:
                row["retention_vs_minilm"] = None
                row["map_overlap_with_minilm"] = None
                row["vector_overlap_with_minilm"] = None
            else:
                row["retention_vs_minilm"] = None if span <= 0 else (row["ami_map"] - floor) / span
                row["map_overlap_with_minilm"] = map_overlap(maps[key], maps["minilm"], AGREEMENT_K)
                # The SAME metric one stage earlier, on the raw vectors. Without it a
                # reader can only compare neighbour-agreement-with-MiniLM (2D) against
                # label-agreement (full dimension), which are different QUESTIONS as
                # well as different stages — so any gap between them says nothing about
                # which stage caused it. This column is what makes that attributable,
                # and it is why the first draft of FINDINGS.md blamed the projection.
                row["vector_overlap_with_minilm"] = map_overlap(
                    vecs[key], vecs["minilm"], AGREEMENT_K
                )
            # precision@k does NOT floor at zero -- a uniform random retriever
            # scores the class prior -- so the comparable number is normalised
            # against the measured control the same way retention is.
            p_span = p_ceiling - p_floor
            row["lift_over_random"] = None if p_span <= 0 else (row["precision_at_k"] - p_floor) / p_span

        results.append({
            "corpus": corpus.key,
            "shape": corpus.shape,
            "description": corpus.description,
            "rows": n,
            "classes": int(len(np.unique(labels))),
            "probe_rows": len(probe),
            "retrieval_k": k,
            "pairwise_pairs_same_class": len(sc_pairs),
            "pairwise_pairs_near_duplicate": len(nd_pairs),
            "near_duplicate_unchanged": unchanged,
            "embedders": list(rows.values()),
        })

    payload = {
        "seed": args.seed,
        "limit": args.limit,
        "umap": {"metric": UMAP_METRIC, "n_neighbors": UMAP_N_NEIGHBORS, "random_state": args.seed},
        "agreement_k": AGREEMENT_K,
        "retrieval_k": RETRIEVAL_K,
        "probe_rows": args.probe_rows,
        "pairwise_positive_rate": PAIRWISE_POSITIVE_RATE,
        "near_duplicate_drop": NEAR_DUP_DROP,
        "platform": f"{platform.system()} {platform.machine()} python{platform.python_version()}",
        "licences": licences([n for _, n, _ in EMBEDDERS if n is not None]),
        "corpora": [asdict(c) for c in corpora],
        "results": results,
    }
    args.out.write_text(json.dumps(payload, indent=2) + "\n")
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
