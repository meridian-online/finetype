#!/usr/bin/env python3
"""NNFT-150: Extended spike — embeddings + statistical features.

Tests whether combining embeddings with simple surface-level features
improves entity type separation.

Usage:
    python3 specs/entity-disambiguation/embedding_spike_extended.py
"""

import re
import time
from pathlib import Path

import duckdb
import numpy as np
from model2vec import StaticModel
from sklearn.metrics import classification_report, silhouette_score
from sklearn.model_selection import StratifiedShuffleSplit
from sklearn.neighbors import KNeighborsClassifier
from sklearn.preprocessing import LabelEncoder, StandardScaler
from sklearn.ensemble import RandomForestClassifier
from sklearn.linear_model import LogisticRegression

SOTAB_VALUES = Path.home() / "datasets/sotab/cta/validation/column_values.parquet"
MODEL_NAME = "minishlab/potion-base-4M"

LABEL_MAP = {
    "Person": "person",
    "Person/name": "person",
    "Place": "place",
    "addressLocality": "place",
    "Organization": "organization",
    "LocalBusiness/name": "organization",
    "MusicAlbum": "creative_work",
    "MusicRecording/name": "creative_work",
    "Event/name": "creative_work",
    "CreativeWork": "creative_work",
    "MusicArtistAT": "person",  # merge into person
}


def load_entity_columns():
    labels_sql = ", ".join(f"'{l}'" for l in LABEL_MAP)
    con = duckdb.connect()
    rows = con.execute(f"""
        SELECT table_name, col_index, gt_label, col_value
        FROM read_parquet('{SOTAB_VALUES}')
        WHERE gt_label IN ({labels_sql})
        ORDER BY table_name, col_index
    """).fetchall()

    columns = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = f"{table_name}___{col_index}"
        if key not in columns:
            columns[key] = {"category": LABEL_MAP[gt_label], "values": []}
        columns[key]["values"].append(col_value)

    print(f"Loaded {len(columns)} columns")
    for cat in sorted(set(c["category"] for c in columns.values())):
        n = sum(1 for c in columns.values() if c["category"] == cat)
        print(f"  {cat}: {n}")
    return columns


# ── Statistical features per column ─────────────────────────────────

ORG_SUFFIXES = re.compile(
    r"\b(Inc|LLC|Ltd|Corp|Co|Company|Group|Foundation|Association|"
    r"University|Institute|Hospital|Church|School|Bank|Restaurant|"
    r"Hotel|Salon|Clinic|Studios?|Records|Entertainment|GmbH|AG|SA|"
    r"Pty|Pvt|PLC|LLP)\b",
    re.IGNORECASE,
)

PERSON_PATTERNS = re.compile(
    r"\b(Mr|Mrs|Ms|Dr|Jr|Sr|III|II|IV)\b\.?", re.IGNORECASE
)


def compute_column_features(values: list[str]) -> np.ndarray:
    """Compute statistical features for a column of string values.

    Returns a feature vector capturing surface-level patterns that
    may distinguish entity types.
    """
    if not values:
        return np.zeros(20)

    lengths = [len(v) for v in values]
    word_counts = [len(v.split()) for v in values]
    n = len(values)

    # Length stats
    mean_len = np.mean(lengths)
    std_len = np.std(lengths)
    max_len = max(lengths)
    min_len = min(lengths)

    # Word count stats
    mean_words = np.mean(word_counts)
    std_words = np.std(word_counts)
    max_words = max(word_counts)

    # Character pattern features
    has_digits = sum(1 for v in values if any(c.isdigit() for c in v)) / n
    has_parens = sum(1 for v in values if "(" in v or ")" in v) / n
    has_ampersand = sum(1 for v in values if "&" in v) / n
    has_comma = sum(1 for v in values if "," in v) / n
    has_apostrophe = sum(1 for v in values if "'" in v or "'" in v) / n

    # Title case ratio (each word starts with uppercase)
    def is_title_case(s):
        words = s.split()
        if not words:
            return False
        return all(w[0].isupper() for w in words if w and w[0].isalpha())

    title_case_ratio = sum(1 for v in values if is_title_case(v)) / n

    # All caps ratio
    all_caps = sum(1 for v in values if v == v.upper() and v != v.lower()) / n

    # Organization suffix matches
    org_suffix_ratio = sum(1 for v in values if ORG_SUFFIXES.search(v)) / n

    # Person title matches
    person_title_ratio = sum(1 for v in values if PERSON_PATTERNS.search(v)) / n

    # Uniqueness (cardinality / n)
    uniqueness = len(set(values)) / n

    # Average number of uppercase-started words
    def count_cap_words(s):
        return sum(1 for w in s.split() if w and w[0].isupper())

    mean_cap_words = np.mean([count_cap_words(v) for v in values])

    # "The" prefix ratio (common in creative works)
    the_prefix = sum(1 for v in values if v.lower().startswith("the ")) / n

    return np.array([
        mean_len, std_len, max_len, min_len,
        mean_words, std_words, max_words,
        has_digits, has_parens, has_ampersand, has_comma, has_apostrophe,
        title_case_ratio, all_caps, org_suffix_ratio, person_title_ratio,
        uniqueness, mean_cap_words, the_prefix,
        mean_words / max(mean_len, 1),  # word density
    ], dtype=np.float32)


FEATURE_NAMES = [
    "mean_len", "std_len", "max_len", "min_len",
    "mean_words", "std_words", "max_words",
    "has_digits", "has_parens", "has_ampersand", "has_comma", "has_apostrophe",
    "title_case", "all_caps", "org_suffix", "person_title",
    "uniqueness", "mean_cap_words", "the_prefix", "word_density",
]


# ── Experiments ─────────────────────────────────────────────────────


def run_experiment(X, labels, le, name, k=11):
    """Run kNN + random forest + logistic regression."""
    print(f"\n── {name} (features: {X.shape[1]}) ──")

    sil = silhouette_score(X, labels, metric="cosine", sample_size=min(5000, len(labels)))
    print(f"Silhouette (cosine): {sil:.3f}")

    splitter = StratifiedShuffleSplit(n_splits=5, test_size=0.3, random_state=42)

    for clf_name, clf_factory in [
        (f"kNN (k={k})", lambda: KNeighborsClassifier(n_neighbors=k, metric="cosine")),
        ("Random Forest", lambda: RandomForestClassifier(n_estimators=100, random_state=42)),
        ("Logistic Regression", lambda: LogisticRegression(max_iter=1000, random_state=42)),
    ]:
        accs = []
        all_y_true, all_y_pred = [], []
        for train_idx, test_idx in splitter.split(X, labels):
            X_train, X_test = X[train_idx], X[test_idx]
            y_train, y_test = labels[train_idx], labels[test_idx]

            # Scale for LR and RF
            if clf_name != f"kNN (k={k})":
                scaler = StandardScaler()
                X_train = scaler.fit_transform(X_train)
                X_test = scaler.transform(X_test)

            clf = clf_factory()
            clf.fit(X_train, y_train)
            y_pred = clf.predict(X_test)
            accs.append((y_pred == y_test).mean())
            all_y_true.extend(y_test)
            all_y_pred.extend(y_pred)

        print(f"\n  {clf_name}: {np.mean(accs):.3f} ± {np.std(accs):.3f}")
        print(classification_report(all_y_true, all_y_pred,
                                    target_names=le.classes_, digits=3))

    return sil


def show_feature_importance(X_features, labels, le, feature_names):
    """Show Random Forest feature importances."""
    print("\n── Feature Importance (Random Forest) ──")
    rf = RandomForestClassifier(n_estimators=100, random_state=42)
    rf.fit(X_features, labels)
    importances = rf.feature_importances_
    for name, imp in sorted(zip(feature_names, importances), key=lambda x: -x[1]):
        bar = "█" * int(imp * 100)
        print(f"  {name:>16}: {imp:.3f} {bar}")


def main():
    print("=" * 70)
    print("NNFT-150: Extended Spike — Embeddings + Statistical Features")
    print("=" * 70)

    columns = load_entity_columns()

    # Load model and embed
    print(f"\nLoading Model2Vec ({MODEL_NAME})...")
    model = StaticModel.from_pretrained(MODEL_NAME)

    column_keys = list(columns.keys())
    le = LabelEncoder()
    labels = le.fit_transform([columns[k]["category"] for k in column_keys])

    # Embeddings
    print("Computing embeddings...")
    t0 = time.time()
    all_values = []
    column_slices = []
    for key in column_keys:
        vals = columns[key]["values"]
        start = len(all_values)
        all_values.extend(vals)
        column_slices.append((start, start + len(vals)))
    all_embs = model.encode(all_values, show_progress_bar=False)
    embed_dim = all_embs.shape[1]
    X_emb = np.zeros((len(column_keys), embed_dim), dtype=np.float32)
    for i, (start, end) in enumerate(column_slices):
        X_emb[i] = all_embs[start:end].mean(axis=0)
    print(f"Embeddings: {time.time() - t0:.1f}s")

    # Statistical features
    print("Computing statistical features...")
    t0 = time.time()
    X_feat = np.array([
        compute_column_features(columns[k]["values"]) for k in column_keys
    ])
    print(f"Features: {time.time() - t0:.1f}s, shape: {X_feat.shape}")

    # Combined
    X_combined = np.hstack([X_emb, X_feat])

    # Run experiments
    run_experiment(X_emb, labels, le, "Embeddings only")
    run_experiment(X_feat, labels, le, "Statistical features only")
    run_experiment(X_combined, labels, le, "Embeddings + features")

    # Feature importance
    show_feature_importance(X_feat, labels, le, FEATURE_NAMES)

    # Summary
    print("\n" + "=" * 70)
    print("COMPARISON SUMMARY")
    print("=" * 70)


if __name__ == "__main__":
    main()
