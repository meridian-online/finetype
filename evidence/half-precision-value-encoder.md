# Half precision for the value encoder — what it cost and what it bought

The dual-encoder value branch shipped its Model2Vec lookup table as F32 while the header
encoder in the same binary shipped F16, and `Model2VecResources::from_bytes` up-casts both
before a single token is embedded. This is the record of converting it: what the artifact
and the binary now weigh, what the whole emitted record does under the change, and which
instruments cannot see it.

Measured 2026-07-28 on macOS arm64 (Darwin 25.1.0), `cargo build --release -p finetype-cli
--bin finetype`, default features (`cpu` + `embed-models`), `[profile.release] lto = true,
codegen-units = 1`.

## The artifact

`models/m2v8m-s43/value_model2vec/model.safetensors`, converted with
`scripts/encoder_dtype.py to-f16`:

| | F32 | F16 |
|---|---|---|
| bytes | 30,236,760 | 15,118,424 |
| sha256 | `f65d0f325faadc1e121c319e2faa41170d3fa07d8c89abd48ca5358d9a223de2` | `16709ebefa4364c0950b2107d21b69cf1c7c873b2686f61533ff2c8b214484f9` |
| mode | 0644 | 0644 |
| `embeddings` | F32 `[29528, 256]`, 30,236,672 B | F16 `[29528, 256]`, 15,118,336 B |

Payload delta **15,118,336**. Mode preserved. `find models -type f ! -perm -044` returns
nothing. Both versions are preserved outside the repository at
`/Users/hugh/github/meridian-online/.finetype-model-backups/`.

## The binary

| | F32 | F16 |
|---|---|---|
| bytes | 67,388,816 | 52,148,240 |
| sha256 | `fbbc907143e1e5b8c07f16325ec151b3d45058cc31521432285bae1830007f43` | `7caff56583b2bdf476f3a09e921e631bc6bfce2f5ef43087c49d8df679b60c3a` |

Delta **15,240,576**, **−22.6 %**.

### The baseline is 152,896 bytes below the figure this change was proposed against

The proposal quoted 67,541,712 → ~52,301,136. Both measured sizes are exactly **152,896
bytes lower**, so the *delta* is unchanged and the saving is what was claimed; only the
starting point moved. See "Baseline drift" below for what accounts for it.

### The 122,240-byte excess, accounted for to the byte

The binary shrank 15,240,576 while the payload shrank 15,118,336 — an excess of
**122,240**, reproduced here exactly. It is two things, measured on these two binaries
rather than on a synthetic stand-in.

`otool -l`, segment file sizes:

| segment | F32 | F16 | delta |
|---|---:|---:|---:|
| `__TEXT` | 64,176,128 | 49,053,696 | 15,122,432 |
| `__DATA_CONST` | 819,200 | 819,200 | 0 |
| `__DATA` | 49,152 | 49,152 | 0 |
| `__LINKEDIT` | 2,344,336 | 2,226,192 | 118,144 |
| | | | **15,240,576** |

Within `__TEXT`, exactly one section moved:

| section | F32 | F16 | delta |
|---|---:|---:|---:|
| `__TEXT,__const` | 53,132,760 | 38,014,424 | **15,118,336** |
| `__text`, `__stubs`, `__stub_helper`, `__gcc_except_tab`, `__cstring`, `__ustring`, `__unwind_info`, `__eh_frame` | | | 0 |

**4,096 — `__TEXT` segment alignment.** The sections shrank by exactly the payload delta,
the segment by 4,096 more. Both `__TEXT` sizes are whole multiples of **16,384**, the arm64
VM page: 64,176,128 = 16384 × 3917 and 49,053,696 = 16384 × 2994, a difference of 923 pages.
The content shrank by 15,118,336 = 922 × 16,384 + 12,288, so rounding up crosses one further
16 KB boundary and the segment sheds 923 pages where the payload accounts for 922. **This
residue is not a constant**: for a payload delta with a different remainder it would have
been 0, 4,096, 8,192 or 12,288.

**118,144 — the ad-hoc code signature.** Three independent readings agree:

- `__LINKEDIT` file size, 2,344,336 → 2,226,192 (Δ 118,144)
- `LC_CODE_SIGNATURE` `datasize`, 522,544 → 404,400 (Δ 118,144)
- `codesign -dvvv` CodeDirectory `size`, 522,520 → 404,376 (Δ 118,144)

The mechanism is the page-hash count: `hashes=16325` → `hashes=12633`, a difference of
**3,692**, at `Hash type=sha256 size=32` — 3,692 × 32 = **118,144**. Both are
`flags=0x20002(adhoc,linker-signed)`.

4,096 + 118,144 = **122,240**. Nothing is left over. The file-size identity closes on both:
`dataoff + datasize` = 66,866,272 + 522,544 = 67,388,816 and 51,743,840 + 404,400 =
52,148,240.

## What the change does to the output

Fixture **`gold-2026-07-14`** — resolved with `scripts/evidence.py resolve-fixture --path
eval/gold/gold_corpus.tsv`, never hard-coded. `eval/gold/gold_corpus.tsv` sha256
`760ee4ace67064edd465d245103677e30171a9ce4bb07decc44bd69f914586a7`, 1037 rows, adjudicated
under `tax-e0baf2e4b3bd`; the checkout is at `tax-bc0dc59de853` and `scripts/evidence.py
verify` reports every label the fixture uses is still a type the checkout defines, so its
scores stay attributable.

**843 of the 1037** gold columns resolve from `eval/gittables/corpus_pass/columns.parquet`
(843 matching rows, 843 distinct keys — no duplicates, so the join needs no tie-break).
Measured through `finetype profile -o csv`, one single-column CSV per column, by
`scripts/encoder_dtype_record_diff.py`.

### F32 build and artifact vs F16 build and artifact

| field | differ |
|---|---|
| `column` | 0 / 843 |
| `type` (the label) | **0 / 843** |
| `confidence` | **170 / 843**, max abs delta **0.0007** (`country_id:id`, 0.4221 → 0.4228) |
| `quality_band` | **0 / 843** |
| `runner_up` | 0 / 843 |
| `broad_type` | 0 / 843 |
| `format_string` | 0 / 843 |
| `transform` | 0 / 843 |
| `is_generic` | 0 / 843 |
| `samples_used` | 0 / 843 |
| `non_null` | 0 / 843 |
| `null` | 0 / 843 |
| `disambiguation` | 0 / 843 |
| `locale` | 1 / 843 — **at the noise floor, see below** |
| **whole record** | **171 / 843** |

The confidence moves are small and unbiased: median abs delta 0.0001, mean 0.000141,
97 up and 73 down. At the 4-decimal resolution `profile -o csv` prints, the magnitude
histogram is 133 moves of 0.0001, 24 of 0.0002, 5 of 0.0003, 1 of 0.0004, 5 of 0.0005 and
2 of 0.0007. Confidence across the 843 columns ranges 0.2098 to 1.0000, so the largest
move is about one part in six hundred of the smallest confidence in the set.

### `detected_locale` is nondeterministic, and the one flip is that

A same-binary, same-model-directory repeat of the whole 843-column measurement differs in
**one** field on **one** column: `locale` on `zip` (sha `14d9f1620452`), DE_CH → NB. The
same column is the single locale difference in every comparison run here, taking a
different value each time — DE_CH, NB, SL, ES_AR across four runs. Eight consecutive
profiles of a fixed 5-value `zip` column on one binary returned six distinct locales
(MS, AR_MA, SV, TH, SV, HR, ID, SV) with every other field identical.

`crates/finetype-cli/src/cmd_run.rs:107` already declines to emit this field for that
reason. The locale flip is therefore **not an effect of half precision** and is excluded
from the claim. A second noise-floor pair (F16 binary and F16 artifact, two runs) differed
in **0 / 843** fields, so `confidence` is deterministic and the 170 is repeatable.

### The rebuild contributes nothing — control

Running the **F16-era binary** against the **F32 artifact** and comparing to the F32-era
binary against the same F32 artifact: 0 / 843 on every field except the one unstable
locale. So none of the 170 confidence moves come from recompiling; comparing the two
artifacts under one binary reproduces the headline exactly (170 / 843, max 0.0007).

### Labels over the full fixture

`scripts/score_gold_anchor.py predict` covers rows the corpus parquet does not, by falling
back to vendored CSVs. It is cited here **for labels and the score only**: it declares a
`confidence` column and writes the empty string into it for every row, so its output is
structurally incapable of recording anything else.

Over all **1037** rows, with the F32 and the F16 artifact under one binary, the two
prediction files are byte-identical — sha256
`bb5530fdb23d35ceaeb158643a8ac29ce17df504d3dd2703bbeb8519289bc2d2` — and **0 / 1037**
labels differ. Both score **880 / 1037 = 0.849** (95% CI 0.826–0.869) under
`score_gold_anchor.py score --reframe`, with identical abstention (136/1037 = 0.131),
macro precision (0.897) and macro recall (0.822).

The absolute is not the 889/1037 = 0.857 recorded when this change was proposed. That
figure was measured on an earlier checkout; this one is `tax-bc0dc59de853`, after the
compact-YYYYMMDD date change edited `labels/definitions_datetime.yaml`. What the
comparison here establishes is that **the two dtypes score the same number**, which does
not depend on which number it is.

## Instruments that cannot see this change

**The fast corpus-honest gate is blind to it by construction.**
`scripts/corpus_honest_gate_fast.sh` builds its raw-Sense cache **once**, from the
candidate binary, and then resharpens that single cache with both binaries. `resharpen`
composes from cached sense labels without the value encode, so the encoder's contribution
is baked identically into both arms and the A/B difference is zero whatever the dtype.
Demonstrated rather than argued: `finetype-F32 resharpen` and `finetype-F16 resharpen` over
the same three-column cache produce byte-identical output, sha256
`bd008d558f4a0957865a69329a2b8144d307bbf04055df12f64a1491fdbd1970`. A green verdict from it
here would measure nothing, so it was not run.

**A label-only prediction file is not a whole-record result.** Two `score_gold_anchor.py
predict` outputs matching by sha256 establish label-invariance and nothing more — the
finding that got two earlier pull requests refused, and the reason
`scripts/encoder_dtype_record_diff.py` exists.

## What this does not establish

- **194 of the 1037 gold columns are outside the record comparison** — they do not resolve
  from the corpus parquet. They are covered for **labels** by the full-fixture score above,
  not for confidence, quality band, runner-up, disambiguation or locale.
- **The columns are classified on very few values.** The parquet's
  `sample_values_truncated` yields 5,734 values across 843 columns — between 1 and 8 each.
  A pooled value embedding over eight tokens is not the regime a user profiling a real
  table is in, and it is where half precision has the least room to matter.
- **One column per CSV**, so nothing here exercises sibling or cross-column context.
- **macOS arm64 only.** The Linux and Windows binaries were not built, and the code
  signature term of the 122,240 is a Mach-O phenomenon that will not appear on either.
- **The gold fixture is not adversarial.** It is curated ground truth, not a search for
  inputs where half precision changes an argmax.
- **Nothing was published.** The converted artifact has not been uploaded to
  `meridian-online/finetype-model`, so no `cargo install` and no CI run downloads it yet.

## Baseline drift — measured, not assumed

The F32 binary built here is **67,388,816** bytes; the figure this change was proposed
against is 67,541,712. The F16 side lands the same 152,896 bytes low, so the shift is in the
*baseline*, not in the conversion, and the 15,240,576-byte saving is unaffected.

**Closed by the review — it is the intervening merges, and the figure reproduces exactly.**
Checking out `c1a6e61` (the pre-merge state this change was proposed from, based on `#78`
and therefore without the three merges that landed after it), restoring the F32 artifact
and running the same `cargo build --release -p finetype-cli --bin finetype` produces
**67,541,712** bytes — the proposed figure, to the byte. So the 152,896 is source drift
across those merges and has nothing to do with the dtype.

The circumstantial reading that pointed both ways was the right one on balance: the window
deleted 2,745 lines across `finetype-model` and `finetype-cli`, including all of
`semantic.rs` and 1,276 lines of `inference.rs`, against 1,045 bytes added to the
`include_str!`-embedded `labels/definitions_datetime.yaml`.

Both endpoints of the release claim reproduce byte for byte from a cold rebuild on the same
machine: F32 restored → 67,388,816, sha256 `fbbc9071…30007f43`; F16 → 52,148,240, sha256
`7caff565…79b60c3a`. The backup at
`output/model-artifact-backups/value_encoder_m2v8m-s43_F32_original.safetensors` is what was
restored, so the conversion is reversible in practice and not only on paper.

## Two things the self-test does not cover

Both were found by mutating the tool a second time during review, and both would produce a
**false null** — which is why neither can have manufactured the result above, since the
result is 170 and not 0. Each is re-derived by making the named edit and running
`scripts/encoder_dtype_record_diff.py self-test`, which still passes:

- **`cmd_emit`'s environment is untested.** Delete `env["FINETYPE_MODEL"] = str(args.model)`
  and every case still passes; both arms would then read `models/default` and every
  comparison would report 0 / 843. The 48 cases are pure-Python over CSV text and never
  reach the subprocess.
- **`only_b` is untested.** Replace `only_b = sorted(k for k in b_by if k not in a_by)` with
  `only_b = []` and every case still passes; a row present only on the B side would vanish
  from the report. The unmatched-row case exercises `only_a` only. In this measurement
  `rows_a = rows_b = common = 843` in all five diffs, so nothing was hidden.

Three further mutations — comparing confidence with a `1e-3` tolerance, rounding it to 3 dp
before comparing, and excluding a declared-unstable field from the whole-record count — were
each killed by a named case.

## Reproducing this

```sh
scripts/encoder_dtype.py inspect models/m2v8m-s43/value_model2vec/model.safetensors
scripts/encoder_dtype.py to-f16 models/m2v8m-s43/value_model2vec/model.safetensors

scripts/encoder_dtype_record_diff.py prepare --out output/half-precision/samples.tsv
scripts/encoder_dtype_record_diff.py emit --binary <bin> --model <dir> \
    --samples output/half-precision/samples.tsv --out output/half-precision/<name>.tsv
scripts/encoder_dtype_record_diff.py diff --a <A>.tsv --b <B>.tsv --noise-floor locale \
    --json evidence/half-precision-<name>.json

scripts/encoder_dtype_record_diff.py self-test              # 48 cases
scripts/encoder_dtype_record_diff_mutations.py              # 11 wrong implementations, 0 survive
```

The record TSVs live in `output/half-precision/` (regenerable, blanket-ignored); the diff
JSONs behind every count above are committed next to this file.
