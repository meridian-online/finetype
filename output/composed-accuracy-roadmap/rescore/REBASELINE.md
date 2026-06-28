# Composed-accuracy roadmap — re-baseline (step-1, t-0001425e)

Live 0.6.38 native composed gold: **795/931 = 0.854** (reframe == legacy; categorical retired
in 0.6.38, so the two scorings converged). 95% CI 0.830–0.875.

**The BACKLOG.md 0.812 / 174-error snapshot is STALE** — the 0.832→0.852 Sharpen campaign
(shipped v0.6.38) and gold re-adjudication #28 already banked since it was written. True
baseline 0.854, **136 live errors** (`live_errors.tsv`).

## Re-anchored bankable clusters (live error transitions, reframe)
| cluster | n | roadmap fix | claimed task |
|---|---|---|---|
| integer_number → increment | 8 | #12 increment over-attractor suppression | t-0001426418 (Tier-B) |
| alphanumeric_id → unknown | 9 | #13 veto_shape_fallback id-residual recovery | t-0001426418 (Tier-B) |
| integer_number → binary | 5 | #14 binary_vocab full-column feed | t-0001426418 (Tier-B) |
| integer_number → npi (false) | 3 | #6 NPI checksum SUPPRESS (over-emit, not recover) | — |
| url → query_string | 7 | url reader arm (investigate) | — |

Ceiling mass (not bankable): plain_text→entity_name ×12, word→entity_name ×5, currency.amount
→decimal ×4 (currency wall), word→region/iata (geo, Tier-C header-corroboration, deferred).

Next: implement Tier-B (t-0001426418), increment suppression first (biggest single cluster),
each gold-gated + corpus-honest (H05 blocking) + RHH kill switch.
