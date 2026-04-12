"""
FineType autoresearch training script. Single-GPU, single-file.
Modify this file to experiment with architecture, optimizer, hyperparameters.
Usage: uv run train.py
"""

import math
import time

import torch
import torch.nn as nn
import torch.nn.functional as F

from prepare import (
    TIME_BUDGET,
    make_dataloader,
    evaluate_accuracy,
    prepare_data,
    CHAR_DIM,
    EMBED_DIM,
    STATS_DIM,
    HEADER_DIM,
)
import prepare as _prep

# ---------------------------------------------------------------------------
# Hyperparameters (edit these directly, no CLI flags needed)
# ---------------------------------------------------------------------------

LEARNING_RATE = 0.001
BATCH_SIZE = 64
DROPOUT = 0.35
WEIGHT_DECAY = 0.01
LR_SCHEDULE = "cosine"  # "cosine" or "constant"
WARMUP_EPOCHS = 2

# ---------------------------------------------------------------------------
# Multi-branch model — matches Candle MultiBranchConfig defaults
# ---------------------------------------------------------------------------
#
# Architecture from crates/finetype-train/src/multi_branch.rs:
#   Char:   Linear(960,300) -> ReLU -> Dropout -> Linear(300,300) -> ReLU -> Dropout
#   Embed:  Linear(512,200) -> ReLU -> Dropout -> Linear(200,200) -> ReLU -> Dropout
#   Stats:  Linear(27,128)  -> ReLU -> Dropout -> Linear(128,64)  -> ReLU -> Dropout
#   Header: LayerNorm(128)  -> Linear(128,128) -> ReLU -> Dropout -> Linear(128,64) -> ReLU -> Dropout
#   Merge:  concat[300+200+64+64=628] -> BatchNorm(628) -> Linear(628,500) -> ReLU -> Dropout -> Linear(500,500) -> ReLU -> Dropout
#   Head:   Linear(500, N_CLASSES)


class BranchMLP(nn.Module):
    """Two-layer MLP branch with GELU activation and dropout."""

    def __init__(self, input_dim, hidden1, hidden2, dropout, input_norm=False):
        super().__init__()
        self.input_norm = nn.LayerNorm(input_dim) if input_norm else None
        self.linear1 = nn.Linear(input_dim, hidden1)
        self.linear2 = nn.Linear(hidden1, hidden2)
        self.dropout = nn.Dropout(dropout)

    def forward(self, x):
        if self.input_norm is not None:
            x = self.input_norm(x)
        x = self.dropout(F.gelu(self.linear1(x)))
        x = self.dropout(F.gelu(self.linear2(x)))
        return x


class MultiBranchModel(nn.Module):
    """Multi-branch column classifier matching the Candle architecture."""

    def __init__(self, n_classes, dropout=DROPOUT):
        super().__init__()

        # Feature branches (header branch removed — inputs are all zeros)
        self.char_branch = BranchMLP(CHAR_DIM, 300, 300, dropout, input_norm=True)
        self.embed_branch = BranchMLP(EMBED_DIM, 200, 200, dropout, input_norm=True)
        self.stats_branch = BranchMLP(STATS_DIM, 128, 64, dropout, input_norm=True)

        # Merge trunk: 300 + 200 + 64 = 564
        merged_dim = 300 + 200 + 64  # 564
        self.merge_bn = nn.LayerNorm(merged_dim)
        self.merge_linear1 = nn.Linear(merged_dim, 500)
        self.merge_linear2 = nn.Linear(500, 500)
        self.merge_dropout = nn.Dropout(dropout)

        # Classification head
        self.head = nn.Linear(500, n_classes)

    def forward(self, x):
        # Split input into branches
        char_x = x[:, :CHAR_DIM]
        embed_x = x[:, CHAR_DIM:CHAR_DIM + EMBED_DIM]
        stats_x = x[:, CHAR_DIM + EMBED_DIM:CHAR_DIM + EMBED_DIM + STATS_DIM]
        header_x = x[:, CHAR_DIM + EMBED_DIM + STATS_DIM:]

        # Branch forward passes (no header branch — all zeros)
        char_out = self.char_branch(char_x)
        embed_out = self.embed_branch(embed_x)
        stats_out = self.stats_branch(stats_x)

        # Merge
        merged = torch.cat([char_out, embed_out, stats_out], dim=-1)
        merged = self.merge_bn(merged)
        merged = self.merge_dropout(F.gelu(self.merge_linear1(merged)))
        merged = self.merge_dropout(F.gelu(self.merge_linear2(merged)))

        # Head
        logits = self.head(merged)
        return logits


# ---------------------------------------------------------------------------
# LR schedule
# ---------------------------------------------------------------------------


def get_lr(step, total_steps, warmup_steps):
    """Get learning rate for current step."""
    if LR_SCHEDULE == "constant":
        if step < warmup_steps:
            return LEARNING_RATE * step / max(warmup_steps, 1)
        return LEARNING_RATE

    elif LR_SCHEDULE == "cosine":
        if step < warmup_steps:
            return LEARNING_RATE * step / max(warmup_steps, 1)
        progress = (step - warmup_steps) / max(total_steps - warmup_steps, 1)
        return LEARNING_RATE * 0.5 * (1 + math.cos(math.pi * progress))

    return LEARNING_RATE


# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

t_start = time.time()
torch.manual_seed(42)

# Device selection
if torch.cuda.is_available():
    device = torch.device("cuda")
    torch.cuda.manual_seed(42)
    print(f"Using CUDA: {torch.cuda.get_device_name()}")
elif hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
    device = torch.device("mps")
    print("Using MPS (Apple Silicon)")
else:
    device = torch.device("cpu")
    print("Using CPU")

# Prepare data (download, extract features, build datasets)
prepare_data()

# N_CLASSES is populated by prepare_data()
n_classes = _prep.N_CLASSES
print(f"n_classes: {n_classes}")

if n_classes == 0:
    print("FAIL: No classes found in data")
    exit(1)

# Build model
model = MultiBranchModel(n_classes=n_classes, dropout=DROPOUT).to(device)
num_params = sum(p.numel() for p in model.parameters())
print(f"Model parameters: {num_params:,} ({num_params / 1e6:.1f}M)")

# Optimizer
optimizer = torch.optim.AdamW(
    model.parameters(),
    lr=LEARNING_RATE,
    weight_decay=WEIGHT_DECAY,
)

# Dataloaders
train_loader = make_dataloader(split="train", batch_size=BATCH_SIZE)
val_loader = make_dataloader(split="val", batch_size=BATCH_SIZE)

# Estimate total steps for LR schedule
est_steps_per_epoch = len(train_loader)
est_epochs_in_budget = max(1, int(TIME_BUDGET / max(est_steps_per_epoch * 0.01, 1)))  # rough estimate
warmup_steps = WARMUP_EPOCHS * est_steps_per_epoch
total_steps_estimate = est_epochs_in_budget * est_steps_per_epoch

print(f"Batch size: {BATCH_SIZE}")
print(f"Steps per epoch: {est_steps_per_epoch}")
print(f"Warmup steps: {warmup_steps}")
print(f"LR schedule: {LR_SCHEDULE}")
print(f"Time budget: {TIME_BUDGET}s")

# ---------------------------------------------------------------------------
# Training loop — fixed wall-clock time budget
# ---------------------------------------------------------------------------

t_start_training = time.time()
model.train()

step = 0
epoch = 0
lr = LEARNING_RATE
best_val_acc = 0.0
best_val_loss = float("inf")
best_state_dict = None

while True:
    epoch += 1
    epoch_loss = 0.0
    epoch_correct = 0
    epoch_total = 0

    for features, labels in train_loader:
        # Check time budget
        elapsed = time.time() - t_start_training
        if elapsed >= TIME_BUDGET:
            break

        features = features.to(device)
        labels = labels.to(device)

        # LR schedule
        lr = get_lr(step, total_steps_estimate, warmup_steps)
        for param_group in optimizer.param_groups:
            param_group["lr"] = lr

        # Forward
        logits = model(features)
        loss = F.cross_entropy(logits, labels)

        # Backward
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        # Track metrics
        epoch_loss += loss.item()
        preds = logits.argmax(dim=-1)
        epoch_correct += (preds == labels).sum().item()
        epoch_total += labels.size(0)
        step += 1

        # Fast fail
        if math.isnan(loss.item()) or loss.item() > 100:
            print("FAIL: loss exploded")
            exit(1)

    # Check time budget
    elapsed = time.time() - t_start_training
    if elapsed >= TIME_BUDGET:
        break

    # Epoch summary
    train_acc = epoch_correct / epoch_total if epoch_total > 0 else 0
    train_loss = epoch_loss / max(1, epoch_total // BATCH_SIZE)
    remaining = max(0, TIME_BUDGET - elapsed)

    # Quick val check every epoch
    val_acc, val_loss = evaluate_accuracy(model, val_loader, device=device)
    model.train()  # Switch back to train mode

    if val_acc > best_val_acc:
        best_val_acc = val_acc
        best_val_loss = val_loss
        import copy
        best_state_dict = copy.deepcopy(model.state_dict())

    print(f"epoch {epoch:3d} | train_loss: {train_loss:.4f} | train_acc: {train_acc:.4f} | "
          f"val_loss: {val_loss:.4f} | val_acc: {val_acc:.4f} | "
          f"lr: {lr:.6f} | remaining: {remaining:.0f}s")

# ---------------------------------------------------------------------------
# Final evaluation
# ---------------------------------------------------------------------------

# Restore best model weights if we found an improvement
if best_state_dict is not None:
    model.load_state_dict(best_state_dict)
    print(f"Restored best model (val_acc={best_val_acc:.6f})")

model.eval()
val_acc, val_loss = evaluate_accuracy(model, val_loader, device=device)

t_end = time.time()
training_seconds = time.time() - t_start_training
total_seconds = t_end - t_start

# Peak VRAM
if torch.cuda.is_available():
    peak_vram_mb = torch.cuda.max_memory_allocated() / 1024 / 1024
else:
    peak_vram_mb = 0.0

# ---------------------------------------------------------------------------
# Machine-readable summary
# ---------------------------------------------------------------------------

print("---")
print(f"val_accuracy:     {val_acc:.6f}")
print(f"val_loss:         {val_loss:.6f}")
print(f"training_seconds: {training_seconds:.1f}")
print(f"total_seconds:    {total_seconds:.1f}")
print(f"peak_vram_mb:     {peak_vram_mb:.1f}")
print(f"num_epochs:       {epoch}")
print(f"num_params_M:     {num_params / 1e6:.1f}")
print(f"n_classes:        {n_classes}")
