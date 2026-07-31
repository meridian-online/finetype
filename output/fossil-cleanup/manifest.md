# Fossil-cleanup retention manifest (ac-01)

**Spec:** `spec 2026-06-10-fossil-cleanup` · **Drafted:** 2026-06-10 · **Status:** awaiting author signature — nothing has been moved.

Classification rule: **keep** requires a live reference (CLAUDE.md, an open spec/task, the shipped-default symlink target, git tracking, the `build_rare_type_gold.py` MODELS table, a preserving memory, or pipeline-infrastructure consumption). **archive** = no live reference but forensically plausible or recoverable elsewhere. **delete** = clearly dead, verdict recorded elsewhere. Default on uncertainty is archive — never delete.

Reference sources consulted: CLAUDE.md; the 14 open specs (`orbit spec list`) and their `spec.yaml` path references; `orbit task ready` (9 open tasks); `git ls-files models/ output/ eval/gittables/`; `scripts/build_rare_type_gold.py` MODELS table; `readlink models/default` → `sherlock-v19-relu-s42`; memory `b3-value-expert-fusion-dead-end`; grep of `crates/` + `scripts/` for infrastructure consumers.

## Summary

| Area | keep | archive | delete | disk keep | disk archive | disk delete |
|---|---|---|---|---|---|---|
| `output/` (31 dirs) | 13 | 18 | 0 | ~1.2 GB | ~3.9 GB | — |
| `models/` (100 entries) | 33 | 46 | 21 | ~95 MB | ~365 MB | ~160 MB |
| `eval/gittables/` (12 large artefacts) | 6 | 3 | 2 | ~9.7 GB | ~30 MB | ~1.3 GB |
| **Total** | **52** | **67** | **23** | **~11 GB** | **~4.3 GB** | **~1.5 GB** |

Immediate reclaim on signature ≈ **1.5 GB** (delete class). Archive class ≈ **4.3 GB** moves out of the working tree but stays recoverable. Note: many archive-class `output/` dirs contain **git-tracked .md reports** — tracked files stay in git regardless of what happens to the untracked bulk around them; "archive" applies to the untracked contents.

---

## output/ (31 directories)

| path | size | class | reason |
|---|---|---|---|
| `output/branch-ablation-v22` | 236K | archive | v22 ablation study; v22 promotion deferred, findings in committed reports — no live reference. |
| `output/cluster-reachability` | 12M | keep | CLAUDE.md (`redesign_memo_v3.md`) + three open reachability specs (2026-05-29/30/31) reference scores, memos, fixtures. |
| `output/corpus-honest-gate` | 206M | keep | CLAUDE.md (gate substrate: `ac03_four_verdict_reproduction.md`, `refined/oracle_aware_bands.md`) + MODELS table (`v0624_pass/`, `fusion_v27_pass/` columns.parquet) + open spec 2026-06-10-human-verified-gold-corpus. Inline note: `.err/.out` run logs inside are prune candidates at execution time, but the dir is keep. |
| `output/corpus-pass-v20` | 743M | archive | Raw corpus pass of superseded v20 geography candidate; canonical scoring parquets live in `ydf-validation-gate`. Regenerable (9h) from archived model. |
| `output/corpus-pass-v21` | 741M | archive | Same — superseded v21 geonames candidate. |
| `output/corpus-pass-v22` | 741M | archive | v22 verdict recorded (CLAUDE.md, `ydf-validation-gate/v22_re_baseline.md`); the scored artefact `v22_gated.parquet` lives in ydf-validation-gate (keep). |
| `output/corpus-pass-v23` | 739M | archive | v23 verdict recorded (relitigation memo, CLAUDE.md); `v23_gated.parquet` lives in ydf-validation-gate (keep). |
| `output/destination-drift-precheck` | 1.4M | keep | CLAUDE.md (`calibration.md` — mandatory pre-check doctrine); contains active mfg-campaign drift reports. |
| `output/distillation-latdec` | 3.8M | archive | Training data for failed latdec retrain; NO-GO verdict recorded in CLAUDE.md + corpus-honest-gate reports. |
| `output/distillation-v20` | 6.5M | archive | Superseded v20 campaign training data. |
| `output/distillation-v21` | 14M | archive | Superseded v21 campaign training data. |
| `output/distillation-v21-geonames` | 7.5M | archive | Superseded v21 variant training data. |
| `output/distillation-v22` | 35M | archive | v22 training data; v22 model weights retained separately (campaign head keep, below). |
| `output/distillation-v23` | 23M | archive | Failed v23 campaign training data; verdict in `v23-precision-retrain/relitigation_memo.md`. |
| `output/distillation-v24` | 23M | archive | Failed v24 campaign training data; drift verdict recorded in CLAUDE.md. |
| `output/distillation-v3` | 6.5M | keep | Open spec 2026-06-08-late-fusion references `sherlock_distilled.csv.gz`. Note: campaign is a recorded dead end (memory `b3-value-expert-fusion-dead-end`) — downgrade to archive when the spec closes. |
| `output/distillation-v4` | 6.2M | archive | Early distillation iteration, superseded; no live reference. |
| `output/eval-ceiling-diagnosis` | 201M | keep | CLAUDE.md (`finding.md`, current-sprint root cause) + open spec 2026-06-10-human-verified-gold-corpus needs `review_sample.csv`. |
| `output/false-veto-sweep` | 376K | archive | Evidence cited only by choice 0091 and the closed 2026-06-05 spec; committed reports carry the finding. |
| `output/gold-eval-anchor` | 204K | keep | Open specs 2026-06-06-latdec-retrain (`metrics_v19...tsv`, `next_step_diagnosis.md`) and 2026-06-10-human-verified-gold-corpus (ancestor fixture machinery). |
| `output/late-fusion` | 797M | archive | Dead-end campaign (memory `b3-value-expert-fusion-dead-end` cites `deferral_finding.md`, `deferral_v1_report.md` — both git-tracked, so they survive archiving). Bulk is regenerable `.f32` feature dumps (~740M). Open spec 2026-06-08 still references the dir — see Needs author call. |
| `output/latitude-decimal-precision` | 160K | keep | Campaign spec 2026-06-06-latitude-decimal-hard-negative-retrain is open; tiny. |
| `output/mining-factory` | 46M | keep | Open spec 2026-06-07-reference-data-mining-factory references `output/mining-factory/` directly. |
| `output/spike-results` | 332K | archive | Early architecture spike, long superseded; no live reference. |
| `output/v22-direction-review` | 20K | keep | CLAUDE.md references `output/v22-direction-review/` (gated cell-2 verdict). |
| `output/v23-precision-retrain` | 1.3M | keep | CLAUDE.md (`relitigation_memo.md`) + open reachability spec references it. |
| `output/v23-sharpen-codes` | 4K | archive | Tiny superseded sharpen experiment; no live reference. |
| `output/v24-numeric-precision` | 1.3M | archive | v24 verdict recorded in CLAUDE.md prose; no file in the dir is referenced. |
| `output/value-level-arch-test` | 29M | keep | Open spec 2026-06-08-late-fusion references `eval_v25_prod.json`. Dead-end note as per distillation-v3. |
| `output/value-level-labelling` | 385M | keep | Open spec 2026-06-08-late-fusion references `cleaned_capped.ndjson` (training data for the HF-preserved v25 expert). Downgrade candidate on spec close — see Needs author call. |
| `output/ydf-validation-gate` | 304M | keep | Load-bearing: CLAUDE.md names `v19_gated.parquet` as the stable gate oracle; MODELS table reads v19/v22/v23 gated parquets; open specs 2026-06-06 and 2026-06-08 reference the dir. Dir-level keep even though some sibling files inside are archive candidates. |

## models/ (100 entries)

Git-tracked dirs are **keep for now** — they fall under the open public-release-readiness task (`t-0001080618b5805331c656b0`: add a restore path, *then* `git rm --cached`). Untracking and any subsequent cleanup is that task's call, not this manifest's.

| path | size | class | reason |
|---|---|---|---|
| `models/default` → `sherlock-v19-relu-s42` | symlink | keep | Shipped default symlink. |
| `models/sherlock-v19-relu-s42` | 9.9M | keep | The shipped default Sense-stage model (symlink target). |
| `models/sherlock-v19-relu-s43` | 9.9M | archive | Sibling seed of shipped run — seed-reproducibility value. |
| `models/sherlock-v19-relu-s44` | 9.9M | archive | Sibling seed of shipped run — seed-reproducibility value. |
| `models/sherlock-v22-boundary-relu-s44` | 9.9M | keep | Campaign head named in CLAUDE.md (training-target baseline for the corpus diagnostic). |
| `models/sherlock-v22-boundary-relu-s42` | 9.9M | archive | Sibling seed of campaign head. |
| `models/sherlock-v22-boundary-relu-s43` | 9.9M | archive | Sibling seed of campaign head. |
| `models/model2vec` | 8.0M | keep | Git-tracked AND pipeline infrastructure — header-branch embedding consumed by `finetype-model` (`model2vec_shared.rs`, `sibling_context.rs`), CLI, MCP, DuckDB crates. |
| `models/sibling-context` | 1.5M | keep | Git-tracked; load-bearing in the default pipeline (per open public-release-readiness task). |
| `models/entity-classifier` | 700K | keep | Git-tracked. |
| `models/sense`, `models/sense_prod`, `models/sense_rust`, `models/sense_spike` | 52K | keep | Git-tracked; original Sense implementation remains in code (decision 0041). |
| `models/test` | 41M | keep | `models/test/model.safetensors` referenced by open spec 2026-06-02-public-release-readiness. |
| `models/char-cnn-v1`, `v2`, `v4`, `v7`, `v8`, `v9`, `v10`, `v11`, `v12` | ~500K | keep | Git-tracked (v7 is a legacy `--model-type` opt-in per open task). |
| `models/char-cnn-v12.snapshot.20260305T045033Z` | 4K | keep | Git-tracked — would otherwise be delete-class snapshot debris; cleanup belongs to the public-release-readiness untracking task. |
| `models/tiered`, `tiered-v1`, `tiered-v3` | 9.2M | keep | Git-tracked (tiered-v1 is a legacy opt-in per open task). |
| `models/tiered-v2.snapshot.20260227T231445Z` | 20K | keep | Git-tracked snapshot — same note as char-cnn-v12 snapshot. |
| `models/sherlock-v1-flat`, `v1-hier`, `v2-flat`, `v2-hier` | 19M | keep | Git-tracked early lineage. |
| `models/fusion-v26` | 4K | keep | Git-tracked (config stub). Would otherwise be delete-class (NO-GO ×2 recorded in memory `b3-value-expert-fusion-dead-end`); flag to the untracking task. |
| `models/char-cnn-base` | 412K | keep | Untracked but active mining-factory campaign artefact (open spec 2026-06-07); created this week. No doc reference yet — see Needs author call. |
| `models/char-cnn-mfg` | 412K | keep | Same — active mining-factory campaign artefact. |
| `models/char-cnn-v15-gittables` | 412K | keep | Named in open task `t-0000ca5a` (infer feature-vector bug repro). |
| `models/char-cnn-v13-feat` | 412K | archive | Superseded char-cnn feature experiment; no live reference. |
| `models/char-cnn-v13-probe` | 396K | archive | Superseded probe experiment. |
| `models/char-cnn-v14-feat` | 412K | archive | Superseded feature experiment. |
| `models/char-cnn-v15-dirty` | 412K | archive | Superseded variant of v15; the referenced sibling is v15-gittables. |
| `models/value-charcnn-v25` | 392K | archive | Explicitly preserved per memory `b3-value-expert-fusion-dead-end` — recoverable from HF `value-charcnn-v25` + local backup `~/mac_backup/finetype-v25-20260608`. Open spec 2026-06-08 references it, but redundancy makes archive safe. |
| `models/fusion-head-v25` | 1.9M | archive | Dead-end fusion campaign's first head; basis of the family-A analysis in the recorded finding. |
| `models/fusion-head-v26` | 1.9M | delete | NO-GO recorded (corpus-honest gate, memory `b3-value-expert-fusion-dead-end`). |
| `models/fusion-head-v27`, `-a05`, `-a07`, `-a10` | 7.8M | delete | NO-GO recorded; a05/a07/a10 are alpha-ablation duplicates of the same dead bet. |
| `models/fusion-v27` | 4K | delete | Config stub of NO-GO candidate (untracked, unlike fusion-v26). |
| `models/sherlock-v3-flat`, `v3-hier` | 9.9M | archive | Historical lineage, superseded. |
| `models/sherlock-v4-baseline`, `v4-sibling` | 10M | archive | Historical lineage. |
| `models/sherlock-v5-current`, `v5-scaled` | 14.5M | archive | Historical lineage (v5-scaled config json is tracked; weights untracked). |
| `models/sherlock-v6`, `v6-gelu`, `v6-gelu-conservative`, `v7` | 29M | archive | Historical lineage incl. activation ablations. |
| `models/sherlock-v10-gelu`, `v11`, `v12`, `v13`, `v14` | 48.5M | archive | Historical lineage. |
| `models/sherlock-v16`, `v16-seed-42/43/44` | 40M | archive | Pre-v19 generation, stepping stone (not a failed promotion). |
| `models/sherlock-v16-smoke` | 10M | delete | Smoke-test run debris. |
| `models/sherlock-v17-seed-42/43/44` | 30M | archive | Pre-v19 generation. |
| `models/sherlock-v18-seed-42/43/44` | 30M | archive | Pre-v19 generation. |
| `models/sherlock-v19-gelu-s42/s43/s44` | 30M | archive | Activation-ablation siblings of the shipped ReLU run. |
| `models/sherlock-v20-geography-relu-s42` | 10M | archive | Failed/superseded geography campaign — retain one seed for forensics. |
| `models/sherlock-v20-geography-relu-s43/s44` | 20M | delete | Duplicate seeds of superseded campaign; raw pass archived (`output/corpus-pass-v20`). |
| `models/sherlock-v21-geonames-geography-relu-s42` | 10M | archive | Superseded geonames campaign — retain one seed. |
| `models/sherlock-v21-geonames-geography-relu-s43/s44` | 20M | delete | Duplicate seeds, verdict superseded by v22 campaign. |
| `models/sherlock-v23-precision-relu-s42-fixed` | 10M | archive | The actual v23 candidate — failed promotion, verdict in relitigation memo; retain one seed. |
| `models/sherlock-v23-precision-relu-s42` | 10M | delete | The pre-fix broken run, superseded by `-fixed`. |
| `models/sherlock-v23-precision-relu-s43/s44` | 20M | delete | Duplicate seeds of failed candidate; verdict recorded (CLAUDE.md + relitigation memo). |
| `models/sherlock-v23proxy-s42` | 10M | delete | One-shot drift-proxy model; its drift report is recorded. |
| `models/sherlock-v24-numeric-relu-s42` | 10M | archive | Failed v24 candidate (latitude 4.3× drift, recorded in CLAUDE.md) — retain one seed. |
| `models/sherlock-v24-numeric-relu-s43/s44` | 20M | delete | Duplicate seeds of failed candidate, verdict recorded. |
| `models/sherlock-v24proxy-s42` | 10M | delete | One-shot drift-proxy model, report recorded. |
| `models/sherlock-latdec-relu-s42` | 9.9M | archive | Failed latdec candidate (NO-GO recorded) — retain one seed; its corpus pass is in the MODELS table. |
| `models/sherlock-latdec-relu-s43/s44` | 20M | delete | Duplicate seeds of failed candidate, verdict recorded. |
| `models/sherlock-latdec-proxy-s42` | 9.8M | delete | One-shot drift-proxy model, report recorded. |
| `models/sherlock-mfg-proxy-s42`, `-proxy2-s42`, `-proxy3-s42` | 29.5M | archive | Drift-proxy iterations for the OPEN mining-factory campaign — proxies are one-shot by design and regenerable in ~10 epochs, but the campaign is live. See Needs author call. |
| `models/*.json` configs + `.gitkeep` | <2K | keep | Git-tracked. |

## eval/gittables/ (directory level)

| path | size | class | reason |
|---|---|---|---|
| `eval/gittables/corpus_pass` | 1.3G | keep | CLAUDE.md (m-19 diagnostic deliverable, `report.md`); `columns.parquet` + `corroborated_gaps.parquet` referenced by open reachability specs and open task (re-baseline diagnostic on v19). |
| `eval/gittables/corpus_pass.run1-buggy` | 815M | delete | Named buggy-run debris, superseded by the canonical pass. |
| `eval/gittables/corpus_pass.run3-spawn-pressure` | 517M | delete | Failed-run debris (spawn-pressure incident); run logs at gittables top level record it; canonical pass supersedes. |
| `eval/gittables/corpus_pass_calibrate_reprocheck` | 616K | archive | Calibration reproducibility check — evidence of the fixed-file-list doctrine, tiny. |
| `eval/gittables/corpus_pass_latdec` | 442M | keep | `corpus_pass_latdec/corpus_pass/columns.parquet` is in the `build_rare_type_gold.py` MODELS table (latdec row). |
| `eval/gittables/cycles` | 5.9M | archive | Per-cycle cron gate/work JSON logs. No script writes/reads the dir by grep; `cycle_log.jsonl` (top level) is what `cron_status.sh` reads. See Needs author call re cron liveness. |
| `eval/gittables/models` | 7.8G | keep | Live gate infrastructure: the YDF oracle (`ydf.bin` 7.0G, `ydf_value` 781M) consumed by `scripts/train_ydf.py` / the `--fill-ydf` phase — required to score every future promotion candidate through the mandatory corpus-honest gate. Largest single keep; if disk pressure bites, this is the one to re-litigate (rebuild cost = full YDF retrain). |
| `eval/gittables/v20_training_candidates` | 908K | keep | 8 files git-tracked. |
| `eval/gittables/.content_hash_cache.tsv` | 152M | keep | Consumed by `scripts/cron_cycle_work.py` and `scripts/gittables_holdout_freeze.py`; regenerable cache but live consumers exist. |
| `eval/gittables/corpus_paths.txt` (+ `.sha256`) | 64M | keep | The fixed corpus file list — cross-model snapshots MUST share it (CLAUDE.md: DuckDB sampling is not seed-reproducible). |
| `eval/gittables/harvest_pool.tsv` | 23.5M | archive | No live reference found in scripts or open specs; plausible mining-factory input — see Needs author call. |
| `eval/gittables/` small top-level files (REPORT.md, eval*.sql, dbpedia_*, schema_*, logs, ydf_* audits) | ~25M | keep | Mostly git-tracked; the rest are cheap run logs co-located with the canonical pass. Not worth itemising. |

## Needs author call (7)

1. **`output/late-fusion` bulk (~780M of `.f32` dumps)** — open spec 2026-06-08 references the dir, but the campaign is a recorded dead end. Classified archive on the memory's authority; confirm the spec should close (which also settles items 2–3).
2. **`output/value-level-labelling` (385M) + `output/value-level-arch-test` (29M) + `output/distillation-v3`** — keep by the open-spec rule, downgrade to archive when 2026-06-08-late-fusion closes. Author to confirm closing intent.
3. **`models/sherlock-mfg-proxy-s42/2/3` (3 × ~10M)** — archive assumes their drift reports already banked the verdicts; if the mining-factory campaign still iterates against them, promote to keep.
4. **`models/char-cnn-base` + `models/char-cnn-mfg`** — classified keep as active mining-factory artefacts, but nothing in the spec or tasks names them. Confirm they are the campaign's working models.
5. **`eval/gittables/cycles` (5.9M)** — cron cycle logs; if the nightly cron is still active it will regenerate or still append. Keep if cron is live, archive if retired.
6. **`eval/gittables/harvest_pool.tsv` (23.5M)** — no reference found; confirm it is not a mining-factory input before archiving.
7. **`output/corpus-pass-v20/v21/v22/v23` (2.9G total)** — classified archive, but if the author is content that the gated parquets in `ydf-validation-gate` are sufficient forensic record, these four are the largest delete upgrade available (raw passes regenerable at ~9h each from the archived models).

---

**Nothing has been moved. Execution is ac-02, gated on author signature.**
