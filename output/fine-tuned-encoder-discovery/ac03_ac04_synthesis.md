# ac-03 + ac-04 — attractor risk + synthesis/recommendation

**Spec:** 2026-06-18-fine-tuned-encoder-discovery
**Date:** 2026-06-18

## ac-03 — would fine-tuning hit the same residual-attractor wall?

**Verdict: the encoder upgrade is the FIRST bet that targets the root cause, so the
attractor prior is genuinely more open than the 0-for-6 record — but a richer encoder
is necessary, not sufficient. The build must pair it with precedence-aware training.**

The reasoning:

- The 6 failed retrains (`categorical-is-a-residual-category`,
  `identity-retrain-broadened-fullname-no-go`) all broadened residual catch-alls under a
  flat 250-softmax fed by the **same static encoder**. Mechanism: when the encoder cannot
  *tell apart* RESIDUAL from `country_code`, the loss minimises by defaulting to the
  high-frequency attractor. The over-emission is partly an **encoding** failure.
- The probe shows a contextual encoder **does** separate those boundaries (0.893 vs static
  0.803). A head on top of separable features has the signal to *not* collapse them — so
  upgrading the encoder plausibly **changes the dynamics**, unlike the falsified head
  (`hierarchical-head-falsified`, head-only) and stranded sibling
  (`sibling-context-is-the-stranded-architecture-layer`, context-only) bets, both of which
  left the static encoder untouched. This is the locus the ceiling discovery named.
- BUT separability ≠ no attractor. The residual-precedence problem (decision 0096) is also
  about **precedence semantics** — `categorical`/`alphanumeric_id` are "no tighter type
  fits" *decisions*, and a flat softmax + frequency imbalance can still bias toward the
  attractor even with a good encoder. A richer encoder removes the *encoding* cause, not
  necessarily the *training-dynamics* cause.

**Training design a build would require (not merely "fine-tune"):**
1. Fine-tune the small encoder (MiniLM-L6) as the header+value representation, with a
   classification head — replacing/augmenting the static Model2Vec header branch.
2. **Precedence-aware training for the residual**: class-balancing or an explicit
   "no-specific-type" abstain outcome so the residual is a *decision*, not a frequency sink
   (the 0096 lesson). Hierarchical/precedence head reconsidered — it was falsified *with the
   static encoder*; the calculus differs on separable features, but treat as unproven.
3. Clear ALL four gates that killed the additive retrains: drift proxy → gold →
   representative → corpus-honest (all shipped this session). The gates are the real test.

## ac-04 — synthesis: can "small + local + fast + fine-tuned" be all four?

**Recommendation: GO to a scoped BUILD-AND-GATE spec — the first model-side GO after
0-for-6 — with eyes open on the caveats. Lead candidate: fine-tune `all-MiniLM-L6-v2` as a
low-band escalation encoder.**

| axis | verdict | evidence |
|---|---|---|
| **small** | yes | MiniLM-L6 = 22M params |
| **local** | yes | embeddable + offline like the 3 models already compiled in; deterministic (fixed weights) |
| **fast** | yes, low-band-only | 6.7 ms/col < 10 ms ceiling; every-column off the table (~12h corpus) |
| **fine-tuned** | yes, with upside | 0.893 is **zero-shot**; fine-tuning on the type task only improves it; standard encoder+head |

Resolving the four ac-04 questions:

1. **Latency (ac-01):** a candidate clears the budget — MiniLM-L6 at 6.7 ms/col in the
   low-band-only regime (gated by the shipped `quality_band < 0.70`). bge-small (11.4ms) and
   Qwen2.5-0.5B (69ms) are interactive-only.
2. **Signal (ac-02):** present and material — contextual encoding separates the contested
   semantic boundaries at 0.893 vs static 0.803 vs the shipped model's 0.684 (+21pp over the
   model). And the decoder LLM's +0.8pp doesn't justify 10× the latency, so a *small encoder*
   is the right tool — the cheapest part of the candidate space wins.
3. **Attractor (ac-03):** the encoder upgrade addresses the root the prior bets missed, so the
   prior is open — but the build needs precedence-aware training and must clear the gates.
4. **Prize vs cost:** up to +9pp gold / +12pp representative (oracle ceiling; realistic less),
   for the cost of a fine-tune pipeline + a low-band escalation integration. The integration
   pattern already exists (three embedded local models).

**The honest framing:** this is a GO to **build and gate**, NOT a GO to ship. Six retrains
died at these gates; the difference this time is a *mechanistic* reason to expect different
behaviour (the encoder is the separability bottleneck, now demonstrably liftable) plus
zero-shot evidence the signal is real. The separability number is a ceiling, not a promise —
the corpus-honest relocation gate remains the arbiter.

**Recommended next spec:** build `finetype-minilm-encoder` — fine-tune all-MiniLM-L6-v2 on
the generator + distilled type data with precedence-aware residual handling, integrate as a
`quality_band`-gated low-band escalation encoder, and run the full promotion scoreboard.
Frame the GO/NO-GO on the corpus-honest gate, not the separability probe.

**What we don't know:** whether fine-tuning + the attractor-aware design actually converts the
0.893 *ceiling* into corpus-honest-clean recall (the thing all 6 retrains failed), and whether
the low-band escalation integration holds latency on real multi-column tables. Those are the
build's job — this discovery establishes the bet is worth making, not that it will land.
</content>
