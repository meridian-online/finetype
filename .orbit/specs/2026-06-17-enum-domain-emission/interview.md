# Design: enum-domain emission (choice 0102, patch increment)

**Date:** 2026-06-17
**Interviewer:** Claude (Opus 4.8)
**Choice:** 0102-categorical-is-an-enum-property (proposed)
**Mode:** design — settle 0102's open questions, scoped to a PATCH release
**Cards:** 0014-profile-validate-precision, 0002-semantic-type-detection

---

## Context

Choice 0102 reframes categorical from a competing semantic leaf to an orthogonal
enum-domain PROPERTY (the DuckDB ENUM model). The author set two hard constraints
for this pass: (1) deliver it as a **patch release**, not 0.7.0; (2) decide how
far enum detection goes now.

Grounding that made it patch-sized: the enum domain is ALREADY emitted —
`crates/finetype-cli/src/enum_emission.rs` writes a sorted `unique_values` →
JSON-Schema `enum`, but gated two ways that 0102 targets: by **label**
(`label_is_enum_eligible` → only `representation.discrete.categorical` +
`representation.boolean.{binary,initials,terms}`) and over the **100-value
sample**. The patch decouples emission from the label and drives it off
full-column structure. No model, gold, or eval change — additive output only.

## Q&A

### Q1: Enum eligibility — which columns get a domain emitted?
**A:** **Structural — any bounded column**, decoupled from the semantic label, with
a denylist where an enum is meaningless (continuous numeric, datetime,
quasi-unique IDs). The cardinality gate + denylist replaces the label gate as the
`enum_overfit` guard. **PLUS** (author): a low cardinality count is not
sufficient — add a **value-structure / similarity test**. Enums are *designed* to
look alike (alpha IDs, verbs, state codes, entity names); the distinct values of a
real enum are structurally cohesive. A low-cardinality column of heterogeneous
values is not a designed enum. Consider a similarity score across **characters or
semantics**.

### Q2: Open vs closed enums in the patch?
**A:** **Open detection only** — emit the OBSERVED bounded domain (value set +
distinct/rows). Validating against KNOWN closed dictionaries (ISO country/currency
codes, out-of-domain flagging) is a separate, larger feature — defer to its own
spec.

---

## Summary

### Goal

Ship enum-domain emission as a **patch**: for any column whose full-column value
structure is a bounded, cohesive domain, emit its observed enum domain as an
ADDITIVE profile output — regardless of the semantic label. Decouple enum-ness
from the categorical label (0102) without touching the model, gold, or eval.

### The enum predicate (the design core)

A column's domain is emitted as an enum when ALL hold, computed over the FULL
column (`ColumnScanStats`, ac-04a — this is its first consumer):

1. **Bounded cardinality** — `distinct ≤ cap` AND `distinct/rows ≤ ratio` (true
   full-column counts, not the sample; current sample cap is 32).
2. **Structural cohesion (NEW, author)** — the distinct values are similar enough
   to be a *designed* set. A **character-shape cohesion** score is the patch
   primary (deterministic, free): map each distinct value to a shape signature
   (length bucket + character-class pattern, e.g. `AA` for two uppercase, `Aaa…`
   for a capitalised word, `9+` for digits), then score concentration — fraction
   of distinct values sharing the dominant shape family, or inverse shape-entropy.
   High concentration ⇒ cohesive enum; heterogeneous shapes ⇒ not an enum.
   **Semantic cohesion** (Model2Vec embedding cluster-tightness over the distinct
   values) is noted as a FUTURE enhancement, not in the patch.
3. **Not denylisted** — exclude open-domain types where an enum is meaningless:
   continuous numeric (decimal/measurement/coordinate), datetime, and
   quasi-unique identifiers. (Full-column cardinality already excludes most of
   these via gate 1; the denylist is the explicit backstop the label gate used to
   provide.)

### Thresholds are a study output, not a fiat (evidence-driven)

`cap`, `ratio`, and the cohesion threshold are calibrated by a study against
labelled signal we already have: the gold categorical/boolean columns (positive
enum examples) vs the specific-type and free-text columns (negatives), measuring
how cleanly the predicate separates designed enums from accidental low cardinality
and from cohesive-but-open types (city/entity). Ship the loosest setting that
keeps enum precision high. No threshold is hand-picked.

### Output (additive, non-breaking)

Extend the existing `enum_emission` path. Minimum: populate `unique_values` (→
JSON-Schema `enum`) for the broadened eligible set. Preferred: a small `enum`
block — `{ open: true, domain: [...], distinct, rows, cohesion_score }` — emitted
alongside the semantic type, so the analyst gets meaning AND domain. Existing
consumers that ignore the new field are unaffected (patch-safe).

### Decisions surfaced

- **Enum eligibility is structural + cohesive, decoupled from the label**
  (Q1) — over modest-curated-list (keeps label coupling) and status-quo (declines
  0102).
- **Open detection only** (Q2) — closed-dictionary validation deferred.
- **Character-shape cohesion is the patch similarity metric**; semantic cohesion
  deferred. (Author surfaced the similarity requirement; char-shape is the
  cheapest faithful realisation.)
- **Thresholds emerge from a calibration study**, not author fiat.

### Explicitly deferred to 0.7.0 (the accuracy-reframe half of 0102)

- Eval scoring of a two-part answer (semantic type + enum property).
- Dropping `representation.discrete.categorical` as a neural/Sharpen target.
- Gold-fixture migration (categorical → semantic type + enum flag).
- The residual-label question (what carries "bounded, no semantic type").
- Closed-enum dictionaries + out-of-domain validation.

### Implementation notes

- Seam: `crates/finetype-cli/src/enum_emission.rs` (+ its `profile.rs` callers).
  Replace the label gate with the structural predicate; replace sample distinct
  with `ColumnScanStats` full-column distinct + a new cohesion fn.
- `ColumnScanStats` (`column/mod.rs`, f1e047d) gains its first consumer — the
  ac-04a plumbing pays off here, not in a skip/rule.
- Patch-safety: additive output, no label change, kill-switchable, golden-test the
  new emission shape.

### Open questions (intent-level)

- Exact `enum` block shape vs reusing `unique_values` only — a packaging call,
  settle in /spec.
- Cohesion metric details (shape alphabet, concentration vs entropy) — a study
  input, not a blocker.

---

**Next step:** `/orb:spec` — ac-00 calibration study (predicate + thresholds on
gold), ac-01 implement the structural+cohesion predicate over full-column stats,
ac-02 additive output + golden tests, ac-03 patch release notes. The 0.7.0
accuracy reframe is a separate future spec against choice 0102.
