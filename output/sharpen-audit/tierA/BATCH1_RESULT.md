# Tier-A Batch 1 — shipped result

Spec 2026-06-27-composed-accuracy-roadmap ac-3. Five swap-proof, demote/decline/recovery-only
Sharpen rules, co-gated against the reproducible m2v8m-s43 baseline.

## Verdict: GO both gates

| gate | result |
|---|---|
| **Gold (reframe headline)** | 771/927 = 0.832 → **780/927 = 0.841** (+9 cols, +0.97pp) |
| **Gold regression check** | 14 gold cols changed → **9 fixes, 0 regressions, 5 neutral** (card-0019 zero-regression: PASS) |
| **Corpus-honest (m2v8m rule-OFF → rule-ON)** | **GO, 0 triggers**; footprint 1,391/837,625 = 0.17% |

Corpus footprint (all expected directions, no relocation):
- 1,079 decimal→integer (#10 IS_FLOAT) — gate scored **+1,082 oracle-correct** (727,539→728,621), a corpus-wide win not relocation
- 303 state→region (#9 dead-alias rename)
- 4+1 entity/word→plain_text (#7), 2 npi→isbn (#6b), 2 state→country_code (2nd-order header reroute)

## The five rules + their gold fixes

| # | rule | kill switch | gold cols landed |
|---|---|---|---|
| #4 | numeric-residual-fallback (validation_veto) | — (post-veto) | `Parent`→decimal ×2, `x_coordinate`/`y_coordinate`→integer ×2 |
| #6b | ISBN-10 header recovery | `isbn_header_recovery` | `Primary ISBN10` npi→isbn ×3 |
| #10 | decimal→integer IS_FLOAT demote (feature_sharpen) | — | `totalCashFromOperatingActivities` decimal→integer ×1 |
| #9 | retired state→region header hint | — | `PROVINCE` state→region ×1 |
| #7 | entity_name/word long-prose → plain_text | — | latent (no gold col this run; 0 regression) |

## Load-bearing implementation note

#10 went into `feature_sharpen` (line 20), NOT `feature_disambiguate` (line 118). The composed path —
native `classify_multi_branch` (mod.rs:1437) AND `compose_from_sense` (1537, the gate's fast path) —
calls `feature_sharpen`. `feature_disambiguate` is the raw-classify step (929). The investigator's
spec anchored the wrong twin; verified against the call graph before applying. Both functions carry an
identical F5 block; F5b in `feature_sharpen` alone makes native + gate agree.

## Substrate
- baseline binary: `output/sharpen-audit/tierA/finetype_base_0823`
- candidate binary: `output/sharpen-audit/tierA/finetype_batch1`
- reusable raw-Sense cache: `output/sharpen-audit/tierA/sense_cache.tsv` (837,625 cols, model-intrinsic)
- gate: `scripts/gate_from_cache.sh` (reuses the cache; re-gating any rule is seconds)
- gate verdict json: `output/sharpen-audit/tierA/gate_batch1/gate/gate_tierA-batch1.json`
