# Spec Review

**Date:** 2026-04-21
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-21-v18-retrain/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (training data, model promotion, eval datasets, cross-workflow release mechanics) + MEDIUM finding in Pass 1 | 3 |
| 3 — Adversarial | not triggered — Pass 2 did not surface cascading structural defects | — |

---

## Findings

### [HIGH] AC-05 sibling-context verification targets a field that does not exist in the multi-branch `config.json`
**Category:** test-gap
**Pass:** 1
**Description:** AC-05 requires that each seed's `config.json` contain "a sibling-context-related key (e.g., `sibling_context_*` or `num_sibling_features > 0`)" and uses the illustrative verification `jq '.sibling_context_dim // .num_sibling_features // empty'` returning a non-null value. The existing multi-branch `config.json` (verified against `models/sherlock-v16/config.json`) has only the keys `char_dim, embed_dim, stats_dim, header_dim, char_hidden, embed_hidden, stats_hidden, header_hidden, valid_dim, valid_hidden, merge_hidden, n_classes, dropout, head_type, activation, use_layer_norm, type_index_keys` — no sibling-context key. Sibling-context information is carried in the **FTMB v3 data format** (`sibling_headers` vector inside each `TableGroup`, see `crates/finetype-train/src/multi_branch.rs:707`), and sibling-context is trained as a **separate model** (`crates/finetype-train/src/bin/train_sibling_context.rs`), not as a branch merged into the multi-branch model's config. As written, AC-05's sibling-context sub-clause is unsatisfiable without a training-infrastructure change — the v18 training script cannot produce that key because the model does not have one. The implementer will either add a spurious key purely to satisfy the check, quietly stub a `num_sibling_features: 0` and pass, or be blocked indefinitely.
**Evidence:** spec.yaml lines 109–130 (ac-05 description + verification); `models/sherlock-v16/config.json` key list (17 keys, none sibling); `crates/finetype-train/src/multi_branch.rs:707` (sibling_headers lives in the FTMB payload, not the model config); `crates/finetype-train/src/bin/train_sibling_context.rs` (separate binary for the sibling-context model).
**Recommendation:** Move the sibling-context verification off `config.json` and onto evidence that actually exists:
- Assert the prep log reports FTMB version ≥ 3 and a non-zero `n_sibling_headers` / `sibling_headers_populated` count across table groups.
- OR assert that a post-training smoke run of `finetype profile <csv>` logs that it took the `classify_columns_with_context` path (requires a tracing log line, may need a small code add).
- OR drop the config.json sub-clause and keep only the prep-log sibling-count assertion.
State the acceptable artefact explicitly — don't leave it as "a sibling-context-related key … e.g., `sibling_context_*`", which invites a placebo field.

### [MEDIUM] AC-05 file-count discrepancy — description lists 5 files, verification asserts "six"
**Category:** constraint-conflict
**Pass:** 1
**Description:** AC-05's description enumerates per-seed artefacts: "`results.json`, `epochs.jsonl`, `config.json` (including injected `type_index_keys`), `model.safetensors`, `eval/report.md`" — five items. The verification method then says "All three model directories exist with **all six** required files." There are five files listed. Existing v16/v17 model directories also contain `label_map.json` as a sixth canonical artefact (verified via `ls models/sherlock-v16/`), which is likely the intended sixth file — but it is not named in the description. An implementer running a literal check will either (a) pass on 5/5 and the verification wording is wrong, or (b) fail on 5/6 looking for an unnamed file. The ambiguity undermines the gate.
**Evidence:** spec.yaml lines 113–129 (description lists 5 artefacts; verification says "six required files"); `ls models/sherlock-v16/` shows 6 files including `label_map.json`.
**Recommendation:** Either add `label_map.json` to the description (matching v16/v17 convention) so the count lines up at six, or change "six required files" to "five required files" in the verification. Prefer the former, since `label_map.json` is load-bearing for CLI/DuckDB/MCP consumers.

### [MEDIUM] AC-07 per-domain regression check relies on a "7 domains" count that is undefined anywhere in the spec
**Category:** test-gap
**Pass:** 2
**Description:** AC-07 verification: "Python scriptable domain-regression check: for each of 7 domains, count columns correct-in-v16-wrong-in-v18; every count ≤ 3." "7 domains" is asserted but never defined in the spec. CLAUDE.md enumerates the 7 taxonomy domains (container, datetime, finance, geography, identity, representation, technology), but that's project context, not spec content — a reviewer or downstream automation reading the spec cold has no authoritative list. Worse, the expanded 352-col eval is dominated by a few large domains and sparsely populated in others: if the `geography` domain has (say) 12 eval columns, a ≤ 3 regression floor is a ~25% loss tolerance; if `container` has 4 columns, ≤ 3 is effectively "no floor at all" (a single non-regression is enough). The floor's effective strictness varies by up to an order of magnitude across domains, yet the constraint is flat.
**Evidence:** spec.yaml lines 12 (constraint), 148–158 (ac-07 verification) — no enumeration of domains, no per-domain column-count context. v16 diagnostic headline (297/352) is in the goal line but the per-domain distribution is not stated.
**Recommendation:** (a) Name the 7 domains explicitly in ac-07's description (or link the taxonomy domain list). (b) Pin per-domain v16 column counts as an output of the triage step (ac-01's per-domain summary table is the natural home — require it to include "v16 correct / v16 incorrect / total" per domain). (c) Consider whether the floor should scale — e.g., "regression ≤ 3 **or** ≤ 25% of that domain's v16-correct columns, whichever is smaller" — or state explicitly that a flat 3 is accepted as the chosen trade-off.

### [MEDIUM] Taxonomy-edit carve-out has no verification path and no downstream-consumer impact check
**Category:** missing-requirement
**Pass:** 2
**Description:** Constraint at spec.yaml line 14 permits taxonomy edits "ONLY if triage surfaces a genuine coverage gap (an expanded-eval type missing from the 240-type taxonomy)". Adding a taxonomy type is a cross-cutting change: it impacts `labels/definitions_*.yaml`, label remap, generators, the model's `n_classes`/`type_index_keys`, the DuckDB extension's type enum, MCP taxonomy resources, and external consumers pinned to the 240-type count. Nothing in the ACs verifies that a taxonomy edit, if taken, is applied consistently end-to-end, nor does any AC require a MADR entry for the taxonomy change (ac-03 covers only the **corpus base** MADR, and ac-08 covers only the sweep-discipline MADR). An implementer could legitimately add `identity.foo.bar`, train v18 on 241 classes, and the spec would be silent on whether the DuckDB extension, MCP taxonomy resource, and external schema consumers were updated.
**Evidence:** spec.yaml line 14 (constraint) — no linked AC; CLAUDE.md "Taxonomy structure" section + DuckDB/MCP integration notes show taxonomy is plumbed through multiple consumers.
**Recommendation:** Add a conditional AC: "If triage surfaces a coverage gap and taxonomy is edited, (a) a dedicated MADR records the new type and its downstream impact, (b) `cargo run -- check` passes post-edit, (c) the `n_classes` delta is reflected in the v18 model config and label_map." Or strengthen the constraint to "Taxonomy edits are out-of-scope for v18 — coverage-gap triage outputs go on the next-sprint backlog."

### [LOW] AC-04 sentinel arithmetic is robust but does not prevent a self-consistent log-only firewall stub
**Category:** test-gap
**Pass:** 2
**Description:** AC-04 requires the sentinel `pre_filter_rows − row_hash_overlap = post_filter_rows` plus the literal marker `hash_filter_active: true`. This is a strong improvement over a pure grep-exists check. However, the sentinel is purely arithmetic: a stub that computes `pre_filter = N`, logs `row_hash_overlap = 0`, logs `post_filter = N`, logs `hash_filter_active: true`, without ever loading `eval/row_hashes.tsv`, will pass. The only guard against this is the `leaked_rows_after_filter: 0` line, which is required **only if overlap was non-zero**. On a fresh run where the corpus genuinely has zero overlap with eval, the sentinel passes without any actual hashing occurring. This is a narrow but real attack surface against silent regression of the prep script.
**Evidence:** spec.yaml lines 87–107 (ac-04 description + verification); `scripts/eval_leakage/__init__.py` + `eval/row_hashes.tsv` (the real artefacts the firewall must read).
**Recommendation:** Require a log line recording the SHA256 of `eval/row_hashes.tsv` actually loaded by the prep run, OR require that `leaked_rows_after_filter` be logged unconditionally (zero-overlap runs should still re-hash post-filter rows as a defence-in-depth check). Either gives the gate a read-side signal in addition to the arithmetic sentinel.

### [LOW] AC-06 halt path under-specifies "investigation hypotheses"
**Category:** test-gap
**Pass:** 1
**Description:** AC-06 states: "If all three are `REJECT`, `progress.md` contains a 'Halt investigation' section with at least three investigation hypotheses before continuing." "Continuing" is undefined — continue to what? A retry sweep? Manual intervention? And "three investigation hypotheses" with no rubric means an implementer can land three one-line guesses ("maybe data", "maybe lr", "maybe model") and clear the gate. The intent — force a pause for diagnosis — is right; the verification is gameable.
**Evidence:** spec.yaml lines 131–143 (ac-06 description + verification).
**Recommendation:** Either (a) specify the schema of each hypothesis (e.g., "each hypothesis names: symptom, testable prediction, falsification step, estimated cost"), or (b) require that at least one hypothesis cite a specific prep-log field, results.json value, or label-remap entry as evidence. Also name the "continuing" gate — e.g., "before a retry sweep is authorised" vs "before the spec is marked halted".

### [LOW] Time / cost envelope is absent — spec has no wall-clock or compute budget
**Category:** missing-requirement
**Pass:** 2
**Description:** Nothing in the spec states how long a v18 sweep is expected to take, what constitutes "taking too long" (a kill/abort condition), or what machine the training is expected to run on. The v17 sweep header estimates "~2.5-3h per seed on M1 Pro with Metal" (`scripts/sweep_v17.sh`), so a 3-seed v18 sweep is plausibly ~7.5–9h wall-clock. If a sweep stalls at epoch 50/100 for seed 2/3 overnight, there is no gate that says "abort, investigate". For an autonomous-agent implementer running `/orb:drive` the absence of a time budget is a real tail-risk — the agent will not abort a runaway training run on its own.
**Evidence:** spec.yaml (no time budget anywhere); `scripts/sweep_v17.sh` lines 33–35 (v17 estimate).
**Recommendation:** Add a constraint or implementer note: "Per-seed wall-clock budget: ≤ 4h on M1 Pro Metal (soft), abort-and-investigate at 6h (hard). Total sweep wall-clock: ≤ 12h." This is low-stakes to get wrong but valuable to have committed.

---

## Honest Assessment

The spec is structurally sound and captures the right lessons — triage-before-sweep, fixed data seed, per-domain regression floor, explicit no-auto-promotion, leakage-firewall verification with a sentinel arithmetic check, pre-assigned MADR numbers to avoid race. The gate-AC verification fields are all substantive (well above the 20-char floor, none placeholder), and the separation of ac-07 (promotion gate) from ac-09 (outcome MADR) from ac-10 (release-scope decision) cleanly decouples "did the model earn promotion" from "are we shipping it publicly".

The blocker is AC-05's sibling-context sub-clause: it asks for a verification artefact (`sibling_context_*` key in `config.json`) that does not exist in the multi-branch model and cannot exist without an infrastructure change. This is a real bug, not a stylistic nit — the implementer will hit it during the first seed run and either fabricate a placebo key or be blocked. The sibling-context constraint (spec line 16) is important and worth keeping, but the verification has to move to evidence that actually exists (FTMB v3 sibling-header population in the prep log, or a tracing log line from `classify_columns_with_context`). The AC-05 five-vs-six file count discrepancy is a trivial editorial fix but reinforces that this AC needs a close read. AC-07's "7 domains" is a credibility issue for the promotion gate — the floor's effective strictness varies by an order of magnitude across domains, and the spec should own that trade-off explicitly rather than elide it. The remaining findings (taxonomy-edit carve-out without a verification path, AC-04 sentinel self-consistency, AC-06 hypotheses gameability, missing time budget) are polish — worth addressing in the same revision cycle but not individually blocking.

One round of REQUEST_CHANGES addressing at minimum the HIGH (ac-05 sibling-context) and both MEDIUMs (ac-05 file count, ac-07 domain definition) should clear APPROVE on the next pass.