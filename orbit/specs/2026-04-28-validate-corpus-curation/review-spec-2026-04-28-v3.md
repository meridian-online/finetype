# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | content signals (eval datasets, training-leakage firewall, taxonomy GT) + 1 MEDIUM finding in Pass 1 | 2 |
| 3 — Adversarial | not triggered (changes are additive, revertible per constraint #11; no cascading scope) | — |

## Findings

### [MEDIUM] AC-06 verification awk index targets the wrong column
**Category:** test-gap
**Pass:** 1
**Description:** AC-06's verification (lines 192-198) asserts `pass_floors=True` for every new-dataset row by running `awk -F'\t' '$6=="False"' <output> | wc -l` returning 0. Column 6 of `prescreen_eval.py`'s output is `total` (the row count), not `pass_floors`. The actual column order, defined at `scripts/prescreen_eval.py:324-341`, is:

```
1=dataset, 2=file_path, 3=column_name, 4=gt_label, 5=full_type,
6=total, 7=non_null, 8=null_rate, 9=unique_ratio, 10=whitespace_ratio,
11=format_variance, 12=shannon_entropy, 13=top_1_skew,
14=pass_floors, 15=pass_notes, 16=error
```

`pass_floors` is field **14**, not field 6. The check `$6=="False"` examines `total`, which is always a numeric string ("100", "5000", etc.), never the literal string "False" — so the test will *always* return 0 regardless of actual prescreen pass/fail state. The realism gate is silently null.

**Evidence:**
- `scripts/prescreen_eval.py:324-341` defines `OUT_FIELDS` in the order above. Verified by reading the file directly.
- The v2 review's own `## Iter-2 expected vs actual` regex was hand-checked but this awk index was not.
- A simple `awk -F'\t' 'NR>1 && $14=="False"' <output> | wc -l` is the correct check.

**Recommendation:** Change AC-06 verification line 197 from `$6=="False"` to `$14=="False"` (or use a header-aware check: `awk -F'\t' 'NR==1{for(i=1;i<=NF;i++)c[$i]=i;next} $c["pass_floors"]=="False"' <output>`). Either form works; the column-name lookup is more robust to future schema drift. Without this fix, AC-06 cannot detect a realism failure even if every new dataset bombs the floor.

### [MEDIUM] AC-10 claims `make ci` runs the leakage firewall regression test, but it doesn't
**Category:** test-gap
**Pass:** 1
**Description:** AC-10 (lines 257-266) reads "Includes the leakage firewall regression test from iter-1 (3/3 PASS expected)." But `make ci` (Makefile:40) is `fmt + clippy + test + check` — none of those targets shells out to the Python firewall test at `scripts/eval_leakage/test_validate_corpus_firewall.py`. Verified:

- `grep -n "firewall\|test_validate_corpus\|test_filter_pipeline\|test_normaliser" Makefile .github/workflows/ci.yml` → 0 matches.
- `cargo test` runs only Rust tests; the Python suite under `scripts/eval_leakage/` is not invoked by any orchestration in this repo.

So AC-10's "3/3 PASS expected" claim is structurally aspirational — `make ci` exiting 0 says nothing about the firewall test's status. Worse, AC-05's verification (line 168-172) leans on the same firewall test having been **extended** to ≥8 PASS, but provides no orchestration that would make that extension actually run during the iteration's verification.

**Evidence:**
- Makefile line 40: `ci: fmt clippy test check`.
- Repo-wide grep (above) shows no invocation of the Python firewall test from CI or `make`.
- This is an inherited gap from iter-1 — not iter-2's fault — but the spec doubles down on the wrong claim instead of fixing it.

**Recommendation:** One of:
  (a) Wire the firewall test into `make ci` (or a new `make test-firewall` target that AC-10 explicitly runs and verifies). Cleanest; closes a real iter-1 hole.
  (b) Drop the "3/3 PASS" claim from AC-10 and make AC-05's verification call the firewall test directly: `python3 scripts/eval_leakage/test_validate_corpus_firewall.py` exits 0. Then AC-05 becomes self-contained; AC-10 reverts to a pure byte-stability gate.
  (b) is the lower-risk shipping path for iter-2 (it adds one line to AC-05 verification; no Makefile change). (a) is the right long-term fix but expands iter-2's scope.

### [LOW] AC-09 grep counts at least 5 rows; the spec contemplates a 6th pre-merge dataset
**Category:** constraint-conflict
**Pass:** 1
**Description:** Constraint #10 (lines 59-70) adds a "Pre-merge escape hatch" allowing the implementer to add ONE additional dataset (a 6th iter-2 pick) if the post-state report shows 0 attributions in either target mechanism. AC-09 verification (lines 248-255) asserts the mismatch table has ≥5 data rows. If the escape hatch fires, the table will have 6 rows — that still passes `≥5`, so this is consistent. But: AC-07's verification (line 216) asserts `^\*\*[0-9]+ of 12 datasets pass` — and the escape-hatch path explicitly says the headline becomes `N of 13 datasets pass`. AC-07 will fail under the escape-hatch path even though the spec authorises it.

**Evidence:**
- spec.yaml:64-65 (escape hatch headline becomes "N of 13 datasets pass").
- spec.yaml:216 (AC-07 verification grep `^\*\*[0-9]+ of 12 datasets pass`).
- spec.yaml:218 (AC-07 verification "Per-dataset table has 12 data rows").

**Recommendation:** Soften AC-07's verification to accept either 12 or 13: `grep -E "^\*\*[0-9]+ of (12|13) datasets pass"` and "Per-dataset table has 12 or 13 data rows." A two-token alternation in the regex is a one-character change; the per-dataset count assertion needs a brief clause covering the escape-hatch case. Today, exercising the escape hatch the spec itself authorises mechanically fails AC-07.

### [LOW] No floor on the realism-gate output path; default vs explicit collide
**Category:** test-gap
**Pass:** 2
**Description:** AC-06 verification (lines 194-198) says `python scripts/prescreen_eval.py --manifest /tmp/validate_corpus_iter2_prescreen.csv` exits 0, then "The output TSV (passed via `--output`)…". But the verification command shown does NOT pass `--output`, and `prescreen_eval.py` defaults to `eval/eval_output/prescreen.tsv` (`scripts/prescreen_eval.py:362-366`). If the implementer follows the verification literally, the output lands at the default path — which would clobber any in-flight m-19 prescreen state. If the implementer adds `--output /tmp/...` to match the parenthetical, fine — but the verification command should match its own description.

**Evidence:**
- spec.yaml:194 verification command (no `--output` flag).
- spec.yaml:195 description ("output TSV passed via `--output`").
- `scripts/prescreen_eval.py:362-366` (default = `eval/eval_output/prescreen.tsv`).

**Recommendation:** Make AC-06's verification command explicit:
```
python scripts/prescreen_eval.py \
  --manifest /tmp/validate_corpus_iter2_prescreen.csv \
  --output /tmp/validate_corpus_iter2_prescreen.tsv
awk -F'\t' 'NR>1 && $14=="False"' /tmp/validate_corpus_iter2_prescreen.tsv | wc -l   # → 0
```
One-line edit; resolves both the awk-index defect (finding #1) and the implicit-output-path issue together.

### [LOW] AC-08 ≥1-each gate cannot fail soft within the spec — escape hatch only delays the cliff
**Category:** failure-mode
**Pass:** 2
**Description:** Constraint #10 (lines 67-70) says "If after the 6th pick a target mechanism still shows 0, AC-08 is downgraded to a documented gap and a follow-up card is created — the iteration ships rather than spinning indefinitely." Good — but `exit_conditions` line 436 still reads "ac-08 satisfied: format_diversity ≥1 AND code_vs_canonical ≥1 in the post-state report." The exit conditions block enforces the strict gate; constraint #10 says the gate becomes a stretch goal in the worst case. Two ship contracts. The implementer reading exit_conditions will hit a hard stop; the implementer reading constraint #10 will know it's softenable. The spec needs to pick one source of truth.

**Evidence:**
- spec.yaml:67-70 (downgrade-to-gap clause).
- spec.yaml:436 (`exit_conditions: ac-08 satisfied: ... ≥1 AND ... ≥1`).

**Recommendation:** Update `exit_conditions` line 436 to: "ac-08 satisfied OR documented as a gap per constraint #10's escape-hatch fallback." That makes the soft-fail path machine-readable from the same block that gates `/orb:drive`'s ship decision. Otherwise constraint #10's escape hatch is rhetorically present but operationally absent.

---

## Honest Assessment

V3 cleanly addresses all six v2 findings (S&P 500 narrowing, AC-06 committed-script disambiguation, no-re-pick vs ≥1-each via the pre-merge escape hatch, sidecar cardinality machine check, AC-05 firewall-test-extended floor, taxonomy-label validity check). That is a good cycle.

The remaining concerns are mechanical and cluster on **verification rigour** rather than design:

1. **AC-06's awk column index is genuinely wrong** — `$6` is `total`, not `pass_floors`. The realism gate is silently null until that's fixed. This is a one-character-but-load-bearing edit.
2. **AC-10 promises a firewall-test result that `make ci` never measures** — inherited from iter-1, but the spec restates it instead of fixing or disclaiming. Easiest fix: have AC-05 invoke the firewall test directly.
3. **The escape hatch the spec authorises makes AC-07 fail** — `12 datasets` is hardcoded in AC-07's verification while constraint #10 contemplates 13. One regex alternation closes this.
4. **`exit_conditions` and constraint #10 disagree** about whether AC-08 is a hard gate or a stretch goal — `exit_conditions` should mirror constraint #10's escape-hatch language so one source of truth governs the ship decision.

None of these rise to design rework. All are textual/mechanical fixes within the spec — likely a 30-line revision. Scope, mechanism targeting, dataset selection, GT discipline, leakage firewall extension, and rollback path are all sound.

The biggest risk is shipping iter-2 with AC-06 mechanically passing (because `$6` is always numeric and never "False") while a quietly-failing realism floor leaves a noisy dataset in the corpus — same shape as iter-1's un_locode misclassification mask, but introduced by a verification-only bug rather than a curation choice. That's worth one more cycle to fix.
