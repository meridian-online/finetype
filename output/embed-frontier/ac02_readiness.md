# ac-02 readiness — prep done, decision rule pre-registered

**Date:** 2026-06-22 · spec `2026-06-21` ac-02 · prep before any training burns

## Prep completed (this session)

1. **Potion artifacts staged + verified.** `models/m2v8m/` (potion-base-8M, [29528,256] F32) and
   `models/m2v32m/` (potion-base-32M, [63091,512] F32), each `model.safetensors` + `tokenizer.json`
   from the HF cache. Verified tokenizer vocab **matches** embedding rows exactly (29528 / 63091) —
   correctly paired, no mismatch. HF potion ships the `embeddings` tensor in the format the Rust
   loader already reads (F32; the loader also handles our F16 4M), so native ship is artifact-ready.
2. **Truncation-proof builder.** `scripts/build_ftmb_v5_potion.py` computes the potion embedding
   aggregation **in Python** (full width) and patches it into `prepare_multibranch_data`, exactly as
   the gte builder does — sidestepping the Rust `EMBED_DIM=128` truncation. One-variable change: embed
   encoder only; char/stats(27)/header/validation(244) and the whole v19 data blend untouched.
   Smoke-tested: single-view 8M -> embed 1024, two-view 4M++32M -> 2560, FTMB v5 round-trips,
   `read_ftmb --verify` PASS. `model2vec` installed into the eval venv (one interpreter build+train+score).
3. **Composed-loss analysis** (`ac02_prep_composed_loss.md`): of m2v-244's 34 composed losses vs v19,
   only **9 are embed-addressable (~1pp)**; **25 are 0096 residual-attractor** (rule-shaped, embed
   can't fix — dominated by alphanumeric_id->h3, 14 cols).

## Two findings that reshape expectations

- **Bigger potions are ~free at the embed step.** Measured ~0.5 ms/col **flat** across 4M/8M/32M
  (encode is tokenization-dominated, not dim-dominated; the 4x dim is NOT 4x latency). So the
  "<=3x latency" budget is met at ~1x — ac-02 collapses to a near-pure **accuracy** question.
- **Composed beating v19 is mostly a rule problem, not an embed one.** The honest embed prize is a
  stronger **Sense** + ~1pp composed; the other 2/3 of the v19 composed gap is the alnum-vs-geohash
  value-rule (a separate, bankable win).

## Pre-registered decision rule (write it before the run reads itself)

Each potion candidate is 3 seeds, scored fresh-vs-fresh against **m2v-244-s44** (Sense 0.521,
composed 0.769), best-of-3.

- **SHIP-worthy (-> ac-03/04/05):** best-of-3 **Sense > m2v-244 + CI** AND latency <= ~3x (a gimme)
  AND composed **improve-or-hold** vs m2v-244 AND corpus-relocation gate (H05) clean.
- **Frontier knee / new baseline:** Sense beats m2v-244 at acceptable latency even if composed only holds.
- **Null (-> stop / fastembed-rs fallback):** Sense flat vs m2v-244 -> capacity doesn't buy it.
- **Report card to watch:** the 9 embed-addressable sibling confusions (isbn/unix-ms,
  country/locale, region/http_method) — if a bigger potion sharpens these, the representation
  genuinely improved.

## Run plan (staged — don't burn all three blind)

1. **potion-8M single-view first**, 3 seeds, as a go/no-go. `build_ftmb_v5_potion.py --potion
   minishlab/potion-base-8M` -> 3-seed train (m2v-244 config but embed_dim 1024) -> Sense + composed + latency.
2. If 8M lifts Sense -> **potion-32M** (embed_dim 2048), 3 seeds.
3. **two-view 4M++32M** (embed_dim 2560) — tests the axis that carried most of gte's gain
   (gte two-view 0.571 vs single floor 0.532). Run even if single-view 8M is flat: a flat
   single-view + a strong two-view is the real signal.
4. If the whole static frontier is flat -> fastembed-rs ONNX (quantised gte/bge) fallback.

**Note on config:** each potion needs an n_classes=244/valid_dim=244 config with `embed_dim` set to
the potion's agg width (1024 / 2048 / 2560) — clone `m2v-244-config.json`, change `embed_dim` only.
The training script must build features with `build_ftmb_v5_potion.py` (NOT vanilla prepare) and use
that per-potion config — same gates (VALID_DIM=244, FTMB truncation) as `overnight_m2v_244.sh`.
