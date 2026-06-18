# build ac-00 — Rust/candle latency probe

**Spec:** 2026-06-18-minilm-encoder-build (GATE, the cheapest kill)
**Date:** 2026-06-18 · candle-transformers 0.8.4, CPU, fp32, off-the-shelf all-MiniLM-L6-v2
**Code:** `crates/finetype-candle-spike/src/bin/minilm_latency.rs` (`cargo run --release --bin minilm-latency`)

## Verdict: CONDITIONAL PASS — Rust feasibility confirmed; the latency budget was optimistic.

The candle BERT path **works** (loads via candle-transformers + tokenizers + safetensors,
embeddable, **deterministic across runs**). But candle's CPU gemm is ~7× slower than the
torch/MKL number the discovery budgeted from, so the latency plan tightens.

| measurement | candle CPU fp32 | vs 10 ms budget |
|---|--:|---|
| single-column | 45.9 ms/col | OVER (5×) |
| **batched (32)** | **10.6 ms/col** | essentially AT budget |
| corpus low-band (30% × 6.6M), single | ~25 h | — |
| corpus low-band, batched | ~5.8 h | vs ~1.6 h every-column baseline |

(Python/torch single-col was 6.7 ms — MKL-accelerated; candle has no equivalent, threading
1/4/8 gave 66/46/49 ms, so threads aren't the lever.)

## What this means for the build

- **Rust is viable** — the encoder runs offline + deterministically in the existing candle
  stack, no new framework. The discovery's "runnable in Rust" claim holds.
- **The 6.7 ms figure was wrong by ~7×** for single-column candle. The build must:
  1. **Batch the low-band escalation** (recovers to ~10.6 ms/col, essentially budget), and
  2. likely **int8-quantise** (untested here — the next cheap lever to clear 10 ms comfortably
     and cut the corpus pass), and/or
  3. consider a **smaller/faster base** in the ac-01 fine-tune (MiniLM-L3, or a more aggressively
     distilled encoder) if quant doesn't suffice.
- **Interactive use is fine regardless** — a handful of low-band columns per table at 46 ms is
  sub-second. The binding constraint is the corpus pass: batched ~5.8 h (vs 1.6 h baseline) is a
  workable cost for an infrequent batch job, improvable with quant.

## Gate decision

Not a NO-GO — a viable path exists at/near budget (batched) with a clear remaining lever
(quant). The bet survives ac-00, but carries a **known latency cost** the discovery's torch
number hid: corpus passes get ~3–4× slower and the escalation MUST batch. ac-01 (fine-tune)
should keep the base small enough to preserve this margin.

## Honest scope

This probed the OFF-THE-SHELF model. A fine-tuned model is the same architecture → same latency.
int8 quantisation in candle (gguf or manual) is the untested lever and the natural ac-00.5
follow-up before committing to the full build. The corpus-pass projections assume the measured
batched rate holds at scale — a real corpus batch run would confirm.
</content>
