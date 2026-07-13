# qualified_name_recovery — scoped recovery of dotted code namespaces (2026-07-13)

**Headline (analyst view):** FineType was calling ~1,000 columns of .NET/Java namespaces
(`ICSharpCode.NRefactory6`, `AgileWizard.Domain`, `Azure.AI.TextAnalytics`) a grab-bag of wrong
types — a person's name, a website, a city, or just "plain text". They now recover to their real
type, `technology.code.qualified_name`, deterministically, with zero real websites or names
mislabelled in the process. This is the residual-audit reservoir (plain_text overclaims), pointed at
the model's active mislabels, not just its abstentions.

## What the guard does

`technology.code.qualified_name` is taxonomy-live but NOT model-predicted (244-dim model). The
`structured_string_refinement` recovery already recovers the **3+-segment** forms FROM the residual
labels via the taxonomy validator (`{2,}` = three-or-more segments). This guard closes the two gaps
that leaves, in two tiers keyed to the corpus-measured **live-Sense** distribution:

- **Tier 1 (residual → qn):** `plain_text`/`word`/`unknown` promoted on `is_qualified_name` — a
  2-segment form needs a **code signal** (underscore / internal CamelCase) that a bare `foo.bar`
  hostname lacks, and must not be a filename; 3+ segments accepted directly. Closes the 2-segment
  gap the taxonomy validator rejects (widening the shared validator is out — `foo.bar` is too common,
  Precision Principle).
- **Tier 2 (confident-mislabel override → qn):** the name/place/host text labels the model actively
  reaches for on a dotted PascalCase token — `entity_name`, `hostname`, `full_name`, `full_address`,
  `city`, `region` — promoted on the stricter `is_qualified_name_strong`: a code signal (underscore /
  internal CamelCase / uppercase across 3+ segments) AND **not a canonical hostname**
  (`looks_like_hostname`: lowercase + common TLD + no underscore). So a genuine `www.breitbart.com`
  (Sense=hostname) is structurally spared.

`username` and `alphanumeric_id` are deliberately **excluded** from the override set — Odoo user-refs
(`base.user_root`) and HDF5 filenames (`…_bf.h5`, `.tar.gz`) overlap the shape; a lowercase
file-extension guard (`ends_in_file_extension`, any depth) additionally rejects multi-extension
filenames while sparing genuine `.Xml`/`.Sql` namespace leaves (Capitalized, not lowercase).

Detectors: `finetype_core::structure::{is_qualified_name, is_qualified_name_strong}`. Guard:
`qualified_name_recovery` in `column/guards.rs`, wired after `s_expression_recovery`. Value-based
(0048), NO header gate, RHH-disableable, NO retrain (0096).

## Why the reservoir is bigger than the residual audit implied

The residual audit sized this off `columns.parquet` `sense_prediction` (`plain_text`). But the LIVE
model (sibling-aware) calls these columns something else entirely. Of 2,294 qualified-name-shaped
columns in the 33k gate sample, only **32%** are residual; the rest the model actively mislabels:
`entity_name` (387), `hostname` (318), `username` (139), `alphanumeric_id` (118), … The win lives in
the OVERRIDE tier, not the residual tier. **Lesson: `columns.parquet` sense is stale — the guard's
real behaviour is only measurable by the gate's live cand-vs-base diff, never the offline sense
label.**

## Gates (all pass)

| Instrument | Result |
|---|---|
| Corpus-honest fast gate (blocking H05) | **GO** — zero triggers, zero bands (qualified_name isn't in the gated-YDF oracle vocab; oracle-contra netted out) |
| Gold (reframe, blocking) | **882/1037 = 0.851 flat** (rule on == rule off); 0 regressions |
| Representative band (advisory) | **195/260 = 0.750 flat**; delta 0 |
| Actual promotions (cand vs base, 33k sample) | **995 columns**, 0 qualified_name lost |
| Mandatory spot-check | **0 real-host FPs**; all 400 constant-column promotes + all tier-2 place/name overrides verified genuine namespaces |

Promotions by source (33k sample): entity_name 327, plain_text 297, hostname 201, word 148,
full_name 10, city 8, region 3, unknown 1.

## Open / noted, not actioned

- **Gold re-adjudication candidate:** `group_id:id` (Odoo external IDs `account.group_account_manager`)
  is gold-labelled `alphanumeric_id`, but the values carry no digit (they fail alphanumeric_id's own
  validator) and are dotted `module.record` refs → `qualified_name` is arguably more correct. Neutral
  to the headline (both wrong under reframe). Panel-proposes/author-ratifies if pursued.
- **Deferred (harder):** lowercase Java packages the model confidently calls `hostname`
  (`com.google.common.collect`) are genuinely ambiguous with deep hostnames and are left as hostname
  (precision over recall). Capitalized packages are recovered; lowercase ones are not.

Substrate: this file; `output/qualified-name-recovery/{gate,eval}/`, `sense_cache.tsv`.
