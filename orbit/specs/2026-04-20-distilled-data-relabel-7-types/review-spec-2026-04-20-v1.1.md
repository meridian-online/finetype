# Spec Review (v1.1)

**Date:** 2026-04-20
**Reviewer:** Context-separated agent (fresh session, cold read of v1.1)
**Spec:** `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (metadata.version 1.1)
**Verdict:** REQUEST_CHANGES

---

## Summary

v1.1 is substantially tighter than v1.0 — the four HIGH findings from the prior review are closed in spirit and (mostly) in mechanism. The residual risks are different in character: v1.0 was "wrong in plan"; v1.1 is "right in plan but thin in verifiable substance" at a couple of seams. Two HIGH-priority issues remain (data-availability evidence for ac-01/ac-02; a tiebreaker/rollback cliff in ac-10–ac-12) plus four MEDIUMs worth closing before `/orb:implement`.

---

## v1.0 HIGH findings — closure assessment

1. **Licensing reality (v1.0 HIGH #1):** **CLOSED.** v1.1 drops the license-review machinery and replaces it with the simpler "public dataset OR generator" rubric (constraints L14–19, L32–35). Only 2 of 7 types go to public datasets; the other 4 go to improved generators and http_method becomes schema-only. This is pragmatic and matches decision 0049's thesis.
2. **Eval gate brittleness (v1.0 HIGH #2):** **CLOSED.** ac-11 + ac-12 now capture a v16 baseline at training start and the gate is `max(235, v16_baseline)`. Good.
3. **HTTP-method ENUM surfaces (v1.0 HIGH #3):** **CLOSED.** Split cleanly into ac-06 (YAML), ac-07 (validator unit test), ac-08 (cascade doc). ac-06 explicitly calls out that enum + pattern must BOTH be updated — and that is correct: `CompiledValidator::is_valid` in `crates/finetype-core/src/validator.rs:95` delegates to a single `jsonschema::Validator` that applies pattern and enum conjunctively (AND), so updating only the enum would still fail on lowercase input because the pattern regex `^(GET|POST|...|CONNECT)$` rejects it.
4. **N=1 email data-blend (v1.0 HIGH #4):** **CLOSED.** Explicitly split back out (goal L10–11, constraint L78–80, changes_from_v1_0). Cross-reference preserved.

## v1.0 MEDIUM findings — closure assessment

5. **Decision 0049 amend-vs-supersede (MEDIUM #5):** CLOSED. ac-09 now asserts "status remains `accepted`", with date-modified + Update section. Unambiguous.
6. **Label-remap validator (MEDIUM #6):** CLOSED. ac-05 mandates a new script `scripts/validate_label_remap.py`. Minor nit below — see H2.
7. **Val-acc 91.2% gate (MEDIUM #7):** CLOSED. Relaxed to 88% catastrophic floor with 88–91.2% manual-review band (constraint L60–64, ac-10). Rationale is given.
8. **3-seed compute budget (MEDIUM #8):** CLOSED. `~7.5h on M1 Pro Metal` now explicit (constraint L56–58).

All eight v1.0 findings are substantively addressed.

---

## v1.1 Findings (graded fresh)

### [HIGH] ac-01/ac-02 data availability is asserted, not evidenced

**Category:** assumption
**Description:** The sourcing table (constraints L21–29) commits to "public dataset (Kaggle/GitHub UA corpus)" for user_agent and "public dataset (GitHub mirror or Kaggle)" for LOINC, with ac-01 requiring "≥ 1000 rows" each. There is zero evidence in the spec or interview that:

- A permissively-licensed user-agent corpus of ≥1000 rows actually exists on Kaggle/GitHub (e.g. ua-parser fixtures are smaller; Mozilla has crowdsourced UA collections but distribution varies).
- A LOINC values dataset of ≥1000 rows exists on Kaggle/GitHub with a license that does not transitively derive from the LOINC User Agreement (LOINC itself is Regenstrief-licensed; derived value dumps may or may not inherit obligations).

v1.0's HIGH #1 was "licensing attrition" — v1.1 dropped the licensing machinery, which makes the spec cleaner, but it also replaced a hard problem with silence. If either of these public datasets does not exist on acceptable terms, ac-01 silently cannot be met and ac-04 (prep-script integration) has no input. There is no fallback AC ("if user_agent public dataset is unavailable, fall back to improved generator").

**Evidence:** spec.yaml L21–29; ac-01 L93–103; interview Q19 A (tabular commitment); no discovery of actual candidate URLs.
**Recommendation:** Either (a) name the candidate datasets in the spec (Kaggle/GitHub URL + row count + license) so the reviewer can verify existence, or (b) add a pre-flight AC: "Before writing loaders, confirm candidate source exists and meets ≥1000 rows + permissive license; if no candidate, convert that type's row to `improved generator` and update the sourcing table." The second is lower-effort and preserves graceful degradation.

### [HIGH] ac-12 "winner" is undefined; no tiebreaker, no rollback

**Category:** failure-mode
**Description:** ac-12 uses "winner" as if it's a single value but never defines it (best profile eval? best val_acc? highest single-type gain?). ac-10 additionally allows retaining checkpoints in the 88–91.2% manual-review band, which opens up these scenarios the spec doesn't answer:

- Two seeds tie on profile-eval correct-count (e.g. both 236/242). Which wins?
- Best profile-eval seed has val_acc in the 88–91.2% band (manual review). Is it eligible or is a lower-scoring strict-pass seed the winner?
- All 3 seeds pass the 88% training floor but the best profile-eval is < max(235, v16_baseline). What happens? Rollback steps are not specified — implementer has no script-level behaviour to fall back on; contrast with `sweep_v16.sh` L240–260 which explicitly handles "no model meets target" (leaves models/default alone).
- Best seed's profile eval is exactly 235 but some individual columns regressed. v1.0 HIGH #2's recommendation ("no silent regressions below v16") is NOT in v1.1 — the gate is pure net-positive.

**Evidence:** ac-10 L213–224, ac-11 L226–234, ac-12 L236–246; `scripts/sweep_v16.sh:240-260` shows a working precedent for explicit no-promotion branching.
**Recommendation:** Define "winner" explicitly (suggest: highest profile-eval correct-count; tiebreak by lower variance across eval columns, else lower seed number). Add an explicit no-promotion path to ac-10 or a new ac: "If no seed meets `winner_score ≥ max(235, v16_baseline)`, do not promote; keep `models/default -> sherlock-v16`; no release. Investigation next step documented in progress.md." This mirrors sweep_v16.sh's L254–260.

### [MEDIUM] ac-11 "moment v17 training begins" is ambiguous for a 7.5h sweep

**Category:** assumption
**Description:** ac-11 says v16 baseline is captured "at the moment v17 training begins" (constraint L66–69; verification L232–234). With a 3-seed sweep spanning ~7.5h, "begins" is not self-evident. Options that all fit the phrasing:
- Before seed 42 data-prep runs (earliest).
- After seed 42 training starts (training ≠ prep).
- At corpus finalisation (after ac-04 prep output is written).

v16 eval GT can drift within a single day (CLAUDE.md recent-work notes 17 label fixes + 15 new columns landing in a single spec). If the baseline is captured before corpus prep and GT is updated mid-sweep, the gate is evaluated against a stale denominator.

**Evidence:** ac-11 L226–234; the v16 precedent pinned the baseline implicitly because sweep + eval ran against the same tree.
**Recommendation:** Pin to a specific step: "v16 baseline captured immediately after `scripts/validate_label_remap.py` passes and `scripts/prepare_multibranch_data.py` emits the first seed's FTMB, against the git SHA of `eval/` at that moment. Same git SHA used for v17 profile eval in ac-12." The "same git SHA" is the load-bearing part — it makes the comparison apples-to-apples regardless of wall-clock.

### [MEDIUM] ac-06 enum-expansion strategy — "pick whichever flows through CompiledValidator correctly" is a decision deferred into implementation

**Category:** test-gap
**Description:** ac-06 L166–168 offers two paths ("add (?i)" vs "enumerate variants explicitly") and tells the implementer to "pick whichever flows through CompiledValidator correctly". This is fine operationally but it is a meaningful schema-design decision (enum size 9 vs 27; regex with inline flag vs without) that will affect `finetype schema` JSON-Schema output and the validation-branch feature dimensionality. A JSON-Schema `enum` is a first-class part of the type contract — bloating it 3× without a decision captured hides the trade-off.

The YAML file (`labels/definitions_technology.yaml:285-286`) currently has `pattern: "^(GET|POST|…|CONNECT)$"` + `enum: [GET, …, CONNECT]` — both uppercase-only. Choosing `(?i)` + enum-stays-uppercase keeps the published schema clean but means the enum no longer enumerates the accepted set (lowercase passes pattern but fails enum → AND → overall fail). So (?i)-only is incorrect; enum must also be expanded or enum must be removed. Spec doesn't call this out.

**Evidence:** `crates/finetype-core/src/validator.rs:95` (is_valid delegates to single `jsonschema::Validator`, conjunctive); ac-06 L162–171.
**Recommendation:** Pick the strategy in the spec, not at implementation time. Suggest: "enumerate all 27 variants explicitly in `enum`; pattern uses `(?i)` OR enumerate all 27 in the pattern alternation too. Validator unit test in ac-07 must cover both pattern and enum keywords (not just `is_valid`)." Document the choice in ac-15 MADR (b).

### [MEDIUM] ac-08 is a non-AC (stylistic cascade documentation)

**Category:** missing-requirement (meta)
**Description:** ac-08 requires "no code change in validation_features.rs; SOURCES.md mentions the cascade explicitly under http_method". The second half is fine. The first half ("no code change") is a negative assertion — it verifies that the implementer *didn't do something*. This is an anti-AC: at PR review time, a reviewer reading the diff already knows validation_features.rs wasn't touched; no independent signal is added. And the "stop calling it a filter" language policy is enforced nowhere concrete (interview, spec, commit messages all read by humans).

**Evidence:** ac-08 L186–197.
**Recommendation:** Collapse ac-08 into ac-06's SOURCES.md entry — "SOURCES.md under http_method documents the validation-branch cascade (retrain picks up pass-rate feature)." Drop the negative code-change assertion; it's implied by scope. Save ac-08 for something load-bearing. (Alternately: mark ac-08 explicitly as a "documentation AC" tagged as such, so /orb:review-pr scans it with looser expectations.)

### [MEDIUM] Per-type sourcing table lives in constraints, not an AC — no structural guard against drift

**Category:** test-gap
**Description:** The critical "type → path" decision table lives in constraints L21–29. If it changes during implementation (e.g. LOINC public dataset turns out to not exist, so LOINC silently becomes "improved generator"), nothing in the acceptance criteria catches the change. ac-03 verifies SOURCES.md *exists* with an entry per type but doesn't assert those entries match the constraints table. ac-04's behaviour branches on whether a type has distilled data or not — which also assumes the constraints table.

`ontology_schema.DistilledDataSource` has a `source_type` enum (public_dataset | generator | schema_only), so the structural shape is there. It's just not wired to an AC.

**Evidence:** constraints L21–29; ac-03 L118–129; ac-04 L131–146.
**Recommendation:** Add explicit wiring — either (a) an AC: "SOURCES.md table matches the constraints sourcing table; any divergence requires a spec amendment", or (b) move the table from constraints into its own `sourcing_table:` field at the top of the spec alongside `goal:` / `constraints:`, and have ac-03 + ac-04 reference it by name. Option (b) makes the table first-class and reviewable.

### [LOW] ac-05 validator scope creep possible

**Category:** assumption
**Description:** `scripts/validate_label_remap.py` doesn't exist today (verified: grep of `scripts/` shows no label-remap validator). Creating a new Python script is fine, but "traverses transitive chains" opens the door to building a mini-type-system: does it just follow A→B chains, or does it also verify every taxonomy key resolves, check for cycles, check aliases, etc.? `data/label_remap.json` is currently 44 entries, flat (no transitive chains visible in the current file). The v16 "broken chains" issue was transitive (description→title→sentence→paragraph→plain_text) — so the script must at least walk transitively, but nothing forces it to stay small.
**Evidence:** `data/label_remap.json` (46 lines, no transitive chains at rest today); CLAUDE.md v16 recent-work section.
**Recommendation:** Bound the script's scope in ac-05: "verifies (i) every key's value is a canonical taxonomy key (in `labels/`), (ii) no cycles, (iii) transitive resolution terminates at a canonical key. No other checks." Keep it deliberately tiny.

### [LOW] ac-15 MADR-count vs implicit-decision-count

**Category:** gap
**Description:** ac-15 enumerates four MADRs. `changes_from_v1_0` also mentions "per-type sourcing table added as first-class constraint" and (implicitly) "decision 0049 amended rather than superseded". The former may be subsumed under MADR (a) "per-type sourcing policy"; the latter is an amendment to an existing decision, not a new one. If that's the intended mapping, say so explicitly — otherwise the reviewer counting `ls decisions/` against the list will wonder whether a fifth MADR is missing.
**Evidence:** ac-15 L272–286; metadata.changes_from_v1_0 L351–361.
**Recommendation:** One-line note inside ac-15 MADR (a): "This MADR absorbs the sourcing-table decision; no separate MADR needed for that alone."

### [LOW] ac-13 HF-upload ordering implicit

**Category:** failure-mode
**Description:** The CI-decouple flow (per CLAUDE.md "Promotion flow") is: (1) HF upload → (2) `FINETYPE_CI_MODEL` bump → (3) `models/default` flip. ac-13 enumerates the three as "(1) HF upload; (2) FINETYPE_CI_MODEL bumped…; (3) models/default flipped" in L251–253 — ordering is present in the numbering but the verification checks are order-independent. If an implementer ran step 2 before step 1, the drift-check would be silent (because CI_MODEL matches default flip once step 3 lands) but HF-download would fail mid-CI during the gap. Small risk; the release skill script almost certainly enforces ordering, but the AC could pin it.
**Evidence:** ac-13 L248–258; CLAUDE.md "Promotion flow (new model → release)".
**Recommendation:** Append to verification: "Order verified by commit history — HF publish PR (or upload commit) lands before the `FINETYPE_CI_MODEL` workflow bump." Low priority; likely enforced by the release skill.

### [LOW] ac-10 seed-naming convention implicit (carryover from v1.0 LOW)

**Category:** failure-mode
**Description:** ac-10 verification path `models/sherlock-v17-seed-{42,43,44}/results.json` (L220–224) presumes the directory naming convention. `sweep_v16.sh` does hardcode `models/sherlock-v16-seed-${SEED}` (L92), so a copy-paste to `sweep_v17.sh` inherits the convention — but the spec doesn't specify this contract.
**Recommendation:** One-line addition — "sweep_v17.sh uses naming convention `models/sherlock-v17-seed-<seed>` mirroring sweep_v16.sh:92." Optional.

---

## Assumption audit

Assumptions still implicit:
- Public user_agent and LOINC datasets of ≥1000 rows exist under permissive licenses. (See H1.)
- v16 GT drift over the v17 training window is negligible if same-SHA-at-eval-time convention is followed. (See M1.)
- `sweep_v17.sh` is a straight adaptation of `sweep_v16.sh`. (Interview Q open question; spec doesn't close it but close enough to be safely inferred.)
- The 88% floor number is "a catastrophic-floor heuristic" not grounded in history. (Spec doesn't cite v13/v14/v15 val_acc as supporting evidence. Interview Q14 cites 91.2% as v16's best but doesn't explain why 88%.) Low priority because the gate is now soft (manual review band).
- http_method enum expansion does not cascade-break any existing consumer of `finetype schema` output (schema consumers depending on exact enum membership could break). Not covered by any AC. Minor — enum-addition is a backward-compatible change for consumers that permit superset values.

## Failure-mode analysis (per-AC highlights)

- **ac-01/ac-02:** source dataset missing/smaller than 1000 → loader silently under-emits. Neither AC has a "what if dataset doesn't meet row-count" clause (see H1).
- **ac-04:** `_DROP_DISTILLED_TYPES` currently contains all 7 types (confirmed L867–875 of prepare_multibranch_data.py); ac-04 requires user_agent + LOINC removed, remaining 4 kept, http_method "in whatever state preserves its schema-only treatment" (L140). Last clause is vague — http_method has no public-dataset and no generator improvement, so its distilled handling should be unchanged (stays in _DROP_DISTILLED_TYPES). Spec should pin that.
- **ac-05:** false pass if the validator is overly permissive (doesn't walk transitively). Covered by the "deliberate broken chain in a test fixture" sub-check — good.
- **ac-10:** all 3 seeds fail the 88% floor → no explicit path (see H2).
- **ac-12:** tiebreaker undefined (see H2).
- **ac-13:** out-of-order promotion steps mostly invisible to CI (see L3).

## Test adequacy

Strong:
- ac-07 — unit test with function-name prefix `ac07_…` for AC-coverage scan. Excellent pattern; other ACs should copy it.
- ac-05 — positive AND negative test (deliberate broken-chain fixture). Good.
- ac-10 — structured output files (results.json, epochs.jsonl) per seed, checkable mechanically.

Weak:
- ac-08 — pure grep-based; verifies absence and language hygiene. Low-signal.
- ac-12 — per-column diff table format is hand-wavy ("rows for every mismatch + v16 baseline column"). Prior v1.0 review flagged a similar concern; v1.1 keeps the phrasing. Not blocking but a template reference (e.g. point at v16's eval-comparison file) would help.
- ac-14 — release verification is four artefact checks but no smoke-test that the new binary actually boots and classifies (the v0.6.17 precedent caught the N=1 email regression post-release; a smoke-test in ac-14 would have found it earlier). Optional, not blocking.

## Gap analysis (what's not in the spec)

- **No rollback AC for failed sweep** (see H2).
- **No fallback AC for unavailable public datasets** (see H1).
- **http_method in _DROP_DISTILLED_TYPES** — ac-04 uses "whatever state preserves schema-only treatment"; should be pinned to "http_method stays in _DROP_DISTILLED_TYPES (6 mislabeled rows remain dropped); YAML schema is the sole change for this type."
- **Golden-tests check post-promotion** — sweep_v16.sh prints `"Run golden tests to verify: cargo test -p finetype-cli --test cli_golden -- --ignored"` (L252–253). Spec doesn't fold golden tests into the promotion gate. Recommend add to ac-13 or ac-14.
- **N=1 email handoff crosswalk** — the email spec's future mechanism decision could (in principle) ask for corpus changes. No AC here says "if the email spec chooses corpus-change, coordinate before v17 training starts." Task-flag only; not a spec-block.
- **Drift check in non-promotion branch** — if sweep fails and no promotion, CI_MODEL bump doesn't land. ac-13's drift-check-silent verification only applies to the happy path. Implied but unstated.

## Constraint check

- **Sourcing policy** (L14–19) is consistent internally — public datasets OR generators only.
- **SSN synthetic-only** (L36–40) is consistent with decision 0049 and with the sourcing table.
- **Compute budget** (L56–58) matches sweep_v16.sh:19 ("~2.5 hours per seed on M1 Pro with Metal"). Realistic.
- **Eval gate math**: `max(235, v16_baseline)` — if v16_baseline turns out to be 234 due to GT drift or a new dataset, the gate is 235 (absolute floor). If v16_baseline goes up to 237, v17 must beat 237. Correct, not consistent-by-design; explicit.
- **Training gate 88%** — no citation. The nearest anchor is v16's best 91.2% (constraint L63, ac-10 L217). 88% is ~3.2 points below; treats the band as "model reasonably trained but possibly corpus-sensitive", which is a defensible heuristic but not a grounded threshold. Not a blocker because the band triggers manual review, not auto-accept.

---

## Honest assessment

v1.1 closes all eight v1.0 findings substantively. It is a real improvement: the licensing-attrition risk is eliminated by design (generators are the fallback), the HTTP-method confusion is resolved into three proper surfaces, and the retrain gates are now scope-aware.

What's left is different in flavour: the v1.0 issues were "wrong mental model"; the v1.1 issues are "correct mental model, thin verification contract". The two HIGH items worth blocking on:

1. **H1: ac-01/ac-02 take on faith that the two public datasets exist.** A 10-minute discovery step — name the candidate URLs in the spec — would convert a speculative AC into a verifiable one. Without this, the implementer may burn hours finding nothing usable.
2. **H2: ac-10–ac-12 have no tiebreaker and no rollback.** Precedent exists (sweep_v16.sh:240–260) and the fix is small: pin "winner", add a no-promotion branch.

The four MEDIUM items (baseline timing, enum strategy, sourcing-table first-class, ac-08 is a non-AC) are real but addressable in a single spec edit pass.

**Recommend REQUEST_CHANGES**: resolve the two HIGHs and pick up the four MEDIUMs in the same pass. Once those land, this spec is ready for `/orb:implement`.
