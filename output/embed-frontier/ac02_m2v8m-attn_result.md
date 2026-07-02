# ac-02 [m2v8m-attn] — bigger-static embed vs m2v-244 (Thu 25 Jun 2026 08:04:03 AEST)

potion: minishlab/potion-base-8M  | embed_dim 1024 | Sense-vs-Sense (offline, no truncation)
baseline m2v-244-s44: Sense 0.521 / composed 0.769  (v19: Sense 0.502 / composed 0.793)

| seed | Sense | composed |
|---|---|---|
| s42 | 0.561 | 0.794 |

Decision (pre-registered, ac02_readiness.md): GO if best-of-3 Sense > 0.521 + CI.
Latency: potion encode ~free (~0.5 ms/col flat 4M->32M, prep); native confirm at ship.
Done Thu 25 Jun 2026 08:06:52 AEST
