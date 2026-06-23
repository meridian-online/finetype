# These offline gate verdicts are INVALID — do not use

predict_multibranch emits RAW pre-veto Sense argmax (0 unknowns). The corpus-honest gate's
baseline/oracle is the NATIVE `finetype profile` x-finetype-label = COMPOSED, post-veto
(m2v-244 native: 101,338 unknowns from the validation veto). Comparing pre-veto candidate
vs post-veto baseline made the veto-demoted labels (binary, year, numeric_code) look like
massive over-emissions. The m2v8m "NO-GO / 22 triggers" is an artifact, NOT potion-8M's
real corpus behaviour.

A valid gate needs COMPOSED candidate predictions — via native `finetype profile` (needs the
dual-encoder Rust work) OR compose_predictions/FINETYPE_INJECT_LABEL (another per-column pass).
