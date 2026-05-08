#!/usr/bin/env python3
"""NNFT-150: Entity type embedding separation spike.

Tests whether Model2Vec value embeddings, aggregated per column,
carry enough signal to separate entity types (Person vs Place vs
Organization vs Creative Work).

Usage:
    python3 specs/entity-disambiguation/embedding_spike.py

Requires: model2vec, duckdb, pandas, scikit-learn, numpy
"""

import json
import time
from pathlib import Path

import duckdb
import numpy as np
from model2vec import StaticModel
from sklearn.metrics import (
    classification_report,
    confusion_matrix,
    silhouette_score,
)
from sklearn.model_selection import StratifiedShuffleSplit
from sklearn.neighbors import KNeighborsClassifier
from sklearn.preprocessing import LabelEncoder

# ── Config ──────────────────────────────────────────────────────────

SOTAB_VALUES = Path.home() / "datasets/sotab/cta/validation/column_values.parquet"
MODEL_NAME = "minishlab/potion-base-4M"  # Same as FineType's Model2Vec
OUTPUT_DIR = Path(__file__).parent

# Map SOTAB GT labels to entity super-categories
LABEL_MAP = {
    # Person names
    "Person": "person",
    "Person/name": "person",
    # Place / geography names
    "Place": "place",
    "addressLocality": "place",
    # Organization names
    "Organization": "organization",
    "LocalBusiness/name": "organization",
    # Creative works (titles)
    "MusicAlbum": "creative_work",
    "MusicRecording/name": "creative_work",
    "Event/name": "creative_work",
    "CreativeWork": "creative_work",
    # Music artists — ambiguous, could be person or entity
    # Include as separate category to see where it falls
    "MusicArtistAT": "music_artist",
}

# ── Load data ───────────────────────────────────────────────────────


def load_entity_columns() -> dict[str, list]:
    """Load SOTAB columns grouped by entity category.

    Returns dict mapping column_key -> {category, values}
    """
    labels_list = list(LABEL_MAP.keys())
    labels_sql = ", ".join(f"'{l}'" for l in labels_list)

    con = duckdb.connect()
    rows = con.execute(f"""
        SELECT table_name, col_index, gt_label, col_value
        FROM read_parquet('{SOTAB_VALUES}')
        WHERE gt_label IN ({labels_sql})
        ORDER BY table_name, col_index
    """).fetchall()

    # Group by column
    columns: dict[str, dict] = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = f"{table_name}___{col_index}"
        if key not in columns:
            columns[key] = {
                "category": LABEL_MAP[gt_label],
                "gt_label": gt_label,
                "values": [],
            }
        columns[key]["values"].append(col_value)

    print(f"Loaded {len(columns)} entity columns from SOTAB")
    for cat in sorted(set(c["category"] for c in columns.values())):
        n = sum(1 for c in columns.values() if c["category"] == cat)
        print(f"  {cat}: {n} columns")

    return columns


# ── Embed ───────────────────────────────────────────────────────────


def embed_columns(
    columns: dict[str, dict], model: StaticModel
) -> tuple[np.ndarray, np.ndarray, list[str]]:
    """Embed all column values and aggregate per column.

    Returns (embeddings, labels, column_keys) where:
    - embeddings: (n_columns, embed_dim) mean of value embeddings
    - labels: (n_columns,) encoded category labels
    - column_keys: list of column identifiers
    """
    le = LabelEncoder()
    all_categories = [c["category"] for c in columns.values()]
    le.fit(all_categories)

    column_keys = list(columns.keys())
    labels = le.transform([columns[k]["category"] for k in column_keys])

    print(f"\nEmbedding {len(column_keys)} columns...")
    t0 = time.time()

    # Batch all values for efficiency, then slice back
    all_values = []
    column_slices = []  # (start, end) indices
    for key in column_keys:
        vals = columns[key]["values"]
        start = len(all_values)
        all_values.extend(vals)
        column_slices.append((start, start + len(vals)))

    # Single batch embed
    all_embeddings = model.encode(all_values, show_progress_bar=False)
    embed_dim = all_embeddings.shape[1]

    # Aggregate per column (mean)
    column_embeddings = np.zeros((len(column_keys), embed_dim), dtype=np.float32)
    for i, (start, end) in enumerate(column_slices):
        column_embeddings[i] = all_embeddings[start:end].mean(axis=0)

    elapsed = time.time() - t0
    print(f"Embedded {len(all_values)} values in {elapsed:.1f}s")
    print(f"Column embeddings shape: {column_embeddings.shape}")

    return column_embeddings, labels, le


# ── Analysis ────────────────────────────────────────────────────────


def compute_class_distances(embeddings, labels, le):
    """Compute pairwise class centroid distances and within-class spread."""
    classes = le.classes_
    n_classes = len(classes)
    centroids = np.zeros((n_classes, embeddings.shape[1]))
    spreads = np.zeros(n_classes)

    for i, cls in enumerate(classes):
        mask = labels == i
        cls_embs = embeddings[mask]
        centroids[i] = cls_embs.mean(axis=0)
        # Average distance from centroid
        dists = np.linalg.norm(cls_embs - centroids[i], axis=1)
        spreads[i] = dists.mean()

    # Pairwise centroid distances
    print("\n── Class Centroid Distances ──")
    print(f"{'':>15}", end="")
    for cls in classes:
        print(f"{cls:>15}", end="")
    print()
    for i, cls_i in enumerate(classes):
        print(f"{cls_i:>15}", end="")
        for j, _ in enumerate(classes):
            d = np.linalg.norm(centroids[i] - centroids[j])
            print(f"{d:>15.3f}", end="")
        print()

    print("\n── Within-Class Spread (avg dist from centroid) ──")
    for i, cls in enumerate(classes):
        n = (labels == i).sum()
        print(f"  {cls:>15}: {spreads[i]:.3f}  (n={n})")

    return centroids, spreads


def run_knn_experiment(embeddings, labels, le, k=5, test_size=0.3, n_splits=5):
    """Run stratified kNN classification experiment."""
    print(f"\n── kNN Classification (k={k}, {n_splits} splits, test={test_size}) ──")

    splitter = StratifiedShuffleSplit(
        n_splits=n_splits, test_size=test_size, random_state=42
    )
    accuracies = []
    all_y_true = []
    all_y_pred = []

    for train_idx, test_idx in splitter.split(embeddings, labels):
        X_train, X_test = embeddings[train_idx], embeddings[test_idx]
        y_train, y_test = labels[train_idx], labels[test_idx]

        knn = KNeighborsClassifier(n_neighbors=k, metric="cosine")
        knn.fit(X_train, y_train)
        y_pred = knn.predict(X_test)

        acc = (y_pred == y_test).mean()
        accuracies.append(acc)
        all_y_true.extend(y_test)
        all_y_pred.extend(y_pred)

    mean_acc = np.mean(accuracies)
    std_acc = np.std(accuracies)
    print(f"  Mean accuracy: {mean_acc:.3f} ± {std_acc:.3f}")

    # Classification report on pooled predictions
    print("\n── Classification Report (pooled across splits) ──")
    print(
        classification_report(
            all_y_true, all_y_pred, target_names=le.classes_, digits=3
        )
    )

    # Confusion matrix
    cm = confusion_matrix(all_y_true, all_y_pred)
    print("── Confusion Matrix ──")
    print(f"{'':>15}", end="")
    for cls in le.classes_:
        print(f"{cls:>15}", end="")
    print()
    for i, cls in enumerate(le.classes_):
        print(f"{cls:>15}", end="")
        for j in range(len(le.classes_)):
            print(f"{cm[i][j]:>15}", end="")
        print()

    return mean_acc, std_acc, accuracies


def run_knn_sweep(embeddings, labels, le, k_values=None, test_size=0.3):
    """Test multiple k values to find optimal."""
    if k_values is None:
        k_values = [1, 3, 5, 7, 11, 15, 21]

    print(f"\n── kNN Sweep (test={test_size}) ──")
    splitter = StratifiedShuffleSplit(n_splits=5, test_size=test_size, random_state=42)

    results = []
    for k in k_values:
        accs = []
        for train_idx, test_idx in splitter.split(embeddings, labels):
            knn = KNeighborsClassifier(n_neighbors=k, metric="cosine")
            knn.fit(embeddings[train_idx], labels[train_idx])
            acc = knn.score(embeddings[test_idx], labels[test_idx])
            accs.append(acc)
        mean_acc = np.mean(accs)
        results.append((k, mean_acc))
        print(f"  k={k:>3}: {mean_acc:.3f}")

    return results


def experiment_without_music_artist(columns, model, le_full):
    """Re-run with music_artist merged into person (since they're person names)."""
    print("\n" + "=" * 70)
    print("EXPERIMENT 2: Music artists merged into person category")
    print("=" * 70)

    # Remap
    remapped = {}
    for key, col in columns.items():
        cat = col["category"]
        if cat == "music_artist":
            cat = "person"
        remapped[key] = {**col, "category": cat}

    le2 = LabelEncoder()
    all_cats = [c["category"] for c in remapped.values()]
    le2.fit(all_cats)

    column_keys = list(remapped.keys())
    labels2 = le2.transform([remapped[k]["category"] for k in column_keys])

    # Reuse embeddings — same columns, same order
    # Need to re-embed since we can't guarantee order
    embeddings2, labels2, le2 = embed_columns(remapped, model)

    sil = silhouette_score(embeddings2, labels2, metric="cosine", sample_size=min(5000, len(labels2)))
    print(f"\nSilhouette score (cosine): {sil:.3f}")

    compute_class_distances(embeddings2, labels2, le2)
    run_knn_experiment(embeddings2, labels2, le2)
    run_knn_sweep(embeddings2, labels2, le2)

    return embeddings2, labels2, le2


def experiment_binary_person_vs_rest(columns, model):
    """Binary classification: person names vs everything else."""
    print("\n" + "=" * 70)
    print("EXPERIMENT 3: Binary — person vs non-person entities")
    print("=" * 70)

    binary = {}
    for key, col in columns.items():
        cat = col["category"]
        # music_artist could go either way — include as person
        if cat in ("person", "music_artist"):
            binary_cat = "person"
        else:
            binary_cat = "non_person_entity"
        binary[key] = {**col, "category": binary_cat}

    embeddings3, labels3, le3 = embed_columns(binary, model)

    sil = silhouette_score(embeddings3, labels3, metric="cosine", sample_size=min(5000, len(labels3)))
    print(f"\nSilhouette score (cosine): {sil:.3f}")

    compute_class_distances(embeddings3, labels3, le3)
    run_knn_experiment(embeddings3, labels3, le3)

    return embeddings3, labels3, le3


# ── Main ────────────────────────────────────────────────────────────


def main():
    print("=" * 70)
    print("NNFT-150: Entity Type Embedding Separation Spike")
    print("=" * 70)

    # Load data
    columns = load_entity_columns()

    # Load model
    print(f"\nLoading Model2Vec ({MODEL_NAME})...")
    t0 = time.time()
    model = StaticModel.from_pretrained(MODEL_NAME)
    print(f"Model loaded in {time.time() - t0:.1f}s")

    # ── Experiment 1: All 5 categories ──
    print("\n" + "=" * 70)
    print("EXPERIMENT 1: All 5 entity categories")
    print("=" * 70)

    embeddings, labels, le = embed_columns(columns, model)

    # Silhouette score
    sil = silhouette_score(
        embeddings, labels, metric="cosine", sample_size=min(5000, len(labels))
    )
    print(f"\nSilhouette score (cosine): {sil:.3f}")
    print("  (>0.5 = strong, 0.25-0.5 = moderate, <0.25 = weak)")

    # Class distances
    compute_class_distances(embeddings, labels, le)

    # kNN classification
    mean_acc, std_acc, _ = run_knn_experiment(embeddings, labels, le)

    # k sweep
    run_knn_sweep(embeddings, labels, le)

    # ── Experiment 2: Merge music_artist into person ──
    experiment_without_music_artist(columns, model, le)

    # ── Experiment 3: Binary person vs non-person ──
    experiment_binary_person_vs_rest(columns, model)

    # ── Summary ──
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"Columns analysed: {len(columns)}")
    print(f"Embedding dim: {embeddings.shape[1]}")
    print(f"Model: {MODEL_NAME}")
    print(f"Silhouette (5-class): {sil:.3f}")
    print(f"kNN accuracy (5-class, k=5): {mean_acc:.3f} ± {std_acc:.3f}")


if __name__ == "__main__":
    main()
