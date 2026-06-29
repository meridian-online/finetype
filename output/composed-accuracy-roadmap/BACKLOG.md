Anchors verified against live code: `CORROBORATION_SCOPE` (mod.rs:1736), `schema_fail_demotion` (value_sharpen.rs:463, gated 416), `checksum::resolve` registers isbn/aba/cusip/sedol but **not npi** (checksum.rs:168), `structured_string_refinement` (mod.rs:2211), `checksum_substance_guard` (mod.rs:2340). The consolidation and stress-test are accurate to the tree. Final deliverable:

---

# FineType composed-accuracy roadmap: 0.812 → the wall

> **PROGRESS LOG (live baseline is 0.864, not 0.812 — this doc's headline is stale).**
> - **2026-06-28** re-baseline: live 0.6.38 = 795/931 = 0.854 (the Sharpen campaign + gold
>   re-adjudication banked since this doc was written). Anchor errors on `rescore/live_errors.tsv`.
> - **2026-06-28** #12 increment suppression: narrow None/Some(false) slice only = 0.854→0.856 (+2),
>   commit acc513c. The meaningful default-demote arm was a corpus-honest NO-GO (98% oracle collapse).
> - **2026-06-29** #13 id-residual veto recovery + #14 binary full-column (commit 5952ff7):
>   **0.856→0.864 (797→804/931), corpus-honest GO, 0 regressions.** #13 banks +7 (ipni_id,
>   BenchmarkName×4, coord_id, ttxid); #14 is gold-neutral (production fix). See memory
>   `tierb-id-residual-veto-recovery-shipped` — incl. the veto-layer-not-post-sharpen-guard finding
>   and the native-gate-via-`gate_candidate_from_cache.py` recipe (the fast resharpen gate is blind
>   to veto-layer rules). Confirms the staged 0.86→0.88→0.90 is optimistic: DEMOTE arms collapse,
>   RECOVERY arms clear. Realistic ceiling ~0.86, not 0.88.

## 1. Headline — the honest reachability read

**Bankable engine fixes carry you from 0.812 to ~0.88. Med-risk header work reaches ~0.90. Gold re-adjudication (a curation change, not an accuracy gain) reaches ~0.93. The wall is ~0.93–0.94. 0.98 is below the irreducible tail and is not reachable on this corpus with deterministic Sharpen work.**

The arithmetic the owner needs: 0.98 = 909/927 = **18 errors allowed total**. We have 174 errors today. Of those:

- **55 are bankable** — value-determinable, low over-emission risk, ship through the normal gold gate. ~36 land via low-risk Sharpen rules; the rest need a value re-pull or ride a gated pilot.
- **119 are ceiling**: 50 gold-debatable, 19 gold-mislabeled (engine already right), 31 unscoreable (no values on disk), plus the currency wall and the geo/entity retrain attractors.

The single most important first move costs **zero engineering**: ~8 of the 174 (idx 27, 29, 63–67, 78) are **phantom** — the error file is a stale snapshot and live 0.6.37 already emits the gold label (country closed-set, coord_header_veto, postal_header_veto). **Re-score on live before spending an hour.** True starting accuracy is likely ~0.821, not 0.812.

The wall is structural, not a backlog gap. **148 of 174 errors sit on contested tiers** (lens-consensus / llm-2panel) where the panel may have split, and **31 errors can't be scored at all** because the source values aren't on disk. The gold corpus is curated-*hard* by design; the gap from ~0.94 to 0.98 is gold re-curation plus an unproven retrain (additive gold-target retrains are 0-for-6 because composed is rule-bound), not deterministic rules.

## 2. Ranked spec backlog (value-per-risk)

Each row is one shippable, gold-gated spec. "cols" = gold columns the spec lands (efficacy after stress-test haircuts, not raw idx count). pp at 927-col denominator.

| # | Fix | Lever | cols | pp | Over-emit risk (stress-test) | Effort | Gate |
|---|-----|-------|------|-----|------------------------------|--------|------|
| 0 | **Re-score on live 0.6.37** (phantom recovery) | re-measure | 8 | +0.86 | n/a — already correct in live | S | none (measurement) |
| 1 | **schema_fail_demotion scope +6 labels** (orcid, aws_arn, cpt, http_method, boolean.terms, ethereum_address) | sharpen_rule | 6 | +0.65 | **low — ship-as-is**. Demote-only, gated to closed validators; can't push untargeted cols *into* a wrong type. Caveat: 42/47 low-card hipc_ids route to word not alphanumeric_id (miss, not relocation); CPT validator passes any 5-digit int (a miss) | S | corpus-honest (confirm http_method/boolean.terms) + gold |
| 2 | **URL recovery reader** in structured_string_refinement | sharpen_rule | 4 | +0.43 | **low — ship-as-is**. `//host.tld` mandate self-limits; residual-gated, mutually exclusive with the 3 existing readers | S | gold + light corpus-honest |
| 3 | **utc CORROBORATION_SCOPE +datetime.offset.utc** (ms-offset floats stay decimal) | sharpen_rule | 4 | +0.43 | **low — ship-as-is**. One-line; decline-only at the <=10% contradiction bar; code's own comment names this target | S | gold + light corpus-honest |
| 4 | **Numeric residual fallback** in veto_shape_fallback (+ mirror into schema_fail_demotion path) | sharpen_rule | 2 (+3 on re-pull) | +0.22 | **low — ship-as-is**. Post-hard-veto only; decimal/integer beats unknown. *Verify the leading-zero guard rejects zero-padded codes* | S | corpus-honest + gold |
| 5 | **Epoch integer recovery** (decline identifier header-hint + veto_shape_fallback epoch arm) | veto_misfire_fix | 6 | +0.65 | **low**. Decline-only + post-veto on a narrow [1e9,2e9]/[1e12,2e12] range | M | corpus-honest + gold |
| 6 | **NPI Luhn enrollment + n_unique==1 backstop** | sharpen_rule | 4 (82,84,85,86) | +0.43 | **low**. Strict checksum both sides; npi is wholly unchecked today (not in resolve()) | M | corpus-honest + gold |
| 6b | **ISBN-10 recovery** (checksum + ISBN10 header) | sharpen_rule | 3 (37,38,39) | +0.32 | **low**. mod-11 checksum verified on all sampled values | S | gold |
| 6c | **Checksum keep-threshold 50%→~90%** | sharpen_rule | (hardens 82) | — | **med — narrow/pilot separately**. True-positive demotion risk: noisy real isbn/aba cols at 80–85% valid lose recall | S | **pilot with gold isbn/aba *recall* cross-check** |
| 7 | **entity_name/word long-prose demotion** (median len >100 → plain_text) | sharpen_rule | 2 | +0.22 | **low — ship-as-is**. LENGTH direction, orthogonal to the litigated short-vocab NO-GO. Precedent measured 0% false-demotion | S | corpus-honest + gold |
| 8 | **full_address whitespace guard** | sharpen_rule | 2 | +0.22 | **low — narrow to whitespace-only**. Drop the number/comma clause (false-demotes comma-less foreign addresses; full_address is locale_specific) | S | corpus-honest watching intl-address recall |
| 9 | **Retired state→region relabel** (LOCATION_TYPES + override tables / scoreboard alias) | sharpen_rule | 1 | +0.11 | **low — ship-as-is**. Dead-alias rename to live successor; cannot emit a new concept | S | card-0019 no-regression gold |
| 10 | **decimal→integer IS_FLOAT demotion** (extend feature_sharpen F5) | sharpen_rule | 1 | +0.11 | **low**. Hard IS_FLOAT signal | S | corpus-honest + gold |
| 11 | **unlocode value-format veto** | sharpen_rule | 1 | +0.11 | **low**. unlocode rare; demote formally-invalid assertions | S | gold |
| 12 | **Increment over-attractor suppression** (increment→integer default, kill-switched) | sharpen_rule | 8 | +0.86 | **low — ship-as-is**. choice-0096 textbook; integer is the safe parent (increment precision 0.056); can't relocate onto untargeted cols | M | **corpus-honest (blocking, H05) + RHH kill switch** |
| 13 | **veto_shape_fallback id-residual recovery a/b/c** (out-of-charset alnum, pure-letter→word, bare msg-id off url) | veto_misfire_fix | 4 (56,58,59,164) | +0.43 | **low**. Demotion/recovery-only | M | corpus-honest + gold |
| 14 | **binary_vocab_veto full-column feed** | sharpen_rule | 2 | +0.22 | **low**. max>1 is the checkable signal (can't confirm from {0,1} sample) | S | corpus-honest + gold |
| 15 | **epoch plausibility-range demote** (76 impossible-range) | sharpen_rule | 1 (+2 weak) | +0.11 | **low** for 76 (range-verifiable); 75/77 lean on constancy (debatable) | S | corpus-honest + gold |
| 16 | **year index/counter-header demotion** (81 conversation_no) | header_corroboration | 1 (+2 weak) | +0.11 | **low** for 81; 79/80 debatable. Do NOT take the inverse (kills unheadered year cols) | S | corpus-honest + gold |
| 17 | **Coordinate lon/lat header tiebreak** (split from the geo cluster) | header_corroboration | 1 | +0.11 | **low — ship-as-is**. Only in the all-\|v\|<=90 ambiguous branch | S | gold + light corpus-honest |
| 18 | **id-residual digit-separator arm** (IPNI/composite keys → alphanumeric_id) | sharpen_rule | 3 (57,60,61) | +0.32 | **med — false-friends** (year-month, ranges, phone). Gate on "not a valid date" + high cardinality. +isbn→veto_safe (low) | M | **blocking corpus-honest pilot** |
| 19 | **float-epoch .0-strip + time-ish header** | header_corroboration | 3 (7,8,9) | +0.32 | **med**. Header gate load-bearing; both labels defensible (lens-consensus) | S | blocking corpus-honest pilot |
| 20 | **IATA closed-set + team-header anti-corroboration** | sharpen_rule | 1 clean (147) +2 debatable | +0.11–0.32 | **low** for 147 closed-set; 148/149 med (need IATA list added to repo) | M | blocking corpus-honest pilot |
| 21 | **TLD recovery + false-friend demotion** | header_corroboration | 2 (163,166) | +0.22 | **med**. Leading-dot collides with file extensions; needs IANA TLD list + header gate | M | blocking corpus-honest pilot |
| 22 | **Geo header-hint value-corroboration gate** (state Static/Dynamic) | veto_misfire_fix | 3 (143,144,145) | +0.32 | **med**. Gate direction is safe (require positive evidence); recall risk on sparse state cols. 141 region single-token is higher risk | M | blocking corpus-honest pilot |
| 23 | **Country-name closed-set** (header 'country' + ISO gazetteer) | header_corroboration | 2 (26,28) | +0.22 | **HIGH — pilot in isolation**. v22 country −31.5% battleground; replace generator seed-list with the ISO country-NAME closed set; idx 28 locale_code→country_code collides with fr/de/us valid ISO codes | M | **blocking corpus-honest, kill switch, watch locale_code→country_code relocation** |
| 24 | **Admin-subdivision county/borough→region** | header_corroboration | 1 (33) (+2 re-pull) | +0.11 | **med-HIGH**. The litigated entity_name→region path (3,752 oracle-refuted moves), header-narrowed | M | **blocking corpus-honest, kill switch, watch entity_name→region** |
| 25 | **Misc structural** (sql_minute leaf @18; array veto @107; path-shape @131) | sharpen_rule / new_type | 2–3 | +0.22 | **low** (18 exact-anchored, 107 demote-only); **med** (131 path-shape) | M | gold + corpus-honest |
| 26 | **embedded-CSV detector** (container.object.csv) | sharpen_rule | 1 | +0.11 | **low**. Header-arity==value-arity gate is narrow | M | gold + corpus-honest |
| 27 | **username / plain_text→alnum / last_name all-caps** | header_corr / sharpen_rule | 3 (40,50,151) | +0.32 | **med**. Header/shape does the work; keep header sets closed + high-card guards tight | S each | blocking corpus-honest pilot |
| 28 | **GOLD RE-ADJUDICATION — clean 14** (iso_ms 15/16/17, zip 83, boolean 87, type-names 122/123, single-token word 132/134/135/136/138/139, windows_path 140) | gold_readjudicate | 14 | +1.51 | **n/a — engine already correct** | M | re-adjudication panel, no engine gate |
| 29 | **GOLD RE-ADJUDICATION — defensible 14** (granularity 13/14, species/code residuals 41/49/51/52/53/155, tri-signal 158, word/entity 127/133/137/159, not-a-TLD 168) | gold_readjudicate | up to 14 | up to +1.51 | **n/a** — contested; panel may keep gold | M | mixed-panel blind + adversarial |

## 3. Staged roadmap

| Milestone | Cumulative | What gets you there |
|-----------|-----------|---------------------|
| **~0.86** (≈797/927) | start by re-scoring on live | #0 phantom (+8) then Tier-A low-risk value-determinable: #1–#11 (schema scope, URL reader, utc scope, numeric fallback, epoch recovery, NPI+ISBN checksums, long-prose, full_address, state→region, IS_FLOAT, unlocode). ~36 engine wins + 8 phantom. Every fix is demote/decline-only or strict-checksum/closed-enum — the safe lever the 0-for-6 history points to. |
| **~0.88** (≈813/927) | +#12–#16: increment suppression (+8, choice-0096 kill-switched), id-residual a/b/c (+4), binary full-column (+2), epoch-76 (+1), year-81 (+1). All deterministic; increment + id-residual ship with both-sides evidence + kill switch + blocking corpus-honest. |
| **~0.90** (≈831/927) | +#17–#27: coordinate tiebreak, float-epoch, IATA, TLD, geo state-gate, country closed-set, county→region, sql_minute/array/path, embedded-CSV, username/alnum/last_name. **This is where the wall starts pressing** — country/region/iata/tld are value-identical boundaries where the header does the work, each a known battleground (v22 country −31.5%). Expect **~1-in-4 to NO-GO and relocate**; net ~+18 not +22. Needs an **IANA TLD list and an IATA airport-code list added to the repo**. Run #23/#24 individually through the blocking corpus-honest gate (H05). |
| **~0.93** (≈859/927) | +#28/#29 gold re-adjudication (+14 clean, +up to 14 defensible). **Zero engine risk, but this is a metric/gold change, not an accuracy gain** — the engine is already correct or defensible on these. Requires a mixed-panel blind + adversarial pass on contested-tier rows. Corrections will skew away from the model (re-adjudication cleans gold rather than inflating it); the 14 clean are free, the other 14 a panel may keep. |
| **THE WALL (~0.93–0.94)** | The remaining ~60 errors are irreducible to Sharpen: currency (5), geo/entity retrain attractors (18), entity/word/plain_text model-blind boundary (~8), values_missing remainder (~11–16), plus the Tier-C NO-GO relocations. **0.98 is below this tail.** |

## 4. The contested tail (~119 ceiling columns)

The wall is four distinct populations. None move via a value rule.

- **Currency value-blind (5: idx 19–23).** Bare numbers (`68070.52`, `265000`) with zero in-value currency symbols — value-identical to decimal/integer; the model itself emits decimal. Three are demoted by *deliberate shipped vetoes* (amount_bare_number_veto, si_number_override). `netIncome`(integer) is value-identical to `base_salary`(amount), so any header-keyword split mis-calibrates and choice 0042 deprecates header guards. **Documented hard wall** (memory `currency-gold-gap-is-column-level`; additive currency retrain 0-for-5). Only move: a larger symbol-bearing currency gold set to calibrate a header split. **Accept the ceiling.**
- **Geo/entity retrain attractors (18: idx 32, 105–112, 114, 124–126, 130, 141, 152/153, 156).** `name`/`place`/`street`/`neighborhood` headers collapse to city/region/street/full_address; the categorical→entity_name vocab collapse (124/125/126/156) **is the litigated corpus-honest NO-GO** (3,752 oracle-refuted entity moves — text_vocab_override is scoped to `word` ONLY for exactly this). **No safe Sharpen rule.** Only a Sense retrain with cleaner free-text/entity negatives helps, and that's 0-for-6 historically. This is the short-code/user_agent/entity residual already flagged in CLAUDE.md (task t-000133e418). **Do NOT chase with rules.**
- **Model-blind text boundary (~8: idx 113, 116–118, 121, 128, 129, 157).** entity_name vs plain_text vs word judgment calls (xmldoc refs, blog titles, UI labels, constant `PA`). The natural lever (small-vocab→demote) is closed by the same NO-GO. Move only via gold re-adjudication or accept.
- **Values-missing (31, partly listed in idx set).** Unscoreable — no source values on disk. ~15–20 become bankable once re-pulled *and* their named fix ships (34/35 county→region, 74/103/104 numeric fallback, 48 orcid, 95 increment, 146 state-gate). The rest are genuinely ambiguous. **This is a data-ops re-pull, not an engine bug** — do it before the gold-re-adjudication panel so the panel can see values.

The honest framing for the owner: **148/174 errors are on contested tiers**. "Moving" them means re-adjudicating gold (Tier D), not improving the engine. The engine is already correct or defensible on ~33 of them.

## 5. What this means for the next training run

**Composed is rule-bound — a retrain is not the lever for the gold targets, and the backlog above proves it.** Of 174 errors, exactly 0 are reliably fixed by a retrain (the 18 data_lever attractors are the *only* retrain-shaped cases and they're 0-for-6). Run the Sharpen/curation program first; it carries 0.81→~0.93 with bounded risk.

The retrain still has a role, but a narrow one: **the geo/entity over-emission attractors (city/region/street/entity_name on `name`-family headers) are gold-invisible corpus-scale warts that only a model with cleaner negatives fixes.** That's the existing t-000133e418 follow-up. Gate it gold-parity + gold-adjudicated relocation review (choice 0104), NOT corpus-honest GO (structurally unpassable by any retrain). Don't expect it to move the gold headline — expect it to stop the over-emission the gold corpus can't see.

**Bundle the free latency wins into that retrain cycle, not into the Sharpen sprint.** The Sharpen program is where the gold pp lives; keep it the priority. When the retrain runs, fold in the short-code/user_agent suppression that needs better training data rather than a rule.

## 6. One line for the owner

**Low-risk deterministic Sharpen fixes take composed gold from 0.81 to ~0.88 (re-score on live first — ~8 errors are already gone), header-corroboration pushes ~0.90, gold re-adjudication reaches ~0.93, and the wall is ~0.94 — 0.98 is below an irreducible tail of currency (value-blind), geo/entity attractors (retrain-only, 0-for-6), and 31 columns with no values on disk, so the last 4 points are gold re-curation and an unproven retrain, not engine work.**