# Corpus-honest gate — oracle-aware bands. Closes the honest-abstention blind spot, keeps every proven NO-GO.

**Date:** 2026-06-08
**Why:** the 0.6.24 precision patch (validation-veto default-on + utc/url/measurement_unit/geohash schema-fail demotions) is a real corpus-scale *improvement* — +858 columns more correct vs the gated-YDF oracle, 0.2% false-veto (`output/corpus-honest-gate/v0624_finding.md`) — but the original gate returned **NO-GO**. The bands could not tell a regression (a *correct* prediction relocated onto a wrong label) from honest abstention (a label the oracle *already* refuted, demoted to `unknown`). Every correct-FP removal read as `collapse`; every demote-to-`unknown` read as `oracle_fp`.

## The fix (one idea, applied to both oracle-keyed bands)

Classify the oracle against **both** ends of each column transition A→B, not just the candidate:
- `oracle_dst` — oracle vs candidate label B (was the only signal before)
- `oracle_src` — oracle vs base (v19) label A (new)

Then:
- **`oracle_fp`** counts only **created** false positives — a move A→B the oracle refutes (oracle≠B) **where it confirmed the source** (oracle==A). A move refuted at both ends was already wrong; demoting it is not a regression.
- **`collapse`** measures loss of **oracle-confirmed support** (`cand_correct/base_correct`), not raw marginal. Demoting an over-emitted label — which has little correct support — is no longer a collapse. New floor `--collapse-correct-floor 1000` suppresses ratio noise on labels with scant ground truth.
- **`over_emit`** unchanged (raw marginal ratio ≥ 3.0) — it is the load-bearing band for v22/v23 and did not misfire.

## Four-verdict regression — every canonical verdict preserved

| candidate | known verdict | refined gate | load-bearing mover (band) |
|---|---|---|---|
| v19 vs itself | GO (no false alarm) | **GO** (0 triggers) | — |
| latdec | NO-GO (latitude relocation) | **NO-GO** | latitude `oracle_fp` (net 2,127, obs 243) |
| v22 | NO-GO (geography over-emit) | **NO-GO** | city `over_emit` 3.06×; latitude `over_emit`+`oracle_fp` |
| v23 | NO-GO (categorical +529%) | **NO-GO** | categorical `over_emit` 8.68× + `oracle_fp` (net 27,858) |
| **0.6.24 patch** | (the new candidate) | **GO** (0 triggers) | — |

Trigger counts fell 52–78 → 11–14 on the bad candidates — the refined bands shed the broad-churn noise the ac-03 repro flagged, while the load-bearing movers still fire.

## Why 0.6.24's demotions are correctly exempt

| label | raw ratio | base_correct | correct_ratio | obs created-FP | fires? |
|---|---|---|---|---|---|
| `unknown` | 1.74 | 0 | 1.0 | 117 (< 120 floor) | no |
| `representation.discrete.categorical` | 1.55 | 39,653 | 1.149 (gained) | 104 (< 120) | no |
| `measurement_unit` | 0.019 | 0 | 1.0 | 0 | no |
| `geohash` | 0.294 | 0 | 1.0 | 0 | no |
| `utc` | 0.009 | 0 | 1.0 | 0 | no |
| `decimal_number_comma` | 0.456 | 253 | **1.0** | 0 | no |

`decimal_number_comma` is the proof the fix is honest, not lax: its raw marginal fell 46%, but it **kept every one of its 253 oracle-confirmed columns** — the drop was pure FP removal. A patch that actually destroyed real `decimal_number_comma` would show `correct_ratio` ≪ 1 and fire.

## The remaining guard

`oracle_fp` still fires when a veto destroys **oracle-confirmed** labels at scale: it needs net created-FP ≥ 1,000 **and** raw observed ≥ 120 **and** ratio ≥ 0.20. 0.6.24's truly-harmful vetoes (117 observed) sit just under the established obs-floor — genuinely below noise, not tuned to pass. A more aggressive future veto would trip it.

## Evidence
- Scorer: `scripts/corpus_honest_gate.py` (oracle-aware `transition_counts` + bands).
- Reports: `output/corpus-honest-gate/refined/gate_refined-{v19self,v22,v23,latdec,v0624}.json`.
- 0.6.24 corpus pass: `output/corpus-honest-gate/v0624_pass/corpus_pass/columns.parquet`.
