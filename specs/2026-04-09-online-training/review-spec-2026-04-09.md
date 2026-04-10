# Spec Review

**Date:** 2026-04-09
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-04-09-online-training/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] ac-01: `--all-types` flag and Parquet output do not exist

**Category:** assumption
**Description:** ac-01 specifies `finetype generate --all-types --samples 5000 --output synthetic.parquet` and expects a Parquet file with columns `values (list of strings), header (string), label (string)`. The actual CLI has no `--all-types` flag. The existing flags are `--samples N` (per-label count) and `--priority P` (minimum release priority). Output is NDJSON with fields `{text, classification}` -- individual string values, not column-level lists. There is no Parquet output support. The output format is completely different from what the spec assumes.
**Evidence:** `finetype generate` CLI definition (main.rs:94-118) shows flags: `--samples`, `--priority`, `--output` (default: `training.ndjson`), `--taxonomy`, `--seed`, `--localized`. The output loop (main.rs:2126-2133) writes NDJSON with `{text, classification}` per sample -- flat value-level records, not column-level records with `values` lists.
**Recommendation:** ac-01 needs a complete rewrite. Either: (A) extend `finetype generate` to support column-level output with Parquet format (new feature work), or (B) reframe ac-01 around the existing NDJSON value-level output and add a separate `prepare.py` step that assembles value-level samples into column-level training records. Option B is more practical since `prepare_multibranch_data.py` already does this assembly.

---

### [CRITICAL] Feature extraction in Python is a from-scratch rewrite, not a port

**Category:** assumption
**Description:** ac-03 says `prepare.py` implements feature extraction matching the FTMB v3 format. The spec treats this as a straightforward task, but the 4-branch feature extraction (960-dim char distribution, 512-dim Model2Vec embeddings, 27-dim column stats, 128-dim header embeddings) is currently implemented in Rust across three crates (`finetype-model/src/char_distribution.rs`, `finetype-model/src/column_stats.rs`, Model2Vec inference). The existing Python script (`prepare_multibranch_data.py`) calls the Rust `finetype extract-features` binary as a subprocess -- it does NOT implement feature extraction in pure Python. Writing a Python reimplementation of 960-dim char distribution (96 chars x 10 aggregation stats including skewness/kurtosis), 27-dim column stats, and Model2Vec embedding aggregation is a multi-day task with high risk of subtle numerical divergence.
**Evidence:** `prepare_multibranch_data.py` (line 1+) documents its approach: it calls `finetype extract-features` subprocess for the actual extraction. The char distribution implementation (`char_distribution.rs`) is 300+ lines of statistical computation. Model2Vec embedding requires loading a specific model and computing mean/variance/min/max aggregations. No Python reference implementation exists in the repo.
**Recommendation:** Three options, in order of preference: (1) Ship a statically-linked `finetype extract-features` binary to the pod and have `prepare.py` call it via subprocess, exactly like `prepare_multibranch_data.py` already does. This eliminates the reimplementation risk entirely. The binary is ~20MB and has no runtime dependencies. (2) Pre-extract features into FTMB format locally and upload the binary file to HuggingFace. Pod just loads pre-extracted features. (3) Accept the reimplementation risk but add a validation gate: extract features for 100 columns via both Python and Rust, assert max absolute difference < 1e-4 per dimension.

---

### [HIGH] n_classes mismatch: spec says 239, model uses 250

**Category:** constraint-conflict
**Description:** The spec consistently says "239 taxonomy types" and "239-class output," but the Candle `MultiBranchConfig` default is `n_classes: 250`. The CLAUDE.md says "239 definitions across 7 domains" but the model is trained on 250 classes. This discrepancy suggests either padding classes, deprecated types, or a label mapping that the PyTorch reimplementation needs to replicate exactly.
**Evidence:** `multi_branch.rs:90` shows `n_classes: 250` as the default config. The spec says 239 types everywhere.
**Recommendation:** Clarify the actual class count and document the label-to-index mapping. The PyTorch model must use the same mapping or results will be incomparable. The existing training data files encode this mapping -- document it in prepare.py.

---

### [HIGH] Profile eval datasets live on the Beelink, not the pod

**Category:** missing-requirement
**Description:** ac-10 says `profile_eval()` in `prepare.py` "loads the 29 eval datasets and manifest." These datasets live at `eval/datasets/csv/` on the Beelink. The spec says profile eval runs on kept models as a confirmation check (ac-20). But where does profile eval actually run? If on the pod, the 29 CSV files (plus manifest) need to be synced there. If on the Beelink, the PyTorch model checkpoint needs to be synced back first, and the Beelink needs PyTorch + CUDA (or CPU inference, which may be slow).
**Evidence:** `eval/datasets/csv/` contains 20+ CSV files locally. The manifest has 340 lines. ac-13 mentions syncing eval datasets to the pod, but ac-10 puts profile_eval in `prepare.py` which runs on the pod.
**Recommendation:** Clarify: profile eval runs on the pod (sync the eval CSVs as part of ac-13) or on the Beelink after pulling the model back. The pod is simpler since all the data and GPU are there. Add the eval dataset sync explicitly to the rsync include list in ac-13.

---

### [HIGH] Agent autonomy mechanism unspecified

**Category:** missing-requirement
**Description:** The spec says "the agent drives the loop" and the Beelink orchestrates, but there is no specification of HOW a Claude Code agent runs autonomously overnight. Karpathy's autoresearch uses Claude Code with a `program.md` and the agent's built-in loop, but Claude Code sessions have context window limits. After ~100k tokens of experiment logs, the context fills up and the agent stops or degrades. The spec assumes 30-40 experiments over 8 hours -- each experiment generates logs, git diffs, and results that accumulate in context.
**Evidence:** The autoresearch `program.md` says "NEVER STOP" and "The loop runs until the human interrupts you, period." This works for Karpathy's setup with 5-min experiments and simple code, but FineType experiments involve more complex output parsing (val_accuracy, profile_eval, feature dimensions). Context window exhaustion is a real risk after 20+ experiments.
**Recommendation:** Add an explicit AC or constraint about context management: (1) The agent should redirect all training output to files (`> run.log 2>&1`) and only read summary metrics via grep -- already implied but worth making explicit. (2) Consider whether the agent should use `git log` to reconstruct state if context is lost. (3) Add an exit condition for "agent context exhaustion" (Claude Code will naturally stop). (4) Consider splitting into shorter sessions (e.g., 10 experiments per session) with a wrapper script that restarts the agent.

---

### [HIGH] SSH access to RunPod pods is not via runpodctl SSH

**Category:** assumption
**Description:** ac-12 says "SSH key exchange is automated: the launch script configures SSH access." The spec implies this is straightforward, but RunPod's SSH model is specific: pods expose SSH on a random high port via a proxy hostname (e.g., `ssh root@<pod-id>.runpod.io -p 12345`). `runpodctl ssh add-key` registers a key with the RunPod account (applied to all new pods), and `runpodctl ssh info <pod-id>` returns the SSH connection details. This is usable but the spec doesn't account for the dynamic port and hostname, which the rsync/SSH commands in ac-13/ac-14 need.
**Evidence:** `runpodctl ssh info <pod-id>` returns connection details (host + port). `runpodctl ssh add-key` adds a key to the account. `runpodctl pod create` has `--ssh` (default true). SSH works, but the connection string is dynamic per pod and must be parsed from the pod create/info output.
**Recommendation:** Make ac-12 more specific: (1) Pre-register the Beelink's SSH public key via `runpodctl ssh add-key`. (2) After pod creation, parse the SSH host/port from `runpodctl ssh info <pod-id>` or `runpodctl pod get <pod-id>`. (3) Write a helper that constructs the SSH/rsync command with the correct host:port. This is automatable but needs explicit handling.

---

### [MEDIUM] Budget guard has no API for pod start time

**Category:** assumption
**Description:** ac-16 says the budget guard tracks "wall-clock hours * hourly rate." The spec assumes the orchestration script knows when the pod started and the exact hourly rate. `runpodctl pod get <pod-id>` likely returns this information, but the spec doesn't specify how to query it. If the pod is preempted and restarted (ac-17), the budget must accumulate across multiple pod lifetimes.
**Evidence:** `runpodctl pod get` returns pod details (likely including `costPerHr` and creation time). The budget guard needs to track cumulative time across preemption events, not just current pod uptime.
**Recommendation:** Specify the budget tracking implementation: (1) Record pod start timestamps locally. (2) On each experiment completion, compute elapsed time and update cumulative spend. (3) On preemption, record the terminated pod's runtime before provisioning a new one. A simple local file (`budget.json`) with start times and cumulative spend suffices.

---

### [MEDIUM] 10-minute training budget may be insufficient for convergence signal on 4090

**Category:** assumption
**Description:** ac-07 specifies a 10-minute fixed training time budget. The interview says this gives "balanced convergence vs throughput." But the multi-branch model with 239+ classes and ~1.2M training samples (5000 * 239) is a non-trivial training job. With 10 minutes on a 4090, the model may only complete 2-5 epochs depending on batch size. For architecture comparison, you need enough epochs to distinguish "this architecture converges faster" from "this architecture just needs more time."
**Evidence:** The Candle training runs (v4, v5) ran for hours to reach convergence. The autoresearch 5-min budget works for GPT-2 small (50M params, simple loss landscape). The multi-branch model is smaller but has 239 classes and heterogeneous feature branches.
**Recommendation:** The 10-minute budget is a reasonable starting point, but add an early experiment specifically to validate it: "Experiment 0: run baseline for 10 min, then 30 min, compare val_accuracy. If the 30-min run is >5% better, increase the time budget." Include this in the program.md as a calibration step.

---

### [MEDIUM] Preemption handling (ac-17) is under-specified

**Category:** missing-requirement
**Description:** ac-17 says the orchestration script "detects the disconnection, waits for a configurable backoff, provisions a new pod, and resumes." Detection is the hard part: SSH commands will fail with connection errors, but distinguishing "pod was preempted" from "network hiccup" requires checking `runpodctl pod get <pod-id>` status. The new pod gets a different IP/port. The network volume preserves data, but any in-flight training run's partial results are lost (no checkpoint saving during the 10-min run).
**Evidence:** Community Cloud pods can be preempted at any time. The spec doesn't mention intra-experiment checkpointing (the 10-min training run has no intermediate saves). If preempted mid-experiment, that experiment is simply lost.
**Recommendation:** (1) Accept that preempted experiments are lost (10 min of GPU time, ~$0.07). (2) Detection: after SSH failure, run `runpodctl pod get <pod-id>` -- if status is TERMINATED/EXITED, it was preempted. If still RUNNING, retry SSH. (3) On preemption: log the crashed experiment in results.tsv, provision a new pod, resume from last completed experiment. (4) Consider adding a mid-training checkpoint at 5 min to halve the worst-case loss.

---

### [MEDIUM] `finetype generate` produces value-level data, not column-level training data

**Category:** gap-analysis
**Description:** The spec's data pipeline has a gap between what `finetype generate` produces and what the multi-branch model consumes. `finetype generate` produces individual `{text, classification}` NDJSON records (one value per line). The multi-branch model consumes column-level feature vectors (960+512+27+128 dims extracted from a list of values). There is no specification of who assembles values into columns, extracts features, and writes the FTMB binary format.
**Evidence:** `finetype generate` output: NDJSON with `{text, classification}`. FTMB training format: binary file with per-column feature vectors. The existing `prepare_multibranch_data.py` does this assembly + extraction by calling the Rust binary. The spec's `prepare.py` is described as doing feature extraction (ac-03) but the gap between raw values and column-level features is substantial.
**Recommendation:** Either: (A) `prepare.py` must also handle grouping values into synthetic columns (sampling N values per type, creating column-level records), or (B) extend `finetype generate` to output column-level records. Option A is simpler and matches the existing `prepare_multibranch_data.py` pattern.

---

### [MEDIUM] rsync requires SSH config that matches RunPod's proxy model

**Category:** assumption
**Description:** ac-13 and ac-14 describe rsync for code/result sync. Standard rsync commands assume `rsync -az ./files/ user@host:/path/`. RunPod pods use `root@<proxy-host> -p <random-port>`, requiring `rsync -e "ssh -p <port>"`. This is automatable but easy to get wrong, and the port changes on every pod creation.
**Evidence:** RunPod SSH is proxied through `<pod-id>.runpod.io` on a random port. `runpodctl ssh info` provides the connection details.
**Recommendation:** The launch script should write a temporary SSH config entry (or export an `RSYNC_RSH` variable) so that subsequent rsync commands "just work." Template: `rsync -az -e "ssh -p $POD_PORT -o StrictHostKeyChecking=no" ./research/ root@$POD_HOST:/workspace/research/`.

---

### [MEDIUM] `runpodctl send/receive` uses croc, not rsync

**Category:** assumption
**Description:** The spec assumes SSH + rsync for file transfer. RunPod's `runpodctl send/receive` uses [croc](https://github.com/schollz/croc) -- a peer-to-peer file transfer tool with a code phrase. This is an alternative but works differently from rsync (no incremental sync, no directory watching, requires running `receive` on the other end). The spec should explicitly choose SSH+rsync (which works but needs port config) vs runpodctl send/receive (simpler but no incremental sync).
**Evidence:** `runpodctl send --help` shows croc-based transfer. `runpodctl receive --help` shows code-based receiving.
**Recommendation:** Stick with SSH + rsync as the spec intends. It supports incremental sync which matters for pulling results after each experiment. Just document the SSH port configuration requirement.

---

### [LOW] Experiment commit tracking may be noisy

**Category:** missing-requirement
**Description:** ac-19 says the commit column is "the short git hash of the train.py modification." ac-20 says kept experiments are git-committed. But the agent is modifying train.py on the Beelink, not on the pod. If the agent commits before syncing to the pod, the commit exists even if training crashes. If it commits after, the timing is more complex. The original autoresearch pattern commits before running, which is clean.
**Evidence:** Karpathy's pattern: modify -> commit -> run -> keep/revert. The spec follows this but doesn't explicitly state whether the commit happens before or after the training run.
**Recommendation:** Make explicit: commit train.py before syncing to pod (matching autoresearch pattern). On discard, `git reset --hard HEAD~1`. On crash, same revert. This is implied but worth stating in ac-18 or ac-20.

---

### [LOW] Exit condition "10 consecutive experiments without improvement" may trigger too early

**Category:** constraint-conflict
**Description:** With staged search (Phase 1: known gaps, Phase 2: architecture, Phase 3: radical), it's plausible that Phase 1 exhausts known-good improvements, then Phase 2's first 10 experiments are all architectural misses before finding a better merge strategy. The 10-consecutive-discard exit would kill the run during a necessary exploration phase.
**Evidence:** The exit conditions list "10 consecutive experiments without improvement (search exhausted)."
**Recommendation:** Either increase to 15-20, or make phase-aware: reset the counter when transitioning between phases. Alternatively, don't make this an automatic exit -- just flag it in the log and let the agent decide whether to continue exploring.

---

### [LOW] No W&B or external metrics sink specified

**Category:** missing-requirement
**Description:** The interview lists "Whether the agent should use W&B or keep metrics local" as an open question. The spec doesn't resolve it. For overnight runs, external observability (even a simple webhook notification) helps the human know if the loop is still running or has crashed.
**Evidence:** Interview open question, not resolved in spec.
**Recommendation:** Keep it simple: metrics stay in results.tsv on the Beelink (already accessible). Add a lightweight notification mechanism: write a one-liner that posts to a webhook (Slack/Discord) when a new "keep" result is found or when the loop exits. Not essential but high value for overnight peace of mind.

---

## Honest Assessment

This spec captures an ambitious and well-structured plan that adapts the autoresearch pattern to FineType's specific needs. The overall architecture -- Beelink orchestrates, RunPod pod trains, SSH+rsync connects them -- is sound. The staged search strategy and two-tier metric design are thoughtful. However, the spec has two critical gaps that would cause implementation to stall immediately: (1) the `finetype generate` command does not support the `--all-types` flag, does not produce Parquet output, and generates value-level data rather than column-level training records; (2) the feature extraction pipeline cannot be trivially reimplemented in Python -- the existing approach calls the Rust binary as a subprocess, and the spec should embrace this rather than assume a Python rewrite. The n_classes mismatch (239 vs 250) also needs resolution before training produces usable models. The infrastructure pieces (RunPod provisioning, SSH, rsync, budget guard, preemption) are all achievable but each has implementation details the spec hand-waves over -- individually minor, collectively they represent 2-3 days of scripting work that should be explicitly scoped. I recommend resolving the two critical findings and the n_classes mismatch before implementation begins.
