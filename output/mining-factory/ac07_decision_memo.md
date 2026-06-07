# ac-07 — decision memo: manufacturing works, the additive multi-branch consumer is dead, pivot to the value-level architecture test

Spec `2026-06-07-reference-data-mining-factory`. This memo answers the three ac-07
questions and records the go/no-go on the roadmap items.

## Did manufacturing dissolve starvation? YES — proven (ac-01 census).

The wall this spec set out to break: real GitTables contains **10 distinct latitudes in
18M rows**, and 66 of 159 types fall below a 50-distinct floor. Scaling GitTables
provably cannot fix it — the diversity is not in the corpus. Manufacturing from
authoritative reference data dissolves it:

- `geography.coordinate.latitude`: **10 → 21,478 distinct** manufactured values.
- `geography.coordinate.longitude`: 24 → 29,140 distinct.
- `geography.location.city`: → 27,437 distinct. Every Tier-1 type clears the 50-distinct
  floor by orders of magnitude (ac-01 census, `output/mining-factory/census.json`).

The funnel (ac-02 JSON-Schema veto), firewall (ac-03, zero eval/gold overlap), and
materialise/blend (ac-04) all hold. **The manufacturing pipeline is the durable,
reusable deliverable of this spec** — it stands regardless of which model consumes it.

## Is the candidate a GO on both instruments? NO CANDIDATE WAS PRODUCED.

ac-05's mandatory destination-drift proxy pre-check returned **NO-GO three times**, so
the overnight multi-branch train never launched and no candidate reached ac-06's gold
anchor or corpus-honest gate. The collapse (full detail in `ac05_proxy_precheck.md`):

| label | v19 base | full-dose | interleaved | light-dose (1/6) |
|---|---:|---:|---:|---:|
| `representation.numeric.decimal_number` | 31.29% | 0.29% | 0.29% | **0.29%** |
| `representation.text.entity_name` | 3.38% | 44.71% | 48.18% | 28.32% |

The decisive fact: **`decimal_number` collapses to 0.29% IDENTICALLY at full dose and at
1/6 dose.** Not a volume problem, not a blend-mechanics problem (interleaving changed
nothing), not corrupt data (manufactured values are well-formed). The multi-branch model
**structurally cannot absorb decimal-shaped coordinates as a distinct class** without
gutting its plain-number prior on the real corpus. Additive hard-negative/reference
retrains are now **0-for-4** (v22 categorical, v23 categorical, v24 latitude, this).

**Consequence for the gate's GO-precision:** still unvalidated. This spec was meant to be
the gate's first genuine GO candidate; it produced no candidate to test, because the
cheaper upstream gate (proxy pre-check) correctly blocked all three. The proxy gate's
NO-GO precision is now 0-for-4-caught — strong; its job is done here.

## Go/no-go on the roadmap items

- **Extend manufacture to Tier-2 types — HOLD.** Coverage is not the bottleneck; the
  *consumer* is. Manufacturing more types only matters once a model can absorb the
  diversity without collapsing. Do not spend on Tier-2 until the architecture question
  is resolved.
- **char-cnn-vs-multi-branch value-level architecture test — GO (executing now).** The
  prior spec `2026-06-04-value-level-ydf-labelling` closed INCONCLUSIVE for exactly one
  reason: cleaned real GitTables starves the rare types (its capped training set carries
  **latitude = 10 distinct, longitude = 24**), so the value-level CharCNN could not be
  tested on the load-bearing confusion family. **The manufactured corpus removes that
  blocker** — it is the non-starved value-level diversity the test always lacked. The
  value-level model classifies one value at a time with no column-level sibling
  entanglement, so the collapse mechanism that kills the multi-branch additive blend
  cannot apply by construction. This is the clean next experiment and the recommended
  pivot.
- **B3 late-fusion model — HOLD.** Depends on the architecture question. Defer until the
  value-level test reports.

## One line

Manufacturing dissolved the starvation wall and is a reusable win; the multi-branch
additive blend is a structural dead end (0-for-4, dose-independent numeric collapse); the
unblocked move is the value-level architecture test, now feasible on the non-starved
manufactured corpus and running.
