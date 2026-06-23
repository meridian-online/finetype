# Results ledger — dual-encoder / potion-8M

Living scoreboard for this spec. Numbers are **native, faithful** (full `finetype
profile` pipeline) unless marked offline. Raw predictions + per-type reports live
under `output/dual-encoder-native/` (gitignored, regenerable); this file is the
tracked synthesis. Update it each time a gate runs.

Last updated: 2026-06-23 (corpus-honest gate in progress).

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
| **corpus-honest gate (H05)** | **BLOCKING GO/NO-GO** | **RUNNING** | pending (33,250-file native pass) |
| rare-type scoreboard | headline support | not yet run | — |

## 4. Bugs found & fixed this spec

| bug | symptom | fix (commit) |
|---|---|---|
| potion training omitted `type_index_keys` | native zeroed validation branch → potion-8M looked −11pp | runtime taxonomy-derived fallback (fb44b26) + potion script persists keys (c2cda72) |
| eval_rule.sh corpus pass used PATH `finetype` (stale 0.6.25) | corpus pass 100%-errored | pass `--finetype-bin "$BIN"` (bd206f1) |

## 5. Open / next

- **Corpus-honest gate verdict** (running) — the blocking decision. If GO → potion-8M
  can retire v19. If NO-GO → trigger list drives value-based rule fixes (ac-03), capped
  at ~4 rules.
- **Re-run code-16M native** with the fix (its 0.663 is invalid; offline 0.781 < potion-8M
  so likely not the pick regardless).
- **m2v-244 status:** its config HAS `type_index_keys`, so its earlier corpus NO-GO was
  NOT this bug — likely a real potion-4M corpus regression. Not re-opened unless potion-8M
  also NO-GOs (shared data blend).
- **If we want to beat (not tie) v19:** target the §2 regressions — datetime epochs and
  geography region/country recall.
