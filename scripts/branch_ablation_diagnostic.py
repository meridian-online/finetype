#!/usr/bin/env python3
"""ac-06 — Embed-branch utilisation diagnostic.

Probes a v21 model vs a v22 model on a 200-column probe set drawn from
the v21 corpus pass:
  - Partition A: 100 columns where ydf_prediction = geography.location.city
    and sense_prediction != geography.* (city-misses).
  - Partition B: 100 columns where sense_prediction = identity.person.full_name
    (the full_name confusables hard negatives aim to address).

For each column, runs inference under three branch configurations:
  - default: all branches active
  - embed-ablated: embed-branch output zeroed before merge
  - char-ablated: char-branch output zeroed before merge

Inference path: re-implements the multi-branch forward pass in numpy,
loading weights from `model.safetensors`. Feature vectors come from
`finetype extract-features --json --header H --validation` invoked
per column.

The load-bearing read: v22 with full branches should beat v22 with
embed ablated by a meaningful margin on partition B (the boundary
work the embed branch is supposed to be doing). v21 should show a
small or no margin (consistent with v21's failure mode — the embed
branch wasn't carrying weight).

Per spec 2026-05-25-v22-boundary-training ac-06.

Output:
  output/branch-ablation-v22/probe_results.md     — delta table + narrative
  output/branch-ablation-v22/probe_columns.tsv    — the 200 probe rows
  output/branch-ablation-v22/predictions.json     — raw per-model/per-config preds
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_CORPUS = REPO / "output/corpus-pass-v21/corpus_pass/columns.parquet"
DEFAULT_OUT_DIR = REPO / "output/branch-ablation-v22"
DEFAULT_FINETYPE_BIN = REPO / "target/release/finetype"


def relu(x):
    import numpy as np
    return np.maximum(x, 0.0)


def linear(x, w, b):
    return x @ w.T + b


def layer_norm(x, w, b, eps=1e-5):
    import numpy as np
    mean = x.mean(axis=-1, keepdims=True)
    var = x.var(axis=-1, keepdims=True)
    return (x - mean) / np.sqrt(var + eps) * w + b


def batch_norm_eval(x, w, b, running_mean, running_var, eps=1e-5):
    import numpy as np
    return (x - running_mean) / np.sqrt(running_var + eps) * w + b


def branch_forward(tensors, prefix, x, has_input_ln=False):
    if has_input_ln:
        x = layer_norm(
            x,
            tensors[f"{prefix}.input_ln.weight"],
            tensors[f"{prefix}.input_ln.bias"],
        )
    h = linear(x, tensors[f"{prefix}.l1.weight"], tensors[f"{prefix}.l1.bias"])
    h = relu(h)
    h = linear(h, tensors[f"{prefix}.l2.weight"], tensors[f"{prefix}.l2.bias"])
    h = relu(h)
    return h


def model_forward(tensors, features, ablate=None):
    """Run a forward pass, optionally zeroing one branch's output.

    `features`: dict with keys char (960), embed (512), stats (27),
                header_features (128), validation (240).
    `ablate`:   None | 'embed' | 'char' | 'stats' | 'header' | 'validation'.
    Returns argmax index over n_classes.
    """
    import numpy as np

    char = np.asarray(features["char"], dtype=np.float32)
    embed = np.asarray(features["embed"], dtype=np.float32)
    stats = np.asarray(features["stats"], dtype=np.float32)
    header = np.asarray(features["header_features"], dtype=np.float32)
    valid = np.asarray(features.get("validation") or [], dtype=np.float32)

    char_out = branch_forward(tensors, "char", char)
    embed_out = branch_forward(tensors, "embed", embed)
    stats_out = branch_forward(tensors, "stats", stats)

    has_header = "header.l1.weight" in tensors
    has_valid = "valid.l1.weight" in tensors and valid.size > 0
    header_out = (branch_forward(tensors, "header", header, has_input_ln=True)
                  if has_header else None)
    valid_out = branch_forward(tensors, "valid", valid) if has_valid else None

    if ablate == "embed":
        embed_out[:] = 0.0
    elif ablate == "char":
        char_out[:] = 0.0
    elif ablate == "stats":
        stats_out[:] = 0.0
    elif ablate == "header" and header_out is not None:
        header_out[:] = 0.0
    elif ablate == "validation" and valid_out is not None:
        valid_out[:] = 0.0

    parts = [char_out, embed_out, stats_out]
    if header_out is not None:
        parts.append(header_out)
    if valid_out is not None:
        parts.append(valid_out)
    merged = np.concatenate(parts, axis=-1)

    merged = batch_norm_eval(
        merged,
        tensors["merge_bn.weight"],
        tensors["merge_bn.bias"],
        tensors["merge_bn.running_mean"],
        tensors["merge_bn.running_var"],
    )
    h = linear(merged, tensors["merge_l1.weight"], tensors["merge_l1.bias"])
    h = relu(h)
    h = linear(h, tensors["merge_l2.weight"], tensors["merge_l2.bias"])
    h = relu(h)
    logits = linear(h, tensors["head.weight"], tensors["head.bias"])
    return int(logits.argmax())


def extract_features(values: list[str], header: str,
                     finetype_bin: Path) -> dict:
    payload = json.dumps(values, ensure_ascii=False)
    proc = subprocess.run(
        [
            str(finetype_bin), "extract-features",
            "--json", "--header", header, "--validation",
        ],
        input=payload, capture_output=True, text=True, check=True,
    )
    return json.loads(proc.stdout)


def build_probe_set(corpus_parquet: Path, n_per_partition: int = 100,
                    seed: int = 42) -> list[dict]:
    import duckdb
    con = duckdb.connect()
    arrow_a = con.execute(f"""
        SELECT column_name, sample_values_truncated, ydf_prediction,
               sense_prediction
          FROM read_parquet('{corpus_parquet.as_posix()}')
         WHERE ydf_prediction = 'geography.location.city'
           AND sense_prediction NOT LIKE 'geography.%'
           AND ydf_confidence >= 0.5
         ORDER BY hash(file_path || column_name || '{seed}')
         LIMIT {n_per_partition}
    """).to_arrow_table()
    arrow_b = con.execute(f"""
        SELECT column_name, sample_values_truncated, ydf_prediction,
               sense_prediction
          FROM read_parquet('{corpus_parquet.as_posix()}')
         WHERE sense_prediction = 'identity.person.full_name'
         ORDER BY hash(file_path || column_name || '{seed}')
         LIMIT {n_per_partition}
    """).to_arrow_table()

    SAMPLE_SEPARATOR = "│"
    probes: list[dict] = []
    for partition, arrow in [("A_city_misses", arrow_a),
                             ("B_full_name_confusables", arrow_b)]:
        cols = {n: arrow.column(n).to_pylist() for n in arrow.column_names}
        for i in range(arrow.num_rows):
            samples_raw = cols["sample_values_truncated"][i] or ""
            samples = [s for s in samples_raw.split(SAMPLE_SEPARATOR) if s]
            if len(samples) < 3:
                continue
            ground_truth = ("geography.location.city"
                            if partition.startswith("A_")
                            else "identity.person.full_name")
            probes.append({
                "partition": partition,
                "column_name": cols["column_name"][i] or "",
                "sample_values": samples,
                "ground_truth": ground_truth,
                "ydf_prediction": cols["ydf_prediction"][i],
                "sense_prediction": cols["sense_prediction"][i],
            })
    return probes


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--corpus-parquet", type=Path, default=DEFAULT_CORPUS)
    p.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    p.add_argument("--finetype-bin", type=Path, default=DEFAULT_FINETYPE_BIN)
    p.add_argument("--v21-model", type=Path,
                   default=REPO / "models/sherlock-v21-geonames-geography-relu-s42")
    p.add_argument("--v22-model", type=Path, required=True,
                   help="Path to the v22 cherry-pick model dir.")
    p.add_argument("--n-per-partition", type=int, default=100)
    p.add_argument("--seed", type=int, default=42)
    args = p.parse_args()

    try:
        import numpy as np  # noqa: F401
        import safetensors.numpy as st  # type: ignore
        import duckdb  # noqa: F401
    except ImportError as exc:  # noqa: BLE001
        print(f"error: missing dependency ({exc}). Install: uv pip install "
              f"numpy safetensors duckdb pyarrow", file=sys.stderr)
        return 2

    args.out_dir.mkdir(parents=True, exist_ok=True)

    # ── (1) Probe set ────────────────────────────────────────────────
    print("building probe set...", file=sys.stderr)
    probes = build_probe_set(args.corpus_parquet,
                             n_per_partition=args.n_per_partition,
                             seed=args.seed)
    print(f"  probes: {len(probes)}", file=sys.stderr)
    with (args.out_dir / "probe_columns.tsv").open("w") as f:
        f.write("partition\tcolumn_name\tground_truth\tydf\tsense\tn_values\n")
        for pr in probes:
            f.write(
                f"{pr['partition']}\t{pr['column_name'][:60]}\t"
                f"{pr['ground_truth']}\t{pr['ydf_prediction']}\t"
                f"{pr['sense_prediction']}\t{len(pr['sample_values'])}\n"
            )

    # ── (2) Extract features for each probe ──────────────────────────
    print("extracting features...", file=sys.stderr)
    features_cache: list[dict] = []
    label_map = None
    for j, pr in enumerate(probes):
        feats = extract_features(pr["sample_values"], pr["column_name"],
                                 args.finetype_bin)
        features_cache.append(feats)
        if label_map is None:
            label_map = feats.get("type_index_keys")
        if (j + 1) % 50 == 0:
            print(f"  {j + 1}/{len(probes)}", file=sys.stderr)
    assert label_map is not None, "extract-features must return type_index_keys"

    # ── (3) Load models + run forward passes ─────────────────────────
    models = {"v21": args.v21_model, "v22": args.v22_model}
    ablations = [None, "embed", "char"]

    results: dict[str, dict] = {}
    for mname, mdir in models.items():
        weights_path = mdir / "model.safetensors"
        if not weights_path.exists():
            print(f"warn: {weights_path} missing — skipping {mname}",
                  file=sys.stderr)
            continue
        print(f"loading {mname} weights...", file=sys.stderr)
        tensors = st.load_file(str(weights_path))
        for ab in ablations:
            ab_label = ab or "all"
            correct_by_part: dict[str, int] = {"A_city_misses": 0,
                                                "B_full_name_confusables": 0}
            total_by_part: dict[str, int] = {"A_city_misses": 0,
                                              "B_full_name_confusables": 0}
            preds: list[dict] = []
            for pr, feats in zip(probes, features_cache):
                idx = model_forward(tensors, feats, ablate=ab)
                pred_label = label_map[idx]
                total_by_part[pr["partition"]] += 1
                if pred_label == pr["ground_truth"]:
                    correct_by_part[pr["partition"]] += 1
                preds.append({
                    "partition": pr["partition"],
                    "column_name": pr["column_name"],
                    "predicted": pred_label,
                    "ground_truth": pr["ground_truth"],
                })
            for part in correct_by_part:
                key = f"{mname}__{ab_label}__{part}"
                results[key] = {
                    "model": mname,
                    "ablation": ab_label,
                    "partition": part,
                    "correct": correct_by_part[part],
                    "total": total_by_part[part],
                    "accuracy": (correct_by_part[part] / total_by_part[part]
                                 if total_by_part[part] else 0.0),
                }
            with (args.out_dir / f"predictions_{mname}_{ab_label}.json").open("w") as f:
                json.dump(preds, f, indent=2)

    with (args.out_dir / "predictions.json").open("w") as f:
        json.dump(results, f, indent=2)

    # ── (4) Write markdown report ────────────────────────────────────
    md = ["# v22 branch-ablation diagnostic\n",
          "Per spec `2026-05-25-v22-boundary-training` ac-06.\n",
          "## Probe set\n",
          f"- {args.n_per_partition} columns in **partition A**: "
          "`ydf_prediction = geography.location.city` ∧ "
          "`sense_prediction NOT LIKE 'geography.%'` (the city-misses).\n",
          f"- {args.n_per_partition} columns in **partition B**: "
          "`sense_prediction = identity.person.full_name` "
          "(the full_name confusables hard negatives target).\n",
          "Ground truth is the YDF prediction for partition A and Sense's "
          "prediction for partition B.\n\n",
          "## Accuracy by (model × ablation × partition)\n\n",
          "| Model | Ablation | Partition | Accuracy | Correct/Total |\n",
          "|-------|----------|-----------|----------|---------------|\n"]
    for k in sorted(results):
        r = results[k]
        md.append(f"| {r['model']} | {r['ablation']} | {r['partition']} | "
                  f"{r['accuracy']:.3f} | {r['correct']}/{r['total']} |\n")
    md.append("\n## Reading the result\n\n")
    md.append("The load-bearing comparison is **v22 default vs v22 embed-ablated** "
              "on the probe set. A meaningful margin (≥5 pp) means the Model2Vec "
              "embed branch is finally carrying weight on the boundary v22 was "
              "trained to learn. A small margin (<2 pp) on v22, comparable to v21's "
              "margin, means even the v22 training-data composition didn't get the "
              "embed branch to contribute — pointing at architectural surgery as the "
              "next move (per ac-08's `Failed` band).\n")
    (args.out_dir / "probe_results.md").write_text("".join(md))
    print(f"wrote {args.out_dir / 'probe_results.md'}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
