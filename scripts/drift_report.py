#!/usr/bin/env python3
"""Full-label-space destination-drift report (spec 2026-06-05-destination-drift-precheck ac-01).

Diffs two Sense-distribution snapshots (baseline vs candidate, as written by
scripts/snapshot_sense_distribution.py) and flags EVERY label whose prediction
rate moves beyond a band. The band has three conjoint conditions — a label trips
only when ALL hold:

  - direction:       cand_rate > base_rate (with --direction up, the default).
                     Destination drift is a boundary ABSORBING columns it should
                     not, and real drift concentrates INTO one destination while
                     benign seed noise disperses across many — so a label that
                     SHRINKS is the targeted cluster shedding FPs or a benign
                     reshuffle, not an over-emit. --direction both restores
                     symmetric flagging for general distribution diffs.
  - absolute floor:  cand_rate - base_rate >= --abs-floor  (suppresses tiny
                     labels whose raw counts wobble on sampling noise), AND
  - relative multiple: max(ratio, 1/ratio) >= --rel-mult     (suppresses large,
                     stable labels that drift a few columns in absolute terms).

## Calibrated band (spec 2026-06-05-destination-drift-precheck ac-04)

Defaults --abs-floor 0.0040, --rel-mult 3.0, --direction up are calibrated
against two paid-for explosions and one negative control, all on ONE fixed
1000-file / 13,533-column corpus sample (snapshots in
output/destination-drift-precheck/, identical file list — DuckDB reservoir
sampling is NOT seed-reproducible, so a shared list is mandatory):

  - v23 (v22 -> v23): NO-GO. representation.discrete.categorical +6.13pp, 4.10×
    — the categorical explosion that closed v23 Failed. Trips.
  - v24 (v19 -> v24): NO-GO. geography.coordinate.latitude +0.64pp, 5.65× — the
    untargeted boundary the hand-picked watch block missed. Trips.
  - negative control (v19 s42 -> s43, same files, different MODEL seed, so every
    move is pure seed noise): GO. No label trips.

Margins to the band (why these thresholds, not tighter):
  - Relative axis: worst noise mover is technology.internet.user_agent at 2.23×;
    smallest true signal is v23 categorical at 4.10×. 3.00× sits near-centrally
    in that (2.23×, 4.10×) gap — 0.77× of headroom below, 1.10× above.
  - Absolute axis: the floor is load-bearing for small-base relative wobblers
    that clear the rel line — identity.commerce.isbn moves +0.32pp at 4.19× and
    is excluded ONLY by the 0.40pp floor; the smallest true signal it must let
    through is v24 latitude at +0.64pp (0.24pp of headroom).
  - Direction: the control's container.array.comma_separated collapses -2.93pp
    and v23/v24 shrink many labels; all excluded structurally, not by threshold.

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
    scripts/drift_report.py BASELINE.json CANDIDATE.json    # calibrated defaults
    scripts/drift_report.py b.json c.json --abs-floor 0.002 --rel-mult 2.0
    scripts/drift_report.py b.json c.json --direction both --json out.json
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
    ap.add_argument("--abs-floor", type=float, default=0.0040,
                    help="absolute rate-change floor, as a fraction of total_cols "
                         "(default 0.0040 = 0.40 percentage points; calibrated, see "
                         "the band section of the module docstring)")
    ap.add_argument("--rel-mult", type=float, default=3.0,
                    help="relative multiple a label's rate must move, on smoothed "
                         "rates (default 3.0×; calibrated to sit near-centrally in the "
                         "gap between the worst model-seed noise mover and the smallest "
                         "paid-for explosion — see the module docstring)")
    ap.add_argument("--direction", choices=("up", "both"), default="up",
                    help="which movers can trip the band. 'up' (default) flags only "
                         "labels that GAIN rate — destination drift is a boundary "
                         "absorbing columns it should not, and real drift concentrates "
                         "INTO one destination while benign seed noise disperses across "
                         "many (a base-N label collapsing is the targeted cluster "
                         "shedding FPs or a benign reshuffle, not an over-emit). 'both' "
                         "restores symmetric flagging for general distribution diffs.")
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
        dir_ok = (d_rate > 0) if args.direction == "up" else True
        flagged = dir_ok and (abs(d_rate) >= args.abs_floor) and (directional >= args.rel_mult)
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
