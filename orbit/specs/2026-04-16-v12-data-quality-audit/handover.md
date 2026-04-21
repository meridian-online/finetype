# Handover: v12 Eval Failure Investigation

**From:** Beelink session (2026-04-16)
**To:** Mac session
**Priority:** Must fix before any further v12 work

## What happened

The v12 overnight training completed successfully on the M1:
- FTMB v4 data prep: 131,147 records, 0 errors, 938.6 MB (verified clean)
- Training: 40 epochs, best val_accuracy 90.9% at epoch 38
- Model saved: `models/sherlock-v12/model.safetensors` + `config.json` with type_index_keys

**But the eval step failed silently.** The overnight script's eval step uses
`|| { echo "WARN" }`, so it continued. The `eval/eval_output/` directory still
contained stale results from the v11 overnight (April 12), and those stale
results were copied into `models/sherlock-v12/eval/` and packaged.

Evidence: `eval-pack-sherlock-v12.tar.gz` contains a report dated 2026-04-12
showing 201/227 — but v12 didn't exist until April 16.

## Immediate tasks on the Mac

### 1. Check the overnight log for the eval failure

```bash
grep -A5 "WARN\|FAIL\|error\|Error" results/overnight-v12-retraining.log | head -40
# Also check around the eval step:
grep -B2 -A10 "Step 3/5\|Evaluating sherlock-v12" results/overnight-v12-retraining.log
```

The most likely failure causes:
- **Taxonomy not loaded during inference** — the v12 model needs taxonomy for
  validation features. The eval script may not set up the taxonomy path correctly.
- **Model loading failure** — the 5-branch model needs the validation branch
  weights in safetensors. Check if `from_bytes()` errors on loading.
- **CLI `profile` command** — may not pass taxonomy through to the classifier
  when running under eval. Check if `classify_column()` gets `taxonomy: None`.

### 2. Re-run the eval manually

```bash
cd ~/github/meridian-online/finetype
git pull
FINETYPE_MODEL_DIR=models/sherlock-v12 ./scripts/eval.sh --model models/sherlock-v12
```

If this fails, run with more verbose output:
```bash
FINETYPE_MODEL_DIR=models/sherlock-v12 RUST_LOG=debug ./target/release/finetype profile eval/datasets/iris.csv -o json 2>&1 | head -50
```

### 3. Once eval succeeds, repackage and compare

```bash
# Repackage v12 eval
./scripts/eval_pack.sh models/sherlock-v12

# Compare against v11 (the April 16 re-eval at 204/227 is the correct v11 baseline)
grep "Profile label accuracy" models/sherlock-v12/eval/report.md
grep "Profile label accuracy" models/sherlock-v11/eval/report.md
```

### 4. Update the spec

The data quality audit spec (`orbit/specs/2026-04-16-v12-data-quality-audit/spec.yaml`)
is BLOCKED pending real v12 eval results. Once you have them:
- Update the misclassification count and score in the spec/interview
- Re-derive the fixed/regressed/persistent breakdown
- Reissue the spec with corrected premises
- The spec structure and ACs are sound — only the numbers need fixing

## State of things

```
| Artifact                        | Status          | Location                                      |
|---------------------------------|-----------------|-----------------------------------------------|
| v12 model (trained)             | GOOD            | models/sherlock-v12/ on Mac                    |
| v12 config + type_index_keys    | GOOD            | models/sherlock-v12/config.json                |
| v12 FTMB training data          | GOOD            | output/multibranch-training/v12-blend-70-30.ftmb |
| v12 eval results                | INVALID/MISSING | Need re-run on Mac                             |
| v11 re-eval (April 16)          | GOOD            | eval-pack-sherlock-v11.tar.gz → 204/227        |
| Data quality audit spec         | BLOCKED         | orbit/specs/2026-04-16-v12-data-quality-audit/       |
| Validation branch infra (ac-01→10) | COMPLETE     | Merged in PR #35 + main                       |
| Overnight script                | NEEDS FIX       | eval failure handling in overnight_v12_retraining.sh |
```

## Spec status: validation-branch-v12

Progress: `orbit/specs/2026-04-15-validation-branch-v12/progress.md`

- ac-01 through ac-10: DONE (infra, training, CLI)
- ac-08: DuckDB extension — not started (blocked on shipping v12)
- ac-11 through ac-14: eval gates — BLOCKED on valid eval results
