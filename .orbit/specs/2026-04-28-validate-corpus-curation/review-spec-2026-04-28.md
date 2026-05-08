# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 6 |
| 2 — Assumption & failure | content signals (eval datasets, training-leakage firewall, taxonomy GT) + 2 MEDIUM findings in Pass 1 | 3 |
| 3 — Adversarial | not triggered (no cascading or rollback risk; all changes additive + revertible) | — |

## Findings

### [HIGH] AC-04 sources.yaml grep verification targets a key that does not exist
**Category:** test-gap
**Pass:** 1
**Description:** The verification line uses `grep -c '^- url:' eval/datasets/sources.yaml` and asserts it "increases by exactly 5." The actual key in `sources.yaml` is `source_url:`, not `url:`. Running the spec's grep against the current file returns **0**, not 42 or any baseline — the regex is wrong and the assertion can never trigger meaningfully.
**Evidence:** `eval/datasets/sources.yaml:291` shows entries start with `- source_url:`. A direct `grep -c '^- url:'` against the current file returns 0 matches; `grep -c 'source_url:'` returns 43.
**Recommendation:** Replace with `grep -c '^  - source_url:' eval/datasets/sources.yaml` (or a Python yaml-aware count). The "increases by exactly 5" check is fine; only the regex is broken.

### [HIGH] AC-03 manifest column description does not match the on-disk schema
**Category:** missing-requirement
**Pass:** 1
**Description:** AC-03 asserts the validate manifest has 9 columns including "`column_name placeholder, gt_label placeholder`" alongside `gt_file`, `source_url`, `licence`, `fetched_date`, `row_count`. The actual iter-1 file (`eval/datasets/validate_manifest.csv` line 1) has these 9 columns: `dataset, file_path, source_url, licence, fetched_date, provenance_status, gt_sidecar_path, row_count, column_count`. There is no `column_name` or `gt_label` per row — those belong to the eval manifest (column-level), not the validate manifest (dataset-level). The fields `provenance_status`, `gt_sidecar_path`, `column_count` are absent from the spec.
**Evidence:** `eval/datasets/validate_manifest.csv:1` (header) and `compute_row_hashes.py:14-19` ("If `gt_sidecar_path` is present (validate manifest) — each row names a whole CSV file"). The validate manifest is per-dataset, not per-column.
**Recommendation:** Rewrite AC-03 to enumerate the actual 9 columns: `dataset, file_path, source_url, licence, fetched_date, provenance_status, gt_sidecar_path, row_count, column_count`. Drop the `gt_file, column_name placeholder, gt_label placeholder` fragment — it conflates the two manifests.

### [HIGH] AC-06 prescreen target manifest is incompatible with the script
**Category:** failure-mode
**Pass:** 2
**Description:** `prescreen_eval.py` (line 392-393) reads `column_name` and `gt_label` from each manifest row — these are eval-manifest fields. The spec instructs running it with `--manifest eval/datasets/validate_manifest.csv`, which has neither. The script will read empty strings for both fields, never resolve a `full_type` from `schema_mapping`, and the realism floors that depend on a type-family resolution may silently degrade or error per row. The verification "exits 0 / status: keep for all 5" is unreliable: the script could exit 0 trivially because no per-column work was ever done.
**Evidence:** `scripts/prescreen_eval.py:390-396` (`row.get("column_name", "")`, `row.get("gt_label", "")`); `eval/datasets/validate_manifest.csv:1` (no such columns).
**Recommendation:** Choose one:
  (a) Iterate the GT sidecars (e.g., `python scripts/prescreen_eval.py --gt-sidecars eval/datasets/validate_corpus/*.gt.yaml`) — requires a small adapter; or
  (b) Synthesise a per-column eval-shaped manifest from each new GT sidecar pre-flight, run prescreen against that synthesis, and discard.
  Either way: spell out which option in the spec, not "run the existing script against the validate manifest."

### [MEDIUM] AC-05 hash-count delta estimate is wrong order of magnitude
**Category:** test-gap
**Pass:** 1
**Description:** AC-05 verification asserts `wc -l eval/row_hashes.tsv` increases by "~25,000" (5 datasets × 5000 rows). The hash table is per `(dataset, column, value)`, not per row. With ~10 columns per dataset, the actual delta is closer to 250,000 (5 × 5000 × 10), modulo dedup. The test as written would pass at 25k but the new state is much larger — a reviewer following the AC at face value would conclude the script malfunctioned.
**Evidence:** `compute_row_hashes.py:32-33` ("Output schema (TSV): dataset \\t column_name \\t normalised_header \\t row_hash"). Each column contributes ~5000 rows.
**Recommendation:** Either drop the magnitude estimate ("`wc -l eval/row_hashes.tsv` strictly grows") or restate it correctly per (dataset, column, value). The leakage firewall regression test is the load-bearing check; the line-count is informational.

### [MEDIUM] AC-08 awk verification mis-parses a markdown pipe table
**Category:** test-gap
**Pass:** 1
**Description:** `awk '/format_diversity/{print $4}'` extracts the 4th whitespace-delimited token from `| format_diversity      |               0 |                 0 |`. The 4th field is `|` (a separator), not the count. The same applies to `code_vs_canonical`. The verification will not produce a numeric value reviewers can compare to ≥1.
**Evidence:** Inspecting `eval/eval_output/validate_corpus.md:11-17` (current iter-1 table). The default awk separator splits on any whitespace; pipes count as their own tokens.
**Recommendation:** Use `awk -F'|' '/format_diversity/{gsub(/ /,"",$3); print $3}'` or a small Python snippet that parses the markdown table cell. Even simpler: assert the count column is non-zero with `grep -E '^\| format_diversity\s+\|\s+[1-9]'` (and similarly for `code_vs_canonical`).

### [MEDIUM] Per-dataset target type `finance.market.ticker_symbol` does not exist in the taxonomy
**Category:** missing-requirement
**Pass:** 1
**Description:** Implementation note for NASDAQ tickers names `finance.market.ticker_symbol` as the validator under test. The current taxonomy (`labels/definitions_finance.yaml`) has no such key — closest neighbours are `finance.securities.cusip/isin/sedol/figi/lei`. Without a target type, the "code_vs_canonical" mechanism for this dataset has nothing to fail against; it will land in `misclassification` or `no_gt`, eroding the AC-08 commitment to ≥1 attribution in code_vs_canonical from this dataset specifically.
**Evidence:** `grep -E "^[a-z_]+\." labels/definitions_finance.yaml` returns no `*.ticker_symbol`. Legacy definitions show a deprecated `finance.stock_ticker` not present in v0.6.19.
**Recommendation:** Either (a) replace NASDAQ tickers with a CvC dataset whose target type exists (CUSIP/ISIN-bearing securities CSV; SEC EDGAR exposes some), or (b) call out explicitly that NASDAQ tickers will GT-label as `representation.identifier.alphanumeric_id` and the CvC mechanism rests on the other two CvC datasets (FIFA, OECD). Today the spec implies a non-existent type fires.

### [MEDIUM] AC-09 mismatch table is doc-only with no machine check beyond grep-for-header
**Category:** test-gap
**Pass:** 2
**Description:** AC-09 verifies a `## Iter-2 expected vs actual` section exists ("returns the section header") and "contains a 5-row table." But the verification only grep-asserts the header line; nothing checks that the table has 5 rows or that the data is honest (vs auto-generated stubs that all say "match: ✓"). Combined with the constraint "mismatch is reported, not corrected," the only real safeguard against a vapid table is reviewer goodwill.
**Evidence:** `ac-09.verification` block says "returns the section header" and "Section contains a 5-row table" — only the first half is grep-checkable; the second half is human inspection.
**Recommendation:** Add a programmatic check: `awk '/^## Iter-2 expected vs actual/,/^## /' eval/eval_output/validate_corpus.md | grep -c '^| ' >= 7` (header row + separator row + 5 data rows = 7 pipe-leading lines). Or assert `grep -cE '^\| [a-z_]+ \|' >= 5` against the section.

### [LOW] AC-07 headline regex doesn't account for the iter-1 baseline/delta suffix
**Category:** test-gap
**Pass:** 2
**Description:** Iter-1's headline is `**3 of 7 datasets pass at P=99%** (baseline: 3 of 7; delta: +0)`. AC-07 asserts the format `**N of 12 datasets pass at P=99%** (iter-1: 3 of 7)` but the verification regex is `^\*\*[0-9]+ of 12` — which matches both forms. That's fine. But the spec sets the *expectation* on the parenthetical text; if the harness emits `(baseline: 3 of 7; delta: +X)` instead, the AC verification still passes while the report semantically diverges from what the spec promised. Minor.
**Evidence:** `eval/eval_output/validate_corpus.md:7` shows the live headline format. `validate_corpus.rs` is byte-frozen by constraint, so the harness will emit `baseline/delta`, not `iter-1: 3 of 7`.
**Recommendation:** Either (a) tighten the spec's headline format to `"**N of 12 datasets pass at P=99%** (baseline: 3 of 7; delta: …)"` to match the frozen harness output, or (b) drop the parenthetical specification entirely and lean on the `^\*\*[0-9]+ of 12` regex.

### [LOW] No rollback path is specified if a new dataset commits then fails review
**Category:** failure-mode
**Pass:** 2
**Description:** The dependency chain in implementation_notes (steps 1–10) commits CSV + manifest row + sources.yaml + GT sidecar in lockstep. If iter-2 ships, then a CC-BY-SA-4.0 dataset's licence is later disputed (or a dataset turns out to overlap a training source), the spec gives no guidance on partial revert. Constraint #8 ("Existing iter-1 manifest and corpus files are byte-unchanged") implies subsequent iters mustn't touch iter-2 either, freezing the licence-defect dataset in place.
**Evidence:** No exit_conditions or constraints address dataset removal post-merge.
**Recommendation:** Add a one-line constraint: "Iter-2 datasets may be removed in a separate revert PR if licence or leakage issues surface; the constraint is byte-unchanged for *clean* iter-1 entries, not for defective entries discovered later." Low priority — won't block ship.

---

## Honest Assessment

The plan is conceptually sound — pure curation, additive PR, mechanism-coverage exit criterion is the right shape — but the verification surface is sloppy in five concrete places (AC-03 schema, AC-04 grep key, AC-05 magnitude, AC-06 manifest-shape, AC-08 awk parse) and one design place (NASDAQ tickers maps to a non-existent taxonomy type, undermining one of three CvC slots in the targeted breakdown).

The biggest risk is **AC-06 silently passing while doing nothing useful**: prescreen_eval.py reads column-level fields the validate manifest doesn't carry; the realism gate could exit 0 vacuously and the implementer would tick the box. Combined with the NASDAQ ticker-symbol gap, iter-2 could ship its 5 datasets, satisfy 11/11 ACs by the letter, and still leave code_vs_canonical at 1 attribution (FIFA + OECD only) — half the symmetry the spec promises.

These are mechanical fixes; none rise to design rework. REQUEST_CHANGES with a focus on tightening verification commands and confirming taxonomy-key reality before committing to NASDAQ tickers.
