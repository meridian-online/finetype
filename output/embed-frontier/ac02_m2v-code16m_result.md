# ac-02 [m2v-code16m] — bigger-static embed vs m2v-244 (Tue 23 Jun 2026 15:46:45 AEST)

potion: minishlab/potion-code-16M  | embed_dim 1024 | Sense-vs-Sense (offline, no truncation)
baseline m2v-244-s44: Sense 0.521 / composed 0.769  (v19: Sense 0.502 / composed 0.793)

| seed | Sense | composed |
|---|---|---|
| s42 | 0.512 | 0.767 |
| s43 | 0.524 | 0.763 |
| s44 | 0.503 | 0.776 |

Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > 0.521 + CI.
Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship.
Done Tue 23 Jun 2026 15:53:58 AEST
