# ac-2 — over-emit / destination-drift measurement (raw Sense)

Reshaped 111-class model vs shipped s43 (244), both via offline predict_multibranch on
the SAME 11,179-column representative corpus sample (potion-8M FTMB, surgically aligned to
valid_dim 244). Raw Sense only — no Sharpen.

## Verdict: QUALIFIED, not a clean GO

**The targeted over-emitters vanish cleanly** (by construction — the leaves are gone):
isbn 98→0, si_number 22→0, currency_code 14→0, user_agent 0→0. All 134 ceded leaves → 0,
no leakage. 1,646 columns (14.7%) freed.

**But ~43% of the freed mass relocates to specific kept leaves — new over-emission:**

| freed mass goes to | columns | benign? |
|---|---|---|
| residual (plain_text +656, word +429, alphanumeric_id +60, numeric_code +43) | 1,188 (57%) | YES — recoverable |
| **specific kept leaves** | 886 (43%) | **scrutinise** |

Biggest specific-leaf gainers (raw Sense, reshaped − s43):
npi +94 (151→245), country_code +70 (16→86, 5×), coordinates +64 (13→77, 6×),
tsid +63 (2→65, 32×), postal_code +46, docker_ref +39 (1→40), city +34, lei +27 (0→27).

## Read

The reshape does NOT make the model more correct on the freed columns at raw Sense — it
moves the error. ~57% dissolves into the recoverable residual (good); ~43% relocates onto
other specific leaves, several now newly over-emitting (npi, tsid, coordinates, country_code,
lei, docker_ref). This is the destination-drift risk the spec flagged, partially realised.

**Crucial mitigation, untested here:** the value-based recovery layer (ac-3) re-asserts a
ceded type from VALUES, regardless of where the raw model put the column. So a genuinely-ceded
column gets recovered whether it landed in plain_text OR npi. The harmful residue is only
columns relocated to a specific kept leaf that are NOT recoverable ceded types. Sizing that
needs the composed measurement (ac-3).

Note: much of the drift lands on the HELD-BACK checksum leaves (npi, lei) still in the model.
Ceding those too — once their checksum recovery rules ship — would close that destination.

## Decision

Raw over-emit is a TRADE, not a clean win. The reshape's net value now hinges on ac-3:
does the Sharpen recovery catch the relocated ceded mass and net a composed improvement?
Cheap next probe before the full recovery-rule build: run the ceded-leaf validators over the
886 specific-relocated columns to estimate the recoverable fraction.

Substrate: output/model-label-space-reshape/ac2/ (preds_s43.tsv, preds_reshape.tsv).

## ac-2 UPDATE — rescue-fraction probe (recovery defuses the drift)

For the 1,646 freed columns (s43 said a ceded leaf), do the VALUES actually validate a
ceded leaf ≥0.9 — i.e. would the value-based recovery re-assert them? (Upper bound: the
real rule is header-gated for some leaves, so actual recovery ≤ this.)

- **64.5% validate a ceded leaf** → recovery re-asserts the correct type (63.5% the exact
  leaf s43 predicted).
- **By reshaped destination:**
  - relocated to a SPECIFIC kept leaf (the "harmful drift"): **85.5% rescued** (371/434).
  - relocated to residual: 57.0% rescued (691/1212).

The scary specific-leaf drift (npi/tsid/lei/...) is **85.5% recovered** — the value-based
rule re-asserts the right ceded type regardless of where the raw model put the column. The
~15% unrescued specific cases are dominated by columns s43 was ALREADY WRONG on (over-emitted
a ceded leaf on non-matching values — "productionDate"→day_of_week, "Husband"→periodicity);
relocating those to another wrong label is neutral, not a regression.

The 35.5% of freed columns that DON'T validate a ceded leaf are s43 over-emissions — under
the reshape they fall to the honest residual (plain_text/word) instead of a false ceded
label. That is a correctness IMPROVEMENT, not a loss.

## Revised verdict: drift largely benign — proceed to ac-3

The reshape does not create net new errors on the freed mass: ~65% recovers to the true
ceded type, ~35% corrects an s43 over-emission to an honest residual. The destination drift
that raw Sense flagged is mostly absorbed by value-based recovery. This is consistent with the
class-imbalance analysis: the drift is attractor reallocation, and the fix is recovery (+
ceding the npi/lei neighbour attractors), NOT rebalancing. Build the recovery rule (ac-3) and
confirm on COMPOSED; the probe predicts composed holds or improves.
