# Spec Review

**Date:** 2026-04-24
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-24-amount-variant-generators/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (training data, model artefacts, eval datasets, cross-spec impact) + Pass 1 findings | 5 |
| 3 — Adversarial | Pass 2 surfaced structural concerns (toothless gate, non-exhaustive mechanism menu, fixture unpinned, cascade tests) | 2 |

---

## Findings

### [HIGH] ac-09 gate test cannot fail — gate is toothless in code

**Category:** test-gap
**Pass:** 1

**Description:** ac-09 verification specifies the gate test "passes with GO if net_lift >= 3; passes with NO-GO (assertion-skip + explicit result message) if net_lift < 3." Both branches result in a passing test. The only enforcement of the GO/NO-GO outcome is a prose entry in `progress.md`. A gate-AC whose test cannot fail is a human-enforced gate, not a machine-enforced one — at odds with the spec's evaluation_principle "Sprint-policy enforcement — the v19 hard gate is codified in MADR and respected" (weight 0.20).

**Evidence:** spec.yaml lines 61–63. MADR contract in ac-05/ac-10 is enforced by file-existence + frontmatter checks in ac-05 verification, but ac-09's outcome gate is never asserted. Compare with ac-03, which *does* assert a numeric threshold (`at least 8 rows have is_correct==0`).

**Recommendation:** Split ac-09 into two separate tests and make the gate binary:
- `test_amvg_ac09_unblock_gate_go` — runs only when progress.md records GO; asserts `net_lift >= 3`.
- `test_amvg_ac09_unblock_gate_no_go` — runs only when progress.md records NO-GO; asserts `net_lift < 3`.

Or, simpler: encode the outcome as a required artefact (e.g., `diagnostics/v19_smoke_verdict.txt` containing exactly `GO` or `NO-GO`), and have the test assert that the file contents match the computed `net_lift` threshold. The CLAUDE.md update in ac-12 already branches on this outcome; machine-enforce the branch point.

---

### [HIGH] Fixture for ac-01 (v18 corpus) is not pinned — risks silent drift

**Category:** missing-requirement
**Pass:** 1

**Description:** ac-01 requires "corpus-count-per-subtype table produced for the v18 training corpus." The interview (lines 183–184) flagged this as an open question: "the v18 `output/multibranch-training/v18.ftmb` was deleted post-sweep per v18 handover line 54 — may need regen." The spec does not resolve this. If the corpus is regenerated with post-v18 generator changes, ac-01 counts the wrong corpus and every downstream diagnostic (ac-02..04 → ac-05 mechanism) inherits the drift.

**Evidence:** spec.yaml ac-01 (lines 20–23) references "v18 training corpus" without pointing to a reproducible source. Interview lines 183–184 explicitly names this as an unresolved fixture question. Compare with m-19 leakage-firewall discipline (row-hash SHA256 at `eval/row_hashes.tsv`) — fixture pinning is load-bearing for this repo's diagnostic integrity.

**Recommendation:** Add a constraint: "Pin the v18 corpus fixture as either (a) a regenerated FTMB from the v18-era `scripts/prepare_multibranch_data.py` at a specific commit SHA, or (b) a row-hash manifest at `diagnostics/v18_corpus_hashes.tsv` that any regeneration must match." ac-01's test should verify the fixture-pin hash before counting.

---

### [HIGH] ac-07 contract couples a Rust/Python test to a MADR markdown token — parsing contract unspecified

**Category:** test-gap
**Pass:** 2

**Description:** ac-07 says "Specific assertion is pinned in the test by the ac-05 MADR's primary mechanism token." No mechanism token extraction contract is specified: is the token a frontmatter field (e.g., `primary_mechanism: overlap`), a tagged line in the body, or a heading? Tests that parse markdown by regex are brittle; if the MADR author writes "value-shape overlap" vs "overlap" vs "Value Shape Overlap", the test may silently pick the wrong assertion branch or fall through to a default.

**Evidence:** spec.yaml ac-05 (line 43) says the MADR "names exactly one primary mechanism (or declares multi-cause with percentages)" — but does not specify the machine-readable form of the name. ac-07 (line 53) assumes a token exists.

**Recommendation:** Require a frontmatter field in the MADR — e.g., `primary_mechanism: one of {imbalance, overlap, confident_wrong, flat_confidence, multi_cause}` — with a closed enum. ac-05 verification should assert the frontmatter field is set to one of these exact tokens. ac-07's test reads this field directly (no body parsing). Also specify behaviour for `multi_cause`: does ac-07 require one post-fix artefact per cause, or only the dominant one?

---

### [MEDIUM] Mechanism menu in ac-05 may not be exhaustive — no "none of the above" branch

**Category:** assumption
**Pass:** 2

**Description:** ac-05 allows one of {volume imbalance, value-shape overlap, confident-but-wrong, flat-confidence, multi-cause}. Plausible mechanisms outside this menu include: tokenisation collapse (digit-run compression erasing subtype signal), embedding-branch saturation, sibling-context attention bias toward `finance.currency.amount` as the majority column in finance-heavy test sets, label-remap chain misrouting a subtype to plain amount at data-prep time, or header-hint cross-domain override. If the true cause is "none of the above", ac-05 forces a false classification, which then anchors ac-06 and ac-07 to the wrong remediation.

**Evidence:** spec.yaml line 42. Interview Q1 correctly punts on hypothesis ("not sure — measure first") but the menu introduced at spec-stage is narrower than the interview's implicit surface. CLAUDE.md "architectural direction" names sibling-context attention, Model2Vec hints, label_remap — none represented in the menu.

**Recommendation:** Add an explicit escape-hatch option: "other — named-and-justified". Require ac-05 MADR to list the candidate mechanisms it *ruled out* (with one-line evidence each), not just the one it selected. This preserves the honest-diagnosis principle (weight 0.30) against confirmation bias toward the menu.

---

### [MEDIUM] ac-08 does not gate on non-target regressions — wider impact invisible

**Category:** failure-mode
**Pass:** 2

**Description:** ac-08 delta artefact has 11 rows (one per target subtype). A remediation that fixes 5 target subtypes but regresses 3 non-target finance columns (e.g., plain `amount`, `price`, `fee`) would report net_lift=+5 and pass ac-09 — while making v19 a net-worse model. The expanded eval manifest is 448 rows; 437 are outside the gate.

**Evidence:** spec.yaml ac-08 (lines 55–58) specifies the 11-row delta only. v18's HELD outcome (decision 0062) was explicitly "net-zero delta" across the full eval — the repo has precedent for treating wider regressions as blocking. ac-09 inherits the narrow scope.

**Recommendation:** Add a widened-scope guard as a sub-criterion of ac-09 (or a new ac-09b): "full-eval delta on the 352-row label score must be >= -1". This is cheap — the smoke eval already runs on the full manifest via `eval/profile_eval.sh`. Without it, the spec can declare v19 unblocked while shipping a regression elsewhere.

---

### [MEDIUM] ac-02 Jaccard test accepts degenerate signatures

**Category:** test-gap
**Pass:** 2

**Description:** The Jaccard test asserts symmetry, diagonal == 1.0, off-diagonals in [0,1]. A broken signature function that collapses every generated value to a single token (e.g., always `"D"` for anything) yields a 12×12 matrix of all 1.0 — symmetric, diagonal == 1.0, off-diagonals in [0,1]. Test passes; diagnostic is garbage.

**Evidence:** spec.yaml ac-02 (lines 27–28).

**Recommendation:** Add an assertion that the 12 signature sets are not all identical (e.g., at least one off-diagonal < 0.99) and that each subtype generates at least 3 distinct signatures from 100 samples. These are weak but catch the degenerate case.

---

### [MEDIUM] ac-04 softmax-consistency check does not verify sum-to-1

**Category:** test-gap
**Pass:** 2

**Description:** "rank-1 confidences sum to a value consistent with softmax output (each row_1 confidence >= each row_2 confidence for the same subtype)" verifies rank ordering only. Softmax outputs sum to 1.0 over the full label space (240 labels), so top-5 may not sum to 1.0 — but the test does not check any sum property. A malformed confidence dump (e.g., raw logits) would pass this test.

**Evidence:** spec.yaml ac-04 (lines 36–38).

**Recommendation:** Assert per-subtype top-5 confidence sum is in (0, 1] (tight bound: top-5 of softmax is at most 1.0, at least 5/240 in the uniform case ≈ 0.021). Better: assert each confidence value is in (0, 1) (strict) and the top-1 is >= 1/240.

---

### [LOW] ac-03 threshold encodes the hypothesis as a test pre-condition

**Category:** test-gap
**Pass:** 2

**Description:** ac-03 test asserts "at least 8 rows have is_correct==0 (matching the v18 diff's 11-persistent-miss finding; tolerates small drift if v16 re-eval changed)." If v16 re-eval on the refreshed corpus shows only 5 subtypes wrong, the test fails — but that outcome would be *good news* (half the problem has already self-resolved). The spec treats the hypothesis's continued truth as a gate.

**Evidence:** spec.yaml ac-03 (lines 32–33). The "tolerates small drift" clause is prose, not encoded.

**Recommendation:** Drop the `>= 8` assertion from the test. Move the "11 persistent misses" claim to the ac-05 MADR's Context section as context-at-time-of-writing, not a pre-condition. ac-03's test should only assert structural integrity (11 rows, is_correct ∈ {0,1}, eval_subtype matches the expected 11 labels).

---

### [LOW] ac-06 remediation via generator.rs edits has cross-cutting impact not scoped

**Category:** failure-mode
**Pass:** 3

**Description:** The conditional remediation menu in ac-06 includes "tightened generator output distribution". `crates/finetype-core/src/generator.rs` is called from training data prep (v19 corpus), taxonomy validation (`finetype check`), eval-row generation, the MCP `generate` tool, and potentially the DuckDB extension. A generator edit for amount subtypes changes synthetic output everywhere. Spec constraint 6 says "every touched area must be tabled in the PR description" — good — but there is no ac that enforces a cross-caller impact scan, and ac-07's pre/post comparison uses the new generator on both sides, masking the change from its own diagnostic.

**Evidence:** spec.yaml ac-06 (lines 46–48), constraint 6 (line 14). CLAUDE.md "Key File Reference" lists generator.rs as core-crate code consumed by multiple downstream crates.

**Recommendation:** Add a constraint: "If ac-06 remediation modifies generator.rs, the PR must include (a) a `cargo run -- check` pass, (b) an explicit note on whether `finetype generate` MCP tool output changes, and (c) ac-07's post-fix diagnostic must re-use the *pre-fix* generator when measuring the pre baseline (or equivalently, regenerate the pre baseline on the new generator and flag the delta)."

---

### [LOW] v19 smoke noise floor not estimated — net_lift >= 3 threshold uncalibrated

**Category:** assumption
**Pass:** 3

**Description:** The gate is "net_lift >= 3 across 11 subtypes". v18 showed seed-to-seed val_acc variance of ±0.0002 across 3 seeds at 100 epochs — very tight. But the 11 target columns have 6 rows each (coverage-closure dataset); per-column label stability across seeds at this row count is not characterised in any v16/v17/v18 artefact cited. A single-seed run could surface 3 spurious flips. The spec accepts this cost (interview Q8 records Nightingale's "directional signal" framing) but doesn't note it as a known limitation.

**Evidence:** spec.yaml ac-09 (lines 61–63), constraint 5 (line 13). Interview lines 107–113.

**Recommendation:** Add a note under ac-09 verification: "Known limitation — 1-seed smoke conflates signal with seed variance. net_lift >= 3 is a directional threshold, not a statistical significance claim. If ac-09 passes GO but with net_lift ∈ {3, 4}, v19-proper must confirm with a 3-seed sweep before any promotion." This preserves the gate while making the caveat explicit.

---

## Honest Assessment

The spec is well-structured: 12 ACs with concrete artefact paths, numeric thresholds in the right places, MADR obligations for every decision, and a conditional remediation design that respects "measure before committing" (constraint 2). The diagnostic arc (ac-01..04 → ac-05 → ac-06..07 → ac-08..09) is the right shape for a mechanism-hunt spec. The mechanism menu is honestly acknowledged as potentially multi-cause.

The biggest risk is **ac-09's toothless gate**: a test that cannot fail turns the v19-unblock decision into a prose-review outcome rather than a machine-enforced one, at odds with the spec's own principles. Combined with the unpinned v18 corpus fixture and the MADR-token-parsing coupling in ac-07, the spec has three load-bearing points where silent drift or ambiguous contracts can undermine the diagnostic integrity that the rest of the spec fights hard to protect. Non-target regression gating (MEDIUM) is the fourth material concern — v19 could ship net-worse while ac-09 reports GO.

Fix those four (HIGH + HIGH + HIGH + MEDIUM: ac-09 gate, ac-01 fixture, ac-07 MADR contract, ac-08/09 wider-scope guard), plus the mechanism-menu escape hatch, and the spec is ready to drive. The remaining LOW/MEDIUM findings are polish — tighten the test assertions, document the noise floor, acknowledge the generator.rs radius.
