> Status: measured, recommendation is DO NOT ALIGN — 2026-07-28

# Serving the header branch its training conditions: measured, and rejected

The multi-branch header branch was **trained** behind frozen sibling-context
attention. `train_multi_branch` loads `models/sibling-context/` whenever it is on
disk (`scripts/overnight_m2v_244.sh` hard-requires it), and
`MultiBranchDataset::batch_groups` runs each table group's header features through
that module before they reach the branch, guarded by `n_cols > 1`.

Inference does not do this. So the branch is served an input distribution it never
learned under. The obvious fix is to reproduce training conditions at serve time.
This document records what happened when that fix was built and measured.

**It is not an improvement. On every instrument that can see it, aligning serving
with training is either neutral or worse. The recommendation is to leave serving
un-enriched — which is what the repo already does — and to consider closing the
skew from the TRAINING side instead.**

---

## 1. What was built, and why the measurement is attributable

Both directions were produced by the same binary shape, differing only in whether
the header branch reads a raw Model2Vec embedding of the column name or one that
sibling-context attention has already mixed across the table's headers.

Three properties make the comparison clean:

- **Identical inputs.** The corpus instrument reconstructs 33,054 tables as CSVs.
  All 33,054 files were byte-identical across the two runs
  (`sha256(sha256 of every file, sorted) = b090e502d7fe…`).
- **Identical Sharpen.** Resharpening ONE raw-Sense cache with each binary produced
  byte-identical composed output over all 837,625 columns
  (`sha256 = 8409f18b0125…`). Every difference reported below therefore originates
  in the Sense stage, not in a rule.
- **Identical attention.** The trainer's `FrozenSiblingContext` and the model
  crate's `SiblingContextAttention` were compared on the real shipped artifact at
  N = 1, 2, 5 and 17 columns: max absolute difference below 1e-5, with a
  degeneracy guard so an all-zero output could not satisfy the tolerance. The
  serving path really was reproducing the training path, not an approximation of
  it. That test ships in this branch
  (`crates/finetype-train/tests/sibling_context_train_serve_parity.rs`).

The serving change itself reproduced the trainer's `n_cols > 1` guard, because a
one-column table group was NOT enriched during training and enriching one at serve
time would open the same skew in the opposite direction. The sample contains no
one-column tables (median 13 columns per table, mean 26.9), so this guard does not
affect any number below.

## 2. What it changes — the full emitted record

`profile -o csv` emits fourteen fields. Over **32 real datasets under
`eval/datasets/csv`, 417 columns, at full value depth**:

| field | columns changed | share |
|---|---:|---:|
| confidence | 190 | 45.6% |
| quality_band | 51 | 12.2% |
| runner_up | 24 | 5.8% |
| disambiguation | 16 | 3.8% |
| **type** | **13** | **3.1%** |
| transform | 11 | 2.6% |
| broad_type | 7 | 1.7% |
| is_generic | 7 | 1.7% |
| format_string | 3 | 0.7% |
| locale | 2 | 0.5% |
| **any field** | **195** | **46.8%** |

30 of the 32 datasets are touched. This is the number that matters for blast
radius: **a user sees a different record on roughly half their columns**, mostly as
a moved confidence and a moved quality band, even though the type itself moves on
only 3.1%.

At corpus scale, over the stratified sample's 837,625 columns:

| stage | columns changed | share |
|---|---:|---:|
| raw Sense, label | 46,994 | 5.61% |
| raw Sense, confidence | 17 | 0.00% |
| raw Sense, any | 47,011 | 5.61% |
| composed, label | 27,197 | 3.25% |
| composed, confidence | 8,286 | 0.99% |
| composed, quality_band | 7,912 | 0.94% |
| composed, runner_up | 11,593 | 1.38% |
| composed, disambiguation_rule | 19,242 | 2.30% |
| **composed, any field** | **41,864** | **5.00%** |

**A previously-reported "composed output differs on 27,300 columns (3.26%)" was a
LABEL count, not a record count.** The composed record moves on 41,864 columns —
53% more than that figure implies. (The raw-Sense figure here, 47,011, is 56 short
of the 47,067 previously reported; the two implementations are not identical and
that gap is not reconciled.)

Per-family net flow on the composed label (columns in − columns out):

| family | net |
|---|---:|
| representation | −5,049 |
| datetime | +3,409 |
| unknown | −1,380 |
| geography | +1,112 |
| container | +781 |
| technology | +433 |
| finance | +394 |
| identity | +300 |

Largest family transitions: representation→representation 6,265 ·
representation→datetime 5,863 · datetime→representation 2,158 ·
unknown→representation 1,850 · representation→geography 1,481 ·
representation→unknown 1,191 · datetime→datetime 947 · representation→identity 780
· geography→representation 735 · representation→technology 683 ·
representation→container 616 · identity→representation 526.

Quality band moves low→high on 5,050 columns and high→low on 2,862. Where the
label is unchanged, confidence rose on 3,323 columns and fell on 1,570, mean delta
+0.133 — enrichment mostly makes the model **more** confident, which is the
property that makes a wrong answer expensive.

## 3. Which direction is better

### 3a. Ground truth — a tie

Fixture `gold-2026-07-14` (sha256 `760ee4ace670…`, taxonomy `tax-e0baf2e4b3bd`).

The gold headline cannot see this change: it runs `predict_multibranch` over an
FTMB built as one singleton group per column, and singletons are never enriched.
So gold was re-scored through a pipeline that CAN see it — `finetype profile` over
each gold row's own source file, multi-column and at full value depth:

| direction | correct | scored | score |
|---|---:|---:|---:|
| enrichment absent (shipped) | 719 | 828 | 0.868 |
| enrichment served (aligned) | 720 | 828 | 0.870 |

The two directions disagree on **7 of 828 columns** — 4 the enriched direction gets
right, 3 the raw one does. A one-column margin is not a result. This instrument
says the directions are indistinguishable, and it is not powerful enough to say
more.

This is NOT the 0.880 gold headline and is not comparable to it: different
pipeline (native profile with the validation veto), different subset (828 rows over
the 762 source files present locally with a unique file stem).

### 3b. A powered oracle — the un-enriched direction wins

Fixture `corpus-oracle-2026-06-07` (the gated-YDF oracle,
sha256 `195b3af9078e…`, over the stratified sample sha256 `f99196a8da4f…`).

The oracle is column-INTRINSIC: derived from the column's values, blind to the Sense
model and to sibling context, and NULLed wherever fewer than half the column's
values pass the label it proposes. It adjudicates 693,499 of the 837,625 columns.

| direction | agrees with oracle | scored | rate |
|---|---:|---:|---:|
| enrichment absent (shipped) | 351,010 | 693,499 | **0.5061** |
| enrichment served (aligned) | 345,508 | 693,499 | 0.4982 |

Net **−5,502 columns** for aligning. Head to head on the 10,724 columns where the
two disagree: the un-enriched direction is right on **8,113**, the enriched one on
**2,611** — 3.1 to 1 against aligning, far outside anything a coin flip produces.

The oracle is a model, not truth, so the absolute ~50% level is not an accuracy
claim. Only the difference carries meaning, and the difference is one-sided.

Where the loss lands is specific. Splitting the sample by whether a table's headers
are real or pandas artefacts (`__index_level_7__`, `Unnamed: 3`, bare integers):

| tables | columns scored | served | absent | net | served-only | absent-only |
|---|---:|---:|---:|---:|---:|---:|
| artefact-headered (5.0% of tables, 10.3% of columns) | 71,277 | 0.1996 | 0.2646 | **−4,631** | 212 | 4,843 |
| real-headered | 622,222 | 0.5324 | 0.5338 | −871 | 2,399 | 3,270 |

**84% of the loss comes from 10% of the columns**, and it has a mechanism: when
every sibling header is a pandas artefact there is no context to borrow, but
attention mixes them in anyway. The single largest transition in the whole
comparison is `representation.numeric.decimal_number → datetime.component.year`,
4,488 columns, and the columns look like this:

```
header='__index_level_61__'   values=['1961.0', '2.0']
header='__index_level_38__'   values=['1955.0', '3.0']
header='__index_level_6__'    values=['1992.0', '4934.8']
```

A column holding `1961.0` and `2.0` is not a year. The enriched direction adds an
estimated 48,922 oracle-contradicted columns to `datetime.component.year` and gains
16 oracle-confirmed ones.

Even with the artefact-headered tables removed, aligning does not win: −871 net,
3,270 losses against 2,399 wins.

### 3c. The corpus-honest gate — cannot adjudicate

Run in both directions on the same sample:

| baseline → candidate | verdict | triggers |
|---|---|---|
| absent → served | **NO-GO** | `datetime.component.year` (4,486 observed contradicted-in, confirmed-correct 1,649 → 1,665), `datetime.date.jp_era_long` (160 observed, zero confirmed correct either side) |
| served → absent | **NO-GO** | `datetime.date.dmy_short_slash` (213 observed contradicted-in, confirmed-correct 6 → 51) |

The gate refuses BOTH directions, which is the correct reading of what it is: a
relocation detector built to gate a rule change against a baseline, not a scorer
that ranks two models. It cannot answer this question. It is recorded because the
change is exactly the class it exists to catch, and because a NO-GO in the aligning
direction is worth more than a shrug: its trigger is 20× larger by observed
contradiction than the reverse direction's, and buys 16 confirmed-correct columns
against the reverse direction's 45.

## 4. What the sample cannot see

Stated because a gate GO is not coverage, and neither is a gate NO-GO.

- **The corpus columns are value-starved.** The reconstructed tables carry at most
  **8 values per column** (mean 4.9, median 4; 29.7% of columns have 3 or fewer).
  Production samples up to 100. With the value branches starved, the header
  branch's share of the decision is inflated, so this instrument probably
  **overstates** how much any header-side change matters. Its verdict may not
  transfer to full-depth columns — and the ground-truthed instrument, which IS
  full-depth, could not confirm or deny it.
- **The ground-truthed instrument is under-powered.** 828 columns, 7 disagreements.
  It can rule out a large effect; it cannot resolve a small one.
- **The sample is ~3% of GitTables and non-adversarial**, stratified on a retired
  model's calls. It has already certified a change that broke real 19th-century
  dates.
- **The oracle is a model.** It agrees with the shipped pipeline on only ~50% of
  the columns it adjudicates. It is used here only as a fixed referee that is blind
  to the thing under test.
- **10.3% of the sample's columns have artefact headers**, an artefact of GitTables'
  pandas provenance rather than a property of user data. That share is a sampling
  accident, not a measured prior for real tables.
- **Locale is not run-to-run stable** and is excluded from the corpus-scale field
  counts. It IS in the 32-dataset native diff (2 columns), where a single process
  produced each side.
- **Nothing here establishes that `models/sibling-context/` is the artifact the
  shipped model was trained behind.** The mechanism is verified — the trainer
  enriches whenever that directory exists, and the overnight script refuses to run
  without it — but neither `config.json` nor `results.json` in `models/m2v8m-s43/`
  records whether enrichment was on for that specific run.

## 5. Comparability with the historical corpus-scale baselines

Every corpus-scale number on record before this was measured on the **enriched**
pipeline, and serving is un-enriched, so those numbers describe a pipeline the repo
no longer runs. That break is now quantified rather than left open:

- **5.00% of composed columns differ** between the two pipelines (41,864 of
  837,625), and 3.25% carry a different label.
- Against a fixed oracle the un-enriched pipeline scores **+0.79 points** higher
  (0.5061 vs 0.4982 over 693,499 adjudicable columns).

So a historical corpus-scale figure measured on the enriched pipeline is, on this
oracle, about 0.8 points **pessimistic** about what the repo ships today, and
disagrees with it on one column in twenty. Both directions are now recorded against
the same fixture in `evidence/fixtures.json`
(`corpus-oracle-2026-06-07`), so the next corpus-scale measurement has a bar to
compare with rather than an incomparable ancestor.

## 6. Recommendation

**Do not align. Leave the header branch served on raw embeddings.**

- No instrument shows aligning is better. The powered one shows it worse by 3.1:1
  head to head; the ground-truthed one shows a tie.
- Aligning would move the emitted record on **46.8% of columns** of real files, and
  it moves confidence UP more often than down, so its errors arrive confident.
- It is the status quo, so choosing it costs nothing and needs no release.

**What it costs if this is wrong:** the header branch keeps running under a
train/serve mismatch, and whatever it loses to that stays invisible. That loss is
now bounded, not unknown — at most a few columns in 828 on real ground truth, and
worth +0.79 points *in the shipped direction's favour* on the powered oracle.

**If the tie-break rule ("when inconclusive, align with training") is weighted above
a model-oracle result on value-starved data, the answer flips.** That is a judgement
call about which instrument to trust, and it is recorded here so it can be made
deliberately rather than by default.

**The skew is real and this is not the place to close it.** The evidence points at
the training side: a header branch trained WITHOUT enrichment has no mismatch to
serve, and the enrichment is not buying accuracy where we can measure it. Testing
that costs one retrain and no serving change.
