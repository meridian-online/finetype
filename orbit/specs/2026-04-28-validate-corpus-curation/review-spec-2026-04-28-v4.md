# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 2 |
| 2 — Assumption & failure | content signals (eval datasets, training-leakage firewall, taxonomy GT) + 1 MEDIUM finding in Pass 1 | 2 |
| 3 — Adversarial | not triggered (changes are additive, revertible per constraint #11; no cascading scope) | — |

## Findings

### [MEDIUM] AC-02's taxonomy-label validity check is silently null — `finetype taxonomy <bad_label> -o json-schema` exits 0
**Category:** test-gap
**Pass:** 1
**Description:** AC-02's invariant (b) (lines 102-106) says: "every `expected_label` value resolves to a real taxonomy type via `finetype taxonomy <label> -o json-schema`". The verification (lines 111-115) restates: "For each `expected_label` value… `finetype taxonomy <label> -o json-schema` exits 0".

Empirical check on the v0.6.19 binary: `finetype taxonomy "fake.nonexistent.label" -o json-schema` prints `Error: unknown type 'fake.nonexistent.label'` to stdout/stderr followed by a "Did you mean…" suggestion list — and **returns exit code 0**. So a typo'd label like `geography.location.us_state` (the very example called out parenthetically as something this check should catch) will pass the gate exactly like a real label will.

This is the same failure shape as v3's AC-06 awk-index defect: a verification command that mechanically passes regardless of the underlying property it's claiming to assert. The "catches typos like `geography.location.us_state` mechanically before the harness runs" promise in AC-02 is structurally aspirational — under the verification command as written, typos go undetected.

**Evidence:**
- `finetype taxonomy "fake.nonexistent.label" -o json-schema` → stdout/stderr contains `Error: unknown type …`, exit code 0 (verified by direct invocation).
- `finetype taxonomy "geography.location.country_code" -o json-schema` → emits valid JSON Schema, exit code 0 (verified).
- Both paths return exit 0 — the exit code carries no signal about label validity.
- spec.yaml:104-106 promises this catches typos "mechanically before the harness runs"; the proposed mechanism cannot do that.

**Recommendation:** Replace the exit-code check with a content-based check. Two viable shapes:

(a) Grep stderr for the error marker:
```
out=$(finetype taxonomy "$label" -o json-schema 2>&1)
echo "$out" | grep -q '^Error: unknown type' && { echo "BAD label: $label"; exit 1; } || true
```

(b) Probe a structural property of the JSON Schema output (e.g., parses as JSON and contains `$schema`):
```
finetype taxonomy "$label" -o json-schema 2>/dev/null | jq -e '.[0]."$schema"' > /dev/null
```

Either works in `scripts/check_validate_gt_labels.sh`; (a) is simpler and matches the binary's actual contract. The spec should pin one shape so the committed script can't drift back to a null gate. This is a one-line invocation change in the AC-02 verification block — same severity-shape as v3's `$6→$14` fix.

### [LOW] AC-02's sidecar cardinality invariant is named but the verification gives only pseudo-code
**Category:** test-gap
**Pass:** 1
**Description:** AC-02 invariant (a) (lines 102-104) says "each sidecar's `columns:` map cardinality equals the CSV's header column count exactly". The verification (line 108-109) gives only `python -c 'import yaml; ...'` (literal ellipsis). The spec leaves the load-bearing check to implementer initiative without a concrete shape.

Compare to AC-06, which spent two iterations getting the awk column index right: the same mechanical rigour hasn't been applied to AC-02's invariant (a). Without a pinned invocation, an implementer might commit a check that loads only the first sidecar, or one that compares against `manifest.column_count` rather than the CSV's actual header — either of which would let the gate slip silently.

**Evidence:**
- spec.yaml:107-115 — verification block has explicit Bash for the file-count check (`find … | wc -l`) and explicit Bash for invariant (b) (the taxonomy check, modulo finding #1 above), but pseudo-code for the cardinality check.
- The pattern across the spec is otherwise good — most other ACs pin exact commands. This one is an outlier.

**Recommendation:** Pin a concrete one-liner in the AC-02 verification, e.g.:
```bash
for sidecar in eval/datasets/validate_corpus/{nyc_taxi,gdelt_events,fifa_players,sp500_constituents,oecd_employment}.gt.yaml; do
  csv="eval/datasets/validate_corpus/csv/$(basename "$sidecar" .gt.yaml).csv"
  csv_cols=$(head -1 "$csv" | awk -F',' '{print NF}')
  yaml_cols=$(python -c "import yaml,sys; print(len(yaml.safe_load(open('$sidecar'))['columns']))")
  [ "$csv_cols" -eq "$yaml_cols" ] || { echo "MISMATCH $sidecar: csv=$csv_cols yaml=$yaml_cols"; exit 1; }
done
```
or commit this as part of `scripts/check_validate_gt_labels.sh` (rename to `scripts/check_validate_gt.sh` since it's broader than labels). One fixed shape, one source of truth.

### [LOW] AC-04 says "35 role: eval entries"; actual count is 36
**Category:** missing-requirement
**Pass:** 2
**Description:** AC-04 description (lines 137-143) says: "The existing 35 role: eval entries and 7 role: validate entries from iter-1 are byte-unchanged." Direct count of `^    role: eval` in the current file returns **36**, not 35. The 7 role: validate count is correct.

This is a fact-check-against-tree drift, not a structural defect. The constraint "iter-1 byte-unchanged" is what matters; 35 vs 36 is a minor narrative inaccuracy. But: AC-04's verification command (line 147-148) reads "`grep -c 'role: validate' eval/datasets/sources.yaml` increases from 7 to 12" — that count is right against what's in the file (the 7 only counts `role: validate` matches; ignoring the comment-line "ac-02 of 2026-04-21" mentioned in the v3 review, the bare grep returns 8 because it matches the comment too — not 7). So a simple `grep -c 'role: validate'` returns 8 today, not 7. The spec's "increases from 7 to 12" implies a delta of 5; the actual gate is "increases by 5", which `grep -c` will report as 8→13.

**Evidence:**
- `grep -cE '^    role: eval' eval/datasets/sources.yaml` → 36.
- `grep -cE '^    role: validate' eval/datasets/sources.yaml` → 7.
- `grep -c 'role: validate' eval/datasets/sources.yaml` → 8 (one match is in a comment header at line 19).

**Recommendation:** Either (a) update AC-04's narrative to "the existing 36 role: eval entries and 7 role: validate entries"; or (b) tighten the verification grep to `grep -cE '^    role: validate'` so it matches the property-bearing lines and not the comment header. (b) is the better fix — it pins the gate to the actual data, not to the prose. Same change for the eval count if AC-04 is going to assert it.

### [LOW] GDELT licence string ambiguity vs SPDX allowlist
**Category:** missing-requirement
**Pass:** 2
**Description:** Implementation note 1 (line 313) describes GDELT events as "CC0 license". The licence allowlist (`eval/licence_allowlist.txt`) contains the SPDX identifier `CC0-1.0`, not bare `CC0`. AC-04's licence-allowlist constraint (line 19-22) requires "a licence in eval/licence_allowlist.txt". A manifest entry with `licence: CC0` will fail the allowlist check; one with `licence: CC0-1.0` will pass.

This is harmless if the implementer normalises to SPDX while sourcing — but the spec uses the loose form in the impl notes, which a literal-following implementer might paste verbatim into the manifest. Iter-1's manifest uses `public-domain` and `PDDL-1.0` — both real allowlist entries — so iter-1 sets a precedent for being SPDX-strict.

**Evidence:**
- spec.yaml:313 — "CC0 license".
- spec.yaml:319 — "CC0 or CC-BY-SA" for FIFA (FIFA: same risk).
- spec.yaml:325 — "PDDL-1.0 / CC-BY-SA" for S&P 500 (CC-BY-SA is also unallowlisted; allowlist has `CC-BY-SA-4.0` only).
- spec.yaml:332 — "CC-BY-4.0" for OECD (correct, in allowlist).
- eval/licence_allowlist.txt — SPDX entries: `CC0-1.0`, `CC-BY-4.0`, `CC-BY-SA-4.0`, etc. No bare `CC0` or `CC-BY-SA`.

**Recommendation:** Edit impl notes to use SPDX-strict identifiers throughout: `CC0-1.0` (GDELT, FIFA), `CC-BY-SA-4.0` (S&P 500 fallback). Adds zero scope; closes a literal-following pitfall. Ideally also a one-line gate in `scripts/check_validate_gt_labels.sh` (or a sibling) that asserts every iter-2 manifest licence value appears in `eval/licence_allowlist.txt` — though this is already covered structurally by the iter-1 ac-02 check from the eval-expansion spec, so probably overkill.

---

## Honest Assessment

V4 is close. The spec correctly absorbs all five v3 findings:
- AC-06 awk index ($14, not $6) — fixed, with explicit `--output` path also pinned.
- AC-10 firewall-test claim — fixed, and AC-05 now invokes the firewall directly so the test actually runs.
- AC-07 escape-hatch headline — fixed via `(12|13)` regex alternation.
- exit_conditions / constraint #10 alignment — fixed.

The remaining concerns are narrow but real:

1. **AC-02's taxonomy-label validity check is silently null** — same shape as v3's awk-index defect (a verification command that always passes regardless of truth value). One-line invocation change closes it. This is the load-bearing finding because AC-02 explicitly promises to catch typos like `geography.location.us_state` mechanically; today's command cannot.

2. **AC-02's sidecar cardinality invariant is described but not pinned** — the prose says what to check, the verification gives `python -c '...'` with literal ellipsis. Same rigour as the rest of the spec demands a pinned one-liner.

3. **AC-04's role-counts are slightly off the tree** — 35 should be 36, and the `grep -c 'role: validate'` form catches the comment header (7 vs 8). Either adjust the narrative or tighten the gate.

4. **Licence strings in impl notes use loose forms (`CC0`, `CC-BY-SA`)** that won't match the SPDX-strict allowlist (`CC0-1.0`, `CC-BY-SA-4.0`). One paste-from-impl-notes mistake bombs AC-04.

None of these rise to design rework. All are textual or one-line-invocation fixes within the spec — likely a 20-line revision. The dataset selection (S&P 500 over NASDAQ tickers; the GICS-Sector-as-sole-CvC-slot framing), mismatch policy, escape-hatch shape, and exit-conditions alignment are sound.

The biggest remaining risk is shipping iter-2 with AC-02's taxonomy check passing on typo'd labels (because exit code 0 is unconditional), and only discovering the mistakes when `make validate-corpus` errors out trying to load a label that doesn't exist. That's identical in shape to v3's quietly-passing realism gate — worth one more cycle to fix the same class of bug.
