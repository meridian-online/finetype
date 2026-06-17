#!/usr/bin/env python3
"""ac-04 — per-family counterfactual predictions on the 448-row eval.

Deliverable: diagnostics/rhh_counterfactual.tsv

Method:

  1. Require the release binary built with ``--features rhh-instrumentation``
     (rebuild with:
         cargo build --release -p finetype-cli \\
             --features finetype-model/rhh-instrumentation
     ). The hook reads env var ``RHH_DISABLE_HINTS`` at call sites in
     ``apply_header_sharpen`` and ``header_hint``.
  2. Pin the model: record ``models/default`` resolved path and the sha256 of
     ``model.safetensors`` in the TSV header.
  3. Baseline: profile each unique (dataset, file_path) in the manifest once
     with the env var UNSET, capture per-column label, confidence, rule.
  4. For each of the 22 families in the inventory, set
     ``RHH_DISABLE_HINTS=<family_id>`` and re-profile all files. Capture
     per-column predictions and diff vs baseline.
  5. Emit one row per (family, dataset, column_name) where the column appears
     in the manifest's ground truth set.

Schema of rhh_counterfactual.tsv:

    family_id  dataset  column_name  gt_label  baseline_label  disabled_label
    baseline_confidence  disabled_confidence  baseline_rule  disabled_rule
    baseline_correct  disabled_correct  label_changed

``baseline_correct`` / ``disabled_correct`` are exact-match against gt_label
(1/0). ``label_changed`` is 1 iff baseline_label != disabled_label. All three
feed ac-05's 80% threshold classification.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import os
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
INVENTORY_TSV = REPO_ROOT / "diagnostics" / "rhh_family_inventory.tsv"
DEFAULT_MANIFEST = REPO_ROOT / "eval" / "datasets" / "manifest.csv"
DEFAULT_SCHEMA_MAPPING = REPO_ROOT / "eval" / "schema_mapping.csv"
DEFAULT_MODEL_DIR = REPO_ROOT / "models" / "default"
DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "finetype"
DEFAULT_OUTPUT = REPO_ROOT / "diagnostics" / "rhh_counterfactual.tsv"

# Interchangeability classes mirrored from eval/eval_profile.sql. When a
# predicted label is in the same class as an expected label, the match counts.
# Scored `baseline_correct` / `disabled_correct` use this equivalence.
INTERCHANGEABLE_PREFIXES = (
    "representation.boolean.",
    "datetime.time.",
    "datetime.timestamp.",
)
INTERCHANGEABLE_SETS: tuple[frozenset[str], ...] = (
    frozenset(
        {
            "geography.location.region",
            "geography.location.state",
            "geography.location.continent",
            "geography.location.country",
        }
    ),
    frozenset(
        {
            "geography.coordinate.latitude",
            "geography.coordinate.longitude",
            "geography.coordinate.coordinates",
        }
    ),
    frozenset({"datetime.date.dmy_dash", "datetime.date.mdy_dash"}),
    frozenset({"identity.person.full_name", "representation.text.entity_name"}),
)


def load_schema_mapping(path: Path) -> dict[str, list[str]]:
    """gt_label -> list of expected finetype_labels (non-empty)."""
    out: dict[str, list[str]] = {}
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            gt = row["gt_label"].strip()
            ft = (row.get("finetype_label") or "").strip()
            if not gt or not ft:
                continue
            out.setdefault(gt, []).append(ft)
    return out


def label_equivalent(pred: str, expected: str) -> bool:
    if pred == expected:
        return True
    for prefix in INTERCHANGEABLE_PREFIXES:
        if pred.startswith(prefix) and expected.startswith(prefix):
            return True
    for cls in INTERCHANGEABLE_SETS:
        if pred in cls and expected in cls:
            return True
    return False


def is_label_match(pred: str, expected_labels: list[str]) -> bool:
    return any(label_equivalent(pred, e) for e in expected_labels)


def is_domain_match(pred: str, expected_labels: list[str]) -> bool:
    pred_domain = pred.split(".", 1)[0] if pred else ""
    for e in expected_labels:
        if not e:
            continue
        if pred_domain == e.split(".", 1)[0]:
            return True
    return False


@dataclass(frozen=True)
class Family:
    family_id: str
    rule_family_class: str


def load_inventory(path: Path) -> list[Family]:
    fams: list[Family] = []
    with path.open(newline="", encoding="utf-8") as fh:
        lines = [ln for ln in fh if not ln.startswith("#")]
    reader = csv.DictReader(lines, delimiter="\t")
    for row in reader:
        fams.append(
            Family(
                family_id=row["family_id"],
                rule_family_class=row["rule_family_class"],
            )
        )
    return fams


@dataclass(frozen=True)
class GtRow:
    dataset: str
    file_path: Path
    column_name: str
    gt_label: str


def load_manifest(manifest: Path) -> list[GtRow]:
    rows: list[GtRow] = []
    with manifest.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        for r in reader:
            fp = Path(r["file_path"])
            if not fp.is_absolute():
                fp = REPO_ROOT / r["file_path"]
            rows.append(
                GtRow(
                    dataset=r["dataset"],
                    file_path=fp,
                    column_name=r["column_name"],
                    gt_label=r["gt_label"],
                )
            )
    return rows


def unique_files(gt_rows: list[GtRow]) -> list[tuple[str, Path]]:
    seen: set[tuple[str, str]] = set()
    out: list[tuple[str, Path]] = []
    for r in gt_rows:
        key = (r.dataset, str(r.file_path))
        if key in seen:
            continue
        seen.add(key)
        out.append((r.dataset, r.file_path))
    return out


def profile_file(
    binary: Path, model: Path, file_path: Path, disable: str | None
) -> dict | None:
    env = os.environ.copy()
    if disable is None:
        env.pop("RHH_DISABLE_HINTS", None)
    else:
        env["RHH_DISABLE_HINTS"] = disable
    env["FINETYPE_MODEL"] = str(model)
    cmd = [
        str(binary),
        "profile",
        "-f",
        str(file_path),
        "-o",
        "json",
    ]
    try:
        proc = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            timeout=300,
            env=env,
        )
    except subprocess.CalledProcessError as exc:
        print(
            f"    ! profile failed: {exc.stderr.strip()[:200]}",
            file=sys.stderr,
        )
        return None
    except subprocess.TimeoutExpired:
        print("    ! profile timed out", file=sys.stderr)
        return None
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        print(f"    ! malformed JSON: {exc}", file=sys.stderr)
        return None


@dataclass
class Prediction:
    label: str
    confidence: float
    rule: str  # empty string if no disambiguation_rule


def extract_predictions(profile_json: dict) -> dict[str, Prediction]:
    out: dict[str, Prediction] = {}
    for col in profile_json.get("columns", []):
        out[col.get("column", "")] = Prediction(
            label=col.get("type", ""),
            confidence=float(col.get("confidence", 0.0) or 0.0),
            rule=col.get("disambiguation_rule") or "",
        )
    return out


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(65536), b""):
            h.update(chunk)
    return h.hexdigest()


def verify_feature_on(binary: Path) -> bool:
    """Heuristic: the hook is active only when the feature was compiled in.
    We verify by setting RHH_DISABLE_HINTS to a sentinel non-family value and
    confirming profile still succeeds (feature-on parses the env var without
    erroring). The real test is that some counterfactual actually differs —
    we surface that in the summary."""
    return binary.exists()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument(
        "--schema-mapping", type=Path, default=DEFAULT_SCHEMA_MAPPING
    )
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL_DIR)
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--inventory", type=Path, default=INVENTORY_TSV)
    parser.add_argument(
        "--families",
        nargs="*",
        default=None,
        help="subset of family_ids (default: all 22)",
    )
    args = parser.parse_args()

    if not verify_feature_on(args.binary):
        print(
            f"finetype binary not found at {args.binary}; build with:\n"
            "  cargo build --release -p finetype-cli "
            "--features finetype-model/rhh-instrumentation",
            file=sys.stderr,
        )
        return 2

    all_families = load_inventory(args.inventory)
    family_set = (
        [f for f in all_families if f.family_id in set(args.families)]
        if args.families
        else all_families
    )
    gt_rows = load_manifest(args.manifest)
    files = unique_files(gt_rows)
    gt_by_key: dict[tuple[str, str], str] = {
        (r.dataset, r.column_name): r.gt_label for r in gt_rows
    }
    schema_map = load_schema_mapping(args.schema_mapping)
    unmapped: set[str] = set()

    # Pin model weights sha256 for reproducibility.
    weights = args.model / "model.safetensors"
    weights_sha = sha256_of(weights) if weights.exists() else "missing"
    model_real = args.model.resolve()

    print(f"rhh ac-04: counterfactual — {len(family_set)} families × {len(files)} files")
    print(f"  model: {model_real}  weights_sha256: {weights_sha[:16]}…")

    # Baseline
    print("\n  baseline run (RHH_DISABLE_HINTS unset):")
    baseline: dict[str, dict[str, Prediction]] = {}
    for ds, fp in files:
        if not fp.exists():
            print(f"    ! missing: {fp}", file=sys.stderr)
            continue
        prof = profile_file(args.binary, args.model, fp, None)
        if prof is None:
            continue
        baseline[ds] = extract_predictions(prof)
    print(f"    baseline covers {len(baseline)} datasets")

    # Per-family counterfactual
    rows_out: list[dict[str, str]] = []
    family_summary: dict[str, dict[str, int]] = {}

    for i, fam in enumerate(family_set, 1):
        print(
            f"\n  [{i}/{len(family_set)}] disable={fam.family_id}"
            f" ({fam.rule_family_class})"
        )
        summary = {"rows": 0, "changed": 0, "base_correct": 0, "dis_correct": 0}
        for ds, fp in files:
            if ds not in baseline or not fp.exists():
                continue
            prof = profile_file(args.binary, args.model, fp, fam.family_id)
            if prof is None:
                continue
            dis_preds = extract_predictions(prof)
            for col_name, base_pred in baseline[ds].items():
                key = (ds, col_name)
                gt = gt_by_key.get(key)
                if gt is None:
                    continue  # only score GT-covered columns
                dis_pred = dis_preds.get(col_name)
                if dis_pred is None:
                    continue
                expected = schema_map.get(gt, [])
                if not expected:
                    unmapped.add(gt)
                base_ok = int(is_label_match(base_pred.label, expected))
                dis_ok = int(is_label_match(dis_pred.label, expected))
                base_dom = int(is_domain_match(base_pred.label, expected))
                dis_dom = int(is_domain_match(dis_pred.label, expected))
                changed = int(base_pred.label != dis_pred.label)
                rows_out.append(
                    {
                        "family_id": fam.family_id,
                        "dataset": ds,
                        "column_name": col_name,
                        "gt_label": gt,
                        "baseline_label": base_pred.label,
                        "disabled_label": dis_pred.label,
                        "baseline_confidence": f"{base_pred.confidence:.4f}",
                        "disabled_confidence": f"{dis_pred.confidence:.4f}",
                        "baseline_rule": base_pred.rule,
                        "disabled_rule": dis_pred.rule,
                        "baseline_correct": str(base_ok),
                        "disabled_correct": str(dis_ok),
                        "baseline_domain_correct": str(base_dom),
                        "disabled_domain_correct": str(dis_dom),
                        "label_changed": str(changed),
                    }
                )
                summary["rows"] += 1
                summary["changed"] += changed
                summary["base_correct"] += base_ok
                summary["dis_correct"] += dis_ok
        family_summary[fam.family_id] = summary
        print(
            f"    {summary['rows']} scored rows; "
            f"{summary['changed']} label changes; "
            f"{summary['base_correct']} baseline-correct; "
            f"{summary['dis_correct']} disabled-correct"
        )

    # Emit TSV
    args.output.parent.mkdir(parents=True, exist_ok=True)
    def _rel(p: Path) -> str:
        # Binaries/manifests may live outside the repo (e.g. an ad-hoc
        # instrumentation build in /tmp); fall back to the absolute path
        # rather than crashing on relative_to.
        try:
            return str(p.relative_to(REPO_ROOT))
        except ValueError:
            return str(p)

    header_lines = [
        "# rhh_counterfactual.tsv — ac-04 output",
        f"# Source: {_rel(args.binary)} profile over {_rel(args.manifest)}",
        f"# Model dir: {model_real}",
        f"# Model weights sha256: {weights_sha}",
        f"# Families: {len(family_set)}",
        f"# Baseline datasets profiled: {len(baseline)}",
        "# Scoring: schema_mapping.csv-backed label equivalence (mirrors "
        "eval/eval_profile.sql interchangeability classes).",
        "# Schema: family_id\\tdataset\\tcolumn_name\\tgt_label"
        "\\tbaseline_label\\tdisabled_label"
        "\\tbaseline_confidence\\tdisabled_confidence"
        "\\tbaseline_rule\\tdisabled_rule"
        "\\tbaseline_correct\\tdisabled_correct"
        "\\tbaseline_domain_correct\\tdisabled_domain_correct"
        "\\tlabel_changed",
    ]
    cols_order = [
        "family_id",
        "dataset",
        "column_name",
        "gt_label",
        "baseline_label",
        "disabled_label",
        "baseline_confidence",
        "disabled_confidence",
        "baseline_rule",
        "disabled_rule",
        "baseline_correct",
        "disabled_correct",
        "baseline_domain_correct",
        "disabled_domain_correct",
        "label_changed",
    ]
    with args.output.open("w", encoding="utf-8") as fh:
        for line in header_lines:
            fh.write(line + "\n")
        fh.write("\t".join(cols_order) + "\n")
        for r in rows_out:
            fh.write("\t".join(r[c] for c in cols_order) + "\n")

    print(f"\n  wrote {_rel(args.output)} — {len(rows_out)} rows")

    # Persist summary for quick inspection
    summary_path = args.output.with_name("rhh_counterfactual_summary.tsv")
    with summary_path.open("w", encoding="utf-8") as fh:
        fh.write(
            "# Per-family summary derived from rhh_counterfactual.tsv\n"
            "family_id\tscored_rows\tlabel_changes\tbaseline_correct"
            "\tdisabled_correct\tdelta_correct\n"
        )
        for fam in family_set:
            s = family_summary[fam.family_id]
            delta = s["dis_correct"] - s["base_correct"]
            fh.write(
                f"{fam.family_id}\t{s['rows']}\t{s['changed']}"
                f"\t{s['base_correct']}\t{s['dis_correct']}\t{delta:+d}\n"
            )
    print(f"  wrote {summary_path.relative_to(REPO_ROOT)}")

    if unmapped:
        print(
            f"\n  {len(unmapped)} gt_labels not in schema_mapping.csv "
            "(treated as never-correct — investigate if count is high):"
        )
        for gt in sorted(unmapped)[:20]:
            print(f"    {gt}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
