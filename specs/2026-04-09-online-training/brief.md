# Brief for Nightingale: RunPod Training Setup for FineType

## Objective

Stand up a cost-effective RunPod training workflow for FineType, a Rust/Candle semantic column-type inference model. Prioritise iteration speed and GPU-hour cost over raw throughput.

## About FineType

FineType is a semantic type inference engine for tabular data columns, written in Rust using the Candle ML framework. It predicts fine-grained column types (dates, currencies, identifiers, categoricals, etc.) from column values and sibling-column context. Key constraints:

- **Model size budget:** 10–50 MB final artefact
- **Architecture direction:** sibling-context attention, Sherlock-style handcrafted feature expansion, hierarchical classification head
- **Training data:** tabular corpora with labelled column types (think Sherlock/Sato/DODUO-style datasets)
- **Inference target:** CPU-friendly, embeddable in data tooling

The small model budget is the single most important fact for infrastructure decisions — FineType does not need H100s, multi-GPU, or distributed training.

## GPU selection on RunPod

Given the 10–50 MB model size and modest batch requirements, the sweet spot on RunPod's marketplace is:

1. **RTX 4090 (24 GB)** — best price/performance for this workload, typically US$0.30–0.50/hr on Community Cloud. Default choice.
2. **RTX A6000 (48 GB)** — if feature-expanded batches or longer sibling-context windows push memory pressure. ~US$0.50–0.80/hr.
3. **A100 40GB** — only if profiling shows the 4090 is genuinely the bottleneck. Unlikely for a sub-50 MB model.

Avoid H100s entirely — they're wasted silicon for this model size.

**Community Cloud vs Secure Cloud:** Community is roughly half the price but runs on third-party hosts. Acceptable for FineType training since the datasets are public-domain tabular corpora. Use Secure Cloud only if a specific run uses sensitive data.

## Deliverables

1. **Training container image**
   - CUDA base image compatible with Candle (e.g. `nvidia/cuda:12.x-devel-ubuntu22.04`)
   - Rust toolchain + FineType training binary built via multi-stage Dockerfile to keep the final image lean
   - Entrypoint reads run config from environment variables: dataset path, output dir, hyperparameters, checkpoint interval
   - Published to a registry RunPod can pull from (GHCR or Docker Hub)

2. **Pod launch script** (`scripts/runpod_train.py`)
   - Uses the RunPod Python SDK to provision a Community Cloud pod with a 4090 by default, configurable via flag
   - Attaches a network volume for datasets and checkpoints
   - Passes run config through environment variables
   - Streams training logs back to the caller
   - Tears the pod down on completion, failure, or Ctrl-C

3. **Checkpoint + resume strategy**
   - Checkpoints written to the mounted network volume at a stable path
   - Training entrypoint detects an existing checkpoint on startup and resumes from it
   - Graceful shutdown on SIGTERM so a preempted Community Cloud pod leaves a clean checkpoint behind
   - Resume-from-checkpoint tested end-to-end as part of the deliverable

4. **Storage layout on the network volume**
   ```
   /workspace/
     datasets/        # persistent, reused across runs
     checkpoints/
       <run_id>/
     artefacts/
       <run_id>/      # final model + metrics
   ```
   Network volume lives in the same RunPod region as the pods to avoid cross-region latency.

5. **Cost tracking doc**
   - Per-run summary: GPU type, wall-clock hours, total cost in AUD, interruptions observed
   - Running tally so we can see cost-per-epoch trends as the architecture evolves

## Best-practice notes for semantic type modelling on rented GPUs

- **Pre-tokenise and pre-featurise datasets once**, store the processed form on the network volume. Sherlock-style feature extraction is CPU-heavy and should not run on every pod startup.
- **Start pods with datasets already on the volume**, not downloaded at boot — boot-time downloads burn paid GPU minutes on I/O.
- **Keep the training loop's checkpoint interval tight** (every few minutes of wall-clock) given Community Cloud preemption risk.
- **Log metrics to a lightweight external sink** (Weights & Biases free tier, or just append-only JSON on the network volume) so an interrupted pod doesn't lose observability.
- **Use short smoke-test runs on a cheap GPU** before committing to longer runs — a 10-minute sanity check on a 4090 is much cheaper than discovering a config bug three hours into an A6000 run.
- **Right-size the pod disk**; container disk on RunPod is billed separately from network volumes and is ephemeral. Keep container disk small, put everything durable on the volume.

## Key RunPod documentation

- Pods overview: https://docs.runpod.io/pods/overview
- Storage types and network volumes: https://docs.runpod.io/pods/storage/types
- `runpodctl` CLI: https://docs.runpod.io/runpodctl/overview
- Python SDK: https://docs.runpod.io/sdks/python/overview
- GraphQL API: https://docs.runpod.io/api-reference/graphql

## Open questions for Hugh

- Default GPU: 4090 or A6000? (Depends on how memory-hungry the sibling-context attention turns out to be.)
- Dataset staging: persistent network volume, or cold-start download from object storage per run?
- Metrics sink: W&B, or keep it local to the network volume for now?

## Out of scope

- RunPod Serverless endpoints (wrong shape for training jobs)
- Multi-GPU or distributed training
- Inference deployment — this brief is training-only
