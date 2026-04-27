# Memo: do we need `--model` on the CLI?

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a direction
**Tags:** cli, model, distribution, dx

## The flag inventory

`--model <PATH>` (default `models/default`) appears on **four user-facing
subcommands** and **two hidden ones**:

```
| Subcommand        | Visibility   | main.rs line |
|-------------------|--------------|--------------|
| infer             | user-facing  | 47           |
| schema (table)    | user-facing  | 216          |
| load              | user-facing  | 235          |
| profile           | user-facing  | 341          |
| eval-gittables    | hidden       | 390          |
| eval              | hidden       | 490          |
```

Two related but separate flags:

- `--model-type` — picks `multi-branch | char-cnn | tiered | transformer`.
  Architecture switch. Multi-branch is the default and the only path
  used in the shipped pipeline. Legacy paths still compile.
- `train --output <PATH>` — training output directory. Different intent
  (where to *write* a new model). Not under discussion.

## Are earlier models valuable to users?

**Empirically, no.** Promotion happens only when a new model is a net
improvement or net-zero with hold (decisions 0049, 0058, 0062, 0069).
A user who installs FineType wants `models/default` — that's the whole
point of the symlink. The reasons someone might want an older model:

```
| Reason                                  | Frequency     | Right answer            |
|-----------------------------------------|---------------|-------------------------|
| Reproducing a paper / benchmark         | Vanishingly rare | provide a recipe in docs |
| Bisecting a regression that hit them    | Rare          | report it; we fix it    |
| Specific perf characteristics           | None measured | n/a                     |
| Curiosity / archaeology                 | Possible      | docs recipe             |
```

There is no "use v16 because it's smaller / faster / better at type X"
story for end users. If v19 regressed type X, the right answer is to
fix the regression in v20, not to pin the user to v16.

## Are earlier models accessible to users?

**Technically yes, but undocumented and clunky.** The HuggingFace
`meridian-online/finetype-model` repo currently hosts:

```
sherlock-v4-sibling, sherlock-v7, sherlock-v11, sherlock-v13,
sherlock-v14, sherlock-v16, sherlock-v19-relu-s42  (+ aux: model2vec,
sibling-context, entity-classifier)
```

To use one, a user would have to:

```bash
hf download meridian-online/finetype-model sherlock-v16 --local-dir ./sherlock-v16
finetype profile -m ./sherlock-v16 file.csv
```

Neither step is documented in user-facing materials. The CLI doesn't
auto-resolve a HuggingFace tag — `--model` only accepts a local path.
The DuckDB extension does auto-download via `hf-hub` but uses
`FINETYPE_CI_MODEL` (or its fallback) for the version, not the
`--model` flag.

Also worth noting: only **7 of 51 local model directories** are on
HuggingFace. The rest are training artefacts that never shipped
(`sherlock-v1-flat`, `sherlock-v10-gelu`, snapshots, sweep outputs).
These cannot be used by external users — they don't exist outside
your laptop.

## Who actually uses `--model`?

Two callers, both internal:

1. **Eval scripts** — `eval/profile_eval.sh`, `scripts/sweep_v17.sh`,
   the m-19 sweeps. They read `FINETYPE_MODEL` env var and pass it as
   `--model $FINETYPE_MODEL`. This is the load-bearing path during
   model development — sweeps can't compare candidates without it.
2. **Maintainer A/B comparison** — manual `finetype profile -m
   models/sherlock-v16 ...` vs `-m models/sherlock-v19-relu-s42 ...`
   to debug specific regressions during a promotion review.

Neither is a user need. Both are dev-loop needs.

## Three options

**A. Remove `--model` from user-facing subcommands; keep the env var.**
The CLI internally resolves `FINETYPE_MODEL || "models/default"`. Eval
scripts already export `FINETYPE_MODEL`, so they don't break. The
`--help` output gets simpler. The `-m` short form goes away (used by
no one under measurement).

```rust
// Resolution helper, called by every subcommand that needs a model:
fn resolve_model_path() -> PathBuf {
    std::env::var("FINETYPE_MODEL")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("models/default"))
}
```

**B. Hide `--model` (`#[arg(long, hide = true)]`).** Stays functional,
disappears from `--help`. Smallest churn — eval scripts unchanged,
internal A/B usage unchanged. Mild smell: a hidden flag is a flag
documenting itself as a thing we don't want to talk about.

**C. Keep `--model` as is.** Document a HF-download recipe in
`docs/USAGE.md` for the rare reproducibility case. Lowest CLI churn.
Doesn't address the underlying clutter — `--help` still surfaces a
flag almost no one needs.

Recommendation when we get to deciding: **A**. The CLI surface is
already too busy (see the three other CLI memos from today). `--model`
is a dev-loop concern leaking into user space. Pulling it behind an
env var keeps maintainers fully capable while making the help output
honest about what end users should care about.

The three FineType env vars then become a coherent set, all aligned
to "override default model resolution":

```
| Env var              | Consumer                            | Default              |
|----------------------|-------------------------------------|----------------------|
| FINETYPE_MODEL       | CLI (was: --model flag)             | models/default       |
| FINETYPE_MODEL_DIR   | DuckDB extension                    | HF download cache    |
| FINETYPE_CI_MODEL    | CI download-model.sh                | sherlock-vNN-...     |
```

Today these three are documented in `CLAUDE.md` "Model-name env vars"
under the Release & Model Promotion section. After option A, the
table reads cleaner — no overlap with a CLI flag.

## The companion question: docs for older models

Whichever option we pick, "how to use sherlock-v16" should have **one
documented page**, not zero. Suggested home: `docs/USAGE.md` with a
short section:

```
## Using a specific model version

The default model auto-downloads. To use an older version (for
reproducibility):

  hf download meridian-online/finetype-model sherlock-v16 \
      --local-dir ~/.cache/finetype/sherlock-v16
  FINETYPE_MODEL=~/.cache/finetype/sherlock-v16 finetype profile file.csv

Available versions: see https://huggingface.co/meridian-online/finetype-model/tree/main
```

This converts "undocumented but technically possible" into "documented
niche path" — which is what we want regardless of CLI churn.

## Side note: `--model-type`

Same audit applies. `--model-type` exposes `char-cnn | tiered |
transformer` to users — three architectures that are not the shipped
pipeline (multi-branch, decision 0041). The legacy paths are dead code
for users; they exist for internal regression testing during promotion.

If we ship option A here, `--model-type` should follow the same logic:
hide it (or remove the legacy variants from the public enum and keep
them behind a feature flag). Worth its own memo — calling it out so we
remember.

## Not action yet

Observation memo. This stacks with three other CLI ergonomics memos
from today:

- `2026-04-27-schema-cli-flag-collision.md`
- `2026-04-27-schema-export-verbosity.md`
- `2026-04-27-validate-required-flags.md`

All four feel like a coherent "v0.7.0 CLI polish" spec. Promote
together when ready.
