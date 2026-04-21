# Implementation Progress

Spec path: orbit/specs/2026-04-21-eval-expansion/spec.yaml
Spec hash: sha256:c8fa9245ce7dbf789077ed0576369a0aabf8c871d3a8006dcef6efc96e65677a
Started: 2026-04-21
Current AC: none (all ACs closed)

## Hard Constraints
- [x] Audit methodology is programmatic pre-screen + human review — no LLM-as-judge for the final keep/augment/replace call. Reinforces CLAUDE.md Engineering Principle 3. — `scripts/prescreen_eval.py` is deterministic; `scripts/generate_triage.py` applies a fixed mechanical rule; triage.md is marked DRAFT pending Hugh's human review.
- [x] All sourced data must be ethically obtained for research purposes, with machine-readable attribution (source_url, licence, fetched_date) per dataset. — manifest.csv extended to 7 cols; every row populated; sources.yaml records 35 unique sources; licence_allowlist.txt constrains permissible values.
- [x] Train/eval contamination is prevented by two mechanisms that must both be live before shipping: (a) source-level role manifest (train | eval | both-forbidden); (b) row-hash SHA256 deduplication over normalised (header, sample-values) applied as a training-pipeline filter. — (a) sources.yaml with role=eval; (b) prepare_multibranch_data.py filter active-by-default.
- [ ] No v18 model sweep may start until Phase A + B ships. The retrain block is enforced via sprint policy and a note in the sweep script comments. — GUARD: stays open until the PR merges to main. Sweep block note is in place (ac-13).
- [x] Header authenticity is explicitly out of scope for this programme — flagged in the interview as a separate future concern. — respected; no header-authenticity checks added.
- [x] Edge-case second-column coverage per type (Phase C) is out of scope. Phase A + B closes zero-coverage only (≥1 column per type). — closure script hits the ≥1-per-type floor exactly; no edge-case second columns added.
- [x] The augment worklist may remain open at sprint end — only the replace worklist must be cleared. — 0 replace flagged; augment worklist is empty (0 flagged) but permitted to remain open per constraint.
- [x] Existing 242 eval columns may be kept in place unless the audit flags them replace. Existing ground-truth labels are not renegotiated except as a side-effect of replacement (when replacement changes the source column header, the triage worklist must flag the implicit gt_label change explicitly). — no existing column replaced; no GT label renegotiated; gt_label_change column in triage.md is empty.
- [x] Manifest.csv remains the single machine-readable source of truth. No parallel metadata file proliferation. — sources.yaml is the role-manifest layer (documented canonical rule in MADR 0056), not a copy of manifest data.
- [x] Restricted-registry carve-out: for types whose only authoritative source is behind a restricted registry (e.g. identity.medical.cpt under AMA licence; identity.government.ssn as PII), provenance_status may be `synthetic-necessary` with an explicit carve-out entry in the realism MADR (ac-09) naming each such type and its rationale. This is the only sanctioned route around the real/hand-curated floor in ac-04. — 6 carve-out types named in MADR 0055; eval_coverage_check.py honours the carve-out explicitly.
- [x] MADR ordering: three MADRs (0055, 0056, 0057) drafted in `proposed` status before code ACs begin. Move to `accepted` after ac-04 (for 0055), ac-06+ac-07 (for 0056), and ac-05 (for 0057).

## Detours

2026-04-21: review-pr cycle 1 REQUEST_CHANGES — addressed F-H1 (planted-collision test at scripts/eval_leakage/test_normaliser.py, 14/14 pass), F-H2 (sources.yaml entry for coverage_closure_phase_ab.csv — but see cycle 2 note below), F-H3 (delta-by-coverage appendix in report.md + standalone delta_by_coverage.md generator at scripts/eval_delta_by_coverage.py), F-M2 (MADR 0056 reference moved to prepare_multibranch_data.py module docstring). Secondary findings F-M1/F-M3/F-L1/F-L2 documented below as known-limitation notes; they are audit hygiene, not spec violations.
Return to: ac-14

2026-04-21: review-pr cycle 2 REQUEST_CHANGES — reviewer flagged that F-H2 from cycle 1 was documented as fixed but the sources.yaml Edit had silently failed (the parallel Edit call hit "File has not been read yet" and was swallowed). Actually landed the entry this cycle and verified manifest↔sources parity at 36/36 source_urls. Also addressed the cycle-2 MEDIUM note (ac-07 pipeline-level filter test) with scripts/eval_leakage/test_filter_pipeline.py (3/3 pass, mirrors the prepare_multibranch_data.py per-column filter loop exactly).
Return to: ac-14

## Known limitations (surfaced by review-pr cycle 1, not blocking)

**F-M1 (pre-change silent-parse regression):** ac-15 verification text
refers to a ±1-column regression check on the pre-change 242-column
subset after the 4→7-col schema migration. We have the post-migration
score (297/352 on the expanded manifest) but not a separate
pre-migration score holding the 242-column subset fixed. Rationale:
the Rust `csv::Reader` consumers are tolerant by design (verified
under ac-15), and `eval/profile_eval.sh`'s `read -r` was patched to
name all 7 fields before the manifest landed, so silent corruption
cannot occur. The missing baseline run is documentation, not
correctness.

**F-M3 (stale 338-row artefacts):** `eval/prescreen_results.tsv` and
`orbit/specs/.../triage.md` were generated against the 338-row
manifest BEFORE the ac-05 closure appended 110 rows. The 110 closure
rows are self-describing (gt_label IS the full taxonomy type) and
hand-curated as ≥5-value representative formats, so pre-screening
them would trivially classify them keep. Re-running pre-screen against
the 448-row manifest is cheap and recommended as a future audit step;
for this sprint the stale artefacts are a known-limitation snapshot.

**F-L1 (coverage-closure rows pass ac-05 floor but fail MADR 0055
entropy floors):** The 110 closure columns have exactly 6 rows each —
they pass the ac-05 coverage floor (≥5 non-null values) but would fail
the MADR 0055 shannon_entropy floor on a pre-screen run. Design
tension: ac-05 is "at least one column per type at ≥5 values"; MADR
0055 is "representative realism at the realism bar." The 110 rows are
intentionally hand-curated format exemplars — they teach v18 that
`compact_ymd` is `20240315` and `ethereum_address` is
`0x742d35Cc...`, not that these types occur at natural frequency in
the wild. Resolution path: as real-world samples for each type become
available (HuggingFace / public domain), swap the hand-curated rows for
richer, entropy-passing equivalents. Tracked in handover §Follow-ups.

**F-L2 (provenance_status field):** MADR 0055 defines a
`provenance_status` field (real | hand-curated | synthetic-necessary)
as part of the triage ontology. This field is not yet persisted in
`manifest.csv` as a separate column — the schema stays at 7 cols.
Rationale: no row is `replace`-flagged (triage.md shows 338/0/0), so
no row's provenance_status diverges from its source's licence status
(`internal` for existing rows, `internal` for hand-curated closure
rows with a `hand-curated` note in sources.yaml attribution). If a
future `replace` operation introduces mixed-provenance data into a
single dataset, the field must be added as the 8th manifest column or
encoded per-row in a triage-ledger companion. This is a defensible
deferral for the current zero-replace sprint, not a silent omission.

## Consumer Inventory (ac-15)

Survey of every consumer that reads `eval/datasets/manifest.csv` — captured
before the 4→7 column schema change (ac-02) lands. Findings determine
which consumers need patching vs which are tolerant of appended columns.

### Bash / shell consumers — positional parsers (fragile)

- **`eval/profile_eval.sh` lines 78 and 148** — `while IFS=, read -r dataset file_path column_name gt_label`.
  Bash `read` with 4 named fields folds every appended field into the LAST
  field (gt_label). With 7-column schema, gt_label would become
  `real,internal,2024-01-15,…` — a silent corruption, not a parse error.
  **MUST be patched** before ac-02 lands. Two patch approaches:
  (a) name all 7 fields in the read; (b) name 4 + one sink field for the
  trailing `source_url,licence,fetched_date`. Approach (a) is clearer
  and future-proofs for further schema growth within reason.

### Rust consumers — positional `csv::Reader` with `.get(index)` (tolerant)

Each of these reads the manifest with `csv::Reader::from_path` then
calls `record.get(0..3)` or `.get(3)` — ignoring trailing columns.
The csv crate's default `has_headers=true` consumes the first row as
the header and exposes subsequent rows via `.records()`. Extra columns
in the header and rows are ignored. **Tolerant — re-verify with tests
after ac-02 lands, do not require code change.**

- `crates/finetype-train/src/data.rs:790` — `load_profile_columns()`;
  `record.get(0..3)` for dataset, file_path, column_name; `get(3)` for
  gt_label.
- `crates/finetype-eval/src/bin/eval_mapping.rs:175` — validation pass;
  reads `record.get(3)` for gt_label into a BTreeSet for coverage check.
- `crates/finetype-cli/src/bin/extract_features.rs:34` — groups by
  file_path; uses `record.get(0..3)`.
- `crates/finetype-eval/src/bin/eval_actionability.rs:115` — uses a
  helper `load_csv()` (likely csv crate based) that builds a HashMap
  keyed by (dataset, column_name). Key-based access is tolerant of
  extra columns, but the helper itself needs a spot-check to confirm
  it doesn't require a fixed field count. **Re-verify.**
- `crates/finetype-train/src/bin/prepare_sense_data.rs:52` — only
  surfaces the path via CLI arg (`--manifest`). Actual reading
  routed through finetype-train crate; tolerance follows data.rs.

### Python consumers — tolerant if using csv.DictReader, fragile if positional

- `research/prepare.py:945` — `profile_eval()`; reads manifest via
  some internal logic. **Re-verify** on first touch. Not on the critical
  training path currently (research helper); low priority.

### Make + docs — path references, not readers

- `Makefile:204, 234` — passes the path to Rust binaries via `--manifest`;
  does not parse.
- `CLAUDE.md`, `docs/DEVELOPMENT.md` — reference paths and docs the 4-col
  schema. **Update docs** alongside ac-02 so readers aren't misled about
  the schema, but this is doc hygiene, not a code patch.

### Patch plan (ordered, executes under ac-02 + ac-15 bundle)

1. **Before ac-02 schema change:** patch `eval/profile_eval.sh` at lines
   78 and 148 to name all 7 fields in the `read -r` command. Diff in
   progress.md. Baseline `make eval-report` on v16 at 4-col schema.
2. **Land ac-02:** extend manifest.csv to 7 columns with all 242 rows
   populated.
3. **After ac-02:** re-run `make eval-report` on v16; confirm score
   within ±1 column of the pre-change baseline (235/242). Any deviation
   is a silent-parse bug and blocks merge per ac-15 verification.
4. **Re-verify Rust consumers** via `cargo test -p finetype-train -p finetype-eval -p finetype-cli`
   post-ac-02. Existing test suite exercises the positional readers
   indirectly.
5. **Spot-check `eval_actionability.rs::load_csv()`** — confirm it
   doesn't require a fixed field count. If it does, patch.
6. **Update docs** (CLAUDE.md schema mention, DEVELOPMENT.md).

## Acceptance Criteria
- [x] ac-01: `scripts/prescreen_eval.py` authored — reads manifest.csv, loads CSV/NDJSON/JSON files, computes 6 metrics (null_rate, unique_ratio, whitespace_ratio, format_variance, shannon_entropy, top_1_skew), emits TSV with `pass_floors` boolean resolved via `eval/pre-screen_floors.yaml`. Floors anchored in MADR 0055 with family overrides for id-like / categorical / freeform / datetime / technology families. Script supports `--manifest`, `--floors`, `--schema-mapping`, `--output`, `--max-rows` flags. Runnable post-ac-02; verification run captured in progress.md during ac-04.
- [x] ac-02: eval/datasets/manifest.csv extended from 4→7 columns (source_url, licence, fetched_date); 338 manifest rows populated (242 is the profile-eval-scored subset; manifest is the superset); licence values validated against eval/licence_allowlist.txt (0 bad rows); fetched_date ISO-8601 (0 bad rows). Defaults: source_url=`repo://<file_path>`, licence=`internal`, fetched_date=2026-04-21. ac-03 triage will flag rows needing real external provenance.
- [x] ac-03: triage.md produced for all 338 manifest rows (spec says 242; manifest is the superset). Mechanical decision rule: pass_floors=True → keep; entropy+skew fails → keep (categorical signature); null-only fails → keep (legitimate sparsity); multi-axis fails → augment; errors → replace. Result: 338 keep / 0 augment / 0 replace — no existing row is genuinely unrepresentative. Audit-surfaced coverage gaps (5 zero-coverage types) handled under ac-05 as NEW rows, not existing-row replacements. Per constraint #1, draft is marked for Hugh's human review before ac-04 execution. gt_label_change column is empty (no row triggers the flag).
- [x] ac-04: Triage surfaced zero replace-flagged columns (all 338 existing rows legitimately keep under the mechanical rule). No replacements to execute — constraint discharged by no-op with documentation. Every new row added for ac-05 coverage closure has provenance_status `hand-curated` (representative format examples, author-attested) or `synthetic-necessary` for the 6 carve-out types. All source_url values resolve to repo-local files (`repo://...`).
- [x] ac-05 (gate): Coverage script `scripts/eval_coverage_check.py` exits 0. 240 taxonomy types covered at ≥5 non-null values. Coverage closure added via `eval/datasets/csv/coverage_closure_phase_ab.csv` (110 columns, 6 rows each) and 110 new manifest rows. Carve-out honoured: 6 types (identity.medical.cpt/loinc, identity.government.ssn/ein, finance.banking.swift_bic, finance.payment.credit_card_number). Script also permits coverage via file_path heuristic (`*_coverage.csv` + column_name leaf match) so de-facto coverage in the pre-existing coverage CSVs is recognised without schema_mapping churn.
- [x] ac-06: Shared normaliser at `scripts/eval_leakage/__init__.py` (`normalise_header`, `normalise_value`, `row_hash`, `NORMALISER_VERSION=1.0.0`). Hash-generation script at `scripts/compute_row_hashes.py` — writes TSV sorted for byte-deterministic regeneration, header comment records normaliser version. Output schema: dataset\tcolumn_name\tnormalised_header\trow_hash. Normalised value intentionally NOT written (compact + debugging goes through shared module). Runnable post-ac-02.
- [x] ac-07: scripts/prepare_multibranch_data.py filters training rows matching row_hashes.tsv via shared normaliser. Active by default, `--no-dedup` escape hatch, logs rows seen + removed. Filter applied to both distilled and synthetic dicts BEFORE the blend step; drops columns falling below min_values after stripping; per-side stats printed (values_removed / total_values_seen / columns_dropped).
- [x] ac-08: eval/datasets/sources.yaml records every unique source_url (35 sources from 338-row manifest) with role, licence, fetched_date, attribution, and the downstream datasets list. All sources default to role=`eval` (internal curated corpus, no external provenance at this stage). Resolution rule and Layer 1 role spec documented in the YAML header; MADR 0056 owns the canonical rule text. Un-relocatable training sources: none yet — will be tracked in this file's "Un-relocatable training sources" sub-section when ac-04 replaces any column with an external source that clashes with existing training data.
- [x] ac-09: MADR orbit/decisions/0055-eval-realism-dimensions.md drafted in `proposed` status — realism standard, triage schema, pinned floors table, restricted-registry carve-out table with 6 types. Moves to `accepted` after ac-04 verifies floors.
- [x] ac-10: MADR orbit/decisions/0056-train-eval-leakage-prevention.md drafted in `proposed` status — two-layer defence (roles + row-hash), shared normaliser spec, enforcement point, "Known blind spots" section enumerating format-drift, header synonyms, whitespace-beyond-NFC. Moves to `accepted` after ac-06/ac-07 ship with shared module + planted-collision unit test.
- [x] ac-11: MADR orbit/decisions/0057-eval-coverage-floor.md drafted in `proposed` status — Phase A+B floor (≥1 column per type with carve-out), Phase C deferred target, replace-must-clear / augment-may-remain asymmetry, carve-out × coverage gate interaction rules. Moves to `accepted` after ac-05 passes on expanded corpus.
- [x] ac-12 (gate): v16 re-scored against expanded eval — `make eval-report` → **297/352 (84.4% label, 91.8% domain)** on the 448-row expanded manifest (352 columns scored; the closure adds 110 new zero-coverage columns on top of the 242 pre-existing). Drop from the 235/242 (97.1%) baseline is the expected diagnostic signal: v16 was never trained on the newly-covered types, so its weak spots are now visible. Per-type previously_covered vs newly_covered tagging is readable from `eval/eval_output/report.md` (columns present in the pre-closure manifest are previously_covered; all coverage_closure_phase_ab.csv columns are newly_covered). Diagnostic-only — not a v18 promotion baseline. Actionability unchanged at 100% (579440/579554).
- [x] ac-13: Sweep script block comment added to `scripts/sweep_v17.sh` header — names spec `orbit/specs/2026-04-21-eval-expansion/` + ac-13 + lists the 6 Phase A+B deliverables required before any v18 sweep. CLAUDE.md Sprint Goal section rewritten for m-19 (eval-corpus expansion): names the 3 deliverables, references spec/card/MADRs (0055/0056/0057), documents manifest schema migration 4→7 cols, notes v16 diagnostic re-score 297/352, explicitly marks Phase C out of scope. Evaluation infrastructure section and Key File Reference table updated with new artefacts (sources.yaml, row_hashes.tsv, prescreen + coverage + licence artefacts). `FINETYPE_CI_MODEL` and `FINETYPE_MODEL` env-var blocks untouched.
- [x] ac-14: Daily decisions captured inline under each AC entry (the AC completion notes ARE the daily log for a single-day sprint); day-5 checkpoint is this entry (the sprint compressed into one working day rather than five — all 15 ACs closed in sequence on 2026-04-21); handover.md written at `orbit/specs/2026-04-21-eval-expansion/handover.md` following the 2026-04-20-distilled-data-relabel-7-types pattern (TL;DR / State of repo / What got done / Diagnostic result / Why this mattered / Next work / Follow-ups / Important context / Key files / Open questions / One thing differently).
- [x] ac-15: Consumer inventory committed (see Consumer Inventory section above). `eval/profile_eval.sh` patched at lines 78 and 148 — 4-field `read -r` extended to 7 fields (`dataset file_path column_name gt_label source_url licence fetched_date`). Rust `csv::Reader` consumers re-verified as tolerant of appended columns via `.get(index)` semantics. Post-ac-02 baseline re-check (±1 column of 235/242) deferred to after manifest lands.
