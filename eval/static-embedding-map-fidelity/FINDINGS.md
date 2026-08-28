# What does a static embedding cost, and which property of the model is charging?

**Answered 2026-08-28 by `map_fidelity.py`, in one pass over three corpora at seed 42. Every figure below is read from `results.json`, committed beside it — the table is *generated* from that file by `check_results.py --emit-table`, and CI refuses a mismatch, so a number here cannot outlive the run that produced it. Re-run rather than quoting from memory: vectors from two model versions are not comparable, so this question returns whenever a model is bumped.**

Run: seed 42, 3,000 rows per corpus and 216 for `finetype-columns`, 800 probes per corpus and 216 for `finetype-columns`, `umap.UMAP(metric="cosine", n_neighbors=15, random_state=42)` over a precomputed kNN graph — the reference stack's own settings — on `Darwin arm64 python3.12.12`.

## The one-line answer

**Neither of the two readings on offer, and the third one this document first reached is also wrong.** Vocabulary and the retrieval objective are each real and each close part of the *ranked* gap — together 31%, 58% and 32% of it on the three corpora. But the full ladder from `potion-base-8M` to `potion-retrieval-32M` barely moves agreement with MiniLM's neighbour picks, **and it barely moves it at both stages**: +0.028 / +0.005 / −0.017 on the raw vectors, against +0.029 / +0.009 / −0.003 on the 2D map. So getting the coarse class right and matching another model's specific neighbours are different targets, and nothing in this ladder moves the second. Which property is charging for the *ranked* gap is answered — size, vocabulary and objective, split below. Which property costs the neighbourhoods is **not answered by this run**.

> [!warning]- This section claimed the projection was the cause, and that was wrong
> The first version read: *"the neighbourhood weakness is a property of the 2D projection, not of the embedder … a cause that lived in the embedder would move both."* It compared `map_overlap_with_minilm` (2D, agreement with MiniLM's specific neighbour picks) against `ami_vectors` (full-dimension, agreement with ground-truth labels) and attributed the gap between them to the projection stage. Those are different **questions**, not only different stages, so the comparison could not attribute anything to a stage.
>
> Caught by the lane's independent verifier, which supplied the missing measurement using this file's own functions. `vector_overlap_with_minilm` — the same metric one stage earlier — is now computed by the harness, and it tracks the 2D figure closely on all three corpora while sitting about 0.10–0.11 higher. The projection costs a roughly constant slice of overlap and does not create the flatness.
>
> The general form, worth recognising elsewhere: **an asymmetry between two stages attributes to a stage only if the same metric is taken at both.** `map_overlap` was called once, on the UMAP output, and the full-dimension vectors were discarded at the end of each arm — so the code made the wrong comparison the convenient one.

## What was varied, and why these four models

| model | dim | vocabulary | whole-word | varies |
|---|---|---|---|---|
| `potion-base-4M` | 128 | 29,528 | 23,700 | — |
| `potion-base-8M` | 256 | 29,528 | 23,700 | size |
| `potion-base-32M` | 512 | 63,091 | 57,263 | size **and** vocabulary |
| `potion-retrieval-32M` | 512 | 63,091 | 57,263 | training objective **only** |

`potion-retrieval-32M` is `potion-base-32M` fine-tuned on a retrieval objective. Same base, same tokenizer, same vocabulary, same Model2Vec on-disk format, and `results.json` records a fingerprint of each arm's output so "same tokenizer" is checked rather than asserted. The 32M pair therefore varies one thing, which is what makes the split below an attribution rather than a correlation.

The earlier run compared `potion-base-8M` and `potion-base-4M`, which differ in size alone. Both are distillations of a general sentence encoder with no retrieval objective, so it could not separate *static embeddings are weak at neighbourhoods* from *distillation without a retrieval objective is weak at neighbourhoods*. That is why this run exists.

## The measurement

Three questions, kept apart because this model class is strong at one and weak at another, and quoting either alone misdescribes it. `AMI (map)`, `retention` and `kNN overlap` read the 2D picture. `P@k` and `lift` are ranked retrieval over the **full-dimension vectors**, k=10. `AP same-class` and `AP near-dup` are pooled average precision under one global threshold — pairwise, deliberately not per-query, because per-query normalisation would turn them back into the ranked question.

**Two floors, and they do not sit at the same place.** `random-384` embeds nothing. `bm25` is DuckDB's `fts` extension over the same corpora — what an analyst has without any model, so a dense arm that does not beat it has not earned its download. AMI and kNN overlap floor at ~0; `P@k` floors at the class prior, so read `lift`, which normalises against the measured control; pooled AP floors at 0.5, fixed by one positive and one negative pair per anchor.

<!-- generated table: check_results.py --emit-table -->
| corpus | arm | AMI (map) | retention | kNN overlap | P@k | lift | AP same-class | AP near-dup |
|---|---|---|---|---|---|---|---|---|
| 20news-body | `minilm` | 0.5594 | 1.000 | 1.0000 | 0.5785 | 1.000 | 0.8108 | 0.9998 |
| 20news-body | `potion-4m` | 0.3827 | 0.684 | 0.1238 | 0.4403 | 0.740 | 0.7068 | 0.9991 |
| 20news-body | `potion-8m` | 0.3974 | 0.710 | 0.1318 | 0.4527 | 0.763 | 0.7016 | 0.9992 |
| 20news-body | `potion-32m` | 0.4285 | 0.766 | 0.1429 | 0.4750 | 0.805 | 0.7161 | 0.9994 |
| 20news-body | `potion-retrieval-32m` | 0.4467 | 0.798 | 0.1603 | 0.4916 | 0.837 | 0.7551 | 0.9999 |
| 20news-body | `random-384` | 0.0008 | 0.000 | 0.0069 | 0.0471 | 0.000 | 0.5003 | 0.5207 |
| 20news-body | `bm25` | -- | -- | -- | 0.4436 | 0.746 | 0.6143 | 0.9971 |
| 20news-subject | `minilm` | 0.4495 | 1.000 | 1.0000 | 0.5544 | 1.000 | 0.7756 | 0.9984 |
| 20news-subject | `potion-4m` | 0.2982 | 0.660 | 0.2644 | 0.4684 | 0.830 | 0.7069 | 0.9924 |
| 20news-subject | `potion-8m` | 0.3015 | 0.667 | 0.2776 | 0.4801 | 0.853 | 0.7117 | 0.9947 |
| 20news-subject | `potion-32m` | 0.3426 | 0.760 | 0.2921 | 0.5101 | 0.912 | 0.7413 | 0.9956 |
| 20news-subject | `potion-retrieval-32m` | 0.3197 | 0.708 | 0.2861 | 0.5234 | 0.939 | 0.7213 | 0.9964 |
| 20news-subject | `random-384` | 0.0047 | 0.000 | 0.0066 | 0.0493 | 0.000 | 0.4745 | 0.5207 |
| 20news-subject | `bm25` | -- | -- | -- | 0.3997 | 0.694 | 0.5325 | 0.9362 |
| finetype-columns | `minilm` | 0.3687 | 1.000 | 1.0000 | 0.5519 | 1.000 | 0.6815 | 0.9434 |
| finetype-columns | `potion-4m` | 0.3193 | 0.875 | 0.3859 | 0.5088 | 0.888 | 0.6841 | 0.9080 |
| finetype-columns | `potion-8m` | 0.3195 | 0.875 | 0.4030 | 0.5069 | 0.883 | 0.6862 | 0.9113 |
| finetype-columns | `potion-32m` | 0.3240 | 0.886 | 0.4141 | 0.5148 | 0.903 | 0.6866 | 0.9200 |
| finetype-columns | `potion-retrieval-32m` | 0.3015 | 0.830 | 0.3998 | 0.5213 | 0.920 | 0.6793 | 0.9024 |
| finetype-columns | `random-384` | -0.0255 | 0.000 | 0.0924 | 0.1690 | 0.000 | 0.5390 | 0.4669 |
| finetype-columns | `bm25` | -- | -- | -- | 0.3940 | 0.588 | 0.6141 | 0.7262 |
<!-- end generated table -->

## Reading it

**The ranked gap splits, and both named causes are real and small.** Taking `potion-base-8M` as the starting point and MiniLM's lift as 1.000:

| corpus | gap 8M → MiniLM | closed by size + vocabulary | closed by objective | left over |
|---|---|---|---|---|
| 20news-body | 0.237 | +0.042 | +0.031 | 0.163 |
| 20news-subject | 0.147 | +0.059 | +0.026 | 0.061 |
| finetype-columns | 0.117 | +0.021 | +0.017 | 0.080 |

Both steps move in the same direction on all three corpora and neither dominates. Together they close roughly a third of the gap on long-form text and column names, and over half of it on short subject lines — so the honest statement is that the earlier finding under-attributed *and* the remainder is still the largest term.

**But the kNN-overlap column, which is the one the deferral cited, barely moves at all.** Across the whole ladder it goes 0.1318 → 0.1603 on bodies, 0.2776 → 0.2861 on subjects, and 0.4030 → 0.3998 on column names — the last one *down*. The same models gained 0.073, 0.086 and 0.037 of ranked lift over the same span, against 0.028, 0.009 and −0.003 of neighbourhood agreement. Whatever is destroying the neighbourhoods is not something a bigger vocabulary or a retrieval objective touches, which is the signature of the projection rather than the embedder.

`finetype-columns` makes the point without any of the ladder: `potion-base-8M` **beats** MiniLM in the raw vector space (`ami_vectors` 0.3924 against 0.3510) and reaches 0.883 of its ranked lift, while agreeing with its map on 40% of neighbours. A model cannot be simultaneously better than MiniLM at the structure and half as good at the neighbourhoods unless the thing losing the neighbourhoods sits after both.

**Pairwise was never in question, and now that is our number rather than a citation.** On `finetype-columns` three of the four static arms beat MiniLM at *is A like B* (`AP same-class` 0.6841, 0.6862 and 0.6866 against 0.6815) and the fourth, `potion-retrieval-32M`, is 0.0022 below it — while BM25 gets 0.6141 and noise gets 0.5390. On long-form text the static arms take 0.70–0.76 against MiniLM's 0.81. Near-duplicate detection is saturated on both 20 Newsgroups corpora — every arm including BM25 scores above 0.93, so that column discriminates nothing there and should not be quoted for those two. On column names it does discriminate: 0.9024–0.9200 for the static arms, 0.9434 for MiniLM, **0.7262 for BM25**.

**A static model beats the lexical floor on all three corpora, but only from 8M up.** On `finetype-columns` the best static arm takes 0.920 of MiniLM's lift against BM25's 0.588. On subject lines, 0.939 against 0.694. On bodies the margin is thinnest — 0.837 against 0.746 — which is the corpus where BM25 has the most terms to work with and is the one to quote if the question is whether a model is worth installing at all.

`potion-base-4M` is the exception and it is worth naming: on 20 Newsgroups bodies it scores 0.740 of lift against BM25's 0.746, so **the smallest model in the family does not beat full-text search on long-form text**. It is the only arm in the file that fails that comparison, and it happens to be one of the two the earlier run was measured on.

## What this changes and what it does not

**It does not reopen semantic search.** That is deferred on a separate and binding reason — there is no efficient embedding creation upstream — and quality was the second reason, not the first. This corrects the second and leaves the first standing.

**It does not justify bundling anything.** The extension carries one model by deliberate decision; a second is a separate question with its own artifact-size and comparability costs.

**What it does establish** is that the figure quoted against static embeddings was a projection figure being read as a retrieval figure, and that ranked retrieval on our own corpora now has its own number: a retrieval-trained static model reaches 84–94% of a hosted transformer's lift over noise, and beats the BM25 an analyst already has on all three corpora.

## Licences

Read live from each model's Hugging Face card metadata in the same pass and recorded in `results.json` under `licences`, with the source of each value. Recorded because a positive result makes bundling the obvious next question and a bundled model is a redistribution.

| model | licence | base model |
|---|---|---|
| `sentence-transformers/all-MiniLM-L6-v2` | apache-2.0 | `nreimers/MiniLM-L6-H384-uncased` |
| `minishlab/potion-base-4M` | mit | — |
| `minishlab/potion-base-8M` | mit | — |
| `minishlab/potion-base-32M` | mit | — |
| `minishlab/potion-retrieval-32M` | mit | `minishlab/potion-base-32M` |

Both licences permit redistribution with the licence text and attribution retained. Neither adds a field-of-use restriction. That answers whether bundling is *permitted*; it says nothing about whether it is wise.

## What this does not say

- **Nothing about speed.** Deliberately not measured here; mixing it in invites the embedding-only figure into a claim it does not support.
- **Nothing about a corpus unlike these three.** Two of the three share a label set — 20 Newsgroups bodies and their own `Subject:` lines — which is what isolates text length from domain, and is also why domain generality is not something this run establishes.
- **Nothing about retrieval quality in absolute terms.** `P@k` here is same-class precision against a 12- or 20-class label set, which is a coarser question than a real retrieval benchmark asks. It supports comparisons between the arms in this file and does not convert to NDCG on NanoBEIR.
- **`finetype-columns` is 216 rows in 12 classes**, after classes with fewer than five members were dropped. It is the most product-relevant shape here and the least statistically comfortable one; treat its figures as indicative and the two 3,000-row corpora as the evidence.
- **Nothing about a threshold an application would set.** Pooled average precision summarises every threshold at once. A shipped duplicate-detection feature needs one, and picking it is a separate measurement.

## Reproducing it

```sh
uv run --script eval/static-embedding-map-fidelity/map_fidelity.py \
  --out eval/static-embedding-map-fidelity/results.json
eval/static-embedding-map-fidelity/check_results.py
```

Roughly half an hour on an M1 Pro, most of it MiniLM over 20 Newsgroups bodies and six UMAP projections per corpus. `--corpus finetype-columns --limit 300` is the fast path for checking the harness still runs.

`check_results.py` is stdlib-only, runs on every pull request, and refuses a results file whose floors have moved, whose arms turn out to have produced identical vectors under different model names, or whose figures disagree with the table above. Regenerate that table with `--emit-table` after any re-run; do not edit it by hand.
