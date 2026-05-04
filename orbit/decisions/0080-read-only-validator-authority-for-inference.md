---
status: accepted
date-created: 2026-05-04
date-modified: 2026-05-04
---
# 0080. Read-only validator authority for the inference module

## Context and Problem Statement

The triangulator (MADR 0079) detects cases where the validator —
not the model — is the broken signal. The Sex/timezone/Name
examples from the interview-Q3 evidence all have the same shape:
predicted_type is correct, validator rejects ≥50% of samples, and
header-match strongly supports predicted. When the inference module
fires this case (mechanism `validator_widening`), what authority
does it have? Three possibilities span the spectrum from passive
flag to autonomous taxonomy mutation. The 2026-05-10 contract has
E04 (taxonomy/validator amendment) as a manual-attended escalation;
that surface defines the read/write boundary for taxonomy state.

## Considered Options

- **Option A — Read-only signal.** Module emits
  `mechanism="validator_widening"`. Cycle worker logs it in the
  sidecar `inference_signals.tsv`. Human review (or a future
  bead) decides whether to amend the validator. No automation.
  Stays inside the contract's halt/escalation framework as-is.
- **Option B — Auto-file an E04 amendment proposal.** When the
  module fires `validator_widening`, it emits a structured
  proposal (suggested validator widening, e.g.
  "add `[Mm]ale|[Mm]ale|MALE|FEMALE` alternations to gender
  enum") into a queue Hugh reviews tabletop-style. Faster path
  to validator improvements; widens the contract surface — every
  cycle now potentially produces taxonomy-amendment proposals
  that need triage.
- **Option C — Auto-amend if confidence is very high.** When
  `validator_widening` fires AND the suggested widening is
  composition-of-canonical-sibling per MADR 0078 AND
  triangulator confidence is ≥0.95, the module commits the
  validator widening directly via a generated PR or direct edit.
  Risky: amends `labels/definitions_*.yaml` autonomously; opens a
  failure mode where a wrong inference cascades into a wrong
  taxonomy edit that future cycles consume as ground truth.

## Decision Outcome

Chosen option: **Option A — read-only signal**, because the
contract's E04 surface is already the right venue for
validator-amendment decisions and routing them through it
preserves the human attention the amendment quality demands.
Auto-file (B) and auto-amend (C) both transfer authority over the
taxonomy from the human-attended E04 to the inference module's
confidence calibration — and the interview evidence establishes
that the validators themselves are noisy, so the inference
module's confidence in "validator-is-broken" assessments is
inherently second-order noisy. Compounding that with auto-amend
authority risks taxonomy drift faster than it produces signal.

The constraint is operationally enforced by:
- Inference module's source has no write-paths to `labels/`,
  `crates/finetype-core/src/`, or any taxonomy artefact. Audited
  via `grep` against the file list at PR review time (ac-08
  verification).
- The mechanism token `validator_widening` is purely diagnostic.
  Downstream consumers (cycle worker, E04 author, future
  taxonomy-amendment beads) use it to triage, not to act.
- If `validator_widening` accumulates at high frequency across
  many cycles, that itself is a signal for a separate card
  (validator-quality audit) — not a trigger for autonomous
  amendment.

### Consequences

- Good, because the failure mode where bad inference cascades
  into bad taxonomy edits is structurally impossible: the module
  cannot edit the taxonomy.
- Good, because E04's existing manual-attended workflow remains
  the authoritative venue for taxonomy state changes; no new
  authority surface to govern.
- Good, because the human attention is concentrated where
  judgement is required — picking which validator widenings are
  canonical (MADR 0078 composition vs hand-rolled Option-A
  variants) — and the autonomous module focuses on signal
  generation.
- Bad, because the slow path to validator amendment is unchanged
  from pre-7zi. If the validators ARE broken in ways the
  inference module can detect, the fix latency is gated on
  human review, not on detection. Mitigated: the
  `inference_signals.tsv` sidecar gives the human reviewer a
  pre-computed list of suspect validators with rationale.
- Neutral, because Options B/C remain technically open as future
  evolution; nothing in this decision precludes a later card
  promoting the proposal-queue or auto-amend path once the
  inference module's calibration is empirically known.

References:
- `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml` (constraint
  block, ac-08)
- `orbit/specs/2026-05-04-autonomous-type-inference/interview.md` (Q4)
- `orbit/contracts/2026-05-10-gittables-90-percent-roundtrip.yaml`
  (E04 escalation)
- `orbit/decisions/0078-validator-alternations-compose-canonical-sibling-patterns.md`
