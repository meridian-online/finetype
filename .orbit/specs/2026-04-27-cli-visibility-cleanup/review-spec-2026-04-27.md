# Spec Review — 2026-04-27

**Date:** 2026-04-27
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-27-cli-visibility-cleanup/spec.yaml v1.0
**Verdict:** REQUEST_CHANGES → addressed in v1.1

---

## Findings & Resolution

```
| # | Severity | Finding                                                       | Resolution in v1.1                                          |
|---|----------|---------------------------------------------------------------|-------------------------------------------------------------|
| 1 | HIGH     | AC-5: named "4 scripts" don't pass --model to finetype CLI    | Re-audited via grep; replaced with correct 9-loci list      |
| 2 | HIGH     | AC-6 + AC-7: verification commands use wrong CLI flag forms   | Rewritten to use positional args (schema, validate)         |
| 3 | MEDIUM   | AC-3: eval-gittables already hidden — verification confused   | AC tightened — verify via grep + unknown-subcommand error   |
| 4 | MEDIUM   | AC-10: `load` subcommand unaccounted in public-surface list   | Added `load` to expected listing (deferred to v0.6.20)      |
| 5 | MEDIUM   | AC-6: schema emitter location misidentified                   | Pinned to main.rs:2744-2760 + 2987-3015; MCP explicitly OOS |
| 6 | LOW      | AC-7: check-only mode behavioural variants underspecified     | Sub-criteria added for --output, --lenient, --append        |
| 7 | LOW      | AC-9: pinned 369/448 fragile under main churn                 | Replaced with "match main HEAD ±0" deterministic invariant  |
| 8 | LOW      | AC-6 downstream: validate's type_confidence becomes NULL      | CHANGELOG note added; existing read-side handles gracefully |
```

## Audit Trail — Script Migration List Correction (Finding 1)

The original spec listed `scripts/distill_batches.sh`, `scripts/eval.sh`,
`scripts/train.sh`, `scripts/llm_label.py` as the 4 scripts to migrate.
None of these pass `--model` to the **finetype** binary:

- `scripts/distill_batches.sh:149` — `--model haiku` → claude CLI.
- `scripts/llm_label.py:51` — Ollama model name.
- `scripts/llm_label.sh` — Ollama wrapper.
- `scripts/train.sh` — has `--model-name` (output dir name), no
  `--model` passed to finetype.
- `scripts/eval.sh:42` — its own `--model` script flag; swaps the
  `models/default` symlink, never invokes `finetype --model`.
- `scripts/overnight_v*.sh` family — all pass `--model "$MODEL_DIR"`
  to `./scripts/eval.sh`, not to finetype directly.

The actual loci that pass `--model PATH` to the **finetype** binary
(grep of `*.sh` and `*.py` for `finetype.*--model` and inspection of
each match):

```
1. eval/profile_eval.sh:115           # primary eval engine
2. scripts/v19_compare.sh:91          # candidate-vs-baseline comparison
3. scripts/sharpen_ablation.sh:66     # raw-model branch
4. scripts/sharpen_ablation.sh:70     # post-sharpen branch
5. scripts/amvg/ac03_confusion.py:51  # amount-variant confusion analysis
6. scripts/amvg/ac07_post_fix.py:62   # amount-variant post-fix verification
7. scripts/rhh/ac03_hit_counts.py:123 # rule-hit-headers AC-3 diagnostic
8. scripts/rhh/ac04_counterfactual.py:199  # rule-hit-headers AC-4 diagnostic
9. scripts/rhh/test_rhh.py:340        # rule-hit-headers test harness
```

The eval-engine path (`eval/profile_eval.sh`) is reachable via
`make eval-profile` and transitively through `scripts/eval.sh` — that's
the load-bearing migration. The `scripts/sharpen_ablation.sh` and
`scripts/v19_compare.sh` paths are recent diagnostic scripts; the rhh/
and amvg/ Python scripts support the most recent diagnostic arcs
(rule-hit-headers and amount-variant) and need migration too.

## Verdict

v1.1 addresses all findings. Implementation may proceed.
