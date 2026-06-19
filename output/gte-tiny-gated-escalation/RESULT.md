# gte-tiny gated escalation — the model-side breakthrough (2026-06-19)

**Claim proven:** the gte-tiny encoder NO-GO (−5.8pp representative, June) was an artefact
of the *type-routed* regime (re-decide ALL contested columns). A **two-sided confidence gate**
— override v19 only where gte-tiny is confident (≥0.95) AND v19 was uncertain (quality_band
∈ {low,medium}) — converts it into a clean improve-or-hold on the blocking gates.

| Regime | Gold (base 740/927=0.798) | Representative --reframe (base 179/259=0.691) |
|---|---|---|
| type-routed (override all contested) | 0.796 | 0.633 (−5.8, NO-GO) |
| one-sided gate (gte≥0.95) | 0.808 | 0.680 (−1.1) |
| **two-sided gate (gte≥0.95 ∧ v19∈{low,med})** | **0.809 (+1.1, +10 cols)** | **0.691 (flat)** |

**Why it works (165:4 witness quality):** on 191 gold columns where gte-tiny disagrees with
v19, gte-tiny is right 165 / v19 right 4 (98%). The char-CNN value-expert that killed the
2026-06-08 deferral thesis was right on 3.6% — gte-tiny is 27× the witness.

**Inputs (all pre-existing on disk):** checkpoint gte_tiny_v2.pt (slim-plan v3 recipe, 8
contested families); v19 per-column confidence from output/calibrated-confidence/calib_{gold,repr}.tsv
(the "sense_confidence NULL" blocker only applied to the CORPUS parquet, not gold/repr);
v19 preds from output/ceiling-and-rules-discovery + representative-accuracy-gate.

**Override counts (two-sided, lowmed/0.95):** gold 66/215 contested, repr 19/80 — far below
the type-routed 215/80, so the corpus relocation surface shrinks proportionally.

## Remaining to a shippable candidate
1. Corpus over-emission check on the gated escalation (smaller surface than the NO-GO version).
2. Retrain the gte head on the POST-0102 label space (categorical retired today → residual
   attractor that caused the word over-emission is structurally gone) + widen beyond 8 families.
3. Rust/candle integration: confidence-gated gte-tiny escalation, B07 audit, green CI, swap.

Reproduce: output/gte-tiny-gated-escalation/two_sided_gate.py (uses .venv in fine-tuned-encoder-discovery).

## Corpus over-emission check (2026-06-19) — YELLOW, mechanism proven, artifact not ship-ready

One-sided gate (gte conf only; STRICTER than the two-sided gate — no v19-uncertain filter
because sense_confidence is NULL in v19_gated.parquet and the corpus source files are not
local to re-profile) on the 33k stratified sample's 100,277 contested columns:

| tau | overridden | word ratio | top drains |
|---|---|---|---|
| 0.90 | 28% | 2.08x (+25,285) | categorical -8.1k, entity -7.0k, city -2.9k |
| 0.95 | 23% | 1.89x (+20,808) | categorical -6.7k, entity -5.2k |
| 0.98 | 16% | 1.65x (+15,179) | categorical -5.2k, entity -3.0k |

The residual (word) over-emits 1.65-2.08x — the same attractor signature as the June NO-GO.
Tightening tau alone does not contain it. Cause: this is the OLD slim-plan head (RESIDUAL =
biggest training class, 6,348). June's rebuild (residual 44%->25%) already cleared this band;
choice 0102 (categorical retired today) removes the categorical->residual pressure (categorical
is the largest single drain). NOT ship-ready as-is.

## Updated path to ship
1. DONE: two-sided gate improve-or-hold on gold+repr (mechanism proven).
2. DONE: corpus over-emission check — caught residual over-emission in the OLD head (this file).
3. NEXT: retrain the gte head on the post-0102 label space with rebalanced residual (~25%),
   widen past the 8 contested families. Re-run this corpus check; target word ratio ~1.0.
4. Persist v19 sense_confidence on the corpus (infra) -> definitive TWO-SIDED corpus gate.
5. Rust/candle integration, confidence-gated, B07 audit, green CI, swap.

## Clean-slate decider probes (2026-06-19) — full label space

**Full-label linear probe (fullprobe.py):** raw gte-tiny + linear head reproduces v19 at
0.716 across 218 labels (structural 0.73, semantic 0.65). Conservative FLOOR (raw+linear;
v19-as-truth penalises gte-tiny on the contested residual types where it is actually better).
The one real structural gap is cardinality/sequence identifiers (alphanumeric_id 0.42,
increment 0.43) — a column-aggregate signal a text encoder cannot see; full-column stats fixes it.

**Char-CNN decider (charprobe.py):** cheap hand-crafted char/stats features lift alphanumeric_id
only +0.05 (0.42->0.47). Char-shape is NOT the bottleneck — cardinality is, and a char-CNN reads
values not column aggregates, so it is blind to the one gap it might justify. => RETIRE char-CNN.

**Verdict:** clean slate = parsers + gte-tiny + full-column stats head. Three components, each
earning its place; char-CNN / Model2Vec / header branches / v19 all retired. See choice 0103 +
spec 2026-06-19-gte-tiny-clean-slate-build.
