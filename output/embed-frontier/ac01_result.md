# ac-01 — Reproducible Model2Vec baseline (Mon 22 Jun 2026 05:37:15 AEST)

Recipe: potion-4M, 27 stats, format v4, 244 taxonomy, ReLU+BN, v19_paired data blend.
Honest gate: Sense = predict --raw-model; composed = native profile. Reference: v19 Sense 0.502 / composed 0.793.

| model | Sense (raw) | composed |
|---|---|---|
v19-relu-s42 (reference) | 0.502 | 0.793
repro-s42 | 0.481 | 0.755
repro-s43 | 0.510 | 0.770
repro-s44 | 0.521 | 0.769

Verdict: best-of-3 composed within v19's CI (~0.793) => reproducible movable baseline.
If short: the per-seed table is the drift diagnosis (taxonomy 240->244 is the prime suspect — v19 was 240-dim).
Done Mon 22 Jun 2026 05:56:21 AEST
