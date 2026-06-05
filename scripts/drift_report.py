#!/usr/bin/env python3
"""Full-label-space destination-drift report (spec 2026-06-05-destination-drift-precheck ac-01).

Diffs two Sense-distribution snapshots (baseline vs candidate, as written by
scripts/snapshot_sense_distribution.py) and flags EVERY label whose prediction
rate moves beyond a band. The band has two conjoint conditions — a label trips
only when BOTH hold:

  - absolute floor:  |cand_rate - base_rate| >= --abs-floor  (suppresses tiny
                     labels whose raw counts wobble on sampling noise), AND
  - relative multiple: max(ratio, 1/ratio) >= --rel-mult     (suppresses large,
                     stable labels that drift a few columns in absolute terms).

Rates, not raw counts: the two snapshots may profile a different total_cols
(different file sample, different model), so every comparison is a rate
(count / total_cols). The relative multiple is computed on Laplace-smoothed
rates (+0.5 column) so a label that appears from nothing (0 -> many) gets a
finite, large ratio instead of a divide-by-zero.

Output: the top movers ranked by absolute rate change (always shown, so the
instrument is legible even on a GO), then an explicit GO / NO-GO with the
flagged-label list. NO-GO when any label trips the band.

This is the canonical collateral instrument that supersedes the snapshot's
hand-picked `watch` block — it measures the whole label vector, not a
pre-registered subset, so a drift in an unwatched boundary (v24's
geography.coordinate.latitude) cannot hide.

Usage:
    scripts/drift_report.py BASELINE.json CANDIDATE.json
    scripts/drift_report.py b.json c.json --abs-floor 0.002 --rel-mult 2.0
    scripts/drift_report.py b.json c.json --json out.json
"""
import argparse
import json
import sys
from pathlib import Path


def load(path):
    d = json.loads(Path(path).read_text())
    total = d.get("total_cols")
    counts = d.get("label_counts", {})
    if not total:
        # Fall back to the histogram sum if total_cols is absent.
        total = sum(counts.values())
    if not total:
        sys.exit(f"{path}: no total_cols and empty label_counts — cannot rate-normalise")
    return d, int(total), counts


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("baseline", type=Path, help="baseline snapshot JSON (pre-train)")
    ap.add_argument("candidate", type=Path, help="candidate snapshot JSON (post-train)")
    ap.add_argument("--abs-floor", type=float, default=0.0020,
                    help="absolute rate-change floor, as a fraction of total_cols "
                         "(default 0.0020 = 0.20 percentage points)")
    ap.add_argument("--rel-mult", type=float, default=2.0,
                    help="relative multiple a label's rate must move, in either "
                         "direction, on smoothed rates (default 2.0 = a doubling/halving)")
    ap.add_argument("--top", type=int, default=20,
                    help="how many ranked movers to print (default 20)")
    ap.add_argument("--json", type=Path, default=None,
                    help="also write the structured report to this path")
    args = ap.parse_args()

    bmeta, btot, bc = load(args.baseline)
    cmeta, ctot, cc = load(args.candidate)

    # Laplace-smoothing denominator for the ratio: half a column on the larger base.
    eps = 0.5 / max(btot, ctot)

    rows = []
    for label in sorted(set(bc) | set(cc)):
        b = bc.get(label, 0)
        c = cc.get(label, 0)
        br = b / btot
        cr = c / ctot
        d_rate = cr - br
        ratio = (cr + eps) / (br + eps)
        directional = max(ratio, 1.0 / ratio)  # magnitude of move in either direction
        flagged = (abs(d_rate) >= args.abs_floor) and (directional >= args.rel_mult)
        rows.append({
            "label": label,
            "base_count": b, "cand_count": c,
            "base_rate": br, "cand_rate": cr,
            "delta_rate": d_rate, "ratio": ratio,
            "direction": "up" if d_rate > 0 else ("down" if d_rate < 0 else "flat"),
            "flagged": flagged,
        })

    rows.sort(key=lambda r: abs(r["delta_rate"]), reverse=True)
    flagged = [r for r in rows if r["flagged"]]

    # The flagged set is the actionable output, so it is NEVER truncated — every
    # label that trips the band is shown, even one whose absolute move is modest
    # but whose relative multiple is large (v24's coordinate.latitude: +0.42pp but
    # 4.4×, which ranks low by Δpp yet is the signal the watch block missed). The
    # --top limit only governs how much unflagged context we append for legibility.
    unflagged = [r for r in rows if not r["flagged"]]
    fill = max(0, args.top - len(flagged))
    display = sorted(flagged + unflagged[:fill],
                     key=lambda r: abs(r["delta_rate"]), reverse=True)

    blabel = bmeta.get("label", str(args.baseline))
    clabel = cmeta.get("label", str(args.candidate))
    print(f"drift report: {blabel} (n={btot}) -> {clabel} (n={ctot})")
    print(f"band: |Δrate| >= {args.abs_floor*100:.3f}pp AND move >= {args.rel_mult:.2f}×\n")
    hdr = f"{'label':46s} {'base':>12s} {'cand':>12s} {'Δpp':>8s} {'×':>7s}  flag"
    print(hdr)
    print("-" * len(hdr))
    for r in display:
        print(f"{r['label']:46s} "
              f"{r['base_count']:5d}/{r['base_rate']*100:5.2f}% "
              f"{r['cand_count']:5d}/{r['cand_rate']*100:5.2f}% "
              f"{r['delta_rate']*100:+7.3f} "
              f"{r['ratio']:6.2f}×  {'NO-GO' if r['flagged'] else ''}")

    verdict = "NO-GO" if flagged else "GO"
    print()
    if flagged:
        names = ", ".join(r["label"] for r in flagged)
        print(f"VERDICT: NO-GO — {len(flagged)} label(s) drifted beyond the band: {names}")
    else:
        print("VERDICT: GO — no label drifted beyond the band")

    if args.json:
        out = {
            "baseline": blabel, "candidate": clabel,
            "baseline_total_cols": btot, "candidate_total_cols": ctot,
            "abs_floor": args.abs_floor, "rel_mult": args.rel_mult,
            "verdict": verdict,
            "flagged": [r["label"] for r in flagged],
            "movers": rows,
        }
        args.json.write_text(json.dumps(out, indent=2))
        print(f"wrote {args.json}", file=sys.stderr)

    # Exit non-zero on NO-GO so a launcher can gate on it.
    sys.exit(1 if flagged else 0)


if __name__ == "__main__":
    main()
