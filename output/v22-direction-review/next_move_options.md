# Next-move options — post-gated baseline

Per spec `2026-05-26-v22-gated-direction-review` ac-02. Reads the
trajectory at `per_subtype_trajectory.md`; informs the decision at
`decision.md`.

## What the trajectory tells us

- Four monotone-movers: city (−10.2%), region (−12.8%), country
  (−31.5%), longitude (−14.3% on n=7). The boundary-training campaign
  is doing what it was designed to do — v19→v20→v21→v22 ratchets
  consistently on the geography subtypes that matter by volume.
- No v22-jumpers. The v22 recipe is not load-bearing on its own —
  v22 continues a trend v20/v21 started. The Model2Vec embed-branch
  contribution (per ac-06 of the v22 spec) is necessary but not the
  sole driver.
- Nine flat subtypes. full_address, street_name, postal_code did not
  respond to any of v20/v21/v22. Their bottleneck is not boundary
  blend.
- One regressed: iata_code (+5 columns; n=6 baseline). Sample
  vanishingly small; not load-bearing.

Headline: the recipe works on the bulk subtypes (the four monotone-
movers absorb 71,690 / 75,442 = 95.0% of v19 cell-2 misses) but is
exhausted on the long tail.

## Option A — v22 as default, return R&D to m-19

**What it is.** Promote `sherlock-v22-boundary-relu-s44` to
`models/default`; update CLAUDE.md's headline number; close this
spec; return R&D budget to m-19 (eval-corpus expansion) which is
the current sprint.

**Pillar served.** Pillar 1 (analyst experience) — analysts get a
+31.5% lift on country, −12.8% on region, −10.2% on city *now*
without paying for another retrain cycle. Pillar 4 (long-running
R&D) — m-19's firewall+coverage work is the bottleneck for every
future Sense retrain; finishing it unblocks the whole pipeline.

**Falsifiable signal that would justify it.** The four monotone-
movers cover 95% of v19 cell-2 misses; the unaddressed 5% sits in
flat subtypes whose bottleneck is not boundary blend. There is no
load-bearing reason to spend another retrain budget before m-19
ships.

**Cost.** Symlink swap + CLAUDE.md update + one commit. Effectively
zero R&D spend.

**What it deprioritises.** The long-tail subtypes (full_address,
street_name, postal_code) stay where they are until a different
intervention class is identified. iata_code regression unaddressed.

## Option B — v22+ patch retrain, extend boundary blend

**What it is.** Same multi-branch recipe, more hard negatives
targeting the flat subtypes (full_address, street_name,
postal_code). Goal: convert flat → monotone-mover for some of those.

**Pillar served.** Pillar 1, if the patch lands.

**Falsifiable signal that would justify it.** Either (a) a credible
mechanism explaining why the flat subtypes didn't respond to v20/
v21/v22 and how a blend reshuffle would change that, or (b)
evidence that the m-19 block doesn't apply to a patch (it does —
see below). Neither holds today.

**Cost.** Overnight train + corpus pass + eval = ~1–2 spec-weeks.

**What it deprioritises.** m-19 Phase A+B. Also: explicitly blocked
by m-19's retrain block, which applies to the eval regime (no train/
eval leakage firewall, no 240/240 coverage), not the model family.
A v22+ patch retrain is a v18+ retrain. Not shippable until m-19
ships.

## Option C — data-composition rebalance for the long tail

**What it is.** Targeted source mining for the flat subtypes. The
hypothesis: full_address / street_name / postal_code didn't respond
because the training distribution doesn't reflect the corpus
distribution for those types — header thinness, locale skew, or
volume shortfall.

**Pillar served.** Pillar 1, downstream — but only after a retrain
to consume the new data.

**Falsifiable signal that would justify it.** A diagnostic showing
which of the three explanations (thin volume, locale skew, header
thinness) holds for the flat subtypes. We don't have that
diagnostic.

**Cost.** 2–3 spec-weeks of mining + a retrain. Mining is partially
unblocked by m-19 (sources.yaml work overlaps), retrain is not.

**What it deprioritises.** m-19's coverage floor and firewall work,
both of which need to ship before the retrain step lands anyway.

## Option D — eval-corpus expansion blocking everything

**What it is.** Pause Sense work entirely; land m-19 Phase A+B
before any v22+ retrain. This is the current sprint's stated
direction (m-19 IN FLIGHT, retrain block explicit in CLAUDE.md).

**Pillar served.** Pillar 4 — m-19 is the long-running R&D bet
that pays back across every future Sense retrain.

**Falsifiable signal that would justify it.** Already justified by
m-19's spec and the existing retrain block; this option is the
status-quo direction.

**Cost.** Zero additional spend — this is what m-19 is already
doing.

**What it deprioritises.** Nothing that should be prioritised
above it.

## Synthesis

Options B and C are blocked by m-19 (which the user confirmed: the
retrain block applies to a v22+ patch retrain; it's about the eval
regime, not the model family). Option D is the current sprint's
direction — running already. Only A is a deliberate act left to
take in this spec's window. It collapses the open v22 question
("does this become the default?") without competing with m-19.

**Pick: A.**
