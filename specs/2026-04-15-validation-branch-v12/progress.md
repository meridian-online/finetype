# Implementation Progress

**Spec:** specs/2026-04-15-validation-branch-v12/spec.yaml
**Started:** 2026-04-15

## Hard Constraints
- [x] FTMB format bumps to v4 — additive header extension (30 bytes), backward-compatible reader for v1/v2/v3
- [x] Validation features are 239-dim, pre-computed into FTMB v4 — write_training_data_v4 includes validation_features per record
- [x] Validation uses CompiledValidator::is_valid() (patterns + numeric ranges) — extract() delegates to taxonomy.get_validator() → is_valid()
- [x] ISO 3166-1 alpha-2 enum added to country_code — 249 codes in definitions_geography.yaml
- [x] 5th branch: validation(239) → Dense → Dense → merge — BranchWeights loaded when valid_dim>0, forward_trunk concatenates output
- [ ] v11 weights NOT frozen — full retrain
- [x] No Sharpen rules (F1-F6, R1-R24) modified — only multi_branch.rs and column.rs (taxonomy threading) touched
- [x] No taxonomy types added or removed — only country_code validation gained enum field
- [x] Type-to-index mapping stored in model config — type_index_keys in MultiBranchConfig, deserialized from config.json
- [x] Validators compiled once, reused across columns — ValidationFeatureExtractor holds mapping, taxonomy holds compiled validators

## Acceptance Criteria
- [x] ac-01: ISO 3166-1 enum on country_code — 249 codes added to labels/definitions_geography.yaml, validator test confirms discrimination
- [x] ac-02: extract_validation_features() → 239-dim f32 vector — ValidationFeatureExtractor in validation_features.rs, 6 tests pass
- [x] ac-03: FTMB v4 format (30-byte header, writer) — write_training_data_v4() writes 30-byte header with valid_dim at offset 28-30, validation features per record
- [x] ac-04: FTMB v4 reader backward-compatible with v1/v2/v3 — read_training_header/read_training_data handle v1/v2/v3/v4, valid_dim defaults to 0 for older versions
- [x] ac-05: 5th validation branch in MultiBranchModel — config fields (valid_dim, valid_hidden, type_index_keys), branch loading, forward_trunk integration, 2 config tests. Training-side: MultiBranchModel gains valid_branch, forward()/forward_trunk()/forward_levels() accept valid_feats
- [x] ac-06: Type-to-index mapping in model config.json — type_index_keys serialized in MultiBranchConfig, validation_extractor built from saved keys at load time
- [x] ac-07: Inference pipeline computes validation features — classify_column() and classify_column_with_enriched_header() accept taxonomy, compute_validation_tensor() extracts and tensorizes features, forward_trunk zeros when absent
- [ ] ac-08: DuckDB extension supports v12 validation features
- [x] ac-09: extract-features CLI outputs validation features — CLI passes valid_dim to MultiBranchDataset construction
- [x] ac-10: Training loop reads FTMB v4 + feeds validation branch — TrainingRecord includes validation_features, dataset batch() returns 6-tuple, training loop passes valid_t to forward(). 73 training tests pass.
- [ ] ac-11: Gate — profile eval ≥ 215/227
- [ ] ac-12: Gate — no regression on v11-correct columns
- [ ] ac-13: Gate — actionability ≥ 96.7%
- [ ] ac-14: Gate — all existing tests pass

## Team Split
- **@nightingale (lead):** ac-01, ac-02, ac-06, ac-07 — taxonomy, validation extraction, config, inference
- **Teammate:** ac-03, ac-04, ac-05, ac-09, ac-10 — FTMB v4, model architecture, CLI, training loop
- **Sequential (after merge):** ac-08, ac-11–14 — DuckDB extension, eval gates
