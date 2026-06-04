# Decision memo — value-level YDF labelling head-to-head

**Headline: INCONCLUSIVE. We could not answer the architecture question,
because cleaned real gittables does not contain enough distinct values of the
rare types to train or test a value-level model on them. The experiment's
premise — "clean real data" — collides with the reality that cleaning real data
leaves 10 distinct latitudes in an 18-million-row set. This is a corpus-supply
finding, not an architecture verdict.**

We set out to settle one question: when v14's value-only CharCNN drove latitude
false-positives to zero, was that a genuine architecture win, or an artefact of
training on clean synthetic data? We rebuilt the experiment on cleaned *real*
gittables data, with a dirty (un-cleaned) control trained on the same recipe,
and scored both against the shipped v19 default.

## Why the answer is INCONCLUSIVE, not NO-GO

An earlier draft of this memo read the result as a NO-GO: latitude collateral
stayed at 0, but the dirty control reached 0 too and net agreement with v19
collapsed to 38%, so "cleaning lost to not-cleaning". That comparison still
holds — same cap, same seed, only the cleaning differs — but it was the wrong
headline. Two facts overturn it.

**1. The model was never shown the rare types.** Counting *distinct* values per
type in the set we actually trained on, **66 of 159 types fall below a
50-distinct floor**. Not latitude alone — h3 (1 distinct), gender_code (2),
longitude (24), ssn (23), and 62 others. A model shown 10 distinct latitudes
cannot tell us whether the value-level architecture learns latitude; it can only
tell us it wasn't taught. latitude=0 collateral is suppression by starvation, not
by discrimination — and starvation is a data verdict, not an architecture one.

**2. Scaling the training set cannot fix it.** The obvious objection is "we
under-capped — lift the cap". We checked: across the **full 18M-row kept set
before any cap**, the distinct counts are identical to the capped set. Of the 66
starved types, **zero** would clear the floor if the cap were lifted. The cap was
never binding. The distinct values do not exist in the corpus:

| type | full kept-set rows | full distinct | capped distinct |
|---|---|---|---|
| geography.index.h3 | 1 | 1 | 1 |
| geography.coordinate.latitude | 10 | 10 | 10 |
| geography.coordinate.longitude | 24 | 24 | 24 |
| identity.government.ssn | 23 | 23 | 23 |
| identity.person.gender_code | 7,437 | 2 | 2 |

gender_code is the clearest tell: 7,437 rows, 2 distinct values. Row count is not
the lever; value diversity is, and the corpus does not carry it.

## Two compounding walls — neither fixed by more rows

1. **gittables is already fully consumed.** The corpus pass ran the whole
   corpus; this kept set IS the corroborated subset of all of it. There is no
   more gittables to scale into.
2. **The cleaning structurally evicts the rare types.** The triad's
   quarantine-first policy keeps latitude-looking values under their columns'
   (non-latitude) labels. Real latitude columns mislabelled at column level —
   most of them in gittables — get quarantined out. The de-noising we built is
   exactly what starves these types.

So reaching 1,000 distinct latitudes would need a corpus that contains them
cleanly-labelled (gittables does not), or relaxing the cleaning (which
reintroduces the noise this spec existed to escape), or synthetic values (the
exact confound v14 was accused of and this spec set out to rule out). None of
those is "scale the training data".

## On the clean-vs-dirty comparison

The clean (38% agreement) < dirty (49% agreement) result is real and survives —
same instrument, same reference, same files. But it is not load-bearing for the
architecture question, because both arms were trained on the same starved value
pool. "Cleaning did not beat not-cleaning" is true of *these starved models* and
says nothing about a value-level model trained on a corpus that actually contains
the rare types. The agreement-with-v19 number was also never a precision measure:
it conflates "the model is wrong" with "the model disagrees with v19", and v19 is
not ground truth.

## What this means for the late-fusion spec

We are not drafting it, but **not** because the architecture failed — because the
experiment could not test it. The honest precedent decision (the Precision
Principle) is to measure validation-gated pass-rate per predicted type, on a data
source that contains the rare types at diversity. That is the follow-up, and it
needs a different data source than cleaned gittables.

## Quarantine / label-error rate

First-pass triad flagged **30,100 columns** (194,765 value-rows) into the
quarantine list. **Do not move quarantine → auto-relabel.** The architecture
question is unresolved; auto-relabelling on a signal this experiment could not
validate would compound an untested step. Keep quarantine-first.

## What we don't know yet

- Whether a value-level model with a **genuine** rare-type floor (met, not
  starved) discriminates rather than suppresses. This experiment could not test
  it; gittables' quarantine-first cleaning structurally starves the rare types.
- The true gated cell-2 / validation-pass precision of a value-level CharCNN, as
  opposed to the v19-agreement proxy used here.

Both require a follow-up with a rare-type data source that carries diversity —
not more gittables, and not a bigger cap.

---

**For a stakeholder:** We tested whether last month's "zero false-latitude"
CharCNN result holds on real data. It does — but only because cleaned real data
contains barely any latitude to get wrong (10 distinct examples in 18 million
rows), and 66 of our 159 types are similarly starved. The bottleneck is not the
model and not the amount of data we sampled — it is that the public table corpus,
once cleaned, simply does not contain enough cleanly-labelled examples of the
rare types to teach or test them. The result is inconclusive, and the next
attempt needs a different source of training values, not more of the same.
