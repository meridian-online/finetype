# ac-00 — username recovery from over-emitted full_name: design grounded in data

**Date:** 2026-06-17 · spec `2026-06-17-full-name-username-veto` · gate AC (doc)
**Substrate:** `eval/gittables/corpus_pass/columns.parquet` (v19/v22-era corpus pass,
2026-05-22), `output/representative-baseline/` (the 2026-06-17 baseline study),
`eval/gold/gold_corpus.tsv`.

## Headline

`full_name` is the model's single largest over-emission: **249,568 corpus columns**.
The dominant mass is login-handle columns the model reads as person names —
**~165k `author` + ~17k `authors`** (HackerNews/GitHub-style handle lists). A clean,
value-based discriminator separates handles from real names, so this is a recoverable
representative-data win.

In plain terms: profile a dump of commit/comment tables and the "Author" column —
full of handles like `tptacek`, `patio11`, `rms` — comes back labelled "full name".
The fix reads the values, sees single tokens with no spaces, and calls it a username.

## The discriminator — internal-whitespace fraction (value-based, 0048)

Real person names are multi-token ("First Last"); login handles are single tokens. The
fraction of a column's sampled values containing internal whitespace separates them
decisively (corpus pass, values exploded from `sample_values_truncated`):

| column (among full_name preds) | values | frac with internal space |
|---|--:|--:|
| **author** | 1,308,646 | **0.005** |
| fullexchangename | 5,688 | 0.547 |
| authors | 95,392 | 0.579 |
| artist | 8,918 | 0.874 |
| name | 11,720 | 0.941 |
| school_name | 1,708 | 0.976 |
| player_name | 172,388 | 0.984 |
| event_name | 10,014 | 0.998 |
| person | 30,188 | 1.000 |
| provider name | 3,610 | 1.000 |

The gap is ~0.005 (handles) vs ≥0.94 (real names) — a threshold at ~0.15 is very safe.
`authors` (0.58) and `fullexchangename` (0.55) are mixed and correctly stay full_name
(or demote elsewhere) — the rule targets the unambiguous single-token mass.

**Rule:** when the resolved label is `identity.person.full_name` AND the sampled values
are handle-shaped (whitespace fraction below threshold AND values match a handle charset:
alphanumeric + `[._-]`, single token) → reclassify to `identity.person.username`.
Sits alongside the existing full_name entity-demotion gate (`column/mod.rs:~852`).

## Corrected scope — gold-NEUTRAL, NOT a gold full_name fix

The prior finding (`output/representative-baseline/finding.md`) and memory
`representative-baseline-is-068` claimed username recovery "also fixes gold's worst
precision leak (full_name P=0.167)". **That is wrong** — verified by joining v19 gold
predictions to gold truth:

`full_name` gold predictions = 6; correct = 1 (P=0.167). The 5 false positives:

| gold column | truth |
|---|---|
| venue_localized_country_name | geography.location.country |
| street_name | representation.text.plain_text |
| longName | representation.text.entity_name |
| gis_nta_name | representation.text.plain_text |
| agency_name | representation.text.plain_text |
| creator | identity.person.full_name (the 1 TP) |

All 5 FPs are **multi-word Title-Case place/org names** — the *opposite* shape from
handles (they all contain spaces). A username (low-whitespace) rule cannot and must not
touch them. The gold leak is a separate, semantic problem (place/org vs person), which
sits at the architectural ceiling (`cardinality-boundary-error-is-real`) and is not in
scope here.

So: **representative-side win** (the ~165k `author` mass; ~12/250 in the baseline study),
**gold-neutral** (gold has 1 username + 1 full_name column; no headline movement expected).
The gold gate's role is no-regression, not lift.

## Why the corpus-honest gate needs care (ac-03)

The relocation is large (~165k full_name→username). The gated-YDF oracle over-emits
full_name itself, so a *correctness-preserving* full_name→username relabel can false-alarm
exactly as categorical→word did (referee speaking a vocabulary the candidate corrected).
Isolate against a same-binary kill-switch-off baseline, and if the collapse band fires on
the handle subset, read the referee in the candidate vocabulary
(`--label-remap identity.person.full_name=identity.person.username`) — but ONLY where the
rule actually fired (handle shape); a genuine full_name→city error must still trip
oracle_fp.

## Limits (over-read discipline)

- The corpus pass predates the four 0.6.27–0.6.29 Sharpen fixes and the reframe; the 249k
  full_name count is directional for sizing, not a current-binary number. The discriminator
  (whitespace fraction) is value-intrinsic and binary-independent — that part transfers.
- The representative username losses were concentrated on `Author` columns from a few
  HN/GitHub dataset families; n=250 supports the family as real and large, but the per-column
  diversity is narrower than the count suggests. The corpus breadth (165k across the full
  pass) is the stronger evidence the mass generalises.
- Raw representative values were sanitised; the value grounding here comes from the corpus
  pass, not the baseline sample.

## Verdict

Gate **cleared**. Discriminator settled (internal-whitespace fraction + handle charset),
scope corrected (representative win, gold-neutral). Proceed to ac-01 (implement) →
ac-02 (gold no-regression) → ac-03 (corpus-honest, isolated + vocabulary-aware).
