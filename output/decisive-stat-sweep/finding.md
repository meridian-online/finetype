# ac-04b decisive-stat sweep — finding

**Spec:** 2026-06-16-duckdb-shellout-ingestion ac-04b
**Date:** 2026-06-17
**Substrate:** `scripts/sweep_decisive_stats.py`, `output/decisive-stat-sweep/sweep.md`

## Headline

**The fast-path skip does not survive the data.** No full-column statistic is
clean enough to retire the neural pass on — every candidate predicate mislabels
far more than 1 column in 50, so the ≥98%-precise carve-out the short-circuit
needed does not exist. The speed-via-skip premise is a **NO-GO**. The accuracy
lever the skip was riding on is real, but it belongs in the existing correction
(Sharpen) layer that keeps the neural vote — not in a skip.

## What we measured

For each candidate predicate, swept its strictness knob and measured, on the
751/931 resolvable gold columns, the **slice precision** — of the columns the
predicate fires on, the fraction whose human label equals the asserted label.
The skip's pass-mark was ≥0.98 (author-confirmed). Best slice precision reached:

| predicate (asserted label)            | best slice precision | gold support | verdict |
|---------------------------------------|----------------------|--------------|---------|
| low-cardinality → categorical         | **0.329** (card≤0.01)| 67           | NO-GO   |
| high-card + alnum → alphanumeric_id   | **0.633** (card≥0.99)| 52           | NO-GO   |
| increment signature → increment       | ~0.13 (gold-blind)   | 1            | NO-GO   |
| exact {0,1} → binary                  | 0.00 (gold-blind)    | 0            | NO-GO   |

## Why every boundary fails — one structural reason

A full-column statistic **separates** these types on average (the June probe's
AUC 0.97–0.99 was real). But the *losing* side of each boundary is a
**high-frequency impostor that shares the exact same signature**, and the stat
cannot see past it:

- **low cardinality** is shared by categorical AND country codes, ISO codes,
  years, and sparse integer columns → 0.33.
- **high-card alphanumeric** is shared by alphanumeric_id AND URLs (17 of the
  misfires), street addresses, UUIDs, and free text → 0.63.
- **contiguous near-unique integer run** is shared by a real auto-increment AND a
  `Year` range (2010…2018), a rank (`GlobalRank` 1…20000), and `ID` columns the
  humans labelled **integer_number** → 1 true positive in 8.
- **exact {0,1} domain** is shared by a boolean flag AND constant columns
  (`nd=1`) and sparse 0/1 integer counts — which the humans labelled
  **integer_number** (`perm_unlink`, `SpeculativeGenerality_OneChildClass`).
  Gold contains **zero** binary columns, so asserting binary can only relocate a
  human label.

This is the Precision Principle exactly: a rule that confirms the wrong type
37–88% of the time is not a validation. Separability (AUC) is a ceiling on what a
*discriminator with a second opinion* can do — not a threshold a *blind
assertion* can ship at.

## The clinching argument

The binary and increment skips would **re-assert the precise labels that two
shipped, gold-validated DEMOTION vetoes exist to strip**:
`binary_vocab_veto` demotes binary→integer, `increment_substance_veto` demotes
increment→integer (both `crates/finetype-model/src/column/mod.rs`). Those vetoes
shipped because the model OVER-emits binary/increment and gold confirmed the
demotions. A skip asserting those labels runs directly against verified evidence
— and would make the gold headline WORSE, not faster-but-equal.

Because all four fail the ≥0.98 gold pre-condition, the corpus-honest relocation
gate (the second co-condition) is moot — there is nothing to promote to it.

## What survives

The column-statistics lever still has a real accuracy prize — the residual recall
gaps the probe found (categorical, alphanumeric_id). But its shape is a
**post-pass rule that keeps the neural vote** and layers the value-shape guards
the bare stat lacks (exclude URL/UUID/address patterns, year ranges, decimal
points, contested-ground checks) — the shipped 0048/0096 veto-fallback pattern
(`veto_shape_fallback`, `text_vocab_override`), NOT a skip. Even compounded with
those guards, alphanumeric_id tops out near ~0.82 on gold — a recall rule, not a
98% skip. That is a separate recall-rule bet, gated on gold + corpus-honest like
the four shipped fixes.

The **free-stats plumbing (ac-04a)** still stands and feeds that post-pass layer.

## One line for a stakeholder

Full-column statistics are a *correction* signal, not a *replacement* for the
model — the neural-skip is a no-go (no stat is 98% clean on its own), and the
lever lives on in the Sharpen layer where the model still votes.

## Scope / what we don't know

- 180/931 gold columns did not resolve to clean stats (column-name mismatches,
  read errors) — the 751 measured is a solid majority but not the whole fixture.
- binary (0) and increment (1) have ~no gold support, so their NO-GO rests on the
  *relocation* evidence (what they fire on is human-labelled integer_number) plus
  the shipped-veto contradiction, not on a precision number. No clean instrument
  asserts these skips are safe; absent positive evidence, the conservative call
  is NO-GO.
