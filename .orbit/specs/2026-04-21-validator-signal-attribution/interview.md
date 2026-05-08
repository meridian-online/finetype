# Discovery: validator-signal attribution

**Date:** 2026-04-21
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml
**Mode:** discovery

---

## Context

MADR 0058 (do-not-promote-v17) surfaced a pipeline gap: `http_method` and
`excel_format` eval columns predict generic
`representation.discrete.categorical` when every sampled value matches
the named type's validator/enum. The initial framing was "wire a
validator-authoritative promotion Sharpen rule."

Hugh pushed back: the multi-branch model already has a **validation
branch** (shipped in v12, spec `.orbit/specs/2026-04-15-validation-branch-v12/`).
Post-hoc promotion would be wallpaper over whatever is preventing that
branch from doing its stated job.

Architecture review (see `findings.md` for the code walk):

- `ValidationFeatureExtractor.extract()` computes a 240-dim pass-rate
  vector via `CompiledValidator::is_valid()` on each type —
  `crates/finetype-model/src/validation_features.rs:77–107`.
- The validation branch MLP is wired into `forward_trunk()` and
  concatenated with char/embed/stats/header before merge —
  `crates/finetype-model/src/multi_branch.rs:767–784`.
- **Critically:** `compute_validation_tensor()` returns `None` unless
  BOTH `self.validation_extractor` AND `taxonomy` are provided
  (multi_branch.rs:710–714). If `None`, `forward_trunk` silently fills
  zeros (multi_branch.rs:771–779).

The silent-zeros pathway is a plausible root cause: if the CLI/eval
inference path doesn't plumb a `Taxonomy` reference through to
`compute_validation_tensor()`, the validation branch receives zeros at
inference regardless of what the training loop provided.

## Q&A

### Q1: How wide should this investigation's scope be?
**Q:** Silent-zeros check only / Silent-zeros + branch-contribution
trace (recommended) / Full validation-branch characterisation (broad)?
**A:** Full validation-branch characterisation (broad). Cover the
silent-zeros yes/no AND the 240-dim vector contents on target columns
AND per-branch ablation to see where the branch earns its keep vs
where it's dead weight.

### Q2: How should we instrument inference?
**Q:** Standalone debug binary / Debug flag on `finetype profile` /
Integration test with fixtures?
**A:** Nightingale's recommendation. **Decision: standalone debug
binary** at `crates/finetype-model/examples/validator_signal_trace.rs`.
Checked in for reproducibility but not shipped as public API. If the
pattern proves reusable, a follow-up spec can promote it to a
`--debug-validation` flag on `finetype profile`.

### Q3: If the silent-zeros hypothesis is confirmed (plumbing bug), do we patch in the same session?
**Q:** Patch + re-eval in same session / Discovery only, fix in
dedicated spec / Defer the decision until we see the evidence?
**A:** Defer the decision until we see the evidence. If the fix is
genuinely trivial AND zero-risk (e.g. pass a taxonomy arg already in
scope), patch-and-measure within the same PR. If the fix is non-trivial
or touches the inference API, stop at discovery and open a dedicated
`/orb:spec` for the fix.

### Q4: Which columns should we trace?
**Q:** http_method / excel_format / a known-good control / country_code?
**A:** **All four.** Covers the two stable-miss failure modes, a
working-case control to confirm the instrumentation isn't lying, and
the v12-designed strong-signal case (country_code) that would reveal
whether the branch is globally dead or selectively dead.

---

## Summary

### Goal
Determine why the multi-branch validation branch does not rescue
`http_method` and `excel_format` eval columns — is it dead at
inference (plumbing), under-weighted (capacity), mis-trained (labels),
or out-competed (feature interaction)? Produce evidence that dictates
the shape of the follow-up spec.

### Constraints
- Discovery only. No production API changes except as follow-up.
- Use sherlock-v16 (current `models/default`) — the measurement must
  match what end-users see.
- Instrumentation is a standalone debug binary, not a shipped flag.
- Four target columns fixed: http_method, excel_format, a known-good
  email control, country_code.

### Success Criteria
- `findings.md` answers all four open questions:
  1. Does sherlock-v16's saved config have `has_validation_branch() ==
     true` and populated `type_index_keys`?
  2. At eval-time inference, is `compute_validation_tensor()` returning
     `Some(tensor)` or silently `None`?
  3. On http_method/excel_format/country_code columns, what does the
     240-dim pass-rate vector actually contain? Does the expected type
     have pass_rate = 1.0?
  4. What are the final per-class logits? Is the expected type in the
     top-5, or floor-level? Does ablating the validation branch (zeros)
     change the prediction?
- Evidence is reproducible via the checked-in debug binary.
- Root cause is named (silent-zeros / under-weight / mis-trained /
  out-competed) with supporting evidence.
- Follow-up path is decided: same-session patch+measure, or dedicated
  spec.

### Decisions Surfaced
- **Instrumentation form**: standalone debug binary at
  `crates/finetype-model/examples/validator_signal_trace.rs`. Not a
  MADR-worthy decision — reversible, scoped to this investigation.
- **Target columns**: http_method, excel_format, email (control),
  country_code. Covers failure + control + v12-designed strong case.
- **Patch policy**: evidence-led. Not pre-committed.

### Open Questions
- Does the debug binary need taxonomy compiled-validators loaded
  separately, or does the model load them? (Answered by the investigation.)
- If the silent-zeros hypothesis is false, which of the three remaining
  hypotheses (under-weight / mis-trained / out-competed) does the
  evidence best support? (Depends on what we observe.)

---

**Next step:** Execute the investigation — write the debug binary,
run it on the four target columns with sherlock-v16, and record
evidence in `findings.md`.
