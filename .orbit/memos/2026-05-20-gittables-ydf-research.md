# Gittables × YDF — research path toward supervised signal

Gittables was selected for its size; we should use it through multiple lenses, not just profile-eval. Proposal floated: train a YDF model (https://ydf.readthedocs.io/en/stable/) on the FineType taxonomy, run it across the gittables corpus, see what we can learn — potentially toward a supervised-learning approach.

## The constraint

Model agreement is not ground truth. Two models trained on the same labels disagreeing tells us which columns are ambiguous, not which prediction is correct. YDF as a second opinion is fine; YDF as a substitute for ground truth is the trap.

## Cheaper first move

Gittables ships with DBpedia / Schema.org semantic-type annotations per column. Where those overlap the FineType taxonomy, they function as weak ground truth — directly answering "where is Sense wrong?" rather than "where do two models disagree?" The overlap map is the prerequisite artefact; YDF runs on top of it as targeted diagnostic, not as the headline.

## Open threads for discovery

- **Overlap mapping** — which DBpedia / Schema.org annotations map cleanly to FineType types? Coverage fraction? Near-match handling?
- **YDF's role** — corpus-wide diagnostic vs. targeted second opinion on columns without annotations
- **Supervised endgame** — do derived labels feed v20 training, the m-19 eval-corpus expansion, or both? How do we keep train/eval leakage controlled (decision 0057)?
- **Eval shape** — unit of measurement (per-column, per-table, per-type), reporting cadence, comparison against the current 369/448 profile-eval baseline
- **Python dependency** — YDF is Python; fine for offline research, but anything that bleeds into runtime conflicts with zero-Python policy

## Likely attachments

Tributary to existing eval-corpus or model-training capabilities — not a new card. Specs fall out of the discovery session.
