# Discovery: Distilled-data relabelling for 7 bad-distilled types (v17)

**Date:** 2026-04-20
**Interviewer:** Nightingale
**Mode:** discovery
**Related:**
- Decision 0049 — preserve synthetic for 7 bad-distilled types
- `.orbit/specs/2026-04-18-v16-data-audit-retrain/training-audit.md` — per-row triage of ~38 mislabeled distilled rows
- `.orbit/specs/2026-04-20-v16-n1-email-regression/interview.md` — bundled fix target
- `.orbit/specs/2026-04-20-v16-release/handover.md` — v0.6.17 shipped with narrow email regression

---

## Context

v16 shipped at 235/242 (97.1%) on the corrected profile eval, with 7 known
error columns. Separately, decision 0049 documented that we had dropped
distilled rows for 7 types (swift_bic, http_method, cpt, loinc, excel_format,
ssn, user_agent) because the distillation pipeline had produced genuinely
mislabeled data (e.g., "ROSEDALE" labeled swift_bic, "GOAT" labeled
http_method). We retained the synthetic generators for those types. The
follow-up work — actually getting real distilled data for the 7 types — was
explicitly deferred as a separate card.

This discovery session scopes that card as sprint m-19.

## Q&A

### Q1: Nature of the work
**Q:** What is this sprint really — relabeling existing mislabeled rows, or
sourcing new real data for the 7 types?
**A:** We don't know yet — triage IS the work. (Later reframed: see Q4.)

### Q2: Prior investigation
**Q:** Have we already triaged the distilled data per row?
**A:** "Didn't we already do this review? I found
`.orbit/specs/2026-04-18-v16-data-audit-retrain/eval-audit.md`."
Verified: eval-audit.md is about ground-truth eval labels, not distilled
training data. The per-row triage lives in `training-audit.md` (same spec
dir), which enumerates ~38 mislabeled rows across swift_bic, http_method,
excel_format, loinc, cpt. SSN and user_agent referenced in decision 0049
prose but not enumerated.

### Q3: Release coupling
**Q:** Does this ship as its own release, bundled with N=1 email, or as a
post-v17 patch?
**A:** Bundle with N=1 email fix — one retrain covers both.

### Q4: True goal (reframe)
**Q:** Is the goal to relabel the ~45 known-bad rows, or to source new real
distilled data for the 7 types?
**A:** Source **new real distilled data** for the 7 types. The ~45-row
relabel has marginal ROI — rows would redistribute to catch-all types and
the eval wouldn't move. Real coverage at scale is the actual fix.

### Q5: Validation target
**Q:** What counts as a successful v17?
**A:** Match or beat v16 (235/242). Any net regression blocks promotion.
(Later softened to net-positive gate — see Q10.)

### Q6: Sourcing approach
**Q:** Where does the new distilled data come from?
**A (multi-select):**
- Official registries (CPT / LOINC / SWIFT / Excel)
- Web-scraped/open-data corpora (user_agent, http_method)
- Keep SSN synthetic-only (privacy/legal)

### Q7: Volume target
**Q:** How much data per type?
**A:** Full representative coverage (~1000+ per type). Exception carved out
for HTTP method — see Q11.

### Q8: Scope — bundle vs separate
**Q:** Bundle N=1 email fix, or keep separate?
**A:** Keep bundled — one retrain covers both.

### Q9: Integration path
**Q:** Where should the new distilled data live in the pipeline?
**A:** **Per-type source loaders.** Each of the 7 types gets its own loader
module (`cpt_loader.py`, `loinc_loader.py`, etc.). Easier to re-run a
single type without touching others; clearer ownership per source.

### Q10: Retraining strategy
**Q:** What retraining strategy for v17?
**A:** **3-seed sweep 42/43/44, 100 epochs, fresh.** Mirror v16 methodology.
Picks best-of-3. (Rollback criterion relaxed from "any regression" to
"net-positive gate only" — see Q13.)

### Q11: HTTP method — fixed-set handling
**Q:** HTTP method is ~10 canonical strings. How to reconcile with the
1000+ row target?
**A:** **Use an ENUM in the JSON Schema** — the validation branch picks it
up. Add the case variants (get/Get/GET/etc.) to the enum. HTTP method
doesn't need 1000 rows of distilled data; it needs the schema gate to
encode its closed set.

**Implication:** the validation branch gets stronger when we encode
closed-set types as enums, independent of the multi-branch classifier.

### Q12: ENUM pattern for other closed-set types?
**Q:** Does the ENUM approach extend to SWIFT BIC / LOINC / CPT / Excel
format tokens?
**A:** **No** — those cardinalities (11k BICs, 95k LOINC, 10k CPT) are too
big for a useful enum. Real distilled data is the right path for those.

### Q13: Rollback / eval gate
**Q:** What is the rollback criterion for v17?
**A:** **Net-positive gate only.** v17 total ≥ 235/242. Individual
regressions acceptable if offset by gains. Simpler, permits trade-offs that
on balance improve the model.

### Q14: Training-time validation gate
**Q:** Beyond the 3-seed sweep, what training-time validation gates the
retrain?
**A:** **Val accuracy ≥ v16 best (91.2%).** Reject checkpoints below
v16's peak val_acc. Automatic guard against catastrophic data-quality
regressions before profile eval runs.

### Q15: Licensing policy
**Q:** How do we handle registry licensing?
**A:** **Public/open only — document sources.** Only use registries with
clear public/open licenses (LOINC, HTTP logs, Excel docs, ICAO). Skip
restrictive ones (CPT is AMA-licensed — revisit at spec time if we can
obtain a license). Add `SOURCES.md` documenting every source, license, and
retrieval date.

---

## Summary

### Goal

Retrain sherlock to v17 using **real distilled data** for the 7 types with
known-bad distilled coverage (swift_bic, http_method, cpt, loinc,
excel_format, ssn, user_agent), sourcing ~1000+ rows per type from
official registries and open-data corpora. Bundle the N=1 email regression
fix into the same retrain. Publish v0.6.18.

### Constraints

- **Data sourcing**: public/open licenses only. Document every source in
  `SOURCES.md`.
- **SSN remains synthetic-only** — do not scrape real SSNs. Improve the
  existing generator if needed.
- **HTTP method**: exception to the 1000+ volume target. Encode as an ENUM
  in the JSON Schema so the validation branch handles it. Add case
  variants (get/Get/GET/…) to the enum.
- **Other closed-set types** (swift_bic/loinc/cpt/excel_format): real
  distilled data, not enums — cardinality too large for useful enums.
- **Integration**: per-type source loaders (one module per type) feeding a
  new distilled corpus. Do not patch `output/distillation-v3/` in place.
- **Retrain methodology**: 3-seed sweep (42/43/44), 100 epochs, fresh
  (not warm-started from v16).
- **Training gate**: reject checkpoints with val_acc < 91.2% (v16 best).
- **Eval gate (rollback criterion)**: net-positive — v17 total ≥ 235/242.
  Individual regressions acceptable if offset by gains.
- **Bundled scope**: N=1 email regression fixed via data-blend (more
  single-value email examples in training), not via a new sharpen rule.

### Success Criteria

1. `output/distillation-v4/` (or equivalent) contains new distilled data
   for 6 of the 7 types (all except SSN) at ~1000+ rows each, sourced from
   documented public registries/corpora.
2. HTTP method encoded as ENUM in the type's JSON Schema; case variants
   included; validation branch picks it up.
3. N=1 email regression resolved in column-mode at N=1.
4. v17 profile eval ≥ 235/242 (97.1%). Net-positive gate passed.
5. `SOURCES.md` documents every distilled source with license and
   retrieval date.
6. v0.6.18 released with 5-platform binaries and Homebrew tap updated.

### Decisions Surfaced

- **Per-type source loaders** over monolithic pipeline patch. Chose for
  ownership clarity and re-runnability (→ will become MADR 00XX).
- **ENUM-in-JSON-Schema for HTTP method** only. Other closed-set types are
  too large for enums to be useful (→ will become MADR 00XX).
- **Net-positive eval gate** over "zero regression" gate. Permits
  on-balance improvements (→ will become MADR 00XX).
- **SSN stays synthetic-only** on privacy/legal grounds. Improve the
  generator if needed (→ confirmed; already in decision 0049).
- **Bundle N=1 email** via data-blend, not a new R32 sharpen rule. Keeps
  the retire-rules-over-time direction (decisions 0038, 0048) intact.

### Open Questions (for /orb:spec)

- **Per-type registry specifics**: exact source URLs, formats, and license
  text for each of LOINC / SWIFT / Excel / CPT (if pursued) / ICAO
  user-agent / HTTP log corpus. Spec will enumerate.
- **SSN generator improvements**: what specifically improves vs the
  current generator? Format-variant coverage? Regional distribution? Spec
  to decide if any change is needed at all.
- **N=1 email data-blend recipe**: how many single-value email rows, what
  value distribution, how does it integrate with the 6-type sourcing run?
- **Distillation v4 directory layout**: CSV-per-type vs combined
  `sherlock_distilled.csv.gz` v4? How does
  `scripts/prepare_multibranch_data.py` consume it?
- **Sweep script**: adapt `scripts/sweep_v16.sh` → `sweep_v17.sh`, or
  parameterise?
- **Label remap**: any updates needed to `data/label_remap.json` to
  accommodate new distilled labels?

---

**Next step:** `/orb:spec` to generate a structured spec from this
discovery. Target ~10–12 acceptance criteria covering the 6 success
criteria above plus the open questions.

---

## Round 2 — follow-up after review-spec

Spec v1.0 was written and reviewed (see `review-spec-2026-04-20.md`).
Reviewer flagged 4 HIGH findings. This second round of discovery
addresses them.

### Q16: HTTP-method ENUM surfaces
**Q:** The ENUM lives on three surfaces — YAML schema,
CompiledValidator, and the learned validation branch. Which does this
spec actually change?
**A:** **All three — split ac-05 into three sub-ACs:**
- **Sub-AC-a (YAML)**: `labels/definitions_technology.yaml` L283–286
  updated so enum + pattern both accept case variants. Either add `(?i)`
  to pattern + expand enum to include lowercase/titlecase, or keep
  case-sensitive but enumerate all variants explicitly.
- **Sub-AC-b (validator)**: unit test in
  `crates/finetype-core/src/validator.rs` asserting
  `CompiledValidator::is_valid("Get")` → true and
  `is_valid("GOAT")` → false.
- **Sub-AC-c (validation branch cascade)**: documented consequence —
  retraining picks up a stronger pass-rate feature for http_method.
  No code change in `validation_features.rs`; the spec language stops
  calling the validation branch a "gate."

### Q17: N=1 email — stay bundled or split out?
**Q:** Multi-branch trains on columns sampled at 100 values. Adding
"single-value email rows" is under-specified and may be incoherent
with column-scoped training. Close the mechanism in this spec, or
split the email fix back out?
**A:** **Split email back out of this spec.** Keeps v17's retrain
blast radius small and focused on 7-type data sourcing. N=1 email
goes back to its own card
(`.orbit/specs/2026-04-20-v16-n1-email-regression/`) with a proper
discovery on mechanism (column-mode augmentation vs value-based rule
vs generator change).

**Spec impact:** Remove ac-06 from v1.1; remove N=1 email from goal
and constraints. Update release expectations — v17 does NOT claim
to fix N=1 email.

### Q18: Licensing strategy
**Q:** Only user_agent and http_method are unambiguously open. How
should we handle LOINC / SWIFT / Excel / CPT under restrictive
licenses?
**A:** **We're over-thinking this. Public datasets (GitHub/Kaggle)
or generators. No license-review gates, no registry scraping with
legal contortions.** If a type has a public dataset on Kaggle or a
GitHub mirror, use it. If not, improve the generator.

**Spec impact:** Drop the SOURCES.md license-review machinery.
Replace with a simple per-type sourcing table (dataset link OR
generator file). Drop the "public/open only — document sources"
constraint as currently phrased; replace with "public datasets OR
synthetic generators, not restricted registries."

### Q19: Per-type sourcing decision
**Q:** For each type, is the first-pass plan public dataset or
generator improvement?
**A (multi-select):**

```
| Type          | Path                                          |
|---------------|-----------------------------------------------|
| user_agent    | Public dataset (Kaggle/GitHub UA corpus)      |
| http_method   | ENUM only — no new rows                       |
| LOINC         | Public dataset (GitHub mirror or Kaggle)      |
| SWIFT BIC     | Improved generator                            |
| CPT           | Improved generator                            |
| Excel format  | Improved generator                            |
| SSN           | Improved generator (synthetic-only)           |
```

### Q20: Generator improvement bar
**Q:** For types falling back to "improved generator," what counts
as "improved"?
**A:** **More format variants + realistic distributions.** Audit
each generator against the specific v16 eval failure for that type;
add missing format variants (dashes/no-dashes/spaces), realistic
country/region distributions (SWIFT BIC country codes), edge cases
that appeared in real data. Quantified: aim for **≥1000 unique
values per generator**.

### Q21: Decision 0049 treatment
**Q:** Amend or supersede?
**A:** **Amend in place.** 0049's core thesis (keep synthetic
generators for the 7 types) is NOT reversed — we're layering on
top. Add a "date-modified" header + "Update 2026-04-2X" section to
`.orbit/choices/0049-preserve-synthetic-for-bad-distilled-types.md`.
Status stays `accepted`.

---

## Updated Summary (v1.1 intent)

### Goal (revised)

Retrain sherlock to v17 on a corpus that combines:
1. Existing v16 training data (with current synthetic generators)
2. Public-dataset distilled rows for **user_agent** and **LOINC**
3. Improved synthetic generators for **SWIFT BIC**, **CPT**,
   **Excel format**, and **SSN** (≥1000 unique values each, audited
   against v16 eval failures)
4. ENUM expansion in the YAML schema for **http_method** (case
   variants) — no new rows

N=1 email regression is **out of scope** for this spec — separate card.

Promotion gate: net-positive profile eval (v17 ≥ 235/242).
Release v0.6.18.

### Constraints (revised)

- **Sourcing policy**: public datasets (Kaggle / GitHub) OR
  synthetic generators. No restricted registry scraping.
- **http_method**: YAML schema only — enum + pattern expanded for
  case variants. No Kaggle corpus. No generator work.
- **SSN**: synthetic-only. Generator audited against v16
  `people_directory.ssn` failure; improvements documented in the
  generator module.
- **Decision 0049**: amended in place, not superseded. Status
  remains `accepted`.
- **N=1 email**: OUT OF SCOPE. Tracked separately.
- **Training gate (revised)**: reject val_acc < 88% (catastrophic
  floor). 88–91.2% triggers manual review, not automatic rejection.
  Rationale: new training corpus makes strict v16-parity
  apples-to-oranges (reviewer finding #7).
- **Eval gate**: capture v16 baseline at training start via
  `eval/profile_eval.sh`; require v17 ≥ max(235/242, v16-at-start).
  Absorbs eval GT drift.
- Retrain methodology: 3-seed sweep (42/43/44), 100 epochs, fresh.
  Budget: ~7.5h wall-clock on M1 Pro Metal. Overnight run.

### Acceptance-criteria shape (revised)

Roughly 13 ACs — 12 from v1.0 with substitutions:
- ac-05 → split into 5a (YAML), 5b (validator unit test), 5c (branch
  cascade doc)
- ac-06 (N=1 email smoke test) → **removed**
- ac-04 (label_remap "no broken chains") → strengthened with an
  explicit validator script requirement
- ac-09 (eval gate) → revised to max(235/242, v16-at-start)
- ac-07 (decision 0049) → "amended, status remains accepted"
- New AC: per-type sourcing table in `output/distillation-v4/SOURCES.md`
  mapping type → dataset-URL or generator-module
- New AC: generator improvements audited vs v16 eval failures; ≥1000
  unique values per generator (SWIFT BIC, CPT, Excel, SSN)
- ac-08 (training gate) → 88% floor, 88–91.2% manual review

**Next step:** `/orb:spec` to regenerate `spec.yaml` as v1.1
incorporating these changes. Re-run `/orb:review-spec` on v1.1 to
confirm the HIGH findings are closed.

