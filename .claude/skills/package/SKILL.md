---
name: package
description: Package a trained FineType model for distribution (tar.gz + SHA256)
user-invocable: true
---

# Package a FineType Model

Run from the finetype repo root (`~/github/noon-org/finetype/`).

## Quick Start

```bash
./scripts/package.sh models/char-cnn-v13
./scripts/package.sh models/char-cnn-v13 --output dist/
```

## What It Does

1. Validates the model directory has required files
2. Creates `finetype-<model-name>.tar.gz` containing:
   - `model.safetensors` — trained weights
   - `config.yaml` — model configuration
   - `labels.json` — label index
   - `manifest.json` — training provenance (if present)
3. Prints file size and SHA256 checksum

## After Packaging

### Upload to HuggingFace

```bash
# Upload individual model files (what CI expects)
huggingface-cli upload hughcameron/finetype models/char-cnn-v13/ char-cnn-v13/

# Or upload the archive
huggingface-cli upload hughcameron/finetype finetype-char-cnn-v13.tar.gz
```

### Update Homebrew Tap

The SHA256 from the package output is needed when updating the Homebrew formula. The release CI workflow handles this automatically for tagged releases.

### Update models/default Symlink

```bash
cd models && rm default && ln -s char-cnn-v13 default
```

## Typical Full Workflow

```bash
./scripts/train.sh --samples 5000 --size large --epochs 15
./scripts/eval.sh --model models/char-cnn-v13
# Check eval results, then:
./scripts/package.sh models/char-cnn-v13
# Upload to HuggingFace, update symlink, tag release
```
