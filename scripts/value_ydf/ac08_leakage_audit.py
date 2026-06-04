#!/usr/bin/env python3
"""ac-08: leakage audit — zero overlap between cleaned set and eval holdout.

The load-bearing firewall is row_hash(header, value) against eval/row_hashes.tsv
(the shared normaliser, choice 0056). The cleaned NDJSON is headerless, so we
audit the kept rows WITH provenance (kept_rows.parquet from ac-04), recovering
each value's source column header. Any collision voids the run.

Secondary structural check: the cleaned set is drawn from the corpus_pass
"measure" partition (file_content_sha256 % 2 == 1); we confirm every kept file
is on that side.

Run from the eval venv:
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python \
    scripts/value_ydf/ac08_leakage_audit.py
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pandas as pd

import common as C

sys.path.insert(0, str(C.REPO / "scripts"))
from eval_leakage import row_hash  # noqa: E402

ROW_HASHES = C.REPO / "eval" / "row_hashes.tsv"
FILES_PARQUET = C.REPO / "eval" / "gittables" / "corpus_pass" / "files.parquet"


def load_eval_hashes(path: Path) -> set[str]:
    hashes: set[str] = set()
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.rstrip("\n")
            if not line or line.startswith("#") or line.startswith("dataset\t"):
                continue
            parts = line.split("\t")
            if len(parts) >= 4:
                hashes.add(parts[3])
    return hashes


def main() -> int:
    eval_hashes = load_eval_hashes(ROW_HASHES)
    kept = pd.read_parquet(C.OUT_DIR / "kept_rows.parquet")
    n = len(kept)

    collisions = []
    headers = kept["column_name"].fillna("").to_numpy()
    values = kept["value"].astype(str).to_numpy()
    for i in range(n):
        if row_hash(headers[i], values[i]) in eval_hashes:
            collisions.append((kept["file_path"].iloc[i], headers[i], values[i][:60]))
            if len(collisions) >= 20:
                break

    # Structural partition check.
    files = pd.read_parquet(FILES_PARQUET, columns=["file_path", "file_content_sha256"])
    sha = dict(zip(files["file_path"], files["file_content_sha256"]))
    part = kept["file_path"].map(lambda p: int(sha[p], 16) % 2 if p in sha else -1)
    part_counts = part.value_counts().to_dict()
    off_partition = int((part == 0).sum())  # 0 == calibrate (eval) side

    clean = len(collisions) == 0 and off_partition == 0
    report = {
        "kept_rows": n,
        "eval_hashes_loaded": len(eval_hashes),
        "row_hash_collisions": len(collisions),
        "partition_counts": {str(k): int(v) for k, v in part_counts.items()},
        "off_partition_rows": off_partition,
        "verdict": "CLEAN" if clean else "VOID",
    }
    (C.OUT_DIR / "ac08_leakage_report.json").write_text(json.dumps(report, indent=2))

    lines = [
        "# ac-08 — leakage audit",
        "",
        f"- kept rows audited: **{n:,}**",
        f"- eval row_hashes loaded: **{len(eval_hashes):,}**",
        f"- row_hash(header,value) collisions with eval holdout: **{len(collisions)}**",
        f"- kept rows on the eval (calibrate, SHA%2==0) partition: **{off_partition}**",
        "",
        f"**Verdict:** {'CLEAN — zero overlap; the run stands.' if clean else 'VOID — overlap detected; the run is void (halt condition).'}",
    ]
    if collisions:
        lines += ["", "## Sample collisions (first 20)", ""]
        for fp, h, v in collisions:
            lines.append(f"- `{h}` = `{v}` ({fp})")
    (C.OUT_DIR / "ac08_leakage_report.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    return 0 if clean else 1


if __name__ == "__main__":
    raise SystemExit(main())
