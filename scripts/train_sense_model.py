#!/usr/bin/env python3
"""NNFT-163: Train Sense model — column-level transformer for semantic routing.

Two architectures:
  A. Lightweight attention over Model2Vec embeddings (fast, minimal)
  B. Small transformer encoder over value embeddings (powerful, slower)

Both are multi-task: broad category (6 classes) + entity subtype (4 classes).
Column name is an input feature with 50% header dropout during training.

Usage:
    # Architecture A (default)
    python3 scripts/train_sense_model.py --arch A

    # Architecture B
    python3 scripts/train_sense_model.py --arch B

    # Both
    python3 scripts/train_sense_model.py --arch both

Requires: torch, model2vec, numpy, scikit-learn, safetensors
"""

import argparse
import json
import time
from pathlib import Path

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from model2vec import StaticModel
from safetensors.torch import save_file
from sklearn.metrics import classification_report, confusion_matrix
from torch.utils.data import DataLoader, Dataset

# ── Config ──────────────────────────────────────────────────────────

MODEL2VEC_NAME = "minishlab/potion-base-4M"
EMBED_DIM = 128  # potion-base-4M embedding dimension
N_BROAD = 6
N_ENTITY = 4
HEADER_DROPOUT = 0.5  # Probability of dropping the header during training
PAD_IDX = 0  # Padding index for value sequences


# ── Dataset ─────────────────────────────────────────────────────────


class SenseDataset(Dataset):
    """Column-level dataset for Sense model training.

    Pre-computes all embeddings as numpy arrays for memory efficiency,
    converts to tensors on __getitem__.
    """

    def __init__(
        self,
        jsonl_path: Path,
        model2vec: StaticModel,
        max_values: int = 50,
        header_dropout: float = 0.0,
    ):
        self.max_values = max_values
        self.header_dropout = header_dropout
        self.items = []

        with open(jsonl_path) as f:
            for line in f:
                item = json.loads(line)
                self.items.append(item)

        # Pre-compute all column embeddings as numpy arrays
        print(f"  Pre-computing embeddings for {len(self.items)} columns...")
        t0 = time.time()
        self._precompute_column_tensors(model2vec)
        print(f"  Done in {time.time() - t0:.1f}s")

    def _precompute_column_tensors(self, model2vec: StaticModel):
        """Pre-compute padded value embeddings and header embeddings per column."""
        N = len(self.items)
        M = self.max_values
        D = EMBED_DIM

        # Allocate output arrays
        self.value_embeds_np = np.zeros((N, M, D), dtype=np.float32)
        self.masks_np = np.zeros((N, M), dtype=np.bool_)
        self.header_embeds_np = np.zeros((N, D), dtype=np.float32)
        self.has_header_np = np.zeros(N, dtype=np.float32)
        self.broad_indices = np.zeros(N, dtype=np.int64)
        self.entity_indices = np.zeros(N, dtype=np.int64)

        # Process columns in batches for encoding efficiency
        BATCH = 500
        for batch_start in range(0, N, BATCH):
            batch_end = min(batch_start + BATCH, N)
            batch_items = self.items[batch_start:batch_end]

            # Collect all strings in this batch
            all_strings = []
            string_indices = []  # (col_offset, slot_type, slot_idx)

            for ci, item in enumerate(batch_items):
                col_idx = batch_start + ci
                values = item["values"][:M]
                n_vals = len(values)

                # Track value strings
                for vi, v in enumerate(values):
                    string_indices.append((col_idx, "value", vi))
                    all_strings.append(v)

                # Track header string
                header = item.get("header")
                if header:
                    string_indices.append((col_idx, "header", 0))
                    all_strings.append(header)
                    self.has_header_np[col_idx] = 1.0

                # Set mask
                self.masks_np[col_idx, :n_vals] = True

                # Set labels
                self.broad_indices[col_idx] = item["broad_category_idx"]
                self.entity_indices[col_idx] = item["entity_subtype_idx"]

            # Batch encode all strings
            if all_strings:
                embeddings = model2vec.encode(all_strings, show_progress_bar=False)

                # Distribute embeddings back to arrays
                for si, (col_idx, slot_type, slot_idx) in enumerate(string_indices):
                    if slot_type == "value":
                        self.value_embeds_np[col_idx, slot_idx] = embeddings[si]
                    else:
                        self.header_embeds_np[col_idx] = embeddings[si]

            if (batch_start // BATCH) % 10 == 0:
                print(f"    Processed {batch_end}/{N} columns...")

        # Free item values to save memory (keep only metadata)
        for item in self.items:
            del item["values"]

    def __len__(self):
        return len(self.items)

    def __getitem__(self, idx):
        value_embeds = torch.from_numpy(self.value_embeds_np[idx])  # [M, D]
        mask = torch.from_numpy(self.masks_np[idx])  # [M]
        header_embed = torch.from_numpy(self.header_embeds_np[idx].copy())  # [D]
        has_header = self.has_header_np[idx]

        # Header dropout during training
        if self.header_dropout > 0 and has_header > 0:
            if torch.rand(1).item() < self.header_dropout:
                header_embed = torch.zeros(EMBED_DIM, dtype=torch.float32)
                has_header = 0.0

        return {
            "value_embeds": value_embeds,
            "mask": mask,
            "header_embed": header_embed,
            "has_header": torch.tensor(has_header),
            "broad_idx": int(self.broad_indices[idx]),
            "entity_idx": int(self.entity_indices[idx]),
        }


# ── Architecture A: Attention over Model2Vec ────────────────────────


class SenseModelA(nn.Module):
    """Lightweight attention over Model2Vec value embeddings.

    Column name embedding serves as the attention query.
    When no header is available, uses a learned query vector.
    """

    def __init__(
        self,
        embed_dim: int = EMBED_DIM,
        hidden_dim: int = 256,
        n_heads: int = 4,
        n_broad: int = N_BROAD,
        n_entity: int = N_ENTITY,
        dropout: float = 0.1,
    ):
        super().__init__()
        self.embed_dim = embed_dim

        # Learned query for when no header is available
        self.default_query = nn.Parameter(torch.randn(1, 1, embed_dim) * 0.02)

        # Project header to query space
        self.header_proj = nn.Linear(embed_dim, embed_dim)

        # Multi-head cross-attention: header (query) attends to values (key/value)
        self.cross_attention = nn.MultiheadAttention(
            embed_dim=embed_dim,
            num_heads=n_heads,
            dropout=dropout,
            batch_first=True,
        )

        # Layer norm after attention
        self.norm = nn.LayerNorm(embed_dim)

        # Also compute simple statistics as auxiliary features
        # mean + std of value embeddings = 2 * embed_dim
        feature_dim = embed_dim + 2 * embed_dim  # attention_out + mean + std

        # Broad category classifier
        self.broad_head = nn.Sequential(
            nn.Linear(feature_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim // 2, n_broad),
        )

        # Entity subtype classifier
        self.entity_head = nn.Sequential(
            nn.Linear(feature_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, hidden_dim // 2),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim // 2, n_entity),
        )

    def forward(self, value_embeds, mask, header_embed, has_header):
        """
        Args:
            value_embeds: [B, N, D] - Model2Vec embeddings of sampled values
            mask: [B, N] - True for real values, False for padding
            header_embed: [B, D] - Model2Vec embedding of column name
            has_header: [B] - 1.0 if header is available, 0.0 otherwise
        Returns:
            broad_logits: [B, N_BROAD]
            entity_logits: [B, N_ENTITY]
        """
        B = value_embeds.shape[0]

        # Build query: header embedding when available, default query otherwise
        header_q = self.header_proj(header_embed).unsqueeze(1)  # [B, 1, D]
        default_q = self.default_query.expand(B, -1, -1)  # [B, 1, D]
        has_h = has_header.unsqueeze(-1).unsqueeze(-1)  # [B, 1, 1]
        query = has_h * header_q + (1 - has_h) * default_q  # [B, 1, D]

        # Cross-attention: query attends to value embeddings
        # key_padding_mask: True means IGNORE (opposite of our mask)
        key_padding_mask = ~mask  # [B, N]
        attn_out, _ = self.cross_attention(
            query, value_embeds, value_embeds,
            key_padding_mask=key_padding_mask,
        )  # [B, 1, D]
        attn_out = self.norm(attn_out.squeeze(1))  # [B, D]

        # Compute mean and std of value embeddings (masked)
        mask_f = mask.unsqueeze(-1).float()  # [B, N, 1]
        n_vals = mask_f.sum(dim=1).clamp(min=1)  # [B, 1]
        val_mean = (value_embeds * mask_f).sum(dim=1) / n_vals  # [B, D]
        val_sq = ((value_embeds - val_mean.unsqueeze(1)) ** 2 * mask_f).sum(dim=1) / n_vals
        val_std = val_sq.sqrt()  # [B, D]

        # Concatenate features
        features = torch.cat([attn_out, val_mean, val_std], dim=-1)  # [B, 3*D]

        # Classification heads
        broad_logits = self.broad_head(features)
        entity_logits = self.entity_head(features)

        return broad_logits, entity_logits


# ── Architecture B: Small Transformer Encoder ───────────────────────


class SenseModelB(nn.Module):
    """Small transformer encoder over value embeddings with column name.

    Prepends a [CLS] token and optional header token to the value sequence,
    then runs through a small transformer encoder.
    """

    def __init__(
        self,
        embed_dim: int = EMBED_DIM,
        hidden_dim: int = 256,
        n_heads: int = 4,
        n_layers: int = 2,
        n_broad: int = N_BROAD,
        n_entity: int = N_ENTITY,
        dropout: float = 0.1,
    ):
        super().__init__()
        self.embed_dim = embed_dim

        # Special tokens
        self.cls_token = nn.Parameter(torch.randn(1, 1, embed_dim) * 0.02)
        self.header_token_proj = nn.Linear(embed_dim, embed_dim)

        # Token type embeddings: 0=CLS, 1=header, 2=value
        self.token_type_embed = nn.Embedding(3, embed_dim)

        # Transformer encoder
        encoder_layer = nn.TransformerEncoderLayer(
            d_model=embed_dim,
            nhead=n_heads,
            dim_feedforward=hidden_dim,
            dropout=dropout,
            batch_first=True,
            activation="gelu",
        )
        self.encoder = nn.TransformerEncoder(
            encoder_layer, num_layers=n_layers
        )

        # Classification heads (from CLS token)
        self.broad_head = nn.Sequential(
            nn.Linear(embed_dim, hidden_dim),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, n_broad),
        )
        self.entity_head = nn.Sequential(
            nn.Linear(embed_dim, hidden_dim),
            nn.GELU(),
            nn.Dropout(dropout),
            nn.Linear(hidden_dim, n_entity),
        )

    def forward(self, value_embeds, mask, header_embed, has_header):
        """
        Args:
            value_embeds: [B, N, D]
            mask: [B, N]
            header_embed: [B, D]
            has_header: [B]
        Returns:
            broad_logits: [B, N_BROAD]
            entity_logits: [B, N_ENTITY]
        """
        B, N, D = value_embeds.shape

        # Build sequence: [CLS] [HEADER] value1 value2 ... valueN
        cls_tokens = self.cls_token.expand(B, -1, -1)  # [B, 1, D]
        header_tokens = self.header_token_proj(header_embed).unsqueeze(1)  # [B, 1, D]

        # Sequence: CLS + header + values
        seq = torch.cat([cls_tokens, header_tokens, value_embeds], dim=1)  # [B, 2+N, D]

        # Token type ids
        type_ids = torch.zeros(B, 2 + N, dtype=torch.long, device=seq.device)
        type_ids[:, 0] = 0  # CLS
        type_ids[:, 1] = 1  # header
        type_ids[:, 2:] = 2  # values
        seq = seq + self.token_type_embed(type_ids)

        # Attention mask: CLS always attends, header conditional, values from mask
        src_key_padding_mask = torch.zeros(B, 2 + N, dtype=torch.bool, device=seq.device)
        # Mask header token when no header available
        src_key_padding_mask[:, 1] = has_header < 0.5
        # Mask padding values
        src_key_padding_mask[:, 2:] = ~mask

        # Transformer encoder
        encoded = self.encoder(seq, src_key_padding_mask=src_key_padding_mask)

        # CLS token output
        cls_out = encoded[:, 0, :]  # [B, D]

        # Classification
        broad_logits = self.broad_head(cls_out)
        entity_logits = self.entity_head(cls_out)

        return broad_logits, entity_logits


# ── Training loop ───────────────────────────────────────────────────


def train_epoch(model, loader, optimizer, device, entity_weight=1.0):
    model.train()
    total_loss = 0
    broad_correct = 0
    entity_correct = 0
    entity_total = 0
    n_samples = 0

    for batch in loader:
        value_embeds = batch["value_embeds"].to(device)
        mask = batch["mask"].to(device)
        header_embed = batch["header_embed"].to(device)
        has_header = batch["has_header"].to(device)
        broad_idx = batch["broad_idx"].to(device)
        entity_idx = batch["entity_idx"].to(device)

        broad_logits, entity_logits = model(
            value_embeds, mask, header_embed, has_header
        )

        # Broad category loss (all samples)
        loss_broad = F.cross_entropy(broad_logits, broad_idx)

        # Entity subtype loss (only entity samples)
        entity_mask = entity_idx >= 0
        if entity_mask.any():
            loss_entity = F.cross_entropy(
                entity_logits[entity_mask], entity_idx[entity_mask]
            )
            entity_preds = entity_logits[entity_mask].argmax(dim=-1)
            entity_correct += (entity_preds == entity_idx[entity_mask]).sum().item()
            entity_total += entity_mask.sum().item()
        else:
            loss_entity = torch.tensor(0.0, device=device)

        loss = loss_broad + entity_weight * loss_entity

        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
        optimizer.step()

        total_loss += loss.item() * broad_idx.shape[0]
        broad_preds = broad_logits.argmax(dim=-1)
        broad_correct += (broad_preds == broad_idx).sum().item()
        n_samples += broad_idx.shape[0]

    return {
        "loss": total_loss / n_samples,
        "broad_acc": broad_correct / n_samples,
        "entity_acc": entity_correct / entity_total if entity_total > 0 else 0,
    }


@torch.no_grad()
def evaluate(model, loader, device, categories, entity_subtypes):
    model.eval()
    all_broad_preds = []
    all_broad_targets = []
    all_entity_preds = []
    all_entity_targets = []

    for batch in loader:
        value_embeds = batch["value_embeds"].to(device)
        mask = batch["mask"].to(device)
        header_embed = batch["header_embed"].to(device)
        has_header = batch["has_header"].to(device)
        broad_idx = batch["broad_idx"].to(device)
        entity_idx = batch["entity_idx"].to(device)

        broad_logits, entity_logits = model(
            value_embeds, mask, header_embed, has_header
        )

        all_broad_preds.extend(broad_logits.argmax(dim=-1).cpu().tolist())
        all_broad_targets.extend(broad_idx.cpu().tolist())

        entity_mask = entity_idx >= 0
        if entity_mask.any():
            all_entity_preds.extend(
                entity_logits[entity_mask].argmax(dim=-1).cpu().tolist()
            )
            all_entity_targets.extend(entity_idx[entity_mask].cpu().tolist())

    broad_acc = sum(
        p == t for p, t in zip(all_broad_preds, all_broad_targets)
    ) / len(all_broad_targets)

    entity_acc = (
        sum(p == t for p, t in zip(all_entity_preds, all_entity_targets))
        / len(all_entity_targets)
        if all_entity_targets
        else 0
    )

    return {
        "broad_acc": broad_acc,
        "entity_acc": entity_acc,
        "broad_preds": all_broad_preds,
        "broad_targets": all_broad_targets,
        "entity_preds": all_entity_preds,
        "entity_targets": all_entity_targets,
    }


def collate_fn(batch):
    """Custom collate function for SenseDataset."""
    return {
        "value_embeds": torch.stack([b["value_embeds"] for b in batch]),
        "mask": torch.stack([b["mask"] for b in batch]),
        "header_embed": torch.stack([b["header_embed"] for b in batch]),
        "has_header": torch.stack([b["has_header"] for b in batch]),
        "broad_idx": torch.tensor([b["broad_idx"] for b in batch], dtype=torch.long),
        "entity_idx": torch.tensor([b["entity_idx"] for b in batch], dtype=torch.long),
    }


def run_training(
    arch: str,
    data_dir: Path,
    output_dir: Path,
    epochs: int = 50,
    batch_size: int = 64,
    lr: float = 5e-4,
    patience: int = 10,
    max_values: int = 50,
    device: str = "cpu",
):
    """Train a Sense model and save results."""
    print(f"\n{'='*60}")
    print(f"  Training Architecture {arch}")
    print(f"{'='*60}")

    # Load Model2Vec
    print("Loading Model2Vec...")
    m2v = StaticModel.from_pretrained(MODEL2VEC_NAME)

    # Load datasets
    print("Loading training data...")
    train_ds = SenseDataset(
        data_dir / "train.jsonl", m2v,
        max_values=max_values, header_dropout=HEADER_DROPOUT,
    )
    print("Loading validation data...")
    val_ds = SenseDataset(
        data_dir / "val.jsonl", m2v,
        max_values=max_values, header_dropout=0.0,
    )

    train_loader = DataLoader(
        train_ds, batch_size=batch_size, shuffle=True,
        collate_fn=collate_fn, num_workers=0,
    )
    val_loader = DataLoader(
        val_ds, batch_size=batch_size, shuffle=False,
        collate_fn=collate_fn, num_workers=0,
    )

    # Load metadata
    with open(data_dir / "meta.json") as f:
        meta = json.load(f)
    categories = meta["broad_categories"]
    entity_subtypes = meta["entity_subtypes"]

    # Create model
    if arch == "A":
        model = SenseModelA(
            embed_dim=EMBED_DIM, hidden_dim=256, n_heads=4,
            dropout=0.1,
        )
    else:
        model = SenseModelB(
            embed_dim=EMBED_DIM, hidden_dim=256, n_heads=4,
            n_layers=2, dropout=0.1,
        )
    model = model.to(device)

    n_params = sum(p.numel() for p in model.parameters())
    print(f"Model parameters: {n_params:,}")

    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=0.01)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(
        optimizer, T_max=epochs, eta_min=lr * 0.01
    )

    # Training loop with early stopping
    best_val_broad = 0
    best_epoch = 0
    best_state = None

    for epoch in range(1, epochs + 1):
        t0 = time.time()
        train_metrics = train_epoch(model, train_loader, optimizer, device)
        val_metrics = evaluate(model, val_loader, device, categories, entity_subtypes)
        scheduler.step()

        elapsed = time.time() - t0
        print(
            f"Epoch {epoch:3d}/{epochs} ({elapsed:.1f}s) | "
            f"Loss: {train_metrics['loss']:.4f} | "
            f"Train broad: {train_metrics['broad_acc']:.3f} entity: {train_metrics['entity_acc']:.3f} | "
            f"Val broad: {val_metrics['broad_acc']:.3f} entity: {val_metrics['entity_acc']:.3f}"
        )

        if val_metrics["broad_acc"] > best_val_broad:
            best_val_broad = val_metrics["broad_acc"]
            best_epoch = epoch
            best_state = {k: v.cpu().clone() for k, v in model.state_dict().items()}

        if epoch - best_epoch >= patience:
            print(f"  Early stopping at epoch {epoch} (best: {best_epoch})")
            break

    # Restore best model
    model.load_state_dict(best_state)
    model = model.to(device)

    # Final evaluation
    print(f"\n--- Final evaluation (best epoch {best_epoch}) ---")
    final = evaluate(model, val_loader, device, categories, entity_subtypes)

    print(f"\nBroad category accuracy: {final['broad_acc']:.4f} ({final['broad_acc']*100:.1f}%)")
    print("\nBroad category report:")
    print(classification_report(
        final["broad_targets"], final["broad_preds"],
        target_names=categories, digits=3,
    ))
    print("Broad category confusion matrix:")
    cm = confusion_matrix(final["broad_targets"], final["broad_preds"])
    print(f"{'':12s}", end="")
    for cat in categories:
        print(f"{cat[:8]:>9s}", end="")
    print()
    for i, cat in enumerate(categories):
        print(f"{cat:12s}", end="")
        for j in range(len(categories)):
            print(f"{cm[i][j]:9d}", end="")
        print()

    if final["entity_targets"]:
        print(f"\nEntity subtype accuracy: {final['entity_acc']:.4f} ({final['entity_acc']*100:.1f}%)")
        print("\nEntity subtype report:")
        print(classification_report(
            final["entity_targets"], final["entity_preds"],
            target_names=entity_subtypes, digits=3,
        ))

    # Save model
    arch_dir = output_dir / f"arch_{arch.lower()}"
    arch_dir.mkdir(parents=True, exist_ok=True)

    save_file(best_state, arch_dir / "model.safetensors")

    # Save config
    config = {
        "architecture": arch,
        "embed_dim": EMBED_DIM,
        "hidden_dim": 256,
        "n_heads": 4,
        "n_layers": 2 if arch == "B" else 0,
        "n_broad": N_BROAD,
        "n_entity": N_ENTITY,
        "max_values": max_values,
        "header_dropout": HEADER_DROPOUT,
        "n_params": n_params,
        "best_epoch": best_epoch,
        "best_val_broad_acc": best_val_broad,
        "best_val_entity_acc": final["entity_acc"],
        "broad_categories": categories,
        "entity_subtypes": entity_subtypes,
    }
    with open(arch_dir / "config.json", "w") as f:
        json.dump(config, f, indent=2)

    # Save detailed results
    results = {
        "broad_acc": final["broad_acc"],
        "entity_acc": final["entity_acc"],
        "broad_report": classification_report(
            final["broad_targets"], final["broad_preds"],
            target_names=categories, digits=4, output_dict=True,
        ),
        "entity_report": (
            classification_report(
                final["entity_targets"], final["entity_preds"],
                target_names=entity_subtypes, digits=4, output_dict=True,
            )
            if final["entity_targets"]
            else None
        ),
        "broad_confusion": cm.tolist(),
    }
    with open(arch_dir / "results.json", "w") as f:
        json.dump(results, f, indent=2)

    print(f"\nSaved model to {arch_dir}/")
    return results


def benchmark_speed(model, m2v, device, max_values_list=[20, 50]):
    """Benchmark inference speed at different sample sizes."""
    print(f"\n{'='*60}")
    print("  Speed Benchmark")
    print(f"{'='*60}")

    model.eval()

    for max_values in max_values_list:
        # Generate dummy data
        dummy_values = ["test value " + str(i) for i in range(max_values)]
        dummy_embeds = m2v.encode(dummy_values, show_progress_bar=False)
        value_embeds = torch.tensor(dummy_embeds, dtype=torch.float32).unsqueeze(0).to(device)
        mask = torch.ones(1, max_values, dtype=torch.bool, device=device)
        header_embed = torch.randn(1, EMBED_DIM, device=device)
        has_header = torch.tensor([1.0], device=device)

        # Pad to max_values if needed
        if value_embeds.shape[1] < max_values:
            pad = torch.zeros(1, max_values - value_embeds.shape[1], EMBED_DIM, device=device)
            value_embeds = torch.cat([value_embeds, pad], dim=1)

        # Warmup
        for _ in range(10):
            with torch.no_grad():
                model(value_embeds, mask, header_embed, has_header)

        # Benchmark
        n_iters = 100
        t0 = time.time()
        for _ in range(n_iters):
            with torch.no_grad():
                model(value_embeds, mask, header_embed, has_header)
        elapsed = (time.time() - t0) / n_iters * 1000  # ms per inference

        # Also time the Model2Vec encoding
        t0 = time.time()
        for _ in range(n_iters):
            m2v.encode(dummy_values[:max_values], show_progress_bar=False)
        encode_ms = (time.time() - t0) / n_iters * 1000

        total_ms = elapsed + encode_ms
        print(
            f"  {max_values:3d} values: "
            f"encode {encode_ms:.1f}ms + model {elapsed:.1f}ms = {total_ms:.1f}ms total"
        )


def main():
    parser = argparse.ArgumentParser(description="Train Sense model")
    parser.add_argument(
        "--arch", choices=["A", "B", "both"], default="both",
        help="Architecture to train (default: both)",
    )
    parser.add_argument("--data", type=str, default="data/sense_spike")
    parser.add_argument("--output", type=str, default="models/sense_spike")
    parser.add_argument("--epochs", type=int, default=50)
    parser.add_argument("--batch-size", type=int, default=64)
    parser.add_argument("--lr", type=float, default=5e-4)
    parser.add_argument("--patience", type=int, default=10)
    parser.add_argument("--max-values", type=int, default=50)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    torch.manual_seed(args.seed)
    np.random.seed(args.seed)

    data_dir = Path(args.data)
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    print(f"Device: {device}")

    archs = ["A", "B"] if args.arch == "both" else [args.arch]

    results = {}
    for arch in archs:
        r = run_training(
            arch=arch,
            data_dir=data_dir,
            output_dir=output_dir,
            epochs=args.epochs,
            batch_size=args.batch_size,
            lr=args.lr,
            patience=args.patience,
            max_values=args.max_values,
            device=device,
        )
        results[arch] = r

    # Speed benchmark
    print("\nLoading Model2Vec for speed benchmark...")
    m2v = StaticModel.from_pretrained(MODEL2VEC_NAME)

    for arch in archs:
        arch_dir = output_dir / f"arch_{arch.lower()}"
        if arch == "A":
            model = SenseModelA()
        else:
            model = SenseModelB()
        # Load saved weights
        from safetensors.torch import load_file
        state = load_file(arch_dir / "model.safetensors")
        model.load_state_dict(state)
        model = model.to(device)
        print(f"\n--- Architecture {arch} ---")
        benchmark_speed(model, m2v, device)

    # Summary comparison
    if len(archs) > 1:
        print(f"\n{'='*60}")
        print("  Architecture Comparison")
        print(f"{'='*60}")
        print(f"{'Metric':<25s} {'Arch A':>10s} {'Arch B':>10s}")
        print("-" * 47)
        for metric in ["broad_acc", "entity_acc"]:
            a_val = results["A"][metric]
            b_val = results["B"][metric]
            print(f"{metric:<25s} {a_val:10.4f} {b_val:10.4f}")


if __name__ == "__main__":
    main()
