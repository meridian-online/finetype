# ac-02 [m2v8m] — bigger-static embed vs m2v-244 (Mon 22 Jun 2026 16:29:10 AEST)

potion: minishlab/potion-base-8M  | embed_dim 1024 | Sense-vs-Sense (offline, no truncation)
baseline m2v-244-s44: Sense 0.521 / composed 0.769  (v19: Sense 0.502 / composed 0.793)

| seed | Sense | composed |
|---|---|---|
| s42 | 0.487 | 0.764 |
| s43 | 0.522 | 0.792 |
| s44 | 0.493 | 0.794 |

Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > 0.521 + CI.
Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship.
Done Mon 22 Jun 2026 16:36:40 AEST
