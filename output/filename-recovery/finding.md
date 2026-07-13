# technology.filesystem.filename — new type + recovery guard (2026-07-14)

**Headline:** ~850 columns of bare filenames (`report_final.xlsx`, `L2M-*.pdf`, MAME `.cpp` sources)
that FineType had no home for — scattered across "plain text", "entity name", "an ID", even "a Bitcoin
wallet" — now type as `technology.filesystem.filename`, a new leaf minted alongside `windows_path`.
Zero non-files were touched.

## The type

`technology.filesystem.filename` — a bare file NAME (stem + real extension, no directory), sibling of
`technology.filesystem.windows_path` (a file *path*). Placed in the filesystem family, NOT
`representation.file.*` — that family holds file *properties* (extension, size, mime_type), not the file
entity. Taxonomy: 250→251 defs, technology 30/30, generator added, samples 100%, all generators pass.

The shape `word.word` is not precise (Precision Principle), so the leaf follows the ticker pattern —
a shape validator with the real precision in the guard's detector (`finetype_core::structure::is_filename`):
curated `FILE_EXTENSIONS`, lowercase-terminal extension, letter-bearing stem, plus the sweep's FP vetoes
(single-char-ext unit veto for `mW.h`/`kW.h`; stem-dots only as a double-extension `archive.tar.gz`, so
a code namespace `system.data.sql` is excluded).

## The guard

`filename_recovery` fires on the RESIDUAL labels + the measured confident-mislabel OVERRIDE set
(entity_name / alphanumeric_id / token_urlsafe / version / full_address / full_name / bitcoin_address /
jwt / isbn / username). `hostname` and `url` are DELIBERATELY EXCLUDED — a bare ccTLD domain (`gov.md`)
is shape-identical to a markdown file, so a confident locator is never overridden. Promote at ≥90% pass
AND ≥3 distinct pass (a near-constant column can't override a foreign prediction). RHH-toggle
`filename_recovery`, NO retrain (0096).

## Gates (all pass)

| Instrument | Result |
|---|---|
| Taxonomy check | 251 defs, technology 30/30, samples 100% |
| Corpus-honest fast gate (blocking) | **GO** — zero triggers, zero bands |
| Gold (reframe, corrected) | **882 on / 881 off = +1** (the guard FIXES a gold defect it surfaced) |
| Representative (advisory) | **195/260 flat** |
| Actual promotions (cand vs base, 33k sample) | **104** — plain_text 47, entity_name 19, alphanumeric_id 15, username 12, token_urlsafe 4, word 4, unknown 2, bitcoin_address 1 |
| Mandatory spot-check | **0 FPs** — 0 constant, 0 non-file (<90% known ext), 0 ccTLD-domain; every sample a genuine file |

## Gold defect surfaced + corrected (author-ratified)

The guard flipped gold row `papers.parquet` `link` (`vds21a-cactus.pdf`, `vds21a-graph.pdf`, … —
**100% bare `.pdf` filenames**) from its label `plain_text` to `filename`. The gold label was a defect;
the guard is right. Corrected in place (value-evidence, gleif-region precedent — author-ratified
2026-07-14), so the guard reads as +1 rather than −1. Unlike the queued `group_id:id` case, this label
is provable by value evidence, not debatable, so it rode the same change.

Est. corpus reach ~700 columns. Substrate: this file; `output/filename-recovery/{gate,eval}/`; roadmap
`output/reservoir-mining/roadmap.md`.
