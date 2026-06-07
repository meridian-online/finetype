set -e
F=output/late-fusion/fusion_feats_v26
BIN=./target/release/train-fusion-head
run() {
  name=$1; shift
  echo "=== $name :: $* ==="
  $BIN train --features $F --output output/late-fusion/sweep_v26/$name --epochs 60 --seed 42 "$@" 2>&1 | tail -8
}
run h768_d10 --hidden1 768 --hidden2 384 --dropout 0.1
run h512_d20 --hidden1 512 --hidden2 256 --dropout 0.2
run h1024_d15 --hidden1 1024 --hidden2 256 --dropout 0.15
run h384_d05 --hidden1 384 --hidden2 192 --dropout 0.05
