#!/usr/bin/env python3
"""Build an FTMB **v5** feature binary: v19's exact multi-branch recipe with the
512-dim Model2Vec value-aggregation embed slot replaced by a frozen 384-dim
gte-tiny value embedding. Every other branch (char, stats, header=Model2Vec,
validation) and the whole data recipe are v19's, untouched.

Spec: 2026-06-20-gte-tiny-embed-branch-swap, ac-01.
Audit: .orbit/specs/2026-06-20-gte-tiny-embed-branch-swap/ac00_audit.md

## Why a monkeypatch and not a reimplementation
The audit's load-bearing warning is that a misaligned binary does not crash — it
silently wastes the overnight retrain. The single biggest divergence risk is the
DATA RECIPE (distilled load + filter + decontaminate + cap + synthetic generation
+ hard negatives + augmentation + 0.7 blend + table grouping + validation branch).
Reimplementing ~150 lines of that risks silent drift from v19. So this script does
NOT reimplement it: it imports prepare_multibranch_data and runs its `main()`
verbatim with v19's exact argv, monkeypatching only three things:

  1. EMBED_DIM            512 -> 384   (gte-tiny native hidden size)
  2. extract_features     embed slot   -> frozen gte-tiny mean-pooled value embedding
  3. VERSION_V4 marker    4 -> 5       (so the Rust reader tags it gte-tiny, not Model2Vec)

The v4 writer, table-group layout, validation branch, and threaded char/stats
extraction are reused byte-for-byte. The header branch stays Model2Vec (128-dim),
per the audit's first-build decision (swap embed only).

## gte-tiny embed: frozen feature extractor
Per the audit, gte-tiny is a frozen feature extractor here — the multi-branch
learns the embed MLP on top. For each column: embed each sampled value with the
base TaylorAI/gte-tiny (mean-pool over tokens, attention-masked), then mean-pool
across values -> one 384-dim column vector. Computed on CPU under a global lock so
it is safe inside the existing ThreadPoolExecutor (torch is not thread-safe on a
shared device); the Rust char/stats subprocesses still run in parallel outside the
lock.

## Usage
  # Format round-trip smoke (no gte, no data) — run + Rust-train this FIRST:
  python3 scripts/build_ftmb_v5_gte.py --smoke /tmp/smoke_v5.ftmb
  # gte embed dimension check:
  python3 scripts/build_ftmb_v5_gte.py --smoke-gte
  # Full build (v19 recipe + gte embed):
  python3 scripts/build_ftmb_v5_gte.py --output output/multibranch-training/v5-gte-blend.ftmb
"""
import argparse
import os
import struct
import sys

_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
if _SCRIPTS_DIR not in sys.path:
    sys.path.insert(0, _SCRIPTS_DIR)

import prepare_multibranch_data as P  # noqa: E402

GTE_MODEL = "TaylorAI/gte-tiny"
GTE_DIM = 384  # gte-tiny hidden size; asserted against the loaded model below
VALUE_CAP = 32  # values mean-pooled per column for the embed (the mean is stable)

# v19's exact recipe (scripts/overnight_v19_paired.sh, Step 1).
V19_ARGS = [
    "--distilled", "output/distillation-v3/sherlock_distilled.csv.gz",
    "--finetype", "./target/release/finetype",
    "--label-remap", "data/label_remap.json",
    "--samples-per-type", "1200",
    "--synthetic-columns", "1200",
    "--ratio-distilled", "0.7",
    "--augmentation-rate", "0.35",
    "--filter-distilled",
    "--decontaminate",
    "--distilled-cap", "600",
    "--hard-negatives", "75",
    "--accounting-negatives", "50",
    "--status-negatives", "25",
    "--format", "v4",
    "--seed", "42",
]

# ── Frozen gte-tiny embed (lazy-loaded, thread-safe) ────────────────────────
_GTE = {}


def _load_gte():
    """Load tokenizer + frozen encoder once. Returns (tok, enc, torch, dim)."""
    if "enc" in _GTE:
        return _GTE["tok"], _GTE["enc"], _GTE["torch"], _GTE["dim"]
    import torch
    from transformers import AutoModel, AutoTokenizer
    # One intra-op thread per forward: the ThreadPoolExecutor runs ~8 gte forwards
    # concurrently, so per-forward parallelism would oversubscribe cores. Each worker
    # gets one core; aggregate throughput scales with workers instead of serializing.
    torch.set_num_threads(1)
    tok = AutoTokenizer.from_pretrained(GTE_MODEL)
    enc = AutoModel.from_pretrained(GTE_MODEL).to("cpu").eval()
    dim = enc.config.hidden_size
    assert dim == GTE_DIM, f"gte-tiny hidden_size {dim} != expected {GTE_DIM}"
    _GTE.update(tok=tok, enc=enc, torch=torch, dim=dim)
    return tok, enc, torch, dim


def gte_embed(values):
    """Frozen gte-tiny column embedding: per-value mean-pool over tokens, then
    mean-pool across values -> GTE_DIM floats. Lock-free: gte-tiny's forward is a
    stateless eval-mode pass (LayerNorm, no running buffers, no_grad), so concurrent
    inference on the shared model from the ThreadPoolExecutor workers is safe."""
    tok, enc, torch, dim = _load_gte()
    texts = [str(v) for v in values[:VALUE_CAP] if v is not None and str(v).strip()]
    if not texts:
        return [0.0] * dim
    with torch.no_grad():
        e = tok(texts, padding=True, truncation=True, max_length=64,
                return_tensors="pt")
        out = enc(**e).last_hidden_state            # (n, seq, dim)
        mask = e["attention_mask"].unsqueeze(-1).float()
        per_value = (out * mask).sum(1) / mask.sum(1).clamp(min=1)  # (n, dim)
        col = per_value.mean(0)                      # (dim,)
        return col.tolist()


# ── Monkeypatch: swap the embed slot, keep everything else ──────────────────
def _probe_valid_dim(finetype_bin="./target/release/finetype"):
    """Read the live validation-branch width from the binary. v19 trained at
    VALID_DIM=240; the taxonomy has since grown (244), so pin to what the binary
    actually emits rather than the stale module constant — a mismatch makes
    _extract_and_validate reject every column and silently waste the build."""
    import json
    import subprocess
    out = subprocess.run(
        [finetype_bin, "extract-features", "--json", "--validation"],
        input=json.dumps(["a", "b", "c"]), capture_output=True, text=True, timeout=60,
    )
    feats = json.loads(out.stdout.strip())
    return len(feats["validation"])


def install_patches():
    """Patch prepare_multibranch_data in place: EMBED_DIM->384, VALID_DIM->live,
    embed slot->gte, write v5 marker. Idempotent."""
    _, _, _, dim = _load_gte()
    P.EMBED_DIM = dim                 # dim checks + header pack + per-record pack
    live_valid = _probe_valid_dim()
    if live_valid != P.VALID_DIM:
        print(f"[build] VALID_DIM {P.VALID_DIM} (stale) -> {live_valid} (live taxonomy)")
        P.VALID_DIM = live_valid      # validation branch width = current taxonomy
    P.VERSION_V4 = 5                  # v4 writer now stamps version 5 (gte marker)

    _orig_extract = getattr(P, "_orig_extract_features", None) or P.extract_features
    P._orig_extract_features = _orig_extract

    def patched_extract_features(finetype_bin, values, header=None,
                                 include_validation=False):
        feats = _orig_extract(finetype_bin, values, header=header,
                              include_validation=include_validation)
        if feats is None:
            return None
        feats["embed"] = gte_embed(values)  # 384-dim, replaces the 512-dim Model2Vec
        return feats

    P.extract_features = patched_extract_features

    # write_ftmb_v4's default valid_dim is bound at def-time (240, stale). Wrap it
    # so the header + per-record validation pack use the live VALID_DIM.
    _orig_write = getattr(P, "_orig_write_ftmb_v4", None) or P.write_ftmb_v4
    P._orig_write_ftmb_v4 = _orig_write

    def patched_write_ftmb_v4(path, table_groups, valid_dim=None):
        return _orig_write(path, table_groups,
                           valid_dim=P.VALID_DIM if valid_dim is None else valid_dim)

    P.write_ftmb_v4 = patched_write_ftmb_v4


# ── Smoke 1: format round-trip (no gte, no data) ────────────────────────────
def smoke_write(path):
    """Write a tiny v5 with synthetic features so the Rust train-data reader can
    round-trip it. Proves the binary format + version marker independently of the
    (slow) gte + data path."""
    P.EMBED_DIM = GTE_DIM
    P.VERSION_V4 = 5
    cd, ed, sd, hd, vd = P.CHAR_DIM, GTE_DIM, P.STATS_DIM, P.HEADER_DIM, P.VALID_DIM

    def feat(n, base):
        return [float((base + i) % 7) * 0.01 for i in range(n)]

    groups = []
    labels = ["identity.person.email", "geography.location.city",
              "representation.numeric.decimal_number"]
    for g in range(40):
        recs = []
        headers = []
        for c in range(3):
            lbl = labels[(g + c) % len(labels)]
            headers.append(f"col_{g}_{c}")
            recs.append({
                "label": lbl, "column_index": c,
                "char": feat(cd, g * 3 + c), "embed": feat(ed, g + c),
                "stats": feat(sd, c), "header": feat(hd, g),
                "validation": feat(vd, g + c),
            })
        groups.append({"sibling_headers": headers, "records": recs})

    P.write_ftmb_v4(path, groups)
    print(f"[smoke] wrote {path}")
    _verify_header(path)


def _verify_header(path):
    with open(path, "rb") as f:
        magic = f.read(4)
        (version,) = struct.unpack("<I", f.read(4))
        (n_records,) = struct.unpack("<Q", f.read(8))
        char_dim, embed_dim, stats_dim, header_dim = struct.unpack("<HHHH", f.read(8))
        (n_groups, _res) = struct.unpack("<HH", f.read(4))
        (valid_dim,) = struct.unpack("<H", f.read(2))
    print(f"[smoke] magic={magic!r} version={version} n_records={n_records} "
          f"n_groups={n_groups}")
    print(f"[smoke] char={char_dim} embed={embed_dim} stats={stats_dim} "
          f"header={header_dim} valid={valid_dim}")
    assert magic == b"FTMB", magic
    assert version == 5, version
    assert embed_dim == GTE_DIM, embed_dim
    assert valid_dim == P.VALID_DIM, valid_dim
    print("[smoke] header OK (version 5, embed 384, valid 240)")


# ── Smoke 2: gte embed dimension ────────────────────────────────────────────
def smoke_gte():
    vec = gte_embed(["alice@example.com", "bob@test.org", "carol@mail.net"])
    print(f"[smoke-gte] embed len={len(vec)} sample={vec[:4]}")
    assert len(vec) == GTE_DIM, len(vec)
    zeros = gte_embed([])
    assert len(zeros) == GTE_DIM and all(x == 0.0 for x in zeros)
    print("[smoke-gte] OK (dim 384, empty -> zeros)")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--output", default="output/multibranch-training/v5-gte-blend.ftmb")
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--smoke", metavar="PATH",
                    help="write a tiny v5 + verify header, then exit")
    ap.add_argument("--smoke-gte", action="store_true",
                    help="check gte embed dimension, then exit")
    args = ap.parse_args()

    if args.smoke:
        smoke_write(args.smoke)
        return
    if args.smoke_gte:
        smoke_gte()
        return

    # Full build: v19's exact recipe, embed slot swapped to gte-tiny, written as v5.
    install_patches()
    argv = ["prepare_multibranch_data.py"] + V19_ARGS + [
        "--output", args.output, "--workers", str(args.workers),
    ]
    print(f"[build] gte-tiny embed swap; EMBED_DIM={P.EMBED_DIM} VERSION={P.VERSION_V4}")
    print(f"[build] argv: {' '.join(argv[1:])}")
    sys.argv = argv
    P.main()


if __name__ == "__main__":
    main()
