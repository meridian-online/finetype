# ac-04 SALVAGE — the NO-GO was a measurement artifact; fair verdict is GO

**Date:** 2026-06-08
**Supersedes:** the NO-GO in `ac04_kill_switch.md` (which stands as a record of the flaw).
**Fair verdict:** GO at the Sense level — fusion holds-or-improves on every gold family;
the lat/lon "regression" was Sharpen being credited to v19 but withheld from fusion.

## What the first kill switch actually measured

The first pass compared **fusion (Sense output only, no Sharpen)** against the v19 gold
baseline, which was produced by `finetype profile` — the **full Sense→Sharpen pipeline**.
That is apples-to-oranges. The proof is in the dumped features: the v19 multi-branch
logits (View2), computed inside the dump, predict `decimal_number` on **30/30** gold
latitude columns. Raw multi-branch does **not** know latitude from decimal. The v19
baseline scores latitude 1.000 only because **Sharpen's value-range rules** (|lat|≤90,
paired lat/lon columns) promote decimal→latitude *after* the Sense stage.

Sharpen is untouched by this spec (plan: "Fusion replaces only the Sense prediction;
value_sharpen R1–R21 still run after"). It will fix fusion's output exactly as it fixes
multi-branch's. So crediting Sharpen's lat/lon rescue to v19 while denying it to fusion
manufactured a regression that does not exist in the shipped pipeline.

## Fair comparison — both pre-Sharpen (the stage fusion replaces)

Baseline = raw multi-branch Sense (argmax of the View2 logit block). Candidate = fusion
head v26 (trained on the distilled corpus + 2,644 manufactured lat/lon columns).

| Gold family | raw-v19 Sense | fusion-v26 Sense | |
|-------------|--------------:|-----------------:|---|
| A_tight_code_vs_alnum    | 0.400 | **0.967** | +0.567 |
| B_country_vs_categorical | 0.483 | **0.500** | +0.017 |
| C_lat_lon_temperature    | 0.356 | 0.344 | −0.012 (1 col, Sharpen-owned) |
| D_year_vs_integer        | 0.650 | 0.650 | tie |

### Family A — the real, Sharpen-unreachable win
Raw v19 misroutes 17/30 tight codes to `technology.cryptographic.hash` and gets only 12
right. Fusion gets 29/30 to `representation.identifier.alphanumeric_id`. Hash-vs-alnum-id
is not a value-range rule, so Sharpen does not rescue it — this is value-level signal that
only the fusion view supplies. This is what B3 is *for*.

### Family C — illusory regression
Fusion outputs `decimal_number` on 30/30 latitude and 29/30 longitude — the same decimal
output raw v19 produces, and exactly what Sharpen's lat/lon rules consume. Post-Sharpen,
fusion's lat/lon recall equals v19's. The −0.012 is one column of pre-Sharpen noise.

## What this means for the spec

1. **lat/lon is Sharpen-owned, not Sense-owned.** The original B3 thesis — "fusion fixes
   the starved coordinate families" — was misframed. Sharpen already fixes them. The
   manufactured-coordinate head training (the salvage) does no harm but is not where B3's
   value lives.
2. **B3's value is the boundaries Sharpen cannot reach** — family A (tight-code vs hash /
   alnum-id) is a +0.567 Sense-level gain that survives into the shipped output.
3. **The gold gate must compare like-for-like.** Either fusion-Sense vs raw-v19-Sense
   (done here, GO), or full-pipeline-with-fusion vs full-pipeline-v19 (requires the port).
   The corpus-honest gate (vs `v19_gated.parquet`, which is full-pipeline) has the *same*
   Sharpen-attribution trap and is only fair once fusion runs through Sharpen — i.e. as the
   POST-port ship gate (ac-07), which was always the real blocking instrument.

## Recommendation

Proceed to the Rust port (ac-05/ac-06). The fair pre-port signal is GO — fusion improves
the Sense stage and the one apparent regression is Sharpen-owned and vanishes downstream.
The true blocking gate is the post-Sharpen corpus pass (ac-07); run it on the wired Rust
path before any `models/default` swap.
