# ac-00 — Bigger-potion embed-swap audit + ac-01 build path

**Date:** 2026-06-21 · spec `2026-06-21-reproducible-baseline-and-static-embeddings` · card 0002

## Headline

A bigger potion (8M/32M) is a **near-drop-in, and `model2vec-rs` is NOT needed.** The Rust
`Model2VecResources` loader is already dimension-agnostic; bigger potions are the *same*
safetensors format (an `embeddings` tensor + `tokenizer.json`). The only thing standing
between us and the richer embedding is a **two-line const bump** (`EMBED_DIM`/`EMBED_AGG_DIM`)
that today silently **truncates** any embedding wider than 128 dims back down to 128.

For the experimental path (ac-01/ac-02 — train in Python, score Sense-vs-Sense), there is
**zero Rust change**: the FTMB builder already carries embed width through the header and the
trainer/scorer read it back. The gte experiment (1536/3072-dim) is the working precedent.

## The potion dims (read from the cached safetensors)

| model            | embeddings shape | embed_dim | EMBED_AGG_DIM (4×) | vocab  |
|------------------|------------------|-----------|--------------------|--------|
| potion-base-4M   | [29528, 128]     | 128       | 512 (current)      | 29 528 |
| potion-base-8M   | [29528, 256]     | 256       | 1024               | 29 528 |
| potion-base-32M  | [63091, 512]     | 512       | 2048               | 63 091 |

All three are already in the HF cache (`~/.cache/huggingface/hub/models--minishlab--potion-base-{4,8,32}M`).
Bigger = wider embedding (and 32M also widens the vocab — handled automatically because
`tokenizer.json` is swapped alongside `model.safetensors`).

## Why it is NOT a literal drop-in (the truncation trap)

`crates/finetype-model/src/embedding_aggregation.rs`:
- `EMBED_DIM = 128` and `EMBED_AGG_DIM = 512` are **compile-time consts** (lines 14, 17).
- `extract_embedding_aggregation` loops with `embed_dim.min(EMBED_DIM)` (lines 60, 78) and
  writes a fixed `[f32; EMBED_AGG_DIM]` result.

So if you swap `models/model2vec/` for potion-8M and rebuild **without** touching the consts,
the loader reads 256-dim vectors and then **discards dims 128..256** — you keep the same
512-wide feature, get none of the richness, and the model still runs (no crash, no signal).
That is the silent failure mode to avoid: capturing the gain *requires* bumping the consts.

`Model2VecResources` itself is clean — it reads the width from the tensor at load
(`embed_dim() = embeddings.dim(1)`, `model2vec_shared.rs:66`) and `encode_batch` allocates
per the runtime dim. No hardcoded width there. Hence: **no `model2vec-rs`.**

## Consumer graph of the embed-width consts (B07)

`codegraph_impact(EMBED_AGG_DIM)` + grep, deduped:

| consumer | file:line | role | change to ship a bigger potion |
|---|---|---|---|
| `EMBED_DIM` const | `embedding_aggregation.rs:14` | width of one stat block | 128 → 256 (8M) / 512 (32M) |
| `EMBED_AGG_DIM` const | `embedding_aggregation.rs:17` | 4× block (mean/var/min/max) | 512 → 1024 / 2048 |
| `extract_embedding_aggregation` | `embedding_aggregation.rs:25` | aggregator | picks up consts automatically |
| extract-features path | `main.rs:1464` | training feature extraction | uses `EMBED_AGG_DIM` import — automatic |
| `models/model2vec/` artifacts | (data) | the embedding matrix + tokenizer | replace via `prepare_model2vec.py` |
| model config `embed_dim` | per-model `config.json` | trainer/inference width contract | must equal the trained FTMB embed width |

The multi-branch **inference** embed slot reads its width from the model/FTMB header, not a
const (consistent with the dim-agnostic stats reader, memory
`multibranch-stats-already-has-cardinality`) — so once the consts + artifacts + config agree,
native inference works. **Edit set for a native ship (ac-05) is the two consts + the model-dir
swap + a config `embed_dim` that matches the trained model.** Small and contained.

The only other `EMBED_AGG_DIM` reference is `scripts/build_ftmb_v5_gte.py:69` — the gte
experiment's own builder, which sets it to `4 × GTE_DIM` and monkey-patches
`prepare_multibranch_data.py`'s `EMBED_DIM`/`VALID_DIM` in place. That is the **template** for
the ac-02 bigger-static builder: point the value encoder at potion-8M/32M, set
`EMBED_DIM = potion_dim`, `EMBED_AGG_DIM = 4 × potion_dim`, leave every other branch untouched.

## Decision

- **`model2vec-rs`: not needed.** In-place const bump + artifact swap covers both 8M and 32M.
- **Experimental (ac-01/ac-02):** Python-only, dim-agnostic, no Rust edit. Score Sense-vs-Sense
  via `score_gold_anchor.py predict --raw-model`.
- **Native ship (ac-05):** bump the two consts, swap `models/model2vec/`, set the config
  `embed_dim`. ~3-line code change; no format bump, no new crate.

## ac-01 build path (27 stats, format v4, potion-4M — the reproducible baseline)

`main` is reverted to v19's bit-exact stats (`COLUMN_STATS_DIM = 27`, commit `bdb7c79`;
`prepare_multibranch_data.py:96 STATS_DIM = 27`). So **no stats untangling** — `overnight_v19_paired.sh`
is the right recipe (format v4, 27 stats, default `models/model2vec` = potion-4M, the
`--samples-per-type 1200 / --distilled-cap 600 / seed 42` blend).

**Two build-hygiene fixes to bake into the ac-01 script — both already cost builds:**

1. **Stale VALID_DIM gate.** The builder was moved to live taxonomy (`VALID_DIM = 244`,
   `prepare_multibranch_data.py:98`, commit `0785515`) and the live taxonomy is **244** types
   (`finetype taxonomy` → 244). But the inline audit gate in `overnight_v19_paired.sh:290`
   still hardcodes `if valid_dim != 240` and would **fail the build**. Fix: assert `== 244`
   (ideally derive from `finetype taxonomy | wc -l`, not a literal). A stale dim here silently
   rejects every column at inference — the exact trap flagged for this session. *(Note: v19
   shipped at 240; building at 244 is correct per the directive and is itself one diagnostic
   axis for ac-01's drift gap — taxonomy 240→244.)*

2. **Truncated-FTMB guard.** Keep the existing `read_ftmb.py --verify` calls (lines 195, 253) —
   but `--verify` only `sys.exit(1)`s on NaN/Inf/dim issues; a mid-write crash that truncates
   the file surfaces only as a `WARNING: EOF at record N (expected M)` and does **not** fail.
   So the build script must additionally **assert the reported record count equals the expected
   group/record count** (grep the verify output for `WARNING: EOF` and abort if present), not
   just trust the exit code.

**Do not** add coordinate "fixes" — v19 composed already nails coordinates (1.00); the gold
coordinate gap is the `decimal_number` residual-attractor (memory
`decimal-number-is-a-residual-attractor`), not a representation gap, and only shows up in
standalone Sense.

## Reference numbers (honest gate — Sense-vs-Sense)

v19 composed 0.793 / Sense 0.502 · gte two-view composed 0.787 / Sense 0.571 ·
Model2Vec 0.39 ms/col vs gte-tiny 39 ms/col (~100×) · cdist fresh-retrain Sense 0.316.
