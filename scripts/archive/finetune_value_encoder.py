#!/usr/bin/env python3
"""Fine-tune gte-small into a type-aware VALUE encoder for the multi-branch embed
branch (spec 2026-06-20-gte-tiny-embed-branch-swap, the release swing).

Why value-level: the embed branch encodes INDIVIDUAL values and 4-stat-aggregates
them, so the encoder must be adapted at the same granularity it is used. We explode
the distilled columns into (value -> column's type label) pairs from the RAW
distilled corpus (cs_train_distilled.tsv comma-joins values lossily) and fine-tune
gte-small's top layers to separate types in the SAME representation we extract:
mean-pool tokens -> L2-normalise -> classify. The classification head is discarded;
only the encoder is saved.

Leakage firewall: the gold/representative eval is scored after this. We apply the
SAME row-hash firewall the feature build uses (filter_eval_leakage) so eval columns'
values never train the encoder.

Output: output/gte-tiny-embed-swap/gte_small_ft.pt  (encoder state_dict + meta)

Usage:
  python3 scripts/finetune_value_encoder.py --smoke      # 1 epoch, tiny subset
  python3 scripts/finetune_value_encoder.py --epochs 6   # full fine-tune
"""
import argparse
import os
import sys
import time

_SCRIPTS = os.path.dirname(os.path.abspath(__file__))
if _SCRIPTS not in sys.path:
    sys.path.insert(0, _SCRIPTS)

import prepare_multibranch_data as P  # noqa: E402

BASE_MODEL = "thenlper/gte-small"  # 384-hidden (same as gte-tiny -> embed stays 1536)
DISTILLED = "output/distillation-v3/sherlock_distilled.csv.gz"
REMAP = "data/label_remap.json"
EVAL_HASHES = "eval/row_hashes.tsv"
OUT = "output/gte-tiny-embed-swap/gte_small_ft.pt"
PER_LABEL_CAP = 1500     # balance the long tail
VALUES_PER_COL = 32      # matches the extraction VALUE_CAP
MIN_VALUES = 5
FT_TOP_LAYERS = 2        # unfreeze the top N transformer layers
MAX_LEN = 64


def build_pairs(rng, smoke=False):
    """Explode leakage-filtered distilled columns into (value, label) pairs."""
    remap = P.load_label_remap(REMAP) if os.path.exists(REMAP) else None
    columns_by_type, _ordered, _stats = P.load_distilled_columns(
        DISTILLED, MIN_VALUES, label_remap=remap)
    # Same firewall as the feature build.
    if os.path.exists(EVAL_HASHES):
        eval_hashes = P.load_eval_row_hashes(EVAL_HASHES)
        before = sum(len(v) for v in columns_by_type.values())
        columns_by_type, _ = P.filter_eval_leakage(columns_by_type, eval_hashes, MIN_VALUES)
        after = sum(len(v) for v in columns_by_type.values())
        print(f"[data] leakage firewall: {before} -> {after} columns", flush=True)

    texts, labels = [], []
    per = {}
    types = sorted(columns_by_type.keys())
    if smoke:
        types = types[:12]
    for lab in types:
        cols = columns_by_type[lab]
        rng.shuffle(cols)
        for values, _header in cols:
            for v in values[:VALUES_PER_COL]:
                v = str(v).strip()
                if not v:
                    continue
                if per.get(lab, 0) >= (200 if smoke else PER_LABEL_CAP):
                    break
                texts.append(v)
                labels.append(lab)
                per[lab] = per.get(lab, 0) + 1
            if per.get(lab, 0) >= (200 if smoke else PER_LABEL_CAP):
                break
    return texts, labels


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--epochs", type=int, default=6)
    ap.add_argument("--smoke", action="store_true")
    ap.add_argument("--out", default=OUT)
    a = ap.parse_args()

    import numpy as np
    import torch
    import torch.nn as nn
    from transformers import AutoModel, AutoTokenizer, get_linear_schedule_with_warmup

    rng = __import__("random").Random(42)
    texts, labels_l = build_pairs(rng, smoke=a.smoke)
    labels = sorted(set(labels_l))
    l2i = {l: i for i, l in enumerate(labels)}
    y = torch.tensor([l2i[l] for l in labels_l])
    print(f"[ft] pairs={len(texts)} labels={len(labels)} epochs={a.epochs} "
          f"smoke={a.smoke}", flush=True)

    dev = "mps" if torch.backends.mps.is_available() else "cpu"
    tok = AutoTokenizer.from_pretrained(BASE_MODEL)
    enc = AutoModel.from_pretrained(BASE_MODEL).to(dev)
    H = enc.config.hidden_size
    nlayers = enc.config.num_hidden_layers
    head = nn.Linear(H, len(labels)).to(dev)

    # Freeze all, then unfreeze the top FT_TOP_LAYERS transformer layers.
    top = {f"encoder.layer.{nlayers - 1 - k}." for k in range(FT_TOP_LAYERS)}
    for n, p in enc.named_parameters():
        p.requires_grad = any(t in n for t in top)
    trainable = [p for p in enc.parameters() if p.requires_grad]
    print(f"[ft] gte-small H={H} layers={nlayers} unfrozen={sorted(top)} "
          f"device={dev}", flush=True)

    # eps=1e-6 (not the 1e-8 default): with this model/optimiser on MPS, 1e-8 makes
    # the loss diverge to NaN after the first step. The clean-slate prototype uses
    # 1e-6 for the same reason. Verified stable for gte-small + gte-tiny.
    opt = torch.optim.AdamW(
        [{"params": trainable, "lr": 2e-5},
         {"params": head.parameters(), "lr": 1e-3}], eps=1e-6, weight_decay=0.01)
    lossf = nn.CrossEntropyLoss()
    BS = 128
    steps = a.epochs * (len(texts) // BS + 1)
    sched = get_linear_schedule_with_warmup(opt, int(0.1 * steps), steps)

    def embed(batch_texts):
        e = tok(batch_texts, padding=True, truncation=True, max_length=MAX_LEN,
                return_tensors="pt").to(dev)
        out = enc(**e).last_hidden_state
        m = e["attention_mask"].unsqueeze(-1).float()
        pooled = (out * m).sum(1) / m.sum(1).clamp(min=1)
        return pooled / pooled.norm(dim=1, keepdim=True).clamp(min=1e-12)  # L2 (extraction-match)

    idx = np.arange(len(texts))
    for ep in range(a.epochs):
        enc.train(); head.train(); np.random.shuffle(idx)
        t0 = time.time(); tot = 0.0; nb = 0
        for b in range(0, len(idx), BS):
            bi = idx[b:b + BS]
            logits = head(embed([texts[i] for i in bi]))
            loss = lossf(logits, y[bi].to(dev))
            opt.zero_grad(); loss.backward()
            torch.nn.utils.clip_grad_norm_(trainable + list(head.parameters()), 1.0)
            opt.step(); sched.step()
            tot += loss.item(); nb += 1
        print(f"[ft] epoch {ep+1}/{a.epochs} ({time.time()-t0:.0f}s) "
              f"loss={tot/max(nb,1):.3f}", flush=True)

    os.makedirs(os.path.dirname(a.out), exist_ok=True)
    torch.save({"base_model": BASE_MODEL, "enc": enc.state_dict(),
                "hidden": H, "ft_top_layers": FT_TOP_LAYERS,
                "labels": labels}, a.out)
    print(f"[ft] saved encoder -> {a.out}", flush=True)


if __name__ == "__main__":
    main()
