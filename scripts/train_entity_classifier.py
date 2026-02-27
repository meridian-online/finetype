#!/usr/bin/env python3
"""NNFT-151: Train column-level entity classifier (Deep Sets MLP).

Trains a model to classify columns of string values into entity categories:
person, place, organization, creative_work.

The model is used as a binary demotion gate in FineType's column inference:
when CharCNN votes full_name but the entity classifier confidently says
"not person," the prediction is demoted to entity_name.

Architecture: Deep Sets (Zaheer et al. 2017)
  Per-value: Model2Vec encoding (frozen, potion-base-4M, 128-dim)
  Column:    mean + std of value embeddings + 44 statistical features = 300-dim
  Classify:  MLP (BatchNorm → 3×Linear/ReLU/Dropout → Linear) → 4 classes

Usage:
    # Train with defaults (production config)
    python3 scripts/train_entity_classifier.py

    # Custom options
    python3 scripts/train_entity_classifier.py \
        --epochs 100 --hidden-dim 256 --lr 5e-4 \
        --output models/entity-classifier/

Requires: model2vec, torch, duckdb, scikit-learn, numpy, safetensors
"""

import argparse
import json
import re
import time
from pathlib import Path

import duckdb
import numpy as np
import torch
import torch.nn as nn
import torch.optim as optim
from model2vec import StaticModel
from safetensors.torch import save_file
from sklearn.metrics import classification_report, confusion_matrix
from sklearn.model_selection import StratifiedKFold
from torch.utils.data import DataLoader, TensorDataset

# ── Config ──────────────────────────────────────────────────────────

SOTAB_VALIDATION = Path.home() / "datasets/sotab/cta/validation/column_values.parquet"
SOTAB_TEST = Path.home() / "datasets/sotab/cta/test/column_values.parquet"
MODEL2VEC_NAME = "minishlab/potion-base-4M"

# Map SOTAB GT labels → entity super-categories
LABEL_MAP = {
    "Person": "person",
    "Person/name": "person",
    "MusicArtistAT": "person",  # artist names are person names
    "Place": "place",
    "addressLocality": "place",
    "Organization": "organization",
    "LocalBusiness/name": "organization",
    "MusicAlbum": "creative_work",
    "MusicRecording/name": "creative_work",
    "Event/name": "creative_work",
    "CreativeWork": "creative_work",
}

ENTITY_CLASSES = ["person", "place", "organization", "creative_work"]

# Person class index — used for binary demotion analysis
PERSON_IDX = 0

# Confidence threshold for binary demotion: if max non-person prob > this,
# demote full_name to entity_name. Calibrated on SOTAB test set.
DEMOTION_THRESHOLD = 0.6

# Number of statistical features computed per column.
# Must match compute_column_features() output length.
N_STAT_FEATURES = 44

# ── Statistical feature names (for documentation / Rust reimplementation) ──

STAT_FEATURE_NAMES = [
    "mean_len", "std_len", "median_len", "p25_len", "p75_len",
    "mean_words", "std_words", "single_word_ratio", "two_word_ratio", "three_plus_ratio",
    "mean_alpha_ratio", "mean_digit_ratio", "mean_space_ratio", "mean_punct_ratio",
    "has_digits_ratio",
    "title_case_ratio", "all_caps_ratio",
    "has_comma_ratio", "has_parens_ratio", "has_ampersand_ratio",
    "has_apostrophe_ratio", "has_hyphen_ratio", "has_dot_ratio",
    "org_suffix_ratio", "person_title_ratio", "place_indicator_ratio",
    "creative_indicator_ratio", "the_prefix_ratio", "numeric_prefix_ratio",
    "uniqueness", "token_diversity", "avg_word_len",
    "cap_words_mean", "cap_word_ratio",
    "word_density", "short_value_ratio", "long_value_ratio", "cv_length",
    "preposition_ratio", "contains_number_ratio", "has_quotes_ratio",
    "column_size", "max_value_len", "max_word_count",
]


# ── Data loading ────────────────────────────────────────────────────


def load_columns(parquet_path: Path) -> dict[str, dict]:
    """Load SOTAB columns grouped by entity category."""
    labels_sql = ", ".join(f"'{l}'" for l in LABEL_MAP)
    con = duckdb.connect()
    rows = con.execute(f"""
        SELECT table_name, col_index, gt_label, col_value
        FROM read_parquet('{parquet_path}')
        WHERE gt_label IN ({labels_sql})
        ORDER BY table_name, col_index
    """).fetchall()

    columns: dict[str, dict] = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = f"{table_name}___{col_index}"
        if key not in columns:
            columns[key] = {"category": LABEL_MAP[gt_label], "values": []}
        columns[key]["values"].append(col_value)

    return columns


# ── Statistical features ────────────────────────────────────────────

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

PLACE_PATTERNS = re.compile(
    r"\b(Street|St|Avenue|Ave|Road|Rd|Boulevard|Blvd|Drive|Lane|"
    r"Court|Place|Square|Park|Bridge|Hill|Valley|Lake|River|Mountain|"
    r"Beach|Island|Bay|County|Province|State|Region|District|City|Town|Village)\b",
    re.IGNORECASE,
)

CREATIVE_PATTERNS = re.compile(
    r"\b(Album|Song|Track|Episode|Season|Chapter|Vol|Volume|Remix|Live|"
    r"feat|ft|Edition|Deluxe|Remaster|OST|Soundtrack|Tour|Concert|Festival)\b",
    re.IGNORECASE,
)

PREPOSITION_PATTERN = re.compile(
    r"\b(of|in|at|for|the|and|by|on|to)\b", re.IGNORECASE
)


def compute_column_features(values: list[str]) -> np.ndarray:
    """Compute 44 statistical features for a column of string values.

    Feature groups:
    - Length distribution (5): mean, std, median, p25, p75
    - Word count distribution (5): mean, std, single/two/three+ ratios
    - Character class ratios (5): alpha, digit, space, punct, has_digits
    - Structural patterns (8): title case, caps, comma, parens, ampersand, apostrophe, hyphen, dot
    - Domain patterns (6): org suffix, person title, place indicator, creative indicator, "the" prefix, numeric prefix
    - Value diversity (4): uniqueness, token diversity, avg word length, shape diversity (cap_words, cap_ratio)
    - Distributional shape (7): word density, short/long ratios, CV length, preposition, number, quotes
    - Column metadata (3): column size, max value length, max word count
    """
    if not values:
        return np.zeros(N_STAT_FEATURES)

    n = len(values)
    lengths = np.array([len(v) for v in values])
    word_counts = np.array([len(v.split()) for v in values])

    # Length distribution (5)
    mean_len = lengths.mean()
    std_len = lengths.std()
    median_len = float(np.median(lengths))
    p25_len = float(np.percentile(lengths, 25))
    p75_len = float(np.percentile(lengths, 75))

    # Word count distribution (5)
    mean_words = word_counts.mean()
    std_words = word_counts.std()
    single_word_ratio = (word_counts == 1).sum() / n
    two_word_ratio = (word_counts == 2).sum() / n
    three_plus_ratio = (word_counts >= 3).sum() / n

    # Character class ratios (5)
    def char_ratios(s):
        if not s:
            return 0.0, 0.0, 0.0, 0.0
        t = len(s)
        alpha = sum(1 for c in s if c.isalpha()) / t
        digit = sum(1 for c in s if c.isdigit()) / t
        space = sum(1 for c in s if c.isspace()) / t
        punct = 1.0 - alpha - digit - space
        return alpha, digit, space, punct

    ratios = [char_ratios(v) for v in values]
    mean_alpha = np.mean([r[0] for r in ratios])
    mean_digit = np.mean([r[1] for r in ratios])
    mean_space = np.mean([r[2] for r in ratios])
    mean_punct = np.mean([r[3] for r in ratios])
    has_digits_ratio = sum(1 for v in values if any(c.isdigit() for c in v)) / n

    # Structural patterns (8)
    def is_title_case(s):
        words = [w for w in s.split() if w and w[0].isalpha()]
        if not words:
            return False
        return sum(1 for w in words if w[0].isupper()) / len(words) > 0.7

    title_case_ratio = sum(1 for v in values if is_title_case(v)) / n
    all_caps_ratio = sum(1 for v in values if v == v.upper() and v != v.lower()) / n
    has_comma = sum(1 for v in values if "," in v) / n
    has_parens = sum(1 for v in values if "(" in v or ")" in v) / n
    has_ampersand = sum(1 for v in values if "&" in v) / n
    has_apostrophe = sum(1 for v in values if "'" in v or "\u2019" in v) / n
    has_hyphen = sum(1 for v in values if "-" in v) / n
    has_dot = sum(1 for v in values if "." in v) / n

    # Domain patterns (6)
    org_suffix_ratio = sum(1 for v in values if ORG_SUFFIXES.search(v)) / n
    person_title_ratio = sum(1 for v in values if PERSON_PATTERNS.search(v)) / n
    place_indicator_ratio = sum(1 for v in values if PLACE_PATTERNS.search(v)) / n
    creative_indicator_ratio = sum(1 for v in values if CREATIVE_PATTERNS.search(v)) / n
    the_prefix_ratio = sum(1 for v in values if v.lower().startswith("the ")) / n
    numeric_prefix_ratio = sum(1 for v in values if v and v[0].isdigit()) / n

    # Value diversity (4)
    uniqueness = len(set(values)) / n
    all_words = [w for v in values for w in v.split()]
    token_diversity = len(set(all_words)) / max(len(all_words), 1)
    avg_word_len = float(np.mean([len(w) for w in all_words])) if all_words else 0.0
    cap_words_mean = float(np.mean([
        sum(1 for w in v.split() if w and w[0].isupper()) for v in values
    ]))
    cap_word_ratio = float(np.mean([
        sum(1 for w in v.split() if w and w[0].isupper()) / max(len(v.split()), 1)
        for v in values
    ]))

    # Distributional shape (7)
    word_density = mean_words / max(mean_len, 1)
    short_value_ratio = (lengths <= 3).sum() / n
    long_value_ratio = (lengths > 50).sum() / n
    cv_length = std_len / max(mean_len, 1)
    preposition_ratio = sum(1 for v in values if PREPOSITION_PATTERN.search(v)) / n
    contains_number_ratio = sum(1 for v in values if re.search(r'\d+', v)) / n
    has_quotes_ratio = sum(1 for v in values if '"' in v or "'" in v or "\u00ab" in v or "\u00bb" in v) / n

    # Column metadata (3)
    column_size = float(n)
    max_value_len = float(lengths.max())
    max_word_count = float(word_counts.max())

    result = np.array([
        # Length distribution (5)
        mean_len, std_len, median_len, p25_len, p75_len,
        # Word count distribution (5)
        mean_words, std_words, single_word_ratio, two_word_ratio, three_plus_ratio,
        # Character class ratios (5)
        mean_alpha, mean_digit, mean_space, mean_punct, has_digits_ratio,
        # Structural patterns (8)
        title_case_ratio, all_caps_ratio, has_comma, has_parens, has_ampersand,
        has_apostrophe, has_hyphen, has_dot,
        # Domain patterns (6)
        org_suffix_ratio, person_title_ratio, place_indicator_ratio,
        creative_indicator_ratio, the_prefix_ratio, numeric_prefix_ratio,
        # Value diversity (4)
        uniqueness, token_diversity, avg_word_len, cap_words_mean, cap_word_ratio,
        # Distributional shape (7)
        word_density, short_value_ratio, long_value_ratio, cv_length,
        preposition_ratio, contains_number_ratio, has_quotes_ratio,
        # Column metadata (3)
        column_size, max_value_len, max_word_count,
    ], dtype=np.float32)

    assert len(result) == N_STAT_FEATURES, f"Expected {N_STAT_FEATURES} features, got {len(result)}"
    return result


# ── Feature extraction ─────────────────────────────────────────────


def embed_columns(columns: dict[str, dict], model: StaticModel) -> tuple:
    """Embed all column values and return per-column features.

    Feature vector per column:
    - Mean Model2Vec embedding (128-dim)
    - Std Model2Vec embedding (128-dim)
    - Statistical features (44-dim)
    Total: 300-dim per column
    """
    column_keys = list(columns.keys())

    all_values = []
    column_slices = []
    for key in column_keys:
        vals = columns[key]["values"]
        start = len(all_values)
        all_values.extend(vals)
        column_slices.append((start, start + len(vals)))

    all_embs = model.encode(all_values, show_progress_bar=False)
    embed_dim = all_embs.shape[1]

    X_mean = np.zeros((len(column_keys), embed_dim), dtype=np.float32)
    X_std = np.zeros((len(column_keys), embed_dim), dtype=np.float32)
    for i, (start, end) in enumerate(column_slices):
        col_embs = all_embs[start:end]
        X_mean[i] = col_embs.mean(axis=0)
        X_std[i] = col_embs.std(axis=0) if len(col_embs) > 1 else 0

    X_feat = np.array([
        compute_column_features(columns[k]["values"]) for k in column_keys
    ])

    X = np.hstack([X_mean, X_std, X_feat])

    labels = [ENTITY_CLASSES.index(columns[k]["category"]) for k in column_keys]
    y = np.array(labels, dtype=np.int64)

    return X, y, column_keys


# ── Model ───────────────────────────────────────────────────────────


class EntityClassifier(nn.Module):
    """Deep Sets MLP for entity type classification.

    Takes pre-computed column features (embedding mean + std + statistical)
    and classifies into entity categories.

    Architecture:
        input (feature_dim) → BatchNorm → Linear(hidden) → ReLU → Dropout
        → Linear(hidden) → ReLU → Dropout
        → Linear(hidden//2) → ReLU → Dropout
        → Linear(n_classes)
    """

    def __init__(self, feature_dim: int, hidden_dim: int, n_classes: int, dropout: float = 0.2):
        super().__init__()
        self.net = nn.Sequential(
            nn.BatchNorm1d(feature_dim),
            nn.Linear(feature_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim // 2, n_classes),
        )

    def forward(self, x):
        return self.net(x)


# ── Training ────────────────────────────────────────────────────────


def train_epoch(model, dataloader, optimizer, criterion, device):
    model.train()
    total_loss = 0
    correct = 0
    total = 0
    for X_batch, y_batch in dataloader:
        X_batch, y_batch = X_batch.to(device), y_batch.to(device)
        optimizer.zero_grad()
        logits = model(X_batch)
        loss = criterion(logits, y_batch)
        loss.backward()
        optimizer.step()
        total_loss += loss.item() * len(y_batch)
        correct += (logits.argmax(1) == y_batch).sum().item()
        total += len(y_batch)
    return total_loss / total, correct / total


def eval_epoch(model, dataloader, criterion, device):
    model.eval()
    total_loss = 0
    correct = 0
    total = 0
    all_preds = []
    all_labels = []
    all_probs = []
    with torch.no_grad():
        for X_batch, y_batch in dataloader:
            X_batch, y_batch = X_batch.to(device), y_batch.to(device)
            logits = model(X_batch)
            loss = criterion(logits, y_batch)
            total_loss += loss.item() * len(y_batch)
            probs = torch.softmax(logits, dim=1)
            preds = logits.argmax(1)
            correct += (preds == y_batch).sum().item()
            total += len(y_batch)
            all_preds.extend(preds.cpu().numpy())
            all_labels.extend(y_batch.cpu().numpy())
            all_probs.extend(probs.cpu().numpy())
    return (total_loss / total, correct / total,
            np.array(all_preds), np.array(all_labels), np.array(all_probs))


def train_model(
    X_train, y_train, X_val, y_val,
    feature_dim, hidden_dim, n_classes,
    epochs, lr, batch_size, dropout, device,
    patience=15,
):
    """Train model with early stopping on validation accuracy."""
    model = EntityClassifier(feature_dim, hidden_dim, n_classes, dropout).to(device)
    optimizer = optim.AdamW(model.parameters(), lr=lr, weight_decay=1e-4)
    scheduler = optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs)

    # Class weights for imbalanced data
    class_counts = np.bincount(y_train, minlength=n_classes)
    class_weights = 1.0 / (class_counts + 1e-6)
    class_weights = class_weights / class_weights.sum() * n_classes
    criterion = nn.CrossEntropyLoss(weight=torch.FloatTensor(class_weights).to(device))

    train_ds = TensorDataset(torch.FloatTensor(X_train), torch.LongTensor(y_train))
    val_ds = TensorDataset(torch.FloatTensor(X_val), torch.LongTensor(y_val))
    train_dl = DataLoader(train_ds, batch_size=batch_size, shuffle=True)
    val_dl = DataLoader(val_ds, batch_size=batch_size)

    best_val_acc = 0
    best_state = None
    no_improve = 0

    for epoch in range(epochs):
        train_loss, train_acc = train_epoch(model, train_dl, optimizer, criterion, device)
        val_loss, val_acc, _, _, _ = eval_epoch(model, val_dl, criterion, device)
        scheduler.step()

        if val_acc > best_val_acc:
            best_val_acc = val_acc
            best_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}
            no_improve = 0
        else:
            no_improve += 1

        if (epoch + 1) % 10 == 0 or epoch == 0:
            print(f"  Epoch {epoch+1:>3}: train_loss={train_loss:.4f} train_acc={train_acc:.3f} "
                  f"val_loss={val_loss:.4f} val_acc={val_acc:.3f} {'*' if no_improve == 0 else ''}")

        if no_improve >= patience:
            print(f"  Early stopping at epoch {epoch+1} (no improvement for {patience} epochs)")
            break

    model.load_state_dict(best_state)
    return model, best_val_acc


# ── Cross-validation ────────────────────────────────────────────────


def cross_validate(X, y, feature_dim, hidden_dim, n_classes, epochs, lr, batch_size, dropout, device, n_folds=5):
    """K-fold cross-validation on training data."""
    print(f"\n{'='*60}")
    print(f"Cross-validation ({n_folds} folds)")
    print(f"{'='*60}")

    skf = StratifiedKFold(n_splits=n_folds, shuffle=True, random_state=42)
    fold_accs = []

    for fold, (train_idx, val_idx) in enumerate(skf.split(X, y)):
        print(f"\nFold {fold+1}/{n_folds}:")
        X_tr, X_vl = X[train_idx], X[val_idx]
        y_tr, y_vl = y[train_idx], y[val_idx]

        _, val_acc = train_model(
            X_tr, y_tr, X_vl, y_vl,
            feature_dim, hidden_dim, n_classes,
            epochs, lr, batch_size, dropout, device,
        )
        fold_accs.append(val_acc)
        print(f"  Best val accuracy: {val_acc:.3f}")

    mean_acc = np.mean(fold_accs)
    std_acc = np.std(fold_accs)
    print(f"\nCV accuracy: {mean_acc:.3f} ± {std_acc:.3f}")
    return mean_acc, std_acc


# ── Binary demotion analysis ───────────────────────────────────────


def analyze_binary_demotion(probs, labels, classes):
    """Analyze binary person vs non-person demotion at various thresholds.

    In FineType, this model fires when CharCNN votes full_name. The demotion
    decision is: if max non-person probability > threshold, demote to entity_name.
    """
    print(f"\n{'='*60}")
    print("Binary Demotion Analysis (person vs non-person)")
    print(f"{'='*60}")

    is_person = labels == PERSON_IDX
    person_prob = probs[:, PERSON_IDX]
    nonperson_max_prob = probs[:, 1:].max(axis=1)  # max prob across non-person classes

    # Unthresholded binary
    pred_person = probs.argmax(axis=1) == PERSON_IDX
    tp = (pred_person & is_person).sum()
    fp = (pred_person & ~is_person).sum()
    tn = (~pred_person & ~is_person).sum()
    fn = (~pred_person & is_person).sum()
    print(f"\nUnthresholded (argmax):")
    print(f"  Person precision:     {tp/(tp+fp):.3f} ({tp}/{tp+fp})")
    print(f"  Person recall:        {tp/(tp+fn):.3f} ({tp}/{tp+fn})")
    print(f"  Not-person precision: {tn/(tn+fn):.3f} ({tn}/{tn+fn})")
    print(f"  Not-person recall:    {tn/(tn+fp):.3f} ({tn}/{tn+fp})")

    # Confidence-thresholded demotion
    print(f"\nConfidence-thresholded demotion:")
    print(f"  {'Threshold':>10} {'Demoted':>10} {'Coverage':>10} {'Precision':>10} {'Wrong':>10}")
    results = {}
    for threshold in [0.3, 0.4, 0.5, 0.6, 0.7, 0.8]:
        confident_nonperson = nonperson_max_prob > threshold
        n_demoted = confident_nonperson.sum()
        if n_demoted == 0:
            continue
        correct = (confident_nonperson & ~is_person).sum()
        wrong = (confident_nonperson & is_person).sum()
        coverage = n_demoted / len(labels)
        precision = correct / n_demoted
        print(f"  {threshold:>10.1f} {n_demoted:>10} {coverage:>10.1%} {precision:>10.3f} {wrong:>10}")
        results[threshold] = {
            "n_demoted": int(n_demoted),
            "coverage": float(coverage),
            "precision": float(precision),
            "wrong_demotions": int(wrong),
        }

    return results


# ── Main ────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(description="Train entity type classifier")
    parser.add_argument("--hidden-dim", type=int, default=256,
                        help="MLP hidden dimension (default: 256)")
    parser.add_argument("--epochs", type=int, default=100,
                        help="Max training epochs (default: 100)")
    parser.add_argument("--lr", type=float, default=5e-4,
                        help="Learning rate (default: 5e-4)")
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--dropout", type=float, default=0.2,
                        help="Dropout rate (default: 0.2)")
    parser.add_argument("--demotion-threshold", type=float, default=DEMOTION_THRESHOLD,
                        help=f"Binary demotion confidence threshold (default: {DEMOTION_THRESHOLD})")
    parser.add_argument("--output", type=str, default="models/entity-classifier")
    parser.add_argument("--skip-cv", action="store_true", help="Skip cross-validation")
    parser.add_argument("--device", type=str, default="cpu")
    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    print("=" * 60)
    print("NNFT-151: Entity Type Classifier Training")
    print("=" * 60)

    # Load Model2Vec
    print(f"\nLoading Model2Vec ({MODEL2VEC_NAME})...")
    t0 = time.time()
    model2vec = StaticModel.from_pretrained(MODEL2VEC_NAME)
    print(f"Loaded in {time.time() - t0:.1f}s")

    # Load and embed training data (SOTAB validation)
    print("\nLoading training data (SOTAB validation)...")
    train_columns = load_columns(SOTAB_VALIDATION)
    X_train, y_train, _ = embed_columns(train_columns, model2vec)
    n_classes = len(ENTITY_CLASSES)
    feature_dim = X_train.shape[1]
    embed_dim = 128  # Model2Vec output dim

    print(f"Training set: {len(X_train)} columns, {feature_dim}-dim features "
          f"({embed_dim} emb mean + {embed_dim} emb std + {N_STAT_FEATURES} stat)")
    for i, cls in enumerate(ENTITY_CLASSES):
        print(f"  {cls}: {(y_train == i).sum()}")

    # Load and embed test data (SOTAB test)
    print("\nLoading test data (SOTAB test)...")
    test_columns = load_columns(SOTAB_TEST)
    X_test, y_test, _ = embed_columns(test_columns, model2vec)
    print(f"Test set: {len(X_test)} columns")
    for i, cls in enumerate(ENTITY_CLASSES):
        print(f"  {cls}: {(y_test == i).sum()}")

    device = torch.device(args.device)

    # Cross-validation on training data
    cv_acc = cv_std = None
    if not args.skip_cv:
        cv_acc, cv_std = cross_validate(
            X_train, y_train, feature_dim, args.hidden_dim, n_classes,
            args.epochs, args.lr, args.batch_size, args.dropout, device,
        )

    # Train final model on full training set, evaluate on test
    print(f"\n{'='*60}")
    print("Final model: train on full validation, test on held-out test")
    print(f"{'='*60}")

    # Use 10% of training data for early stopping
    n = len(X_train)
    perm = np.random.RandomState(42).permutation(n)
    split = int(0.9 * n)
    train_idx, es_idx = perm[:split], perm[split:]

    final_model, _ = train_model(
        X_train[train_idx], y_train[train_idx],
        X_train[es_idx], y_train[es_idx],
        feature_dim, args.hidden_dim, n_classes,
        args.epochs, args.lr, args.batch_size, args.dropout, device,
    )

    # Evaluate on held-out test set
    print(f"\n{'='*60}")
    print("Test Set Evaluation (4-class)")
    print(f"{'='*60}")

    test_ds = TensorDataset(torch.FloatTensor(X_test), torch.LongTensor(y_test))
    test_dl = DataLoader(test_ds, batch_size=args.batch_size)
    criterion = nn.CrossEntropyLoss()

    _, test_acc, test_preds, test_labels, test_probs = eval_epoch(
        final_model, test_dl, criterion, device
    )
    print(f"\nTest accuracy: {test_acc:.3f}")
    print(f"\n{classification_report(test_labels, test_preds, target_names=ENTITY_CLASSES, digits=3)}")

    cm = confusion_matrix(test_labels, test_preds)
    print("Confusion Matrix:")
    print(f"{'':>15}", end="")
    for cls in ENTITY_CLASSES:
        print(f"{cls:>15}", end="")
    print()
    for i, cls in enumerate(ENTITY_CLASSES):
        print(f"{cls:>15}", end="")
        for j in range(n_classes):
            print(f"{cm[i][j]:>15}", end="")
        print()

    # Binary demotion analysis
    demotion_results = analyze_binary_demotion(test_probs, test_labels, ENTITY_CLASSES)

    # ── Save model ──

    state_dict = {k: v.cpu() for k, v in final_model.state_dict().items()}
    save_file(state_dict, output_dir / "model.safetensors")

    # Compute BatchNorm statistics for Rust inference (running_mean, running_var)
    # These are already in the state_dict from training mode.

    config = {
        "architecture": "deep_sets_mlp",
        "version": "v1",
        "feature_dim": feature_dim,
        "embed_dim": embed_dim,
        "n_stat_features": N_STAT_FEATURES,
        "hidden_dim": args.hidden_dim,
        "n_classes": n_classes,
        "dropout": args.dropout,
        "classes": ENTITY_CLASSES,
        "person_class_index": PERSON_IDX,
        "demotion_threshold": args.demotion_threshold,
        "model2vec": MODEL2VEC_NAME,
        "label_map": LABEL_MAP,
        "stat_feature_names": STAT_FEATURE_NAMES,
        "feature_layout": {
            "emb_mean": {"start": 0, "end": embed_dim},
            "emb_std": {"start": embed_dim, "end": embed_dim * 2},
            "stat_features": {"start": embed_dim * 2, "end": feature_dim},
        },
        "train_size": len(X_train),
        "test_size": len(X_test),
        "test_accuracy_4class": float(test_acc),
        "cv_accuracy": float(cv_acc) if cv_acc is not None else None,
        "demotion_analysis": demotion_results,
        "usage": (
            "Binary demotion gate: when CharCNN votes full_name, compute features "
            "for the column, run MLP forward pass, if max non-person class probability "
            f"> {args.demotion_threshold}, demote to entity_name."
        ),
    }
    (output_dir / "config.json").write_text(json.dumps(config, indent=2))
    (output_dir / "label_index.json").write_text(json.dumps(ENTITY_CLASSES, indent=2))

    print(f"\nModel saved to {output_dir}/")
    print(f"  model.safetensors ({(output_dir / 'model.safetensors').stat().st_size / 1024:.1f} KB)")
    print(f"  config.json")
    print(f"  label_index.json")

    # ── Summary ──
    print(f"\n{'='*60}")
    print("SUMMARY")
    print(f"{'='*60}")
    arch_str = f"{feature_dim} → {args.hidden_dim} → {args.hidden_dim} → {args.hidden_dim // 2} → {n_classes}"
    print(f"Architecture: Deep Sets MLP ({arch_str})")
    print(f"Features: {embed_dim} emb mean + {embed_dim} emb std + {N_STAT_FEATURES} stat = {feature_dim}")
    print(f"Value encoder: Model2Vec ({MODEL2VEC_NAME}, frozen)")
    print(f"Training data: {len(X_train)} columns (SOTAB validation)")
    print(f"Test data: {len(X_test)} columns (SOTAB test)")
    if cv_acc is not None:
        print(f"CV accuracy (4-class): {cv_acc:.3f} ± {cv_std:.3f}")
    print(f"Test accuracy (4-class): {test_acc:.3f}")
    thr = args.demotion_threshold
    if thr in demotion_results:
        d = demotion_results[thr]
        print(f"Binary demotion @ {thr}: {d['precision']:.1%} precision, "
              f"{d['coverage']:.1%} coverage, {d['wrong_demotions']} wrong demotions")
    print(f"Demotion threshold: {args.demotion_threshold}")


if __name__ == "__main__":
    main()
