# Encoder build — revised slim plan (zoom-out, 2026-06-19)

**Premise:** gte-tiny broke the semantic ceiling. Most of the promotion apparatus is scar
tissue from one specific failure — additive retrains of the flat-softmax multi-branch that
relocated errors at corpus scale (0-for-6). gte-tiny is a *separate encoder*, not an additive
softmax rebalance, so it doesn't share that failure mode. The process should shrink to match.

## Architecture (slim)

A type-routed two-stage. **No confidence machinery, no escalation gating.**

```
column ─▶ deterministic parsers (datetime, codes, coords, numeric formats) ──▶ type
       └▶ static multi-branch (structural + confident types) ──────────────────▶ type
              └─ if it predicts a CONTESTED-SEMANTIC type (geo / person /
                 entity / residual) ─▶ gte-tiny re-decides ─────────────────────▶ type
```

Routing is by **predicted type** — deterministic, needs no per-column confidence and no
corpus-pass change. Each tool runs where it is strong: parsers for parseable, static for
structural, gte-tiny for the semantic boundary it just proved it can separate (0.811 vs 0.684).

## Ceremony dropped from this path (with why)

- **Drift proxy** — predicts flat-softmax distribution explosions before an overnight retrain.
  gte-tiny has no shared softmax to rebalance; the failure it guards against cannot occur. N/A.
- **Confidence-capture + low-band escalation machinery** — the "minimally patch v19" inheritance.
  Replaced by predicted-type routing. Deletes the corpus-pass-confidence prerequisite entirely.
- **Gated-YDF oracle as a *blocking* arbiter** — wrong ~42% on contested ground; this session it
  *inflated* the encoder NO-GO by scoring correct fixes of v19's errors as `oracle_fp`. Demoted
  to a directional sanity lens, not a promotion blocker.

## Gates kept (the truth, right-sized)

- **Gold + representative accuracy = the BLOCKING gates.** Human-verified truth on curated and
  uniform-random production columns. This is what told us the truth (0.811) when the curated
  harness and the YDF oracle both misled. Non-negotiable.
- **Corpus over-emission sanity check (directional, not blocking).** Does gte-tiny over-emit any
  label wildly at corpus scale, on its OWN predictions? Keep the *question* the corpus-honest gate
  asks; drop the vs-v19-YDF relocation apparatus. A big over-emission is a flag to investigate, not
  an automatic NO-GO.
- **Deterministic parsers** for parseable types — orthogonal, cheapest accuracy, ship independently.

## Steps (collapsed from 5 to 3 + ship)

1. **Fine-tune gte-tiny as the semantic classifier** — clean rebalanced training (mine real
   columns + vocab/distilled labels, region-fixed via admin2, entity from distilled not v19 calls,
   residual ~25%), the working v3 recipe (partial-unfreeze top-2, discriminative LR enc 2e-6/head
   1e-3, natural CE, grad-clip, warmup), **saved model**. Target: beat shipped on gold + representative.
2. **Wire the type-routed two-stage** and evaluate **on gold + representative** (truth) — the
   blocking gates — plus the corpus over-emission sanity check. No confidence dance.
3. **GO on human truth** → Rust integration: candle gte-tiny, type-routed, B07 audit, green CI, swap.

## Immediate next action

Start step 1 — the production fine-tune of gte-tiny on the clean v2 training set, saved, evaluated
on gold + representative. (The v3 recipe already trains cleanly to 0.811; this scales it to the
routed semantic label space and banks a saved model.) No corpus-pass-confidence work needed.

## Scope note

This slims the methodology **for the encoder path**. Generalising it (e.g. demoting the
corpus-honest gate from H05-blocking for all future work) is a separate decision — a choice/MADR —
not assumed here. The drift proxy and corpus-honest gate stay as-is for any future additive
multi-branch retrain, where they DO earn their keep.
</content>
