# Decision — v22 becomes the new default Sense model

Per spec `2026-05-26-v22-gated-direction-review` ac-03.

**Pick: Option A.** Promote `sherlock-v22-boundary-relu-s44` to
`models/default`. Update CLAUDE.md to reflect v22 as the headline
Sense-stage model. Return R&D budget to m-19 (eval-corpus
expansion) without another retrain.

**Pillar served.** Pillar 1 (analyst experience). The four
monotone-movers — country (−31.5%), region (−12.8%), city (−10.2%),
longitude (−14.3% on n=7) — absorb 95% of v19 cell-2 misses
(71,690 / 75,442). Analysts who profile geography-heavy datasets
see a step-change in country accuracy and material lifts on
region/city *now*, without paying for another retrain cycle.

**Load-bearing evidence.** Per `per_subtype_trajectory.md`: the
v19→v20→v21→v22 ratchet is monotone on the dominant subtypes
(country, region, city, longitude). There are no v22-jumpers —
the recipe works as a campaign, not as a v22-only spike. The flat
subtypes (full_address, street_name, postal_code) didn't respond
to any of v20/v21/v22 and won't respond to a same-recipe retrain;
their bottleneck is not boundary blend.

**Alternatives not taken.**

- *Option B (v22+ patch retrain).* Blocked by m-19's retrain block.
  The block applies to the eval regime (no train/eval leakage
  firewall, no 240/240 coverage), not the model family — a v22+
  patch is a v18+ retrain. Even setting m-19 aside, no mechanism
  is in evidence for why a same-recipe reshuffle would convert
  flat subtypes into monotone-movers.
- *Option C (data-composition rebalance).* Partially blocked by
  m-19 (mining unblocked, retrain step blocked) and lacks the
  diagnostic naming which of thin-volume / locale-skew / header-
  thinness explains the flat subtypes. Worth picking up only with
  that diagnostic in hand, after m-19 ships.

**Soak.** Per ac-05, the gated baseline's calibration is
conservative (50% threshold, length-only validations skipped); the
next 2–3 corpus passes will reveal whether v22's gated band
(−10.4%) is stable or drifts outside [−8%, −13%]. The decision
here is robust to drift in that range — country's −31.5% lift sits
well outside the band's calibration sensitivity.
