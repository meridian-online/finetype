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

**Root cause (investigated 2026-06-24 — NOT worse data):** v19 and potion-8M train on
the *identical* recipe — `build_ftmb_v5_potion` literally reuses a constant named
`V19_ARGS` (same `distillation-v3` corpus, same `data/label_remap.json`, same
1200/0.7/600 blend, same hard-negatives, format v4). The human distilled corpus is
Sherlock-78 (`artist`, `city`, `name`…); the collapsed taxonomy types come from
**synthetic generation** (`finetype generate`, `--synthetic-columns 1200`).

The numeric_code collapse is a **synthetic-generator flaw, not drift**: 92% of generated
`numeric_code` values are plain all-digit integers (`356`, `7130`, `580`) — only 8% carry
the leading-zero signal that defines the type. They are indistinguishable from
`integer_number`. A fresh flat softmax cannot carve out a class whose examples are 92%
identical to a larger neighbour, so it collapses numeric_code → integer_number. This is
decision 0096 (residual/attractor types are rule-shaped, not trainable into flat softmax).
v19 emitting numeric_code (59k, only ~12% oracle-confirmed) is the *anomaly* — a quirk of
its specific unreproducible historical weights, and imprecise at that.

**Implication:** the fix for numeric_code is the deterministic leading-zero RULE
(t-0000a86b), not better data — and it helps v19 too. The other triggers (user_agent 7.2×
over-emit, isbn collapse, si_number over-emit) likely have the same generator-quality
root cause and need the same audit. So "fix the data drift" is more precisely **"audit the
synthetic generators + add value-based rules for the attractor types,"** NOT "v19 had better
data" (it didn't — it's the same data).

### Re-reading the NO-GO triggers by trust level (2026-06-24)

Prompted by "numeric codes have partial leading-zero coverage" — investigating the real
population flipped the headline trigger:

- **numeric_code collapse is largely an ORACLE ARTIFACT, not a regression.** Of the 5,342
  oracle-"confirmed" numeric_code cols, **0% have any leading zero and 88% are headed `id`**
  (sequential integers like 2038329, 2038328…). v19 was OVER-emitting numeric_code onto
  integer ID columns; potion-8M correctly declines. The gate's "collapse" counts that as a
  loss because the referee is gated-YDF, which CLAUDE.md says is 42%-wrong on contested
  ground and must not adjudicate without a gold cross-check (numeric_code's validator passes
  any digit string → rubber-stamps the over-emission). The leading-zero rule wouldn't fire
  here (0% leading zeros) and shouldn't (they're IDs). **Discount numeric_code.**
- **The NO-GO survives on the OVER-EMIT side.** 11 over-emit/oracle_fp triggers (oracle-AWARE,
  the trustworthy band) — potion-8M over-asserts `user_agent` 7.2×, `username` 3.4×,
  `currency_code` 3.2×, `version` 2.9×, `si_number` 2.5×, etc. These are CREATED false
  positives, still ≫4, and a real defect. So the problem is **over-emission of synthetic
  types**, not "lost numeric_code."
- `isbn` collapse (1,733) is more credible than numeric_code (real checksum validator) —
  worth a gold check. `compact_ymd` (611) minor.

**Refined fix direction:** the real defect is the fresh retrain **over-emitting** a cluster
of synthetic types (user_agent/username/currency_code/version/si_number). That points at the
synthetic *generators* for those types (too-broad examples → over-prediction), not a
numeric_code recovery problem. Audit those generators first; numeric_code is a separate,
precision-safe rule (t-0000a86b) for the genuine leading-zero/header-driven codes.

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

## 6. Rule co-adaptation to v19 — quantified (2026-06-24)

**The wall to beating v19 composed is the v19-tuned Sharpen layer, not the embeddings.**

Raw→composed transition grid on gold (reframe, n=927; raw = pure model argmax,
no veto/Sharpen):

| model | raw acc | composed | FIX | BREAK | net lift |
|---|---|---|---|---|---|
| v19 | 0.471 | 0.797 | 335 | 33 | **+302** |
| potion-8M | 0.520 | 0.794 | 295 | 41 | **+254** |

potion-8M's *raw* model is more accurate (0.520 vs 0.471, ~+45 cols), but the
deterministic layer gives v19 **48 more columns of net lift** — which cancels the raw
lead, yielding the composed tie (~3-col gap). The rules convert v19's errors better.

**Per-rule attribution of the gap (gold; small-sample, directional):**
- *Rules that recover v19 but not potion-8M:* `veto_fallback:id` (5), `veto:hash` (5).
- *Rules that BREAK potion-8M (right→wrong) but not v19:* `header_hint_cross_domain`
  (link_id/state, 6), `veto:alphanumeric_id` (3), + singletons. 14 cols total.

So the co-adaptation is **real but localized to a handful of rules**, not broad. The
header-hint and alphanumeric_id-veto rules misfire on potion-8M's (different, correct)
raw predictions and turn them wrong.

**Implication / path to BEAT v19:** potion-8M's raw Sense already wins; re-fitting the
~dozen co-adapted rules to its error distribution (fix the header-hint/alnum-veto
misfires, port what veto_fallback:id+hash do for v19) would push composed past v19's
0.798 — a tractable rule edit, not an architecture rebuild. This is separate from the
SHIP blocker (corpus over-emissions: user_agent/currency_code generators).

**Architecture note:** the embed aggregation is a *shared* ceiling (v19 aggregates
identically; gte-tiny clean-slate pools too). Per-value cross-attention (orig Sense,
sense.rs) is the only untested representation path — but given the co-adaptation
finding, re-fitting rules is the cheaper first lever to actually beat v19.

## 7. Rule re-fit traced to mechanism — it's gold-only, doesn't ship (2026-06-24)

Traced the 14 potion-8M gold breaks to their exact rule: **every one is a deprecated
hardcoded header hint (or veto) overriding a CORRECT value-based model prediction**:
- `link_id`/`episode_url_id` → forced to `url` (values are `msg…`/`BV1…` IDs)
- `state` → forced to `geography.state` (values are `"Static"`, a status field)
- `BenchmarkName`/`coord_id` → alphanumeric_id *vetoed* to unknown (they ARE alnum IDs)

This is decision 0042 biting: hints override on column *name*, ignoring *values*. And it
penalises strong models more — potion-8M's raw is right more often (0.520 vs 0.471), so the
blind override clobbers correct answers more often. Real co-adaptation, confirmed at column
level. The principled fix (gate header hints by value-consistency / model confidence) aligns
with 0042/0048 and removes a structural penalty on any strong model.

**BUT — two findings cap its value:**
1. **Gold-only & within noise.** Recovers ~14 cols / 927 ≈ +1.5pp (potion 0.794 → ~0.81),
   inside the ±3pp gold CI. Confirms the thesis; not a significant "beat v19."
2. **Does NOT unblock shipping.** The corpus over-emissions (the actual blocking gate) are
   **raw-model, not header-hint**: profiling shows `currency_code`→UDP/TCP/EDT and
   `user_agent`→prose fire with "(no rule — raw model)". Those are the model over-asserting
   (3-letter attractor; free-text→UA), fixable only by RETRAIN (generator/negative changes),
   not by the rule layer. The header-hint fix touches none of them.

**Conclusion:** the rule re-fit confirmed the co-adaptation but is the wrong lever for
shipping. The ship blocker is raw-model over-emission → the real next step is a retrain with
fixed synthetic generators / hard-negatives for the over-emitted types. The header-hint
value-consistency guard remains a worthwhile *standalone* cleanup (0042-aligned, un-penalises
strong models) but it's gold-only and load-bearing on v19, so it should be its own gold-gated
change, not bundled with the ship push.

## 8. The corpus gate is structurally anti-model-swap (2026-06-24)

"Has any development passed the corpus gate?" — checked every recorded run:
- **GO (21): every one is a RULE change** on v19 (postal-veto, country-code-corrob,
  isbn-checksum-guard, coord-guard, state-code, amount-veto, veto-fallback…).
- **NO-GO: every MODEL retrain, without exception** — v22, v23, latdec×3, mfg-coords×3,
  fusion-v26/v27, m2v-244, potion-8M×2. **0% model pass rate.**

**Why structural:** the gate's oracle is v19's gated-YDF — it measures *deviation from
v19*. A retrain moves 12–25% of all columns (latdec 12.0%, v22 21.3%, v23 24.7%), so the
bands trip by construction; a rule change is a small targeted delta and stays in-band.
CLAUDE.md's own audit says gated-YDF is 42%-wrong on contested ground and must not
adjudicate without a gold cross-check — yet it's the blocking model-swap referee.

**Consequence:** using corpus-gate-GO as the BLOCKING criterion to retire v19 is permanent
v19 lock-in. This — not candidate quality — is why no reproducible default has ever shipped.

**Way out (needs owner decision + recorded choice amending H05's role for model swaps):**
use the gate to FIND relocations, adjudicate them with GOLD not gated-YDF. Already shown
benign: numeric_code (potion more correct), si_number, username. Fix the genuinely-wrong:
user_agent, currency_code. Ship on gold-parity (0.794 ≈ 0.797) + representative +
gold-cleared relocations. Precedent: the 0.6.29 composition-aware band recalibrated the
gate when it false-alarmed (text-vocab NO-GO→GO).

## 9. Toward the swap: choice 0104 + value-guards (2026-06-24)

- **Choice 0104 recorded:** corpus gate = gold-adjudicated relocation review for MODEL
  swaps (blocking only on gold-confirmed regressions); stays a blocking GO for rule changes.
- **currency_code validator → ISO-4217 enum** (was `^[A-Z]{3}$`, the Precision-Principle
  anti-pattern). Gold-safe: v19 holds 0.797, potion-8M holds 0.794. Suppresses the
  currency_code over-emission on UDP/TCP/EDT; real currencies (incl. lowercase) keep the type.
- **Whack-a-mole tell:** with currency_code vetoed, UDP/TCP flip to `iata_code` (the next
  3-letter attractor). Confirms the corpus over-emission is the model over-asserting on short
  codes generally — rule-vetoes shift it, don't solve it. Reinforces 0104 (ship on
  gold-adjudication, not a clean gate) and the spec's ">4 rules → fix the data" guidance.
- **user_agent** (prose → user_agent): NOT veto-eligible (absent from veto_safe.txt), so it
  needs a dedicated value-shape Sharpen rule, OR — given the whack-a-mole — it's better
  addressed by the retrain. On gold it's not a confirmed regression (potion ties v19), so under
  0104 it is not strictly ship-blocking; it is a product-quality wart.

**Remaining to ship potion-8M:** decide user_agent (rule vs accept-as-documented vs retrain),
gold-adjudicate the remaining triggers, then the deliberate release (package → HF → swap →
version → CHANGELOG → CLAUDE.md). The release is a reviewed action, not auto-fired.

## 10. SHIPPED — v0.6.36 (2026-06-24)

potion-8M (m2v8m-s43) is the default, **v19 retired**. ac-04 dual-encoder distribution
built + verified: build.rs embeds the value encoder, from_bytes loads it (embedded binary
runs with no disk models), download-model.sh fetches it. HF upload (5 files) verified; CI
green on main (Linux build + download + drift check); binary release v0.6.36 — all 5
platforms green (incl. Windows MSVC + macOS), GitHub release + Homebrew + install site
updated. Gated by gold parity (0.794 ≈ 0.797) + gold-adjudicated relocation review (choice
0104), currency_code ISO-4217 fix, type-key fallback. Residual short-code/user_agent
over-emission → follow-up retrain (t-000133e418).
