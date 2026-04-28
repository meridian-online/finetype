# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (eval datasets, training-leakage firewall, taxonomy GT) + 2 MEDIUM findings in Pass 1 | 2 |
| 3 — Adversarial | not triggered (changes additive + revertible; no cascading scope) | — |

## Findings

### [HIGH] S&P 500 sourcing notes reference taxonomy keys that do not exist (`us_state`, `cik`)
**Category:** missing-requirement
**Pass:** 1
**Description:** The implementation_notes block for S&P 500 (lines 287–291) names two taxonomy keys as the validators-under-test: `geography.location.us_state` (for "Headquarters Location") and an implied identifier validator for `CIK`. Neither exists in the v0.6.19 taxonomy. The actual geography type is `geography.location.state_code` (2-letter abbreviations only), and there is no CIK type at all. The S&P 500 dataset was introduced in v2 as the explicit replacement for NASDAQ tickers precisely because "v0.6.19 taxonomy has no `finance.market.ticker_symbol` type" — but the replacement repeats the same mistake on two of the four columns the spec leans on for CvC signal.
**Evidence:**
- `labels/definitions_geography.yaml:339` defines `geography.location.state_code` (2-letter abbreviations); no `us_state` key exists anywhere.
- `grep -n 'central_index_key\|cik' labels/*.yaml` returns zero matches — no CIK validator.
- Spec lines 285–294 read "`Headquarters Location` → … tests `geography.location.us_state` validator strictness" and "`CIK` → SEC central index key, 10-digit zero-padded; tests identifier-validator handling of leading zeros."
- "City, ST" or "City, State" is a *composite* string, not a single state value, so even pointing GT at `state_code` would mislabel the column.
**Recommendation:** One of:
  (a) Narrow the S&P 500 CvC commitment to **just** `GICS Sector` (controlled vocab → `representation.discrete.categorical`) and explicitly mark `Symbol`, `Headquarters Location`, `CIK` as text/word/categorical control columns with no CvC expectation; or
  (b) Replace S&P 500 with a CvC dataset whose target columns map to validators that demonstrably exist (e.g., a CSV bearing CUSIP/ISIN-bearing securities where canonical-vs-non-canonical formatting fires against the `finance.securities.cusip` / `isin` validator that *does* exist in the taxonomy).
  Either way, the spec's CvC slot count drops from "3 datasets (FIFA, S&P 500, OECD)" to "2 datasets reliably (FIFA, OECD) + S&P 500 contributing only via GICS Sector." That still satisfies AC-08 (≥1 attribution) but the spec should say so plainly.

### [HIGH] AC-06 prescreen synthesis script is conditionally specified ("if non-trivial")
**Category:** test-gap
**Pass:** 1
**Description:** AC-06 (lines 169–170) tells the implementer to commit `scripts/synth_prescreen_manifest.py` "if non-trivial, or run inline as a documented shell pipeline if trivial." This is a genuine ambiguity gate — two implementers will draw the line differently, and the spec gives no decision rule. Worse, the inline-pipeline path leaves no committed artefact for the next iter (iter-3 will have to re-derive the synthesis), and the "documented shell pipeline" verification is unauditable post-hoc.
**Evidence:** Line 169–170 of spec.yaml. Compare iter-1 spec (`orbit/specs/2026-04-28-validate-precision-corpus/spec.yaml`) which committed concrete scripts like `eval/datasets/validate_manifest.csv` rather than leaving artefact-existence conditional.
**Recommendation:** Pick one path and pin it. Recommended: always commit `scripts/synth_prescreen_manifest.py` (the synthesis is small but reusable for iter-3+), and have AC-06 verification grep `git ls-files scripts/synth_prescreen_manifest.py` returns 1. The "trivial" path adds review noise for no durable artefact.

### [MEDIUM] AC-08 hinges on the harness attributing CvC; spec acknowledges mismatch but exit_conditions enforces it
**Category:** constraint-conflict
**Pass:** 2
**Description:** Constraint #10 (lines 53–57) reads "Mechanism mismatch is reported, not corrected. … No re-pick policy." But `exit_conditions` line 399 reads "ac-08 satisfied: format_diversity ≥1 AND code_vs_canonical ≥1 in the post-state report." If FIFA's `nationality` column lands in misclassification (parallel to iter-1's un_locode), and S&P 500's GICS Sector column also misclassifies (likely — it's a free-form short-string column the model has historically mis-routed), then CvC stays at 0 and the exit condition cannot be met without a re-pick — directly contradicting Constraint #10. The spec doesn't have a fallback if the targeted mechanisms genuinely don't fire.
**Evidence:**
- spec.yaml:53–57 (constraint #10 — no re-pick).
- spec.yaml:399 (exit condition — strict ≥1 each).
- Iter-1 precedent: 4 of 7 datasets misclassified (un_locode, world_population, us_baby_names, rio2016_athletes) before the targeted mechanism could fire. Misclassification rate was high enough that all 5 CvC-targeted datasets misclassifying is plausible.
**Recommendation:** Add an explicit fallback to constraint #10 OR weaken the exit condition. Two options:
  (a) "If after picking 5 datasets neither mechanism fires, the spec ships with a documented gap and a follow-up card; AC-08 is downgraded to a stretch goal." (Honest about the limitation.)
  (b) "If a mechanism remains at 0 after the 5 picks, the implementer may add a 6th dataset within the same bucket as a one-time exception; constraint #10 applies only to *post-merge* mismatches, not pre-merge picks that produce no signal." (Adds a re-pick escape hatch.)
  Pick one. Today the spec promises both "no re-pick" and "≥1 each" — those collide.

### [MEDIUM] Manifest schema names a 9-column shape but field count drifts
**Category:** constraint-conflict
**Pass:** 1
**Description:** AC-03 (line 98) names 9 columns: `dataset, file_path, source_url, licence, fetched_date, provenance_status, gt_sidecar_path, row_count, column_count`. Constraint #8 forbids modifying iter-1 rows. A row count of 9 for the manifest header is correct against the on-disk file; that part is right. But the implementation_notes step 2 (line 333) says "Update validate_manifest.csv (additive — 9-column dataset-level rows)" while step 4 says "synthesise the per-column eval-shaped prescreen manifest (one row per (dataset, column_name, gt_label) tuple, joining validate_manifest.csv + GT sidecars)." This implies a join key between dataset-level manifest and column-level GT sidecars. The join key isn't named — `gt_sidecar_path` in manifest gets you to a YAML file, but the YAML's column-key shape has to match by convention, not contract. If a GT sidecar omits a header that exists in the CSV, the synthesis silently drops it and prescreen never sees it; the realism gate becomes incomplete.
**Evidence:** spec.yaml:333–337 (synthesis step). No spec defines what happens if `len(GT-sidecar.columns) != CSV-header-count` — and constraint #4 ("100% GT column coverage") *should* prevent that, but there's no machine check.
**Recommendation:** Add a pre-synthesis check: "Before AC-06 runs, verify each new GT sidecar's `columns:` map cardinality matches the CSV's header column count exactly. Fail the iteration if mismatch." This binds constraint #4 to a deterministic gate, closes the silent-drop hole, and makes AC-02's "100% column coverage" claim audit-checkable mechanically.

### [LOW] AC-05 wc-l grows-strictly check is too weak
**Category:** test-gap
**Pass:** 2
**Description:** AC-05 verification (line 140) says `wc -l eval/row_hashes.tsv` "strictly grows from the iter-1 baseline." Strictly grows is satisfied by a single new row — but the firewall semantics depend on **all** new rows being captured, not just one. A regression where `compute_row_hashes.py` silently skips 4 of the 5 new datasets (e.g., a parsing failure on one dataset surfaces but the other 4 hit a TypeError mid-run) would still produce strict growth.
**Evidence:** Line 140; `scripts/compute_row_hashes.py` design. The leakage firewall regression test mentioned in line 142–145 is the load-bearing check, but the spec leaves it implicit which exact behaviour is tested.
**Recommendation:** Two non-conflicting strengthenings:
  (a) Assert the firewall regression test grew its assertion set (3/3 → 8/3 or whatever) — i.e. require at least 1 PASS per new dataset. Or
  (b) Require `wc -l eval/row_hashes.tsv` grew by at least N where N = sum-over-new-datasets of (column_count × min(row_count, 5000)) modulo dedup-tolerance. The spec already computes this magnitude in line 137–138; just turn it into a floor.
  Low priority — the firewall test is the real check. Cosmetic improvement only.

### [LOW] No machine check that GT sidecars reference labels that exist in the live taxonomy
**Category:** failure-mode
**Pass:** 2
**Description:** AC-02 verifies sidecar key counts match CSV header counts — good. But it doesn't verify that each `expected_label` value refers to a real taxonomy type. A typo like `geography.location.us_state` (per finding #1) in a GT sidecar would silently make the round-trip harness emit a "no_gt" or "unknown" attribution when the validator lookup fails. The spec already has the linter material lying around (`finetype taxonomy KEY -o json-schema` returns non-zero exit on unknown keys, per CLAUDE.md card 0006 / MADR 0070), but isn't wired into AC-02.
**Evidence:** AC-02 verification (line 88–91): "count keys under `columns:` matches the CSV's header column count." No label-validity assertion.
**Recommendation:** Add a one-liner check to AC-02 verification: for each `expected_label` in each new GT sidecar, `finetype taxonomy <label> -o json-schema` exits 0. Catches the us_state-style errors mechanically before the harness even runs.

---

## Honest Assessment

The v2 spec corrects **8 of 9** v1 findings cleanly: AC-03 schema, AC-04 grep key, AC-05 magnitude framing, AC-06 manifest-shape via synthesis, AC-08 awk parse via leading-cell grep, AC-09 doc-only via programmatic awk count, AC-07 headline regex, and the rollback path. That's strong revision work.

The remaining concerns are concentrated in two places:

1. **The S&P 500 substitution introduces fresh taxonomy gaps** — pivoting from `ticker_symbol` (doesn't exist) to `us_state` (also doesn't exist) and `CIK` (also doesn't exist) was a sideways move. Only `GICS Sector → representation.discrete.categorical` is sound. The spec needs to acknowledge this and narrow the S&P 500 commitment, OR re-pick once more. Either is fine; today the spec quietly inherits the same problem v1 flagged.

2. **The "no re-pick" constraint collides with the strict ≥1-each exit condition.** Iter-1's 4-of-7 misclassification rate is the relevant base rate; the spec's 5 picks could plausibly all land in misclassification, leaving CvC at 0. The spec must either weaken AC-08 (stretch goal) or add a re-pick escape hatch. As written, those two clauses fight each other.

The biggest risk is shipping iter-2 satisfying 11/11 ACs by the letter while leaving CvC at 0 attribution, the very gap the iteration was scoped to close. That outcome is structurally possible under the current spec.

These are mechanical fixes — none rise to design rework. REQUEST_CHANGES with focus on tightening the S&P 500 sourcing notes and resolving the no-re-pick / ≥1-each contradiction.
