# Implementation Progress

**Spec:** orbit/specs/2026-04-09-online-training/spec.yaml
**Started:** 2026-04-10

## Hard Constraints
- [x] PyTorch for research; Candle port is a separate follow-up
- [x] Single-file train.py is the only file the agent modifies
- [x] Fixed prepare.py for data loading, feature extraction, and eval harness
- [x] Human-edited program.md for agent instructions
- [x] Feature extraction via finetype binary (from GitHub releases), not reimplemented in Python
- [x] No multi-GPU, no distributed training
- [x] Beelink Mini PC orchestrates (always-on, already running Claude Code)
- [x] Persistent RunPod pod with SSH — port determined once at creation
- [x] Multi-branch architecture (decision 0041) is the starting point
- [x] Context-lean agent pattern: redirect output to files, grep for metrics only
- [x] $40 RunPod budget for first training session — $39.96 actual spend (account balance was backstop)
- [x] RTX 4090 Community Cloud (~$0.40/hr, 24 GB VRAM) — used Secure Cloud at $1.98/hr (community sold out)
- [x] 10 min fixed training time budget per experiment — verified: 944 epochs/run, 40.7 MB peak VRAM
- [x] Training data from HuggingFace: sherlock-annotated + finetype-synthetic — uploaded and verified
- [x] Model budget: 10-50 MB final artefact — 1.2M params, ~5 MB serialized
- [x] Prefer retraining over new rules (decision 0038) — 80 experiments, all architecture/hyperparameter changes

## Acceptance Criteria

### Phase 1: Data preparation
- [x] ac-01: Synthetic data generation — 1,185,000 samples (5000/type × 237 types, 2 types lack generators: password, plain_text)
- [x] ac-02: Uploaded to HuggingFace — https://huggingface.co/datasets/meridian-online/finetype-synthetic (90.2MB, verified download)

### Phase 2: Research codebase
- [x] ac-03: Feature extraction via finetype binary subprocess — research/prepare.py extract_features() matches prepare_multibranch_data.py pattern
- [x] ac-04: Data loading from HuggingFace with configurable mix — research/prepare.py with HF fallback to local files, seeded splits
- [x] ac-05: evaluate_accuracy() function — research/prepare.py evaluate_accuracy(model, dataloader, device)
- [x] ac-06: PyTorch multi-branch model matching Candle architecture — research/train.py MultiBranch class
- [x] ac-07: 10-min fixed time budget training — research/train.py TIME_BUDGET-based loop
- [x] ac-08: Machine-readable summary output — research/train.py autoresearch-format summary block
- [x] ac-09: program.md with staged search instructions — research/program.md with 3 phases + FineType context
- [x] ac-10: profile_eval() confirmation using eval CSVs — research/prepare.py profile_eval() function

### Phase 3: Infrastructure
- [x] ac-11: Pod launch script with self-destruct timer — scripts/runpod_launch.sh
- [x] ac-12: SSH key + config automation — launch script parses ssh info, writes .runpod_ssh.env
- [x] ac-13: Initial pod setup (repo clone, finetype binary, file sync) — launch script initial setup section
- [x] ac-14: Results sync pod → Beelink — scripts/runpod_sync.sh pull
- [x] ac-15: Pod teardown with volume preservation — scripts/runpod_stop.sh
- [x] ac-16: Budget guard ($35 threshold) — scripts/runpod_budget.sh check/update/status
- [x] ac-17: Preemption handling — scripts/runpod_sync.sh run exits code 2 on preemption
- [x] ac-23: Beelink watchdog cron — scripts/runpod_watchdog.sh, installed/removed by launch/stop
- [x] ac-24: Four-layer zombie protection pre-flight gate — all 4 layers implemented

### Phase 4: Experiment loop
- [x] ac-18: Autonomous experiment loop — 80 experiments over ~20h on RTX 4090
- [x] ac-19: results.tsv experiment log — 81 rows (header + 80 experiments), correct TSV format
- [x] ac-20: Keep/discard logic — 6 kept, 74 discarded/crashed, branch advanced correctly
- [x] ac-21: Baseline establishment with calibration — experiment 0: 92.4% val_accuracy
- [x] ac-22: Overnight session gate (30+ experiments) — 80 experiments (2.7× gate)

## Results

**Best: 96.6% val_accuracy** (229/237 correct), up from 92.4% baseline (+4.2pp).

Winning architecture (5 cumulative improvements):
1. Weight decay 0.01 + best-model checkpointing (+1.7pp)
2. Remove dead header branch + AdamW optimizer (+0.4pp)
3. GELU activations replacing ReLU (+1.3pp)
4. LayerNorm on all branch inputs (+0.4pp)
5. Replace merge BatchNorm with LayerNorm (+0.4pp)

Key finding: simple 2-layer MLP branches + GELU + LayerNorm is the sweet spot. Every "clever" technique (focal loss, mixup, SWA, attention fusion, label smoothing, SAM, etc.) made things worse.

## Postmortem

**Zombie protection failure** (decision 0045): 3 of 4 safety layers failed.
- Pod self-destruct: runpodctl on pod had no API credentials
- Beelink watchdog: RunPod API uptimeSeconds always returned 0
- Actual GPU rate $1.98/hr vs reported $0.59/hr (3.4× discrepancy)
- Total spend: $39.96 of $40 budget (account balance was the only working safeguard)

## Notes
- Diagnostics fixed: removed unused `os` import, `labels` dead code, `importlib` dead import, initialized `lr` before loop
- Remaining pyright warnings on `datasets` import are expected (conditional HF import, resolves at runtime on pod)
- Header branch removed during research (synthetic data lacks headers) — re-add during Candle port with real Model2Vec features
