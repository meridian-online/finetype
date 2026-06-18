# ac-02 + ac-03 — implementation + validation

**Spec:** 2026-06-18-calibrated-confidence-abstention
**Date:** 2026-06-18 · binary 0.6.34

## ac-02 — what shipped (code)

A `quality_band` (high / medium / low) and a `runner_up` type (on the `low` band)
added to the **profile** output. Purely additive — the predicted label and the raw
`confidence` are unchanged. Default-on, per author approval.

- **Thresholds** (`crates/finetype-cli/src/profile.rs`): high ≥ 0.85, medium
  0.70–0.85, low < 0.70 — the data-driven knees from ac-00.
- **runner_up**: the second-best per-value vote (`vote_distribution`), surfaced only
  on the `low` band and only when it differs from the emitted label.
- **Formats**: json (`quality_band`, `runner_up`), csv (two new trailing columns),
  plain (`⚑ low (maybe X)` / `~ medium`; high unmarked = trusted), markdown
  (Quality column).

**B07 consumer audit (run before the edit, per H03):** `codegraph_impact ColProfile`
→ 3 symbols, all in `profile.rs` — `ColProfile` is a function-local struct, not
exported. MCP and the DuckDB extension consume the JSON *output*, not the struct, so
the additive fields cannot break them. The JsonSchema/`x-finetype-*` machine-contract
path (the `json_schema` crate, read by `validate`/duckdb/`transform_projection`) was
deliberately left UNCHANGED — heavier cross-crate surface, out of scope for this
analyst-facing increment (documented follow-up if MCP wants the band in the schema).

**CI mirror green:** `cargo fmt`, `cargo clippy -p finetype-cli -- -D warnings`,
`cargo test --workspace` (all pass; `cli_golden.rs` reads profile JSON by key, so the
additive fields don't break it). Predictions/headline unchanged by construction — no
label or confidence logic was touched.

## ac-03 — does the signal help the analyst? GO (with one honest caveat)

Band precision on both fixtures (v19, --reframe lens):

| band | gold P (n) | repr P (n) |
|---|---|---|
| **high** (≥0.85) | **0.912** (464) | **0.816** (147) |
| medium (0.70–0.85) | 0.736 (87) | 0.676 (34) |
| **low** (<0.70) | 0.772 (254) | **0.538** (78) |

1. **Separation is real and strong at the top.** The `high` band is 0.91 (gold) /
   0.82 (repr) — clearly above everything below it. An analyst can trust a `high`
   column. On representative data the full ladder is monotonic (0.82 → 0.68 → 0.54):
   the `low` band is barely better than a coin flip, exactly the "scrutinise" signal.
2. **The caveat — medium is the muddy tier.** On gold, medium (0.736) and low
   (0.772) invert, but within heavily overlapping CIs (n=87 vs 254) — they are
   statistically indistinguishable there. So the load-bearing signal is **high vs
   not-high**, plus the sharp **low cut on production data**; the medium tier is a
   soft "spot-check" middle, not a clean accuracy stratum. This matches its framing
   and is not a reason to withhold the feature — but it is stated, not hidden.
3. **runner_up** populates on **160/254 (63%) gold** and **26/78 (33%) repr**
   low-band columns, and carries the *correct* answer in **12 gold / 6 repr** cases
   where the top guess was wrong. A real minority bonus — the analyst occasionally
   sees the right type as "maybe X" — not the core value, which is the band.
4. **No regression.** The fields are additive; the predicted label and confidence
   are untouched, so the gold/repr headline is unchanged by construction.

**Verdict: ship it on (as approved).** The feature is not cosmetic — `high`
genuinely marks trustworthy columns and `low` genuinely marks the coin-flip ones on
production data. The honest limit: the three-band granularity over-promises a clean
medium tier that the data does not support; the dependable read is "high = trust,
low = scrutinise (here's the runner-up)."
</content>
