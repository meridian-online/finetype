# Per-type distilled caps are cosmetic in the v4 training path

Discovered mid-v27 (spec `2026-06-11-categorical-identifier-recall-retrain`,
closed Failed-informative): `prepare_multibranch_data.py` caps
`columns_by_type` via `cap_distilled_columns()` but never rebuilds
`ordered_distilled`, and `group_distilled_by_proximity()` (line ~2429) —
which produces the v4 sibling-grouped table groups that become FTMB records —
consumes `ordered_distilled`. So the v13 global `--distilled-cap 600`, the
latdec `decimal=2600` override, and the new `--type-cap` flag have all been
**cosmetic** for v4-format training data: every qualifying distilled row
flows into table groups uncapped.

Implications:
- v19's actual training distribution is NOT what its build log's capping
  table claims (e.g. entity_name logged 12,530→600 but ~12.8k records ship).
- latdec's "one-variable cap lift" was a no-op; its 2,540 hard negatives
  entered via ordered_distilled regardless.
- Any future retrain that reasons about per-type training mass from the cap
  flags is reasoning from fiction.

The fix is one structural change — rebuild `ordered_distilled` from the
capped `columns_by_type` after `cap_distilled_columns()` — but it is
LOAD-BEARING: it would change every future build's distribution, including
a byte-faithful v19 rebuild. It needs its own validation (v19-recipe rebuild
+ FTMB label-distribution diff + a gold-anchor sanity run) before any
retrain relies on caps again.

Workaround that exists today: enforce per-label masses in the blend CSV
itself (`build_v27_recall_distilled.py` `BASE_KEEP`/`MINED_KEEP`,
deterministic md5-ranked downsampling) — proven in v27 round 2 (FTMB masses
landed exactly on design).

Candidate shape: small `code`-type spec against card 0002 (or a training-
infra card if one exists by then).
