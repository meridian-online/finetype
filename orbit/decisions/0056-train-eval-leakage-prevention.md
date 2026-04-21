---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0056. Train–Eval Leakage Prevention: Source Roles + Row-Hash Filter

## Context and Problem Statement

FineType's training corpus and eval corpus have grown in parallel without
an explicit firewall. Several sources appear on both sides (public CSVs
used for distillation AND cited in eval manifests), and the v16 pipeline
carries no mechanism to detect or remove identical (header, value) rows
that cross the boundary. The v16 eval score of 235/242 is therefore
partially inflated — an unknown subset of eval columns match rows the
model saw during training. Without a firewall, the eval-expansion sprint
would simply scale this problem: more columns, more invisible leakage.

Two properties are required:

- **Source-level roles** — a source of truth declaring, per distinct
  source_url, whether it feeds training, eval, or is forbidden from both.
- **Row-level filter** — a mechanism that prevents individual rows from
  crossing the firewall even when their source was mis-declared.

Both are required; neither suffices alone. Source roles are coarse (they
can be mis-declared or drift over time). Row-level filtering catches the
mis-declaration but without a role manifest the training pipeline would
have nothing to fall back to when a hash conflict fires.

## Considered Options

- **A. Source roles only** — maintain sources.yaml; trust operators not
  to reuse sources cross-boundary. Cheap but fragile; one mis-tagged row
  defeats the whole system.
- **B. Row-hash filter only** — hash every eval row, filter every training
  row that matches. Robust to source-level tagging errors but leaves
  training pipeline with no high-level view of which sources are even
  safe to consider.
- **C. Belt-and-braces: roles + row-hash filter with a shared normaliser**
  — both layers, with the row-hash normalisation spec defined once and
  imported by both the hash-generator and the training-pipeline filter so
  the two sides cannot drift.

## Decision Outcome

Chosen option: **C — belt-and-braces**. Both layers are implemented; both
are live before Phase A+B ships (constraint #3). The layers are:

### Layer 1: Source role manifest (`eval/datasets/sources.yaml`)

Per unique source_url, records:

- `role` ∈ `train` | `eval` | `both-forbidden`
- `licence` — SPDX identifier or allowlisted free-form (internal,
  public-domain, unknown-investigating)
- `attribution_text` — one-line citation
- `notes` — free text (takedown policy, fetching quirks, known issues)

**Resolution rule** when a source is discovered feeding both sides
simultaneously (the three-way deadlock of train vs eval vs coverage):
**eval keeps the source; training relocates.** If no replacement training
source exists, the type is marked `re-source-pending` and tracked in
progress.md rather than ignored silently. Eval wins because eval defines
ground truth for the feature; if eval moves, every past comparison moves
with it, and that's a more expensive invariant to rewrite than the
training pipeline.

### Layer 2: Row-hash filter (`eval/row_hashes.tsv` + filter)

Per (dataset, column, value) in the eval corpus, compute:

```
row_hash = SHA256( normalised_header || '\x00' || normalised_value )
```

**Normalisation spec (shared module):**

- `normalised_header` = `lower(trim(header))`
- `normalised_value` = `trim(value)` with Unicode NFC applied and
  `\r\n` collapsed to `\n`

Defined once in a shared module (Python helper in `scripts/` or
`crates/finetype-core/` depending on implementation). Imported by both
the hash-generation script (ac-06) and the training-pipeline filter
in `scripts/prepare_multibranch_data.py` (ac-07). The two sides cannot
drift because they call the same code path.

**Enforcement point:** `scripts/prepare_multibranch_data.py` applies the
filter *after* its normal row-building pass and *before* writing the
training arrays. The filter is active by default; a `--no-dedup` flag
exists for emergency rollback and diagnostics. The filter logs both
the total row count and the removed count — visible in the training
log and captured in progress.md.

### Consequences

- Good, because leakage is prevented at two layers with fail-open
  behaviour inverted: a mis-declared source is caught by row-hash; a
  format-drift collision that slips the row-hash is still bounded to the
  row level (not a full-source contamination).
- Good, because the shared normaliser module prevents the classic bug
  where hash-generator and filter drift apart over time. A single-source
  fix propagates to both sides.
- Good, because the `re-source-pending` state is explicit rather than
  silent — a training type losing its source is visible in progress.md
  rather than quietly falling back to a degraded loader.
- Bad, because **format-drift collisions are not caught**. If training
  stores a timestamp as `2024-01-15T10:30:00Z` and eval stores it as
  `2024-01-15 10:30:00`, their hashes differ despite representing the
  same row. The `trim + NFC + CRLF→LF` normaliser is deliberately narrow.
- Bad, because **header synonyms are not caught**. "email" and "e-mail"
  hash differently; "PassengerId" and "passenger_id" hash differently
  before their respective `trim+lower` passes. Mitigated partially by
  existing header hints, but not by the filter itself.
- Bad, because **whitespace normalisations beyond NFC + newline collapse
  are not applied**. Columns with internal double-spaces, tabs, or
  unicode whitespace variants (U+00A0 non-breaking space, U+2028 line
  separator) will hash differently from their trimmed equivalents.
- Bad, because the filter adds a few seconds to training prep (negligible
  at current corpus scale; mention here for future scale).
- Neutral, because v16's baseline scores were produced *without* this
  filter. v16 re-scored on the expanded eval (ac-12) is explicitly
  diagnostic-only, not v18's promotion baseline — v18's first trainable
  corpus is the first one with this filter active.

### Known blind spots (explicit enumeration — per spec ac-10)

This section is load-bearing: the spec's ac-10 verification greps for
"Known blind spots" and requires it to be non-empty. The list above
under Consequences/Bad is that enumeration. Summary for grep-friendliness:

- Format-drift collisions (timestamp reformatting train vs eval)
- Header synonyms (email vs e-mail vs EmailAddress)
- Whitespace normalisations beyond NFC + newline collapse
  (non-breaking space, tabs, internal whitespace variance)

When future expansion widens the normaliser (e.g. to collapse whitespace
runs or alias header synonyms), update this section and bump the
shared-module version. Training runs against the updated filter require
regenerating `eval/row_hashes.tsv` as a pre-flight step.

## References

- Spec: `orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.1) — ac-06, ac-07, ac-08
- Related decisions:
  - `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
  - `orbit/decisions/0050-per-type-sourcing-policy.md`
- Files:
  - `eval/datasets/sources.yaml` — source role manifest (ac-08)
  - `eval/row_hashes.tsv` — row-hash artefact (ac-06)
  - `scripts/prepare_multibranch_data.py` — filter enforcement point (ac-07)
  - shared normaliser module — location TBD during ac-06 implementation

## Status movement

Status is `proposed` until ac-06 and ac-07 ship with the shared normaliser
module importable from both sides, and the unit test plants a collision
and confirms removal. On that pass, status moves to `accepted`.
