# ac-02 prep — where m2v-244 loses composed, and how much an embed can recover

**Date:** 2026-06-22 · spec `2026-06-21` ac-02 prep · inputs: ac-01 pred TSVs (gold, 927 cols)

## Headline — a bigger embed can realistically recover only ~1pp of the composed gap

m2v-244 composed 0.769 vs v19 0.793 = a net 2.3pp gap (34 losses − 12 wins where m2v
already beats v19). Splitting the 34 losses by what could actually fix them:

| bucket | count | fixable by a richer embed? |
|---|---|---|
| **embed-addressable** (Sense wrong, gold NOT a residual type) | **9** | **Yes — the real ac-02 target (~1.0pp)** |
| residual-attractor (Sense wrong, gold IS a residual type) | 25 | **No** — decision 0096, rule-shaped |
| rule-owned (Sense was right, Sharpen changed it) | 0 | n/a |

**So the composed gap is almost entirely Sense-driven (34/34 of the losses are Sense
losses, zero rule damage) — but two-thirds of it is the 0096 residual-attractor pathology,
which a better embed or retrain provably cannot fix.** The honest ceiling for a bigger
potion, judged on these losses, is ~1pp of composed recovery — it will NOT close the v19
gap on its own.

## The 9 genuinely embed-addressable (sibling confusions a richer embed can sharpen)

- **isbn → unix_milliseconds** (3: "Primary ISBN13") — 13-digit ISBN read as a unix-ms epoch.
- **country_code → locale_code** (2: country_id, venue_country) — close sibling confusion.
- **region → http_method** (2: borough, work_location_borough) — wild miss, embed should kill it.
- **unix_milliseconds → ean** (1), **full_name → full_address** (1).

These are real type-vs-type confusions where a sharper representation legitimately competes.
The Sharpen rules are model-agnostic, so fixing the Sense lands the composed for free.

## The 25 residual-attractor losses (0096 — needs a value rule, not an embed)

- **alphanumeric_id → h3** (14: msg_id…) — the single biggest item. The model over-emits the
  "tighter" h3/geohash onto alphanumeric IDs; 0096 says the residual (alphanumeric_id) can't be
  made a flat-softmax winner, so no retrain/embed reliably recovers it. This is the **geohash
  veto** opportunity already on the backlog (earthquake-id-geohash task) — a parallel win,
  independent of ac-02.
- plain_text → entity_name (4), word → state_code/continent/tld (4), numeric_code → integer/postal (2).

## What this changes for ac-02

1. **Judge the candidate on SENSE, not "beat v19 composed."** The embed legitimately competes on
   Sense (gte two-view hit 0.571). Composed beating v19 is unlikely from embed alone because the
   gap is mostly rule-shaped — so ac-03's bar stays **improve-or-hold composed vs the m2v-244
   baseline**, with beating v19 composed an aspiration reached by embed + the alnum→h3 veto together.
2. **Watch the 9 sibling confusions as the embed's report card** — isbn/unix-ms, country/locale,
   region/http_method. If a bigger potion sharpens these, the representation genuinely improved.
3. **The alnum→h3 veto (14 cols) is a separate, bankable rule win** — file/pursue it independent of
   the potion swap; it's the largest single lever on composed and the embed can't touch it.

One line: *a bigger potion's honest prize is a stronger Sense and ~1pp of composed; the other
2/3 of the v19 composed gap is a value-rule problem (alphanumeric-id vs geohash), not an embed one.*
