# Spec Review

**Date:** 2026-04-21
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-21-v18-retrain/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (training data, eval datasets, model promotion, symlink flip, cross-cutting release config) + 1 MEDIUM finding in Pass 1 | 3 |
| 3 — Adversarial | not triggered — Pass 2 surfaced no cascading structural defects | — |

---

## Findings

### [MEDIUM] Constraint references a non-existent acceptance criterion `ac-11`
**Category:** constraint-conflict
**Pass:** 1
**Description:** The final constraint states: "No auto-promotion. Winner selection (ac-08) and promotion decision (ac-11) are separate steps with manual checkpoints." There is no `ac-11` in the spec (ACs run ac-01..ac-10). Additionally, `ac-08` is "Data-seed-discipline MADR committed," **not** winner selection — winner selection actually lives in `ac-07` ("Winner selection: highest profile-eval score > highest val_acc > lowest seed number"). This is a cross-reference bug that will confuse the implementer and any downstream automation (`/orb:drive`, `/orb:implement`) that extracts AC IDs from constraints.
**Evidence:** spec.yaml lines 20 (constraint) vs lines 131–143 (ac-07 winner selection lives here; ac-08 is the data-seed-discipline MADR).
**Recommendation:** Fix the cross-references. Likely intent: "Winner selection (ac-07) and promotion decision (ac-10) are separate steps…" — but please confirm against the author's mental model rather than guessing.

### [MEDIUM] AC-04 does not actually verify the row-hash firewall is *active* — only that logging fields exist
**Category:** test-gap
**Pass:** 1
**Description:** The leakage-firewall constraint says: "Log line asserting hash-filter active is required in prep output." AC-04's verification is `grep -E "corpus_base|pre_filter_rows|row_hash_overlap|leaked_rows_after_filter" ... | wc -l` returns ≥ 4. This grep passes whenever the four *keys* appear in the log regardless of values. A sweep that silently disabled the firewall could still log `row_hash_overlap: 0` (because it never ran the filter) and `leaked_rows_after_filter: 0` (because it filtered nothing) and pass the gate. The constraint explicitly calls for a "hash-filter active" assertion; AC-04 does not enforce it.
**Evidence:** spec.yaml line 13 (constraint: "Log line asserting hash-filter active is required"), lines 74–85 (ac-04 description + verification).
**Recommendation:** Add to AC-04's verification either (a) a separate required log line `hash_filter_active: true` and grep for it, or (b) a sentinel (e.g., assert `pre_filter_rows - row_hash_overlap = post_filter_rows` and `post_filter_rows > 0`). A ≥4-line-count grep is not sufficient to prove the firewall ran.

### [MEDIUM] AC-10 embeds full-release-checklist steps inside a retrain spec without a rollback path
**Category:** missing-requirement
**Pass:** 2
**Description:** AC-10 conditionally asks the implementer to (a) upload to HuggingFace, (b) bump `FINETYPE_CI_MODEL` in both CI workflow files, (c) flip `models/default`, (d) reference a release tag and PR. These are 4 cross-cutting changes touching CI, distribution, and runtime defaults — each is individually reversible but the spec does not define a rollback sequence if, e.g., the HF upload succeeds but the `FINETYPE_CI_MODEL` bump fails drift-check, or if users report regressions post-flip. The release skill (`/release`) exists for this flow; AC-10 appears to duplicate part of it without integration. Contrast with v16 release PR #39's discipline (decouple download-model.sh from `models/default`) — the retrain spec should delegate release mechanics to the release skill, not re-specify them.
**Evidence:** spec.yaml lines 159–171 (ac-10); CLAUDE.md "Promotion flow" section lists the same 3-step flow; a `release` skill is available in-session.
**Recommendation:** Narrow AC-10 to "Release scope decided and recorded in handover.md with one of {v0.6.18 shipped, internal-only, held}." Delegate the actual HF/CI/symlink mechanics to the release skill, or spec them in a separate release card once v18 clears the gate. Keep ac-10's verification check (handover.md exists + names the decision) — that part is testable and appropriately scoped.

### [LOW] AC-01 row-count threshold (≥45) softer than description (every failure, ~55)
**Category:** test-gap
**Pass:** 1
**Description:** AC-01 description requires "every v16 misclassification on the expanded 352-col eval (~55 columns)". Verification accepts row count ≥ 45 "allowing triage to merge related failures". The 10-row gap between "every failure" and "≥ 45 allowed" is unspecified — is merging bounded (e.g., max 2 failures per merged row) or unbounded? An implementer could legitimately land 45 rows covering only 45 failures if they assume merging means "skip the tail."
**Evidence:** spec.yaml lines 24–30 (description: "every ... ~55") vs lines 36–40 (verification: "Row count ≥ 45").
**Recommendation:** Either (a) tighten verification to "every v16 failure is accounted for in exactly one row; row count ≤ 55 allows merging two related failures into one row, never dropping failures", or (b) require a separate `triage.md` header field stating `total_v16_failures_covered: NN` that equals the expanded-eval v16 failure count.

### [LOW] Corpus-base MADR (ac-03) and data-seed MADR (ac-08) numbering race
**Category:** missing-requirement
**Pass:** 2
**Description:** Both ac-03 and ac-08 require "a new decision file at `.orbit/choices/NNNN-...md` (next available number)". The next available number is 0060 (decisions 0001–0057, 0059 exist; 0058 appears skipped). Two concurrent MADRs both claiming "next available" will race — ac-03 is gated before prep runs, ac-08 is gated by the sweep-discipline adoption. If both are drafted in parallel they may both pick 0060. This is a minor hygiene issue but visible in git conflicts during implementation.
**Evidence:** spec.yaml lines 62–70 (ac-03), lines 131–143 (ac-08); `ls .orbit/choices/` shows 0057 and 0059 (0058 skipped).
**Recommendation:** Pre-assign the numbers in the spec (e.g., "ac-03 → 0060, ac-08 → 0061") to avoid race, or make explicit that they're written sequentially (ac-03 before ac-08) with a note to re-check `ls .orbit/choices/` before each write.

### [LOW] Sibling-context constraint has no verification AC
**Category:** test-gap
**Pass:** 2
**Description:** Constraint: "Sibling-context attention preserved — `classify_columns_with_context` remains the default profile path. v18 training data must exercise the sibling-context branch (verify during prep)." No acceptance criterion tests this. AC-02 verifies prep runs once; AC-05 verifies per-seed artefacts exist. Neither asserts the sibling-context branch was exercised. If a future `prepare_multibranch_data.py` change accidentally disables sibling-context synthetic columns, v18 would train, pass the gate, and silently ship a regressed inference pipeline.
**Evidence:** spec.yaml line 16 (constraint) has no matching AC; interview line 87 defers verification to "confirm it's exercised in prep" — unspecified how.
**Recommendation:** Add a verification sub-clause to AC-05 (or a new AC) asserting the prep log contains a sibling-context-related tensor/feature count and that `config.json`'s branch configuration includes the sibling-context branch. Alternatively, a smoke-test `finetype profile` invocation post-training that verifies `classify_columns_with_context` is the active path.

### [LOW] No AC-wide assumption that the v4 distilled corpus branch still cleanly applies
**Category:** assumption
**Pass:** 2
**Description:** The interview references "Held assets on branch `distilled-data-relabel-7-types-v17`" (v4 distilled loaders, generator improvements, widened patterns, http_method ENUM-only). If ac-03 chooses `v4` or `v4 + additions` as the corpus base, the spec assumes those branch assets cleanly rebase onto the post-m-19 main. Given m-19 landed after v17 was shelved (manifest schema change 4→7 cols, row-hash firewall, sources.yaml, prep-script filter changes), rebase conflicts in `prepare_multibranch_data.py` are plausible. Nothing in the ACs flags this risk or requires a rebase-clean check before prep.
**Evidence:** interview.md lines 26 (held assets); CLAUDE.md m-19 description (manifest schema migration, firewall added to prep script).
**Recommendation:** Add a sub-bullet to ac-03 (or as a new pre-flight AC): "If corpus base includes v4 assets, confirm branch `distilled-data-relabel-7-types-v17` rebases cleanly onto main or document required merge resolutions in the corpus MADR." This makes the rebase risk visible and decidable.

---

## Honest Assessment

The spec is well-structured and the core shape is right: triage-before-sweep, fixed data seed, per-domain regression floor, no auto-promotion. These are the correct lessons from v17's net-zero outcome and from the m-19 eval-expansion work. The `ac_type: gate` verifications are all substantive (well above the 20-char floor, none placeholder) and the promotion gate is scoped to a testable domain-regression check, not a vague "quality improvement" hand-wave.

What blocks APPROVE is a cluster of concrete defects that will trip the implementer or downstream automation: (1) the `ac-11`/`ac-08` cross-reference is wrong and will confuse both `/orb:drive` and a human reader; (2) AC-04's leakage-firewall verification is a count-of-substrings grep that does not actually prove the firewall ran; (3) AC-10 duplicates release-skill mechanics inside a retrain spec without a rollback path. The biggest risk is AC-04: a silent firewall regression is exactly the failure mode the m-19 work was designed to prevent, and the current verification would not catch it. The other findings (AC-01 row-count softness, MADR-number race, sibling-context verification gap, v4-rebase assumption) are polish rather than blockers but are cheap to fix in the same revision cycle. Recommend one REQUEST_CHANGES round addressing at minimum the MEDIUM findings; the spec should clear APPROVE easily on the second pass.
