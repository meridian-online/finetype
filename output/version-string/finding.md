# version_string_recovery — header-gated software-version recovery (2026-07-14)

**Headline:** ~414 corpus columns of software versions (`1.6.1`, `1.11.23`, `1.2.2`) under
`ver` / `version` / `sbt_version` / `repo2docker_version` / `latest_stable_release_number`
headers — which FineType was throwing away as `unknown` — now type as
`technology.development.version`. Zero non-versions were touched.

## Why the header gate is LOAD-BEARING (not corroboration)

Unlike the sweep's other three recoveries, the version shape is NOT self-precise. A bare
`1.2.3` is value-ambiguous with a `YYYY.MM.DD` date and a `YYYY.MM.PATCH` calendar version —
they share the three-dotted-number shape. So value alone over-promotes: the shaped reservoir
is 570 columns, but the header gate cuts it to 372, removing 20 date columns among them. The
guard therefore fires ONLY when `header_corroborates_version` passes (a `version`/`ver`/`build`/
`firmware`/`release`/`rev` token, and NO `date`/`time`/`year`/… token — `release_date` is
vetoed) AND ≥90% of values pass `is_version_string`.

`is_version_string` carries the one discriminator the taxonomy's plain SemVer regex lacks — a
**year veto**: any four-digit 1900–2099 component is a date / calver, not a SemVer release, so
`2021.03.15` and `1.2020.0` are rejected while `10.15.7` and `1.20.0` pass. Header gate + year
veto together are what make a value-ambiguous shape safe.

## The pipeline-ordering lesson (why FIRE_ON includes the numeric labels)

The reservoir mine showed these columns' *composed* Sense label is `unknown`, so the first cut
fired on the residual set. It recovered nothing — a gate-vs-live-consistent zero. The reason is
the Sharpen order inside `run_sharpen`:

    raw model: unknown  →  feature_sharpen: 1.6.1 looks float-ish → integer_number/decimal_number
    →  [apply_post_sharpen_guards: MY GUARD runs here — sees NUMERIC, not unknown]
    →  validation veto: integer_number fails on 1.6.1 → unknown

The `unknown` I mined is a veto artifact created *after* the guards run. At guard time the label
is numeric. So FIRE_ON is the residual set PLUS `integer_number` / `decimal_number`. That is
safe: `is_version_string` demands exactly three dotted components with the year veto, so no
genuine integer (`42`) or decimal (`3.14`) can pass, and the version header gates it further.
(Lesson banked: when a column's live-Sense label is a veto-demoted `unknown`, the guard must
fire on the *pre-veto* label, which is what the raw-model cache carries once resharpened.)

## Design specifics

- RESIDUAL + numeric FIRE_ON (`unknown` / `plain_text` / `word` / `integer_number` /
  `decimal_number`). A confident date leaf (`dmy_short_dot`, `ymd_dot`) is deliberately NOT
  included — that value-ambiguous boundary is `value_sharpen`'s Rule 31 job (impossible-date-
  segment demotion); overriding a confident date on a header alone is the mistake the exclusion
  avoids.
- **NO distinct-cardinality floor** (unlike filename / delimited_array). A constant version
  column (`1.6.1` on every row) is normal and correct — a table's rows share one software
  version — so the header gate, not diversity, is the precision.
- NO new leaf (the leaf existed), NO retrain (0096), RHH-toggle `version_string_recovery`.

## Honest scope: thin distinct reach

414 recoveries, but **348 are one replicated `ver=1.6.1` dataset** dumped across gittables files
(≈ 24 distinct real-world scenarios: `repo2docker_version` 23, `latest_stable_release_number`
15, `version`/`sbt_version`/`scala_version`/`package_version`/`affects version` the rest). Every
one is a correct recovery, but the diversity is low — this is a correctness fix on a common
column type, not a broad-reach one. Genuine version diversity beyond gittables is expected in
production.

## Gates (all pass)

| Instrument | Result |
|---|---|
| Unit + guard tests | detector (SemVer accept / date-year-veto reject) + guard (header-gate / year-veto / residual-only / promotes) green |
| Corpus-honest fast gate (blocking H05) | **GO** — zero triggers, zero bands; version leaf ratio 1.45 but correct_ratio 1.0 (composition-aware band nets it as confirmed growth) |
| Gold (reframe) | **882/1037 flat**; guards-on-vs-off = **0 rows flipped** (gold-neutral — targets are corpus, not curated gold) |
| Representative (advisory) | **195/260 = 0.750** — flat vs standing baseline |
| Mandatory spot-check (414 recovered) | **0 FP** — every column version-headed SemVer; header distribution is all version tokens (`ver` 348, `repo2docker_version` 23, `latest_stable_release_number` 15, `version` 8, …) |

## Addendum (2026-07-14): camelCase header gap + the value-evidence question

**Value-evidence tier — investigated, rejected.** Could a column's own values disqualify the
date hypothesis (`0.2.53` is impossible as any date → version, no header)? Measured
(`value_evidence.py`, ordering-agnostic impossible-date test across all six DMY/MDY/YMD
permutations): it doesn't pay. Of 580 SemVer-shaped columns, **499 are constant** (`ver=1.6.1`
repeated) — no second value to disqualify anything, so value-evidence is structurally impossible
for 86% of the reservoir. Only 82 columns carry an impossible-date value, 47 already header-gated;
the incremental headerless reach is 35 columns, ~20 genuine. And going headerless opens an FP door
the header shuts: `fax :: 963.777.4065` (a phone number) and `TIME :: 12.41.56` (a clock) are both
impossible-as-date yet not versions. So value-evidence trades the header gate for a phone/time
problem to reach ~20 columns — not worth it. The value-evidence logic that DOES work is already
shipped: `value_sharpen` Rule 31 (impossible-DMY-segment → version) on date-sensed columns.

**camelCase header gap — fixed (61 more columns, 0 FP).** The scout surfaced a real gate gap: the
header tokeniser split only on non-alphanumerics, so glued camelCase version headers slipped
through — `psychopyVersion` (54 cols), `AgentVersion`, `FixVersions`, `platformBuildVersionName`
tokenised to one glued token that wasn't exactly `version`. Fixed with a camelCase-aware tokeniser
(`header_word_tokens`: split on non-alphanumerics AND lowercase/digit→uppercase boundaries), so
`psychopyVersion` → `[psychopy, version]` matches while all-lowercase `conversion` (no boundary)
stays intact and never matches. Measured false friends (`conversion`/`subversion`/…) among
SemVer-shaped columns: **zero**, doubly-guarded by the value gate. Gate GO, **61 new version
columns, 0 lost, 0 FP**, gold 882/1037 flat (0 rows flipped). Whole guard now recovers 475.
Substring matching was rejected in favour of camelCase tokenisation — the latter is the root-cause
fix (a glued-word tokeniser) with no false-friend exposure at all.

Substrate: this file; `output/version-string/{mine.py, value_evidence.py, gate/, gold_pred_*.tsv}`;
roadmap `output/reservoir-mining/roadmap.md`.
