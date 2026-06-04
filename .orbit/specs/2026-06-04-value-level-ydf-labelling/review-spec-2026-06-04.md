# Spec Review

**Date:** 2026-06-04
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-06-04-value-level-ydf-labelling
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 blocking |
| 2 — Assumption & failure | content signals (training data, eval datasets, model inputs, leakage firewall) | 2 LOW |
| 3 — Adversarial | not triggered | — |

---

## Findings

### [LOW] Self-grading firewall (ac-02 OOF) protects the YDF prior, not the CharCNN holdout

**Category:** assumption
**Pass:** 2
**Description:** ac-02's cross-fitting (OOF) stops a gittables value being graded by a YDF tree that trained on it. That is the right guard *for the prior*. But the spec's headline claim is settled by ac-07 — the CharCNN head-to-head on the corpus-pass gated lens. The CharCNN trains on the ac-04 cleaned set (which contains folded-in gittables values) and is then scored on the same corpus. The thing that keeps that comparison honest is the row_hash + `file_content_sha256 MOD 2` firewall (ac-08), *not* the OOF cross-fit. The two firewalls protect different stages and ac-08 is the load-bearing one for the verdict.
**Evidence:** ac-02 ("Cross-fitting (OOF) is used so any gittables value later folded into training is never graded by a tree that saw it"); ac-07 scores cell-1 on the corpus; ac-08 ("Any overlap voids the run"). The substrate firewall files exist — `eval/row_hashes.tsv` (31.8M) and `eval/gittables/corpus_pass/columns.parquet` (300.6M) are both present.
**Recommendation:** None required — ac-08 already gates this and is correctly marked non-negotiable in the goal's halt conditions. Noted only so the implementer treats ac-08, not ac-02's OOF, as the leakage gate for the ac-07 verdict. No spec change.

### [LOW] ac-05 / ac-06 input provenance is unambiguous but worth a one-line guard in the cleaned-NDJSON manifest

**Category:** test-gap
**Pass:** 2
**Description:** ac-06 trains "on the ac-04 (or ac-05) cleaned NDJSON". ac-05 is skippable. If ac-05 runs but is then rejected (does not improve the kept set), ac-06 must train on the ac-04 set — the spec says this, but nothing in the artefact chain records *which* set was actually consumed. A stale ac-05 NDJSON on disk could be picked up silently.
**Evidence:** ac-05 ("keep the single-pass set and record why"); ac-06 ("ac-04 (or ac-05) cleaned NDJSON").
**Recommendation:** Have the ac-06 model manifest (`models/char-cnn-v15-gittables`) record the sha256 / path of the exact NDJSON it trained on. This is a build-hygiene nicety, not a spec gap — the decision is already specified.

---

## Structural scan detail (Pass 1)

- **AC testability:** Every AC is concrete and measurable. ac-00 pins exact numbers (37-dim, 240-dim, named golden values) — and they verify against the tree: `FEATURE_DIM = 37` in `crates/finetype-model/src/features.rs:15`, `decimal_places` at index 36 (line 61) with the exact "latitude 4dp vs magnitude 1dp" framing the AC cites, and 240 taxonomy schemas per CLAUDE.md. ac-03/04/07 carry numeric floors (conf 0.85, per-type N) and explicit go/no-go bands.
- **Gate-AC description check (deterministic):** One gate AC — ac-07 (`is_gate=1`). Description is non-empty, not a placeholder token, far exceeds 20 chars. **Passes all three rules.** It is also the strongest AC in the spec: the win condition is stated in three explicit bands and the spec instructs the verdict be set against that bar *before* the run, with an anti-rationalisation clause. This is exactly the discipline a confounded probe needs.
- **Constraint conflicts:** None. The QUARANTINE-not-relabel policy, the 0.85 floor, and the categorical-exclusion rule are mutually consistent and align with CLAUDE.md (decision 0048 value-based rules; the "zero categorical positive targets" rule mirrors the standing taxonomy constraint).
- **Scope vs goal:** Tightly matched. The goal is a single settle-the-confound experiment; the out-of-scope list explicitly fences off Model2Vec, model promotion, Sharpen rules, and the column-level YDF lens — the last of which is the right call, since leaving it untouched is what keeps the ac-07 metric comparable to prior v19/v22 baselines.
- **Obvious gaps:** Error handling and halt conditions are unusually well covered — ac-03 is a deliberate cheap go/no-go that gates the heavy machinery, and three named halt conditions (starved types, implausible quarantine rate, leakage) sit in the goal. Rollback is inherent: nothing is promoted to `models/default` (explicit out-of-scope), so a NO-GO leaves only artefact files behind.

---

## Honest Assessment

This plan is ready to implement. It is one of the cleaner specs I have reviewed: the central risk — that v14's latitude=0 was a synthetic-data artefact, not an architecture — is named in the goal, and the spec is structured precisely to kill that confound with cell-2 (the dirty-data CharCNN control) rather than dodge it. The win condition is pre-committed with an explicit anti-rationalisation clause, so the verdict cannot drift after the numbers land. ac-03 (survivor census) is a genuinely good piece of design — an afternoon's go/no-go that refuses to build the triad over a starved type, which is the failure mode that would have wasted the whole run.

The two LOW findings change nothing about readiness: both are build-hygiene notes (treat ac-08 not ac-02 as the verdict's leakage gate; record which NDJSON ac-06 actually consumed), and the underlying decisions are already correctly specified. Every concrete technical claim I could check against the codebase held — the 37-dim feature contract, the decimal_places field, the schema-gate commit (24c8f90, "decimal-precision feature + column schema-validation gate"), the gated-lens scripts, and all referenced data files.

The biggest *real* risk is not a spec defect — it is the empirical one the spec exists to measure: a value-level YDF has no column context, so column-context-dependent types (categoricals of repeated short codes) will likely regress, and ac-07 correctly reframes that regression as the *output* (the size of the deferred Model2Vec job) rather than a failure. That is the honest framing. Approve.
