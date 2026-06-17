# ac-03 — corpus-honest gate: GO (after isolation + a referee-vocabulary fix)

**Date:** 2026-06-17 · spec `2026-06-17-enum-accuracy-reframe` · blocking gate (H05)

## Verdict: GO (isolated, oracle read in the candidate's vocabulary)

The enum reframe (categorical retired → text.word) clears the corpus-honest gate with
zero triggers, once two confounds are removed.

## The two false alarms, and why they were false

**Run 1 (candidate vs the default v19_gated baseline): NO-GO — categorical + isbn.**
Both were artifacts of a STALE baseline. The default baseline is the May-2026 v19 pass
(finetype ~0.6.19); the candidate is 0.6.33, so the gate scored ~7 months of shipped
fixes as if they were the reframe.
- **isbn collapse (8,024→771):** logically impossible for a categorical→word remap to
  touch ISBN — it was the accumulated ISBN-checksum-guard fixes shipped since the baseline.
- **categorical collapse (67,864→0):** conflated 0.6.19's categorical volume with the
  reframe.

**Run 2 (ISOLATED: reframe-OFF 0.6.33 baseline vs reframe-ON 0.6.33 candidate): NO-GO —
categorical only.** The ONLY delta is categorical→word (the kill switch
`RHH_DISABLE_HINTS=enum_reframe_residual` reverts the whole reframe, so the off-pass is
true pre-0102). isbn vanished from triggers — PROVING it was the stale baseline. The
true reframe volume is 8,683 categorical (0.6.33 already emits far less than 0.6.19's 67k).
The relocation is oracle-clean: categorical shed −25,128 oracle-confirmed columns; word
gained +25,128 at correct_ratio 1.0, and word tripped NO band.

**The remaining categorical `collapse` was a referee-vocabulary mismatch.** The oracle
(gated YDF) still emits the abolished label `categorical`. Judging candidate("word")
against oracle("categorical") on the very columns the reframe defines as equivalent is a
tautological "contradiction", not an error — so the collapse band false-alarmed on an
intentional, correctness-preserving relabel. This is the symmetric twin of the `over_emit`
false alarm the gate already fixed (composition-aware band, 0.6.24/0.6.29).

## The fix: read the referee in the candidate's vocabulary

`corpus_honest_gate.py --label-remap OLD=NEW` applies the same label remap to the oracle
`y` before judging. Run 2 + `--label-remap representation.discrete.categorical=
representation.text.word` → **GO, zero triggers**.

- **Principled:** when a candidate retires/renames a label, the oracle (fixed label space)
  must be translated or it speaks a dead vocabulary.
- **Safe / narrow:** only the named label is neutralised; a genuine misclassification
  (oracle says `city`, candidate says `word`) is NOT remapped and still trips oracle_fp.
- **Default-empty → identity:** every other candidate is byte-identical. Four-verdict
  regression re-confirmed on the passes still on disk — **v19self GO, latdec NO-GO**
  (v22/v23 gated parquets cleaned off disk since the original run, but guaranteed
  unchanged: with no remap the SQL is byte-for-byte the prior gate).

## Bottom line

The reframe is corpus-honest-clean: its only corpus effect is categorical→word, the
oracle agrees word is correct for those columns (correct_ratio 1.0, no created FPs survive
the bands), and the gate's own logic returns GO once it judges in the post-reframe
vocabulary. Gold gate also clean (0.800, no regression). ac-03 gate **cleared**.
