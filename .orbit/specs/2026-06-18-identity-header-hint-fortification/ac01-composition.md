# ac-01 — hard-negative composition + scope correction

**Date:** 2026-06-18 · spec `2026-06-18-identity-header-hint-fortification` · gate AC (doc)
**Substrate:** `eval/gittables/corpus_pass/columns.parquet`, `/tmp/rhh_identity_counterfactual.tsv`
(per-column ac-00 re-run), `finetype generate`.

## Scope correction (discovered driving ac-01) — NARROW to username, do NOT delete the whole hint

The per-column ac-00 counterfactual shows the 8 eval columns `substring_matcher_identity`
actually carries are **phone (×2), first_name (×2), height (×2), weight (×2)** — NOT
username/full_name. Without the hint the model emits: phone→cas_number, first_name→word,
height/weight→decimal/integer.

`height`/`weight`/`first_name` are **header-only types** — a column of `180, 175, 168` *is*
integers; only the header `height_cm` makes it "person.height". There is no value signal to
learn, so the model can never carry them from values alone. Deleting the WHOLE
`substring_matcher_identity` family is therefore an unwinnable value-learning bet (the same
semantic ceiling as cardinality-boundary-error-is-real).

**Re-scope:** this retrain targets **username recovery** (the real production mass) and
**preserves full_name**. The header-hint change narrows from "delete the family" to "retarget
the author/name arms" once the model carries author+handle→username. Whole-family deletion
(phone/height/weight) is a SEPARATE, harder question — deferred, not in this spec.

Consequently the **ac-04 gate is re-scoped**: from "disable substring_matcher_identity → ≥80%"
(unachievable — bundles header-only types) to the **username/full_name OUTCOME** — gold
username & full_name precision/recall (no full_name regression) + corpus author-column
relocation correctness (the isolated-pass measure that already works).

## The opportunity (corpus-measured)

165,949 `author`/`login`-headed columns; the model defaults **164,812 (99.3%) to full_name**
— driven by the `author→full_name` hint. By value shape:

| bucket (value shape) | n | correct label |
|---|--:|---|
| HANDLE (space-frac ≤0.15, distinct-frac ≥0.9) | **108,150 (65%)** | username |
| ambiguous / mixed | 56,872 (34%) | exclude from training |
| MULTI-TOKEN (space-frac ≥0.5) | 927 (0.6%) | full_name (the legit case) |

So "author" in this corpus overwhelmingly means *login handle* — the hardcoded
`author→full_name` hint is wrong ~99% of the time. Abundant, clean positives exist
(unlike numeric_code, which had none).

## Composition

Additive over the v22 boundary blend (v23/v24 recipe), per-type capped (~1800):

1. **username positives (header+value paired):** sample from the 108,150 clear-handle
   `author`/`login` columns — header carries `author`/`login`/`screen_name`/`created_by`,
   values are high-cardinality single-token handles. Teaches header+value→username. PLUS the
   existing `finetype generate` synthetic usernames (already in training).
2. **full_name preservation:** the 927 multi-token author columns + the existing full_name
   distilled/synthetic — so the legit author=real-name case is not lost.
3. **hard negatives (must NOT become username):**
   - single-token HIGH-distinct name columns: country, state, `api_name`, `column_name`
     (the residual leak the cardinality guard could not close, spec
     2026-06-17-full-name-username-veto ac-03);
   - low-cardinality single-token vocabularies: exchange codes (`NMS`/`NYQ`),
     drug/role/platform names (the 55% false-positive class from the same ac-03).

## The risk (state it plainly)

`username` is ALREADY a generic catch-all attractor (`HARDCODED_GENERIC_LABELS`) — the value
branches predict it readily for any short token, which is why it is demoted today. Training it
harder is precisely the 0-for-5 over-emission failure mode: it could spill username onto every
short-token column. **The destination-drift proxy pre-check (ac-02) is the gate that catches
this** — NO-GO means do not launch. The hard negatives in (3) are the specific counterweight.

## Limits

- Positives are labelled by the value-shape heuristic (handle = high-card single-token), so
  label quality is bounded by it — but the HEADER pairing (author/login) is the additional
  signal the learned branch gains over the brittle rule, which is the point.
- The 56,872 ambiguous columns are excluded (mixed 8-sample whitespace — likely author lists);
  including them would inject noise.
