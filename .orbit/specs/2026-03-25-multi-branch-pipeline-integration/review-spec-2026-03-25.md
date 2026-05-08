# Spec Review

**Date:** 2026-03-25
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-03-25-multi-branch-pipeline-integration/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] No Sharpen wiring exists — both multi-branch methods return bare results
**Category:** failure-mode
**Description:** `classify_multi_branch` and `classify_multi_branch_with_enriched` return immediately after the forward pass with `column_features: None` and no rule application. The spec describes "wiring Sharpen" but doesn't specify the insertion points or modification strategy for these two methods.
**Evidence:** Lines 1948–1994 and 2002–2049 in column.rs — both return bare ColumnResult. Neither calls `feature_disambiguate`, `disambiguate`, or locale detection. The Sharpen layer only exists in the Sense→Sharpen path.
**Recommendation:** AC-1 must explicitly state that both methods are modified in-place to: (a) compute ColumnFeatures, (b) call feature_disambiguate, (c) call disambiguate, (d) call header hint logic — in that order.

### [CRITICAL] F2/F3 vote guards will silently never fire with single-entry vote distribution
**Category:** failure-mode
**Description:** F2 checks `docker_in_votes` and F3 checks `hs_in_votes` against the vote distribution. Multi-branch produces `vec![(label, confidence)]` — only one entry. Neither docker_ref nor hs_code will appear as runner-up. Rules compile but silently never fire.
**Evidence:** F2 line 2143: `votes.iter().any(|(l, _)| l == "technology.container.docker_ref")`. F3 line 2188: `votes.iter().any(|(l, _)| l == "geography.trade.hs_code")`.
**Recommendation:** AC-2 verification must explicitly require these vote guards are removed, and unit tests must test with single-entry distributions.

### [CRITICAL] Attractor demotion (R15) will systematically over-demote with multi-branch votes
**Category:** failure-mode
**Description:** R15 computes `majority_fraction = top_count / n_samples`. With multi-branch, `top_count` is `confidence as usize` (0.85 → 0), giving `majority_fraction = 0/100 = 0.0`. This always triggers demotion. Also `select_fallback` iterates `votes.iter().skip(1)` — empty with single-entry, always falls to hardcoded fallback.
**Evidence:** Line 3910: `let majority_fraction = *top_count as f32 / n_samples as f32;`
**Recommendation:** Spec must explicitly address how confidence is carried in the Sharpen path — either pass confidence directly as majority_fraction or redesign the votes parameter.

### [CRITICAL] DuckDB extension has no multi-branch code path
**Category:** missing-requirement
**Description:** Extension uses `OnceLock<CharClassifier>` with embedded weights at build time. No mechanism to load `MultiBranchClassifier` which requires runtime filesystem artifacts (model.safetensors, config.json, model2vec/).
**Evidence:** `lib.rs` line 51: `static CLASSIFIER: OnceLock<finetype_model::CharClassifier>`. No multi-branch constructor.
**Recommendation:** Scope DuckDB to a follow-on milestone, or define concrete build strategy for embedding multi-branch weights.

### [CRITICAL] MCP server constructor takes CharClassifier, no multi-branch injection point
**Category:** missing-requirement
**Description:** `FineTypeServer::new(char_classifier: CharClassifier, ...)` is hardwired. AC-7 claims MCP returns multi-branch predictions but there's no code path to inject a MultiBranchClassifier.
**Evidence:** `crates/finetype-mcp/src/lib.rs` lines 174–195.
**Recommendation:** AC-7 must include MCP constructor changes as explicit deliverable.

### [WARN] sherlock-v4-sibling model existence not confirmed
**Category:** assumption
**Description:** Spec names sherlock-v4-sibling as default model but git status shows sherlock-v1/v2 only. No training prerequisite defined.
**Recommendation:** Add prerequisite confirming model is trained, or define training recipe.

### [WARN] No leading-zero true-positive test for F5
**Category:** test-gap
**Description:** AC-4 verifies false positive elimination but says "if present in eval set" for true positive preservation. If no genuine leading-zero columns exist in eval, F5's preservation path is untested.
**Recommendation:** Name specific eval columns with leading zeros or add synthetic fixture.

### [WARN] R12 untested with single-entry top_labels
**Category:** test-gap
**Description:** R12 triggers on `top_labels.iter().any(|l| numeric_types.contains(l))`. Works with single entry but no existing test confirms year/increment/postal branching with multi-branch input shape.
**Recommendation:** Add explicit tests with single-entry top_labels.

### [WARN] --legacy flag contradicts "removed" constraint
**Category:** constraint-conflict
**Description:** Constraint says "Sense removed from inference path." AC-8 says "gated behind --legacy." Exit condition says "build succeeds without Sense artifacts." If --legacy compiles in Sense loading, removal is incomplete.
**Recommendation:** Drop --legacy entirely or clarify it's compile-time gated.

---

## Honest Assessment

The core idea is sound and the rule-level analysis is thorough, but the spec treats "Sharpen" as if it already exists and just needs wiring. It doesn't — both multi-branch methods return bare results with no post-processing. The attractor demotion rule will systematically over-demote because of a type mismatch between vote counts and confidence scores. The DuckDB extension and MCP server have no multi-branch code path at all. Before implementation, the spec needs to: (1) define explicit Sharpen insertion points in the two multi-branch methods, (2) address the confidence-vs-count type mismatch in attractor demotion, (3) scope DuckDB/MCP to follow-on or define build strategy, and (4) resolve the --legacy contradiction.
