# Results ledger — dual-encoder / potion-8M

Living scoreboard for this spec. Numbers are **native, faithful** (full `finetype
profile` pipeline) unless marked offline. Raw predictions + per-type reports live
under `output/dual-encoder-native/` (gitignored, regenerable); this file is the
tracked synthesis. Update it each time a gate runs.

Last updated: 2026-06-23 (corpus-honest gate DONE — NO-GO).

## 1. Headline scoreboard — gold corpus (reframe, n≈927)

| candidate | native composed | offline composed (no veto) | note |
|---|---|---|---|
| **v19 (shipped baseline)** | **0.797** (739/927) | — | the number to tie/beat |
| **potion-8M** best-of-3 (s43) | **0.794** (736/927) | 0.800 | **ties v19** ✓ |
| potion-8M s42 / s43 / s44 | 0.756 / 0.794 / 0.781 | 0.764 / 0.792 / 0.794 | best-of-3 mandatory (seed spread ~4pp) |
| potion-code-16M | 0.663 ⚠️ INVALID (pre-fix) | 0.781 (s44 best) | re-run native with the type_index_keys fix; offline already < potion-8M |

**Key correction (2026-06-23):** the first native potion-8M reading (0.690) was a
config bug (missing `type_index_keys` zeroed the validation branch), NOT a model
regression. Fixed; true value is 0.794. See ac01-native-verdict.md.

## 2. Where to improve — potion-8M per-type vs v19 (gold recall)

Recall **regressions** vs v19 (the actionable list; support ≥3):

| type | support | v19 R | potion-8M R | Δ |
|---|---|---|---|---|
| datetime.epoch.unix_milliseconds | 4 | 1.00 | 0.25 | −0.75 |
| datetime.epoch.unix_seconds | 15 | 0.67 | 0.53 | −0.13 |
| geography.location.region | 15 | 0.73 | 0.60 | −0.13 |
| geography.location.country | 11 | 0.82 | 0.73 | −0.09 |

Recall **gains** vs v19: `representation.text.entity_name` 0.62 → 0.75 (+0.12).

Lowest **absolute** recall on potion-8M (support ≥5 — weakest types regardless of v19):

| type | support | recall | precision |
|---|---|---|---|
| finance.currency.amount | 5 | 0.00 | n/a |
| technology.internet.top_level_domain | 6 | 0.33 | 0.67 |
| datetime.epoch.unix_seconds | 15 | 0.53 | 1.00 |

**Read:** the gap is small and concentrated in **datetime epochs** (unix_seconds/ms)
and **geography region/country** recall — consistent with a tie, not a broad
weakness. These are the first places to look if we want potion-8M to *beat* v19
rather than tie. unix_seconds has P=1.00 / R=0.53 → it's under-asserting (missing
true positives), a recall problem, not a precision one.

## 3. Gate status (promotion order)

| gate | role | status | result |
|---|---|---|---|
| gold-anchor / gold accuracy | efficacy + headline | DONE | potion-8M 0.794 ties v19 0.797 ✓ |
| destination-drift (Sense dist vs v19) | advisory pre/post | DONE | 1 label over band: `technology.internet.user_agent` (advisory) |
| representative accuracy | advisory | not yet run | — |
| **corpus-honest gate (H05)** | **BLOCKING GO/NO-GO** | **DONE** | **NO-GO** — 14 real triggers (post categorical→word remap); 33,250 files, 7.6% err |
| rare-type scoreboard | headline support | not run (NO-GO upstream) | — |

### Corpus-honest gate — the faithful verdict (2026-06-23)

**NO-GO.** Gold tie did NOT translate to corpus-clean. Run with the correct binary
(post bd206f1) + working validation branch (post fb44b26) + categorical→word remap.
This is the REAL verdict (the earlier config-bug run was an artifact). 14 triggers,
far past the ~4-rule cap → per spec, fix the data drift, don't paper over.

**Collapses (potion-8M loses v19-confirmed support):**

| label | v19 marginal | cand marginal | confirmed v19→cand | meaning |
|---|---|---|---|---|
| representation.identifier.numeric_code | 59,157 | 463 | **7,014 → 0** | leading-zero codes collapse to integer_number (+327k) — the big one |
| identity.commerce.isbn | 8,024 | 787 | 1,949 → 216 | isbn collapse |
| datetime.date.compact_ymd | 1,987 | 943 | 1,408 → 797 | partial collapse |

**Over-emits onto oracle-refuted columns (oracle_fp / over_emit):**

| label | ratio (cand/v19) | note |
|---|---|---|
| technology.internet.user_agent | 7.2× | biggest over-emit (21k→151k) |
| datetime.date.compact_dmy | 4.9× | also collapses sibling compact_ymd |
| representation.numeric.si_number | 2.5× | shared with m2v-244 |
| identity.person.last_name | 2.0× | |
| + docker_ref, coordinates, alphanumeric_id, version, username, currency_code, country | 1.4–2× | |

**Root cause:** fresh-retrain data/recipe drift — the SAME signature as m2v-244
(numeric_code/isbn/si_number). m2v-244's config HAD type_index_keys, so its NO-GO
was real, not this bug — confirming the drift is in the data blend, inherited by
potion-8M. Gold-reproducible ≠ corpus-faithful.

## 4. Bugs found & fixed this spec

| bug | symptom | fix (commit) |
|---|---|---|
| potion training omitted `type_index_keys` | native zeroed validation branch → potion-8M looked −11pp | runtime taxonomy-derived fallback (fb44b26) + potion script persists keys (c2cda72) |
| eval_rule.sh corpus pass used PATH `finetype` (stale 0.6.25) | corpus pass 100%-errored | pass `--finetype-bin "$BIN"` (bd206f1) |

## 5. Conclusion & next

**potion-8M is NOT shippable: ties v19 on gold (0.794) but a corpus-honest NO-GO
(14 triggers). v19 stays the default.** The dual-encoder works and is kept as proven
infrastructure. This is the faithful conclusion — the blocking gate did its job.

**The real bet (revived, now validated): fix the fresh-retrain data/recipe drift.**
Task t-000133e418 — its premise is confirmed (real drift, not the config bug).
Targets, in impact order:
1. **numeric_code leading-zero collapse** (7,014 confirmed lost → integer). Banked rule
   t-0000a86b (integer→numeric_code when leading-zero ratio ≥0.5 + fixed-width all-digit)
   — model-agnostic, helps v19 too. Highest-value single fix.
2. **user_agent over-emit 7.2×** — the largest over-assertion; diagnose why fresh retrains
   over-predict it.
3. **isbn / si_number** collapse+over-emit — shared with m2v-244, data-blend issue.
4. The drift is in the **data recipe**, not the encoder — so a corpus-clean fresh model
   is the prerequisite, after which the dual-encoder (if a bigger value encoder is still
   wanted) is ready.

**Not pursued:** ac-03 rule stack (14 triggers ≫ 4-rule cap); code-16M native re-run
(offline 0.781 < potion-8M, and it shares the same data drift — would NO-GO too).
