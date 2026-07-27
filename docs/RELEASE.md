# Release & Model Promotion

Reference for promoting a new model and cutting a release.

## Runtime dependency: the `duckdb` CLI (choice 0100, v0.6.32)

`profile` and `validate` shell out to the external `duckdb` CLI for all
CSV/Parquet ingestion, so it is a **hard runtime dependency** (on PATH). The
Homebrew formula template (`.github/workflows/release.yml`) declares
`depends_on "duckdb"`. This is a **shell-out, not a link** — the cross-platform
release build is unchanged (no `libduckdb` compile), so the Windows/MSVC
amalgamation risk that applies to a `duckdb` *pin bump* (see the binary-release
pre-flight) does **not** apply to ingestion. Any CI job that runs `profile`/
`validate` end-to-end must install the `duckdb` CLI (the smoke job does).

## Promotion flow (new model → release)

After the v0.6.17 release we decoupled CI from the `models/default` symlink (see `.orbit/specs/2026-04-20-ci-decouple-default-symlink/`). The 3-step flow:

1. **Publish to HuggingFace** — upload the trained model directory to `meridian-online/finetype-model` on HF.
2. **Bump `FINETYPE_CI_MODEL`** in `.github/workflows/ci.yml` and `.github/workflows/release.yml` (workflow-level `env:` blocks).
3. **Flip `models/default`** — `ln -sfn <new-model> models/default`.

Steps 2 and 3 may ship in the same PR. Step 1 must precede step 2 (or step 2 can be deferred if the promotion is purely a runtime change).

**Quality gates before any of this.** A candidate clears the promotion-order scoreboard *before* the flip (CLAUDE.md "Promotion order"): gold-anchor → drift proxy → gold + rare-type scoreboard → **representative accuracy (advisory)** → **external-data band (advisory)** → corpus-honest gate (**blocking**). The representative band (`eval/repr/representative_corpus.tsv`, scored `score_gold_anchor.py … --reframe`) is reported alongside gold and flags an advisory drop on the candidate-vs-v19 delta; it never blocks on its own. The external-data band (`scripts/external_band.py` over `eval/datasets/gold_external/`, held labels live-derived from `gold_corpus.tsv`) profiles whole real external tables the model never trained on and reports the same candidate-vs-baseline delta; also advisory, also never blocking. Only gold + the corpus-honest relocation gate block. See specs `2026-06-18-representative-accuracy-gate` and `output/external-band/2026-07-11-first-reading.md`.

A non-blocking drift check (`.github/scripts/check-ci-model-drift.sh`) warns in CI when `FINETYPE_CI_MODEL` and `models/default` disagree — legitimate during promotion PRs, but visible so divergence isn't silent for weeks.

### Step 0: pack the encoders in half precision before uploading

A Model2Vec encoder's `model.safetensors` is a **lookup table**, not a trained graph.
`Model2VecResources::from_bytes` up-casts it to F32 before a single token is embedded, so
whether it was stored F16 or F32 is invisible to inference — and F32 costs twice the
download, twice the `include_bytes!` payload in the release binary, and twice the resident
bytes while the file is being parsed. The `models/model2vec` header encoder has always
shipped F16; the dual-encoder value branch did not, which is the whole of the difference.

Convert before uploading to HuggingFace, once per encoder directory:

```sh
scripts/encoder_dtype.py inspect models/<model>/value_model2vec/model.safetensors
scripts/encoder_dtype.py to-f16 models/<model>/value_model2vec/model.safetensors
```

The tool refuses (exit 1, nothing written) rather than storing a value F16 cannot hold as
`inf`, reports any element that rounds to zero, and is a no-op on a file that is already
F16 — so running it twice, or on a directory you are not sure about, is safe. Rounding is
IEEE round-half-to-even out of CPython's `struct`, which is why the same input produces
the same bytes on a laptop and on a CI runner: these bytes get compiled into a binary, and
a per-platform difference there is a byte-drift failure waiting to happen.

Then re-check the promotion scoreboard **on the converted artifact**. Half precision is a
packaging change, not a free one: it perturbs the encoder's rows in the last few
thousandths, and an argmax over 244 classes is entitled to move. The gold re-score through
the real `profile` path (`scripts/score_gold_anchor.py predict … --binary
./target/release/finetype`, with `FINETYPE_MODEL` pointed at each variant) is what
establishes it did not.

Note that the *offline* scorer, `predict_multibranch`, reads **precomputed** features from
an FTMB and therefore never touches the value encoder at all. It cannot see this change,
and a "no difference" from that path would measure nothing.

See also: `DEVELOPMENT.md` for the three model-name env vars (`FINETYPE_CI_MODEL`, `FINETYPE_MODEL`, `FINETYPE_MODEL_DIR`) — each read by exactly one consumer.
