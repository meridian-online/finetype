# build ac-00.5 — GPU (Metal/M1) + device latency

**Spec:** 2026-06-18-minilm-encoder-build (follow-up to ac-00)
**Date:** 2026-06-18 · candle-transformers 0.8.4, all-MiniLM-L6-v2 fp32, Apple M1 Pro
**Code:** `crates/finetype-candle-spike/src/bin/minilm_latency.rs --features metal` (DEVICE=metal|cpu)

## Verdict: the GPU resolves the latency objection. Metal batched is ~free.

| measurement | CPU fp32 | **Metal (M1 GPU)** | speedup |
|---|--:|--:|--:|
| single-column | 45.8 ms/col | 14.2 ms/col | 3.2× |
| **batched(32)** | 10.7 ms/col | **0.07 ms/col** | **150×** |
| corpus low-band (30% × 6.6M) | 5.9 h | **~2 min** | — |
| budget (≤10 ms/col) | single OVER / batched OVER | single OVER / **batched PASS** | — |
| deterministic | yes | yes | — |

The M1 GPU is enormously faster on **batched** matmuls (the corpus-pass regime): low-band
escalation over the whole corpus drops from ~6 h (CPU) to ~2 minutes (Metal). Single-column
(batch-1) only 3× faster — kernel-launch overhead dominates a single tiny input — but at 14 ms
it's still sub-frame interactively.

## The load-bearing caveat: Metal is Apple-Silicon-only

FineType ships cross-platform (Linux x86/arm, macOS x86/arm, Windows). So the GPU win applies to:

- **Mac analysts, interactively** — a handful of low-band columns at 14 ms = imperceptible.
- **A corpus pass hosted on Apple Silicon (this M1) or a CUDA Linux box** (candle has a `cuda`
  feature, same story) — now minutes, not hours.

It does **not** help a no-GPU Linux/Windows host running the shipped binary. There the CPU number
governs: **batched 10.7 ms/col — at budget**, workable; int8 quantisation remains the lever to
give margin and is the open ac-00.5b probe (candle quantized BERT, untested — now lower priority
since the GPU path already clears it where a GPU exists).

## Net effect on the build

ac-00's "conditional pass" firms up: the latency objection is **resolved on GPU hosts** (Metal/
CUDA) and **at-budget on CPU** when batched. Practical plan for the build:

- Encoder always runs **batched** over a file's low-band columns (never one-at-a-time).
- Corpus passes run on the M1/a GPU box → trivially fast.
- Cross-platform CPU release: at-budget batched; int8 quant if margin is wanted.
- Interactive use: fine on every platform.

So latency no longer threatens the build. The real risk moves entirely to the **accuracy /
attractor** question (ac-01/ac-02) — does fine-tuning convert the 0.893 separability ceiling into
corpus-honest-clean recall without re-broadening the residual. That is the next thing to probe.
</content>
