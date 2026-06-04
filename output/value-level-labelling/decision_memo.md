# Decision memo — value-level YDF labelling head-to-head

**Headline: the latitude win is real but trivially won, and the cleaning that was
the whole point of this spec did not beat doing nothing. NO-GO on the late-fusion
spec for now.**

We set out to settle one question: when v14's value-only CharCNN drove
latitude false-positives to zero, was that a genuine architecture win, or an
artefact of training on clean synthetic data? We rebuilt the experiment on
cleaned *real* gittables data, with a dirty (un-cleaned) control trained on the
same recipe, and scored both against the shipped v19 default.

## What the data tells us

**1. Latitude=0 survived on real data.** The cleaned CharCNN
(`char-cnn-v15-gittables`) produced **0 latitude collateral** and **1 utc
collateral** across 1,871 columns, versus v19's footprint of 1 latitude and 10
utc. On the narrow question "does the latitude-suppression vanish once you leave
synthetic data?", the answer is no — it holds. v14 was not a synthetic-only
mirage in that literal sense.

**2. But the control got the same result, for free.** The dirty CharCNN
(`char-cnn-v15-dirty`), trained on the un-cleaned blend, *also* produced 0
latitude and 1 utc collateral. The corroboration triad — the cleaning machinery
this entire spec exists to build and test — bought **nothing** on the disease.
Whatever suppresses latitude is shared by the un-cleaned model.

**3. And it suppresses latitude by suppressing almost everything.** The cleaned
model agrees with v19 on only **38%** of columns (the dirty model: 49%). The
disagreement is **broad** — 1,160 of 1,870 columns differ on non-disease types,
not a narrow band of column-context-dependent types. The latitude=0 is the
trivial face of this: the value-level CharCNN barely predicts latitude on
*anything* (latitude footprint 0), so of course it never predicts it wrongly.
Suppression by starvation, not by discrimination.

This was the caveat flagged at ac-03 and carried forward: the triad's
quarantine-first policy keeps latitude-looking values under their columns'
non-latitude labels, leaving only 10 distinct latitude survivors in the kept
set. A model that never learns to say "latitude" cannot say it wrongly — but it
also cannot say it rightly.

## Which win-condition bar does this hit?

**None cleanly — and the confound control is what breaks the tie toward NO-GO.**

- *Bar 1 (clean win — latitude≈0 AND no net regression):* fails. Net agreement
  with v19 collapses to 38%.
- *Bar 2 (validated, dip confined to column-context types):* fails. The
  regression is broad, not confined; this is not a bounded Model2Vec-branch job.
- *Bar 3 (latitude collateral returns → synthetic artefact, NO-GO):* the literal
  trigger is not met — latitude did not return, it stayed at 0.

The result sits off the anticipated decision tree. The pre-committed bars
imagined the failure mode would be "latitude comes back". The actual failure mode
is subtler and is only visible *because* the dirty control was run: latitude
stays at 0, but the control reaches the same 0 with higher net agreement, so the
cleaning method demonstrably did not win on the messy data v19 had. That is the
confound control doing exactly its job.

## Cost in column-context-dependent types (the Model2Vec job)

Not measurable as a bounded number here, and that is itself the finding. We
expected the loss to concentrate in header/sibling-context-dependent types — the
gap a future late-fusion (Model2Vec header branch) spec would close. Instead the
loss is broad. A value-level CharCNN used as a column profiler is a weak column
profiler across the board, not a strong one with a localised context blind spot.
Late-fusion cannot rescue a 38%-agreement base; it is a refinement, not a
foundation.

*Instrument caveat, stated honestly:* agreement-with-v19 is a proxy, not gated
cell-2 accuracy — v19 is not ground truth, and the value-level model is being
applied to a column-level task it was not built for, so the absolute 38%
understates true quality somewhat. But the load-bearing comparison is
*relative*: clean (38%) < dirty (49%), same instrument, same reference, same
files. Cleaning lost to not-cleaning. That comparison is robust to v19's own
imperfection.

## Quarantine / label-error rate

First-pass triad flagged **30,100 columns** (194,765 value-rows) into the
quarantine list — columns where YDF confidently predicted a different label that
validated better than the weak column label. **Do not move quarantine →
auto-relabel.** With the architecture itself NO-GO, auto-relabelling on a signal
this experiment just showed does not produce a winning model would be compounding
an unvalidated step. Keep quarantine-first.

## Go / no-go on the late-fusion spec

**NO-GO, for now.** The value-only CharCNN's latitude suppression is real but
trivially achieved (starvation, reproduced by the dirty control) and bundled with
a broad column-level regression that late-fusion is not positioned to fix. The
corroboration triad did not beat its own un-cleaned control. Drafting a
late-fusion spec on this foundation would be building on a 38%-agreement base.

## What we don't know yet

- Whether a value-level model with a **genuine** latitude class (floor met, not
  starved) would discriminate rather than suppress — this experiment could not
  test it, because gittables' quarantine-first cleaning structurally starves
  latitude. Answering it needs a different data source or a relabelling step this
  memo just argued against.
- The true gated cell-2 accuracy of these models, as opposed to the v19-agreement
  proxy. Wiring a value-level CharCNN into the column-level gated pipeline is a
  separate build; the relative clean-vs-dirty finding did not require it.

---

**For a stakeholder:** We tested whether last month's "zero false-latitude"
CharCNN result was a real architecture win or a fluke of clean training data. It
reproduces on real data — but only because the model learned to almost never
predict latitude at all, the un-cleaned control does exactly the same thing, and
both agree with our shipped model on under half of columns. The cleaning step we
built bought nothing. We're not drafting the follow-on late-fusion spec yet.
