# scripts/archive

Orphaned research/experiment scripts, relocated here in the 2026-07-10 code
audit (`output/code-audit/findings.md`, Tier 1). Every script here had **zero
executable references** — nothing in any `.sh`, `.py` import, CI workflow,
Makefile, launchd plist, or glob invocation ran them — verified four ways
(reference scan over exec extensions, a git-grep filename pass, glob/dynamic
blind-spot checks, and a non-executable-reference bucketing that confirmed the
only remaining mentions were `.tsv` provenance columns and a `.gitignore`
comment, neither of which a move can break).

They are **retained, not deleted** — provenance for the research that produced
the shipped taxonomy, gates, and gold fixtures. Run any of them in place:
`python scripts/archive/<name>.py`.

## Why they went dormant

- **External-dataset ingestion** — dead since the human-verified gold corpus
  became the canonical eval (choice 0095): `compose_corpus`, `download_datasets`,
  `extract_columns`, `extract_gittables`, `extract_sherlock`, `extract_sotab`,
  `sample_gittables`, `sherlock_type_mapping`, and the dbpedia spikes
  (`audit_dbpedia_coverage`, `extend_dbpedia_mapping`, `finalize_dbpedia_overlap_spike`).
- **Pre-gold labelled-eval harness** — superseded by `score_gold_anchor.py`:
  `eval_ydf_on_labelled`, `grade_labelled_eval`, `infer_labelled_eval`,
  `sample_labelled_eval`, `audit_labelled_eval_vocabulary`.
- **Readjudication / phase-2 one-offs** — spent after the gold re-adjudication
  shipped: `aggregate_readjudication`, `apply_author_labels`, `apply_phase2`,
  `apply_readjudication_v2`, `analyze_llm_labels`, `phase2_plausibility_scan`,
  `migrate_gold_reframe`.
- **Gold-corpus construction one-offs** — gold is now append-only /
  emission-driven, not regenerated wholesale: `build_gold_corpus_candidates`,
  `build_gold_corpus_external`, `build_representative_fixture`,
  `build_spot_check_tsv`, `gold_fp_analysis`, `gold_queue_prioritise`,
  `generate_triage`.
- **Probes / studies** — exploratory, findings folded into decisions:
  `probe_coldist`, `probe_hierarchical_head`, `probe_range_features`,
  `probe_sibling_sweep`, `enum_predicate_study`, `bench_infer_floor`.
- **Distillation experiments**: `distill_batch`, `distill_concat`,
  `rescore_distillation`, `validate_distillation`, `build_identity_distilled`.
- **Other spent audits / builders**: `audit_geography_code_patterns`,
  `build_boundary_worklist`, `compute_cell_deltas_gated`,
  `embed_frontier_composed_loss`, `extract_identity_hard_negatives`,
  `extract_per_column_rejects`, `finetune_value_encoder`,
  `synth_prescreen_manifest`, `training_data_audit`, `validate_label_remap`,
  `validate_sources_yaml`.
