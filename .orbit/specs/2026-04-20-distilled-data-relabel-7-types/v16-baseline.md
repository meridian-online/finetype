# v16 Baseline — Captured at Corpus-Freeze

**Spec ref:** `.orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3), acceptance_criteria `ac-10`.

**Purpose:** Pin sherlock-v16's profile eval score on the exact
corrected-GT corpus that v17 will be scored against (ac-11), so the
promotion gate `max(235/242, v16_baseline)` is honest under corpus churn.

---

## Pinned state

```
| Field                    | Value                                     |
|--------------------------|-------------------------------------------|
| score                    | 235 / 242 (97.1% label, 96.3% domain)     |
| git SHA (eval/ inputs)   | debedc15f6a339abb23135b8e4938cde7cc4a9f9  |
| eval SHA covers          | eval/datasets/, eval/schema_mapping.yaml  |
| wall-clock timestamp     | 2026-04-20T06:02:03Z                      |
| CLI version              | finetype 0.6.17                           |
| model evaluated          | models/sherlock-v16                       |
```

## Reuse policy (ac-11)

The same four fields above — score, eval inputs SHA, CLI version, model
dir — define the comparison baseline used in ac-11 to evaluate v17's 3
seeds. Specifically:

1. **Same eval inputs.** `eval/profile_eval.sh` must be invoked with
   `eval/datasets/manifest.csv` and `eval/schema_mapping.yaml` exactly
   as they exist at SHA `debedc15f6a339abb23135b8e4938cde7cc4a9f9`.
   Any drift in these files between now and v17 eval triggers a
   re-measurement of v16 on the drifted corpus (not a re-use of the
   pinned 235/242).
2. **Same CLI.** Evaluation binary: `finetype 0.6.17` compiled from the
   current branch HEAD. A CLI rebuild between seeds that changes
   inference semantics would also require re-measurement.
3. **Promotion gate.** `v17_winner_score ≥ max(235, v16_baseline_score)`.
   Since v16_baseline = 235, the gate is effectively `v17_winner ≥ 235`.
4. **No re-measurement against drifted GT** once the sweep starts.

## Reproducing this capture

```bash
git rev-parse HEAD                                          # source SHA
git log -n 1 --format=%H -- eval/datasets/ eval/schema_mapping.yaml
./target/release/finetype --version
FINETYPE_MODEL=models/sherlock-v16 make eval-report
grep "Profile label accuracy" eval/eval_output/report.md
```

## Corpus freeze notes

At the moment of this capture, `output/distillation-v4/` contains:

```
output/distillation-v4/
├── SOURCES.md                                    12.7 KB
├── loinc.csv                 (2,109 unique)      66 KB
├── user_agent.csv            (17,812 unique)     2.4 MB
└── loaders/
    ├── loinc.py
    └── user_agent.py
```

CSVs are gitignored; provenance flows through the loaders + SOURCES.md.
The v4 directory is frozen from this point until v17 sweep completes.
Any change to v4 content before ac-11 invalidates the baseline above.
