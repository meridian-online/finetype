# Discovery: remove-header-hints

**Date:** 2026-04-24
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml (parent)
**Mode:** discovery

---

## Context

MADR 0042 (accepted 2026-03-24) directed FineType to remove regex header
hints in favour of learned approaches (multi-branch header branch,
Model2Vec semantic matching, sibling-context attention). The decision was
never executed. Since 0042:

- MADR 0034 (2026-03-13) removed one hint (`id` / `identifier`)
- MADR 0048 (2026-04-18) pivoted to "value-rules only, no new
  header-rules" — reiterated 0042's principle but stopped short of
  removal
- MADR 0065 (2026-04-24, 8 hours before this discovery) added **11 more**
  exact-match arms to `header_hint()` to patch the amount-variant collapse

Evidence from 0065 confirmed the rule-based header system is the mechanism
behind subtype collapse: the substring matcher at `column.rs:4303-4314`
over-generalised on `amount`, forcing every variant header back to plain
amount via Sharpen hint-override branches. Hugh's framing: "header hints
are a bad pattern in my view."

**Surface audit (conducted during discovery):**

```
| Mechanism                              | Location                          | Notes                                       |
|----------------------------------------|-----------------------------------|---------------------------------------------|
| `header_hint()` match table            | column.rs:3999 (~185 lines)       | Canonical exact/near-exact arms             |
| Substring matchers (h.contains/h==)    | column.rs throughout (~70 sites)  | The mechanism that caused 0065 collapse     |
| `header_hint_measurement` branch | column.rs:979, 2183 | Scientific-unit override |
| `header_hint_location` / `_keep`       | column.rs:1053                    | Geo rescue/preservation                     |
| `header_hint_geo_override`             | column.rs:1558                    | Geo promotion branch                        |
| `header_hint_person_override`          | column.rs (tagged)                | Person-name override                        |
| `header_hint_sci_measurement`          | column.rs (tagged)                | Distinct from generic measurement           |
| `header_hint_cross_domain`             | column.rs                         | 0.85 threshold gate for Model2Vec           |
| `header_hint_same_category`            | column.rs                         | 0.95 threshold gate for Model2Vec           |
| `header_hint_generic` / `_fallback`    | column.rs                         | Catch-all branches                          |
| `sense_geo_hint_override` / `_rescue`  | column.rs:1558, 1573              | Legacy Sense-path branches still on disk    |
| `--no-header-hint` CLI flag            | main.rs (infer/load/profile)      | One global kill-switch — implies a gate     |
```

The `header_hint()` match table is only ~30% of the surface. The remaining
~70% is branch-and-substring logic inside `apply_header_sharpen()` and
`disambiguate_categorical()`. **Model2Vec semantic hints** (in `semantic.rs`,
733 lines) are the learned layer MADR 0042 envisioned as the path forward —
they are **out of scope** for removal.

---

## Q&A

### Q1: Scope of removal
**Q:** Given the full inventory, what gets removed — the `header_hint()`
match table only, all rule-based pathways, or all rule-based pathways
minus the Model2Vec threshold gates?
**A:** Full + keep threshold gates. Delete `header_hint()`, all 12
`header_hint_*` branches, the h.contains/h== substring family, and the
legacy `sense_geo_*` branches. Preserve `header_hint_cross_domain` (0.85)
and `header_hint_same_category` (0.95) — those are scoring gates for
Model2Vec, not hardcoded rules. Model2Vec semantic matching stays.

### Q2: Staging
**Q:** Per-domain sequenced, risk-classified batches, or big-bang?
**A:** Per-domain, sequenced. Each domain ships in its own PR with its
own regression gate.

### Q3: Regression floor
**Q:** Zero label regression, domain-correct floor, or bounded net delta?
**A:** Domain-correct floor. Label may regress during transition; domain
correctness must hold (296/352 domain on expanded eval is the anchor).
Specific-type churn acceptable as long as downstream SQL casting still
works.

### Q4: First-domain ordering
**Q:** Which domain goes first? (datetime/technology/identity/finance-last)
**A:** Delegated to Nightingale. Recommended sequence recorded below.

### Q5: Treatment of the 11 amount-variant arms shipped in 0065
**Q:** Fortify training data first, remove with finance stage, or
grandfather indefinitely?
**A:** Fortify training data first, then remove. Add amount-variant
training rows (variant header × variant values) to the corpus. Retrain
(v19 per MADR 0066 hard gate). Remove arms only after v19 proves the
model can distinguish variants. Honours both MADR 0042 (mitigation clause:
"add to training data rather than re-adding rules") and MADR 0066 (3-seed
hard gate before promotion).

### Q6: Rollback policy when a stage regresses beyond Q3 floor
**Q:** Full revert + fortify, value-rules per 0048, or partial revert?
**A:** Full revert + training-data fortification. Matches MADR 0042
mitigation clause verbatim. No value-rule compensation (that's a separate
spec per MADR 0048 principles).

### Q7: Diagnostic arc before each stage
**Q:** Pre-measure every stage, measure once + remove per-stage, or
remove-and-observe?
**A:** Measure once, remove per-stage. The mechanism (rule-based matching
overrides learned signal) is already named. A per-stage ac-01..ac-04 arc
would be ritual, not evidence. One upfront measurement against the full
eval corpus identifies which hints fire, which are redundant with the
model, which are load-bearing. Produces a removal roadmap. Per-domain
PRs execute from the roadmap.

### Q8: Per-stage gate mechanics
**Q:** Full profile eval + domain floor, full + per-column diff, or
targeted + full eval?
**A:** Full profile eval + per-column diff. The v18 retrain precedent
(0062) shows aggregate numbers hide churn — per-column diff caught "8
fixes / 8 regressions net-zero" that the domain_delta=0 headline missed.
Threshold: `domain_delta ≥ 0`, per-domain regression ≤ 3 columns, every
regression enumerated in PR description.

### Q9: Sequencing relative to v19 retrain
**Q:** Serial after v19, parallel against v16, or defer to v19 discovery?
**A:** Defer to v19 discovery. v19 hasn't been discovered yet — its scope
determines whether parallel is safe. If v19 is narrow (amount-variant
fortification only), non-finance domains can ship against v16 in
parallel. If v19 broadens header-branch training, earlier stages need
revalidation — serial is safer. Record as an open question in the spec.

### Q10: `--no-header-hint` CLI flag
**Q:** Survive, repurpose to `--no-model2vec-hint`, or delete entirely?
**A:** Delete entirely. Once rule-based hints are gone, the flag controls
nothing. Model2Vec stays and is always on (MADR 0042: "header signal is
a learned model input"). A future diagnostic flag for Model2Vec is a
separate decision, not in scope.

---

## Summary

### Goal

Execute the deferred MADR 0042 direction. Remove all rule-based header-hint
pathways from FineType — `header_hint()` match table, 12 `header_hint_*`
branches, h.contains/h== substring matchers, legacy `sense_geo_*` branches,
and the `--no-header-hint` CLI flag — while preserving the Model2Vec
semantic-hint layer (the learned approach MADR 0042 endorsed) and its
threshold gates (0.85 cross-domain, 0.95 same-category).

### Constraints

1. **Per-domain staging.** One PR per domain. Each stage has its own
   regression gate. Seven stages total (datetime, finance, geography,
   identity, representation, technology, container).
2. **Domain-correct regression floor.** `domain_delta ≥ 0` against the
   448-row eval manifest. Per-domain regression ≤ 3 columns. Specific-type
   label churn acceptable if the domain is preserved.
3. **Full eval + per-column diff per stage.** Aggregate numbers insufficient;
   every regression must be enumerated in the PR description.
4. **Finance stage requires v19 retrain prerequisite.** The 11 amount-variant
   arms shipped in MADR 0065 cannot be removed until training-data
   fortification + v19 retrain proves the model can distinguish variants.
   v19 must pass MADR 0066 hard gate before the finance stage can ship.
5. **Full rollback on regression.** If a stage breaches the floor, full
   revert + training-data fortification. No value-rule compensation. No
   partial rollback.
6. **Upfront measurement before any stage.** Single diagnostic pass
   enumerates which hints fire on the eval corpus, classifies each as
   model-covered (safe to remove) or model-gap (requires training-data
   fortification first). Produces the per-domain removal roadmap.
7. **Model2Vec and its threshold gates remain.** Out of scope for removal.
8. **`--no-header-hint` CLI flag removed in the final stage.**

### Success Criteria

- All 7 domain stages shipped, each with passing regression gate.
- `header_hint()` function deleted.
- All 12 `header_hint_*` disambiguation-rule tags removed from the
  emitter (`apply_header_sharpen()` + helpers).
- `h.contains(...)` / `h == "..."` header-string-match family
  reduced to zero call sites in `column.rs`.
- `sense_geo_hint_override` and `sense_geo_rescue` branches deleted.
- `--no-header-hint` CLI flag deleted.
- Model2Vec semantic-hint layer intact and functioning as the header-signal
  fallback.
- MADR 0042 marked as executed (status update).
- MADR 0028 (hardcoded > Model2Vec priority) marked superseded — no hardcoded
  layer exists to take priority.
- v16 profile eval after removal: `domain_score ≥ 296/352` (no domain
  regression across all 7 stages cumulatively).

### Decisions Surfaced

- **Model2Vec stays as the learned header-signal fallback** — challenged
  during Q1; confirmed by Hugh. Reason: MADR 0042 literally names it as
  the path forward; removing it would break the "urn vs url" edge case
  and eliminate graceful degradation on novel headers.
- **Finance goes last, gated on v19 retrain.** The 11 arms shipped in
  MADR 0065 are not exempt from removal — they become the canonical test
  case for "training-data fortification + retrain" replacing hardcoded
  patches. This operationalises MADR 0067's "retrain IS the lever when
  the mechanism is model-capacity, not pipeline-glue" framing.
- **No value-rule fallback.** Q6 locked full revert rather than MADR 0048
  value-rule compensation. Value-rules remain a valid tool for separate
  specs but are not part of the rollback ladder here.
- **Diagnostic amortised across stages.** One upfront measurement
  produces the roadmap; per-stage diagnostic arcs are not warranted
  (mechanism is already named).

### Recommended Domain Sequence (Nightingale's proposal for the spec)

1. **technology (26 types)** — lowest risk. IP/URL/UUID are strongly
   covered by Model2Vec semantic similarity (see `semantic.rs:612-624`
   tests); v16 header branch strong. Small surface area. Proves the
   removal pattern.
2. **identity (33 types)** — email/phone/gender already have value-rule
   backup per MADR 0048 (R28 email_display, R29 phone_e164). Strong
   learned signal. Medium-low risk.
3. **geography (25 types)** — the `header_hint_location` /
   `header_hint_geo_override` family. lat/long vs depth-error is a known
   hard case but explicitly deferred to model improvements per 0048 —
   so no regret to remove the hint.
4. **representation (33 types)** — numeric_code, integer_number,
   percentage. Many epoch/measurement hints live here; most are
   patches for cases the v16 header branch handles correctly.
5. **datetime (84 types)** — largest surface but v16 datetime accuracy
   is very strong. Most hints are epoch/unix-timestamp disambiguators
   (similar numeric shape → header is distinguishing signal). Moderate
   risk; shipped later so the pattern is proven.
6. **container (11 types)** — smallest domain, low traffic. Quick stage.
7. **finance (28 types) — LAST.** v19 retrain prerequisite. Must not
   start until v19 is discovered, specced, shipped, and passing MADR 0066
   hard gate.

This order is a proposal; the spec phase may re-sequence based on the
upfront-measurement output (Constraint 6) if evidence contradicts it.

### Open Questions (for the spec)

1. **v19 sequencing.** Does v19 retrain happen before the first domain
   stage, or in parallel against v16? Depends on v19 scope — record as
   open in the spec, resolve in the v19 discovery.
2. **Upfront-measurement output format.** What does the roadmap look like?
   Likely a TSV with columns `hint_id | domain | types_affected |
   current_hit_count | model_top1_without_hint | classification
   (covered|gap)`. Exact schema locked in the spec.
3. **MADR updates.** Does this work produce a new MADR (e.g. 0068
   "execution log for 0042") or status updates to 0042 (accepted →
   executed) and 0028 (accepted → superseded)? Recommend status updates
   — 0042 already records the direction, this spec executes it.
4. **Sibling-context attention coupling.** `sibling_context.rs` uses
   header embeddings as input to cross-column attention. Does header-hint
   removal affect sibling-context behaviour? Upfront measurement should
   verify.
5. **Golden test updates.** `cli_golden.rs` and `semantic.rs:612` have
   tests that exercise `header_hint()` directly. These tests need to be
   updated or removed as part of each stage.
6. **MCP/DuckDB surface.** Do MCP tools or DuckDB scalar functions expose
   any header-hint-dependent behaviour? `finetype mcp` and
   `finetype_detail()` both go through the same pipeline — but confirm
   during the upfront measurement.

---

## Next Step

Run `/orb:spec .orbit/specs/2026-04-24-remove-header-hints/interview.md` to
crystallise this discovery into a structured specification with ACs for
the upfront measurement + each domain stage.
