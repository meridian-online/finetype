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
- **Dependency hygiene:** read the licence of each dependency a change adds or alters, Rust ones included: cargo-deny enforces a repo's `deny.toml`, but its allow-list does not decide the review. A dependency offered under a choice of licences, an SPDX `OR` expression such as `Apache-2.0 OR GPL-2.0-only`, is judged by the most permissive licence it offers; under an `AND` expression each licence listed applies, so it is judged by the least permissive one; and a licence carrying an SPDX `WITH` exception is judged as its base licence. A permissive licence passes. A strong copyleft licence such as GPL or AGPL is a 🔴 must-fix, even where a `deny.toml` allows it. Any other licence is flagged by name so the merge is decided knowing it: weak copyleft such as MPL-2.0 or LGPL, whether or not a deny.toml allows it, and a source-available, non-commercial or unidentified licence. Flag a git-tree or pinned dependency too.
- **An arc operator is not one source:** a change that adds or extends an arc operator (an `op:` step type in arc's catalogue) whose inputs are named for one source, or whose script holds one dataset's columns, endpoints, paging or model, is a finding at Medium or above: an operator takes those from its Protocol step's `with:`.
- **A merge check tests only what the change can break:** during open development, a check that blocks a merge tests the repo's code against inputs pinned in the repo. A check that blocks a merge on a comparison with another repo's `main` or the live open lake is a finding at Medium: that comparison is a scheduled drift report, which never fails a merge or opens a P1 and whose findings are fixed in batches, since it shows two repos moved apart, not a fault in the change. So is a merge check on a non-deterministic output, such as the Splink crosswalk, that compares its bytes, hash or exact row count instead of its shape (columns, keys, a row-count range). Strict cross-repo checks run at release, and a failure a reader can see stays an alarm.

## Nit cap
Report at most 5 nits, then "plus N similar items."

## Skip
Generated files, lockfiles, and anything CI already enforces (formatting/lint).
