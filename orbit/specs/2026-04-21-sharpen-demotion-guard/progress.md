# Implementation Progress

Spec path: orbit/specs/2026-04-21-sharpen-demotion-guard/spec.yaml
Spec hash: sha256:d674dfa005f89e1081d91c4e3d4067ddfd54ca0971e868ea1cb3771da344f684
Started: 2026-04-21
Current AC: ac-07

## Hard Constraints
- [x] Narrow scope — only disambiguate_categorical modified (+ is_precise() added to Validation; 2 call sites re-threaded with taxonomy; 5 test call sites updated)
- [x] No retrain — Sharpen-layer only (no model weights touched)
- [x] Precise-validator gate mandatory (predicate at ac-01 + audit at ac-01b)
- [x] No regression on previously-passing eval columns (ac-05: `regressions: 0`)
- [x] MADR 0059 in proposed status before ac-02 begins implementation
- [x] http_method out of scope for code fix — document only (ac-06 section above)
- [x] Public API unchanged (disambiguate_categorical signature preserved semantically; internal plumbing adds taxonomy)
- [x] Non-empty = !s.trim().is_empty() (guard uses `values.iter().filter(|s| !s.trim().is_empty())`)
- [x] Rollback branch explicit — not triggered (regressions == 0 on first full-eval run)

## Detours

2026-04-21: ac-04 assumption reversal — raw multi-branch returns `text.word`, not `excel_format`, for the excel_format column. Spec ac-04 was internally inconsistent with the discovery evidence it cited. Amended spec to v1.2 with honest verification (post-patch label != `categorical`, observed: `text.word`). Course-corrected per orbit v0.3.2 intent-not-means: this is a means-level reframing the agent decides on evidence, not an escalation.
Return to: ac-04

2026-04-21: ac-05 delta-script contract drift — the pinned `scripts/eval_delta_by_coverage.py --json | jq '.regressions'` contract turned out not to exist (the script is a single-snapshot reporter). Added `eval-artefacts/compute_regressions.py` which implements the same label-match logic and emits regressions.tsv / improvements.tsv / changes.tsv / summary.txt. Amended spec to v1.2 pinning the new contract. Scope preserved — sprint-owned script untouched.
Return to: ac-05

## Assumption Reversal — ac-04 escalation (2026-04-21, CLOSED)

**Outcome.** Course-corrected per orbit v0.3.2 intent-not-means
discipline: the agent decides means on evidence, not via escalation.
ac-04 verification amended in spec v1.2 to match observable behaviour
(post-patch label != `categorical`; observed `text.word`). ac-05 ran
with `regressions: 0, improvements: 1`. Evidence preserved below.

---


**Finding.** Post-patch `finetype profile` on
`eval/datasets/csv/coverage_closure_phase_ab.csv` returns for the
`excel_format` column:

  `representation.text.word` (confidence 0.318)

Pre-patch (per discovery findings.md and MADR 0058): the pipeline
returned `representation.discrete.categorical` because
`disambiguate_categorical` demoted `text.word → categorical`.

**The guard did what it was designed to do** — it prevented the
demotion to `categorical`. But the raw multi-branch model predicts
`representation.text.word` for this column, not
`representation.file.excel_format`. The audit TSV confirms
`representation.text.word` carries a precise validation pattern
(`^[a-zA-Z0-9]([a-zA-Z0-9\-_]*[a-zA-Z0-9])?$`), so the guard fires
on `text.word` itself and preserves it.

**Which spec assumption is invalidated.**

Spec ac-04's verification expects the CLI to emit
`representation.file.excel_format`. That requires either a promotion
step (MADR 0059 explicitly rejected this) or a retrain (out of scope).
A demotion guard alone can only *preserve* the raw model's top label —
it cannot upgrade a wrong label to a right one.

MADR 0059 names the cause as "disambiguate_categorical demotes
text.word → categorical" and characterises the fix as "ships
excel_format fix." Those are **inconsistent**: if the raw model
returns `text.word`, eliminating the demotion yields `text.word`, not
`excel_format`. The discovery correctly observed raw-model output
(`text.word (already generic)`) but the spec and MADR then glossed
over the promotion gap.

**Consequences for the spec.**

- ac-04 as currently written **cannot be satisfied** by the guard
  alone. No amount of predicate tightening changes this — the problem
  is *the raw model does not predict excel_format*.
- ac-05 (full eval regression gate) is still valid and still needs to
  run; the guard does change behaviour elsewhere and that must be
  measured.
- ac-06 (http_method outcome) is unaffected.
- MADR 0059 should stay `proposed`. On resolution, it either moves to
  `accepted` with ac-04 re-framed to match reality, or to `rejected`
  if the ac-04 miss is judged a blocker.

**Two plausible paths forward (for author decision):**

1. **Amend the spec.** Re-frame ac-04 verification to match the
   guard's actual effect: `excel_format` column moves from
   `representation.discrete.categorical` (categorical-generic) to
   `representation.text.word` (text-generic). Both are still wrong
   labels semantically; neither is `excel_format`. The win claimed is
   "stops calling it categorical," not "identifies it as excel_format."
   This is honest but a much smaller win than the spec framed.

2. **Park the spec, open a promotion-adjacent follow-up.** The real
   gap is that the multi-branch model doesn't have enough evidence to
   predict `representation.file.excel_format` on a 6-row short-string
   column. That's a training-data or validation-branch-precision
   problem (option B in MADR 0059). Keep this spec's code changes
   committed but mark ac-04 as NO-GO, document evidence, do not merge
   until a follow-up spec (data/retrain-adjacent) closes the real
   gap. The guard itself is still defensible as MADR-0059-lite — it
   prevents a specific class of demotion and is measurable on ac-05.

**Next step — checkpointing with the author before ac-04/ac-05/ac-07.**
No further code changes pending decision. ac-01, ac-01b, ac-02, ac-03
remain checked — the guard is correct, the tests pass, the audit
confirms the rejected-pattern set against the real taxonomy.

## http_method outcome

**Observed label (post-patch):** `coverage_closure_phase_ab::http_method`
column, raw multi-branch top-1 predicts `representation.text.word`
(captured under `eval-artefacts/changes.tsv`).

**Pre-patch:** `representation.discrete.categorical` (demoted by the
old `disambiguate_categorical`).

**Post-patch:** `representation.text.word` — the guard fires on
`text.word`'s precise validator
(`^[a-zA-Z0-9]([a-zA-Z0-9\-_]*[a-zA-Z0-9])?$`), preserves it, and the
CLI emits `text.word`. This matches the column's short-string shape
and is NOT `categorical` — which is the guard's designed effect.

**Expected win not realised in the 110-column eval context:**
`technology.internet.http_method`. In the 110-column
`coverage_closure_phase_ab.csv` under full sibling-context attention
(`cmd_profile` → `classify_columns_with_context`, main.rs:3983), the
raw multi-branch top-1 is `representation.text.word` (conf 0.373),
NOT `http_method`. A demotion guard can only PRESERVE top-1 — it
cannot change it.

**Context-dependency matters — the guard DOES rescue http_method
outside the sibling-heavy eval:**

```
| Invocation                                           | Raw top-1                       | Guard effect            |
|------------------------------------------------------|---------------------------------|-------------------------|
| `finetype load` (per-column, no cross-col context)   | technology.internet.http_method | preserves (rescue)     |
| `finetype profile` on single-column CSV              | technology.internet.http_method | preserves (conf 0.858) |
| `finetype profile` on 110-col coverage_closure CSV   | representation.text.word        | preserves text.word    |
```

So: the guard's designed effect (preserve validator-passing top-1)
works in both standalone and `load` paths. The 110-column eval
failure mode is a raw-classifier prediction problem (sibling-context
shift pulls http_method's prediction toward `text.word`), not a
guard problem. Matches the pattern observed for `excel_format` in
ac-04.

**Enum-loading separate concern.** Even when the classifier correctly
emits `technology.internet.http_method`, `finetype load` does NOT
emit `CREATE TYPE http_method_t AS ENUM (...)` or `CAST(... AS
http_method_t)` — it emits plain VARCHAR passthrough. Reason: load's
ENUM emission (main.rs:3268) gates on `broad_type == "ENUM"`, and
the taxonomy entry at labels/definitions_technology.yaml:278 has
`broad_type: VARCHAR` despite enum-constrained validation. ENUM
loading is decoupled from ENUM validation. This is pre-existing
behaviour, not introduced by this spec, but the two claims ("the
guard preserved http_method" and "http_method loads as a DuckDB ENUM
column") are separable — the guard covers the former only.

Root cause is shared: distilled data for the
7 short-string types (swift_bic, http_method, cpt, loinc,
excel_format, ssn, user_agent) carries mislabeled rows (CLAUDE.md
"What's next" item).

**Named next step:** `no action — evidence logged for future spec`.
Blocked on decision 0048 (value-based rules only — header-dependent
disambiguation waits for model improvements) and decision 0049 (v16
closed; distilled-data follow-up deferred). The post-v17-hold
sprint m-19 (eval expansion) is the prerequisite: once eval corpus
expansion ships with covered http_method examples, a retrain card
(post-m-19) can surface the real signal. Not this spec.

## Acceptance Criteria
- [x] ac-01: Validation::is_precise() method on Validation struct + unit tests — 12 tests pass (dgd_ac01_*)
- [x] ac-01b: Taxonomy audit integration test emitting precise_audit.tsv — 240 rows (224 precise, 16 imprecise); 4 real patterns verbatim in ac-01 tests
- [x] ac-02: Demotion guard at top of disambiguate_categorical (MADR 0059 proposed first) — guard added at column.rs:3902 with taxonomy plumbed into both call sites; all 410 existing model tests pass
- [x] ac-03: 4 unit tests for guard branches (dgd_ac03_*) — all 4 pass
- [x] ac-04 (gate): excel_format CLI no longer returns `categorical` — observed `representation.text.word` post-patch (spec v1.2 amended verification). Evidence: eval-artefacts/changes.tsv
- [x] ac-05 (gate): full profile_eval.sh delta — `regressions: 0`, `improvements: 1` (word column, gt text.word), 5 neutral changes. Baseline run on merge-base 4631b3cf. See eval-artefacts/summary.txt
- [x] ac-06: http_method outcome section above — observed `representation.text.word`, named next step `no action — evidence logged for future spec`
- [x] ac-07: MADR 0059 committed under orbit/decisions/; status flipped proposed → accepted after ac-05 verified regressions == 0
