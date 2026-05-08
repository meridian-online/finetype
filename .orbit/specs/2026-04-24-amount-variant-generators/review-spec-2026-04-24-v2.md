# Spec Review

**Date:** 2026-04-24
**Reviewer:** Context-separated agent (fresh session, v1.1 cycle 2)
**Spec:** .orbit/specs/2026-04-24-amount-variant-generators/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 |
| 2 — Assumption & failure | content signals (training data, model artefacts, eval datasets, cross-spec impact) | 3 |
| 3 — Adversarial | not triggered — Pass 2 surfaced no cascade or rollback concerns | — |

---

## Cycle 1 → Cycle 2 regression check

v1.0 surfaced 3 HIGH, 4 MEDIUM, 3 LOW findings. v1.1's changelog claims all ten
are addressed. Verified against the spec text:

```
| v1.0 finding                                      | Severity | v1.1 location                                                                 | Verdict |
|---------------------------------------------------|----------|-------------------------------------------------------------------------------|---------|
| ac-09 toothless gate (test cannot fail)           | HIGH     | ac-09 verdict file `GO`/`NO-GO`; test asserts verdict ↔ net_lift agree        | FIXED   |
| v18 corpus fixture unpinned                       | HIGH     | constraint 10 + ac-01 row-hash pin `diagnostics/v18_corpus_hashes.tsv`        | FIXED   |
| ac-07 MADR markdown-token parsing unspecified     | HIGH     | constraint 11 closed-enum frontmatter + ac-07 reads field directly            | FIXED   |
| ac-05 mechanism menu non-exhaustive               | MEDIUM   | `other` added to enum; `## Ruled Out` with ≥3 alternatives + evidence         | FIXED   |
| ac-08 no non-target regression guard              | MEDIUM   | new ac-10 (341-row full-eval guard + override semantics)                      | FIXED   |
| ac-02 degenerate-signature test gap               | MEDIUM   | ac-02 assertions (d) not-all-equal and (e) ≥3 distinct signatures/subtype     | FIXED   |
| ac-04 softmax sum not verified                    | MEDIUM   | ac-04 adds (c) strict (0,1), (d) top-1 ≥ 1/240, (f) top-5 sum in (0, 1.0]     | FIXED   |
| ac-03 hypothesis encoded as pre-condition         | LOW      | ac-03 drops `>=8 wrong` threshold; asserts structural integrity only          | FIXED   |
| ac-06 generator.rs cross-caller impact unscoped   | LOW      | constraint 12 — `cargo run -- check`, MCP note, pre-fix generator for ac-07   | FIXED   |
| 1-seed noise floor uncalibrated                   | LOW      | constraint 13 — net_lift ∈ {3,4} requires 3-seed confirmation in v19-proper   | FIXED   |
```

All ten v1.0 findings materially addressed. The fixes tighten machine-enforced
contracts (verdict files, frontmatter enums) rather than papering over via
prose, which is the right direction for this spec's integrity.

---

## Findings (v1.1 review)

### [MEDIUM] ac-02 RNG seeding not specified — Jaccard matrix is not bit-reproducible

**Category:** test-gap
**Pass:** 2

**Description:** ac-02 samples 100 generated values per subtype via
`generator.generate_value()` which uses `self.rng`. Constraint 8 requires
diagnostic artefacts to be "reproducible TSV files, each regenerable from a
single script committed alongside the spec." The ac-02 test asserts matrix
symmetry, diagonal, range, non-degenerate signatures, and ≥3 distinct
signatures per subtype — but does not require the sample to be deterministic
across runs. Two invocations of the script may produce distinct
`jaccard_matrix.tsv` contents differing in the fourth decimal place, which
breaks the reproducibility principle (evaluation_principles weight 0.20) and
also means ac-07's pre/post comparison for `overlap` mechanism
(`mean off-diagonal reduced by ≥ 0.1`) measures sample noise on top of the
actual fix.

**Evidence:** spec.yaml ac-02 verification (lines 32–33); constraint 8 (line 17).
Contrast with ac-01, which pins determinism explicitly ("the script produces an
identical file on a second invocation given the same v18_corpus_hashes.tsv pin").

**Recommendation:** Add to ac-02 verification: "The sampling script seeds the
generator RNG deterministically (e.g., seed=42 per subtype) and the test asserts
the script produces an identical file on a second invocation." This also
strengthens ac-07 `overlap` post-fix by ensuring the 0.1-threshold measures
fix-signal, not sampling jitter.

---

### [LOW] ac-07 `imbalance` 30% ratio-reduction threshold is unachievable on near-balanced corpora

**Category:** assumption
**Pass:** 2

**Description:** ac-07's `imbalance` assertion requires `max_count/min_count`
ratio in `corpus_counts_post.tsv` to be reduced by ≥ 30% versus
`corpus_counts.tsv`. If the diagnosis names `imbalance` at a modest pre-ratio
(e.g., 2.0), a 30% reduction means post-ratio ≤ 1.4 — possibly unachievable
without oversampling the minority classes to an artificial density. The test
is well-calibrated for severe imbalance (ratio in double digits) but can
force-fail a good-faith remediation when the pre-ratio is borderline.

**Evidence:** spec.yaml ac-07 verification `imbalance` branch (line 61).

**Recommendation:** Non-blocking for approval. Consider softening to "ratio
reduced by ≥ 30% OR post-ratio ≤ 1.5, whichever is first achieved." If the
mechanism is `imbalance` but ac-07 still fails on this technicality, the spec
author can document the near-balanced case in the MADR and note the gate's
limitation.

---

### [LOW] ac-07 `confident_wrong` post-fix assertion overlaps ac-09 target gate

**Category:** test-gap
**Pass:** 2

**Description:** If `primary_mechanism: confident_wrong`, ac-07 asserts "≥ 3
target subtypes' top-1 prediction flips from wrong to correct" on the same 11
eval columns. ac-09 asserts `net_lift >= 3` on the same 11 columns
post-retrain. In the `confident_wrong` branch, mechanism-verification and
unblock-gate collapse into substantially the same measurement — the spec's
declared separation between "mechanism-verified (ac-07)" and "measurable lift
(ac-09)" becomes fictitious for this mechanism token. This weakens the
constraint-4 principle ("mechanism-verified without lift means the hypothesis
was wrong; lift without mechanism means the fix is unattributable") in exactly
the mechanism where the distinction matters least.

**Evidence:** spec.yaml ac-07 `confident_wrong` branch (line 63) vs ac-09
net_lift definition (line 74). Not a defect — both measurements are valid —
but the "separation of concerns" framing in constraint 4 is narrower than the
text suggests for this branch.

**Recommendation:** Non-blocking. Optional: for `confident_wrong`, run ac-07's
pre/post on the *same v16 model* (validating value_sharpen or rule-based
remediation moves predictions without retraining), while ac-09 runs on the
v19-smoke retrained model. This keeps the arc honest — ac-07 verifies the
mechanism can be nudged at inference time; ac-09 verifies the nudge survives
retraining end-to-end. If the intended remediation is data-only (corpus edit
+ retrain), this separation cannot apply and the two measurements legitimately
converge — note this in the MADR.

---

## Honest Assessment

v1.1 is a materially stronger spec than v1.0. All ten cycle-1 findings were
addressed with machine-enforced contracts (verdict files, closed-enum
frontmatter, row-hash pins, dual regression-guard) rather than prose patches —
the right shape of fix for a spec whose entire value proposition is
"mechanism-verified, not mechanism-claimed." The non-target regression guard
(new ac-10) is the structurally most important add: v18's HELD precedent is
now respected by construction, not by reviewer vigilance.

The three residual findings are implementation polish: ac-02 sampling
determinism, ac-07 `imbalance` threshold edge case, and ac-07 / ac-09 overlap
on the `confident_wrong` branch. None of them threaten the integrity of the
diagnostic arc. The MEDIUM (RNG seeding) is worth a one-line constraint add
during implementation but does not warrant another spec cycle — the fix is
mechanical and the implementing agent can add it without re-interviewing.

Ready to drive. Biggest residual risk is that the `primary_mechanism` menu
still classifies any genuine "pipeline artefact" cause (label_remap miswiring,
sibling-context bias, embedding saturation) as `other` — which is permitted
and contract-enforced via the `post_fix_assertion` frontmatter escape-hatch —
but the `other` branch is the least tested in v1.1. If ac-05 lands on `other`,
review the `post_fix_assertion` contract carefully at implementation time.
