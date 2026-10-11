# Review checklist

The Meridian review standard. roborev reads a repo's `REVIEW.md` as its review guidelines, and each Meridian code repo carries this same file at its root. Change it at the canonical copy and carry the change to the others.

## Important — 🔴 must fix before merge
- **Correctness:** logic bugs, race conditions, incomplete flows, wrong results.
- **Security:** input validation, secrets in logs/output, injection.
- **Stability:** unhandled errors, panics, unhandled edge cases.

## Meridian-specific — check on each review
- **Cross-repo privacy:** a PUBLIC repo's code / commit / PR must not contain private planning terms, the private planning-repo name, private PR numbers, or strategy/instrument terms.
- **Verified, not asserted:** load-bearing claims are backed by a real run / golden image / passing test — not prose.
- **Determinism where promised:** byte-identical outputs; no `HashMap` on an order-bearing path.
- **Dependency hygiene:** no new dependency without a licence check (permissive only; the `deny.toml` gate). Flag any git-tree/pinned dep.

## Nit cap
Report at most 5 nits, then "plus N similar items."

## Skip
Generated files, lockfiles, and anything CI already enforces (formatting/lint).
