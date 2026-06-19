#!/bin/bash
# Overnight clean-label retrain + auto-score. Result table -> OVERNIGHT_RESULT.md
cd /Users/hugh/github/meridian-online/finetype || exit 1
VENV=output/fine-tuned-encoder-discovery/.venv/bin/python
D=output/gte-tiny-clean-slate
DATA=$D/cs_train_clean.tsv
RESULT=$D/OVERNIGHT_RESULT.md
mkdir -p /tmp/onsc

score() { # gold_tsv pred reframe_flag
  rm -f /tmp/onsc/report_*.md 2>/dev/null
  python3 scripts/score_gold_anchor.py score --gold "$1" --predictions "$2" --model-name t --out-dir /tmp/onsc $3 >/dev/null 2>&1
  grep -hE "Headline" /tmp/onsc/report_*.md 2>/dev/null | head -1 | grep -oE "= [0-9.]+" | grep -oE "[0-9.]+" | head -1
}

echo "# Overnight clean-label retrain — started $(date)" > $RESULT
echo "" >> $RESULT
echo "Training set: cs_train_clean.tsv (152,639 rows, 205 labels, 25% residual, model-independent)." >> $RESULT
echo "v19 baseline: gold 0.798, repr 0.691 (--reframe)." >> $RESULT
echo "" >> $RESULT

# Run A — linear head, 16 epochs, trajectory snapshots
$VENV $D/overnight_train.py --data $DATA --head linear --epochs 16 --snapshots 8,12,16 --prefix clean_lin
# Run B — MLP head, 16 epochs
$VENV $D/overnight_train.py --data $DATA --head mlp --epochs 16 --snapshots 16 --prefix clean_mlp

echo "| checkpoint | gold standalone | gold composed | repr standalone | repr composed |" >> $RESULT
echo "|---|---|---|---|---|" >> $RESULT
for ck in clean_lin_e8 clean_lin_e12 clean_lin_e16 clean_mlp_e16; do
  [ -f $D/$ck.pt ] || continue
  $VENV $D/overnight_score.py $D/$ck.pt
  gsa=$(score eval/gold/gold_corpus.tsv /tmp/on_${ck}_gold_standalone.tsv "")
  gco=$(score eval/gold/gold_corpus.tsv /tmp/on_${ck}_gold_composed.tsv "")
  rsa=$(score eval/repr/representative_corpus.tsv /tmp/on_${ck}_repr_standalone.tsv "--reframe")
  rco=$(score eval/repr/representative_corpus.tsv /tmp/on_${ck}_repr_composed.tsv "--reframe")
  echo "| $ck | ${gsa:-ERR} | ${gco:-ERR} | ${rsa:-ERR} | ${rco:-ERR} |" >> $RESULT
done
echo "" >> $RESULT
echo "Finished $(date)" >> $RESULT
