# Dataset provenance registry

Every dataset that contributes to FineType training or evaluation is
recorded in `eval/datasets/sources.yaml` with verifiable integrity. This
folder holds the contract (this README) and the per-source snapshot JSONs
referenced from there. Per spec `2026-05-24-dataset-provenance-registry`.

## What the registry tracks

Each entry in `eval/datasets/sources.yaml` records, for one dataset:

| Field | Purpose | Examples |
|-------|---------|----------|
| `source_url` | Canonical key — URL, `repo://…` URI, or `dataset://<name>` sentinel for external paths | `https://download.geonames.org/export/dump/`, `repo://output/distillation-v3/sherlock_distilled.csv.gz`, `dataset://gittables` |
| `role` | Leakage role (per choice 0056). One of `train`, `eval`, `validate`, `both-forbidden` | `train` |
| `licence` | SPDX identifier where possible | `CC-BY-4.0`, `Unicode-DFS-2016`, `PDDL-1.0` |
| `attribution` | Human-readable credit + usage note | "GeoNames gazetteer … CC-BY 4.0" |
| `fetched_date` | When the data was first registered | `2026-05-24` |
| `datasets` | List of dataset names this entry contributes to | `[geonames]` |
| `local_path` | Where the bytes live — `repo://…` for in-repo, absolute path for external | `$HOME/datasets/geonames` |
| `snapshot` | Pointer to the JSON snapshot under `eval/datasets/snapshots/` | `eval/datasets/snapshots/geonames-2026-05-24.json` |
| `dataset_version` | Stable identifier (date, release tag, or content hash) | `46.0.0`, `2026-05-24` |

The first six fields are the existing leakage manifest from choice 0056.
The last three are the integrity extension added by spec
`2026-05-24-dataset-provenance-registry`.

## How to add a dataset

1. Place the data on disk. In-repo data (small, under git) lives under
   the appropriate `data/` or `output/` subdirectory; large external data
   lives under `$HOME/datasets/<name>/`.
2. Run the register script:

   ```bash
   # In-repo single-file dataset
   python3 scripts/dataset_register.py <name> <path/to/file> \
     --role train --licence <SPDX> \
     --attribution "<credit + usage note>"

   # External directory tree
   python3 scripts/dataset_register.py <name> "$HOME/datasets/<name>" \
     --role train --licence CC-BY-4.0 \
     --attribution "<credit + usage note>"

   # Very-large path-list dataset (don't hash every file)
   python3 scripts/dataset_register.py <name> "$HOME/datasets/<name>" \
     --mode index-only \
     --index-file path/to/index.txt \
     --role train --licence <SPDX> \
     --attribution "<credit + usage note>"
   ```

3. The script writes `eval/datasets/snapshots/<name>-<version>.json` and
   adds or updates the corresponding `sources.yaml` entry.
4. Commit both the snapshot JSON and the updated `sources.yaml`.

## How to verify integrity

Before any long-running training or eval pipeline, run the verify script:

```bash
python3 scripts/dataset_verify.py sherlock cldr gittables geonames
```

- Exit `0` — every file matches the snapshot.
- Exit `2` — drift detected (missing file, extra file, or hash mismatch).
  The drift report names the affected files.
- Exit `3` — configuration error (snapshot or sources.yaml entry missing).

Launcher scripts (`scripts/overnight_v21_geography.sh`, etc.) call
`dataset_verify.py` as a pre-flight check; a non-zero exit aborts the run.

## Snapshot modes

**`full`** — every file under `local_path` is hashed (SHA256 + size).
Use for datasets up to a few thousand files. Verify re-hashes everything
and is exhaustive.

**`index-only`** — for very-large path-list datasets (GitTables with 1M+
parquets). The snapshot hashes only:
- the index file (the canonical path manifest) itself,
- a deterministic 1000-file sample drawn from the index by stride.

Verify checks the index file's hash + re-hashes the same 1000-file
sample. This catches: index drift, sample-file deletion, sample-file
modification. It does NOT catch silent modification of files outside the
sample — for that, periodic full re-snapshots are the maintainer's job.

The two modes are not interchangeable; a snapshot records its
`snapshot_mode` and verify dispatches accordingly.

## What does NOT belong in the registry

- **Personal sandbox datasets** that don't drive any committed model or
  eval run. If the data isn't referenced by a launcher script or a spec,
  it doesn't need a snapshot.
- **Derived artefacts** produced by FineType itself (e.g.
  `output/distillation-v20/sherlock_distilled_with_geography.csv.gz`).
  These are reproducible from the registered inputs + a script invocation
  — the inputs and the script are what need provenance, not the output.
- **Private external datasets** belonging to other projects, even if
  they're stored locally. The registry covers data that contributes to
  the FineType codebase; downstream uses of FineType keep their own
  registries.

## Backfilled datasets (as of 2026-05-24)

| Name | Mode | Where |
|------|------|-------|
| `sherlock` | full | `repo://output/distillation-v3/sherlock_distilled.csv.gz` |
| `cldr` | full | `repo://data/cldr/` |
| `gittables` | index-only | `/Users/hugh/datasets/gittables/` (index at `eval/gittables/corpus_paths.txt`) |

Smaller per-file eval datasets (the 35+ entries under
`repo://eval/datasets/csv/`) keep their leakage-role fields but do not
yet have snapshots — they can be backfilled iteratively as routine
maintenance touches them. The schema permits omitting the three
integrity fields for entries not yet backfilled; the verify script
skips entries without a `snapshot` field.

## See also

- `choice 0056 (train-eval-leakage-prevention)` — the role
  manifest contract this extends.
- `choice 0090 (dataset-provenance-pattern)` — architectural
  record for the integrity extension and the index-only mode.
- `spec 2026-05-24-dataset-provenance-registry` — the
  spec that established this pattern.
