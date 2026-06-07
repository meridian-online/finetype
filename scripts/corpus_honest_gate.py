#!/usr/bin/env python3
"""ac-02/ac-03 (spec 2026-06-07-corpus-honest-quality-gate) — the honest gate.

Reads a candidate's corpus predictions against the v19 baseline on the stratified
sample (ac-01) and emits a corpus-scale GO/NO-GO that the curated instruments miss.

Why a transition matrix, not a count delta. The sample is stratified on v19's CALLS,
so it oversamples where v19 predicted each label. A marginal count on it is biased:
latdec's latitude count on the sample DROPS (the sample is latitude-rich and latdec
corrected v19's latitude FPs) while the corpus latitude count RISES (+1,840), because
latdec's new FPs land in the decimal pool the sample only covers at 11.5%. A naive
delta would read the wrong SIGN and clear latdec — exactly the curated-instrument
failure this gate exists to stop.

The honest read scales each per-column transition (v19 label A -> candidate label B)
by 1 / sample_rate[A]. That un-biases the sample: latdec's 457 observed decimal->
latitude moves / 0.1147 (decimal's sample rate) = ~3,985 estimated corpus moves,
recovering the true figure.

The oracle is the BASELINE's GATED ydf (ydf_prediction_gated from v19_gated.parquet),
not the candidate's. YDF is a property of the column's data, independent of the Sense
model, so it is read once from the v19 pass and is always present. (The candidate
pass may not have run --fill-ydf at all — the latdec parquet's ydf is 100% NULL — so
any metric keyed on the CANDIDATE's ydf reads a meaningless zero. That artifact is
exactly what made the latdec bet's FP metric appear to hit zero.) The GATE matters:
raw YDF is demonstrably noisy (msg_id->iso6346, team-codes->country_code), so raw
contradiction floods common labels like word with tens of thousands of bogus refutes;
the gated oracle NULLs any YDF label fewer than 50% of the column's values pass, the
canonical scoring lens. Against the stable gated oracle, latdec's new latitude calls
are not "hidden on ydf=NULL" — they sit on columns the oracle positively labels
decimal (421 of 457 sample moves; 36 silent). Three bands read the scaled flows:

  over_emit  — est. candidate marginal / v19 marginal >= rel-mult            (v23: +529%)
  collapse   — est. candidate marginal / v19 marginal <= collapse-frac       (v22: country -31.5%)
  oracle_fp  — inflow X->B (X != B) onto columns the baseline oracle labels
               as something OTHER than B: predictions the oracle refutes      (latdec relocation)

oracle_fp is the load-bearing band: it counts the false positives a fix CREATES on
columns the oracle actively contradicts, measured against the stable baseline oracle
that the bet's own (empty) candidate-ydf metric could not see.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_BASELINE = REPO / "output/ydf-validation-gate/v19_gated.parquet"
DEFAULT_SAMPLE = REPO / "output/corpus-honest-gate/stratified_sample.files.txt"
DEFAULT_RES = REPO / "output/corpus-honest-gate/stratified_sample.resolution.json"


def duck_csv(sql: str) -> list[list[str]]:
    r = subprocess.run(["duckdb", "-noheader", "-csv", "-c", sql],
                       capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stderr)
        raise SystemExit(f"duckdb failed (exit {r.returncode})")
    out = []
    for line in r.stdout.splitlines():
        line = line.rstrip("\n")
        if line:
            out.append(next(iter([_split_csv(line)])))
    return out


def _split_csv(line: str) -> list[str]:
    # transition rows are simple (label,label,bool,int) — no embedded commas in
    # label names, but quote-strip defensively.
    return [c.strip().strip('"') for c in line.split(",")]


def transition_counts(baseline: Path, candidate: Path, sample: Path) -> list[tuple]:
    """Rows of (s19, scand, oracle_rel, cnt) on the sample.

    oracle_rel classifies the baseline oracle (v19 ydf) against the CANDIDATE label:
      agree     — oracle == candidate label
      contradict— oracle is non-null and != candidate label (oracle refutes)
      silent    — oracle is null (no opinion)
    """
    sql = f"""
WITH samp AS (SELECT column0 AS file_path
              FROM read_csv('{sample.as_posix()}', header=false)),
b AS (SELECT file_path, column_name, sense_prediction s, ydf_prediction_gated y
      FROM read_parquet('{baseline.as_posix()}')
      WHERE file_path IN (SELECT file_path FROM samp)),
c AS (SELECT file_path, column_name, sense_prediction s
      FROM read_parquet('{candidate.as_posix()}')
      WHERE file_path IN (SELECT file_path FROM samp)),
j AS (SELECT b.s AS s19, c.s AS scand,
             CASE WHEN b.y IS NULL THEN 'silent'
                  WHEN b.y = c.s THEN 'agree'
                  ELSE 'contradict' END AS oracle_rel
      FROM b JOIN c USING(file_path, column_name))
SELECT s19, scand, oracle_rel, COUNT(*) cnt FROM j GROUP BY 1,2,3;
"""
    rows = duck_csv(sql)
    return [(r[0], r[1], r[2], int(r[3])) for r in rows]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    ap.add_argument("--candidate", type=Path, required=True)
    ap.add_argument("--sample", type=Path, default=DEFAULT_SAMPLE)
    ap.add_argument("--resolution", type=Path, default=DEFAULT_RES)
    ap.add_argument("--label", default="candidate", help="name for the report")
    ap.add_argument("--rel-mult", type=float, default=3.0,
                    help="over-emit: est marginal / v19 marginal >= this")
    ap.add_argument("--collapse-frac", type=float, default=0.6,
                    help="collapse: est marginal / v19 marginal <= this")
    ap.add_argument("--oracle-fp-ratio", type=float, default=0.20,
                    help="oracle_fp: net contradicted inflow / v19 marginal >= this")
    ap.add_argument("--oracle-fp-floor", type=int, default=1000,
                    help="oracle_fp: AND net contradicted inflow >= this many columns")
    ap.add_argument("--oracle-fp-obs-floor", type=int, default=120,
                    help="oracle_fp: AND RAW (unscaled) contradicted inflow >= this "
                         "many observed sample columns — suppresses rare-source "
                         "scaling amplification (a handful of cols * 1/rate noise)")
    ap.add_argument("--marginal-floor", type=int, default=2000,
                    help="min v19 corpus marginal for over-emit/collapse to fire "
                         "(suppresses tiny-label ratio noise)")
    ap.add_argument("--out-dir", type=Path,
                    default=REPO / "output/corpus-honest-gate")
    args = ap.parse_args()

    res = json.load(open(args.resolution))
    corpus_full = {x["label"]: x["full_cols"] for x in res["per_label"]}

    trans = transition_counts(args.baseline, args.candidate, args.sample)

    # sample rate per v19 source label A = observed sample cols(A) / corpus cols(A)
    sample_src = {}
    for row in trans:
        a, cnt = row[0], row[3]
        sample_src[a] = sample_src.get(a, 0) + cnt
    rate = {a: sample_src[a] / corpus_full[a] for a in sample_src if corpus_full.get(a)}

    # scaled corpus flows
    labels = set(corpus_full) | {row[1] for row in trans}
    est_marginal = {b: 0.0 for b in labels}        # est candidate marginal
    contra_in = {b: 0.0 for b in labels}           # inflow X->B oracle refutes (X!=B)
    contra_out = {b: 0.0 for b in labels}          # outflow B->X oracle refuted (X!=B)
    silent_in = {b: 0.0 for b in labels}           # inflow X->B oracle silent (X!=B)
    obs_contra_in = {b: 0 for b in labels}         # RAW (unscaled) inflow count, X->B refuted
    for a, b, rel, cnt in trans:
        r = rate.get(a)
        if not r:
            continue
        scaled = cnt / r
        est_marginal[b] += scaled
        if a != b:
            if rel == "contradict":
                contra_in[b] += scaled
                contra_out[a] += scaled
                obs_contra_in[b] += cnt
            elif rel == "silent":
                silent_in[b] += scaled

    rows = []
    verdict = "GO"
    triggers = []
    for b in labels:
        v19m = corpus_full.get(b, 0)
        em = est_marginal[b]
        ratio = (em / v19m) if v19m else float("inf") if em else 0.0
        # net contradicted inflow: predictions of B the oracle refutes that the
        # candidate ADDS beyond the contradictions it clears elsewhere on B.
        net_contra = contra_in[b] - contra_out[b]
        bands = []
        if v19m >= args.marginal_floor and ratio >= args.rel_mult:
            bands.append("over_emit")
        if v19m >= args.marginal_floor and ratio <= args.collapse_frac:
            bands.append("collapse")
        if (net_contra >= args.oracle_fp_floor
                and obs_contra_in[b] >= args.oracle_fp_obs_floor
                and v19m and net_contra / v19m >= args.oracle_fp_ratio):
            bands.append("oracle_fp")
        if bands:
            verdict = "NO-GO"
            triggers.append(b)
        rows.append({
            "label": b, "v19_marginal": v19m,
            "est_cand_marginal": round(em), "ratio": round(ratio, 3),
            "contra_in": round(contra_in[b]), "contra_out": round(contra_out[b]),
            "net_contra_in": round(net_contra), "obs_contra_in": obs_contra_in[b],
            "silent_in": round(silent_in[b]),
            "bands": bands,
        })

    rows.sort(key=lambda x: (not x["bands"], -x["net_contra_in"],
                             -abs(x["ratio"] - 1)))
    report = {
        "label": args.label, "verdict": verdict,
        "candidate": args.candidate.as_posix(),
        "bands": {"rel_mult": args.rel_mult, "collapse_frac": args.collapse_frac,
                  "oracle_fp_ratio": args.oracle_fp_ratio,
                  "oracle_fp_floor": args.oracle_fp_floor,
                  "oracle_fp_obs_floor": args.oracle_fp_obs_floor,
                  "marginal_floor": args.marginal_floor},
        "triggers": triggers,
        "movers": [r for r in rows if r["bands"]
                   or abs(r["ratio"] - 1) > 0.15 or r["net_contra_in"] > 300][:25],
    }
    args.out_dir.mkdir(parents=True, exist_ok=True)
    out = args.out_dir / f"gate_{args.label}.json"
    out.write_text(json.dumps({"report": report, "all_labels": rows}, indent=2))
    print(json.dumps(report, indent=2))
    print(f"\nwrote {out}", file=sys.stderr)
    return 0 if verdict == "GO" else 1


if __name__ == "__main__":
    sys.exit(main())
