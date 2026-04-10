# FineType autoresearch

This is an experiment to have the LLM do its own research on improving FineType's multi-branch column type classifier.

## Setup

To set up a new experiment, work with the user to:

1. **Agree on a run tag**: propose a tag based on today's date (e.g. `apr9`). The branch `autoresearch/<tag>` must not already exist — this is a fresh run.
2. **Create the branch**: `git checkout -b autoresearch/<tag>` from current HEAD.
3. **Read the in-scope files**: The research directory is self-contained. Read these files for full context:
   - `prepare.py` — fixed constants, data prep, feature extraction, dataloaders, evaluation. Do not modify.
   - `train.py` — the file you modify. Model architecture, optimizer, training loop, hyperparameters.
4. **Verify data exists**: Check that the finetype binary is available at `$FINETYPE_BIN` (default: `/workspace/bin/finetype`). If not, tell the human. Check that cached features exist in `$FINETYPE_CACHE` or that HuggingFace/local datasets are accessible.
5. **Initialize results.tsv**: Create `results.tsv` with just the header row. The baseline will be recorded after the first run.
6. **Confirm and go**: Confirm setup looks good.

Once you get confirmation, kick off the experimentation.

## Context

FineType is a semantic type inference engine that classifies tabular data columns into 239 types across 7 domains (identity, geography, datetime, finance, technology, representation, container). The model uses 4 feature branches:

- **Char branch** (960-dim): Character distribution features from column values
- **Embed branch** (512-dim): Model2Vec embedding aggregation over values
- **Stats branch** (27-dim): Column-level statistics (nulls, uniques, lengths, etc.)
- **Header branch** (128-dim): Model2Vec embedding of the column header name

These features are extracted by the `finetype extract-features` Rust binary — you do NOT control feature extraction. Your job is to build the best classifier on top of these fixed 1627-dim feature vectors.

**Known accuracy gaps** (from prior work):
- Numeric code false positives (F5 rule in Sharpen)
- Entity name confusion (company vs. person vs. place)
- Date format ambiguity (US vs. EU date, ISO variants)
- Prior results: v5 multi-branch raw = 154/190, v4+Sharpen = 155/190 on profile eval

**Prior architecture** (Candle baseline you're matching):
- Char: 960 -> 300 -> 300
- Embed: 512 -> 200 -> 200
- Stats: 27 -> 128 -> 64
- Header: LayerNorm(128) -> 128 -> 64
- Merge: BatchNorm(628) -> 500 -> 500
- Head: 500 -> N_CLASSES
- Dropout: 0.35 everywhere

## Experimentation

Each experiment runs on a single GPU. The training script runs for a **fixed time budget of 10 minutes** (wall clock training time, excluding startup/data loading). You launch it simply as: `uv run train.py`.

**What you CAN do:**
- Modify `train.py` — this is the only file you edit. Everything is fair game: model architecture, optimizer, hyperparameters, training loop, batch size, dropout, LR schedule, loss function, regularization, branch dimensions, merge strategy, etc.

**What you CANNOT do:**
- Modify `prepare.py`. It is read-only. It contains the fixed evaluation, data loading, feature extraction, and training constants (time budget, feature dimensions, etc).
- Install new packages or add dependencies. You can only use what's already in `pyproject.toml` (torch, datasets, huggingface-hub).
- Modify the evaluation harness. The `evaluate_accuracy` function in `prepare.py` is the ground truth metric.

**The goal is simple: get the highest val_accuracy.** Since the time budget is fixed at 10 minutes, you don't need to worry about training time — it's always 10 minutes. Everything is fair game: change the architecture, the optimizer, the hyperparameters, the batch size, the model size. The only constraint is that the code runs without crashing and finishes within the time budget.

**VRAM** is a soft constraint. Some increase is acceptable for meaningful accuracy gains, but it should not blow up dramatically.

**Simplicity criterion**: All else being equal, simpler is better. A small improvement that adds ugly complexity is not worth it. Conversely, removing something and getting equal or better results is a great outcome — that's a simplification win. When evaluating whether to keep a change, weigh the complexity cost against the improvement magnitude.

**The first run**: Your very first run should always be to establish the baseline, so you will run the training script as is.

## Output format

Once the script finishes it prints a summary like this:

```
---
val_accuracy:     0.XXXXXX
val_loss:         X.XXXXXX
training_seconds: XXX.X
total_seconds:    XXX.X
peak_vram_mb:     XXXXX.X
num_epochs:       XX
num_params_M:     X.X
n_classes:        XXX
```

You can extract the key metric from the log file:

```
grep "^val_accuracy:" run.log
```

## Logging results

When an experiment is done, log it to `results.tsv` (tab-separated, NOT comma-separated — commas break in descriptions).

The TSV has a header row and 5 columns:

```
commit	val_accuracy	memory_gb	status	description
```

1. git commit hash (short, 7 chars)
2. val_accuracy achieved (e.g. 0.812345) — use 0.000000 for crashes
3. peak memory in GB, round to .1f (e.g. 12.3 — divide peak_vram_mb by 1024) — use 0.0 for crashes
4. status: `keep`, `discard`, or `crash`
5. short text description of what this experiment tried

Example:

```
commit	val_accuracy	memory_gb	status	description
a1b2c3d	0.812345	4.2	keep	baseline
b2c3d4e	0.825678	4.3	keep	increase LR to 0.003 with cosine schedule
c3d4e5f	0.801234	4.2	discard	switch to GeLU activation
d4e5f6g	0.000000	0.0	crash	double model width (OOM)
```

## The experiment loop

The experiment runs on a dedicated branch (e.g. `autoresearch/apr9` or `autoresearch/apr9-gpu0`).

LOOP FOREVER:

1. Look at the git state: the current branch/commit we're on.
2. Tune `train.py` with an experimental idea by directly hacking the code.
3. git commit.
4. Run the experiment: `uv run train.py > run.log 2>&1` (redirect everything — do NOT use tee or let output flood your context).
5. Read out the results: `grep "^val_accuracy:\|^peak_vram_mb:" run.log`
6. If the grep output is empty, the run crashed. Run `tail -n 50 run.log` to read the Python stack trace and attempt a fix. If you can't get things to work after more than a few attempts, give up.
7. Record the results in the tsv (NOTE: do not commit the results.tsv file, leave it untracked by git).
8. If val_accuracy improved (higher), you "advance" the branch, keeping the git commit.
9. If val_accuracy is equal or worse, you git reset back to where you started.

The idea is that you are a completely autonomous researcher trying things out. If they work, keep. If they don't, discard. And you're advancing the branch so that you can iterate.

**Timeout**: Each experiment should take ~10 minutes total (+ a few seconds for startup and eval overhead). If a run exceeds 15 minutes, kill it and treat it as a failure (discard and revert).

**Crashes**: If a run crashes (OOM, or a bug, or etc.), use your judgment: If it's something dumb and easy to fix (e.g. a typo, a missing import), fix it and re-run. If the idea itself is fundamentally broken, just skip it, log "crash" as the status in the tsv, and move on.

**Session restarts**: If your session restarts, reconstruct state from `results.tsv` (which experiments ran and their results) plus `git log` (which commits are on the branch). Resume the loop from where you left off.

## Staged search strategy

Think of experimentation in phases to avoid random thrashing:

**Phase 1 (runs 1-15): Data & basics**
- Run baseline first
- Data mix ratio (synthetic_ratio in prepare_data)
- Learning rate sweep (0.0001, 0.0003, 0.001, 0.003, 0.01)
- Dropout sweep (0.1, 0.2, 0.35, 0.5)
- Batch size (32, 64, 128, 256)
- LR schedule (constant vs. cosine)
- Weight decay (0, 0.01, 0.1)

**Phase 2 (runs 16-30): Architecture**
- Branch hidden dimensions (wider/narrower)
- Merge strategy (concat vs. attention-weighted vs. gated)
- Residual connections in branches
- Additional merge layers
- Different normalization (LayerNorm everywhere vs. BatchNorm)
- Branch-specific dropout rates

**Phase 3 (runs 31+): Radical ideas**
- Label smoothing
- Mixup / CutMix on features
- Multi-task learning (domain prediction + type prediction)
- Focal loss for imbalanced classes
- Feature-level attention across branches
- Knowledge distillation from ensemble
- Gradient accumulation for larger effective batch
- Cyclical learning rates

## FineType-specific insights

When designing experiments, keep in mind:

- **239 types across 7 domains** — this is a fine-grained classification problem with significant class imbalance
- **Header features matter** — the header branch provides strong semantic signal (a column called "email" is probably an email)
- **Stats branch is small but powerful** — 27 dimensions of deterministic statistics (null rate, unique rate, lengths, etc.) are very discriminative
- **The char branch is the largest** (960-dim) — character distribution is the bread-and-butter signal
- **Known confusion pairs**: numeric codes vs. integers, entity names vs. free text, US dates vs. EU dates, various datetime formats
- **The merge strategy is critical** — how you combine 4 very different signal types matters a lot

**NEVER STOP**: Once the experiment loop has begun (after the initial setup), do NOT pause to ask the human if you should continue. Do NOT ask "should I keep going?" or "is this a good stopping point?". The human might be asleep, or gone from a computer and expects you to continue working *indefinitely* until you are manually stopped. You are autonomous. If you run out of ideas, think harder — try combining previous near-misses, try more radical architectural changes, read the code more carefully. The loop runs until the human interrupts you, period.
