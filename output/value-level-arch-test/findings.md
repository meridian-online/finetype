# Value-level architecture test — findings

The pivot ac-07 recommended, now run. Question: on the non-starved manufactured
corpus, can a **value-level CharCNN** (one value at a time, no column-level sibling
entanglement) do what the multi-branch additive blend could not — **learn** the starved
confusion family **without collapsing** the plain-number classes?

Two arms, identical recipe (char-cnn, feature_dim=0, 8 epochs, seed 42), shared test:
- **Arm A (starved baseline)** — train on cleaned real GitTables only (latitude=10,
  longitude=21 distinct values).
- **Arm B (manufactured)** — same split plus manufactured reference diversity
  (latitude/longitude=10,000 each).

`decimal_number`/`integer_number` training is **identical in both arms**, and their test
is **100% real-corpus**, so any decimal/integer movement between arms is attributable
purely to adding coordinate diversity — the clean, unconfounded read.

## Result

| confusion family | Arm A (starved) | Arm B (mfg) | read |
|---|---:|---:|---|
| `geography.coordinate.latitude` | **0.000** | **0.740** | LEARN: 0 → 74% |
| `geography.coordinate.longitude` | **0.000** | **0.822** | LEARN: 0 → 82% |
| `representation.numeric.decimal_number` | 0.975 | **0.763** | HOLD: −21pp |
| `representation.numeric.integer_number` | 0.548 | **0.356** | HOLD: −19pp |
| `geography.location.city` | 0.672 | 0.801 | +13pp |
| `geography.location.region` | 0.089 | 0.116 | ~flat (both poor) |
| `geography.address.postal_code` | 0.340 | 0.932 | +59pp |

Overall recall (162 types, 30,684 values): Arm A **0.678** → Arm B **0.788** (+11pp).

## Read 1 — the value-level model LEARNS the starved family

Arm A has no concept of coordinates: every test latitude/longitude lands on
`decimal_number` (96.8% / 79.7%) or `plain_text`. Arm B recalls 74% / 82%. Manufacturing
+ value-level architecture dissolves starvation at the **model** level — the thing the
multi-branch additive blend never achieved (it could not even be trained: the proxy
pre-check blocked it three times).

**Caveat, stated honestly:** the latitude/longitude test set is 100% manufactured-source
(no real-corpus coordinates exist to test on — that *is* the starvation). So Arm B's
74%/82% is on held-out values from its own training distribution; we cannot claim it
generalises to arbitrary field-collected coordinates. What we *can* claim is robust: the
model acquired a coordinate concept that did not exist before, because Arm A — scored on
the identical test — gets 0%. The test is not trivially easy; it requires having learned
coordinate diversity.

## Read 2 — it does NOT cleanly HOLD the numeric boundary, but it degrades gracefully

Adding coordinates costs `decimal_number` 21pp and `integer_number` 19pp. The lost
decimals bleed onto `longitude` (14.2%) and `latitude` (6.9%) — the model pulls
decimal-shaped values toward the coordinate classes it just learned.

**This is the same tension that killed the multi-branch blend, but the failure magnitude
is categorically different.** Multi-branch wiped `decimal_number` emission on the real
corpus from 31.29% → **0.29%** (a ~99% collapse — the class effectively vanished).
The value-level model retains `decimal_number` as a **76%-recall, fully-populated
class**. It pays a boundary tax; it does not destroy the category. Graceful degradation
vs catastrophic collapse.

(The two numbers are different metrics — multi-branch's is Sense emission rate on the
corpus, value-level's is per-type recall on a balanced test — so compare the *shape*, not
the digits: one class survived as viable, the other was annihilated.)

## The deeper finding — the latitude/decimal boundary is inherently column-level

`40.7128` is a valid latitude **and** a valid `decimal_number`. With only the value —
no header, no sibling column context — the distinction is genuinely underdetermined. The
two arms sit at opposite ends of the same unavoidable trade-off:
- Arm A, with no coordinate prior, calls every decimal-shaped value a decimal: 97.5%
  decimal recall, 0% latitude.
- Arm B, having learned coordinates, splits that decimal-shaped space: gains lat/lon,
  loses 21pp of decimals to coordinate labels.

You cannot have both at the value level, because the disambiguating signal is not in the
value — it is in the column. A column of `40.7, 51.5, -33.8` is coordinates; a column of
`3.14, 2.71, 1.41` is decimals; any single member is ambiguous. **This is the strongest
argument yet for the B3 late-fusion model: a value-level prediction supplies the broad
type, and column-level context (sibling distribution, header) is the tiebreaker on
exactly this coordinate/decimal boundary.**

## Verdict & next move

- **Value-level architecture is validated** on the two things multi-branch failed: it
  learns starved families (lat/lon 0 → 74%/82%, postal +59pp) and keeps the numeric class
  alive as a viable category (decimal 76%) rather than collapsing it.
- **The residual coordinate/decimal ambiguity is irreducible at the value level** — it is
  a column-level decision by nature.
- **GO on drafting the B3 late-fusion model** as the consumer: value-level CharCNN as the
  broad-type substrate, column context as the boundary tiebreaker. HOLD the additive
  multi-branch retrain route (dead, 0-for-4). Manufacturing stands as the reusable
  diversity source feeding the value-level substrate.

## Reproduce

```
python3 scripts/mining_factory/build_value_level_split.py
finetype train -d output/value-level-arch-test/value_train_mfg.ndjson  --model-type char-cnn --epochs 8 --seed 42 -o output/value-level-arch-test/model_mfg_nf
finetype train -d output/value-level-arch-test/value_train_base.ndjson --model-type char-cnn --epochs 8 --seed 42 -o output/value-level-arch-test/model_base_nf
python3 scripts/mining_factory/eval_value_level.py --model output/value-level-arch-test/model_mfg_nf  --label armB_mfg
python3 scripts/mining_factory/eval_value_level.py --model output/value-level-arch-test/model_base_nf --label armA_base
```

**Inference note (latent CLI bug found this session):** `finetype infer` on a char-cnn
trained with `--use-features` silently zero-fills the 37-dim feature vector
(`classify_batch` → `model.infer` → `infer_with_features(.., None)`), corrupting the head
and collapsing every prediction to `plain_text`. The feature-supplying path
(`classify_batch_with_features`) exists in `finetype-model` but no CLI command calls it.
Both arms here are trained `feature_dim=0` so `infer` is correct; the bug is filed
separately. It affects any feature-trained char-cnn run through `finetype infer`,
including the prior `models/char-cnn-v15-gittables`.
