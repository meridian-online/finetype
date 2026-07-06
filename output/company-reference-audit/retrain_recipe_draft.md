# Retrain recipe draft — numeric/text attractor hard negatives (task t-000133e418)

**Date:** 2026-07-05 · **Status:** DRAFT — awaiting author review before any build
**Architecture:** unchanged — potion-8M dual-encoder, `scripts/overnight_potion.sh` recipe, additive
hard negatives only.

## Root cause, confirmed in the training data

The shipped FTMB (`output/multibranch-training/m2v8m-244.ftmb`) contains **zero real (distilled)
rows for npi and upc** — 325 / 288 purely synthetic positives each. The model has never seen a
10-digit financial figure labelled anything but npi-adjacent. Same shape for the text side:
user_agent has 4 distilled rows, height 2, weight 65. The attractor is a data hole, not an
architecture fault — which is why the fix is negatives, not rules.

One stroke of luck: the W2b generator fixes (Luhn parity, GS1 anchoring) mean this retrain's
synthetic npi/upc positives are now genuinely check-digit-valid. A retrain run before W2b would
have trained on fakes.

## Mined hard-negative sources and counts

All mined from the 33k-file stratified gate sample passes (in-sample columns; corpus-scale is
~16.7× for npi, ~41.5× for upc). Mining substrate on disk:

| family | source (all under `output/company-reference-audit/`) | candidates |
|---|---|---|
| npi | `w3_baseline_with_oracle.parquet` ⋈ `eval_w2b_substance/sample_pass/` (guard-demoted set) | 2,227 cols / 1,379 files |
| upc | same join | 294 + 16 retained-by-chance |
| user_agent | `ua_demoted_negatives.parquet` (preserved from the W1 pre/post join; the pre-W1 pass lived in a GC-able tmp dir) | 3,851 cols / 3,409 files |
| height / weight / locale_code | `eval_sexp/sample_pass/` (newest composed pass) + oracle join | 1,191 / 996 / 776 emissions |

### Proposed target-label split (after dedup + per-header caps)

| mined family → target label | available | proposed rows | notes |
|---|---|---|---|
| npi → `representation.numeric.integer_number` | 741 | ~610 | financial headers (marketCap, grossProfit, ebit, longTermDebt…) 509; TEAM_ID-style pure-digit IDs 119; out-of-epoch-range tail 113 |
| npi → `datetime.epoch.unix_seconds` | 845 | ~150 | **the headline surprise**: the largest npi attractor group is market/weather epochs — `regularMarketTime` alone is 742 cols (capped to 50); 807/845 oracle-co-signed unix_seconds |
| npi → excluded | 641 | 0 | 104 empty, 537 ambiguous (epoch-range values under non-time headers: TTL, Created, NAV); ~380 recoverable later with a curated header list — held as reserve |
| upc → `integer_number` | 296 | ~215 | yfinance 11–13-digit magnitudes (190) + `particleId` runs (86→50); includes the 16 coincidental-checksum-pass financial cols mined by header+value rule, not guard verdict |
| user_agent → `representation.text.plain_text` | 3,759 | ~1,500 | 97.6% of the raw-model UA attractor is prose (forum posts, transcripts, logs) under content-ish headers; zero genuine UA strings in the whole demoted set |
| height → `decimal_number` / `integer_number` / `plain_text` | 57 / 31 / 214 | ~290 | instrument heights (height_mm 0.095, Antenna_Height_metres), screen pixels, text under height-ish headers |
| weight → `decimal_number` / `integer_number` / `plain_text` | 199 / 55 / 163 | ~400 | statistical weights (weight_decay 0.0001, min_weight_fraction_leaf), flags, race names. **359 "N mph" wind cols EXCLUDED** — quantity-with-unit taxonomy gap, noted for a future leaf, not trained as plain_text |
| locale_code → `representation.text.word` | 443 | ~400 | 2-letter small-vocab tokens that are neither ISO-639 nor ISO-3166 (veg_type EN, act_tag sd, handedness); the 48 bare-"en" ISO-639 columns stay out (legitimately locale_code) |

**Total: ~3,565 mined rows** across 5 destination labels. Relative to base FTMB mass:
integer_number +19%, plain_text +23%, word +27%, unix_seconds/decimal to be verified at
`read_ftmb --stats` time. This is the v27 controlled-mass shape (1–3k/label), not the v24 shape
(78.6k rows, exploded latitude).

Belt-and-braces exclusions: the 195 npi-demoted columns whose oracle co-signs npi (~19 could be
real); empty-value columns (the loader's min_values≥5 drops them anyway); the 780 empty Joomla
header-branch FPs on height (untrainable — `unknown` is not a model class; separate engine
question: profile asserts a type on zero-value columns).

### Blend construction rules

New builder `scripts/build_attractor_negatives_distilled.py`, modelled on
`build_v27_recall_distilled.py`:

- Base = `output/distillation-v3/sherlock_distilled.csv.gz` (the additive chain over v22-era
  files is not reproducible from disk — only manifests survive).
- Masses enforced IN the blend CSV (MINED_KEEP dict, md5-ordered truncation) — `--distilled-cap`
  is cosmetic on the v4/v5 path (ordered-distilled-cap bug, still open).
- Mined rows **carry their real corpus headers** (identity-fortification precedent; header context
  is the discriminator the checksum can't be). Note: base distilled rows are header-less, so this
  shifts the header-branch distribution for the destination labels — the proxy watches this.
- Per-header cap 50, dedup by (file_content_sha256, column_name).
- Audit gate (exit 3): leakage vs ALL FOUR fixture identity lists (gold_eval_anchor,
  gold_corpus_candidates, gold_corpus_candidates_external, representative_corpus) on durable
  (sha256, column_name) identity + the row-hash audit; zero rows labelled categorical post-remap;
  per-family row floors; blend manifest json.

## Pre-flight blockers (must clear before STEP 1)

1. **244 vs 247 taxonomy drift — BLOCKING.** Live binary reports 247 leaves / 247-wide validation
   vector; `overnight_potion.sh` hard-gates ==244 and `models/m2v8m-244-config.json` is 244-wide.
   **Recommendation: go 247** — `n_classes` 247 + `valid_dim` 247, new config file, update the
   script gates. This is the letter of the settled principle ("n_classes = the live taxonomy");
   the three recovery-only leaves get synthetic positives automatically and their Sharpen guards
   stay as backstops. Alternative (smaller change): keep 244 classes, widen only the gates.
2. **No m2v8m-s43 drift baseline exists.** Every snapshot in `output/destination-drift-precheck/`
   is v19/v22-era. One-time: snapshot `models/default` on the fixed 1,000-file list
   (`sense_dist_v19fx_s42.files.txt` — MUST reuse; reservoir sampling isn't reproducible) before
   any proxy run.
3. `proxy_pretrain.sh` landmines: defaults `--config` to the legacy v13 config (override with the
   candidate config); reuses `models/sherlock-<name>-s42` if it exists (mint fresh names); v5 FTMB
   carries precomputed embeds so no `--value-encoder` needed (verify STORE_VALUES=0 at build).
4. Mint a **fresh `--tag`** — `overnight_potion.sh` exists-skips both FTMB and seeds; reusing
   `m2v8m` would silently train nothing.
5. `eval_rule.sh` model mode gives **no warning** when `--gate-baseline` is omitted and silently
   gates vs retired v19 — always pass the fresh baseline.

## Gate sequence (mandated order, with predicted outcomes)

1. Build blend → audit gate → `build_ftmb_v5_potion.py --distilled … --output <newtag>.ftmb` →
   `read_ftmb --stats` (the only honest accounting of what actually enters training).
2. One-time m2v8m-s43 baseline snapshot (pre side of the mandatory Sense-distribution check).
   safety_score: computable for the (sense=npi, ydf=X) pairs via a hand-built gaps parquet —
   advisory only, will compute alongside the blend audit.
3. **Drift proxy precheck** (~30–60 min, 1 seed × 10 epochs on the full candidate FTMB):
   `proxy_pretrain.sh --name <tag>-proxy --ftmb … --baseline sense_dist_m2v8mfx_s43.json
   --config <candidate config>` → calibrated band `--abs-floor 0.0040 --rel-mult 3.0 --direction up`.
   Exit 1 = do NOT launch overnight; iterate by shrinking the largest family first (UA plain_text),
   rebuild, re-proxy. The proxy caught both paid-for explosions (v23 categorical, v24 latitude)
   retroactively at rank 1.
4. Overnight 3-seed (`overnight_potion.sh --tag <newtag>`; ~7h train+score after FTMB build).
5. Post-train (all mandatory): full Sense-distribution check vs the s43 snapshot; gold +
   representative (`--reframe`, companion) + rare-type scoreboard via `score_gold_anchor.py`.
6. **Corpus-honest gate, fresh-vs-fresh**: rebuild binary once; baseline pass = `models/default`
   + that binary on the 33k sample; join `ydf_prediction_gated` from
   `output/ydf-validation-gate/v19_gated.parquet`; candidate pass = same binary + candidate model;
   `corpus_honest_gate.py --baseline <joined> --candidate <pass>`.
   **Predicted verdict: NO-GO with collapse on exactly npi + upc, by construction.** Arithmetic:
   gated-YDF confirms npi correctly in 9.6% of assertions (upc 2.7%), so the max HONEST
   correct_ratio ≈ 0.096 / 0.027 vs the 0.6 collapse threshold — passing would require keeping
   ~56–59% of the known-wrong calls the retrain exists to remove. **Pre-registered adjudication:**
   per-column checksum verification of the moved columns (`ydf_reliability.py` method) + the
   choice-0104 gold-adjudicated relocation review. Any OTHER banded label is a genuine relocation
   signal and gets no ride on this adjudication.
7. Swap is the author's call (gold parity + relocation review) — never autonomous.

## Success criteria

- Gold: no regression from 844; the three npi-predicted gold rows flip (+2 available on
  longTermDebt/TEAM_ID → integer_number; utc → unix_seconds would be +1 more — the epoch negative
  group targets exactly this).
- Gate sample: npi est marginal ~42.7k → within ~2× of genuine mass (~5k incl. the guard-measured
  keep tail); upc ~12.9k → <1.5k; no banded labels beyond the two predicted collapses.
- Representative band: no advisory flag (delta within CI of 0.691-era baseline).
- Sense distribution: no untargeted label past the calibrated band pre AND post.

## Scope decisions (recommendations made, author can flip)

- **Short-code cluster (version, docker_ref, compact_dmy, coordinates, last_name, country +
  guard-masked username/currency_code/si_number): DEFER to a round-2 mining pass.** No
  provenance-grade demoted set exists for them (the npi/upc/UA sets are ready-made and labelled);
  this blend already spans 5 destination labels and the v23 lesson says don't stack clusters on
  the first bet. The census (ship-gate movers, guard status per label) is banked in this session's
  substrate for round 2.
- **W4 founder-style org/person negatives: DEFER.** Same task home, but it needs a generator
  change (`gen_entity_name` founder arm) + its own mining pass, and it wasn't in the WHAT-TO-FIX
  list. Fold into round 2 alongside short-code.
- **"N mph" wind columns: EXCLUDED** — quantity-with-unit is a taxonomy gap, not a plain_text
  fact. Logged for the taxonomy-gap ledger.

## Proxy adjudication — 2026-07-06, overnight launched on documented evidence

The 10-epoch proxy NO-GO'd twice; the first was an instrument failure (encoder-less proxy
dir profiles zero columns → empty snapshot reads as NO-GO; fixed in proxy_pretrain.sh),
the second was real with two flags. The pre-registered "iterate cheaply" loop ran as
follows before the overnight was launched:

1. **Per-column diff** (150 fixed-list files, baseline vs proxy):
   `unix_seconds` gainers = 49, of which 45 are `Created` columns whose values ARE unix
   epochs — the baseline mislabels them `decimal_number`, so these are corrections —
   4 are financial-header leaks (`totalAssets` et al.). `json_array` gainers = 40, of
   which 31 are one `Content` file-family — single-source concentration on a 336-row
   synthetic class.
2. **20-epoch probe** (same FTMB, same calibrated band): `json_array` MELTED (not
   flagged — the documented under-convergence false-flag mode, v24 compact_ymd
   precedent). `unix_seconds` persisted byte-stable (67→310, 4.60×, +1.80pp).
3. **Adjudication:** the only surviving flag is the recipe's own targeted destination
   (npi→unix_seconds negatives), moving in the intended direction, dominated by verified
   corrections. The v23/v24 explosions this band exists to catch were UNTARGETED labels;
   a band with `--direction up` cannot distinguish intended recall on a small-base label
   from over-emit. Launched the overnight with `PROXY_ADJUDICATED=1`.

**Morning checklist (the leak tail this adjudication defers to post-train instruments):**
- financial-header 10-digit → unix_seconds transitions at corpus scale (the 4/49 tail);
- `decimal_number` −10.8pp and `entity_name` −2.9pp in the 20-epoch proxy — proxy
  down-moves are documented-unreliable, but verify both in the post-train FULL-model
  snapshot before gating;
- the standard stack: gold no-regression + representative, rare-type scoreboard, blocking
  corpus-honest gate fresh-vs-fresh (expected signature: collapse on npi+upc only).

## Results — 2026-07-06 (overnight complete; corpus-honest gate pending)

Three seeds trained (s42 best: 64 epochs). Scoreboard vs the shipped default m2v8m-s43:

| instrument | shipped | attneg-s42 | read |
|---|---|---|---|
| gold composed | 0.856 (844/986) | **0.855** | parity — no curated-ground regression |
| gold raw Sense | 0.522 | **0.580** | +5.8pp — the attractor layer itself improved |
| representative (260 random cols) | 0.712 | **0.746** | +9 cols; no advisory flag |
| npi on fixed 13.5k-col sample | 84 | **3** | the 10-digit attractor is dead raw |
| upc on same | 16 | **4** | same |
| post-train sense check | — | 1 flag | unix_seconds 67→284, the adjudicated Created-epoch correction |
| json_array (proxy scare) | 19 | 12 | confirmed under-convergence artifact, gone at convergence |

**Open before any swap talk:** (1) corpus-honest gate (running) — expected collapse on
npi+upc (pre-adjudicated); a band on ANY other label — watch `decimal_number`, which
redistributed −10.7pp to unknown/plain_text/word/unix at convergence, and `entity_name`
−2.6pp — is a genuine relocation signal. (2) rare-type scoreboard. (3) The swap itself is
the author's call (gold parity + gold-adjudicated relocation review, choice 0104).

## What we don't know yet

- Whether header-carrying negatives shift the header branch for integer_number/plain_text in a way
  that costs recall elsewhere (base distilled rows are header-less) — the proxy is the instrument.
- unix_seconds/decimal_number base masses in the FTMB (checked at read_ftmb time; if unix_seconds
  base is small, +150 real epochs is a meaningful recall gain but also a distribution shift).
- Whether a 10-epoch proxy is sensitive to text-side drift from the plain_text addition (the band
  was calibrated on numeric/categorical explosions).
