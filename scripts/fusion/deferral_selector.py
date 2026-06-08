#!/usr/bin/env python3
"""B3 deferral selector — learning-to-defer over frozen v19 + value-expert.

The additive-fusion thesis failed twice (v26, v27): summing the value view's
generic-attractor prior into multi-branch collapses the corpus label space at
breadth. This script tests the reframe — multi-branch (v19) is a GUARANTEED FLOOR;
a tiny selector decides, per column, whether to OVERRIDE v19 with the value-expert
label. Breadth is preserved by construction (we only ever swap v19's label for the
expert's on columns the selector is confident about), so collapse is impossible:
worst case the selector never fires and we are exactly v19.

Inputs (cached, frozen — no model retrain):
  <stem>.f32         row-major float32, layout = mean(240)|max(240)|vote(240)|scalars(8)|mb_logits(240)
  <stem>.labels.tsv  row_idx \t label_index \t label_name   (gold)
  <stem>.meta.json   carries label_order (240) + layout dims

Per row:
  base_pred   = argmax(mb_logits)        -> v19's label  (the floor)
  expert_pred = argmax(mean-prob block)  -> value-expert's label
Decision rows = rows where base_pred != expert_pred (the only rows an override changes).
Target on decision rows: 1 if expert_pred == gold (override helps), else 0 (keep floor).
Rows where neither is right are labelled 0 — overriding can't raise accuracy, so prefer
the floor (asymmetric: only fire when we're confident the expert is actually right).

Offline report (held-out split, no corpus pass needed):
  - base_acc  : the v19 floor on held-out
  - fused_acc : accuracy after applying the selector at the chosen threshold
  - delta     : fused - base  (MUST be >= 0 — improve-or-hold is the whole point)
  - override coverage + precision across a threshold sweep
  - per-class recall delta — does it recover the collapsed/starved types without
    bleeding breadth elsewhere?
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from sklearn.ensemble import HistGradientBoostingClassifier

REPO = Path(__file__).resolve().parents[2]
ROW_DIM = 968
MEAN_OFF, MEAN_END = 0, 240          # value-expert mean-prob block
MB_OFF, MB_END = 728, 968            # multi-branch logits


def load(stem: Path):
    meta = json.loads((stem.with_suffix(".meta.json")).read_text())
    labels = meta["label_order"]
    assert len(labels) == 240, len(labels)
    feats = np.fromfile(stem.with_suffix(".f32"), dtype="<f4")
    n = feats.size // ROW_DIM
    feats = feats.reshape(n, ROW_DIM)
    gold = np.full(n, -1, dtype=np.int64)
    with (stem.with_suffix(".labels.tsv")).open() as f:
        next(f, None)
        for line in f:
            ri, li, _ = line.rstrip("\n").split("\t", 2)
            gold[int(ri)] = int(li)
    return feats, gold, labels


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--stem", type=Path,
                    default=REPO / "output/late-fusion/fusion_feats_v26")
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--val-frac", type=float, default=0.2)
    ap.add_argument("--precision-floor", type=float, default=0.70,
                    help="min override precision on val to accept a threshold")
    ap.add_argument("--out", type=Path,
                    default=REPO / "output/late-fusion/deferral_v1_report.md")
    ap.add_argument("--eval-stem", type=Path, default=None,
                    help="second feature stem (e.g. gold anchor) to apply the trained "
                         "selector to and report per-label base-vs-fused recovery")
    args = ap.parse_args()

    feats, gold, labels = load(args.stem)
    n = feats.shape[0]
    base_pred = np.argmax(feats[:, MB_OFF:MB_END], axis=1)
    expert_pred = np.argmax(feats[:, MEAN_OFF:MEAN_END], axis=1)

    rng = np.random.default_rng(args.seed)
    perm = rng.permutation(n)
    n_val = int(n * args.val_frac)
    val_idx, tr_idx = perm[:n_val], perm[n_val:]

    disagree = base_pred != expert_pred
    # decision-row training set (train split only)
    tr_dec = tr_idx[disagree[tr_idx]]
    y_tr = (expert_pred[tr_dec] == gold[tr_dec]).astype(np.int64)
    X_tr = feats[tr_dec]

    print(f"rows={n}  disagree={disagree.sum()} ({disagree.mean():.1%})  "
          f"train-decision={len(tr_dec)}  pos-rate={y_tr.mean():.3f}", file=sys.stderr)

    clf = HistGradientBoostingClassifier(
        max_iter=300, learning_rate=0.08, max_leaf_nodes=31,
        l2_regularization=1.0, random_state=args.seed,
        class_weight="balanced", early_stopping=True, validation_fraction=0.1)
    clf.fit(X_tr, y_tr)

    # ---- held-out evaluation ----
    base_acc = (base_pred[val_idx] == gold[val_idx]).mean()
    val_dec = val_idx[disagree[val_idx]]
    p_override = clf.predict_proba(feats[val_dec])[:, 1]
    expert_right = (expert_pred[val_dec] == gold[val_dec])

    # threshold sweep
    sweep = []
    for tau in np.round(np.arange(0.30, 0.96, 0.05), 2):
        fire = p_override >= tau
        n_fire = int(fire.sum())
        prec = float(expert_right[fire].mean()) if n_fire else float("nan")
        # net correct = correct overrides - harmful overrides
        net = int(expert_right[fire].sum()) - int((~expert_right[fire]).sum())
        fused_acc = base_acc + net / len(val_idx)
        sweep.append((float(tau), n_fire, n_fire / len(val_idx), prec, net,
                      float(fused_acc), float(fused_acc - base_acc)))

    # choose tau: maximise net subject to precision >= floor
    eligible = [s for s in sweep if not np.isnan(s[3]) and s[3] >= args.precision_floor]
    chosen = max(eligible, key=lambda s: s[4]) if eligible else max(sweep, key=lambda s: s[4])
    tau = chosen[0]

    # per-class recall delta at chosen tau
    fire_mask = np.zeros(n, dtype=bool)
    p_all = np.zeros(n)
    p_all[val_dec] = p_override
    fire_mask[val_dec] = p_override >= tau
    fused_pred = base_pred.copy()
    fused_pred[fire_mask] = expert_pred[fire_mask]

    def per_class_recall(pred):
        rec = {}
        for c in range(240):
            m = gold[val_idx] == c
            if m.sum() >= 10:
                rec[c] = (pred[val_idx][m] == c).mean()
        return rec
    base_rec = per_class_recall(base_pred)
    fused_rec = per_class_recall(fused_pred)
    deltas = sorted(((labels[c], fused_rec[c] - base_rec[c], base_rec[c], fused_rec[c],
                      int((gold[val_idx] == c).sum())) for c in base_rec),
                    key=lambda t: t[1])

    fused_acc = (fused_pred[val_idx] == gold[val_idx]).mean()

    # ---- write report ----
    lines = []
    lines.append("# B3 deferral selector v1 — offline efficacy probe\n")
    lines.append(f"**Features:** `{args.stem.name}`  ({n} rows, {disagree.mean():.1%} base/expert disagree)\n")
    lines.append(f"**Architecture:** v19 floor + HistGBM override selector on 968-dim cached features. "
                 f"Override fires iff base≠expert AND P(override-helps) ≥ τ.\n")
    lines.append("## Headline\n")
    lines.append(f"- **v19 floor (held-out):** {base_acc:.4f}")
    lines.append(f"- **fused (τ={tau}):** {fused_acc:.4f}  →  **Δ {fused_acc - base_acc:+.4f}**")
    lines.append(f"- override coverage **{chosen[2]:.2%}** of all columns, precision **{chosen[3]:.1%}** "
                 f"({chosen[4]:+d} net correct)\n")
    lines.append("## Threshold sweep (held-out)\n")
    lines.append("| τ | fire | coverage | override-precision | net | fused_acc | Δ vs floor |")
    lines.append("|---|------|----------|--------------------|-----|-----------|------------|")
    for t, nf, cov, prec, net, fa, d in sweep:
        mark = " ◄" if t == tau else ""
        lines.append(f"| {t:.2f} | {nf} | {cov:.2%} | {prec:.1%} | {net:+d} | {fa:.4f} | {d:+.4f}{mark} |")
    lines.append("\n## Per-class recall Δ at chosen τ (val classes ≥10 cols)\n")
    lines.append("Top gains (override recovers starved/collapsed types):\n")
    lines.append("| label | base | fused | Δ | n |")
    lines.append("|-------|------|-------|---|---|")
    for name, d, br, fr, ncol in [x for x in reversed(deltas) if x[1] > 1e-9][:15]:
        lines.append(f"| {name} | {br:.3f} | {fr:.3f} | {d:+.3f} | {ncol} |")
    losses = [x for x in deltas if x[1] < -1e-9]
    lines.append(f"\nLosses (override hurt a class): **{len(losses)}**\n")
    if losses:
        lines.append("| label | base | fused | Δ | n |")
        lines.append("|-------|------|-------|---|---|")
        for name, d, br, fr, ncol in losses[:15]:
            lines.append(f"| {name} | {br:.3f} | {fr:.3f} | {d:+.3f} | {ncol} |")
    lines.append("\n## Read\n")
    verdict = ("PROMISING — improve-or-hold holds offline" if fused_acc - base_acc > 0.0005
               else "FLAT — selector adds little net at the precision floor")
    lines.append(f"**{verdict}.** The floor guarantees Δ≥0 in expectation at high precision; "
                 f"the real test is whether per-class gains land on the Sharpen-unreachable "
                 f"boundaries and survive the corpus-honest gate (rare-label relocation).\n")
    # ---- optional: apply trained selector to a second feature set (gold anchor) ----
    if args.eval_stem is not None:
        ef, eg, elabels = load(args.eval_stem)
        assert elabels == labels, "label order mismatch between train and eval stems"
        eb = np.argmax(ef[:, MB_OFF:MB_END], axis=1)
        ee = np.argmax(ef[:, MEAN_OFF:MEAN_END], axis=1)
        edis = eb != ee
        ep = np.zeros(ef.shape[0])
        ep[edis] = clf.predict_proba(ef[edis])[:, 1]
        efire = edis & (ep >= tau)
        efused = eb.copy()
        efused[efire] = ee[efire]
        eb_acc = (eb == eg).mean()
        ef_acc = (efused == eg).mean()
        lines.append(f"\n## Eval stem: `{args.eval_stem.name}` ({ef.shape[0]} cols)\n")
        lines.append(f"- v19 floor **{eb_acc:.3f}** → fused (τ={tau}) **{ef_acc:.3f}**  "
                     f"(**{ef_acc - eb_acc:+.3f}**); selector fired on **{int(efire.sum())}** cols\n")
        lines.append("Per-label base-vs-fused recall (the boundaries B3 targets):\n")
        lines.append("| label | base | fused | Δ | n | fired | expert-right@fire |")
        lines.append("|-------|------|-------|---|---|-------|-------------------|")
        for c in range(240):
            m = eg == c
            ncol = int(m.sum())
            if ncol < 3:
                continue
            br = (eb[m] == c).mean()
            fr = (efused[m] == c).mean()
            nf = int(efire[m].sum())
            er = int(((ee == eg) & efire & m).sum())
            if nf or abs(fr - br) > 1e-9:
                lines.append(f"| {labels[c]} | {br:.3f} | {fr:.3f} | {fr - br:+.3f} | "
                             f"{ncol} | {nf} | {er}/{nf} |")

    args.out.write_text("\n".join(lines))
    print("\n".join(lines))
    print(f"\nwrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
