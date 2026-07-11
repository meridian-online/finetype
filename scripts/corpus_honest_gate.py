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

  over_emit  — (est. candidate marginal − oracle-CONFIRMED correct growth)
               / v19 marginal >= rel-mult — composition-aware so stacked
               honest fixes cannot trip it while relocation still does
               (refined/composition_aware_over_emit.md)                      (v23: +529%)
  collapse   — oracle-CONFIRMED candidate support / v19 confirmed support
               <= collapse-frac (a loss of CORRECT mass, not raw marginal)    (v22: country)
  oracle_fp  — CREATED false positives: a move A->B the oracle refutes (oracle
               != B) where the oracle CONFIRMED the source (oracle == A) — a
               column that was correct as A and is now wrong as B             (latdec relocation)

oracle_fp is the load-bearing band: it counts the false positives a fix CREATES on
columns the oracle actively contradicts, measured against the stable baseline oracle
that the bet's own (empty) candidate-ydf metric could not see.

Both oracle-keyed bands read the oracle against BOTH ends of each transition (see
transition_counts). This is what lets the gate distinguish a regression (a correct
prediction relocated onto a wrong label) from honest abstention (a label the oracle
ALREADY refuted, demoted to unknown by the validation veto). The earlier raw-marginal
collapse + candidate-only oracle_fp scored every correct-FP removal as a regression —
a NO-GO false alarm on a precision-hardening patch (the 0.6.24 finding).
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
# Baseline SOURCING (de-fossilised 2026-07-11). The gate scores a candidate's
# relocation against a REFERENCE corpus pass — and that reference MUST be the
# STANDING CURRENT-DEFAULT model, not the retired v19 oracle. Workflow per
# promotion round: run the current models/default over the ac-01 stratified
# sample with --fill-ydf to emit sense_prediction + ydf_prediction_gated, then
# pass that parquet as --baseline. There is deliberately NO default: an implicit
# v19 baseline silently measured deviation-from-v19 (0% structural pass for any
# retrain; choice 0104), the exact fossil this removes. RETIRED_V19_BASELINE is
# kept ONLY as a fingerprint so main() can warn if someone points back at it.
RETIRED_V19_BASELINE = REPO / "output/ydf-validation-gate/v19_gated.parquet"
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


def transition_counts(baseline: Path, candidate: Path, sample: Path,
                      remap: dict | None = None) -> list[tuple]:
    """Rows of (s19, scand, oracle_dst, oracle_src, cnt) on the sample.

    The oracle is the baseline's gated YDF. We classify it against BOTH ends of
    the transition so the bands can tell a CREATED false positive from an
    INHERITED one:
      oracle_dst — oracle vs the CANDIDATE label (agree / contradict / silent)
      oracle_src — oracle vs the BASE (v19) label  (agree / contradict / silent)

    Why both: a move A->B that the oracle refutes (oracle != B) is only a NEW
    false positive if the column was correct before — i.e. the oracle CONFIRMED
    the source (oracle == A). If the oracle refuted A too, the column was already
    wrong; demoting it (e.g. A -> unknown) trades a confident mistake for an
    honest abstention and must not be scored as a regression. Without oracle_src
    the gate counts every demote-to-unknown as oracle_fp and every correct-FP
    removal as a collapse — punishing the Precision Principle it exists to serve.
    """
    # Oracle in the CANDIDATE's vocabulary. The gated YDF speaks a FIXED label
    # space; when a candidate retires/renames a label (e.g. the enum reframe
    # abolishes `representation.discrete.categorical` -> `representation.text.word`,
    # spec 2026-06-17-enum-accuracy-reframe), the oracle's old-label verdicts must be
    # translated or every retired-label column reads as a tautological "contradiction"
    # (oracle says the abolished label, candidate says its successor) and the collapse
    # band false-alarms. `--label-remap OLD=NEW` applies the SAME remap to the oracle
    # `y` so the referee judges in the candidate's vocabulary. DEFAULT EMPTY -> identity:
    # every other candidate (the four-verdict regression) is byte-identical. Safe: it
    # only neutralises the remapped label; a genuine misclassification (oracle says
    # `city`, candidate says `word`) is NOT remapped and still trips oracle_fp.
    y_expr = "b.y"
    for old, new in (remap or {}).items():
        oo = old.replace("'", "''")
        nn = new.replace("'", "''")
        y_expr = f"CASE WHEN {y_expr} = '{oo}' THEN '{nn}' ELSE {y_expr} END"
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
                  WHEN ({y_expr}) = c.s THEN 'agree'
                  ELSE 'contradict' END AS oracle_dst,
             CASE WHEN b.y IS NULL THEN 'silent'
                  WHEN ({y_expr}) = b.s THEN 'agree'
                  ELSE 'contradict' END AS oracle_src
      FROM b JOIN c USING(file_path, column_name))
SELECT s19, scand, oracle_dst, oracle_src, COUNT(*) cnt FROM j GROUP BY 1,2,3,4;
"""
    rows = duck_csv(sql)
    return [(r[0], r[1], r[2], r[3], int(r[4])) for r in rows]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--baseline", type=Path, default=None,
                    help="REQUIRED (no default). Parquet of the STANDING "
                         "CURRENT-DEFAULT model's corpus pass on the stratified "
                         "sample, carrying sense_prediction + ydf_prediction_gated. "
                         "The gate scores the candidate's relocation against THIS "
                         "reference. Do NOT pass the retired "
                         "output/ydf-validation-gate/v19_gated.parquet — that scores "
                         "deviation-from-v19 (choice 0104).")
    ap.add_argument("--candidate", type=Path, required=True)
    ap.add_argument("--label-remap", action="append", metavar="OLD=NEW",
                    help="Remap an oracle label before judging, so the referee speaks the "
                         "candidate's vocabulary when the candidate retires/renames a label "
                         "(e.g. representation.discrete.categorical=representation.text.word for "
                         "the enum reframe). Repeatable. Default none -> identity (every other "
                         "candidate is byte-identical; preserves the four-verdict regression).")
    ap.add_argument("--sample", type=Path, default=DEFAULT_SAMPLE)
    ap.add_argument("--resolution", type=Path, default=DEFAULT_RES)
    ap.add_argument("--label", default="candidate", help="name for the report")
    ap.add_argument("--rel-mult", type=float, default=3.0,
                    help="over-emit: est marginal / v19 marginal >= this")
    ap.add_argument("--collapse-frac", type=float, default=0.6,
                    help="collapse: oracle-confirmed cand support / v19 support <= this")
    ap.add_argument("--collapse-correct-floor", type=float, default=1000,
                    help="collapse: min oracle-confirmed v19 support for the band to "
                         "fire (suppresses ratio noise on labels with little ground "
                         "truth — an over-emitted label has scant correct support, so "
                         "demoting it is not a collapse)")
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

    if args.baseline is None:
        print(
            "error: --baseline is REQUIRED and has no default.\n"
            "  Pass the parquet of the STANDING CURRENT-DEFAULT model's corpus pass on the\n"
            "  ac-01 stratified sample (its sense_prediction + ydf_prediction_gated columns).\n"
            "  The gate must score the candidate's relocation against the CURRENT default,\n"
            "  never the retired v19 oracle — measuring deviation-from-v19 is the fossil this\n"
            "  removes (0% structural pass for any retrain; choice 0104).\n"
            "  Generate the reference pass over the ac-01 sample with --fill-ydf, then:\n"
            "    scripts/corpus_honest_gate.py --baseline <current_default>.parquet \\\n"
            "                                  --candidate <candidate>.parquet",
            file=sys.stderr,
        )
        return 2
    blooks = str(args.baseline).lower()
    if args.baseline.resolve() == RETIRED_V19_BASELINE.resolve() or "v19_gated" in blooks:
        print(
            "WARNING: --baseline looks like the RETIRED v19 oracle "
            "(output/ydf-validation-gate/v19_gated.parquet).\n"
            "  This gate would then score deviation-from-v19 — structurally unpassable by any\n"
            "  model retrain (0% pass rate; choice 0104). Use the STANDING CURRENT-DEFAULT\n"
            "  baseline unless you are deliberately reproducing a historical verdict.",
            file=sys.stderr,
        )

    remap = {}
    for pair in (args.label_remap or []):
        old, _, new = pair.partition("=")
        if not old or not new:
            print(f"error: --label-remap expects OLD=NEW, got {pair!r}", file=sys.stderr)
            return 2
        remap[old] = new

    res = json.load(open(args.resolution))
    corpus_full = {x["label"]: x["full_cols"] for x in res["per_label"]}

    trans = transition_counts(args.baseline, args.candidate, args.sample, remap)

    # sample rate per v19 source label A = observed sample cols(A) / corpus cols(A)
    sample_src = {}
    for row in trans:
        a, cnt = row[0], row[4]
        sample_src[a] = sample_src.get(a, 0) + cnt
    rate = {a: sample_src[a] / corpus_full[a] for a in sample_src if corpus_full.get(a)}

    # scaled corpus flows
    labels = set(corpus_full) | {row[1] for row in trans}
    est_marginal = {b: 0.0 for b in labels}        # est candidate marginal
    contra_in = {b: 0.0 for b in labels}           # CREATED FP inflow X->B (oracle confirmed X, refutes B)
    contra_out = {b: 0.0 for b in labels}          # correct support B shed by a CREATED-FP move out
    silent_in = {b: 0.0 for b in labels}           # inflow X->B oracle silent (X!=B)
    obs_contra_in = {b: 0 for b in labels}         # RAW (unscaled) CREATED-FP inflow count
    base_correct = {b: 0.0 for b in labels}        # oracle-CONFIRMED baseline (v19) support of B
    cand_correct = {b: 0.0 for b in labels}        # oracle-CONFIRMED candidate support of B
    for a, b, odst, osrc, cnt in trans:
        r = rate.get(a)
        if not r:
            continue
        scaled = cnt / r
        est_marginal[b] += scaled
        # oracle-confirmed support at each end (the truth the bands defend)
        if osrc == "agree":
            base_correct[a] += scaled
        if odst == "agree":
            cand_correct[b] += scaled
        if a != b:
            # CREATED false positive: the column was CORRECT as A (oracle == A)
            # and is now refuted as B. A move the oracle refuted at BOTH ends
            # (osrc == 'contradict') was already wrong — demoting it to B
            # (e.g. unknown) is not a regression, so it never counts here.
            if osrc == "agree":
                contra_in[b] += scaled
                contra_out[a] += scaled
                obs_contra_in[b] += cnt
            elif odst == "silent":
                silent_in[b] += scaled

    rows = []
    verdict = "GO"
    triggers = []
    for b in labels:
        v19m = corpus_full.get(b, 0)
        em = est_marginal[b]
        ratio = (em / v19m) if v19m else float("inf") if em else 0.0
        # net CREATED false positives on B: oracle-confirmed predictions relocated
        # onto B, minus oracle-confirmed support B itself lost to a created-FP move.
        net_contra = contra_in[b] - contra_out[b]
        # oracle-CONFIRMED support change — collapse is a loss of CORRECT mass,
        # not of raw marginal (demoting an over-emitted label is not a collapse).
        bc, cc = base_correct[b], cand_correct[b]
        correct_ratio = (cc / bc) if bc else (float("inf") if cc else 1.0)
        # over_emit is composition-aware (author-accepted 2026-06-12, sibling
        # of the 0.6.24 oracle-aware refinement): growth the oracle CONFIRMS
        # as correct is netted out of the ratio, so stacked honest fixes in
        # one direction cannot trip the band while relocation still does.
        # v23's explosion stays caught: its categorical growth was oracle-
        # refuted, not confirmed, so netting confirmed growth changes nothing
        # for it (four-verdict reproduction re-run with this band — see
        # output/corpus-honest-gate/refined/composition_aware_over_emit.md).
        confirmed_growth = max(0.0, cc - bc)
        adj_ratio = ((em - confirmed_growth) / v19m) if v19m else ratio
        bands = []
        if v19m >= args.marginal_floor and adj_ratio >= args.rel_mult:
            bands.append("over_emit")
        if bc >= args.collapse_correct_floor and correct_ratio <= args.collapse_frac:
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
            "adj_ratio": round(adj_ratio, 3),
            "base_correct": round(bc), "cand_correct": round(cc),
            "correct_ratio": round(correct_ratio, 3),
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
        "oracle_label_remap": remap or None,
        "bands": {"rel_mult": args.rel_mult, "collapse_frac": args.collapse_frac,
                  "collapse_correct_floor": args.collapse_correct_floor,
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
