---
name: release
description: >
  Release a new FineType model version — pre-flight checks, HuggingFace upload,
  symlink update, card/CLAUDE.md refresh, and commit.
user-invocable: true
---

# /release

Ship a trained model to HuggingFace so the DuckDB extension, CI, and end users pick it up.

## Usage

```
/release <model_dir>
```

Where `<model_dir>` is a path under `models/` (e.g. `models/sherlock-v11`).

## Instructions

### 1. Pre-flight Checks

Run these checks and **stop on any failure**:

1. **Model artifacts exist** — verify `<model_dir>` contains:
   - `model.safetensors` (required — the weights)
   - `config.json` (required — architecture config)
   - `label_map.json` (required — class index mapping)
   - `results.json` (optional — training metrics)

2. **Tests pass** — `cargo test -p finetype-model` must pass with zero failures.

3. **Zero warnings** — `cargo build --workspace 2>&1 | grep "^warning\[" | head -1` must be empty (build.rs info lines are fine).

4. **Config sanity** — read `config.json` and print key fields: `n_classes`, `activation`, `head_type`. Confirm `n_classes` matches the taxonomy count (`239` as of current taxonomy).

5. **Current default** — read `models/default` symlink and show what's being replaced.

Present the pre-flight summary and wait for confirmation before proceeding.

### 2. Upload to HuggingFace

Upload the three required files to `meridian-online/finetype-model`:

```python
from huggingface_hub import HfApi
api = HfApi()

for file in ["model.safetensors", "config.json", "label_map.json"]:
    api.upload_file(
        path_or_fileobj=f"{model_dir}/{file}",
        path_in_repo=f"{model_name}/{file}",
        repo_id="meridian-online/finetype-model",
        repo_type="model",
        commit_message=f"Add {model_name}/{file}"
    )
```

Where `model_name` is the directory basename (e.g. `sherlock-v11`).

After upload, verify the files are accessible:
```bash
curl -sfI "https://huggingface.co/meridian-online/finetype-model/resolve/main/{model_name}/model.safetensors"
```

### 3. Update Default Symlink

```bash
ln -sfn {model_name} models/default
```

### 4. Update Card & CLAUDE.md

If eval results exist (check `<model_dir>/eval/` or `eval/eval_output/profile_results.json`):

1. **Card 0002** — update the `goal:` field with the new eval baseline
2. **CLAUDE.md** — update the `Default model:` line and `Profile eval:` numbers in both Current State and Evaluation infrastructure sections

If no eval results exist, note this and skip — the eval must run before CLAUDE.md can be updated.

### 5. Commit & Push

Stage and commit:
```bash
git add models/default cards/0002-semantic-type-detection.yaml CLAUDE.md
git commit -m "Publish {model_name} to HuggingFace, update default model"
git push
```

### 6. Verify CI Download

The CI download script (`.github/scripts/download-model.sh`) reads `models/default` to determine which model to fetch. After pushing, CI will automatically download the new model on next run.

Optionally, dry-run the download script locally:
```bash
.github/scripts/download-model.sh
```

### 7. Summary

Report:
```
Released {model_name} to HuggingFace.

  HF repo:  meridian-online/finetype-model
  Files:    {model_name}/model.safetensors, config.json, label_map.json
  Default:  models/default → {model_name}
  Eval:     {score}/{total} ({pct}% label)

  DuckDB extension and CI will download automatically.
```

## Rollback

If a release needs to be reverted:

1. Revert the `models/default` symlink to the previous model
2. Commit and push — CI and DuckDB extension will revert on next download
3. HuggingFace files can stay (old versions are still available by path)

The previous model's files remain in the HF repo — no data is deleted.
