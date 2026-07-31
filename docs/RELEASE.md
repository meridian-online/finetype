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

After the v0.6.17 release we decoupled CI from the `models/default` symlink (see `spec 2026-04-20-ci-decouple-default-symlink`). The 3-step flow:

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

What it is worth, measured on `m2v8m-s43` (macOS arm64): the artifact goes 30,236,760 →
15,118,424 bytes and the release binary 67,388,816 → **52,148,240**, −22.6 %. The binary
sheds 122,240 bytes more than the artifact does, and that is expected rather than a
mystery: 4,096 of it is the 16 KB-aligned `__TEXT` segment crossing one further page
boundary, and 118,144 is `__LINKEDIT` carrying 3,692 fewer sha256 code-signature page
hashes at 32 bytes each. Full workings in `evidence/half-precision-value-encoder.md`.

Then re-check the promotion scoreboard **on the converted artifact**. Half precision is a
packaging change, not a free one: it perturbs the encoder's rows in the last few
thousandths, and an argmax over 244 classes is entitled to move. The gold re-score through
the real `profile` path (`scripts/score_gold_anchor.py predict … --binary
./target/release/finetype`, with `FINETYPE_MODEL` pointed at each variant) is what
establishes the **labels** did not.

Know what that scorer can see, because it is less than it looks. `predict` writes one row
per column — `file_content_sha256`, `column_name`, `predicted_label`, and a `confidence`
field it declares in the header and then writes as the empty string for every row, because
`_profile_column` returns `x-finetype-label` and nothing else. Two identical prediction
files therefore establish **label**-invariance and nothing more: confidence, quality band,
runner-up, disambiguation and detected locale can all move without shifting that file's
sha256. Under half precision the confidences *do* move — **170 of 843** resolvable gold
columns, maximum |Δ| **0.0007**, with **0** label and **0** quality-band changes, on fixture
`gold-2026-07-14`. That is the arithmetic consequence of the storage change and not a
regression, but a release note that calls the output unchanged on the strength of that file
is overstating what was measured.

For a whole-record claim, diff the whole record. `scripts/encoder_dtype_record_diff.py`
does exactly that — it drives `finetype profile -o csv` once per gold column and compares
every emitted field — and `evidence/half-precision-value-encoder.md` is the record it
produced, with the committed diff JSONs beside it. Use
`scripts/compare_composed_records.py` for the fields Sharpen can move.

Two traps that measurement found, both of which will bite the next person:

- **`detected_locale` is not run-to-run stable.** A same-binary, same-artifact repeat of the
  whole 843-column sweep moved it on one column, and eight consecutive profiles of one
  fixed column returned six distinct locales. Run the repeat *first* and subtract that
  floor; a single locale flip attributed to a dtype change is noise about noise.
- **The fast corpus-honest gate cannot see an encoder change at all.**
  `scripts/corpus_honest_gate_fast.sh` builds its raw-Sense cache once, from the candidate
  binary, then resharpens that one cache with both binaries — and `resharpen` composes from
  cached sense labels without the value encode. Both arms therefore carry the *same*
  encoder output whatever the artifact says, so the gate returns green by construction. Do
  not quote it here.

Use the `profile` path, not the offline one, and know why. The offline scorer,
`predict_multibranch`, takes its branch features **precomputed** from an FTMB, so whether
it opens the value encoder at all depends on the model config. For a config with **no**
`value_attention` block — which is every model currently shipped, including `m2v8m-s43`,
whose value encoder is wired through `value_embed_model` and resolved by the *inference*
loader instead — it never reads the encoder, and a "no difference" from that path would
measure nothing. For a config that **does** carry `value_attention`, it loads the
directory given by `--value-encoder` to build the attention pool and hard-errors if that
flag is missing, so there it does see the change. Either way `profile` is the path a user
runs, so that is the one the re-score has to go through.

See also: `DEVELOPMENT.md` for the three model-name env vars (`FINETYPE_CI_MODEL`, `FINETYPE_MODEL`, `FINETYPE_MODEL_DIR`) — each read by exactly one consumer.
