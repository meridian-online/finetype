---
description: Evaluate a FineType model against the full eval suite (profile + actionability + report). Saves the current default-model symlink, swaps in the model under test, runs the suite, and restores the symlink on exit.
when_to_use: User says "eval", "evaluate", "score the model", "run the eval suite", or names a model dir and asks how it stacks up. Also use after `/train` completes when the author wants accuracy numbers before deciding on `models/default`.
argument-hint: "[model-path]"
arguments: model_path
allowed-tools: Bash, Read
---

# Evaluate a FineType Model

Run from the finetype repo root (`~/github/meridian-online/finetype/`).

## Quick Start

```bash
# Evaluate current default model
./scripts/eval.sh

# Evaluate a specific model
./scripts/eval.sh --model models/char-cnn-v13
```

## What It Does

1. **Profile eval** — Tests against 21 datasets (120 columns). Measures label and domain accuracy.
2. **Actionability eval** — Tests datetime format_string parse rates across predicted types.
3. **Report generation** — Produces unified markdown dashboard.

## Key Metrics

- **Label accuracy** — Exact type match (e.g., 95.7% = 111/116 columns correct)
- **Domain accuracy** — Domain-level match (e.g., 98.3% = 114/116)
- **Actionability** — Format string parse rate for datetime types (e.g., 96.2%)

## How Model Swap Works

When `--model` is specified, the script:
1. Saves the current `models/default` symlink target
2. Re-points it to the specified model
3. Runs the full eval suite
4. Restores the original symlink (even on error, via trap)

## Output Files

```
eval/eval_output/profile_results.csv        # Per-column predictions
eval/eval_output/actionability_results.csv  # Per-value parse results
eval/eval_output/report.md                  # Unified dashboard
```

## Interpreting Results

- Compare against baseline in CLAUDE.md (currently 95.7% label, 98.3% domain, 96.2% actionability)
- Regressions of >1% label accuracy warrant investigation before shipping
- Check `report.md` for per-type precision breakdown
