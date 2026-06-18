# ac-04 — does the representative fixture earn its keep?

**Spec:** 2026-06-18-representative-accuracy-gate (GATE, observation)
**Date:** 2026-06-18 · all numbers `score_gold_anchor.py … --reframe`, binary 0.6.34

## The measurement

Two candidates, both instruments, same tooling:

| candidate | gold | repr | gold Δ | repr Δ |
|---|---|---|---|---|
| v19 (shipped) | 739/927 = **0.797** | 179/259 = **0.691** | — | — |
| s44 (identity retrain, gold-NO-GO'd 2026-06-18) | 726/927 = **0.783** | 177/259 = **0.683** | −1.4pp | −0.8pp |

## Verdict — outcome (b), with a sharp nuance

**On the one available NO-GO candidate, the representative band TRACKS gold rather
than carrying independent detection — and is the *weaker* detector of the two.**

- Both instruments move the same direction (down). No divergence.
- The repr drop (0.008) is **well inside the ±6pp CI**, so the advisory flag would
  **not fire** on s44. Gold caught the regression clearly (−13 columns); repr lost
  2, indistinguishable from noise.
- Mechanism: s44's regression is the full_name catch-all broadening — a
  regression concentrated on *hard boundary* columns (place/org names mislabelled
  full_name). Gold is a curated-*hard* slice that over-samples exactly those
  columns, so it is the sharper instrument for this failure mode. The uniform-random
  representative draw under-samples them, so it sees the regression only faintly.

**Per ac-04(b): the fixture is a cheaper-to-trust corroborator, not a unique
detector — and the advisory framing (ac-00) is the honest ceiling on its
authority.** I did not manufacture a divergence; on today's only NO-GO candidate a
clean one does not exist (the spec anticipated this).

## But the fixture still earns its keep — for LEVEL, not delta

The detection role is unproven; the **level** role is proven and is what card 0020
scenario 1 actually asks for ("the headline you're shown is the headline you get"):

- **repr 0.691 vs gold 0.797 — a standing 10pp gap.** Reporting both at every
  promotion makes visible that the model is ~0.69 on a random production column,
  not the ~0.80 the curated headline implies. That honesty is the deliverable;
  it required this fixture to exist and is independent of the delta question.
- The gap is not an artefact: v19 and s44 both show it (0.797/0.691 and
  0.783/0.683), so it is a stable property of the population difference, not noise.

## Net

- **Keep the fixture for LEVEL reporting** — proven, and it is scenario 1's goal.
- **Hold the advisory-flag DETECTION role as unproven** — on the only NO-GO
  candidate it was the weaker, noise-level detector. Its authority stays advisory
  (never blocking); revisit if a future candidate regresses on *common* columns
  gold under-samples, which is the failure mode where representative would lead.
- **The honest one-liner:** the representative fixture's job is to keep the
  *reported* headline production-true, not to be a sharper regression alarm than
  gold — gold + corpus-honest remain the blocking gates.
</content>
