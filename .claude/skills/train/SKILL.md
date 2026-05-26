---
description: Kick off a FineType Sense-stage multi-branch retrain (overnight `scripts/overnight_v*_*.sh` pipeline) inside a named tmux session so the author can attach and follow along. Falls back to legacy CharCNN training when the user explicitly asks for it.
when_to_use: User says "train", "retrain", "kick off vNN training", or names an overnight training script. Also use when promoting a planned v-number from spec to running pipeline.
argument-hint: "[script-path]"
arguments: script_path
allowed-tools: Bash, Read
---

# Train a FineType model

Run from the finetype repo root (`/Users/hugh/github/meridian-online/finetype`).

Two training paths live in this repo:

- **Sense multi-branch (current).** The v0.6.19 default Sense-stage
  model is multi-branch; every retrain since v19 uses the
  `scripts/overnight_v*_*.sh` family. Multi-hour, multi-seed, kicked
  off in a tmux session so the author can attach and follow along.
- **CharCNN (legacy).** The original Sense implementation. Still in
  code; rarely retrained. Skip to the *Legacy CharCNN path* section
  at the bottom if that's what you need.

## Sense multi-branch — the agent's procedure

1. **Resolve the script.** If the user passed a path
   (`/train scripts/overnight_v22_boundary.sh`), use it. Otherwise
   list recent training scripts by mtime and confirm the pick in one
   line:
   ```bash
   ls -t scripts/overnight_v*_*.sh | head -5
   ```

2. **Derive the tmux session name** from the script's version tag:
   `scripts/overnight_v22_boundary.sh` → `sense-train-v22`. Stable
   naming lets the author re-attach predictably across Claude
   sessions.

3. **Pre-flight checks** (fast — fail before spawning):
   - Repo root reached (`git rev-parse --show-toplevel` matches CWD).
   - `tmux` and `cargo` on path.
   - Training script is executable.
   - `results/` exists (`mkdir -p results`).
   - No process holds the expected output dir under `models/`.

4. **Spawn or reuse the tmux session.** The agent NEVER attaches —
   only the author does. The agent creates the session detached and
   sends the command:
   ```bash
   SESSION=sense-train-v22
   SCRIPT=scripts/overnight_v22_boundary.sh
   LOG=results/$(basename "$SCRIPT" .sh).log

   if tmux has-session -t "$SESSION" 2>/dev/null; then
     tmux new-window -t "$SESSION" -n "$(date +%H%M)"
   else
     tmux new-session -d -s "$SESSION" -n "$(date +%H%M)"
   fi
   tmux send-keys -t "$SESSION" "cd $(pwd) && ./$SCRIPT 2>&1 | tee -a $LOG" C-m
   ```

5. **Report the attach command** to the author in one line:
   ```
   tmux attach -t sense-train-v22    # Ctrl-b d to detach
   ```
   Also surface the log path in case they prefer `tail -f`.

6. **Do NOT poll.** Overnight runs are multi-hour. When the author
   asks for an update later, read the log tail and report the last
   gate / epoch / status line — don't sleep-loop.

### tmux conventions

- **Session per generation** — `sense-train-v22`, `sense-train-v23`.
  Multiple seeds in one generation share the session.
- **Window per kickoff** — named `HHMM`, so re-runs accumulate
  scrollable windows.
- **Detached create** — `tmux new-session -d` so the agent's shell
  returns immediately. The script writes its own log; the tmux
  window is for live viewing.
- **Log path mirrors script name** — `results/<script-basename>.log`
  matches the overnight scripts' own convention; the outer `tee`
  above just hardens against a script that forgets to redirect.

### What runs inside the session

`overnight_v*_*.sh` encapsulates the full Sense pipeline: mining →
blend → audit gates → training (N seeds) → cherry-pick. The author
will see the same structural sequence every generation.

### Common follow-ups

- **Cherry-pick + symlink swap** — the script prints the best seed
  on completion. The author makes the `ln -sfn <best> models/default`
  call.
- **Corpus pass** — after swap, run the GitTables corpus pass (~6–7 h)
  to a fresh `output/corpus-pass-vNN/` directory. See the closing
  block of the training script for the exact command.
- **Cell deltas** — compute v19/.../vNN deltas via the
  `compute_v22_cell_deltas.py` family (write a vNN-aware copy if the
  generation isn't covered yet).

### If something goes wrong

- **Session exists but is dead** — `tmux kill-session -t <name>`
  then re-create. Don't try to revive.
- **Pre-flight gate fails** — read the log, surface the exact gate
  name (e.g. `FAIL: dataset_verify`). Don't retry blindly.
- **Author wants to abort** — `tmux send-keys -t <session> C-c`,
  then let them inspect. The agent doesn't auto-kill.

## Legacy CharCNN path

Only use this when the user explicitly asks for CharCNN; the modern
Sense pipeline above is the default.

```bash
./scripts/train.sh --samples 1000 --size small --epochs 10
./scripts/train.sh --samples 5000 --size large --epochs 15 --seed 42
```

| Preset | embed_dim | num_filters | hidden_dim |
|--------|-----------|-------------|------------|
| small  | 32        | 64          | 128        |
| medium | 64        | 128         | 256        |
| large  | 128       | 256         | 512        |

Hardware auto-detect: macOS → Metal, Linux+NVIDIA → CUDA, else CPU.
Output: `models/char-cnn-vN/` (auto-incremented), log alongside.

All flags:
```
--samples N         Samples per type (default: 1000)
--size PRESET       small|medium|large (default: small)
--epochs N          Training epochs (default: 10)
--seed N            Random seed (default: 42)
--embed-dim N       Override embedding dimension
--num-filters N     Override CNN filters
--hidden-dim N      Override hidden layer dimension
--model-name NAME   Output dir name (default: auto char-cnn-vN)
--data FILE         Use existing NDJSON (skip generation)
```

After training: `./scripts/eval.sh --model models/char-cnn-vN`
and `./scripts/package.sh models/char-cnn-vN`.
