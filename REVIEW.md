# Review checklist

The Meridian review standard. roborev reads a repo's `REVIEW.md` as its review guidelines. Each Meridian code repo carries this same file at its root, byte for byte, so a change to it is made in each of them in the same pass.

## Important — 🔴 must fix before merge
- **Correctness:** logic bugs, race conditions, incomplete flows, wrong results.
- **Security:** input validation, secrets in logs/output, injection.
- **Stability:** unhandled errors, panics, unhandled edge cases.

## Meridian-specific — check on each review
- **Cross-repo privacy:** a PUBLIC repo's code / commit / PR must not contain private planning terms, the private planning-repo name, private PR numbers, or strategy/instrument terms.
- **Verified, not asserted:** load-bearing claims are backed by a real run / golden image / passing test — not prose.
- **Determinism where promised:** byte-identical outputs; no `HashMap` on an order-bearing path.
- **Dependency hygiene:** no new dependency without a licence check, permissive licences only. Where the repo has a `deny.toml`, cargo-deny is that check; elsewhere, check the licence by hand. Flag a git-tree or pinned dependency that the change adds or alters.

## Nit cap
Report at most 5 nits, then "plus N similar items."

## Skip
Generated files, lockfiles, and anything CI already enforces (formatting/lint).
