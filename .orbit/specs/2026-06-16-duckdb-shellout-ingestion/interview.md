# Discovery: universal free-stats + neural short-circuit (ac-04 design)

**Date:** 2026-06-17
**Interviewer:** Claude (Opus 4.8)
**Spec:** 2026-06-16-duckdb-shellout-ingestion (ac-04)
**Cards:** 0007-duckdb-sql-extension, 0014-profile-validate-precision
**Mode:** design (settle one open question before code)

---

## Context

ac-04 has two separable pieces, gated differently (per the task kickoff brief, t-00002e8a):

1. **Free-stats plumbing** — low risk, no behaviour change. The duckdb ingestion scan
   (`read_csv_input`, `crates/finetype-cli/src/profile_io.rs`) already materialises the full
   column. Add the full-column aggregates (`count(*)`, `approx_count_distinct`, `min`, `max`,
   increment signature) in the same SQL pass and thread them to the column-statistics rule layer
   for EVERY input (CSV / Parquet / table). Closes the deferred `column-statistics-lever`
   ac-05/ac-07.
2. **Neural short-circuit** — behaviour change. When a full-column stat is *decisive*, skip the
   neural Sense pass and emit the stat's label directly. The speed lever — and the risk.

The open question this design exists to settle: **how conservative must the "decisive stat"
predicate be to clear the corpus-honest gate?**

## Substrate already established

- **The shipped full-column-stat rules are demotion-only refutations.** `increment_substance_veto`
  and `binary_vocab_veto` (`crates/finetype-model/src/column/mod.rs:2816`) run *after* the neural
  pass and only move a label *down* when a full-column stat proves the model's specific claim
  false. `values_form_increment` (`column/mod.rs:308`) already returns `Some(true)/Some(false)/None`
  — the veto fires on `Some(false)`. The short-circuit is the *affirmative* mirror: it asserts a
  label and suppresses the model's vote. **Different risk class** — we have evidence the stats are
  trustworthy as a veto, none for running them instead of the model.
- **The origin probe** (`memory column-stats-separate-residual-gaps`, 2026-06-16): full-column
  cardinality / increment signatures separate the residual recall gaps the per-value model is
  structurally blind to — categorical-vs-alnum AUC 0.985, integer-vs-increment 0.972. Its own
  caveat: *"separability ceiling on gold, not shipped recall; must clear the corpus-honest gate
  (relocation) like any rule; increment gold support thin (n=1)."*
- **Categorical is a residual attractor** (`memory categorical-is-a-residual-category`):
  low-cardinality→categorical is a known corpus-scale over-emitter, so it is the *least* tautological
  candidate and carries the highest relocation risk.

## Q&A

### Q1: Do we have evidence skipping the neural path is safe?
**Q:** Is there direct evidence that a decisive-stat skip is safe?
**A:** No. Two adjacent pieces only — separability on gold (the probe, AUC), and the stats as a
*demotion referee* (the shipped vetoes, which always let the model vote first). Neither is the test
that matters: "stat alone, no neural pass" has never been measured against the corpus. A skip is the
riskiest shape of the four-failed-retrains failure mode — it throws away the second opinion that
would catch a relocation.

### Q2: Should the conservatism bar be chosen up front?
**Q:** Pick the skip semantics / decisive-stat scope / gate bar a priori?
**A:** No — those should emerge from studies, not be elicited. The conservatism threshold is a
boundary to be *found* by measurement.

### Q3: What is the north star this serves?
**Q:** What is success?
**A:** Unchanged: **98%+ accuracy at a very fast inference rate.** The short-circuit must serve both
— it is the fast path, and it must not cost accuracy.

### Q4: What is the accuracy pass-mark, and over what?
**Q:** Is 98% the precision bar on the captured slice, plus whole-corpus gold non-regression?
**A:** Yes. The skip must be ≥98% right *on the columns it fires on*, and separately the whole-corpus
gold headline must not regress.

---

## Summary

### Goal

Settle the conservatism of the neural short-circuit's "decisive stat" predicate **empirically**.
The short-circuit is a **carve-out, not a global bet**: it serves the slice of columns where a
full-column statistic is right ≥98% of the time — fast, from the stat, without waking the model —
and leaves everything else to the neural pass. High accuracy and fast inference from one mechanism.

### The study (the design's core deliverable — this is what settles the open question)

For each candidate decisive-stat predicate — **increment**, **exact-binary**,
**low-cardinality→categorical**, **high-cardinality→alphanumeric_id** — build the affirmative
predicate with a tunable strictness knob (e.g. increment's fill-ratio + near-unique thresholds;
binary's exact-{0,1} vs allow-strays; cardinality's distinct/rows cutoff). Sweep the knob loose→tight
and, at each setting, read:

1. **Slice precision** — precision of the asserted label, on gold, over *only the captured columns*.
   Must hold **≥ 0.98**.
2. **Relocation** — corpus-honest gate on the ac-01 stratified sample. Must be **GO** (blocking; the
   only relocation detector).
3. **Whole-corpus gold headline** — must not regress.
4. **Throughput** — captured-column fraction × neural-inference cost saved (the speed win).

**Ship rule per predicate:** ship the skip at the **loosest knob setting that holds slice precision
≥ 0.98 AND earns a corpus-honest GO AND does not regress gold.** Loosest-that-holds maximises the
captured fraction, hence the speed win. A predicate that cannot reach ≥0.98 at any setting without
relocating **does not get to skip** — at most it ships as a post-pass recall rule that leaves the
neural vote intact (the existing 0048/0096 veto-fallback pattern). The conservatism bar is whatever
the curve says it is.

### Constraints

- Reuse the shipped predicate code where it exists — the affirmative side of `values_form_increment`
  (`Some(true)`) and the exact-{0,1} domain test are already written and demotion-validated.
- Full-column stats are computed in the duckdb scan (free-stats plumbing), available to the rule
  layer for every input before the neural pass.
- Corpus-honest NO-GO is blocking (H05); a GO on a skip is necessary, never sufficient on its own —
  slice precision and gold non-regression are co-conditions.
- `categorical` is a residual attractor: expect it to be the predicate most likely to *fail* the
  ≥0.98 / GO bar and fall back to post-pass-rule-only. Do not special-case it to pass.

### Success Criteria

- Each candidate predicate has a measured **{knob → captured-count, slice precision, corpus-honest
  verdict, throughput}** curve.
- Every shipped skip: slice precision ≥ 0.98, corpus-honest GO, gold headline non-regressing.
- Measured throughput delta (with vs without short-circuit) recorded.
- The decisive-stat predicate's conservatism threshold is *derived from the curve*, not asserted.

### Decisions Surfaced

- **Conservatism emerges from a per-predicate sweep, not from author preference** (this session).
  Alternatives rejected: (a) pre-committing a skip-semantics / scope / gate-bar triple by fiat —
  the author cannot answer pre-study and shouldn't; (b) a single global accuracy bet — the
  short-circuit is a carve-out scoped to its high-precision slice.
- **The skip's pass-mark is per-slice precision ≥ 0.98 + whole-corpus gold non-regression + a
  corpus-honest GO** (author-confirmed). All three, blocking.
- **Free-stats plumbing and the short-circuit are gated separately** — plumbing is no-behaviour-change
  and ships on parity; only the skip carries the precision/relocation gate.

### Implementation Notes

- Affirmative reuse: `values_form_increment == Some(true)` → assert `representation.identifier.increment`;
  exact-{0,1} domain → `representation.boolean.binary`. Both predicates exist and are demote-validated.
- The free-stats SQL extends the existing single-query scan (`read_csv_input`); no second pass.
- The short-circuit fires *before* the neural pass, so it must consult the full-column stats the
  plumbing surfaces — build the plumbing first, then the skip on top.
- Throughput measurement wants a corpus slice with a known short-circuit-eligible fraction, so the
  speed win is reported as fraction-captured × per-column neural cost, not just a wall-clock number.

### Open Questions (intent-level)

- None blocking. The conservatism threshold is intentionally left to the study; that is the design's
  resolution, not an omission.

---

**Next step:** `/orb:spec` — turn this into ac-04 sub-criteria: (1) free-stats plumbing + parity
[code], (2) per-predicate sweep study producing the curve [observation], (3) ship eligible skips at
loosest-safe setting, record throughput [code + observation].
