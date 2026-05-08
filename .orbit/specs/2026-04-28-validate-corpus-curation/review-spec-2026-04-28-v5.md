# Spec Review

**Date:** 2026-04-28
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-28-validate-corpus-curation/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 |
| 2 — Assumption & failure | content signals (eval datasets, training-leakage firewall, taxonomy GT) + 1 MEDIUM finding in Pass 1 | 2 |
| 3 — Adversarial | not triggered (changes are additive, revertible per constraint #11; no cascading scope) | — |

## Findings

### [MEDIUM] AC-09's awk range expression is mechanically broken — verification returns 0 regardless of section content
**Category:** test-gap
**Pass:** 1
**Description:** AC-09 verification (lines 290-296) uses an awk range expression to extract the `## Iter-2 expected vs actual` section and count its rows:

```bash
awk '/^## Iter-2 expected vs actual/,/^## /' eval/eval_output/validate_corpus.md \
  | grep -cE '^\| [a-z][a-z0-9_]+ \|'
```

The range expression `/^## Iter-2 expected vs actual/,/^## /` terminates immediately because the **start line itself matches the end pattern** `^## `. Awk's range expressions are inclusive at both ends; when start and stop patterns overlap on the same line, the range is exactly one line long — the header line.

Direct empirical check (synthetic file with the exact section format from implementation_notes lines 405-410):

```
=== awk range output ===
## Iter-2 expected vs actual
=== count ===
0
```

The grep correctly counts 0 because the awk filter discarded the entire body of the section. This is the **same failure shape as v3's `$6/$14` awk-index defect and v4's `finetype taxonomy` exit-code defect**: a verification command that mechanically passes-or-fails regardless of the property it claims to assert. AC-09's gate has no signal — an iter-2 PR with the section present and complete will fail this check; an iter-2 PR that omits the section entirely will also fail this check; both look identical to the gate.

The defect persists regardless of section position: if the section is at the end of the file (the realistic case per implementation_notes step 7), the range expression still terminates on the start line because it self-matches.

**Evidence:**
- spec.yaml:290-296 — verification block.
- Synthetic-file empirical test (above) — 0 lines extracted from a well-formed section with 5 data rows.
- The dataset-slug regex `^\| [a-z][a-z0-9_]+ \|` itself is correct — verified against `nyc_taxi`, `gdelt_events`, `sp500_constituents`, `fifa_players`, `oecd_employment`; all 5 match. The defect is purely in the awk range, not in the grep.

**Recommendation:** Replace the awk range expression with a flag-based extraction that excludes the start line:

```bash
awk '/^## Iter-2 expected vs actual/{flag=1; next} /^## /{flag=0} flag' \
  eval/eval_output/validate_corpus.md \
  | grep -cE '^\| [a-z][a-z0-9_]+ \|'
```

Empirically this returns 5 (or N) for a section with N data rows, 0 for an absent section. Alternative: anchor the end pattern away from the start pattern, e.g. `/^## Iter-2 expected vs actual/,/^---$/` (if a horizontal rule precedes the next section), or simply `grep -A 50 '^## Iter-2 expected vs actual' file | grep -cE '^\| [a-z][a-z0-9_]+ \|'`. The flag-based form is cleanest and matches the spec's existing rigour for AC-06 and AC-08.

### [LOW] AC-08's verification block is unreconciled with the constraint #10 + exit_conditions gap-downgrade path
**Category:** constraint-conflict
**Pass:** 2
**Description:** AC-08's `verification` (lines 269-278) requires **both** grep checks to return non-zero counts:

```
grep -E '^\| format_diversity\s+\|\s+[1-9]' ...   # ≥1 line
grep -E '^\| code_vs_canonical\s+\|\s+[1-9]' ...   # ≥1 line
```

Constraint #10 (lines 53-70) and exit_conditions (lines 484-485) both contemplate a gap-downgrade path: "if after the 6th pick a target mechanism still shows 0, AC-08 is downgraded to a documented gap and a follow-up card is created — the iteration ships rather than spinning indefinitely." Exit_conditions echoes this: "ac-08 satisfied: format_diversity ≥1 AND code_vs_canonical ≥1 ... OR documented as a gap per constraint #10's pre-merge escape-hatch fallback."

But AC-08's verification block knows nothing about the gap path. A reviewer running AC-08's verification when the escape hatch fired (FD or CvC still at 0 after the 6th pick) will get a failed grep and no signal that this is the documented-gap branch. The implementer may either:

(a) skip AC-08 entirely (loses traceability),
(b) hand-edit the report to fake a count (corrupts data),
(c) bounce on which gate to satisfy.

This is a coherence gap between three places that all describe AC-08's exit condition: the AC's own verification, constraint #10, and exit_conditions. The verification block is the load-bearing one (reviewers actually run it); the prose-only paths in constraint #10 / exit_conditions don't show up in mechanical gates.

**Evidence:**
- spec.yaml:262-278 — AC-08 description + verification (mandatory ≥1 in both mechanisms).
- spec.yaml:53-70 — constraint #10 (escape-hatch + downgrade clause).
- spec.yaml:484-485 — exit_conditions (gap path explicitly named).
- The three places agree in prose; only AC-08's machine gate doesn't reflect the disjunction.

**Recommendation:** Fork AC-08's verification to acknowledge the gap path. Two viable shapes:

(a) Add a third allowed condition — "OR a follow-up card exists at `.orbit/cards/00NN-validate-corpus-iter3.yaml` referencing the iter-2 gap":
```bash
fd_hit=$(grep -cE '^\| format_diversity\s+\|\s+[1-9]' eval/eval_output/validate_corpus.md)
cvc_hit=$(grep -cE '^\| code_vs_canonical\s+\|\s+[1-9]' eval/eval_output/validate_corpus.md)
gap_card=$(ls .orbit/cards/*validate-corpus-iter3* 2>/dev/null | wc -l)
if [ "$fd_hit" -ge 1 ] && [ "$cvc_hit" -ge 1 ]; then
  echo "AC-08 satisfied: both mechanisms ≥1"; exit 0
elif [ "$gap_card" -ge 1 ]; then
  echo "AC-08 satisfied via gap-downgrade: follow-up card filed"; exit 0
else
  echo "AC-08 FAIL: neither both mechanisms ≥1 nor gap-card present"; exit 1
fi
```

(b) Move the gap-path acknowledgement to a per-AC `notes:` field (if the spec schema allows) so the verification stays the strict ≥1 form and the implementer is explicitly directed to the gap path in the same AC. (a) is more rigorous; (b) is lighter. Either closes the coherence gap.

### [LOW] AC-02 verification has a stale rationale comment about `finetype taxonomy` exit codes
**Category:** missing-requirement
**Pass:** 2
**Description:** AC-02 verification (lines 122-124) carries a parenthetical comment:

> `# finetype taxonomy can exit 0 on unknown labels with a valid` <br>
> `# prefix — content-grep on stderr is the load-bearing check`

This is empirically wrong on the v0.6.19 binary. Direct invocation:

```
$ finetype taxonomy "fake.nonexistent.label" -o json-schema; echo EXIT=$?
Error: unknown type 'fake.nonexistent.label'

Did you mean:
  finance.rate.yield
  ...
EXIT=1
```

The exit code is **1**, not 0, for unknown labels. The comment originates from the v4 review (which empirically claimed exit 0). The grep is still correct as a defensive content-check (it doesn't rely on exit codes — note the `|| true`), but the rationale embedded in the spec is misleading. A future maintainer reading the spec will look at this comment, run the binary, see exit 1, and conclude either (a) the spec is buggy or (b) the binary's behaviour changed and the grep is now redundant.

**Evidence:**
- spec.yaml:122-124 — the parenthetical comment.
- Direct empirical: `finetype taxonomy "fake.nonexistent.label" -o json-schema` exits 1 (verified on v0.6.19).
- v4 review-spec-2026-04-28-v4.md:25 — claimed "exit code 0" for unknown labels; this v5 finding contradicts that empirically.

**Recommendation:** Update the comment to reflect actual behaviour:

> `# finetype taxonomy exits 1 on unknown labels (verified v0.6.19)` <br>
> `# but content-grep on the error marker is the defence-in-depth check` <br>
> `# in case the exit-code contract changes — same shape as a typo guard.`

Or remove the comment entirely if the spec wants to stay agnostic about exit-code semantics. Either way, the grep itself is correct and well-written; only the rationale needs updating. Trivial textual fix.

---

## Honest Assessment

V5 absorbs all four v4 findings cleanly:

- **AC-02 taxonomy-label check** — content-grep on `^Error: unknown type` is correct and works regardless of exit-code semantics. Defensive against future binary changes.
- **AC-02 cardinality invariant** — pinned in committed `scripts/check_validate_gt.sh` with explicit Bash. No more pseudo-code.
- **AC-04 role counts** — narrative updated to 36; verification uses `^    role: validate` indentation to filter the comment-line. Both fixes correct.
- **Licence strings** — SPDX-strict identifiers (`CC0-1.0`, `CC-BY-SA-4.0`) used throughout impl notes with explicit allowlist-mismatch warnings for the bare forms.

The remaining concerns are narrow:

1. **AC-09's awk range expression is broken** — the verification mechanically returns 0 regardless of whether the mismatch table is present and complete. This is the **same shape as v3/v4's running-defect class**: a verification command that doesn't verify the property it claims to assert. One-line awk fix closes it. This is the load-bearing finding because AC-09 is documented as the mechanism-mismatch reporting gate, but as written the gate has no signal. Same severity-shape as v3's `$6→$14` and v4's exit-code defect — a regression of the same class of bug, in a new location.

2. **AC-08's gate doesn't reconcile with the gap-downgrade path** in constraint #10 / exit_conditions. The escape-hatch is articulated in prose three places but not in AC-08's mechanical verification — leaving the implementer guessing which gate to satisfy when the 0-count case fires.

3. **AC-02 has a stale comment** about `finetype taxonomy` exit codes (carried over from v4's incorrect empirical claim). Cosmetic but misleading.

None of these rise to design rework. All are textual or one-line-invocation fixes within the spec — likely a 10-line revision. The dataset selection (S&P 500 over NASDAQ tickers; the GICS-Sector-as-sole-CvC-slot framing), mismatch policy, escape-hatch shape, exit-conditions alignment, AC-02 cardinality + label gates, AC-04 role-counts, and licence-string discipline are all sound.

The biggest remaining risk is shipping iter-2 with AC-09's verification gate broken — the mismatch table could be present and well-formed (passing AC-09 in spirit) but failing the mechanical check; or absent entirely and failing the same way. The gate carries no information either way. This is exactly the class of bug v3 and v4 also flagged in different ACs; a fourth cycle to close one more instance is consistent with how this spec has converged.

The drive.yaml shows this is a §5a synthetic BLOCK at `review_cycles.review_spec == 3`, with cycle 4 launched as Hugh's explicit override after v3's finding cluster. This v5 review identifies one new MEDIUM finding of the same class as the prior cycles — an awk-mechanic defect. A short v6 cycle to fix the awk and the two LOWs would be the clean exit; alternatively, given the BUDGET_OVERRIDE history and the narrowness of the findings, Hugh may choose to APPROVE with these noted as drive-time fixes during implementation. The verdict here is REQUEST_CHANGES because the AC-09 defect is mechanical and identical in shape to defects this review process has already caught twice; it deserves a textual fix before the spec freezes for implement.
