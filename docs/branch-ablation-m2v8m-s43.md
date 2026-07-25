> Status: measured, NOT merged — the documented 0.853 baseline is stale and the
> artefact's permanent home is an open convention question — 2026-07-25

# Branch ablation — `m2v8m-s43`, five branches, per-label

**Four of the five Sense branches had never been ablated, on any model. This is that measurement, on the model that actually ships.**

Each run below is one forward pass over the same 931 gold columns with one branch's input replaced by zeros and every weight left untouched. Nothing was retrained.

**Zeroing an input is a lower bound on a branch's contribution, not a deletion warrant.** The surviving branches never get a chance to re-fit, so a branch that looks free here may not be. Nothing in this file proposes deleting anything.

## Read this first — the documented baseline no longer reproduces

`scripts/score_clean_label.sh` and `scripts/run_clean_label_retrain.sh` both assert a go/no-go bar of **composed `794/931 = 0.853`**. The un-ablated control was run first, before any ablation, and lands at **819/931 = 0.880** — 25 columns above it.

That gap is **not** a mis-wiring. It is fully attributable to two things that shipped after the bar was written (2026-06-28, `f903a2c`):

| Step | Fixture | Binary | Composed | Attributed to |
|---|---|---|---:|---|
| documented bar | gold @ 2026-06-28 | pre-0.6.41 | 794/931 = 0.853 | — |
| today's pipeline on the **old** fixture | gold @ 2026-06-28 | 0.6.53 | 805/931 = 0.865 | Sharpen releases 0.6.41 / 0.6.50 / 0.6.53 typing guards: **+11** |
| today's pipeline on **today's** fixture | gold @ today | 0.6.53 | 819/931 = 0.880 | gold re-adjudication: **+14** |

The gold fixture at the bar's commit had exactly **931 rows** — the same 931 this FTMB covers, none dropped — but **37 of those 931 curated labels have since been re-adjudicated** (mostly `integer_number` → `identifier.numeric_code`), and 106 new columns were added that this 2026-06-22 FTMB cannot score. So `0.853` is a stale constant, not a live invariant: it cannot be reproduced on current `main` by any correctly-wired run, because the ground truth moved underneath it.

Independent checks that the wiring is sound:

- Two separately-built `finetype` release binaries produced byte-identical headline numbers.
- FTMB dims (char 960 / embed 1024 / stats 27 / header 128 / valid 244, 931 records) match `models/m2v8m-s43/config.json` exactly.
- Every stage's exit code was checked and row counts asserted at 931 after both the reshape and the compose — `score_clean_label.sh` swallows failures on both the composer (`|| cp`) and the scorer (`>/dev/null 2>&1 || true`), so these runs did not use it.

**Consequence for the ablation numbers below: none.** Every run shares one fixture, one binary and one model; only the zeroed branch differs, so fixture drift cancels exactly in a delta. The control level is restated as **Sense 492/931 = 0.528, composed 819/931 = 0.880**.

## What was run

| | |
|---|---|
| Model | `models/m2v8m-s43` — 244 classes, the shipped default |
| Branch widths | char 450 · embed 300 · stats 96 · header 96 · valid 128 = 1070-wide trunk |
| Features | `output/embed-frontier/gold_m2v8m.ftmb` — FTMB v5, 931 records, char 960 / embed 1024 / stats 27 / header 128 / valid 244 |
| Gold | `eval/gold/gold_corpus.tsv` (1037 rows, 931 joinable), enum-reframe scoring |
| Pipeline | `predict_multibranch` → reshape → `compose_predictions.py` (real Sharpen) → `score_gold_anchor.py score --reframe` |
| Ablation | `zeros_like` on the branch input immediately before the forward; all five inputs are dense f32 |

## Headline — Sense is primary

Sharpen is value-based and label-agnostic, so composition repairs branch damage and masks the effect. **Read the Sense columns.** Δ columns is signed — negative is columns the ablation cost. *Sharpen absorbs* is the share of the Sense change that does not survive composition; a **negative** value means composition **amplified** the change instead of absorbing it.

| Run | Sense | Sense acc | Δ acc | Δ cols | Composed | Composed acc | Δ acc | Δ cols | Sharpen absorbs |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| **control** | 492/931 | 0.528 | — | — | 819/931 | 0.880 | — | — | — |
| **zero char** | 479/931 | 0.515 | -0.013 | -13 | 805/931 | 0.865 | -0.015 | -14 | -8% |
| **zero embed** | 424/931 | 0.455 | -0.073 | -68 | 720/931 | 0.773 | -0.107 | -99 | -46% |
| **zero stats** | 487/931 | 0.523 | -0.005 | -5 | 811/931 | 0.871 | -0.009 | -8 | -60% |
| **zero header** | 512/931 | 0.550 | +0.022 | +20 | 827/931 | 0.888 | +0.008 | +8 | +60% |
| **zero valid** | 280/931 | 0.301 | -0.227 | -212 | 778/931 | 0.836 | -0.044 | -41 | +81% |

Three things this table says that the record did not:

1. **The validation branch is the load-bearing one.** Zeroing it costs 212 Sense columns — more than three times the embed branch's 68 — and takes Sense accuracy from 0.528 to 0.301. Every prior architecture opinion ranked embed and char first.
2. **The header branch is net-negative at inference on this fixture.** Removing it *gains* 20 Sense columns and 8 composed columns. It is the only branch whose removal improves the score.
3. **Sharpen does not merely mask branch damage — for the embed branch it amplifies it.** Zeroing embed costs 68 Sense columns but 99 composed ones, because the rule stack acts on the injected label and a wrong label is sharpened into a confidently wrong one.

Read (2) with the fixture in mind: the gold set is curated-hard and headers in it were selected to be contested, so "the header branch hurts here" is not "the header branch hurts in production". It is a reason to measure, not a reason to cut.

## Per-label recall — Sense (raw model output). PRIMARY

Δ is recall against the control. A negative Δ is that branch carrying the type; `0` means the trunk recovered the type from the other four.

| Curated label | Support | Control recall | zero char | zero embed | zero stats | zero header | zero valid |
|---|---:|---:|---:|---:|---:|---:|---:|
| representation.numeric.integer_number | 167 | 0.353 | +0.006 | +0.318 | +0.024 | +0.018 | -0.054 |
| representation.text.RESIDUAL | 119 | 0.252 | 0 | -0.202 | -0.017 | +0.059 | -0.109 |
| representation.numeric.decimal_number | 99 | 0.768 | -0.243 | -0.152 | 0 | +0.131 | -0.101 |
| representation.identifier.alphanumeric_id | 62 | 0.613 | -0.048 | -0.484 | +0.081 | +0.081 | -0.355 |
| datetime.date.iso | 56 | 0.929 | 0 | 0 | 0 | 0 | -0.929 |
| geography.location.country_code | 54 | 0.833 | +0.037 | -0.814 | -0.018 | +0.019 | -0.777 |
| geography.coordinate.longitude | 45 | 0.111 | 0 | +0.045 | -0.022 | -0.044 | -0.044 |
| technology.internet.url | 44 | 0.614 | -0.182 | +0.159 | -0.023 | -0.159 | -0.569 |
| datetime.component.year | 41 | 0.927 | +0.073 | -0.025 | 0 | 0 | -0.025 |
| geography.coordinate.latitude | 39 | 0.179 | +0.539 | +0.513 | -0.051 | -0.076 | -0.153 |
| geography.location.city | 24 | 0.917 | 0 | 0 | -0.042 | -0.042 | 0 |
| representation.identifier.numeric_code | 24 | 0.333 | -0.083 | -0.208 | -0.041 | +0.042 | -0.041 |
| identity.commerce.isbn | 18 | 0.889 | -0.445 | -0.445 | 0 | 0 | 0 |
| datetime.epoch.unix_seconds | 15 | 0.000 | 0 | +0.400 | 0 | 0 | 0 |
| geography.location.region | 15 | 0.600 | -0.133 | -0.533 | -0.133 | 0 | -0.267 |
| representation.text.entity_name | 12 | 0.750 | +0.083 | -0.417 | -0.083 | +0.167 | 0 |
| geography.location.country | 11 | 0.818 | 0 | -0.818 | 0 | -0.091 | 0 |
| representation.boolean.terms | 10 | 0.600 | 0 | -0.500 | 0 | +0.400 | -0.300 |
| datetime.timestamp.sql_standard | 9 | 0.889 | +0.111 | +0.111 | 0 | 0 | -0.889 |
| geography.location.state_code | 7 | 0.143 | -0.143 | 0 | 0 | -0.143 | -0.143 |
| datetime.offset.timezone_abbreviation | 6 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| representation.identifier.increment | 6 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.iso_milliseconds | 5 | 0.400 | +0.600 | +0.400 | -0.200 | 0 | -0.400 |
| geography.address.postal_code | 5 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| technology.internet.top_level_domain | 5 | 0.400 | +0.400 | -0.400 | 0 | 0 | -0.400 |
| datetime.epoch.unix_milliseconds | 4 | 0.250 | +0.250 | +0.750 | 0 | 0 | -0.250 |
| geography.address.full_address | 4 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.offset.iana | 3 | 1.000 | 0 | -0.667 | -0.333 | -0.333 | -0.667 |
| datetime.timestamp.iso_8601_milliseconds | 3 | 1.000 | 0 | 0 | 0 | 0 | -1.000 |
| geography.transportation.iata_code | 2 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| representation.identifier.uuid | 2 | 1.000 | 0 | 0 | -0.500 | 0 | 0 |
| technology.internet.hostname | 2 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| container.object.csv | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.date.dmy_slash | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.dmy_hm | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.sql_minutes | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| geography.location.continent | 1 | 1.000 | 0 | -1.000 | 0 | 0 | 0 |
| geography.transportation.icao_code | 1 | 1.000 | 0 | 0 | 0 | 0 | -1.000 |
| identity.person.full_name | 1 | 1.000 | 0 | -1.000 | 0 | 0 | -1.000 |
| identity.person.gender | 1 | 1.000 | 0 | -1.000 | 0 | 0 | 0 |
| identity.person.username | 1 | 1.000 | 0 | 0 | 0 | 0 | -1.000 |
| representation.boolean.binary | 1 | 1.000 | 0 | -1.000 | 0 | 0 | 0 |
| representation.file.extension | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| technology.filesystem.filename | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| technology.filesystem.windows_path | 1 | 0.000 | 0 | 0 | +1.000 | 0 | 0 |

## Per-label recall — composed (after real Sharpen). Secondary

Same shape after Sharpen. Where a Sense Δ shrinks to `0` here, a rule repaired what the branch loss broke — which is why the Sense table is the primary one.

| Curated label | Support | Control recall | zero char | zero embed | zero stats | zero header | zero valid |
|---|---:|---:|---:|---:|---:|---:|---:|
| representation.numeric.integer_number | 167 | 0.910 | -0.018 | -0.024 | 0 | +0.006 | -0.024 |
| representation.text.RESIDUAL | 119 | 0.697 | -0.050 | -0.394 | -0.008 | +0.026 | -0.042 |
| representation.numeric.decimal_number | 99 | 0.990 | -0.010 | 0 | -0.010 | 0 | -0.081 |
| representation.identifier.alphanumeric_id | 62 | 0.774 | -0.064 | -0.484 | +0.016 | +0.032 | -0.016 |
| datetime.date.iso | 56 | 0.929 | 0 | 0 | 0 | 0 | 0 |
| geography.location.country_code | 54 | 0.926 | +0.037 | -0.019 | 0 | +0.055 | -0.111 |
| geography.coordinate.longitude | 45 | 0.978 | 0 | 0 | 0 | 0 | -0.045 |
| technology.internet.url | 44 | 0.841 | 0 | 0 | 0 | 0 | -0.114 |
| datetime.component.year | 41 | 0.976 | 0 | 0 | 0 | 0 | 0 |
| geography.coordinate.latitude | 39 | 1.000 | 0 | 0 | 0 | 0 | -0.026 |
| geography.location.city | 24 | 0.958 | 0 | 0 | -0.041 | -0.041 | 0 |
| representation.identifier.numeric_code | 24 | 0.667 | -0.084 | -0.125 | -0.042 | 0 | -0.125 |
| identity.commerce.isbn | 18 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.epoch.unix_seconds | 15 | 0.733 | 0 | 0 | 0 | 0 | 0 |
| geography.location.region | 15 | 0.867 | -0.067 | -0.267 | -0.134 | 0 | -0.200 |
| representation.text.entity_name | 12 | 0.750 | 0 | -0.167 | -0.083 | 0 | -0.083 |
| geography.location.country | 11 | 0.818 | 0 | -0.363 | 0 | -0.091 | 0 |
| representation.boolean.terms | 10 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.sql_standard | 9 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| geography.location.state_code | 7 | 0.857 | +0.143 | 0 | 0 | +0.143 | +0.143 |
| datetime.offset.timezone_abbreviation | 6 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| representation.identifier.increment | 6 | 1.000 | 0 | -0.167 | 0 | 0 | 0 |
| datetime.timestamp.iso_milliseconds | 5 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| geography.address.postal_code | 5 | 0.800 | +0.200 | +0.200 | 0 | +0.200 | +0.200 |
| technology.internet.top_level_domain | 5 | 0.800 | 0 | -0.400 | 0 | 0 | -0.400 |
| datetime.epoch.unix_milliseconds | 4 | 0.500 | 0 | +0.500 | 0 | 0 | +0.500 |
| geography.address.full_address | 4 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.offset.iana | 3 | 1.000 | 0 | -0.333 | -0.333 | -0.333 | -0.667 |
| datetime.timestamp.iso_8601_milliseconds | 3 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| geography.transportation.iata_code | 2 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| representation.identifier.uuid | 2 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| technology.internet.hostname | 2 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| container.object.csv | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.date.dmy_slash | 1 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.dmy_hm | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| datetime.timestamp.sql_minutes | 1 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| geography.location.continent | 1 | 1.000 | 0 | -1.000 | 0 | 0 | 0 |
| geography.transportation.icao_code | 1 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| identity.person.full_name | 1 | 1.000 | 0 | -1.000 | 0 | 0 | -1.000 |
| identity.person.gender | 1 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| identity.person.username | 1 | 0.000 | 0 | 0 | 0 | 0 | +1.000 |
| representation.boolean.binary | 1 | 1.000 | 0 | 0 | 0 | 0 | 0 |
| representation.file.extension | 1 | 0.000 | 0 | 0 | 0 | 0 | 0 |
| technology.filesystem.filename | 1 | 1.000 | 0 | 0 | -1.000 | 0 | -1.000 |
| technology.filesystem.windows_path | 1 | 1.000 | -1.000 | -1.000 | 0 | 0 | -1.000 |

## Under-emission (Sense)

Of a type's true columns that the model missed, the share that landed on the abstain residual (`unknown` / `word` / `plain_text` / `categorical`) rather than on a concrete sibling. A branch loss that raises under-emission pushed the model into declining to call, not into calling something else.

| Run | FN→abstain | of support | rate | Δ vs control |
|---|---:|---:|---:|---:|
| **control** | 24 | 931 | 0.026 | — |
| **zero char** | 20 | 931 | 0.021 | -0.004 |
| **zero embed** | 9 | 931 | 0.010 | -0.016 |
| **zero stats** | 24 | 931 | 0.026 | +0.000 |
| **zero header** | 34 | 931 | 0.037 | +0.011 |
| **zero valid** | 38 | 931 | 0.041 | +0.015 |

## Reproduce

```sh
cargo build --release --bin predict_multibranch -p finetype-train
cargo build --release   # compose_predictions.py shells out to `finetype profile`

# the --zero-* flags pass straight through score_clean_label.sh's trailing "$@"
scripts/score_clean_label.sh models/m2v8m-s43 \
    output/embed-frontier/gold_m2v8m.ftmb <tag> --zero-char
```

Check each stage's exit code yourself; that script does not, and its `0.853` trailer is the stale bar described above. Budget ~4 min per pass — the cost is `compose_predictions.py` launching `finetype profile` once per column, 931 subprocesses; the forward itself is under two seconds.

## What this does not say

- **It does not license deleting a branch.** Confirming a deletion needs a multi-seed retrain without the branch; a zeroed input understates contribution because the other branches cannot compensate.
- It says nothing about training cost, only about what the trained trunk leans on at inference.
- The gold set is curated-hard and is the engine's optimisation target, so these absolute accuracies overstate production accuracy on a randomly drawn column. **The deltas are the point, not the levels.**
- Recall deltas on labels with single-digit support are one or two columns moving. Read the support column before reading the delta.
- *Sharpen absorbs* is a ratio of two small integers for the char and stats rows (13 and 5 columns). Only the embed and validation rows have enough movement for it to mean anything.
