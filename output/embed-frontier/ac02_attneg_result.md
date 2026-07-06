# ac-02 [attneg] — bigger-static embed vs m2v-244 (Mon  6 Jul 2026 10:50:14 AEST)

potion: minishlab/potion-base-8M  | embed_dim 1024 | Sense-vs-Sense (offline, no truncation)
baseline m2v-244-s44: Sense 0.521 / composed 0.769  (v19: Sense 0.502 / composed 0.793)

| seed | Sense | composed |
|---|---|---|
| s42 | 0.580 | 0.855 |
| s43 | 0.559 | 0.847 |
| s44 | 0.560 | 0.855 |

Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > 0.521 + CI.
Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship.
Done Mon  6 Jul 2026 10:59:11 AEST
