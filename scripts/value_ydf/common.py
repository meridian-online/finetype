"""Shared helpers for the value-level YDF labelling pipeline
(spec 2026-06-04-value-level-ydf-labelling).

One contract, two paths: the synthetic-prior path (ac-01) and the
gittables-scoring path (ac-02) both export features through the ac-00 Rust bin
and read them back here, so the 277-dim feature order never drifts.
"""
from __future__ import annotations

import csv
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

from eval_leakage import normalise_value  # noqa: E402

EVAL_MANIFEST = REPO / "eval" / "datasets" / "manifest.csv"
EXTRACT_BIN = REPO / "target" / "release" / "extract_value_features"
FINETYPE_BIN = REPO / "target" / "release" / "finetype"
MODELS_DIR = REPO / "eval" / "gittables" / "models"
OUT_DIR = REPO / "output" / "value-level-labelling"

# Columns the exporter prepends before the 277 feature columns.
META_COLS = ["classification", "text"]


def eval_value_set(manifest: Path = EVAL_MANIFEST) -> set[str]:
    """Normalised values present in the eval holdout (column-level manifest).

    A header-agnostic firewall for the *headerless* synthetic prior: matching on
    value alone cannot under-exclude relative to the (header, value) row_hash, so
    it is the conservative choice. The header-bearing row_hash firewall against
    eval/row_hashes.tsv does the load-bearing work on the gittables side (ac-02,
    ac-08), where real headers exist.
    """
    import compute_row_hashes as crh  # __main__-guarded, safe to import

    vals: set[str] = set()
    with open(manifest, newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            fp = REPO / row["file_path"]
            if not fp.exists():
                continue
            for v in crh.load_column_values(fp, row["column_name"]):
                nv = normalise_value(v)
                if nv:
                    vals.add(nv)
    return vals


def export_features(ndjson: Path, out_csv: Path, manifest_out: Path | None = None) -> None:
    """Run the ac-00 per-value feature exporter (the shared contract)."""
    if not EXTRACT_BIN.exists():
        raise SystemExit(
            f"missing {EXTRACT_BIN} — run: cargo build --release -p finetype-cli "
            "--bin extract_value_features"
        )
    cmd = [str(EXTRACT_BIN), "--input", str(ndjson), "--output", str(out_csv)]
    if manifest_out:
        cmd += ["--emit-manifest", str(manifest_out)]
    subprocess.run(cmd, check=True, cwd=str(REPO))


def feature_columns(columns) -> list[str]:
    """The 277 feature column names (everything after the meta columns)."""
    return [c for c in columns if c not in META_COLS]
