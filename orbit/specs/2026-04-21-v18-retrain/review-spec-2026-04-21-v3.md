# Spec Review

**Date:** 2026-04-21
**Reviewer:** Context-separated agent (fresh session)
**Spec:** /Users/hugh/github/meridian-online/finetype/orbit/specs/2026-04-21-v18-retrain/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

```
| Pass | Triggered by                                                        | Findings |
|------|---------------------------------------------------------------------|----------|
| 1 — Structural scan        | always                                         | 4        |
| 2 — Assumption & failure   | content signals (training data, models, eval)  | 5        |
| 3 — Adversarial            | not triggered — no structural cascades surfaced| —        |
```

Content signals present: training data / ground truth / eval datasets / model promotion / data migrations (row-hash firewall). Pass 2 ran.

---

## Findings

### [HIGH] ac-04 requires prep-script log markers that do not yet exist in the codebase
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-04 demands that the prep output contain six specific literal log markers — `corpus_base`, `pre_filter_rows`, `row_hash_overlap`, `post_filter_rows`, `hash_filter_active: true`, `leaked_rows_after_filter: 0`, plus `eval_hash_table_sha256: <hex>`. A grep of `scripts/prepare_multibranch_data.py` finds none of these strings present today. The spec frames ac-04 as a verification assertion on `results/sweep-v18.log`, but does not explicitly designate any AC as "add the instrumentation." ac-02 only requires that `sweep_v18.sh` invoke prep once with `--seed 42`; it does not require modifying `prepare_multibranch_data.py` itself.
**Evidence:** `scripts/prepare_multibranch_data.py` currently emits `sibling_headers`/`n_sibling_headers` log content but none of the six hash-firewall markers or the SHA256 marker. The row-hash filter code exists (it's referenced as active in the m-19 sprint goal and `eval/row_hashes.tsv` is populated) but its observability surface is not yet aligned to ac-04's verification contract. Spec lines 90–117.
**Recommendation:** Add an explicit code AC (or amend ac-02) that names the prep-script instrumentation work: "`prepare_multibranch_data.py` (or `sweep_v18.sh`) must emit the following seven log lines in order: … ". Without this, a correct-looking sweep run will trivially fail ac-04's verification on instrumentation gaps, and the implementer will have to infer that adding the markers is in scope.

### [HIGH] Dependency chain ac-01 → ac-03 → prep → ac-02 is implicit; sweep_v18.sh smoke-test in ac-02 cannot honour it
**Category:** constraint-conflict
**Pass:** 2
**Description:** Constraints require triage (ac-01) to complete before the corpus MADR (ac-03) is written, and the corpus MADR to be `accepted` before prep runs. ac-02's verification, however, includes a dry-run smoke test: "`SEEDS=(42)` completes prep once and training once." The smoke test demands a runnable prep invocation — which demands a decided corpus base. If the implementer runs ac-02's smoke test before ac-01/ac-03 complete, they have either (a) no corpus to point at, or (b) they pick one silently, which undermines ac-03's "decision before prep" contract.
**Evidence:** Constraint line "Corpus base (v3 / v4 / v4+additions) is a triage-informed decision — must be recorded in a MADR before prep runs." (line 18) vs ac-02 verification: "Smoke test: dry-run with `SEEDS=(42)` completes prep once and training once." (lines 68–69). ac-03's verification gates on the file existing with status `accepted` (lines 83–88), but ac-02's verification as written does not defer the smoke test.
**Recommendation:** Make the dependency explicit in ac-02: either (a) scope ac-02's smoke test to a script-syntax / dry-run-without-execution check (no prep invocation), or (b) annotate that ac-02's runtime smoke test runs only after ac-03 is `accepted`. Alternatively, restructure so ac-02 covers only the script skeleton and a new AC covers the end-to-end smoke.

### [MEDIUM] ac-07 winner-selection tie-breaker order is not load-bearing against the gate
**Category:** test-gap
**Pass:** 2
**Description:** ac-07 defines winner selection as "highest profile-eval score > highest val_acc > lowest seed number" and then applies the promotion gate (`≥ 297/352` AND per-domain regression ≤ 3) to the winner. This risks a false negative: the highest-profile-eval seed may fail the per-domain regression floor while a lower-profile-eval seed passes it. The spec does not say whether to (a) fail the gate and HALT, or (b) fall through to the next seed. Given that per-domain regression is the explicit anti-pattern the spec is trying to avoid (v17 trap), falling through feels correct but is unwritten.
**Evidence:** ac-07 description (lines 164–177) defines selection then verification. No branch for "winner fails gate but other seed passes." Constraint "No auto-promotion. Winner selection (ac-07) and release-scope decision (ac-10) are separate steps with manual checkpoints." (line 22) hints at manual override but does not answer the tie-break-vs-gate ordering.
**Recommendation:** Add one sentence to ac-07: either "If winner fails the gate, the sweep is HELD regardless of other seeds' scores" (strict) or "If winner fails the gate, candidate re-selection may descend the tie-break order; each candidate must independently pass the gate" (permissive). Current silence forces the implementer to guess.

### [MEDIUM] ac-01's "equals the v16 failure count" verification is brittle to manifest/eval drift
**Category:** failure-mode
**Pass:** 2
**Description:** ac-01 verification requires `total_v16_failures_covered: N` where N "equals the v16 failure count computed from `eval/eval_output/profile_results.csv` vs `eval/datasets/manifest.csv` (label + domain match)." But the m-19 sprint goal notes v16's diagnostic re-score produced 297/352 (~55 failures). The v16 baseline is described as "pinned via git SHA of eval inputs" in the constraints (line 12), but ac-01's verification does not cite a SHA; it reads live files. If the eval output or manifest is regenerated between triage write and verification, N drifts silently and the equality check fails for non-triage reasons.
**Evidence:** Spec line 12: "Promotion gate (pinned v16 baseline via git SHA of eval inputs)". Spec lines 39–51: ac-01 verification pins to live paths (`eval/eval_output/profile_results.csv`, `eval/datasets/manifest.csv`) without naming a SHA.
**Recommendation:** Either (a) extend the "pinned via git SHA" pattern to ac-01 by having triage.md record the SHA of the inputs it was computed against and verifying N against those pinned inputs, or (b) add a pre-flight step that regenerates `profile_results.csv` against v16 immediately before triage and records the SHA used.

### [MEDIUM] ac-05 "six files each" list silently omits val/test artefacts
**Category:** test-gap
**Pass:** 2
**Description:** ac-05 enumerates exactly six required files per seed: `results.json`, `epochs.jsonl`, `config.json`, `model.safetensors`, `label_map.json`, `eval/report.md`. Several training runs in this repo's history (per CLAUDE.md training infrastructure section) also produce `results.json` + `epochs.jsonl` as incrementally-written artefacts — fine, those are listed. But the `eval/report.md` artefact is ambiguous: is it the profile eval markdown written by `eval/profile_eval.sh`? Or a different per-seed eval report? This path is not standard for this repo — profile eval normally writes to `eval/eval_output/report.md`, not into `models/<name>/eval/report.md`. The implementer will either create a new convention or fail the AC.
**Evidence:** CLAUDE.md file-reference table: "Eval manifest (7-col, 448 rows) | `eval/datasets/manifest.csv`"; "`make eval-report` generates `eval/eval_output/report.md`." Spec ac-05 line 129: "`eval/report.md`" relative to the model directory. The relative-path location is unspecified.
**Recommendation:** Either (a) name the absolute path (e.g., `models/sherlock-v18-seed-XX/eval/report.md`) explicitly and note what writes it, or (b) if it's a per-seed `profile_eval.sh` invocation writing into the model dir, confirm the wrapper pattern exists (or add it as part of ac-02's script work).

### [LOW] Time-budget constraint has no AC enforcement
**Category:** test-gap
**Pass:** 2
**Description:** The constraint "Time/compute budget: per-seed training ≤ 4h soft / 6h abort-and-investigate hard on M1 Pro Metal. Total sweep wall-clock ≤ 12h end-to-end (prep + 3 seeds + eval per seed)." is load-bearing (6h hard abort) but no AC verifies it. There is no hook saying "if any seed exceeds 6h, abort and file a halt note." In practice this is likely an operator-enforced constraint, but the spec's AC surface is silent on measurement.
**Evidence:** Spec line 23. No matching AC in 1–10.
**Recommendation:** Either (a) downgrade the "6h abort" from load-bearing to "soft guidance" in the constraint text (and document what happens on 8h overrun), or (b) add verification: a wall-clock measurement to `results/sweep-v18-summary.csv` and an AC that fails if any `time_secs > 21600` without an `abort_reason` logged.

### [LOW] ac-09 conditional verification branches are under-specified
**Category:** test-gap
**Pass:** 2
**Description:** ac-09 requires `0062-v18-outcome.md` with status `accepted` and "Outcome section explicitly states 'promoted' or 'held' and includes a per-column fixes/regressions breakdown." The spec's exit_conditions include three terminal states — `promoted`, `held`, `halted` — but ac-09 lists only two branches. If the sweep halts (ac-06 HALT condition), ac-09's verification demands a fixes/regressions breakdown that doesn't exist (no eval was run).
**Evidence:** Spec ontology_schema.sweep_outcome: `enum[promoted, held, halted]` (line 252). exit_conditions (lines 266–269) enumerate all three. ac-09 verification (lines 212–215) only names `promoted` or `held`.
**Recommendation:** Extend ac-09 to include a `halted` branch: on halt, the MADR records hypotheses from ac-06's halt investigation (not fixes/regressions), and there is no requirement for a per-column breakdown.

### [LOW] ac-06 "HALTED not retried in-session" is a workflow instruction, not a verifiable AC
**Category:** test-gap
**Pass:** 2
**Description:** ac-06's description says "The v18 spec moves to status HALTED; any retry is a separate follow-up spec, not an in-session loop." Verification is the hypothesis-shape check on `progress.md`. There's no verifiable assertion that the spec metadata actually moved to `status: halted`. The spec's current metadata has `version: 1.2` and no status field at all.
**Evidence:** Spec metadata block lines 271–283 — no `status:` field. ac-06 verification (lines 152–161) doesn't require metadata update.
**Recommendation:** Either add a `status: active` field to metadata now (so the halt transition is verifiable) and require its update in ac-06 verification, or remove the "status HALTED" language from ac-06 and make the halt signal solely the `progress.md` section + the absence of any promotion action.

### [LOW] ac-03 requires "at least three specific triage.md rows" as evidence — triage may have fewer relevant rows for v3-corpus choice
**Category:** failure-mode
**Pass:** 2
**Description:** ac-03 verification: "cites at least three specific triage.md rows (by dataset::column) as evidence." But if triage concludes the corpus should stay on v3 (no change), the MADR may cite rows in the negative — e.g., "the 7 types v4 was built around do not appear in the expanded-eval failure set, therefore v3 is sufficient." That's a defensible conclusion with zero rows being cited positively; the "at least three rows" floor forces artificial padding.
**Evidence:** ac-03 description (lines 72–82) does not constrain the polarity of the evidence. Verification (lines 83–88): "cites at least three specific triage.md rows."
**Recommendation:** Soften to "cites at least three specific triage.md rows OR records an explicit negative-evidence argument naming the failure categories that would have justified v4 and why they are absent."

---

## Honest Assessment

This spec is well-structured, rigorously gated, and clearly iterated (v1.2 revision notes show two prior review cycles). The constraints are load-bearing and specific; the ACs are largely testable; MADR pre-numbering to avoid race conditions is a nice touch. The data-seed-discipline insight is sound and well-motivated.

The biggest risk is the HIGH finding on ac-04: the verification contract names log markers that the prep script does not currently emit. If the implementer reads "ac-04 is a code AC" and assumes the markers exist, they will cargo-run the sweep, notice `hash_filter_active: true` is missing from the log, and either (a) fail the AC and ask, or (b) silently add the markers ad-hoc. Making the prep-script instrumentation an explicit scope item closes this gap.

The second HIGH (ac-01/ac-03 → prep ordering vs ac-02 smoke test) is a workflow-order ambiguity the implementer will probably resolve correctly but the spec should not force them to. Both HIGHs are fixable with small edits — not design-level rework. MEDIUMs are tractable clarifications. This is a REQUEST_CHANGES for precision, not a BLOCK.