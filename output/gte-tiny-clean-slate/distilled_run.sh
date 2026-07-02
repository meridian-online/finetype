#!/bin/bash
cd /Users/hugh/github/meridian-online/finetype || exit 1
VENV=output/fine-tuned-encoder-discovery/.venv/bin/python
D=output/gte-tiny-clean-slate
R=$D/DISTILLED_RESULT.md
mkdir -p /tmp/dsc
score(){ rm -f /tmp/dsc/report_*.md 2>/dev/null; python3 scripts/score_gold_anchor.py score --gold "$1" --predictions "$2" --model-name t --out-dir /tmp/dsc $3 >/dev/null 2>&1; grep -hE "Headline" /tmp/dsc/report_*.md|head -1|grep -oE "= [0-9.]+"|grep -oE "[0-9.]+"|head -1; }
echo "# Distilled-data retrain (gte-tiny on Sherlock-distilled + structural) — $(date)" > $R
echo "v19: gold 0.798 / repr 0.691 | corpus-label clean-slate: gold 0.770 / repr 0.618" >> $R
$VENV $D/overnight_train.py --data $D/cs_train_distilled.tsv --head mlp --epochs 8 --snapshots 8 --prefix distilled
$VENV $D/overnight_score.py $D/distilled_e8.pt
gco=$(score eval/gold/gold_corpus.tsv /tmp/on_distilled_e8_gold_composed.tsv "")
gsa=$(score eval/gold/gold_corpus.tsv /tmp/on_distilled_e8_gold_standalone.tsv "")
rco=$(score eval/repr/representative_corpus.tsv /tmp/on_distilled_e8_repr_composed.tsv "--reframe")
rsa=$(score eval/repr/representative_corpus.tsv /tmp/on_distilled_e8_repr_standalone.tsv "--reframe")
echo "| | gold standalone | gold composed | repr standalone | repr composed |" >> $R
echo "|---|---|---|---|---|" >> $R
echo "| distilled_e8 | ${gsa:-ERR} | ${gco:-ERR} | ${rsa:-ERR} | ${rco:-ERR} |" >> $R
echo "Done $(date)" >> $R
