#!/usr/bin/env python3
# Per spec 2026-05-23-ydf-specialist-geography ac-01, ac-02, ac-05.
"""Extract YDF-specialist training-data candidates from m-19's corpus pass.

Selects columns where YDF predicts a `<domain>.*` type with confidence
above a threshold AND Sense's prediction is either non-domain or a
domain-sibling. Filters out holdout overlap and rows where Sense's
non-domain prediction is plausibly header-driven (the circular-precision
guard — cue list grounded in `column.rs::header_hint()` via the audit
TSV at `eval/gittables/ydf_extract_cue_audit.tsv`).

Outputs three artefacts under `eval/gittables/v20_training_candidates/`:

  - `<domain>.tsv`              — one row per surviving column
  - `<domain>.leakage.json`     — row-hash overlap check (ac-02)
  - `<domain>.extract_stats.json` — exclusion counts (ac-01)

Per-column `row_hash` field: comma-separated list of per-value hex
hashes (one per value in sample_values), computed via the canonical
`scripts/eval_leakage/__init__.py::row_hash(header, value)`. The
leakage check (ac-02) interprets this as: any per-value hash overlap
with `eval/row_hashes.tsv` counts as leakage. This matches the
production filter `filter_eval_leakage()` in
`scripts/prepare_multibranch_data.py:843` (call sites at lines 3087
and 3088 — cycle-3 review finding M, pinned per drive note).

Domain-agnostic by construction (ac-05): all per-domain knobs
(prediction prefix) live in
`eval/gittables/ydf_extract_domain_configs.yaml`. Running with a new
`--domain` requires editing the YAML, not this script — `git diff
scripts/extract_ydf_specialist_training_data.py` between runs MUST be
empty.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

import pyarrow.parquet as pq
import yaml

REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO / "scripts"))
from eval_leakage import row_hash  # type: ignore  # noqa: E402

DEFAULT_COLUMNS_PARQUET = REPO / "eval" / "gittables" / "corpus_pass" / "columns.parquet"
DEFAULT_HOLDOUT = REPO / "eval" / "gittables" / "holdout_paths.txt"
DEFAULT_ROW_HASHES = REPO / "eval" / "row_hashes.tsv"
DEFAULT_CUE_AUDIT = REPO / "eval" / "gittables" / "ydf_extract_cue_audit.tsv"
DEFAULT_OUT_DIR = REPO / "eval" / "gittables" / "v20_training_candidates"
DEFAULT_DOMAIN_CONFIGS = REPO / "eval" / "gittables" / "ydf_extract_domain_configs.yaml"
COLUMN_RS = REPO / "crates" / "finetype-model" / "src" / "column.rs"

SAMPLE_DELIM = "│"  # U+2502, same as gittables_corpus_pass.py's column writer


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--domain", required=True, help="Domain key matching a block in domain_configs.yaml")
    p.add_argument("--min-confidence", type=float, default=0.7,
                   help="YDF confidence floor (default 0.7 — preliminary per spec ac-01)")
    p.add_argument("--columns", default=str(DEFAULT_COLUMNS_PARQUET))
    p.add_argument("--holdout", default=str(DEFAULT_HOLDOUT))
    p.add_argument("--row-hashes", default=str(DEFAULT_ROW_HASHES))
    p.add_argument("--cue-audit", default=str(DEFAULT_CUE_AUDIT))
    p.add_argument("--domain-configs", default=str(DEFAULT_DOMAIN_CONFIGS))
    p.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR))
    return p.parse_args()


def load_domain_config(path: Path, domain: str) -> dict:
    with path.open() as f:
        cfg = yaml.safe_load(f) or {}
    domains = cfg.get("domains") or {}
    if domain not in domains:
        sys.exit(f"error: domain '{domain}' not in {path} (known: {sorted(domains)})")
    block = domains[domain]
    prefix = block.get("domain_prefix")
    if not prefix or not prefix.endswith("."):
        sys.exit(f"error: domain '{domain}' missing or malformed 'domain_prefix' (must end with '.')")
    return block


def load_cue_audit(path: Path, expected_sha: str) -> list[tuple[str, str, str]]:
    """Load the cue audit TSV; assert its recorded sha matches current column.rs."""
    if not path.exists():
        sys.exit(f"error: cue audit TSV missing: {path}\n"
                 f"  Generate with: python3 scripts/audit_sense_header_cues.py")
    recorded_sha = None
    cues: list[tuple[str, str, str]] = []
    with path.open() as f:
        for ln in f:
            ln = ln.rstrip("\n")
            if ln.startswith("# source_sha256:"):
                recorded_sha = ln.split(":", 1)[1].strip()
                continue
            if ln.startswith("#") or ln.startswith("header_cue\t"):
                continue
            if not ln:
                continue
            parts = ln.split("\t")
            if len(parts) < 3:
                continue
            cue, kind, sense_type = parts[0], parts[1], parts[2]
            cues.append((cue, kind, sense_type))
    if recorded_sha is None:
        sys.exit(f"error: cue audit TSV missing source_sha256 header: {path}")
    if recorded_sha != expected_sha:
        sys.exit(
            f"error: cue audit is stale — column.rs has changed since audit\n"
            f"  recorded:  {recorded_sha}\n"
            f"  current:   {expected_sha}\n"
            f"  Regenerate with: python3 scripts/audit_sense_header_cues.py"
        )
    return cues


def resolve_gittables_root() -> Path:
    root = os.environ.get("GITTABLES_ROOT")
    if not root:
        # Look at the holdout file for a recorded root marker.
        root = _root_from_holdout_marker()
        if not root:
            sys.exit("error: GITTABLES_ROOT not set and no root marker in holdout_paths.txt")
    root_path = Path(root).resolve()
    if not root_path.is_dir():
        sys.exit(f"error: GITTABLES_ROOT does not exist: {root_path}")
    return root_path


def _root_from_holdout_marker() -> str | None:
    if not DEFAULT_HOLDOUT.exists():
        return None
    with DEFAULT_HOLDOUT.open() as f:
        for ln in f:
            if ln.startswith("# root:"):
                return ln.split(":", 1)[1].strip()
            if not ln.startswith("#"):
                return None
    return None


def load_holdout(path: Path) -> set[str]:
    out: set[str] = set()
    with path.open() as f:
        for ln in f:
            ln = ln.strip()
            if not ln or ln.startswith("#"):
                continue
            out.add(ln)
    return out


def load_eval_row_hashes(path: Path) -> set[str]:
    out: set[str] = set()
    with path.open() as f:
        header_idx: int | None = None
        for ln in f:
            ln = ln.rstrip("\n")
            if not ln or ln.startswith("#"):
                continue
            if header_idx is None:
                # First non-comment line is the header
                cols = ln.split("\t")
                if "row_hash" not in cols:
                    sys.exit(f"error: row_hashes file missing 'row_hash' column: {path}")
                header_idx = cols.index("row_hash")
                continue
            parts = ln.split("\t")
            if len(parts) > header_idx:
                out.add(parts[header_idx])
    return out


def normalise_header(h: str) -> str:
    """Match column.rs: lower, replace _/- with space, trim."""
    return (h or "").lower().replace("_", " ").replace("-", " ").strip()


def matches_cue(header_norm: str, cue: str, kind: str) -> bool:
    if kind == "exact":
        return header_norm == cue
    if kind == "substring":
        return cue in header_norm
    if kind == "ends_with":
        return header_norm.endswith(cue)
    if kind == "starts_with":
        return header_norm.startswith(cue)
    return False


def cue_guard_excludes(
    header: str,
    sense_pred: str,
    domain_prefix: str,
    non_domain_cues: list[tuple[str, str, str]],
) -> bool:
    """Drop row if header matches a non-domain cue AND sense is non-domain.

    Spec ac-01 exclusion rule (circular-precision guard).
    """
    if sense_pred.startswith(domain_prefix):
        return False
    norm = normalise_header(header)
    for cue, kind, _sense_type in non_domain_cues:
        if matches_cue(norm, cue, kind):
            return True
    return False


def relativise(file_path: str, gittables_root: Path) -> str:
    abs_path = Path(file_path).resolve()
    try:
        return str(abs_path.relative_to(gittables_root))
    except ValueError:
        sys.exit(
            f"error: file_path does not resolve under GITTABLES_ROOT={gittables_root}\n"
            f"       {abs_path}"
        )


def tsv_escape(s: str) -> str:
    return (s or "").replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n").replace("\r", "\\r")


def main() -> int:
    args = parse_args()
    domain = args.domain

    # Validate $GITTABLES_ROOT and load all inputs.
    gittables_root = resolve_gittables_root()

    domain_block = load_domain_config(Path(args.domain_configs), domain)
    domain_prefix: str = domain_block["domain_prefix"]

    column_rs_sha = hashlib.sha256(COLUMN_RS.read_bytes()).hexdigest()
    all_cues = load_cue_audit(Path(args.cue_audit), expected_sha=column_rs_sha)
    non_domain_cues = [c for c in all_cues if not c[2].startswith(domain_prefix)]

    holdout = load_holdout(Path(args.holdout))
    eval_hashes = load_eval_row_hashes(Path(args.row_hashes))

    print(f"domain: {domain} (prefix={domain_prefix!r})", file=sys.stderr)
    print(f"cue audit: {len(all_cues)} cues; {len(non_domain_cues)} non-{domain.rstrip('.')} cues used by guard", file=sys.stderr)
    print(f"holdout: {len(holdout)} paths", file=sys.stderr)
    print(f"eval row hashes: {len(eval_hashes)}", file=sys.stderr)
    print(f"GITTABLES_ROOT: {gittables_root}", file=sys.stderr)

    # Load columns.parquet (single-pass).
    print(f"loading {args.columns} ...", file=sys.stderr)
    table = pq.read_table(args.columns)
    n_total = table.num_rows
    print(f"  {n_total:,} rows", file=sys.stderr)

    # Filter columns we need.
    cols = {
        name: table.column(name).to_pylist()
        for name in (
            "file_path", "column_name", "sense_prediction", "ydf_prediction",
            "ydf_confidence", "sample_values_truncated", "is_trivial",
        )
    }

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    tsv_path = out_dir / f"{domain}.tsv"
    stats_path = out_dir / f"{domain}.extract_stats.json"
    leakage_path = out_dir / f"{domain}.leakage.json"

    stats = {
        "domain": domain,
        "domain_prefix": domain_prefix,
        "min_confidence": args.min_confidence,
        "input_rows": n_total,
        "excluded_trivial": 0,
        "excluded_ydf_below_threshold": 0,
        "excluded_ydf_not_in_domain": 0,
        "excluded_sense_same_as_ydf": 0,
        "excluded_holdout": 0,
        "excluded_cue_guard": 0,
        "emitted_rows": 0,
        "column_rs_sha256": column_rs_sha,
        "cue_audit_sha256": _file_sha(args.cue_audit),
        "row_hashes_sha256": _file_sha(args.row_hashes),
    }

    dropped_due_to_overlap = 0
    emitted = 0
    rows_for_tsv: list[list[str]] = []

    for i in range(n_total):
        if cols["is_trivial"][i]:
            stats["excluded_trivial"] += 1
            continue
        ydf = cols["ydf_prediction"][i] or ""
        if not ydf.startswith(domain_prefix):
            stats["excluded_ydf_not_in_domain"] += 1
            continue
        conf = cols["ydf_confidence"][i]
        if conf is None or conf < args.min_confidence:
            stats["excluded_ydf_below_threshold"] += 1
            continue
        sense = cols["sense_prediction"][i] or ""
        if sense == ydf:
            stats["excluded_sense_same_as_ydf"] += 1
            continue
        file_path = cols["file_path"][i] or ""
        if file_path in holdout:
            stats["excluded_holdout"] += 1
            continue
        header = cols["column_name"][i] or ""
        if cue_guard_excludes(header, sense, domain_prefix, non_domain_cues):
            stats["excluded_cue_guard"] += 1
            continue

        sample_values_raw = cols["sample_values_truncated"][i] or ""
        values = [v for v in sample_values_raw.split(SAMPLE_DELIM) if v]
        per_value_hashes = [row_hash(header, v) for v in values]
        # Per-value leakage firewall (ac-02): drop columns where any value
        # collides with eval/row_hashes.tsv. The firewall is preventive —
        # the dropped count is reported for transparency; the emitted TSV
        # has zero overlap by construction.
        if any(h in eval_hashes for h in per_value_hashes):
            dropped_due_to_overlap += 1
            continue

        rel_path = relativise(file_path, gittables_root)
        rows_for_tsv.append([
            rel_path,
            header,
            sample_values_raw,
            ydf,
            f"{conf:.6f}",
            sense,
            ydf,  # candidate_target_label = YDF's high-confidence call
            ",".join(per_value_hashes),
        ])
        emitted += 1

    stats["emitted_rows"] = emitted
    stats["dropped_due_to_overlap"] = dropped_due_to_overlap

    # ac-02: leakage report. `overlap_count` is the count IN THE EMITTED
    # TSV (zero by construction — the firewall drops upstream).
    # `dropped_due_to_overlap` records what the firewall caught.
    leakage_report = {
        "overlap_count": 0,
        "dropped_due_to_overlap": dropped_due_to_overlap,
        "eval_hashes_checked": len(eval_hashes),
        f"{domain}_rows_emitted": emitted,
        "domain": domain,
    }
    with leakage_path.open("w") as f:
        json.dump(leakage_report, f, indent=2)
        f.write("\n")
    print(
        f"leakage: overlap_count=0 (after firewall); dropped_due_to_overlap={dropped_due_to_overlap}; "
        f"eval_hashes_checked={len(eval_hashes)}",
        file=sys.stderr,
    )

    # Write TSV.
    header_cols = [
        "file_path", "column_name", "sample_values", "ydf_prediction",
        "ydf_confidence", "sense_prediction", "candidate_target_label", "row_hash",
    ]
    with tsv_path.open("w") as f:
        f.write("\t".join(header_cols) + "\n")
        for row in rows_for_tsv:
            f.write("\t".join(tsv_escape(c) for c in row) + "\n")
    print(f"wrote {emitted:,} rows to {tsv_path}", file=sys.stderr)

    with stats_path.open("w") as f:
        json.dump(stats, f, indent=2)
        f.write("\n")
    print(f"wrote stats to {stats_path}", file=sys.stderr)
    print(
        f"summary: trivial={stats['excluded_trivial']:,} "
        f"not_in_domain={stats['excluded_ydf_not_in_domain']:,} "
        f"below_conf={stats['excluded_ydf_below_threshold']:,} "
        f"sense_eq_ydf={stats['excluded_sense_same_as_ydf']:,} "
        f"holdout={stats['excluded_holdout']:,} "
        f"cue_guard={stats['excluded_cue_guard']:,} "
        f"leakage_firewall={dropped_due_to_overlap:,} "
        f"-> emitted={emitted:,}",
        file=sys.stderr,
    )
    return 0


def _file_sha(path: str) -> str:
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


if __name__ == "__main__":
    sys.exit(main())
