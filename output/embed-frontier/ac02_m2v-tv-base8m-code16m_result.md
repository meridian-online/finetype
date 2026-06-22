# ac-02 [m2v-tv-base8m-code16m] — bigger-static embed vs m2v-244 (Tue 23 Jun 2026 03:53:52 AEST)

potion: minishlab/potion-base-8M ++ minishlab/potion-code-16M | embed_dim 2048 | Sense-vs-Sense (offline, no truncation)
baseline m2v-244-s44: Sense 0.521 / composed 0.769  (v19: Sense 0.502 / composed 0.793)

| seed | Sense | composed |
|---|---|---|
| s42 | 0.511 | 0.763 |
| s43 | 0.513 | 0.769 |
| s44 | 0.511 | 0.770 |

Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > 0.521 + CI.
Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship.
Done Tue 23 Jun 2026 04:01:16 AEST
