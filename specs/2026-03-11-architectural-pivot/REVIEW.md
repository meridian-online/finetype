# Sense & Sharpen — Nightingale's Review

## Context

Hugh wrote a discovery brief proposing an architectural pivot from FineType's current tiered CharCNN cascade (34 models, 18 disambiguation rules) to a two-stage "Sense & Sharpen" pipeline: a column-level transformer for semantic routing, followed by scoped CharCNN + validation for format classification. This review is my assessment of the brief, its risks, and a recommended approach.

---

## Verdict: The Diagnosis Is Sound, the Treatment Needs Phasing

The brief correctly identifies that FineType has hit a structural ceiling. The evidence is compelling:

- **CLDR regression** (116/120 → 107/120) is the smoking gun — a well-planned, five-phase retraining made things *worse*, revealing cascading fragility in the tier graph, not isolated data issues.
- **CharCNN capacity ceiling** (≤20 labels per T2 model) is a hard architectural limit — documented in NNFT-126 and decision-002. We cannot scale the taxonomy or add locale variants without fracturing the model graph further.
- **Rule proliferation** is real. The 18 rules have strict ordering dependencies (Duration must run before Attractor Demotion; UTC Offset must run between them; Entity Demotion must fire before header hints but has a guard that blocks header hints entirely). The interaction surface is becoming difficult to reason about — and each new accuracy fix adds another rule.
- **v0.1.8 → v0.3.0 was a lateral move** at the macro level (GitTables *regressed* 1.3pp, SOTAB gained 1.1pp). The tiered architecture + disambiguation system didn't deliver the step change we expected. This suggests the architecture, not just tuning, is the bottleneck.

The Burn prototype confirming that a Transformer handles 4-level locale labels where CharCNN collapsed — that's the architectural existence proof we need.

**I support this pivot.** But I think it needs sharper phasing to manage risk, and some of the brief's expectations need calibrating.

---

## Feedback on Specific Proposals

### 1. Sense Model Scope: Narrow First, Expand Later

The brief asks Sense to produce four outputs simultaneously:
- Broad semantic category
- Entity subtype
- Locale signal
- Confidence estimate

That's ambitious for a spike. The entity classifier alone took significant effort for 4-class classification at 75.8% accuracy. I'd recommend the spike targets **two outputs**:

1. **Broad category routing** (format / entity / numeric / temporal / text / geographic) — this directly replaces T0→T1 and is the highest-value signal
2. **Entity subtype** (person / place / organisation / creative_work) — we already have 2,911 labelled SOTAB columns for this

Locale signal can come in Phase 2. It depends on training data we'd need to curate (column-level locale labels don't exist in SOTAB/GitTables), and the CLDR infrastructure is ready whenever we are. Confidence calibration is a training hyperparameter, not a spike priority.

### 2. Rule Reduction: Be Honest About What Survives

The brief suggests reducing 18 rules to ~5-8 with Sense absorbing Rules 14-18. Let me be specific about what can and cannot be absorbed:

**Sense can absorb (3 rules):**
- Rule 18 (entity demotion) — entity subtyping moves into Sense
- Rule 14 (duration override) — better T0 routing means SEDOL won't reach duration values
- Rule 17 (UTC offset override) — better temporal routing distinguishes offset from time

**These stay regardless (value-level decisions Sense can't make):**
- Rules 1-3 (date/coordinate disambiguation) — US vs EU slash requires value parsing, not column semantics
- Rules 4-8 (IPv4, day/month detection, boolean, gender) — pattern-based, already fast and correct
- Rule 12 (numeric type disambiguation) — multi-signal value analysis (sequential pattern, port clustering, year range)
- Rule 15 (attractor demotion) — validation failure is a value-level signal, orthogonal to Sense
- Rule 16 (text length demotion) — value-level measurement

**Realistic expectation: 18 → 12-14 rules**, but the remaining rules are simpler because they operate in a narrower scope (no more "Rule X must run before Rule Y or the cascade breaks"). The real win is **reduced interaction complexity**, not rule count.

### 3. Taxonomy Changes Should Be Phased Separately

Mixing a taxonomy audit with a model architecture change doubles the risk. If the revised taxonomy *and* new model both perform differently, we can't attribute causation.

I recommend:
- **Phase 0 (now, independent):** Taxonomy audit. Collapse the 7 types the brief identifies. This immediately gives the existing CharCNN a cleaner label space — it might improve accuracy *before* the pivot. Re-baseline eval scores.
- **Phase 1 (spike):** Validate Sense architecture against the *current* taxonomy (or Phase 0 revised taxonomy). Don't add new types yet.
- **Phase 3+ (after integration):** Add new types (ticker, ISIN, flight_code etc.) one batch at a time with eval coverage.

### 4. Spike Timeline: Budget 1 Week, Not 2-3 Days

The brief suggests 2-3 days. That's too tight given:
- **Day 1-2:** Data curation — extracting column-level category + entity labels from SOTAB. The raw annotations exist but need transformation into (sampled_values, broad_category, entity_subtype) training tuples.
- **Day 3-4:** Model architecture + training. At least two variants per the brief's Open Question #1 (lightweight attention over Model2Vec embeddings vs small encoder).
- **Day 5:** Evaluation against current system + speed benchmarks + FINDING.md.

Data curation is the longest pole. We need to go from "16,765 SOTAB columns with Schema.org type annotations" to "N columns with (sample_values, category_label)" — and the Schema.org → broad category mapping isn't trivial.

### 5. DuckDB Extension: Out of Scope, But Must Benefit

The DuckDB extension is excluded from this plan's scope, but the architecture must be designed so it benefits from every accuracy improvement.

The current extension (flat CharCNN, scalar `finetype()` function) is a starting point, not an end state. Hugh's insight is right: the extension could evolve toward higher-level analyst functions — `read_file()` that ingests CSV/JSON/XML with type inference baked in, `validate_table()` that checks data against a spec. These are closer to the Noon pillars than a bare scalar classifier.

**Design constraint for Sense & Sharpen:** The Sense model's inference interface must be embeddable — no assumption that it only runs in the CLI. If the DuckDB extension adopts Sense later (for `read_file()` or `validate_table()` where per-column latency is acceptable), the code should support that without refactoring. Concretely:
- Sense model weights should be embeddable via `include_bytes!` (same as current CharCNN)
- The `ValueClassifier` trait or a new `ColumnSensor` trait should be usable from both CLI and extension contexts
- Column sampling should work on DuckDB vectors, not just Rust `Vec<String>`

DuckDB extension redesign is a separate future workstream. This plan just ensures we don't lock it out.

### 6. Column Sampling Strategy: A Concrete Proposal

The brief's Open Question #2 (how many sampled values?) has a practical answer from our entity classifier experience:

- **Observation #7097:** 5 airport name values triggered wrong prediction via header hints; 80 values gave correct entity_name prediction via disambiguation rule. Sample size matters enormously.
- **Entity classifier:** Uses up to 20 values. Works for the 4-class task but may miss rare patterns.

**Proposal:** Sample **50 values** using stratified sampling (top-K by frequency weighted toward diverse values, not just most common). This gives:
- Enough statistical signal for entity subtyping (proven at 20, better at 50)
- Locality signal from character distribution (sufficient for date ordering)
- Fast enough for transformer inference (~10ms for 50 embeddings through a small model)

Test 20 vs 50 in the spike, but design for 50 as the default.

### 7. Header Signal Integration: Into Sense, Not Separate

Open Question #4 has a clear answer: **column name should be an input to Sense**, not a separate post-classification system. This is one of the strongest architectural arguments for the pivot — the current system has a semantic hint classifier that runs *after* classification and tries to override it, with geography protection guards and entity demotion guards to prevent bad overrides.

Making the column name an input to Sense means:
- "company_name" biases toward organisation *during* classification, not after
- "latitude" biases toward geography *during* classification, not via a guard
- The header hint overrides, geography protection, entity demotion guard — all of that complexity collapses

This is where the most rule complexity dies. Not in removing individual rules, but in making header signal and model prediction a single unified decision.

---

## Recommended Phasing

### Phase 0: Taxonomy Audit & Simplification (1-2 days)
- Audit all 171 types against collapse/expand criteria from the brief
- Create revised taxonomy YAML set (target: ~160 types after collapsing ~7-10 niche types)
- Update eval schema mappings (SOTAB, profile)
- Re-baseline all eval scores
- **Value:** Cleaner label space for existing CharCNN, immediate benefit regardless of pivot

### Phase 1: Sense Model Spike (1 week)
- Build column-level transformer prototype (Python/PyTorch)
- Two outputs: broad category routing + entity subtype
- Column name as input feature (not separate)
- Sample 50 values per column, test 20 vs 50
- Train on SOTAB columns + profile eval datasets
- Test two architectures:
  - **A.** Lightweight attention over Model2Vec value embeddings (fast, minimal)
  - **B.** Small transformer encoder over character sequences (powerful, slower)
- Evaluate against current T0 routing + entity classifier
- Benchmark speed at column level
- Produce FINDING.md with go/no-go recommendation

**Go criteria (refined):**
- Broad category accuracy > 95% (current T0 is ~98% but simpler task; new categories are harder)
- Entity subtype accuracy > 78% (exceeds current 75.8%)
- Column inference < 50ms for 50 sampled values
- Clear path to Candle/Rust implementation

### Phase 2: Integration Design (2-3 days)
- Design Sense → Sharpen interface (which CharCNN models does each broad category invoke?)
- Map surviving disambiguation rules to the new pipeline
- Design unified ColumnClassifier that composes Sense + scoped CharCNN + rules
- Plan Candle implementation of the winning Sense architecture
- **Decision checkpoint before implementation**

### Phase 3: Rust Implementation (1-2 weeks)
- Implement Sense model inference in Candle (Rust)
- Wire into ColumnClassifier as pre-processing stage
- Scope CharCNN invocation based on Sense output
- Simplify surviving disambiguation rules (narrower scope = simpler logic)
- Update evaluation, re-baseline all scores

### Phase 4: Expansion & Polish (ongoing)
- Taxonomy expansion (new types: ticker, ISIN, flight_code, etc.)
- CLDR locale integration into Sense stage
- Profile eval expansion to 200+ columns
- Documentation overhaul

### Future: DuckDB Extension Redesign (separate workstream)
- Adopt Sense model for column-level functions (`read_file`, `validate_table`)
- Evaluate higher-level analyst functions beyond scalar `finetype()`
- The Sense & Sharpen architecture is designed to be embeddable; this workstream picks it up when ready

---

## Risk Mitigation

| Risk | Mitigation |
|---|---|
| Sense model underperforms on spike | Keep current architecture; Phase 0 taxonomy cleanup still delivers value |
| Training data insufficient | Start with SOTAB entity columns (2,911) + synthetic category labels; expand incrementally |
| Transformer too slow | Architecture A (attention over Model2Vec embeddings) is fast; fall back to sampling fewer values |
| Regression on format types | CharCNN retained for all format types; Sense only *routes*, doesn't replace |
| Two-model coordination complexity | Clean interface design in Phase 2; single ColumnClassifier owns the pipeline |
| DuckDB extension locked out | Embeddable Sense interface (include_bytes!, trait-based); extension redesign is a future workstream |

---

## What I'd Start Tomorrow

If you approve the direction, the immediate next step is:

1. **Create a backlog task for Phase 0** (taxonomy audit) — this is independent and delivers value now
2. **Create a backlog task for Phase 1** (Sense model spike) — time-boxed at 1 week
3. **Record the architectural decision** — decision-004: Sense & Sharpen pivot rationale

Phase 0 can run in parallel with Phase 1 data curation. The taxonomy audit informs the Sense model's category labels but doesn't block the spike.
