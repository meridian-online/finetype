# Progress — Gittables multi-lens corpus diagnostic

**Spec:** `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/spec.yaml`
**Card:** `0014-profile-validate-precision`

---

## 2026-05-20 — ac-01 closed (disposition MADR)

MADR `0087-gittables-diagnostic-absorbs-m19.yaml` written and accepted.
Three dispositions captured:

- **m-19** folded in (realism → corroboration filter; coverage → satisfied
  by construction; leakage firewall → constraint 6).
- **MADR 0066** re-anchored: v20 promotion gated on diagnostic's
  `report.md` shipping AND surfacing ≥1 `training_data_addition` gap.
- **Phase 2 inference module** (MADRs 0083–0086) paused; Phase 1
  contract live.

Reciprocal annotations landed on m-19's spec.yaml, MADR 0066, and MADRs
0083–0086. `orbit spec update --ac-check ac-01` flipped the AC.

Commit: `0247671 gittables-multi-lens-diagnostic: establish spec and close ac-01 disposition`.

---

## 2026-05-21 — ac-02 closed (DBpedia overlap spike)

**Result: `design_path = "proceed"`.** Default 4-lens corroboration
applies; ac-09's downgrade branch stays dormant.

Spike output: `eval/gittables/dbpedia_overlap_spike.json`

```
sample_seed:            20260520
n_tables:               94      (one lex-first parquet per topic directory)
n_columns_total:        1,249
n_annotated_columns:    996     (79.7% of columns have a DBpedia annotation)
n_mappable_columns:     490     (39.2% map to a non-trivial FineType type)
overlap_fraction:       0.3923
overlap_ci_lower:       0.3656  (Wilson 95% CI lower bound)
overlap_ci_upper:       0.4197  (Wilson 95% CI upper bound)
design_threshold:       0.20
design_path:            "proceed"  (CI upper 0.42 ≫ threshold 0.20)
```

The CI's lower bound (0.366) is itself well above the 0.20 threshold —
the design path is robust to substantial sampling variance.

### Method

1. Enumerated 94 topic directories under `/Users/hugh/datasets/gittables/`
   and selected the lexicographically-first parquet from each.
2. Extracted each parquet's `gittables` KV-metadata blob via DuckDB's
   `parquet_kv_metadata()`, JSON-parsed.
3. Per column, recorded the `dbpedia_semantic_column_types.<col>.id`
   (and the Schema.org analogue, retained for inspection but not voting
   per ac-09's lens-vote contract).
4. Hand-classified the 98 distinct DBpedia classes appearing in the
   sample against `eval/gittables/dbpedia_finetype_mapping.tsv` —
   `direct` / `partial` / `no_finetype_equivalent` per the
   ac-05 schema. "Mappable" means `direct` or `partial`.
5. Computed Wilson 95% CI on the proportion of mappable columns over
   the full sampled column count.

### Artefacts

- `scripts/spike_dbpedia_overlap.py` — Phase 1 extractor (94 parquets →
  raw TSV + universe TSV).
- `scripts/finalize_dbpedia_overlap_spike.py` — Phase 2 finalizer
  (raw + mapping → JSON summary with CI + design path).
- `eval/gittables/dbpedia_overlap_spike_raw.tsv` — per-column DBpedia /
  Schema.org annotation records.
- `eval/gittables/dbpedia_overlap_spike_universe.tsv` — distinct
  annotation classes with corpus counts.
- `eval/gittables/dbpedia_finetype_mapping.tsv` — hand-curated DBpedia
  → FineType mapping (98 rows covering the spike's universe; this seeds
  ac-05's full-corpus mapping but ac-05 will extend to every class
  appearing ≥10× in the full corpus).
- `eval/gittables/dbpedia_overlap_spike.json` — ac-02 deliverable.

### Caveats

- **Lex-first sampling bias.** Deterministic sampling of the
  lexicographically-first parquet per topic skews toward filenames that
  sort early (numerics, URL-encoded non-ASCII). The 39% mappability rate
  is a representative-but-imperfect estimate; the CI does not capture
  this sampling-design bias, only the within-sample sampling error.
  Acceptable for the spike's binary decision (CI upper bound is far
  above threshold), but should be re-examined in ac-06's full-corpus
  pass.
- **Mapping conservatism.** When a DBpedia class plausibly fits a
  non-trivial FineType type but with a semantic gap (e.g. DBpedia's
  `name` could be a person, organisation, or generic entity), it was
  classified `partial` and counted as mappable. A stricter mapping
  (direct-only) would lower the overlap fraction.
- **The 98 sampled classes are a strict subset of `dbpedia_labels.csv`'s
  122-class universe.** ac-05's full-corpus mapping must extend to the
  remaining ~24 classes that didn't appear in the lex-first sample.

---

## Open at end of session

- ac-01 ✅ closed (2026-05-20)
- ac-02 ✅ closed (2026-05-21)
- ac-03 next: YDF training pipeline + leakage audit.
- ac-04..ac-13 pending.
