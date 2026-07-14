# delimited_array_recovery — self-precise delimited-list recovery (2026-07-14)

**Headline:** ~130 corpus columns of genuine delimited lists — `Biography|Comedy|Drama`
genre tags, `subjects: nanoparticles;polymers;raman`, `(0, 23)` ranges, `('soxr','ompn')`
gene-regulatory edges — that FineType strands as "plain text", "an entity name", or
"unknown" now type as the `container.array.*` leaf for their delimiter. No place, address,
money, date, decimal, or name column was touched.

## The precision story (why this is NOT ~590)

The roadmap sized this at ~590 on a naive "≥90% homogeneous delimited list" count. That
count is a mirage, because **the bare comma has no self-precise signature**: it separates
list items *and* lives inside dates (`Dec 30, 2020`), money (`$928,760,770`), decimals
(`10,91`), addresses (`Carier Site, East Street, Braintree`), and places
(`Winter Park, Florida`). A comma between two words is structurally identical whether it is
a list separator or an intra-entity comma — so a bare-comma "list" cannot be told from a
`city`/`full_address` by value alone (Precision Principle).

So the guard recovers only where the delimiter *is* self-precise:

- **Bracket** `[a, b, c]` / `('x', 'y')` → `comma_separated`. The brackets disambiguate the
  comma, so even a two-element list `[-1, 0]` is admitted.
- **Pipe** `a|b|c` → `pipe_separated`. Pipe never lives inside dates/money/prose.
- **Semicolon** `a;b;c` → `semicolon_separated`. Likewise self-precise.

The bare-comma majority is deliberately left alone — that exclusion is the honest scope, and
it is exactly why 590 collapses to a clean ~130.

## The detector + guard

`finetype_core::structure::delimited_list_delim(&str) -> Option<ListDelim>` — bracket / pipe /
semicolon branches with the vetoes baked in: reject `://` (URLs with commas), prose parts
(>6 words), heterogeneous positional records (`list_homogeneous`), split datetimes
(`Tuesday, 21 Feb 2017 | 7:58 AM ET`), and — the post-spot-check tighten — any part carrying a
bracket/brace `(){}[]` (a flat-list element is a scalar, never a JVM signature `(L…;L…` or a
`path/|hash/|{json}` record). A **bare** two-element pipe/semicolon list must be two alphabetic
single tokens, which drops `id|number` records and numeric coordinate/decimal pairs whose
two-element ambiguity a bracket would otherwise resolve.

`delimited_array_recovery` (guards.rs) fires on `{plain_text, word, unknown, entity_name}` —
the labels where a delimited-list value is unambiguously a mislabel — with **per-column
delimiter voting** (each cell votes its delimiter, the winner must carry ≥90% of the passing
cells, and its `container.array.<delim>_separated` leaf is assigned). Promote at ≥90% pass AND
≥3 distinct pass the winning delimiter. NO new leaf (the four `container.array.*` leaves exist),
NO retrain (0096), RHH-toggle `delimited_array_recovery`.

### Interaction with `ceded_leaf_recovery` (measured)

`ceded_leaf_recovery` runs downstream and reclaims any `[…]` that JSON-validates → `json_array`
(a *more* precise leaf), so valid- and Python-repr bracketed lists (`[20000,10000,15000]`,
`['email','phone']`) end at `json_array`, not `comma_separated` — correct, and left as-is. The
bracket branch's genuine unique contribution is **paren-tuples** `(…)`, which ceded's `[]`-only
json_array validator does not touch (89 of the 130 recoveries: `(0, 23)` ranges,
`('soxr','ompn')` gene edges, `(1,2,3)` int arrays).

## Held-back (deferred follow-ups)

- **Numeric-sense overrides** (`coordinate`, `currency.amount_comma`): a bracketed two-element
  numeric list carries a genuine coordinate/decimal ambiguity (`[lat, lon]` is a defensible
  coordinate). Excluded from FIRE_ON; needs an element-count carve-out + its own measurement.
- **comma_separated decompose on paren-tuples**: the leaf's `STRING_SPLIT(col, ',')` leaves the
  wrapping parens in the split output for `(0, 23)`. The *label* is correct; the decompose
  polish (strip a single wrapping bracket pair) is a separate taxonomy refinement.

## Gates (all pass)

| Instrument | Result |
|---|---|
| Unit + guard tests | detector (accepts/rejects) + guard (voting/decline/spare/distinct) green |
| Corpus-honest fast gate (blocking H05) | **GO** — zero triggers, zero bands |
| Gold (reframe) | **882/1037 flat**; guards-on-vs-off = **0 rows flipped** (gold-neutral — targets are corpus residual, not curated gold) |
| Representative (advisory) | **195/260 = 0.750** — flat vs standing baseline |
| Mandatory spot-check (130 changed) | **0 false friends, 0 code artifacts** — comma 89 (paren-tuples), semicolon 32 (tag/int lists), pipe 9 (genre/RDF lists); smallest distinct = 3 (gate-enforced) |

The spot-check earned its keep again: the first cut recovered 133, and eyeballing every changed
column by distinct cardinality caught a 3-column code-artifact tail (2 JVM method signatures, 1
`path|hash|json` record) the gate + gold are blind to. The bracket/brace-in-part tighten removed
exactly those 3, zero collateral (comma unchanged at 89, semicolon 34→32, pipe 10→9).

Substrate: this file; `output/delimited-array/{mine,buckets}.py`; `gate/`, `gold_pred_*.tsv`,
`repr_pred_cand.tsv`; roadmap `output/reservoir-mining/roadmap.md`.
