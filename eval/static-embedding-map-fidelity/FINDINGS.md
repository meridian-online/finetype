# Does a static embedding change what a 2D map looks like?

**Answered 2026-08-23 by `map_fidelity.py`. Every figure below is read from `results.json`, which that script wrote and which is committed beside it. Re-run it rather than quoting this file from memory — vectors from two model versions are not comparable, so this question returns whenever a model is bumped.**

Run: seed 42, 3,000 rows per corpus, `umap.UMAP(metric="cosine", n_neighbors=15, random_state=42)` over a precomputed kNN graph — the reference stack's own settings — on `Darwin arm64 python3.12.12`.

| corpus | shape | embedder | AMI (map) | AMI (vectors) | retention | kNN overlap with MiniLM |
|---|---|---|---|---|---|---|
| 20news-body | long-form | all-MiniLM-L6-v2 | 0.5594 | 0.5237 | 1.000 | 1.0000 |
| | | potion-base-8M | 0.3974 | 0.3765 | **0.710** | **0.1318** |
| | | potion-base-4M | 0.3827 | 0.3685 | 0.684 | 0.1238 |
| | | random (control) | 0.0008 | −0.0010 | 0.000 | 0.0069 |
| 20news-subject | short | all-MiniLM-L6-v2 | 0.4495 | 0.4594 | 1.000 | 1.0000 |
| | | potion-base-8M | 0.3015 | 0.3309 | **0.667** | **0.2776** |
| | | potion-base-4M | 0.2982 | 0.3120 | 0.660 | 0.2644 |
| | | random (control) | 0.0047 | −0.0012 | 0.000 | 0.0066 |
| finetype-columns | very short | all-MiniLM-L6-v2 | 0.3687 | 0.3510 | 1.000 | 1.0000 |
| | | potion-base-8M | 0.3195 | 0.3924 | **0.875** | **0.4030** |
| | | potion-base-4M | 0.3193 | 0.3929 | 0.875 | 0.3859 |
| | | random (control) | −0.0255 | 0.0053 | 0.000 | 0.0924 |

`retention` is `(AMI − AMI_random) / (AMI_MiniLM − AMI_random)`: the share of the transformer's recoverable cluster structure the static model keeps, measured above a noise floor rather than above zero.

## The answer, in the words the goal uses

**A 2D map built from static vectors shows *most* of the cluster structure a hosted transformer would have shown — between two-thirds and seven-eighths of it — and it is never noise. But it does not show *the same map*.**

Both halves are load-bearing and they point in different directions, so neither should be quoted without the other.

**Cluster structure substantially survives.** On every corpus the static map recovers a large majority of the class structure the transformer map recovers, and the control is the reason that sentence means anything: seeded Gaussian noise scores −0.03 to 0.005, so the measurement can plainly tell an embedder that knows nothing from one that does. A claim that static embeddings *destroy* the picture is refuted here.

**Parity is refuted too, and by the same numbers.** The gap is 12.5 to 33 points of retention, it appears on all three corpora, and it is largest in absolute terms on long-form text — the transformer's AMI falls 0.162 on 20news bodies against 0.049 on column names. That is the direction the "no contextual attention" account predicts: the more context there is to attend to, the more is lost by not attending to it.

## The finding nobody was looking for, and it is the one that changes the product claim

**The two maps agree on regions and disagree on neighbourhoods.** The kNN-overlap column is label-free: it asks what fraction of a point's 20 nearest neighbours *in the picture* are the same across the two maps. It is **0.13 on long-form text**, 0.28 on short text and 0.40 on column names.

So even on `finetype-columns`, where the static model keeps 87.5% of the cluster structure and actually **beats MiniLM in the raw vector space** (0.3924 against 0.3510 — the projection, not the embedder, is where it loses), fewer than half of any point's visual neighbours are the same points.

**That splits the product claim in two.** An analyst who reads a static map at the level of *regions* — "these rows are one thing, those are another" — gets substantially what the transformer would have given them. An analyst who reads it at the level of *this point and the ones beside it* — nearest-neighbour lookup, "show me rows like this one" — does not. Those are different features and they should not be sold on one measurement.

## What this does not say

- **Nothing about speed.** Deliberately not measured here; mixing it in invites the embedding-only figure into a claim it does not support.
- **Nothing about retrieval.** The published static-embedding gap is a retrieval gap and this harness does not re-measure it.
- **Nothing about a corpus unlike these three.** Two of the three share a label set — 20 Newsgroups bodies and their own Subject: lines — which is what isolates text length from domain, and is also why domain generality is not something this run establishes.
- **`finetype-columns` is 216 rows in 12 classes**, after classes with fewer than five members were dropped. It is the most product-relevant shape here and the least statistically comfortable one; treat its figures as indicative and the two 3,000-row corpora as the evidence.

## Reproducing it

```sh
uv run --script eval/static-embedding-map-fidelity/map_fidelity.py \
  --limit 3000 --out eval/static-embedding-map-fidelity/results.json
```

Roughly half an hour on an M1 Pro, most of it MiniLM over 20news bodies. `--corpus finetype-columns --limit 300` is the fast path for checking the harness still runs.
