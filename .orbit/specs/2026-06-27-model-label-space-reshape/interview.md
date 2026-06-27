# Discovery — model architecture & label-space reshape (2026-06-27)

Owner question: *"Given how much ground we've made with the Sharpen layer, are
there any architecture or labelled decisions to improve the speed (training &
inference), simplicity and accuracy of the model?"*

Method: 4-lane substrate survey (label-space, head-architecture, speed/simplicity,
dead-end catalog) over the 108-choice decision register + memories + code, then an
adversarial synthesis cross-checking every candidate against the proven-dead list.
Workflow run `wf_e68a56d0-90b`.

## Thesis

The Sharpen layer has outgrown the model's job. Composed accuracy is rule-bound —
three retrains lifted raw Sense +4–7pp and moved composed zero. The model's fine
value-determinable label space is therefore not where it adds value; it is the
SOURCE of the over-emission that NO-GOs every fresh retrain. Reshape the model to
its real reduced job: open-vocab semantics + broad shape (model-only), cede the
closed/format/checksum tail to validators (correct-by-construction). One reframe =
simplicity + accuracy + the unblock for a retrain that can finally ship.

## Ranked OPEN levers (the spec's ACs)

| # | lever | moves | status | effort |
|---|-------|-------|--------|--------|
| 1 | **Drop validator-ownable leaves from the training label space** (isbn, mac, email, ip, utc-offset, iso_ms, ethereum, …); Sharpen+validators own them | accuracy, simplicity | open bet | med |
| 2 | **Fix-or-drop the synthetic over-emitters** (user_agent 7.2×, currency_code 3.2×, si_number 2.5×); value-determinable fold into #1, header-driven get better negatives (= t-000133e418) | accuracy, simplicity | needs measurement | med |
| 3 | **Abstaining loose-vs-tight head** — the only lever on the 56% over-tighten error; DE-RISK with a separability probe first | accuracy | needs measurement | high |
| 4 | **Delete 0107-blessed orphan modules** (CharCNN/Tiered/TextClassifier/legacy Trainer) | simplicity, train-speed | safe win | med |
| 5 | **dual→single potion encoder** A/B (−20–40ms); **drop the validation branch** (Sharpen veto already does it) — both inside the reshape retrain | infer-speed, simplicity | needs measurement | med |
| 6 | **Record the division-of-labour MADR** + amend H05 for model swaps | accuracy, simplicity | open bet | med |
| 7 | numeric value-range into the abstain GATE only (never the softmax/27-stat trunk) | accuracy | open bet (post #3) | med |
| 8 | wire `deterministic_fast_path` into the profile/batch column path (today only `infer` no-header, main.rs:1328) | infer-speed | needs measurement | med |

Note: the batch-path taxonomy/model hoist the survey ranked #1 is **already shipped
this session** (commit 841cd2b, profile.rs) — banked, not re-chased.

## DO NOT RE-PROPOSE (proven dead — the synthesis guardrail)

- **Bigger static encoder** (potion-8M/code-16M/two-view): 0.522/0.524/0.513 vs 4M 0.521 — static Sense ceiling ~0.52, frontier closed.
- **Transformer/gte backbone in-model**: best Sense 0.571 but composed TIED 0.787 at ~100× latency; 2-encoder oracle caps 0.599 (0.15 below target).
- **Value-fusion / late-fusion Sense replacement**: NO-GO ×2 at corpus breadth (unknown +52/68%, categorical 10.7×/6.05×); code removed in 0107.
- **Per-class / per-leaf m2v witness / feature-specialiser suite**: carves precision only on CLOSED vocab (3 of 4 already Sharpen-owned); open vocab collapses R@P.90 0.000 at AUC 0.987.
- **Naive 44-stat trunk**: cdist Sense 0.316 (the ~0.19 fresh-retrain penalty); 27 stats recover to 0.521. Numeric range may enter only via the abstain gate.
- **Encoder/pooling swap expecting a COMPOSED gain** (gte-tiny, per-value attn): composed TIED; composed is rule-bound at the Sense→Sharpen seam, not the representation.
- **Hierarchical Domain→Family→Type head**: gold 0.685 vs flat 0.718; splitting the output doesn't fix shared-trunk interference, it adds surface.
- **Coarsen the 244-class label space FOR TRAIN-SPEED**: softmax width is negligible compute on a frozen-embedding MLP (the retired-240 trap). Coarsening for accuracy/division-of-labour is the DIFFERENT open bet above.
- **Merge word+plain_text in TRAINING**: residual is a softmax attractor; the cure was shrinking it 44%→25%, not merging.
- **Blind decisive-stat neural-SKIP**: low-card→categorical 0.33, exact-{0,1}→binary 0.00 gold precision — AUC 0.97-0.99 is a ceiling for a SECOND-opinion discriminator, not a blind-assertion threshold.
- **Calibrated all-label feature specialiser (YDF-style)**: YDF gold 0.505 (geography 0.31, technology 0.00), 42.1% wrong on contested ground.
- **Revive CharCNN/Tiered/Transformer value paths**: removed in 0107; value-expert fusion NO-GO ×2 with label collapse.
- **Additive hard-negative retrains into a flat softmax**: 0-for-6 first-try; residual/precedence labels become universal attractors (v27 8× mass-cut made categorical WORSE). Must be value-based Sharpen rules.

## Recommended sequence

1. **ac-6(a) dead-code deletion** — free, do anytime (gold no-regression).
2. **ac-0 → ac-3 leaf-drop** — the one high-leverage move; single-seed first, fold the encoder/branch ablations (ac-6b) into the same retrain.
3. **ac-4 abstaining-head probe** — only after the leaf-drop clears the gate (the retrain it needs is the same one); kill on no-separability.
4. **ac-7 MADR** — author decision the ship depends on; draft once ac-2/ac-3 show the reshape works.
