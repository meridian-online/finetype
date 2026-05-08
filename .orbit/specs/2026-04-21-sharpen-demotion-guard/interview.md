# Design: Sharpen demotion guard — validator-confirmed named types are not demoted to generic categorical

**Date:** 2026-04-21
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml
**Mode:** design (drive iteration 1)

---

## Context

Card 0002 — *Semantic type detection*. 4 scenarios, goal: 201/227 (88.5%)
baseline extending upward. Current state on the expanded eval: 297/352
label (84.4%) / 326/352 domain (92.6%) on v16 diagnostic re-score (the
drop from 235/242 is because eval-expansion surfaced previously-uncovered
type coverage).

### Prior specs on this card (most recent 3)
- `2026-04-21-eval-expansion/` — 448-row corpus with 240/240 coverage
  shipped PR #42. Enabled the measurement that surfaced this gap.
- `2026-04-15-validation-branch-v12/` — added the validation branch
  that was the subject of the `2026-04-21-validator-signal-attribution`
  discovery.
- `2026-04-12-accuracy-gap-retraining/` — earlier retrain push.

### Discovery evidence (front-loaded — feeds this design)

Location: `.orbit/specs/2026-04-21-validator-signal-attribution/`
(interview.md + findings.md + `validator_signal_trace.rs` debug binary).

Three findings govern this design:

1. **Silent-zeros hypothesis FALSE.** Validation branch receives real
   features at inference. No plumbing bug.
2. **Validator precision pollution.** On `http_method` values
   (GET/POST/...), 25+ types return pass_rate = 1.000 — expected type
   ranked 25/240 tied with 24 others. The validation branch CAN
   discriminate where validators are precise (country_code enum,
   email regex — ablation verified) and CANNOT where they aren't
   (http_method, excel_format).
3. **Post-processing demotes correct predictions.** For http_method,
   raw multi-branch returns `http_method` (0.595); full CLI pipeline
   returns `categorical` (0.373) — something between raw and final
   strips the correct answer. For excel_format, raw multi-branch
   returns `text.word` (already generic), `disambiguate_categorical`
   at `column.rs:3881` then demotes `text.word → categorical` because
   its guard "top is generic + 3-20 unique short non-numeric values"
   matches exactly.

### Reframe

The original framing — "validator-authoritative promotion" — was a
`categorical → named_type` promotion rule. The evidence shows the
actual pattern is `named_type → categorical` demotion. The correct
intervention is a **demotion guard, not a promotion step**.

### Gap this design closes

Narrow, no-retrain, Sharpen-layer intervention that prevents
`disambiguate_categorical` from demoting a named-type prediction when
every sampled value passes that type's validator AND the validator is
precise.

---

## Q&A

The four design questions were proposed interactively and delegated to
the agent ("too low level"). Decisions recorded with reasoning.

### Q1 — Where should the demotion guard live?

**Options:**
- A. `disambiguate_categorical` only (column.rs:3881)
- B. `disambiguate_categorical` + trace+patch the http_method path
- C. Generalised guard across all Sharpen demoters

**A. (`disambiguate_categorical` only).**

Reasoning:
- Discovery confirmed `disambiguate_categorical` is the demoter for
  `excel_format`. That's the concrete, verified fix.
- `http_method`'s demotion path is different and unconfirmed. Best
  hypothesis is sibling-context enrichment (pre-Sharpen), which would
  need a different intervention entirely.
- Generalised (C) risks regression across untested paths.
- Ship the narrow, verified fix first. If `http_method` doesn't
  rescue, open a follow-up card with the evidence from this spec's
  implementation + eval.

### Q2 — How is "validator-confirmed" defined?

**Options:**
- A. All values pass + validator is "precise"
- B. All values pass + pass-rate vector uniqueness check
- C. All values pass (simplest)

**A. (all pass + precise validator).**

Reasoning:
- The 25-way pass_rate tie on short strings is a *training-time*
  concern for the validation branch (its feature vector is flat on
  those columns). At *guard time* the raw label has already been
  chosen, so the risk shifts to "the raw label is a loose-validator
  type."
- Option C (all pass only) fails when the raw label is, say,
  `representation.text.word` whose validator is functionally `^.+$` —
  every short string passes, the guard would fire, demotion would be
  wrongly blocked.
- Option B requires computing the 240-dim pass-rate vector at
  Sharpen time — adds latency per column.
- Option A defines "precise" as:
  (a) enum-constrained (non-empty enum list), OR
  (b) regex with ≥1 anchored character class (not `^.+$`, not
      a pattern that matches every alphanumeric short string).
- Aligns with CLAUDE.md's Precision Principle: *"A validation that
  confirms 90% of random input is not a validation."*
- A helper `CompiledValidator::is_precise()` encapsulates the check;
  unit-testable.

### Q3 — Acceptance test surface

**Options:**
- A. Eval delta on expanded manifest (448 rows) only
- B. Unit test + narrow eval on 2 columns
- C. Both — unit + full eval

**C. (both).**

Reasoning:
- The guard touches every categorical demotion path. Unit alone
  leaves regression risk unbounded across 35 datasets.
- Narrow eval on 2 columns (B) would confirm the happy-path fix but
  not catch collateral damage.
- Full `profile_eval.sh` runs in minutes on the 35 datasets. Cheap
  insurance.
- Acceptance:
  (a) Unit tests in `column.rs` tests module cover the guard —
      at least: precise-and-passes (guard fires), precise-and-fails
      (guard doesn't fire), imprecise-and-passes (guard doesn't fire),
      no-validator (guard doesn't fire).
  (b) Full `profile_eval.sh` produces label accuracy ≥ baseline AND
      `excel_format` column flips categorical → excel_format AND
      no previously-passing column regresses.

### Q4 — Record a MADR?

**Options:**
- A. New MADR `demotion-guard-over-promotion`
- B. Spec-only, cite discovery in spec body
- C. MADR later if the direction is re-opened

**A. (new MADR).**

Reasoning:
- This reverses the direction implied by MADR 0058's follow-up
  ("open a validator-authoritative promotion spec"). A future reader
  encountering 0058 would expect to find a promotion spec, not a
  demotion guard. The MADR is the bridge.
- The decision is evidence-driven (discovery findings.md) and worth
  preserving beyond spec body, which is implementation-focused.
- Orbit discipline: *decisions captured, not forgotten.*
- MADR number: next available in `.orbit/choices/` (0059 based on
  latest = 0058).

---

## Summary

### Goal
Add a demotion guard to `disambiguate_categorical` that preserves a
named-type prediction when every non-empty sampled value passes that
type's validator AND the validator is precise. Narrow, no-retrain,
ships `excel_format` fix. Records `http_method` as open follow-up.

### Constraints

- **Narrow scope.** Only `disambiguate_categorical` is modified. Other
  Sharpen rules (attractor demotion, header sharpen, etc.) are
  untouched in this spec.
- **No retrain.** Must ship without retraining any model. Sharpen-
  layer change only.
- **Precise-validator gate.** "All values pass" alone is insufficient
  (discovery evidence: 25+ types pass_rate=1.000 on short strings).
  The guard requires the validator to be enum-constrained OR to have
  an anchored char class in its regex.
- **No regression on previously-passing eval columns.** Full
  `profile_eval.sh` is the gate — not a narrow 2-column test.
- **MADR required.** MADR 0059 documents the rejection of the
  promotion approach.

### Success Criteria

- Unit tests cover the 4 guard branches (precise+pass / precise+fail /
  imprecise+pass / no-validator).
- `finetype profile` on
  `eval/datasets/csv/coverage_closure_phase_ab.csv` returns
  `representation.file.excel_format` for the `excel_format` column
  (currently returns `representation.discrete.categorical`).
- Full `profile_eval.sh` on the 448-row expanded manifest:
  - label accuracy ≥ 297/352 (current v16 baseline)
  - domain accuracy ≥ 326/352
  - no previously-passing column regresses (delta script must
    show 0 regressions on pre-existing rows; the `excel_format`
    column is a fix).
- `http_method` behaviour documented in `progress.md` — if unchanged
  (still categorical), evidence for follow-up card.
- MADR 0059 committed.

### Decisions Surfaced

- **Demotion guard over promotion rule** (→ MADR 0059). Discovery
  showed the failure pattern is `named_type → categorical` demotion,
  not `categorical → named_type` omission. The original
  promotion-style spec is rejected.
- **"Precise validator" defined as:** enum-constrained OR regex with
  ≥1 anchored character class. Helper on `CompiledValidator`.
- **Guard scope narrowed to `disambiguate_categorical` only.** Other
  demoters (attractor demotion, header sharpen, the unknown http_method
  path) are out of scope — open follow-up if evidence warrants.

### Open Questions

- `http_method`'s demotion mechanism remains unconfirmed (likely
  pre-Sharpen sibling-context enrichment). Captured in spec's
  progress.md as a documented finding; does not block this spec.
- What's the exact predicate for "anchored char class" in the
  `is_precise` helper? — the spec will specify. First pass: reject
  regexes whose pattern string is one of `{^.+$, ^.*$, ^\\S+$}` or
  whose character class set contains only `.`, `\\w`, or `\\S`;
  accept all others.

---

**Next step:** `/orb:spec` to crystallise into `spec.yaml`.
