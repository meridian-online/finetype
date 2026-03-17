# Phase 4: LLM Distillation Runbook

**Task:** NNFT-269
**Hardware:** M1 MacBook with Ollama
**Model:** Qwen3 32B (Q4, ~20GB)
**Goal:** Label 10K+ real-world columns with FineType's 250-type taxonomy

---

## 1. Setup (one-time)

### Install Ollama

```bash
brew install ollama
```

### Pull the model (~20GB download)

```bash
ollama pull qwen3:32b
```

### Verify it works

```bash
ollama run qwen3:32b "What type of data is this column? Header: 'zip_code'. Values: ['90210', '10001', '60614']. Answer with just the type name." --nowordwrap
```

### Check your RAM

Model RAM requirements — pick the right size for your machine:

- **16GB M1:** Use `qwen3:8b` (~5GB) — runs comfortably in the background with ~11GB free
- **32GB M1:** Use `qwen3:14b` (~9GB) or `qwen3:32b` (~20GB)
- **⚠️ qwen3:32b will NOT work on 16GB** — causes immediate swap thrashing

### Clone finetype repo (if not already on MacBook)

```bash
git clone https://github.com/hughcameron/finetype.git
cd finetype
```

---

## 2. Get Real-World Data

### Option A: GitTables (recommended — largest, most diverse)

Download the GitTables benchmark subset (1,101 tables, manageable size):

```bash
# Download from Zenodo
mkdir -p data/gittables
cd data/gittables
# Full dataset: https://zenodo.org/records/6517052
# Start with the benchmark subset (~500MB)
wget "https://zenodo.org/records/6517052/files/gittables_benchmark.zip"
unzip gittables_benchmark.zip
cd ../..
```

### Option B: Kaggle datasets (easy, diverse)

```bash
pip install kaggle
# Download a few popular datasets
kaggle datasets download -d datasnaek/youtube-new -p data/kaggle/
kaggle datasets download -d zynicide/wine-reviews -p data/kaggle/
kaggle datasets download -d shivamb/netflix-shows -p data/kaggle/
# Unzip each
```

### Option C: Your own CSVs

Any directory of CSV files works. The script processes all `.csv` files it finds.

---

## 3. Run the Labelling Script

```bash
# From the finetype repo root
./scripts/llm_label.sh data/gittables/ output/llm_labels.csv

# Or for a specific directory of CSVs:
./scripts/llm_label.sh /path/to/csvs/ output/llm_labels.csv

# Options:
#   --model qwen3:14b     # Use smaller model (faster, less RAM)
#   --max-columns 10000   # Stop after N columns
#   --max-values 15       # Sample N values per column (default: 10)
#   --skip-finetype       # Skip FineType comparison (if not installed on MacBook)
```

### What the script does

For each CSV file found:
1. Reads headers and samples 10 values per column
2. Prompts Qwen3 with the column data + FineType's 250-type taxonomy
3. Validates the response is a valid FineType type label
4. Optionally runs `finetype profile` for comparison
5. Appends results to the output CSV

### Expected throughput

- **Qwen3 32B on M1 Pro/Max:** ~1-3 columns/second (~3,000-10,000 columns/hour)
- **Qwen3 14B on M1:** ~3-5 columns/second (~10,000-18,000 columns/hour)

### Output format

```csv
source_file,column_name,sample_values,llm_label,llm_confidence,finetype_label,agreement
data/sales.csv,customer_email,"['john@example.com','jane@co.uk']",identity.person.email,high,identity.person.email,yes
data/sales.csv,order_date,"['2024-01-15','2024-02-28']",datetime.date.iso,high,datetime.date.iso,yes
data/sales.csv,amount,"['$1,234.56','$789.00']",finance.currency.amount,high,finance.currency.amount,yes
```

---

## 4. Monitor Progress

The script prints a running summary:

```
[1234/10000] Processed: data/sales.csv:amount → finance.currency.amount (agree)
  Valid labels: 1189/1234 (96.4%)
  Agreement with FineType: 1023/1189 (86.0%)
  Invalid/rejected: 45 (3.6%)
```

### Common issues

| Issue | Fix |
|---|---|
| Ollama not running | `ollama serve` in another terminal |
| Model not loaded | `ollama pull qwen3:32b` |
| Out of memory | Switch to `qwen3:14b` or close other apps |
| Slow inference | Normal — ~0.5-1s per column on M1 |
| Invalid labels | Script retries once with stricter prompt |

---

## 5. Analyze Results

After the run completes:

```bash
# Quick summary
python3 scripts/analyze_llm_labels.py output/llm_labels.csv

# This shows:
# - Total columns labelled
# - Valid label rate
# - Agreement rate with FineType
# - Top disagreements (where LLM and FineType differ)
# - Type coverage (how many of 250 types were seen)
# - Confusion matrix for disagreements
```

### Key metrics to report

1. **Valid label rate** — % of LLM outputs that match a real FineType type
2. **Agreement rate** — % where LLM and FineType agree
3. **Systematic disagreements** — types where they consistently differ (these are the interesting ones!)
4. **Coverage** — how many of 250 types appeared in real-world data
5. **Ceiling check** — manually spot-check 200+ columns where they disagree. Who's right?

---

## 6. Return Results

Copy the output CSV back to the Beelink for integration:

```bash
scp output/llm_labels.csv hugh@beelink:~/github/noon-org/finetype/data/llm_labels/
```

Or commit to a branch:

```bash
git checkout -b nnft-269-llm-labels
git add output/llm_labels.csv
git commit -m "NNFT-269: LLM distillation labels from Qwen3 32B"
git push -u origin nnft-269-llm-labels
```
