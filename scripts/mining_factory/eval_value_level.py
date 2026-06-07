#!/usr/bin/env python3
"""Value-level architecture test — per-value eval of a CharCNN model.

Runs `finetype infer --model-type char-cnn` over the shared test split and reports
per-type recall, with the confusion family as the load-bearing read. The key
questions, answered per-value:
  - latitude/longitude recall: did the model LEARN the starved family (Arm A ~0 from
    10 train examples; Arm B from 10k manufactured)?
  - decimal_number / integer_number recall: did it HOLD (training identical in both
    arms), or collapse the way the multi-branch blend did?
  - confusion structure: when a latitude/decimal is wrong, WHERE does it go?

Inference goes one value per line through `finetype infer -f`, so values containing
newlines (multi-line plain_text/entity blobs — not the confusion family) are dropped
and counted as `skipped_multiline`. Output order is preserved, so predictions align
to inputs by position.
"""
from __future__ import annotations

import argparse
import csv
import io
import json
import subprocess
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
OUT_DIR = REPO / "output" / "value-level-arch-test"
BIN = str(REPO / "target/release/finetype")

CONFUSION_FAMILY = [
    "geography.coordinate.latitude", "geography.coordinate.longitude",
    "representation.numeric.decimal_number", "representation.numeric.integer_number",
    "geography.location.city", "geography.location.region",
    "geography.address.postal_code",
]


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--model", required=True, help="model dir (FINETYPE_MODEL)")
    ap.add_argument("--label", required=True, help="arm label for the report")
    ap.add_argument("--test", type=Path, default=OUT_DIR / "value_test.ndjson")
    ap.add_argument("--model-type", default="char-cnn")
    args = ap.parse_args()

    texts: list[str] = []
    golds: list[str] = []
    skipped = 0
    with args.test.open(encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            r = json.loads(line)
            v = r["text"]
            if "\n" in v or "\r" in v or v == "":
                skipped += 1
                continue
            texts.append(v)
            golds.append(r["classification"])

    tmp = OUT_DIR / f"_infer_in_{args.label}.txt"
    tmp.write_text("\n".join(texts) + "\n", encoding="utf-8")

    env = {"FINETYPE_MODEL": args.model, "PATH": "/usr/bin:/bin"}
    proc = subprocess.run(
        [BIN, "infer", "-f", str(tmp), "-o", "csv", "-v", "--confidence",
         "--model-type", args.model_type],
        env=env, capture_output=True, text=True)
    if proc.returncode != 0:
        print(f"infer FAILED: {proc.stderr[:500]}")
        return 1

    preds: list[str] = []
    reader = csv.reader(io.StringIO(proc.stdout))
    for row in reader:
        if len(row) >= 2:
            preds.append(row[1])
    tmp.unlink(missing_ok=True)

    if len(preds) != len(texts):
        print(f"WARN: {len(preds)} preds != {len(texts)} inputs — alignment unsafe")
        n = min(len(preds), len(texts))
        preds, texts, golds = preds[:n], texts[:n], golds[:n]

    # The taxonomy carries locale-specialised variants (geography.address.postal_code.ID),
    # which the model may emit. Score at BASE-TYPE granularity: a prediction counts for
    # gold type g if it equals g or is a locale-specialisation g.<locale>. Collapse every
    # prediction to its longest matching gold base for both scoring and confusion display.
    gold_types = sorted(set(golds), key=len, reverse=True)

    def base_of(pred: str) -> str:
        for g in gold_types:
            if pred == g or pred.startswith(g + "."):
                return g
        return pred

    per_type_total: Counter = Counter()
    per_type_correct: Counter = Counter()
    confusion: dict[str, Counter] = defaultdict(Counter)
    for gold, pred in zip(golds, preds):
        bp = base_of(pred)
        per_type_total[gold] += 1
        if bp == gold:
            per_type_correct[gold] += 1
        confusion[gold][bp] += 1

    overall = sum(per_type_correct.values()) / max(1, sum(per_type_total.values()))

    report = {
        "label": args.label, "model": args.model, "model_type": args.model_type,
        "n_eval": len(texts), "skipped_multiline": skipped,
        "overall_recall": round(overall, 4),
        "confusion_family": {}, "per_type_recall": {},
    }
    for t in sorted(per_type_total):
        report["per_type_recall"][t] = {
            "recall": round(per_type_correct[t] / per_type_total[t], 4),
            "n": per_type_total[t]}
    for t in CONFUSION_FAMILY:
        tot = per_type_total.get(t, 0)
        if not tot:
            continue
        top = confusion[t].most_common(4)
        report["confusion_family"][t] = {
            "recall": round(per_type_correct[t] / tot, 4), "n": tot,
            "top_predicted": [{"label": l, "pct": round(100 * c / tot, 1)} for l, c in top]}

    out = OUT_DIR / f"eval_{args.label}.json"
    out.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    print(f"\n=== {args.label}  (model={args.model}) ===")
    print(f"n_eval={len(texts):,}  skipped_multiline={skipped:,}  overall_recall={overall:.3f}")
    print(f"\n{'confusion family':45s} {'recall':>7s} {'n':>6s}   top predicted")
    for t in CONFUSION_FAMILY:
        cf = report["confusion_family"].get(t)
        if not cf:
            continue
        tops = " ".join(f"{p['label'].split('.')[-1]}:{p['pct']}%" for p in cf["top_predicted"])
        print(f"{t:45s} {cf['recall']:>7.3f} {cf['n']:>6,}   {tops}")
    print(f"\nwrote {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
