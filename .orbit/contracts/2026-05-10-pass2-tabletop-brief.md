# Pass-2 tabletop brief — GitTables 90% round-trip

**Session:** 2026-05-10 (interactive, Hugh + Nightingale)
**Predecessor contract:** `orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml` (sha `c4012f6a…`)
**Output:** `orbit/contracts/2026-05-10-gittables-90-percent-roundtrip.yaml` (replaces predecessor)
**Cycles audited:** 8 fired (7 full + 1 partial — `df6a4260`); manual halt landed before H13 trigger
**Model under measurement:** `sherlock-v19-relu-s42` / `820d5712…` — unchanged across all 8 cycles
**Pre-activation status:** all 5 P1 cron-pack beads CLOSED. Cron is activated; this is a cycle-audit brief, not a barriers-to-activation brief.
**New P2 engineering beads from cycles:** `finetype-ymi` (bash 3.2 crash) OPEN · `finetype-dwk` (launcher idiom) OPEN

---

## §1 Objective function

### How the gate behaves (read first)

The gate metric is a **promotion benchmark**, not an exploration tool. Three properties:

1. **Frozen holdout.** 2000 files in `eval/gittables/holdout_paths.txt`, fixed at sprint start (seed `20260503`). Every cycle re-measures the same 2000 files.
2. **Two-stage pass criterion** — *not* `valid_rows / all_rows`. A file passes iff both:
   - (a) ≥ 80% of its columns are predicted as a non-trivial type (excludes `representation.text.plain_text` and `representation.numeric.decimal_number`-as-fallback), **and**
   - (b) ≤ 1% of rows are rejected by the validator on those non-trivial columns.
   `gate_score = files_passing / 2000`. Two-stage shape closes the metric-gaming hole flagged in pass-1 hot-wash item 2.
3. **Deterministic.** Same files × same model × deterministic `profile`/`validate` ⇒ byte-identical output. **The gate moves only when the model moves.**

The cycle's *novelty surface* lives in the **corpus-value metric** — 3000 fresh working-slice files per cycle (round-robin, content-hash-deduped). Two metrics, two jobs:

| metric | surface | purpose | moves? |
|---|---|---|---|
| gate (§1) | frozen 2000-file holdout | promotion threshold | only on retrain |
| corpus value (§1) | rolling working slice | failure-mechanism discovery + harvest | every cycle |

### Gate trajectory

```
cycle      gate     pass/2000   err   t(s)    Δ
c7e7a6cc   0.0785   157/2000     54   128     —    (baseline)
99de5b2c   0.0785   157/2000     54   482   +0.0000
ba3bea78   0.0785   157/2000     54   469   +0.0000
df6a4260     —          —         —    —      —    (PARTIAL — H02; no gate run)
58b9ec4a   0.0785   157/2000     54   638   +0.0000
e8fa8742   0.0785   157/2000     54   618   +0.0000
639463c4   0.0785   157/2000     54   211   +0.0000
aaf42b66   0.0785   157/2000     54   229   +0.0000
```

Seven byte-identical 0.0785 measurements. **157 of 2000 holdout files clear both stage-1 thresholds.** Distance to 90%: **82.15 absolute points**. H13's 8-consecutive-Δ<0.1% trigger would have fired on cycle 9 absent the manual halt — exactly as designed.

54-of-2000 holdout files error on every cycle (2.7%) — same files, same errors. Distinct sub-population currently lumped with fails.

### Corpus-value trajectory

```
working slice visited:  21,524 files (~0.6% of 1.0M-file corpus target)
  classifier_quality_issue:  19,186 (89.1%)
  clean_pass:                 1,731  (8.0%)
  corpus_quality_issue:         604  (2.8%)

failure_log:                21,789 entries
harvest_pool:               21,789 entries
```

Corpus-value clean_pass rate (8.0%) tracks the gate (7.85%). **The model performs at ~8% on GitTables-shaped data regardless of how it's sliced** — gate is not a holdout artefact; it's representative.

### Recommendations — §1

1. Keep 90% gate target unchanged. Goal is correct.
2. No amendment to §1 numeric targets. The three bracketing constraints remain valid and untriggered.
3. Add a third outcome bucket: split gate report from `pass / fail` to `pass / fail / error`. The 54 erroring holdout files are a stable sub-population — clarifies whether the next move is model, harness, or corpus filter.
4. Strike the stale ΔT estimate. Update from `~1h on M1` to `~5-10 min on M1`. Real range observed: 128-638s.
5. **Implication for §5 (central tabletop question):** with 7 byte-identical gate scores, the cron has demonstrated it cannot move the gate without intervention. **Either E01 fires (retrain initiates) or H13 fires (contract retires).**

---

## §2 Standing orders

### What fired correctly

| step | result |
|---|---|
| Cycle preamble — contract checksum | ✓ every cycle recorded `contract_sha256 = c4012f6a…` |
| Cycle preamble — lockfile acquisition | ✓ acquired by `scripts/cron_preamble.sh` (finetype-nms) |
| Cycle preamble — disk check | ✓ `free_disk_gb_start` 137-143 GB; H01 never fired |
| Cycle preamble — `cycle_id` + timestamp | ✓ UUID propagated to all log writes |
| Per-cycle — `profile`→`validate` | ✓ ran on 7/8 cycles |
| Per-cycle — append-only logs | ✓ stress-tested by cycle 4 (3 concurrent workers; zero torn rows) |
| Per-cycle — branch application | ✓ B01-B06 fired |
| Postamble — cycle_log entry | ✓ every cycle |
| Postamble — direct push (no PR) | ✓ commits `4f12546`, `d09a72c`, `25ab828` pushed to `main` |
| Invariants — failure_log / coverage_log monotonic | ✓ verified across all 8 |
| Invariants — holdout frozen | ✓ `holdout_paths.txt` byte-identical |
| Scheduling — launchd cadence | ✓ ~2.2-2.5h between cycle starts |

### Edge cases / skipped / errored

1. **Stale ΔT estimate.** Contract said `~1h on M1`; reality 128-638s (mean ~6 min). Order-of-magnitude over-estimate.
2. **Lockfile lifecycle ambiguity (cycle 8).** `cron_preamble.sh` acquires the lock with `$$`, then exits; agent always observes lockfile holding *its own* `cycle_id` with a *dead* PID. Worker reasoned correctly but cost a cycle of escalation reasoning.
3. **`harness_dirty: true` on 3 of 7 full cycles.** Score byte-identical regardless, but reproducibility from `harness_sha` alone is broken on those cycles. No contract policy.
4. **Cycle 4 partial** — see §3 B10 below.

### Recommendations — §2

1. **Clarify lockfile semantics.** Lockfile content = `cycle_id` only (no PID). H04 trigger then becomes "lockfile contains a `cycle_id` ≠ current cycle_id." Decouples preamble shell-PID writing from agent reading.
2. **Strike stale ΔT estimate** (already in §1).
3. **Add `harness_dirty` gate to preamble.** Halt if `git diff --quiet` returns non-zero.
4. **Add `git pull --rebase` to postamble before push.** Belt-and-braces for the no-PR direct-push policy.
5. **No amendment to invariants block.** All four survived cycle 4 stress test.

---

## §3 Branch table

### Branch firing across 7 full + 1 partial

```
branch                                  total fires
B01_misclassification_detected           6,908
B02-candidate (validator-narrow flag)    7,668
B03_clean_pass                           1,491
B04_trivial_predictions_no_rejects       1,931
B05_training_data_harvest               18,680
B06_partial_file_read                      525
B07_load_bearing_file_edit                   0
B08_retrain_promote                          0
B09_retrain_hold                             0
```

### Observations

- **`B02-candidate` is worker self-disambiguation.** Contract B02 conflated detection (always allowed) with action (paired commit subject to B07 — retrain-mode-only). Worker correctly downgraded to "diagnosis logged; widening reviewer-gated" by inventing the `-candidate` suffix.
- **B07-B09 not dead code.** Retrain-mode branches blocked by E01 (which is structurally jammed — see §5). Don't amend; fix E01.
- **Cycle 4 unmatched-state pattern.** Worker chose accept-and-move-on via the standing-order backstop. Worked but was generic catch-all rather than designed flow. Codify as B10.

### Recommendations — §3

1. **Add `B10_partial_cycle_recovery`**: accept-as-is partial data; tag affected cycle in cycle_log; do NOT attempt to "complete" prior cycle; cross-cycle analytics filter or annotate partial cycles. Escalate if ≥ 2 partial cycles in last 10.
2. **Split B02 into `B02a_validator_narrow_detected` (measurement-mode, every cycle) + `B02b_validator_narrow_widen` (retrain-mode only, gated on E01a fire).** Removes worker-invented `-candidate` syntax; codifies discipline.
3. **No amendment to B07 / B08 / B09.** Re-audit at next tabletop after retrain mode has run.
4. **Acknowledge B05 as the harvest engine.** 18,680 fires produced the actual training-data dividend.

---

## §4 Halts

### Halt firing across 8 cycles

```
halt                                         fires    notes
H01_disk_space                                  0     free disk 137-143 GB; threshold 20 GB
H02_post_edit_verification_failure              1     cycle df6a4260 — caught launcher-idiom failure
H03_load_bearing_edit_without_audit             0     B07 never fired
H04_concurrent_cycle                            0     trigger-semantics ambiguity surfaced cycle 8
H05_retrain_regression                          0     no retrains attempted
H06_implausible_improvement                     0     gate Δ = 0.0 every cycle (model frozen)
H07_score_model_mismatch                        0     no candidates evaluated
H08_failure_log_corruption                      0     stress-tested cycle 4 — append-only intact
H09_coverage_log_corruption                     0     stress-tested cycle 4 — append-only intact
H10_accidental_binary_push                      0     no binary commits attempted
H11_corrupted_candidate_model                   0     no candidates trained
H12_multi_prediction_pipeline_drift             0     —
H13_holdout_stagnation                          0     counter at 6-7 of 8 (manual halt before fire)
```

### H02 — fired exactly as designed

Cycle 4: cron-firing agent wrapped commands in `nohup ... &` inside `Bash run_in_background:true`. Wrapper bash exited 0 immediately; agent took the exit as completion and launched fresh duplicates. Three concurrent workers wrote to the same logs under one `cycle_id`. H02 caught the silent-success window between "tool returned" and "next verification fired" — exactly the iter-3 review-pr scenario S2 the halt was load-bearing against. Append-only invariant intact.

### H13 — about to fire by design

Manual halt pre-empted cycle 9 trigger. H13 is the contract's natural retirement signal working correctly. Stagnation is the accurate reading of a frozen-model deterministic gate.

### H04 — false-positive observation cycle 8

Worker observed lockfile holding own cycle_id + dead PID, reasoned through preamble lifecycle, did not halt. Trigger semantics need disambiguation.

### What the data does NOT support

H05 / H07 / H11 / H12 / H03 / H10 never exercised. Cannot conclude over-engineered — defer judgement to next tabletop after retrain mode and load-bearing-edit activity have run.

### Recommendations — §4

1. **Specify H13 null-Δ semantics.** Trigger: "Δ < 0.1% absolute over 8 consecutive cycles **with non-null gate measurements**; cycles with null gate (partial, harness-error, deferred) PAUSE the counter without resetting it." Matches cycle 4's conservative reading.
2. **Disambiguate H04 trigger** (paired with §2 lockfile-content simplification): "Lockfile contains a `cycle_id` that differs from the current cycle's id at agent-read time. Lockfile content is `cycle_id` only; presence-with-self-id is normal post-preamble state and does NOT trigger."
3. **No new halts.** HAZOP matrix from pass-1 stands. Speculative additions declined: cycle elapsed-time watchdog (infrastructure scope), harvest growth-rate floor (no signal of need), launcher-idiom enforcement halt (H02 already catches; finetype-dwk fixes underlying bug).
4. **No amendment to H02 / H08 / H09.** Validated by cycle 4 stress test.
5. **Keep H05 / H07 / H11 / H12 as-written.** Re-audit at next tabletop.

---

## §5 Escalations

### Escalation firing across 8 cycles

```
escalation                                    fires
E01_retrain_trigger_threshold_met               7   (every full cycle — degenerate condition)
E02_three_consecutive_hold_retrains             0
E03_load_bearing_path_without_consumer_graph    0
E04_holdout_stagnation                          0   (paired with H13 — manual halt)
E05_implausible_improvement                     0   (Δ=0 mathematically)
E06_corpus_quality_issue_volume                 0   (B06 rate 2.4% < 10% threshold)
```

### E01 — the central contract failure

Fired in 7 of 7 full cycles, identical degenerate condition each time:

> failure_log distinct `(predicted_type, inferred_correct_type)` pairs grew 140 → 164 → 172 → 185 → 197 → 201 across cycles, **all 21,789 entries have `inferred_correct_type = "unknown"`** because the cycle worker has no autonomous type-inference module. The `(predicted, inferred)` pair-structure collapses to a single-axis "20 distinct predicted_types" threshold — trivially crossed by cycle 1.

Worker correctly elected "halt and surface on unmatched state" — autonomous retrain on noise would burn training compute. But E01 fires every cycle to no effect: pure escalation noise.

**Diagnosis: E01 was authored assuming an inference capability the cycle worker doesn't have.** Pass-1 hot-wash item 5 listed observable infrastructure (gate harness, lockfile, append-only logs) but missed the *cognitive* infrastructure — the type-inference module. Numbers (20 pairs / 30%) were tagged "first-pass calibration; re-tune in pass-2"; the calibration is now due, and the diagnosis is **structural**, not numeric.

### E02-E06 — untested

E02 / E03 require retrain-mode and load-bearing-edit activity respectively (both blocked by E01). E04 didn't fire because manual halt landed before H13. E05 cannot fire while model frozen. E06 threshold calibrated correctly (2.8% vs 10%).

### Recommendations — §5

1. **Split E01 into `E01a_retrain_trigger_autonomous` (gated on `inferred_correct_type != "unknown"`) + `E01b_retrain_calibration_human_attended` (calibration backstop; surfaces tabletop ask when N predicted_types accumulate without inference).** Codifies the 7-cycle worker discipline. Numbers (200 distinct types, 20,000 visited files) are calibration suggestions; current state already exceeds both → E01b would fire immediately on contract activation, which is correct.
2. **File parallel engineering bead: "autonomous type-inference module for cycle worker."** P1. This unblocks E01a. Without it, every retrain is forever human-attended via E01b.
3. **No amendment to E02-E06 numeric triggers.** Untested; re-audit next tabletop.
4. **Methodology validation worth noting.** Worker's 7 escalation_notes are remarkably consistent: trigger / reasoning / decision / amendment, every cycle. The "halt and surface, never improvise" standing order earned its keep.

---

## Hot-wash candidates

Methodology-spine questions surfaced by real cycle data. Orbit-memo fodder for `2026-05-02-tabletop-autonomy-contract.md` pass-3.

1. **Presupposed-capability failure mode.** Pass-1 covered observable infrastructure but missed *cognitive* infrastructure (the inference module E01 needed). Pass-3 candidate: when authoring an escalation, trace each variable in the trigger to its data source — directly logged / computable / requires-cycle-worker-capability. Capability-gated variables become engineering deliverables before the escalation can fire.

2. **Escalation-as-noise risk.** E01 fired 7× to no effect; signal density approached zero by cycle 3. Pass-3 candidate: halt rule for stuck escalations — *if same escalation_id fires N consecutive cycles with no resolution, suppress until contract amendment or human ack lands.* Keeps escalations as alarms, not heartbeat.

3. **Worker `B02-candidate` invention.** Two-stage branches (detection always allowed; action retrain-mode-gated) seem like a recurring shape. Pass-3 candidate: branch authoring template explicitly factors `detection_action` from `intervention_action`.

4. **Determinism as load-bearing methodology fact.** Byte-identical 0.0785 across 7 cycles is the strongest evidence the gate harness is correct. Drift would mean harness bug. Pass-3 candidate: when designing a frozen-baseline gate, run twice unchanged after activation; non-byte-identical second run = halt before proceeding. H06 catches *implausible improvement* but not *implausible drift* in a frozen baseline.

5. **H02 + B10 pairing as methodology pattern.** Halts and branches form complementary pairs around shared failure modes. Pass-3 candidate: when adding a halt, ask "what state will exist when this halt fires, and which branch handles it next cycle?" Generic "halt and surface" backstop signals an incomplete halt.

6. **Counter-ambiguity in cross-cycle invariants.** H13's "8 consecutive cycles" admitted three readings under cycle 4's null-Δ slot (pause/reset/advance). Pass-3 candidate: contract must specify behaviour for missing/null/partial cycles at write time. Default presumption: pause.

7. **Worker-quality is downstream of standing-order quality.** Worker produced 7 high-quality cross-referenced escalation notes by following "halt and surface, never improvise." Don't sacrifice that rule even when individual cycles look like they could "just handle it."

---

## Pre-activation status

```
finetype-e6d   ✓ closed   Gate-metric harness (all ACs checked)
finetype-s16   ✓ closed   Content-hash dedup (all ACs checked)
finetype-nms   ✓ closed   Cron lockfile + preamble/postamble (all ACs checked)
finetype-87j   ✓ closed   Append-only logs + integrity tooling (all ACs checked)
finetype-53r   ✓ closed   launchd plist for cron-firing agent (closed in same activation pack)
```

All five closed 2026-05-03 in one activation pack with reason: *"5-bead autonomy-contract activation pack: content-hash dedup, holdout freeze, gate harness, append-only logs + integrity, cron preamble/postamble, launchd plist + install/rotation. 23 tests passing."*

### New P2 engineering beads from cycle escalations

```
finetype-ymi   ○ open   Fix scripts/gittables_gate.sh empty-array crash on bash 3.2
finetype-dwk   ○ open   Cron-firing agent: standardise long-running launcher idiom
```

### New bead recommended from this brief

P1, filed alongside the May-10 contract: *Autonomous type-inference module for cycle worker (unblocks E01a).* Without it, every retrain is forever human-attended.

---

## Pass-2 amendments — summary

Folded into `2026-05-10-gittables-90-percent-roundtrip.yaml`:

- **§1**: ΔT estimate updated `~1h` → `~5-10 min`; pass / fail / error 3-bucket reporting.
- **§2**: Lockfile content = `cycle_id` only (no PID); `harness_dirty` gate added to preamble; `git pull --rebase` added to postamble before push.
- **§3**: B02 split into `B02a_validator_narrow_detected` + `B02b_validator_narrow_widen`. New `B10_partial_cycle_recovery`.
- **§4**: H13 null-Δ semantics specified (pause counter on null gate). H04 trigger disambiguated (cycle_id-based, not lockfile-presence-based).
- **§5**: E01 split into `E01a_retrain_trigger_autonomous` + `E01b_retrain_calibration_human_attended`.

Not folded in (out of contract scope):
- Inference-module engineering work — filed as separate P1 bead.
- Worker-launcher-idiom standardisation — finetype-dwk.
- gate.sh bash 3.2 fix — finetype-ymi.
