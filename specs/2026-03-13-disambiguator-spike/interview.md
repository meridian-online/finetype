# Interview: Learned Disambiguator — Replacing the Rule Cascade

**Date:** 2026-03-13
**Interviewer:** Nightingale

---

## Context

The FineType blog post ("Less Typing, More Signal") demonstrates profiling on the USGS earthquake dataset. The `id` column (`us6000pgkh`) is misclassified as `representation.identifier.increment` (BIGINT) at 83% confidence — a hardcoded header hint (`"id" => increment`) overrides the model's correct value-level classification. The `load` command then generates `CAST(id AS BIGINT)` which fails on alphanumeric values.

This exposed a deeper concern: the disambiguation pipeline has grown to 75+ hardcoded header hints and 5 hand-written feature rules (F1–F5), with fragile assumptions. The `id` → `increment` mapping assumes all IDs are numeric sequences, but real-world `id` columns are UUIDs, alphanumeric codes, slugs, or sequential numbers.

Key reference: LLM distillation findings (`specs/llm-distillation/FINDINGS.md`) showed the LLM was genuinely better than FineType on ~990 cases, particularly for ambiguous types where header context matters more than character patterns.

## Interview Q&A

### Q1: Approach
**Q:** The core issue is that `load` generates casts from profile results without considering confidence. What's the fix you have in mind?
**A:** Both — confidence threshold in load as safety net, plus improving inference. Mentioned the LLM distillation work as potential input for inference improvements.

### Q2: Inference direction
**Q:** For the inference improvement: LLM disagreements as training signal, deterministic pipeline fixes, or scope to load only?
**A:** Asked for a diagnosis first — was the earthquake `id` issue an inference failure or a header hint?

### Q3: Diagnosis result + scope
**Q:** Root cause is the `id` header hint overriding the model's correct classification. Also 3–4 other misclassifications in the dataset. What scope?
**A:** Full earthquake dataset accuracy pass. Also concerned about the growing number of hardcoded rules and their underlying assumptions — `id` fields are of many types.

### Q4: Hardcoded rules concern
**Q:** Are you questioning the hardcoded hint approach itself, or specific bad mappings?
**A:** Question the approach itself. The hardcoded hint table has grown to a maintenance burden.

### Q5: Replacement direction
**Q:** Three options: hints as soft priors, prune to high-confidence only, or move hints to taxonomy definitions?
**A:** Move hints to taxonomy definitions — make them data-driven, editable, testable, version-controlled alongside type definitions.

### Q6: Scope splitting
**Q:** Should we split taxonomy hint migration from the load safety net?
**A:** Taxonomy migration first. The load safety net is a band-aid — if hints are correct, you don't need it.

### Q7: YAML schema for header patterns
**Q:** Simple string list, structured with match type, or regex patterns?
**A:** Regex patterns. Maximum flexibility.

### Q8: Hint-model interaction
**Q:** How should taxonomy-based hints interact with the model's prediction?
**A:** This needs to be data-driven, not rule-based. Reminded of the Sherlock-inspired features already in the pipeline. The features are underutilised — they should inform when to trust hints vs values.

### Q9: Data-driven approach
**Q:** Header match as input feature to a learned model, feature-based guards on YAML patterns, or full classifier retrain?
**A:** This needs a spike to investigate.

### Q10: Spike scope
**Q:** What should the spike investigate?
**A:** Feature-based disambiguator spike: take 36 column-level features + header embedding + model vote distribution, train a small classifier to predict the correct type. Test on the profile eval's 174 columns.

### Q11: Quick safety net
**Q:** Ship a load confidence threshold now while the spike runs?
**A:** No — spike first, then decide. Don't add more thresholds that might need rethinking.

### Q12: Remove id hint
**Q:** Remove the `id` → increment hardcoded hint now?
**A:** Yes — remove it now. It's actively harmful and `id` columns are genuinely ambiguous. The model already classifies correctly without the hint.

---

## Summary

### Goal
Investigate whether a learned disambiguator (using column-level features + header embedding + vote distribution) can replace the hardcoded rule cascade (75+ header hints, 5 feature rules). This is a spike — the outcome is knowledge, not shipped code.

### Immediate action
Remove the `id` → `increment` hardcoded header hint. It's actively harmful for alphanumeric ID columns and the model classifies correctly without it.

### Constraints
- Spike uses the profile eval's 174 labelled columns as ground truth
- Disambiguation model should be small (logistic regression or shallow MLP)
- Must maintain ≥ 97% label accuracy on profile eval (no regression)
- Feature-based approach preferred over additional rules
- Long-term direction: move header patterns from hardcoded function to taxonomy YAML with regex patterns

### Success Criteria (spike)
- Evidence that a learned disambiguator matches or beats the current rule cascade on profile eval
- Identification of which features are most predictive for disambiguation
- Recommendation on whether to proceed with full implementation

### Open Questions
- Training data source: 174 profile eval columns may be too few for a learned model
- Should LLM distillation data (5,359 columns) be used as supplementary supervision?
- Model architecture: logistic regression vs MLP vs decision tree
- Whether the semantic hint classifier (Model2Vec) should be replaced or integrated
- Taxonomy YAML header_patterns schema: regex patterns confirmed, but exact YAML structure TBD
