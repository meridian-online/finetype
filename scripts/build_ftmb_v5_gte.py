#!/usr/bin/env python3
"""Build an FTMB **v5** feature binary: v19's exact multi-branch recipe with the
512-dim Model2Vec value-aggregation embed slot replaced by a frozen gte-tiny value
embedding under Model2Vec's SAME aggregation — 4 statistics (mean ++ variance ++ min
++ max) over gte's 384-dim per-value vectors = 1536-dim. Every other branch (char,
stats, header=Model2Vec, validation) and the whole data recipe are v19's, untouched.

The aggregation is matched on purpose: Model2Vec encodes each value to 128-dim then
keeps mean/var/min/max (4x128=512). A bare-mean gte slot would silently DOWNGRADE
the aggregation (dropping variance — the within-column-spread signal that separates
categorical from free-text), confounding the encoder upgrade. Matching it makes the
swap a true one-variable change: encoder only.

Spec: 2026-06-20-gte-tiny-embed-branch-swap, ac-01.
Audit: spec 2026-06-20-gte-tiny-embed-branch-swap (ac00_audit.md)

## Why a monkeypatch and not a reimplementation
The audit's load-bearing warning is that a misaligned binary does not crash — it
silently wastes the overnight retrain. The single biggest divergence risk is the
DATA RECIPE (distilled load + filter + decontaminate + cap + synthetic generation
+ hard negatives + augmentation + 0.7 blend + table grouping + validation branch).
Reimplementing ~150 lines of that risks silent drift from v19. So this script does
NOT reimplement it: it imports prepare_multibranch_data and runs its `main()`
verbatim with v19's exact argv, monkeypatching only three things:

  1. EMBED_DIM            512 -> 1536  (gte 384-dim x 4 aggregation statistics)
  2. extract_features     embed slot   -> frozen gte-tiny + 4-stat column aggregation
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
GTE_DIM = 384  # gte-tiny hidden size (per-value); asserted against the loaded model
# Model2Vec's embed branch aggregates a column with 4 statistics (mean ++ variance
# ++ min ++ max), each 128-dim -> 512. We mirror that aggregation exactly on gte's
# 384-dim per-value vectors -> 4 x 384 = 1536. This keeps the swap a true
# one-variable change (encoder only) instead of silently downgrading aggregation.
EMBED_AGG_DIM_PER = 4 * GTE_DIM  # 1536 per encoder
EMBED_AGG_DIM = EMBED_AGG_DIM_PER  # total; set to 2x in two-view mode (main/install)
VALUE_CAP = 32  # values encoded per column before aggregation (the stats are stable)

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

# ── gte embed (lazy-loaded, thread-safe). One encoder, or two for two-view ──
_ENCODERS = []            # loaded encoder stack [{tok, enc, torch}, ...], concat order
_ENCODER_CKPT = None      # single-view: fine-tuned encoder .pt {base_model, enc}
_TWO_VIEW_FT_CKPT = None  # two-view: frozen GTE_MODEL ++ this fine-tuned encoder


def _load_encoders():
    """Load the encoder stack once. Single-view: [frozen base] or [fine-tuned ckpt].
    Two-view: [frozen GTE_MODEL, fine-tuned ckpt] — their embeds are concatenated, so
    the multi-branch sees a format-preserving view AND a semantic view and learns to
    weight each per type (spec 2026-06-20, ac-03 two-view bet). Each encoder is frozen;
    the multi-branch learns the embed MLP on top."""
    if _ENCODERS:
        return _ENCODERS
    import torch
    from transformers import AutoModel, AutoTokenizer
    # One intra-op thread per forward: ~8 forwards run concurrently across the
    # ThreadPoolExecutor; per-forward parallelism would oversubscribe cores.
    torch.set_num_threads(1)

    def _load(base=None, ckpt=None):
        if ckpt:
            c = torch.load(ckpt, map_location="cpu", weights_only=False)
            base = c["base_model"]
            enc = AutoModel.from_pretrained(base)
            enc.load_state_dict(c["enc"])
            tag = f"fine-tuned {ckpt} (base {base})"
        else:
            enc = AutoModel.from_pretrained(base)
            tag = f"frozen {base}"
        tok = AutoTokenizer.from_pretrained(base)
        enc = enc.to("cpu").eval()
        assert enc.config.hidden_size == GTE_DIM, \
            f"encoder hidden_size {enc.config.hidden_size} != {GTE_DIM}"
        print(f"[build] encoder view {len(_ENCODERS)}: {tag}")
        return {"tok": tok, "enc": enc, "torch": torch}

    if _TWO_VIEW_FT_CKPT:
        _ENCODERS.append(_load(base=GTE_MODEL))          # view 0: format-preserving
        _ENCODERS.append(_load(ckpt=_TWO_VIEW_FT_CKPT))  # view 1: semantic
    elif _ENCODER_CKPT:
        _ENCODERS.append(_load(ckpt=_ENCODER_CKPT))
    else:
        _ENCODERS.append(_load(base=GTE_MODEL))
    return _ENCODERS


def _embed_one(ed, texts):
    """4-stat L2-normed column embedding for ONE encoder -> EMBED_AGG_DIM_PER floats.
    Per-value: mean-pool tokens (masked) -> L2-normalise (GTE's canonical cosine space,
    also bounds features to [-1, 1]). Column: mean ++ var ++ min ++ max."""
    tok, enc, torch = ed["tok"], ed["enc"], ed["torch"]
    with torch.no_grad():
        e = tok(texts, padding=True, truncation=True, max_length=64, return_tensors="pt")
        out = enc(**e).last_hidden_state            # (n, seq, dim)
        mask = e["attention_mask"].unsqueeze(-1).float()
        pv = (out * mask).sum(1) / mask.sum(1).clamp(min=1)  # (n, dim)
        pv = pv / pv.norm(dim=1, keepdim=True).clamp(min=1e-12)
        return torch.cat([pv.mean(0), pv.var(0, unbiased=False),
                          pv.min(0).values, pv.max(0).values]).tolist()


def gte_embed(values):
    """Column embed = per-encoder 4-stat L2-normed aggregate, concatenated:
    EMBED_AGG_DIM_PER (1536) single-view, or 2x (3072) two-view (frozen ++ fine-tuned).
    Lock-free: stateless eval forward, safe across the ThreadPoolExecutor workers."""
    encs = _load_encoders()
    texts = [str(v) for v in values[:VALUE_CAP] if v is not None and str(v).strip()]
    if not texts:
        return [0.0] * EMBED_AGG_DIM
    return [x for ed in encs for x in _embed_one(ed, texts)]


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
    _load_encoders()
    P.EMBED_DIM = EMBED_AGG_DIM        # dim checks + header pack + per-record pack
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
    P.EMBED_DIM = EMBED_AGG_DIM
    P.VERSION_V4 = 5
    cd, ed, sd, hd, vd = P.CHAR_DIM, EMBED_AGG_DIM, P.STATS_DIM, P.HEADER_DIM, P.VALID_DIM

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
    assert embed_dim == EMBED_AGG_DIM, embed_dim
    assert valid_dim == P.VALID_DIM, valid_dim
    print(f"[smoke] header OK (version 5, embed {EMBED_AGG_DIM}, valid {valid_dim})")


# ── Smoke 2: gte embed dimension ────────────────────────────────────────────
def smoke_gte():
    vec = gte_embed(["alice@example.com", "bob@test.org", "carol@mail.net"])
    print(f"[smoke-gte] embed len={len(vec)} sample={vec[:4]}")
    assert len(vec) == EMBED_AGG_DIM, len(vec)
    # variance block (slots dim..2*dim) must be non-zero for >1 distinct value
    var_block = vec[GTE_DIM:2 * GTE_DIM]
    assert any(abs(x) > 1e-9 for x in var_block), "variance block all-zero"
    zeros = gte_embed([])
    assert len(zeros) == EMBED_AGG_DIM and all(x == 0.0 for x in zeros)
    print(f"[smoke-gte] OK (dim {EMBED_AGG_DIM}, {EMBED_AGG_DIM // EMBED_AGG_DIM_PER} view(s) x "
          f"4x{GTE_DIM}, variance live, empty -> zeros)")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--output", default="output/multibranch-training/v5-gte-blend.ftmb")
    ap.add_argument("--workers", type=int, default=8)
    ap.add_argument("--encoder-checkpoint", default=None,
                    help="single-view fine-tuned encoder .pt ({base_model, enc}); default frozen base")
    ap.add_argument("--two-view-ft", default=None,
                    help="two-view: embed = frozen GTE_MODEL ++ this fine-tuned encoder (3072-dim)")
    ap.add_argument("--smoke", metavar="PATH",
                    help="write a tiny v5 + verify header, then exit")
    ap.add_argument("--smoke-gte", action="store_true",
                    help="check gte embed dimension, then exit")
    args = ap.parse_args()

    global _ENCODER_CKPT, _TWO_VIEW_FT_CKPT, EMBED_AGG_DIM
    _ENCODER_CKPT = args.encoder_checkpoint
    if args.two_view_ft:
        _TWO_VIEW_FT_CKPT = args.two_view_ft
        EMBED_AGG_DIM = 2 * EMBED_AGG_DIM_PER  # frozen ++ fine-tuned

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
    mode = ("two-view frozen++ft" if _TWO_VIEW_FT_CKPT
            else "ft" if _ENCODER_CKPT else "frozen")
    print(f"[build] gte embed swap [{mode}]; EMBED_DIM={P.EMBED_DIM} VERSION={P.VERSION_V4}")
    print(f"[build] argv: {' '.join(argv[1:])}")
    sys.argv = argv
    P.main()


if __name__ == "__main__":
    main()
