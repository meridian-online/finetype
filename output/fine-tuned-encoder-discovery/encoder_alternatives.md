# Encoder alternatives to MiniLM (HN thread review)

**Spec:** 2026-06-18-minilm-encoder-build · **Date:** 2026-06-18
**Prompt:** author flagged HN item 46081800 ("stop using all-MiniLM-L6-v2") given speed+accuracy matter.
**Code:** `output/fine-tuned-encoder-discovery/alternatives_probe.py`

## Verdict: yes — switch the lead candidate from MiniLM-L6 to **gte-tiny**. Same speed, better accuracy, free.

The thread optimizes for *general retrieval* (long context, MTEB, multilingual) and recommends
*bigger* models (EmbeddingGemma 300M, Qwen3-Embedding-0.6B, bge-base, nomic). That's the wrong
objective for us: our binding constraint is **CPU inference speed**, our inputs are ~8 short
values (<<512 tokens), and we fine-tune a *classification head* — not retrieval. Its critiques
of MiniLM (old, 512-ctx) are moot here. And Gemma/Qwen3/nomic use non-BERT architectures that
candle-transformers may not support → integration risk, slower anyway.

## Head-to-head on OUR objective (244 contested cols, header+values, 5-fold CV ranking)

| model | size | separability | candle CPU batched | candle Metal batched | candle? |
|---|--:|--:|--:|--:|---|
| **gte-tiny** | **46 MB** | **0.872** | 10.6 ms | 0.09 ms | ✓ same BERT path |
| MiniLM-L6 (incumbent) | 90 MB | 0.807 | 10.6 ms | 0.07 ms | ✓ |
| bge-small | 130 MB | 0.787 | (slower) | — | ✓ |
| static potion-32M | 120 MB | 0.754 | 0.05 ms | — | ✓ (static) |
| static potion-8M | 30 MB | 0.750 | 0.05 ms | — | ✓ (static) |
| gte-small | 67 MB | 0.717 | — | — | ✓ |
| ModernBERT-embed-base | 149 MB | 0.778 | — | — | ✓ candle has `modernbert.rs`; 39ms CPU (torch), 5× slower |
| mdbr-leaf-ir (MongoDB) | 23 MB | 0.774 | — | — | ✓ BERT; IR-tuned, Apache-2.0 |

(CV numbers are an internally-consistent ranking — no StandardScaler — so lower than the scaled
0.893 baseline; the +6.5pp gap is the signal.)

### Follow-up: mdbr-leaf-ir and ModernBERT (author-flagged) — both lose to gte-tiny

- **mdbr-leaf-ir** (MongoDB, 23M, Apache-2.0, #1 BEIR ≤100M, candle-runnable): separability
  **0.774** — below gte-tiny (0.872) and MiniLM (0.807). Strong *retrieval* pedigree doesn't
  transfer to our classification task (the IR-vs-classification caveat, confirmed).
- **ModernBERT-embed-base** (149M, modern arch, candle HAS `modernbert.rs`): separability
  **0.778** AND **~5× slower on CPU** (39 ms vs gte-tiny 7.4 ms, torch). Disqualified on both
  axes — its flash-attention/8k-context efficiency is a GPU/long-context win, irrelevant to our
  short-input CPU case. Only comes at 149M (no tiny variant), 6× gte-tiny.

**Sweep complete (8 candidates): gte-tiny wins on separability AND is among the fastest/smallest.**
The repeated lesson — bigger general-retrieval/modern models do NOT separate our specific
short-input type boundaries better; tiny + task-fit beats them. Caveat: zero-shot probe; a
fine-tune *could* shift the ranking, but ModernBERT's 5× CPU cost is disqualifying regardless,
and gte-tiny leads zero-shot by a clear margin. **gte-tiny remains the build's lead encoder.**

## Findings

1. **gte-tiny is a strict win over MiniLM** — +6.5pp separability, half the disk size, and
   **identical candle latency** (it's the same 6-layer/384-hidden BERT; the size delta is the
   embedding table, not compute, so forward-pass cost is the same). It dropped into the existing
   candle bin unchanged (`MINILM_DIR=<gte-tiny> ./minilm-latency` ran, deterministic). No
   downside — better accuracy at the same speed and integration.
2. **Latency budget unchanged** — same architecture → same numbers: CPU batched ~10.6 ms (at
   budget), Metal batched ~0.09 ms (trivial). The ac-00/ac-00.5 conclusions carry over.
3. **Static caps ~0.75** — potion-32M barely beats 8M; below the contextual encoders. Confirms a
   contextual encoder is needed; static alone won't crack the contested boundary (but stays the
   fast path for the confident ~70%).
4. **Bigger ≠ better here** — bge-small (130MB) and gte-small (67MB) both *underperform* gte-tiny
   on this task. Tiny + fine-tuned for the task beats bigger general-retrieval models.

## Action for the build

- **Lead encoder candidate: `gte-tiny`** (was MiniLM-L6). Better separability, same candle
  latency + path, smaller footprint. Drop-in for ac-00/ac-01.
- Keep MiniLM-L6 as a fallback; both are validated candle-runnable.
- Skip the thread's large retrieval models (wrong objective + candle risk). Re-probe only if a
  small **ModernBERT**-class encoder with candle support appears (modern arch, still BERT-family).
- Confirm gte-tiny's absolute separability under the scaled/fine-tuned pipeline during ac-01.
</content>
