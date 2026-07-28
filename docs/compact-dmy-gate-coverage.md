# The fast corpus-honest gate says GO — and it could not see this change

- verdict: **GO**, `triggers: []` — raw verdict in `docs/compact-dmy-gate-verdict.json`
- label: `compact-dmy-range`
- candidate: working tree; baseline: `origin/main` label files, same model, own build
- command: `scripts/corpus_honest_gate_fast.sh <wd> <candidate-bin> <baseline-bin> compact-dmy-range`
- sample: the 33,220-file stratified sample (33,054 tables profiled), ~3% of GitTables

## The verdict is not coverage, and here it is not even evidence

`datetime.date.compact_dmy` **does not appear in the gate's movers at all**, and
of the 25 movers it does report, **none has a single nonzero candidate-side
signal** (`est_cand_marginal`, `base_correct`, `cand_correct`, `contra_in`,
`contra_out` are 0 across all 25). A GO from a gate that registered no movement
whatsoever is a gate reporting that nothing happened.

Something did happen. The same change, measured by a real two-sided profile pass
over 1,723 tables, moves 946 columns off this label and costs 68 genuine
YYYYMMDD columns their type (`docs/compact-dmy-blast-radius.txt`).

The reason is mechanical, not statistical, and it is worth naming because the
usual caveat — "the sample is ~3% and non-adversarial" — is the *weaker* of the
two here:

> The fast gate is a **sharpen-rule** instrument. It takes ONE sibling-aware
> raw-Sense pass and then replays it through two `resharpen` runs, baseline and
> candidate. The per-label **validator pass-rate vector is computed in the Sense
> stage**, which is exactly the part that is cached and shared. A validator edit
> therefore cannot propagate to either side of the comparison. The gate is
> structurally blind to this class of change, not merely underpowered against it.

This is why the probes exist. `scripts/probe_compact_date_residual.sh` profiles
six column families end to end through the CLI on four sides and finds in
seconds what the gate cannot see at any sample size, and
`scripts/compact_dmy_blast_radius.sh` runs the genuine two-sided pass the gate's
fast path skips.

A GO here means only: this change did not trip a sharpen-rule regression band.
It is worth having for that. It is not evidence that the change is safe, and
nothing in this branch rests on it.
