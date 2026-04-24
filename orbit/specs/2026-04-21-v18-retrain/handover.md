# v18 Retrain — Handover

**Spec:** `orbit/specs/2026-04-21-v18-retrain/spec.yaml` v1.3
**Outcome:** HELD (not promoted)
**Decision of record:** `orbit/decisions/0062-v18-outcome.md`
**Winner:** `models/sherlock-v18-seed-42/` at 297/352 (val_acc 0.9134)

## Release Scope Decision

**No promotion.** `models/default` symlink stays on v16. HuggingFace model card stays on v16. CI `FINETYPE_CI_MODEL` env var stays on v16.

Rationale: net-zero label-accuracy delta (8 fixes / 8 regressions / 47 persistent). Datetime domain regression at the gate limit (+3 of a 3-column allowance). See decision 0062 for full reasoning.

## Deliverable Status

| AC | Status | Artefact |
|---|---|---|
| ac-01 | ✅ | `triage.md` — 55 v16 failures enumerated |
| ac-02 | ✅ | `scripts/sweep_v18.sh`, `scripts/prepare_multibranch_data.py` (8 markers) |
| ac-03 | ✅ | `orbit/decisions/0060-v18-corpus-base.md` (v3 base) |
| ac-04 | ✅ | Row-hash firewall ACTIVE in `results/sweep-v18.log` (8 markers, 0 leaked) |
| ac-05 | ✅ | 3-seed sweep cleanly in 440 min; `results/sweep-v18-summary.csv` |
| ac-06 | ✅ | Training gate — all 3 seeds AUTO_ACCEPT (≥0.912) |
| ac-07 | ✅ | Promotion gate — seed 42 meets both conditions at the limit |
| ac-08 | ✅ | `orbit/decisions/0061-sweep-data-seed-discipline.md` (first adopter) |
| ac-09 | ✅ | `orbit/decisions/0062-v18-outcome.md` (held) |
| ac-10 | ✅ | This document |

## Data Artefacts on Branch

Branch: `distilled-data-relabel-7-types-v17` (active; retained from v17).

```
models/sherlock-v18-seed-42/   — winner
models/sherlock-v18-seed-43/   — 296/352 (one below v16)
models/sherlock-v18-seed-44/   — 292/352 (five below v16)
results/sweep-v18.log          — full sweep log with 8 markers
results/sweep-v18-summary.csv  — per-seed results
orbit/specs/2026-04-21-v18-retrain/
  spec.yaml                    — v1.3 (accepted)
  interview.md
  triage.md                    — 55 v16 failures
  progress.md                  — AC tracking
  v16-v18-diff.md              — per-column diff (fixes/regressions/persistent/churn)
  review-spec-2026-04-21*.md   — review forks (cycles 1-3)
  drive.yaml                   — full-auto drive state
  handover.md                  — this document
orbit/decisions/
  0060-v18-corpus-base.md
  0061-sweep-data-seed-discipline.md
  0062-v18-outcome.md
```

The shared FTMB at `output/multibranch-training/v18.ftmb` was deleted post-sweep (saves ~950 MB; reproducible from `scripts/sweep_v18.sh` prep block if ever needed).

## Follow-Up Cards (Backlog)

> **2026-04-24 supersession — MADR 0067.** The framing below ("needs
> per-subtype generators", "needs generator exemplars") was a presumption
> that the remedy lived in training data. For the amount-variant cluster
> (item 1), that presumption was tested by
> `orbit/specs/2026-04-24-amount-variant-generators/` and found **false**:
> the primary mechanism was `header_hint()` over-generalisation, not a
> training-data gap. The fix was a 12-arm table edit to
> `crates/finetype-model/src/column.rs`; no retrain was performed; net
> target lift was +11 / 11 on the 11 eval columns. Items 2 and 3 below
> may or may not have the same mechanism — run the diagnostic arc
> (ac-01..ac-04 of the amount-variant spec) on each cluster before
> committing to training-layer work. See MADR 0065 (mechanism), MADR 0066
> (v19 retrain hard gate), MADR 0067 (this framing correction).

The v18 diff surfaces three concrete capability cards for v19:

1. **Amount-variant generator card** — ~~11 amount subtypes collapse to plain `amount` in both v16 and v18. Needs per-subtype generators with distinct value-shape signatures.~~ **SHIPPED 2026-04-24** as a pipeline-layer fix; no generator work was needed. See `orbit/specs/2026-04-24-amount-variant-generators/`.
2. **Container-type generator card** — 8 container types (`xml`, `csv`, `html`, `yaml`, `json_array`, `query_string`, `semicolon_separated`, `whitespace_separated`) collapse to `categorical`. **Run the diagnostic arc before writing generators** — the collapse may be a Sharpen hint-layer artefact rather than a training-data gap.
3. **Datetime-subtype generator card** — 6 datetime subtypes (`iso_microseconds`, `jp_era_short`, `julian`, `ordinal`, `pg_short_offset`, etc.) collapse to their nearest specific timestamp. **Run the diagnostic arc before writing generators** — same caveat; datetime has 84 taxonomy definitions and substring aliasing in `header_hint()` is a plausible primary cause.

Also carried forward:

- **v4-UA-adoption card** (decision 0060 follow-up) — v4's distilled UA loader (17,812 UAs ex ua-parser/uap-core) in isolation, now that v18's v3-corpus baseline is measured.
- **Expanded-eval regressions** (v18's 8 regressions): sha256/git_sha → tsid, mdy_short_slash → dmy_short_slash, token_urlsafe → url, weekday_full_month → weekday_abbreviated_month, inchi / smiles / server_hostname. Worth a triage pass before v19.

## Validation Checklist

Pre-handover gate verification:

- [x] `grep -c "hash_filter_active: true" results/sweep-v18.log` → 1
- [x] All 8 prep markers present in log (`corpus_base`, `eval_hash_table_sha256`, `pre_filter_rows`, `row_hash_overlap`, `post_filter_rows`, `hash_filter_active`, `leaked_rows_after_filter`, `n_sibling_headers`)
- [x] Three per-seed dirs with 6+ files each (config.json, label_map.json, model.safetensors, results.json, epochs.jsonl, eval/report.md)
- [x] `results/sweep-v18-summary.csv` present with 3 rows
- [x] `models/default` symlink unchanged (still points at v16)
- [x] `FINETYPE_CI_MODEL` in `.github/workflows/*.yml` unchanged
- [x] No HuggingFace upload performed

## Next Drive

v19 prep requires a triage decision on the 2 remaining follow-up clusters (container-type, datetime-subtype). The amount-variant cluster (largest, 11 persistent misses) shipped as a pipeline-layer fix on 2026-04-24 — see `orbit/specs/2026-04-24-amount-variant-generators/` and MADRs 0065/0066/0067. Recommend `/orb:discovery` on `container-type generators` next, following the same diagnostic arc (ac-01 corpus counts → ac-02 value-shape Jaccard → ac-03 confusion → ac-04 raw softmax top-k) before committing to training-layer remediation.
