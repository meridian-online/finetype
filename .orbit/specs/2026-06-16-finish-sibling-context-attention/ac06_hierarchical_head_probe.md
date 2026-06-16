> **CORRECTION (2026-06-16, before any new run): the hierarchical head was ALREADY
> TRAINED and FALSIFIED.** I recommended a training run from the error-decomposition
> ceiling below without first checking for prior art — that was a miss. The experiment
> exists: `output/mining-factory/coord-only/ac06_hierarchical_verdict.md` (choice 0097
> research arm, 2026-06-13). On identical data, the hierarchical head scored gold **0.685
> vs the flat head's 0.718** (−3.3pp), Sense drift NO-GO, corpus-honest gate NO-GO (8 vs
> 7), and `numeric_code` collapsed HARDER (3% kept vs 18%). Crucially it *created* NEW
> cross-domain instability (`calver` 8.2×, `docker_ref`) — the opposite of the cross-domain
> fix the ceiling below predicted. Mechanism: "splitting the OUTPUT head does not fix
> interference that originates in the SHARED REPRESENTATION; it adds interference surface."
> The +12.2% ceiling below is a real PROPERTY of the errors but is UNREACHABLE by a head
> split — the probe mis-located the fix. **No new hierarchical run launched.** See memory
> hierarchical-head-falsified.

# Layer-2 probe — hierarchical Domain→Family→Type head: a REAL recall bet

**Date:** 2026-06-16 · `scripts/probe_hierarchical_head.py` · raw: `ac06_hierarchical_head_probe_output.txt`
**Context:** ac-01's sweep showed sibling-context is a thin gold-recall lever. This probes
the program's second layer before committing either build. No model, no tables — gold
labels vs v19 predictions, decomposed by taxonomy domain/family.

## Headline: the hierarchical head has a large ceiling where sibling-context had none

| level | v19 gold accuracy |
|---|---:|
| LEAF (the headline) | 0.782 |
| FAMILY | 0.801 |
| **DOMAIN** | **0.878** |

v19 knows the **domain** 87.8% of the time but picks the right **leaf** only 78.2% — a
9.6pp gap. The flat 250-way softmax leaks across domain boundaries that a coarse-to-fine
head is built to hold.

## Where v19's 203 leaf errors sit

| error class | count | of errors | of gold | hierarchy can…|
|---|---:|---:|---:|---|
| **CROSS-domain** (pred in wrong domain) | 87 | 43% | 9.3% | prevent (if domain head right) |
| **pred = unknown** | 27 | 13% | 2.9% | rescue (if domain known) |
| cross-family / same-domain | 71 | 35% | 7.6% | maybe (needs family head) |
| WITHIN-family | 18 | 9% | 1.9% | **cannot fix** |

**Hierarchy recall ceiling = 114 cols = +12.2% gold** (cross-domain + unknown-rescue) — vs
sibling-context's ~1 column. Only **9% of errors are genuinely irreducible** by hierarchy.

## Per recall-gap label

- **categorical** (54): 27 cross-domain (iata, city…), 17 cross-family, 4 unknown, only 6
  within-family — the biggest gap is largely a domain-leakage problem.
- **integer** (40): 28 cross-FAMILY same-domain (increment 17, binary 6, ordinal 3 — all
  representation), 10 cross-domain (year/unix → datetime). Domain constraint won't fix the
  within-representation 28; a FAMILY head might.
- **plain_text** (31): 18 cross-domain (city, url…) — strongly domain-leakage.
- **alphanumeric_id** (18): 7 unknown + 8 cross-domain (url) — mostly rescuable.
- **date.iso** (15): 11 cross-FAMILY within datetime (→ timestamp.sql_standard / iso_8601_*).
  Domain head won't help; this is a datetime-internal granularity problem.
- **decimal** (13): 6 unknown + 4 cross-domain (utc).

## Honest reading — ceiling, not expectation

The +12.2% is the ceiling assuming a **near-perfect domain head**. The realism caveat is
load-bearing: the 87 cross-domain errors are exactly the columns where v19's leaf already
chose the wrong domain, so the shared trunk's domain signal is *weakest there*. A domain
head helps only if it beats v19's current **0.878** implicit domain accuracy **on the hard
columns** — that is the open question. Two mechanisms, different odds:
- **Domain-constraint** (prevent cross-domain leakage) → targets the 114 domain-wrong /
  unknown errors; gated on the domain head's accuracy.
- **Coarse-to-fine training** (residual de-contamination, choice 0096) → targets the 89
  same-domain errors; lets structured leaves train without categorical/unknown attractor
  competition.

## Verdict & recommendation

**Re-order the program: hierarchical head FIRST.** Unlike sibling-context (ceiling ~0 on
gold), this has a +12.2% ceiling and only 9% irreducible errors. The head is already built
(`char_cnn.rs` `HeadType::Hierarchical` + `HierarchyMap`, `use_hierarchical` flag) and needs
NO new data format — it trains on the existing corpus, far cheaper than the sibling FTMB-v3
build. The decisive next test is a small hierarchical training run measuring leaf recall +
domain accuracy vs the flat baseline; if the domain head beats 0.878 on the hard columns,
the ceiling is reachable. Sibling-context demotes to a later, advisory corpus-precision
hardening for coordinates.
