# Spec Review

**Date:** 2026-03-27
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-27-autoresearch-architecture-search/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] FTMB reader underspecified — will fail on real data
**Category:** assumption
**Description:** AC-1 specifies a Python FTMB reader for v2 and v3 formats but provides zero details on exact binary layout, byte offsets, field types, or padding. The Rust implementation is the source of truth — FTMB binary format is not documented anywhere in the repo.
**Evidence:** The spec says "v2 (24-byte header, flat records) and v3 (28-byte header, table-grouped with sibling headers)" but doesn't specify the actual byte-level structure.
**Recommendation:** Add a `FTMB_FORMAT.md` to `research/` documenting the byte-by-byte layout from `crates/finetype-train/src/multi_branch.rs`. Alternatively, implement the FTMB reader in Rust as a small binary and call it from Python via subprocess.

### [CRITICAL] Eval feature extraction has no validation against corruption
**Category:** failure-mode
**Description:** AC-3 describes a one-time script to extract eval features, but there's no checksum, version tag, or validation in eval.ftmb. If the file is stale or corrupt, all experiments report invalid accuracies. AC-5 only checks "results.tsv has ≥4 entries" — a corrupted eval.ftmb would still produce entries, just meaningless ones.
**Evidence:** No pre-flight check in program.md to verify eval.ftmb matches the current Rust binary and taxonomy version.
**Recommendation:** Add metadata header to eval.ftmb (timestamp, finetype version, taxonomy version, feature dims). prepare.py validates these on load. Add pre-flight check to program.md.

### [CRITICAL] Deterministic training assumes no hardware randomness on Metal
**Category:** assumption
**Description:** "Deterministic training (seed 42) — any improvement is real signal" assumes PyTorch on Metal is fully deterministic. Some Metal kernels (batched matmul, scatter ops) are non-deterministic by default.
**Evidence:** PyTorch Metal backend does NOT guarantee determinism with seed alone. Rust v7 was trained with explicit Candle determinism, but PyTorch may not match.
**Recommendation:** Add to AC-2: enforce `torch.use_deterministic_algorithms(True)`. Add pre-flight: run baseline twice and verify identical eval accuracy. If Metal is non-deterministic, fall back to CPU.

### [CRITICAL] Git commit/reset loop has no rollback plan for corrupted train.py
**Category:** failure-mode
**Description:** AC-5 describes keep/discard via git commit/reset, but there's no check that modified train.py compiles or imports before committing. An agent could commit a syntax error.
**Evidence:** No pre-commit validation in spec or program.md.
**Recommendation:** Add to program.md: syntax-check (`python -m py_compile train.py`) and import-check (`python -c 'import train'`) before every commit. Log compile failures as `status: error` in results.tsv.

### [CRITICAL] No definition of "architecture change" — agent could break assumptions
**Category:** missing-requirement
**Description:** "Architecture-only changes" is vague. Agent could change loss function, number of branches, data loading, or eval metric — all technically "not hyperparams" but breaking comparability.
**Evidence:** The spec doesn't enumerate what the agent CANNOT modify.
**Recommendation:** Add explicit allow/deny list to constraints. Allow: branch MLP architectures, fusion operation, trunk structure. Deny: hyperparams, loss function, feature dimensions, number of branches, data loading, eval metric.

### [WARN] 10-minute wall-clock budget has no enforcement mechanism
**Category:** missing-requirement
**Description:** AC-2 says "trains within the 10-minute budget" but doesn't specify a timeout. An agent could launch a 30-minute experiment.
**Evidence:** No timeout mechanism in train.py or program.md.
**Recommendation:** Add `signal.alarm(600)` or equivalent timeout to train.py. Log timeouts as `status: timeout` in results.tsv.

### [WARN] results.tsv format is not validated
**Category:** failure-mode
**Description:** No specification of column order, value format (172/214 vs 0.804 vs 80.4%), or TSV vs CSV. Agent interpretations could produce incompatible files.
**Recommendation:** Add exact format specification to program.md with an example row.

### [WARN] Label-to-index mapping could diverge from taxonomy
**Category:** assumption
**Description:** AC-1 specifies "label-to-index mapping matching the 239-type taxonomy" but doesn't specify how it's generated. Taxonomy evolves (250→239 in v6).
**Recommendation:** Generate mapping at runtime from labels/definitions_*.yaml, not hard-coded.

### [WARN] No exit criteria enforcement for "no improvement after 40 experiments"
**Category:** missing-requirement
**Description:** Exit condition mentions 40+ experiments with no improvement but doesn't specify how the agent detects or acts on this.
**Recommendation:** Add convergence detection to program.md: if 40 consecutive experiments show no improvement, stop and log `status: converged`.

### [INFO] Phase 1 time estimate is optimistic
**Category:** assumption
**Description:** "Phase 1: ~3 hours" underestimates FTMB reader implementation + eval feature extraction + determinism validation.
**Recommendation:** Revise to 5–6 hours, or use Rust subprocess approach to avoid reimplementing binary parsing.

### [INFO] No tiebreaker for equal-accuracy experiments
**Category:** missing-requirement
**Description:** Greedy-keep doesn't handle ties. With integer accuracy resolution, ties are plausible.
**Recommendation:** Add tiebreaker to program.md: fewest trainable parameters, then earliest git log order.

### [INFO] Inference timing methodology unspecified
**Category:** missing-requirement
**Description:** "inference_ms_per_sample" has no specification of warmup passes, averaging method, or variance tolerance.
**Recommendation:** Specify: 5 warmup passes, 10 timed passes, report median.

---

## Honest Assessment

This plan is conceptually solid — adapting autoresearch to FineType is a good idea, the wall-clock budget is a clever fix for inference speed, and the 3-file architecture is clean. But the spec has critical gaps in three areas: (1) FTMB reader is underspecified and will block Phase 1, (2) eval feature extraction has no validation so the loop could produce garbage, and (3) the git rollback loop has no safety checks for corrupted train.py. The biggest risk is Phase 1 taking 6–8 hours instead of 3, especially if Metal determinism proves unreliable. If these are addressed — particularly documenting the FTMB binary format, adding pre-flight checks, and specifying what the agent can/cannot change — this is ready to implement.
