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

## Corpus-honest gate — 2026-07-06 13:20, NO-GO, candidate NOT promotable as-is

`gate_attneg.json` (fresh-vs-fresh, default bands): NO-GO, nine triggers. Reading them
against the pre-registration:

**Pre-adjudicated (the fix landing):** npi collapse 42,675→842 (cand ≈2× genuine mass —
inside the success criterion); upc collapse 12,873→2,492 (every survivor oracle-co-signed
12-digit shape); aba_routing collapse 7,522→374 — same checksum-shape-matcher class
(gated-YDF asserts aba correctly 5.5%), not pre-registered by name but same adjudication
basis. unix_seconds oracle_fp: growth 39.8k→65k with oracle-confirmed support GROWING
(38.6k→44.2k) — mostly the Created-family correction, BUT 921/2,707 (34%) of in-sample
inflow sits under financial-ish headers = the leak tail is material, not noise.

**Genuine regressions (do NOT ride the adjudication):**
- **decimal_number → unknown: 39,911 in-sample columns, 95% float-shaped, 3,049 distinct
  headers** (`bookValue -0.138` → unknown). Mechanism (inferred, fits all evidence): the
  financial-header integer negatives taught "yfinance-family header → integer_number";
  the model now asserts integer on the FLOAT-valued siblings of those headers; the integer
  validator rejects decimals; the veto nulls to unknown. Gold/repr couldn't see it (repr
  went UP anyway); the gate could. Third instance of the additive-retrain pattern
  (v23 categorical, v24 latitude, attneg decimal).
- **compact_ymd collapse 1,987→430**: genuine YYYYMMDD date columns (`CLM_FROM_DT
  20080707…`) now integer_number/unknown — the 10/12-digit→integer signal dragged the
  8-digit neighbour.
- whitespace_separated over_emit 2,707→14,133 (entity_name/plain_text inflow) and
  docker_ref 1.5× — smaller, same bucket.

## Recommended round 2 (one more overnight cycle)

1. **Shape-split the financial family**: mine the float-valued columns of the same
   yfinance headers as decimal_number POSITIVES (a few hundred, capped) so the header
   signal stops implying integer; keep integer-valued → integer_number.
2. **Teach the unix boundary both ways**: financial-header epoch-range 10-digit columns
   (the ~300 clearly-financial rows of the round-1 EXCLUDE bucket) → integer_number.
3. **compact_ymd preservation positives**: 8-digit month/day-valid columns under date-ish
   headers → compact_ymd (capped ~200).
4. Rebuild blend → FTMB → proxy (expect the unix flag again — pre-adjudicated) →
   overnight → full gate stack.

**Round-2 blend BUILT and audited (2026-07-06 afternoon, awaiting launch go):** 3,805
mined rows = round-1 families unchanged + `decfix` 597 (corpus-mined financial float
columns → decimal_number positives; the 37,784-column damage set itself lives in <5-value
per-ticker files the loader can't train on, so the signal comes from the corpus's larger
financial tables) + `ymdfix` 193 (month/day-valid YYYYMMDD columns → compact_ymd).
**`unixfix` was dropped as structurally empty**: every corpus candidate for
financial-header epoch-range 10-digit lives in <5-value files (300/300 measured) — the
financial→unix leak stays a documented residual for now.

Round-1 artifacts stay: models/attneg-s42..44, output/attneg-retrain/ (passes + gate),
output/distillation-attneg/. Nothing was swapped; models/default untouched.

## Derisk review — 2026-07-06 evening (6 agents; verdict: round 2 as staged would have NO-GO'd again)

Live-model verification overturned the round-1 diagnosis: the learned shortcut is
**constant/near-constant numeric column → integer_number** (94% of the decimal damage is
constant-like; only 20% financial-headed; synthetic probes flip on constancy, not
header). decfix-as-staged covered 0.2% of the damage population. compact_ymd's true
damage route is constant repeated dates asserted iso_8601 → vetoed; part of its headline
loss is baseline-error CORRECTION (8-digit financials wrongly called dates). The ws
inflow is small-vocab multi-word phrase columns. The see-saw risk (decimal positives
re-opening npi) is REFUTED (no 10-digit floats exist; all round-1 negatives retained).
Literature validated counterfactual balancing + contributed the negative-denoise rule
and worst-slice eval. Substrate: this session's r2_*.json scratchpad files + the
workflow journal.

**Blend v3 (BUILT, audited 2026-07-06 evening): 102,451 base (10 poison ws rows
removed) + 5,244 mined.** Changes vs v2: decfix re-mined from the damage transitions
themselves, constant-first (1,195 rows; 6,906 constant candidates found at ≥5 values —
the earlier "damage is untrainable" finding was an artifact of sampling only tiny
files); txtpres 782 entity/plain preservation positives from the ws-gainer transitions;
datecw 54 'date'-header→iso counterweights; integer negatives denoised (date-shaped
non-financial dropped); builder now reports per-header label entropy + fails on silent
zero buckets.

**Round-3 launch package (one command):** `TAG=attneg2 ./scripts/overnight_attractor.sh`
→ FTMB → 20-epoch proxy (a NO-GO flagging EXACTLY {unix_seconds} is pre-adjudicated and
continues; anything else stops) → **damage-recovery precheck** (new instrument,
`scripts/damage_recovery_check.py`: profiles the six named damage/stay-dead sets with
the proxy model; pre-registered thresholds decimal/ymd/ws ≥50% recovery, npi/upc ≥90%
stay-dead; BLOCKING) → 3-seed overnight → post-train checks. Release bar pre-registered
per the review's trigger table; release tooling needs two patches before /release
(5-file dual-encoder HF upload list; stale hardcoded taxonomy count).

## Pre-registered swap + release bar — author-approved 2026-07-06 evening, BEFORE round-3 results

Author authorisation ("proceed with your recommendations"): if ALL FOUR conditions below
hold on the round-3 (attneg2) results, the swap and full release proceed same-day without
a further ask; if ANY misses, stop — no third blend iteration, bank the work.

1. **Gold, same-day regenerated for BOTH models:** candidate ≥ the current default on
   the current fixture (default reads 845/988-era; regenerate both at decision time —
   never compare to a stale snapshot).
2. **Representative band:** candidate ≥ 0.712 (the shipped default's current reading);
   no advisory flag.
3. **Corpus-honest gate (fresh-vs-fresh):** triggered labels ⊆ the pre-registered table —
   ACCEPTABLE: npi / upc / aba_routing collapse (gated-YDF checksum shape-matcher false
   alarms; residual npi marginal ≤ ~2× genuine mass per the round-1 criterion),
   unix_seconds oracle_fp with oracle-confirmed support RISING. BLOCKING: decimal_number,
   compact_ymd, whitespace_separated, word/unknown net-confirmed-loss beyond the
   composition-aware netting, or ANY label not named here.
4. **Damage-recovery at convergence:** the six named sets green on the best seed
   (decimal/ymd/ws ≥50% recovered, npi/upc ≥90% stay-dead), re-scored with the full
   model, not just the proxy.

On pass: apply swap (`ln -sfn attneg2-s<best> models/default`), `/release` (model + patch
binary; release-skill dual-encoder patches applied 2026-07-06), merge `figi-checksum` to
main. Residuals that stay open regardless: financial→unix leak (untrainable at the
min_values floor), whitespace validator tightening (next campaign, gate-creditable),
W4 founder-style + short-code round-2 mining.

## Round-3 (attneg2) results vs the pre-registered bar — 2026-07-07 04:00, THREE OF FOUR PASS

Best seed attneg2-s44 (35/44/47-epoch seeds; gold table s42 0.854 / s43 0.858 / s44 0.870
on the overnight scorer). Same-day, both models, current fixtures:

| bar condition | result | verdict |
|---|---|---|
| 1. gold (same-day) | candidate **847/988 = 0.857** vs default 845/988 = 0.855 | **PASS** (+2) |
| 2. representative | candidate **186/260 = 0.715** vs default 185/260 = 0.712 | **PASS** (+1) |
| 3. gate triggers ⊆ table | npi/upc/aba collapse ✓ (npi 42,675→793, cleaner than round 1) BUT four unlisted labels banded | **FAIL** |
| 4. damage recovery at convergence | all six sets green (decimal 87.9/97.5%, ymd/ws 100%, npi/upc stay-dead 100%) | **PASS** |

**Per the author's all-or-stop authorisation: NO swap, NO release, NO third blend.**

The four unlisted triggers, characterised per-column:
- **compact_ymd oracle_fp — the real one.** Marginal 1,987→21,173; 1,758 in-sample
  integer→ymd inflow is 8-digit FINANCIAL figures (`ebit 15800000`, `goodWill 90000000`,
  `otherCurrentAssets 41308000`) plus GAME_ID constants — the reverse "constant 8-digit →
  date" shortcut the literature review predicted; the 54 datecw counterweights were too
  small against 198 constant-date positives. Note oracle-confirmed ymd support DOUBLED
  (1,733→3,340): the repair also worked; it over-shot. **Root enabler: the compact_ymd
  validator is still SHAPE-ONLY (`^\d{8}$`)** — `41308000` = month 80 keeps the label, so
  the veto can't strip the junk. Month/day range validation is W2 item 7 from the original
  audit, never shipped.
- **amount_accounting over_emit** 2,776→11,575 (4.2×) — decfix's financial floats
  neighbourhood; needs the same per-column look before any fix.
- **cpt oracle_fp** 922→1,749 — the "any 5-digit" attractor, small.
- **isbn collapse** 8,024→1,717 — 13-digit GS1 checksum class; gated-YDF asserts isbn
  correctly 16.8% of the time (reliability table), so this is almost certainly the same
  shape-matcher false alarm as npi/upc/aba — but it was not pre-registered, so it does not
  self-adjudicate.

**Recommendation (author decision):** a deterministic trim round, not a fourth training
run — wire compact_ymd/compact_mdy/compact_dmy month/day range validation (the
never-shipped W2.7; converts the ymd junk into veto-strippable assertions), assess
amount_accounting/cpt the same way, then re-gate CHEAPLY via the rule-mode fast path with
attneg2-s44 held constant. If the trimmed gate's trigger set collapses to the checksum
class (npi/upc/aba/isbn), re-present the bar with isbn's reliability evidence. attneg2
stays banked and unswapped until then; models/default untouched.

## Trim re-gate results vs the bar — 2026-07-07 09:20, STILL 3 OF 4 (gate NO-GO), STOP

Both models re-passed on the range-validated binary (`scripts/attneg2_trim_regate.sh`,
`results/attneg2_trim_regate.log`), then same-binary gate + gold + repr.

| bar condition | result | verdict |
|---|---|---|
| 1. gold (same-day, trim binary) | candidate **849/988 = 0.859** vs default 845/988 = 0.855 | **PASS** (+4) |
| 2. representative (trim binary) | candidate **186/260 = 0.715** vs default 185/260 = 0.712 | **PASS** (+1, no flag) |
| 3. gate triggers ⊆ table (`gate_attneg2-trim.json`) | compact_ymd CLEARED ✓, but 3 non-table labels still band | **FAIL** |
| 4. damage recovery (round-3 full-model) | six sets green | **PASS** |

**The trim fixed its target and unmasked a hidden one.** compact_ymd left the trigger
set (the range validator routes the 8-digit financial/ID junk to integer; `20171231`
keeps the label, `15800000`/`10102373` fail). The remaining 7 triggers:

- **npi 42,675→793, upc, aba_routing** — in the pre-registered checksum table (gated-YDF
  shape-matcher false alarms). ACCEPTABLE.
- **isbn** collapse — same checksum class (gated-YDF 16.8% reliable). Blessable per the
  re-presentation note; not the blocker.
- **compact_mdy** collapse (base_correct 1088 → cand 79) — NEW, **exposed not created by
  the trim**. With the shape-only `^\d{8}$` gone, the candidate's pre-existing mdy recall
  hole is visible: it demotes genuine constant-repeated MMDDYYYY dates (`BOOKING DATE`,
  `ARREST DATE` = `10262016`) to word/unknown — the round-1 constant-numeric→not-a-type
  shortcut, repaired for ymd (blend v3 `ymdfix`) but never for mdy. NARROW: 29 in-sample
  cols, 2 header names, ~5 criminal-justice datasets. NOT trim-fixable (Sense-stage loss,
  lands on word/unknown before Sharpen). Fixable only by a word/unknown→compact_mdy
  recovery guard (no-retrain playbook) or a 4th blend with mdy repair negatives (ruled out).
- **cpt** oracle_fp (922→1,738) — salaries/percentiles/hertz → medical codes
  (`Median`=50000, `sample_rate_hertz`=22050); baseline AND oracle both say
  integer_number (247 in-sample FPs, 30 names, 9 earnings/survey datasets). **The
  numeric-attractor disease RELOCATED**: squeezing npi(10-digit)/upc(12-digit) pushed
  5-digit mass onto cpt (negatives were constant-numeric→integer; salaries vary, so they
  weren't covered). NO cheap fix — CPT has no checksum and salaries fall inside its numeric
  range; only a retrain mining 5-digit cpt negatives closes it. NOT in table.
- **amount_accounting** over_emit 4.2× — benign: 355 empty pandas index columns
  (`__index_level_N__`, `Unnamed: 0`) + 243 within-family `amount→amount_accounting`
  sibling reshuffle + 326 stable; only 5 real FPs (color_rgb). Low harm; not in table.

**Decisive fact: gold is structurally blind to the whole trade.** compact_mdy, cpt,
amount_accounting, npi, upc are ALL absent from the 988-col gold fixture — so the +4 gold /
+1 repr come from elsewhere and cannot see the relocation. The corpus-honest gate is the
only instrument that sees it, and it is BLOCKING (H05): **no headline overrides a blocking
NO-GO.**

**Recommendation (author decision): NO-GO — stop and bank, per the all-or-stop
pre-authorisation.** The retrain killed the npi/upc attractor broadly (34+6 datasets) but
could not clear the gate without (a) relocating the attractor to cpt and (b) exposing an
unrepaired mdy date-recall hole — neither cheaply/deterministically fixable for THIS
candidate (compact_mdy needs a new recovery guard AND cpt still blocks; cpt needs the
ruled-out 4th blend). Shipping it would relocate error, the one thing the campaign is
committed not to do. **Round-4 recipe seed (future campaign):** one retrain mining hard
negatives across ALL numeric-code attractors simultaneously — npi/upc (constant + varying),
**cpt 5-digit salaries/hertz**, aba — plus **compact_mdy constant-date repair** (mirror the
ymdfix family). attneg2-s44 stays banked as the round-4 base (value encoder co-located,
release-ready if the author instead elects to WAIVE cpt+compact_mdy — a deliberate H05
override, not recommended). models/default untouched; no artifact cleanup until the author
rules (candidate lives among the banked dirs).

## What we don't know yet

- Whether header-carrying negatives shift the header branch for integer_number/plain_text in a way
  that costs recall elsewhere (base distilled rows are header-less) — the proxy is the instrument.
- unix_seconds/decimal_number base masses in the FTMB (checked at read_ftmb time; if unix_seconds
  base is small, +150 real epochs is a meaningful recall gain but also a distribution shift).
- Whether a 10-epoch proxy is sensitive to text-side drift from the plain_text addition (the band
  was calibrated on numeric/categorical explosions).
