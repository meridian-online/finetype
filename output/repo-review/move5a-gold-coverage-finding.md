# Move 5(a) — the real state of per-guard gold coverage (2026-07-11)

**Trigger:** the impediment review's finding t7 — "the headline eval is blind to the guard campaign
it grades … ~0 rows for every guard since June, so 'gold flat' checks unrelated labels." Acting on
move 5(a) ("add verified positives + hard-negatives to gold at each guard ship") surfaced that the
premise is **substantially refuted by inspection**. This documents the true coverage so the
tracking is honest.

## What the coverage actually is

Every recent guard's **demote-target attractor column is already a gold row** — verified against
`eval/gold/gold_corpus.tsv`:

| guard (ship) | demote-target column | gold row present? | gold label |
|---|---|---|---|
| geo_code_nonmembership_demotion (a326e1e) | `work_type`, `permit_type` | ✅ | → `word` |
| geo_code_membership_vote (8b78199) | `jurisdiction` | ✅ | → `country_code` |
| unlocode membership (f18ec7e) | `ticker` | ✅ | → `word` |
| locale_code (a976c69) | `venue_country` | ✅ | → `country_code` (see note) |
| npi checksum (v0.6.41) | `longTermDebt`, `marketCap` | ✅ | → `integer_number` |
| npi-epoch | `utc`, `idPedido` | ✅ | → `unix_seconds` / `unix_milliseconds` |

So the guards are **not invisible to gold**. Their hard-negatives were captured organically — the
external-data band (move 1) and the LLM/lens gold adjudication pulled exactly these attractor
columns into the fixture. A guard's "gold +1/+2" is a **real gold row flipping wrong→right**, not a
score on an unrelated label. t7's "checks unrelated labels" is overstated on the evidence.

## Where the gap is real (narrower than t7 claimed)

1. **Genuine POSITIVE coverage for rare checksum types is absent — and un-sourceable from current
   data.** `npi`, `upc`, `imei`, `issn`, `orcid`, `cas`, `iso6346`, `dea`, `mime_type`, `password`
   have **zero `curated_label` positives** in gold. This is NOT an oversight: genuine columns of
   these types are ~absent from both the gittables corpus AND the vendored external pool (checked —
   the pool has `lei`, which IS a gold positive at recall 1/1, but no npi/upc/imei/…). You cannot
   gold what the data doesn't contain. **The keep-side of these guards is instead covered by unit
   tests** (each checksum guard asserts a valid check digit stays). That is a complete-but-different
   instrumentation shape than t7 imagined: keep-side by unit test, demote-side by gold hard-negative.
2. **Efficacy magnitudes are quoted off the gate, not gold.** The `npi −87% / upc −95%` figures in
   CLAUDE.md are counts off the 33k gate — the instrument that is ~42% wrong on contested ground and
   only 9.6%/2.7% reliable on npi/upc specifically. **Fixed this session:** the quote is now tagged
   gated-YDF-**directional**, with the DIRECTION noted as gold-corroborated by the hard-negatives
   above (move 5(d)).

## Note surfaced in passing (separate issue)

`venue_country` is gold-labelled `country_code`, but the live pipeline demotes it `locale_code →
unknown` (the locale_code guard over-fires into abstention on a genuine country column). That is an
**under-emission** miss (the abstention failure class from the external-band split), not a move-5
item — logged here for the abstention/round-4 thread.

## The go-forward discipline (codified)

- **Every guard ship confirms its demote-target attractor is a gold row** (add one from the demoted
  set if absent), so "gold flat" always exercises the guard's mechanism. High-volume geo/company-ref
  guards get this free via the external band; certainty-roadmap guards should add one explicitly.
- **Genuine-positive gold coverage for rare checksum types is a known data-availability gap**, tied
  to the deferred quarterly external fresh-fetch (growth policy). Until then, keep-side stays on unit
  tests, and efficacy magnitudes stay labelled gated-YDF-directional — never as gold-verified.

## Verdict on move 5(a)

**Closed as refined, not as executed-verbatim.** The prescribed hard-negatives already exist; the
discipline is now codified; the one honest doc fix (directional efficacy) is applied. The residual —
genuine checksum positives — is a data-availability limit, not a backlog item, until fresh external
data is fetched.
