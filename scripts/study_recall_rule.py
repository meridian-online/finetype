#!/usr/bin/env python3
"""ac-01 / ac-02: categorical + alphanumeric_id recall-rule study
(spec 2026-06-17-categorical-alnum-recall-rule).

A POST-PASS recall rule overrides the model's label to TARGET when the model
predicted one of a set of TRIGGER labels AND the full-column stats match a shape
condition. This measures, on gold, each candidate rule's:

  recovered      gold==TARGET & pred in TRIGGERS         (recall gain — was FN, now TP)
  broke_correct  gold==pred (model was right) & overridden (regression — destroys a TP)
  new_fp         gold not in {TARGET} & overridden        (new FP for TARGET)
  override_prec  recovered / fired                        (precision of the override itself)

Ship candidate = max recovered whose override does NOT drop TARGET precision below
the ac-00 baseline CI and breaks ~no correct predictions. No model is run here —
predictions come from the ac-00 baseline; stats are full-column DuckDB aggregates.

Run: eval/gittables/.venv/bin/python scripts/study_recall_rule.py
"""
import csv
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from sweep_decisive_stats import colstats, resolve, con  # noqa: E402

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLD = os.path.join(REPO, "eval/gold/gold_corpus.tsv")
PRED = os.path.join(REPO, "output/categorical-alnum-recall/predictions_baseline.tsv")
OUTDIR = os.path.join(REPO, "output/categorical-alnum-recall")

CAT = "representation.discrete.categorical"
ALN = "representation.identifier.alphanumeric_id"


def short(l):
    return l.split(".")[-1] if l else "(none)"


def load():
    gold = {}
    meta = {}
    for r in csv.DictReader(open(GOLD), delimiter="\t"):
        k = (r["file_content_sha256"], r["column_name"])
        gold[k] = r["curated_label"]
        meta[k] = (r["file_path"], r["column_name"])
    pred = {}
    for r in csv.DictReader(open(PRED), delimiter="\t"):
        pred[(r["file_content_sha256"], r["column_name"])] = r["predicted_label"]
    # compute full-column stats for every gold column that has a prediction
    stats = {}
    for k, (fp, col) in meta.items():
        if k not in pred:
            continue
        p = resolve(fp)
        if not p:
            continue
        s = colstats(p, col)
        if s is None:
            try:
                schema = [c[0] for c in con.execute(
                    f"DESCRIBE SELECT * FROM read_csv_auto('{p}', SAMPLE_SIZE=-1, ignore_errors=true)"
                    if not p.endswith(".parquet") else f"DESCRIBE SELECT * FROM read_parquet('{p}')"
                ).fetchall()]
                m = {str(c).strip().lower(): c for c in schema}
                cn = m.get(col.strip().lower())
                s = colstats(p, cn) if cn else None
            except Exception:
                s = None
        stats[k] = s
    return gold, pred, stats


def baseline_pr(gold, pred, target):
    tp = sum(1 for k, g in gold.items() if g == target and pred.get(k) == target)
    fp = sum(1 for k, p in pred.items() if p == target and gold.get(k) != target)
    fn = sum(1 for k, g in gold.items() if g == target and pred.get(k) != target)
    return tp, fp, fn


def evaluate(gold, pred, stats, target, triggers, statcond, label):
    """Apply override rule, return metrics dict."""
    fired = recovered = broke_correct = new_fp = 0
    for k, p in pred.items():
        if p not in triggers:
            continue
        s = stats.get(k)
        if s is None or not statcond(s):
            continue
        fired += 1
        g = gold.get(k)
        if g == target:
            recovered += 1          # was FN (p != target since p in triggers != target)
        else:
            new_fp += 1
            if g == p:
                broke_correct += 1   # model had it right, we overrode
    tp0, fp0, fn0 = baseline_pr(gold, pred, target)
    tp1, fp1 = tp0 + recovered, fp0 + new_fp
    r0 = tp0 / (tp0 + fn0) if (tp0 + fn0) else 0
    r1 = tp1 / (tp1 + (fn0 - recovered)) if (tp1 + fn0 - recovered) else 0
    p0 = tp0 / (tp0 + fp0) if (tp0 + fp0) else 0
    p1 = tp1 / (tp1 + fp1) if (tp1 + fp1) else 0
    ovp = recovered / fired if fired else float("nan")
    return dict(label=label, fired=fired, recovered=recovered, new_fp=new_fp,
                broke_correct=broke_correct, override_prec=ovp,
                R0=r0, R1=r1, P0=p0, P1=p1)


def main():
    gold, pred, stats = load()
    out = open(os.path.join(OUTDIR, "recall_study.md"), "w")

    def emit(*a):
        line = " ".join(str(x) for x in a)
        print(line)
        out.write(line + "\n")

    n_stats = sum(1 for v in stats.values() if v)
    emit(f"# ac-01/02 recall-rule study (gold, baseline preds)\n")
    emit(f"stats computed for {n_stats}/{len(pred)} predicted gold columns\n")
    for tgt in (CAT, ALN):
        tp, fp, fn = baseline_pr(gold, pred, tgt)
        emit(f"baseline {short(tgt):16s} TP={tp} FP={fp} FN={fn} "
             f"P={tp/(tp+fp):.3f} R={tp/(tp+fn):.3f}")
    emit("")

    def row(m):
        emit(f"  {m['label']:42s} fired={m['fired']:3d} recov={m['recovered']:3d} "
             f"newFP={m['new_fp']:2d} broke={m['broke_correct']:2d} "
             f"ovP={m['override_prec']:.2f}  R {m['R0']:.3f}->{m['R1']:.3f}  "
             f"P {m['P0']:.3f}->{m['P1']:.3f}")

    # ── ac-02 alphanumeric_id ────────────────────────────────────────────────
    emit("## ac-02 alphanumeric_id — override TRIGGERS where high-card + alpha + digit")
    emit("   (alnum = mixed letters+digits, near-unique; unknown = model gave up)")

    def alnum_cond(cmin, a, d):
        return lambda s: s["card"] >= cmin and s["alpha_frac"] >= a and s["digit_frac"] >= d
    for trig in (["unknown"],
                 ["unknown", "technology.internet.url"],
                 ["unknown", "technology.internet.url", "representation.text.plain_text"]):
        tset = set()
        for t in trig:
            tset.add(t if "." in t else t)  # 'unknown' stays bare
        # map bare 'unknown' to how predictions encode it
        triggers = {("unknown" if t == "unknown" else t) for t in trig}
        for cmin, a, d in [(0.90, 0.3, 0.3), (0.95, 0.5, 0.5), (0.99, 0.5, 0.3)]:
            m = evaluate(gold, pred, stats, ALN, triggers,
                         alnum_cond(cmin, a, d),
                         f"{'+'.join(short(x) for x in trig)} card>={cmin} a>={a} d>={d}")
            row(m)
        emit("")

    # ── ac-01 categorical ────────────────────────────────────────────────────
    emit("## ac-01 categorical — override TRIGGERS where low-card small vocab")
    emit("   (entity_name/plain_text already corpus-honest-REFUTED; word already targeted)")

    def cat_cond(cmax, dmax):
        return lambda s: s["card"] <= cmax and s["nd"] <= dmax and s["n"] >= 4
    for trig in (["representation.text.word"],
                 ["representation.text.word", "representation.discrete.ordinal"],
                 ["representation.text.word", "representation.discrete.ordinal",
                  "representation.text.entity_name", "representation.text.plain_text"]):
        triggers = set(trig)
        for cmax, dmax in [(0.10, 20), (0.30, 30), (0.60, 50)]:
            m = evaluate(gold, pred, stats, CAT, triggers, cat_cond(cmax, dmax),
                         f"{'+'.join(short(x) for x in trig)} card<={cmax} nd<={dmax}")
            row(m)
        emit("")

    print(f"[written to {os.path.join(OUTDIR, 'recall_study.md')}]")
    out.close()


if __name__ == "__main__":
    main()
