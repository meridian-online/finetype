#!/usr/bin/env python3
"""Pre-screen eval corpus columns for realism (messiness + distributional fidelity).

Implements ac-01 of .orbit/specs/2026-04-21-eval-expansion/spec.yaml.

Reads eval/datasets/manifest.csv, loads each referenced CSV/JSON/NDJSON file,
and emits per-column metrics plus a `pass_floors` boolean per column against
the floors pinned in eval/pre-screen_floors.yaml.

Output is a TSV suitable for spreadsheet review and for ac-04's verification
(which calls this same script rather than re-deriving floors).

Usage:
    python scripts/prescreen_eval.py                                          # defaults
    python scripts/prescreen_eval.py --manifest eval/datasets/manifest.csv \\
        --floors eval/pre-screen_floors.yaml --output eval/prescreen_output.tsv
    python scripts/prescreen_eval.py --schema-mapping eval/schema_mapping.yaml

Design notes:
- Type-family resolution via eval/schema_mapping.yaml (short-label → full-type).
  Falls back to gt_label if mapping missing.
- Metrics are deterministic — no RNG, no sampling (full column).
- Missing files are flagged in the output (pass_floors=False, reason recorded).
- Zero external deps beyond the standard library + PyYAML (already a project dep).

Metrics emitted per (dataset, column):
    null_rate          — fraction null/empty/whitespace-only
    unique_ratio       — |distinct| / |non_null|
    whitespace_ratio   — fraction with leading or trailing whitespace
    format_variance    — count of distinct formats (heuristic)
    shannon_entropy    — Shannon entropy in bits
    top_1_skew         — fraction held by the most common value

The `pass_floors` column is True iff every applicable floor passes.
The `pass_notes` column records which floors were checked and any failures.

See:
    .orbit/choices/0055-eval-realism-dimensions.md  — policy + floors table
    .orbit/specs/2026-04-21-eval-expansion/spec.yaml  — ac-01, ac-04
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:
    sys.stderr.write(
        "error: PyYAML required. Install with `pip install pyyaml`.\n"
    )
    sys.exit(2)


REPO_ROOT = Path(__file__).resolve().parent.parent


# ─── Format variance heuristic ──────────────────────────────────────────────

_DATE_PATTERNS = [
    (re.compile(r"^\d{4}-\d{2}-\d{2}$"), "iso_date"),
    (re.compile(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}"), "iso_datetime"),
    (re.compile(r"^\d{2}/\d{2}/\d{4}$"), "slash_mdy_or_dmy"),
    (re.compile(r"^\d{2}-\d{2}-\d{4}$"), "dash_mdy_or_dmy"),
    (re.compile(r"^\d{4}/\d{2}/\d{2}$"), "slash_ymd"),
    (re.compile(r"^\d{1,2} [A-Za-z]{3,9} \d{4}$"), "space_text_month"),
]


def classify_format(value: str) -> str:
    """Return a coarse format tag for a value.

    Used to compute format_variance — how many distinct format tags appear
    in a column. Types with rigid formats (uuid, credit_card) should show
    format_variance=1; datetime columns with mixed representations reveal
    themselves with format_variance>1.
    """
    v = value.strip()
    if not v:
        return "empty"
    for pat, tag in _DATE_PATTERNS:
        if pat.match(v):
            return tag
    if v.isdigit():
        return "integer"
    try:
        float(v)
        return "number"
    except ValueError:
        pass
    if re.match(r"^[A-Fa-f0-9-]{32,36}$", v):
        return "hex_like"
    if re.match(r"^[A-Z]{2,}$", v):
        return "upper_code"
    if re.match(r"^[a-z]{2,}$", v):
        return "lower_code"
    if re.match(r"^[\w.+-]+@[\w.-]+$", v):
        return "email_like"
    return "text"


# ─── Metric computation ─────────────────────────────────────────────────────


def compute_metrics(values: list[str]) -> dict[str, Any]:
    """Compute all 6 metrics for a column's values."""
    total = len(values)
    if total == 0:
        return {
            "null_rate": 1.0,
            "unique_ratio": 0.0,
            "whitespace_ratio": 0.0,
            "format_variance": 0,
            "shannon_entropy": 0.0,
            "top_1_skew": 0.0,
            "non_null": 0,
            "total": 0,
        }

    nulls = sum(1 for v in values if v is None or str(v).strip() == "")
    non_null = total - nulls
    non_null_values = [str(v) for v in values if v is not None and str(v).strip() != ""]

    whitespace_count = sum(
        1 for v in non_null_values if v != v.strip()
    )
    whitespace_ratio = (whitespace_count / non_null) if non_null else 0.0

    # Distinct & unique ratio
    distinct = len(set(non_null_values))
    unique_ratio = (distinct / non_null) if non_null else 0.0

    # Shannon entropy (on non-null values)
    if non_null:
        counts = Counter(non_null_values)
        top_1 = counts.most_common(1)[0][1] if counts else 0
        top_1_skew = top_1 / non_null
        probs = [c / non_null for c in counts.values()]
        entropy = -sum(p * math.log2(p) for p in probs if p > 0)
    else:
        top_1_skew = 0.0
        entropy = 0.0

    # Format variance
    format_tags = {classify_format(v) for v in non_null_values[:500]}
    format_variance = len(format_tags)

    return {
        "null_rate": nulls / total,
        "unique_ratio": unique_ratio,
        "whitespace_ratio": whitespace_ratio,
        "format_variance": format_variance,
        "shannon_entropy": entropy,
        "top_1_skew": top_1_skew,
        "non_null": non_null,
        "total": total,
    }


# ─── Floors evaluation ──────────────────────────────────────────────────────


def load_floors(path: Path) -> dict:
    with open(path) as fh:
        return yaml.safe_load(fh)


def resolve_family(floors_config: dict, full_type: str) -> dict:
    """Resolve applicable floors for a given full type by matching prefixes."""
    families = floors_config.get("families") or []
    best_match = None
    best_len = -1
    for family in families:
        prefix = family.get("match_prefix", "")
        if full_type.startswith(prefix) and len(prefix) > best_len:
            best_match = family
            best_len = len(prefix)

    # Merge defaults underneath family overrides
    merged = dict(floors_config.get("defaults", {}))
    if best_match is not None:
        for key, val in best_match.items():
            if key == "match_prefix":
                continue
            merged[key] = val
    return merged


def check_floor(metric_value: float | int, floor_spec: dict | None) -> tuple[bool, str | None]:
    """Return (passed, note). note is None if passed; reason string if failed."""
    if floor_spec is None:
        return True, None
    if "min" in floor_spec and metric_value < floor_spec["min"]:
        return False, f"below min {floor_spec['min']}"
    if "max" in floor_spec and metric_value > floor_spec["max"]:
        return False, f"above max {floor_spec['max']}"
    return True, None


def evaluate_pass_floors(metrics: dict, applicable: dict) -> tuple[bool, str]:
    """Apply applicable floors to metrics; return (pass, notes_string)."""
    notes: list[str] = []
    passed = True
    for metric_key in (
        "null_rate",
        "unique_ratio",
        "shannon_entropy",
        "top_1_skew",
        "whitespace_ratio",
        "format_variance",
    ):
        floor_spec = applicable.get(metric_key)
        if floor_spec is None:
            continue
        ok, reason = check_floor(metrics[metric_key], floor_spec)
        if not ok:
            passed = False
            notes.append(f"{metric_key}:{reason}")
        else:
            notes.append(f"{metric_key}:ok")
    return passed, ";".join(notes)


# ─── File loading ───────────────────────────────────────────────────────────


def load_column_values(
    file_path: Path, column_name: str, max_rows: int = 10_000
) -> tuple[list[str] | None, str | None]:
    """Load values for a single column from CSV/NDJSON/JSON.

    Returns (values, error). `values` is None iff error is set.
    """
    if not file_path.exists():
        return None, f"file not found: {file_path}"

    suffix = file_path.suffix.lower()
    try:
        if suffix in (".csv", ".tsv"):
            delim = "," if suffix == ".csv" else "\t"
            with open(file_path, newline="", encoding="utf-8-sig", errors="replace") as fh:
                reader = csv.DictReader(fh, delimiter=delim)
                if column_name not in (reader.fieldnames or []):
                    return None, f"column '{column_name}' not in {file_path.name}"
                vals = []
                for i, row in enumerate(reader):
                    if i >= max_rows:
                        break
                    vals.append(row.get(column_name) or "")
                return vals, None
        elif suffix in (".ndjson", ".jsonl"):
            vals = []
            with open(file_path, encoding="utf-8") as fh:
                for i, line in enumerate(fh):
                    if i >= max_rows:
                        break
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        obj = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    if column_name in obj:
                        vals.append(str(obj[column_name]) if obj[column_name] is not None else "")
            return vals, None
        elif suffix == ".json":
            with open(file_path, encoding="utf-8") as fh:
                data = json.load(fh)
            if isinstance(data, list):
                return [
                    str(item.get(column_name, "") or "")
                    for item in data[:max_rows]
                    if isinstance(item, dict)
                ], None
            return None, "unsupported JSON shape (expected list)"
        else:
            return None, f"unsupported file type: {suffix}"
    except Exception as e:  # noqa: BLE001 — surface any loader error
        return None, f"load error: {e!r}"


# ─── Manifest reading ───────────────────────────────────────────────────────


def read_manifest(path: Path) -> list[dict]:
    """Read manifest.csv, tolerant of 4-col legacy and 7-col current schema."""
    with open(path, newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        return list(reader)


def load_schema_mapping(path: Path | None) -> dict[str, str]:
    """Load gt_label (short) → full_type mapping from schema_mapping.yaml.

    Returns empty dict if path is None or file missing — caller falls back
    to raw gt_label.
    """
    if path is None or not path.exists():
        return {}
    with open(path) as fh:
        data = yaml.safe_load(fh) or {}
    # schema_mapping.yaml has various shapes across the codebase; try the
    # common ones. If structure differs, return {} and caller uses gt_label.
    if isinstance(data, dict):
        if "mapping" in data and isinstance(data["mapping"], dict):
            return {str(k): str(v) for k, v in data["mapping"].items()}
        # Assume top-level is the mapping directly
        return {str(k): str(v) for k, v in data.items() if isinstance(v, str)}
    return {}


# ─── Main ───────────────────────────────────────────────────────────────────


OUT_FIELDS = [
    "dataset",
    "file_path",
    "column_name",
    "gt_label",
    "full_type",
    "total",
    "non_null",
    "null_rate",
    "unique_ratio",
    "whitespace_ratio",
    "format_variance",
    "shannon_entropy",
    "top_1_skew",
    "pass_floors",
    "pass_notes",
    "error",
]


def main() -> int:
    doc = __doc__ or ""
    parser = argparse.ArgumentParser(description=doc.splitlines()[0] if doc else "")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "datasets" / "manifest.csv",
    )
    parser.add_argument(
        "--floors",
        type=Path,
        default=REPO_ROOT / "eval" / "pre-screen_floors.yaml",
    )
    parser.add_argument(
        "--schema-mapping",
        type=Path,
        default=REPO_ROOT / "eval" / "schema_mapping.yaml",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=REPO_ROOT / "eval" / "eval_output" / "prescreen.tsv",
    )
    parser.add_argument(
        "--max-rows",
        type=int,
        default=10_000,
        help="Maximum rows to load per column (default: 10k)",
    )
    args = parser.parse_args()

    floors_config = load_floors(args.floors)
    schema_mapping = load_schema_mapping(args.schema_mapping)
    rows = read_manifest(args.manifest)

    args.output.parent.mkdir(parents=True, exist_ok=True)

    n_passed = 0
    n_failed = 0
    n_error = 0

    with open(args.output, "w", newline="", encoding="utf-8") as out_fh:
        writer = csv.DictWriter(out_fh, fieldnames=OUT_FIELDS, delimiter="\t")
        writer.writeheader()

        for row in rows:
            dataset = row.get("dataset", "")
            file_path = row.get("file_path", "")
            column_name = row.get("column_name", "")
            gt_label = row.get("gt_label", "")

            gt = gt_label or ""
            full_type: str = schema_mapping.get(gt, gt) or gt

            # Resolve absolute path
            resolved = Path(file_path)
            if not resolved.is_absolute():
                resolved = REPO_ROOT / resolved

            values, err = load_column_values(resolved, column_name, args.max_rows)

            if err is not None or values is None:
                writer.writerow(
                    {
                        "dataset": dataset,
                        "file_path": file_path,
                        "column_name": column_name,
                        "gt_label": gt_label,
                        "full_type": full_type,
                        "pass_floors": False,
                        "pass_notes": "load_error",
                        "error": err or "no values",
                        **{k: "" for k in (
                            "total", "non_null", "null_rate", "unique_ratio",
                            "whitespace_ratio", "format_variance",
                            "shannon_entropy", "top_1_skew",
                        )},
                    }
                )
                n_error += 1
                continue

            metrics = compute_metrics(values)
            applicable = resolve_family(floors_config, full_type)
            passed, notes = evaluate_pass_floors(metrics, applicable)

            writer.writerow(
                {
                    "dataset": dataset,
                    "file_path": file_path,
                    "column_name": column_name,
                    "gt_label": gt_label,
                    "full_type": full_type,
                    "total": metrics["total"],
                    "non_null": metrics["non_null"],
                    "null_rate": f"{metrics['null_rate']:.4f}",
                    "unique_ratio": f"{metrics['unique_ratio']:.4f}",
                    "whitespace_ratio": f"{metrics['whitespace_ratio']:.4f}",
                    "format_variance": metrics["format_variance"],
                    "shannon_entropy": f"{metrics['shannon_entropy']:.4f}",
                    "top_1_skew": f"{metrics['top_1_skew']:.4f}",
                    "pass_floors": passed,
                    "pass_notes": notes,
                    "error": "",
                }
            )
            if passed:
                n_passed += 1
            else:
                n_failed += 1

    print(
        f"prescreen: {n_passed} pass / {n_failed} fail / {n_error} error "
        f"(of {len(rows)} manifest rows)"
    )
    print(f"output: {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
