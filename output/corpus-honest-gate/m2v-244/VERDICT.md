# m2v-244 corpus-honest gate — NO-GO (decisive, broad)

**Date:** 2026-06-23 · candidate `models/m2v-244-s44` vs stable v19_gated oracle · 33,250-file stratified sample

## Verdict: NO-GO — m2v-244 is NOT shippable

m2v-244 reproduces v19 on the **curated gold** (composed 0.769 ≈ v19 0.793, within CI) but at
**corpus scale it massively relocates error**. Gold (931 hand-picked columns) is blind to it; the
corpus-honest gate caught it — the v22 lesson, again. 13 trigger labels.

### Collapses (m2v-244 stops predicting what v19 gets right)
| label | v19 | m2v-244 | confirmed lost |
|---|---|---|---|
| `numeric_code` | 59,157 | **238** | 7,328 → **0** (routed to integer_number, +255k — the leading-zero zero-loss failure) |
| `isbn` | 8,024 | 521 | 1,948 → 238 |
| `npi` | 42,675 | 18,608 | 3,086 → 1,809 |
| `compact_ymd` | 1,987 | 702 | 1,408 → 635 |

### Created false positives (over-emit onto oracle-refuted columns)
`file_size` 6.8×, `pipe_separated` 5.5×, `username` 3.3×, `alphanumeric_id` 2.7×,
`docker_ref` 2.4×, `locale_code` 2.1×, `si_number` 2.0× (8,813 contradicted), `unknown` +229k.

## What this means — the deeper reproducibility gap

**Gold-reproducible ≠ corpus-faithful.** ac-01 proved we can rebuild v19's *gold* score, but the
fresh-retrain pipeline does NOT reproduce v19's *corpus behaviour*. Between v19 (built April 2026)
and now, the data blend / generators / 240→244 taxonomy drifted in ways gold cannot see, and the
fresh model collapses numeric_code/isbn/npi and over-emits si_number/file_size/alphanumeric/locale.

Consequences:
1. **m2v-244 cannot ship.** The zero-code "unblock today" path is closed.
2. **potion-8M and code-16M likely inherit this** — same data blend + recipe, only the embed differs;
   a swap won't fix a data-driven collapse. Their tying-v19-on-gold result must be treated as
   unproven until each clears its OWN corpus-honest gate (offline path, before any native work).
3. **v19 remains the default** — still the only corpus-clean model we have, despite being 240-label
   and unreproducible. The dead-end is deeper than "v19 is frozen": our reproductions are corpus-broken.
4. **We skipped the destination-drift precheck** on m2v-244 (built it as the baseline directly). That
   1,000-file/10-epoch proxy would have flagged the numeric_code collapse + si_number over-emit early
   and cheaply. Every fresh retrain — baseline included — must run it.

## The real blocker, now named

Escaping the dead-end requires diagnosing **why fresh retrains collapse numeric_code (and isbn/npi)
and over-emit si_number/file_size** — a data/recipe investigation, not an embedding choice. That is
the actual reproducibility gap; gold hid it, the corpus gate exposed it.
