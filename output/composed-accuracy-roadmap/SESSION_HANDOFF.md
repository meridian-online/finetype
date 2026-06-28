# Session handoff → CLEAN-LABEL RETRAIN — DONE, thesis REFUTED

> **RESOLVED 2026-06-28.** The clean-label retrain RAN. **Verdict: NO-GO — training-label
> quality is NOT the accuracy ceiling.** Holding the shipped arch + Sharpen fixed and swapping
> geo/person labels to GeoNames/Wikidata clean positives gave composed gold (reframe) **0.845 ≈
> s43 0.853 (FLAT, within CI)** — clean labels don't move composed even for the semantic mass.
> The shipped model already saturates semantic gold (city 0.958, country_code 0.926, continent
> 1.000), so there was no headroom. 4th confirmation of `composed-is-rule-bound`. **Stop chasing
> data/labels for composed accuracy.** Full verdict: `output/clean-label-retrain/VERDICT.md`;
> memory `clean-label-retrain-refutes-data-ceiling`. The original hypothesis below is kept for
> the record.

---

_Updated 2026-06-28 (end of the model-label-space-reshape session)._

## Headline (ORIGINAL HYPOTHESIS — now refuted, see banner above)

The label-space reshape is **NO-GO** — and the post-mortem points at a bigger,
unexamined lever: **the training labels, not the model, are the suspected ceiling.**
Next session holds the model still and cleans the labels — the one experiment two
months of architecture work never ran.

## What this session settled (all committed + pushed)

- **Reshape (drop 134 validator-ownable leaves → 111-class model + recovery rule): NO-GO.**
  3-seed composed gold 0.811/0.832/0.792 (mean 0.812) vs s43 baseline 0.853 = −4.1pp
  mean, all below baseline — consistent, not variance. choice 0108 → rejected; memory
  `reshape-leaf-drop-costs-gold`; `output/model-label-space-reshape/VERDICT.md`.
- **Diagnosed precisely (not Sense↔Sharpen fighting):** the −2pp decomposes into −1.8pp
  kept-class model deficit (the forced 131k→87k training-data cut) + −3.1pp ceded
  recovery gap. Sharpen lifts both models equally; it never overrode a correct Sense.
- **The recovery rule (`ceded_leaf_recovery`) is gold-clean** (zero over-fire) but a no-op
  on the shipped 244-model — kept on the branch, not shipped.
- **Fast-path sized:** a value-only deterministic front-door skips the model on **~8% of
  corpus columns** (`fastpath_sizing.md`). Independent of the reshape; could ship alone.

## NEXT SESSION = clean-label retrain (run `/design` to scope the spec)

**The thesis (load-bearing):** the raw Sense ceiling (~0.52) is **stable across every
architecture** we've tried (potion-4M/8M/16M, two-view, attention, gte transformer:
0.50–0.57, oracle cap 0.599). A ceiling that doesn't move when you change the model is the
fingerprint of a **label/data ceiling**, not a model limit. And the data lever is already
**proven on raw Sense** (`encoder-data-lever-proven`, 2026-06-18): corpus-mined columns +
GeoNames/Wikidata **vocabulary-membership labels** lifted the contested *semantic* set from
the shipped 0.684 → **0.82–0.89** (GeoNames made city/country/country_code near-perfect).

**The gap (what's never been run):** that proof was always entangled with a transformer
encoder upgrade (which died: composed-tied, 100× latency) and measured on **raw Sense**, never
isolated on **composed gold** with the **shipped static architecture**. As far as the substrate
shows, the cell "static model + clean vocab labels + composed gold" is **empty**. Every big
experiment changed the *model* on top of the *same noisy distilled-Sherlock labels*.

**The experiment:** rebuild the training set with **vocabulary-membership labels** (GeoNames
geo, Wikidata person — generators exist: `scripts/generate_geonames_geography.py`,
`scripts/generate_wikidata_person_columns.py`) for the **semantic families**, **hold the
shipped architecture + Sharpen fixed**, retrain, measure **composed gold**. One retrain.
It isolates the variable every prior experiment confounded.

**The honest caveat:** *composed is rule-bound* has 3 confirmations — the skeptic's prior is
"Sharpen already compensates, composed won't move." BUT the data lever targets the **semantic
mass** (geography/person — open-vocab, *no validator*), which is exactly the bucket Sharpen
structurally CANNOT fix (ceiling-discovery called it "irreducible by the model"). That's the
one place clean labels could move composed where rules can't. **Decisive either way:** if
composed moves, the "model is the ceiling" framing flips; if not, the ceiling is proven
irreducible and we stop chasing the model for good.

## Author's two carry-over ideas (bake into the spec)

1. **Don't trust labels — sample-check continuously (training AND eval).** We've found large
   numbers of issues in gold over recent sessions (33 re-adjudications) and validator traps
   this session (color_hex passing bare numbers). Vocabulary-membership has its OWN failure
   modes — `region` collapses on GeoNames admin1 names (recall 0.07–0.33), and `[PA,TX,NY,CA]`
   correctly → residual but membership can mislabel. So make "pull the real values and check
   they make sense" a **gate before training on any label source**, not an afterthought — or
   we just swap distilled-noise for vocab-noise. Sample N columns per family, eyeball, fix.
2. **Keep an eye on inference speed — decoupled track.** Honest framing: **label count itself
   is NOT a speed lever** (softmax width is negligible — the "retired-240 trap"). The real
   levers are (a) the **~8% fast-path** (sized this session, ships independently), (b) the
   **free IO/encoder wins** in `next-train-research/RESEARCH.md` (single potion-4M encoder
   −20–40ms; batch-path taxonomy hoist; deterministic fast-path before model load), and
   (c) the architecture-speed connection to WATCH: **if clean labels let a SMALLER/simpler
   model hit the same composed accuracy, that's the real win.** So when running the clean-label
   retrain, also try a smaller encoder / fewer params and see if clean labels make it viable.

## Key memories to read at session start

`encoder-data-lever-proven` (the proof + the GeoNames/Wikidata recipe),
`determinability-probe-gold-is-the-ceiling` (gold-label quality + taxonomy gaps cap the score),
`composed-is-rule-bound` (the caveat), `sense-stage-ceiling-and-free-latency-wins`,
`reshape-leaf-drop-costs-gold` (why the model lever is closed),
`ceiling-discovery-both-levers-dead` (the semantic mass = the target bucket).

## Substrate

`output/fine-tuned-encoder-discovery/` (the data-lever proof: `contested_residual_mining_proof.md`,
`build_ac01_*.md`, `mining_proof.py`, `geonames_proof.py`), `eval/gold/lens_reference/`
(GeoNames files: cities15000, admin1CodesASCII, iso3166), `output/next-train-research/RESEARCH.md`
(the 0.60–0.66 abstaining-head alternative + the free latency wins),
`scripts/prepare_multibranch_data.py` (where the training blend + labels are built).

## First moves

1. `orbit session prime`; read this + the memories above.
2. `/design` a clean-label-retrain spec — ac-0 should be the **label-trust audit** (idea 1):
   sample-verify GeoNames/Wikidata vocab labels per family before any retrain.
3. The decisive cheap experiment: one static retrain on the clean-label blend for the semantic
   families → composed gold vs s43 0.853 (the go/no-go on "data is the ceiling").
4. Carry the speed track (idea 2) opportunistically — try a smaller encoder in the same retrain.
