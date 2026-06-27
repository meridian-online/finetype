# Taxonomy-gap discovery — next-session brief

**Intent (author, 2026-06-19):** stop mining datetime (it's long-tail — see memory
`datetime-handling-is-long-tail`); spend the next session finding **taxonomy gaps** — real
types the engine cannot name, hiding behind `plain_text`/`word`/`unknown` or a wrong specific
leaf. This is the next accuracy lever. Promote this memo to a `/discovery` spec at session
start.

## The question

Where is FineType forced to give a *wrong or fallback* answer because the right type
**doesn't exist in the taxonomy**? Adding a missing type is "expanding coverage is the path to
accuracy" (Precision Principle) — the highest-leverage taxonomy work.

## Method (validated — reuse the determinability-probe machinery)

The determinability probe (`output/determinability-probe/findings.md`, 2026-06-16) already
proved the approach: blind Claude panels (the distillation teacher) over contested
residual-label errors, header + values only, no gold/model. 100% determinable (≥2 of 3 agree),
controls 15/15. It surfaced taxonomy gaps cleanly. Re-run that, but aimed at gaps:

1. **Mine the residual buckets.** From gold + corpus: columns the engine labels
   `representation.text.plain_text` / `.word` / `unknown` (the fallback-heavy families), and
   columns where gold itself used `plain_text` as a lazy catch-all. Cluster by header + value
   shape.
2. **Blind-panel adjudicate** each cluster: "what specific type is this?" Mix in a
   genuinely-independent teacher family (Qwen3:32b Ollama in-stack) as a third panel — the
   probe's caveat was that 3 Claude panels share priors.
3. **Tally recurring "other" answers** → candidate new types, with corpus volume per candidate
   (size the win before building, like the zoneless-ISO 0.01% sizing).
4. **Rank by volume × determinability.** A gap is worth a leaf only if it's both common AND
   the panel labels it confidently/consistently.

## Known candidates (seed list — already surfaced, do NOT re-derive)

From **choice 0095 taxonomy-gap ledger** (gaps surfaced during gold adjudication):
- `ISO-timestamp-without-Z` — ALREADY SCOPED (spec `2026-06-19-zoneless-iso-datetime-leaves`,
  deferred; ~615 cols). datetime — likely skip per the long-tail finding.
- `DMY-slash-with-seconds`, `tz-abbreviation`, `numeric-month` — datetime, long-tail, low priority.
- **fallback-heavy `plain_text` families** — THIS is the rich vein. Decompose it.

From **determinability-probe findings** (the 7 taxonomy-gap columns, panel said "other"):
- `street_name`, `file_path` / filename, `publication_year`, `link` (URL-ish), `block`.

From the **#3-utc unblock** (2026-06-27, composed-accuracy-roadmap):
- **`numeric UTC offset`** — a column of numeric hour-offsets (`-8`, `5.5`, `+5.45`) that
  *means* a timezone offset but has no leaf: `datetime.offset.utc` is STRING-only ("UTC
  +05:00", validator `^UTC [+-]\d{2}:\d{2}$`), so numeric offsets fall to `decimal_number`.
  Surfaced by the `utc_offset` cluster (5 gold cols, OpenFlights + gittables). LOW priority:
  decimal IS the honest storage type and the cluster is already resolved correctly by
  `utc_bare_number_veto`; a leaf would only recover the semantic, not accuracy. Decide
  add-leaf vs document-as-decimal at the next taxonomy pass. Small volume.

**Anti-candidate (do NOT add a leaf):** `entity_name` vs `plain_text` is NOT a taxonomy gap —
the boundary is real and `entity_name` is kept distinct (choice 0102). The confusion is the
*model over-emitting* entity_name (12 of 16 plain_text→entity_name re-adjudications KEPT gold;
0-for-6 retrain attractor, task t-000133e418). Fix is training negatives, not taxonomy. Same
for `integer`↔`binary` (value-determinable subtype) — keep both.

## What to produce

A ranked taxonomy-gap ledger: candidate type · corpus volume · determinability · example
columns · recommended action (add leaf / Sharpen rule / leave). Then (separate spec) implement
the top 1–3 that clear a volume bar — each needs taxonomy entry + generator + alignment, and
the deterministic-or-rule path to emit it (model retraining is the expensive last resort; the
datetime parser showed a value-based rule can assert a new leaf without retraining).

## Pointers

- Probe machinery + findings: `output/determinability-probe/`
- Gold corpus: `eval/gold/gold_corpus.tsv`; score `scripts/score_gold_anchor.py`
- Corpus values + current labels: `eval/gittables/corpus_pass/columns.parquet`
  (`sense_prediction`, `sample_values_truncated`)
- Evolution policy: choice 0095 (gold is append-only; emission-driven expansion)
- Related: memory `datetime-handling-is-long-tail`
