# ac-01 — value-level YDF prior (synthetic bootstrap)

- synthetic values generated: **47600** (232 types)
- leakage firewall dropped: **4240** (eval-holdout collisions)
- trained on: **34688**, held out: **8672**
- held-out synthetic accuracy: **0.9031**
- per-type support: min **6**, max **200**
- feature contract: **277** dims (37 deterministic + 240 schema)
- model: `eval/gittables/models/ydf_value`

Firewall note: synthetic values are headerless, so the firewall is a conservative value-level exclusion against the eval holdout values (cannot under-exclude vs the header-bearing row_hash). The load-bearing row_hash(header,value) firewall runs on the gittables side (ac-02/ac-08).
