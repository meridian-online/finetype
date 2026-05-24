#!/bin/bash
# Validates that dataset_verify.py catches a deliberate one-byte corruption.
# Per spec 2026-05-24-dataset-provenance-registry ac-03 close evidence.
set -e
cd /Users/hugh/github/meridian-online/finetype

TMPDATA=$(mktemp -d)
TMPSNAP=$(mktemp)
trap "rm -rf $TMPDATA $TMPSNAP" EXIT

# Snapshot a tiny temporary "dataset" — write snap JSON OUTSIDE the data dir
echo "hello world" > $TMPDATA/file_a.txt
echo "second file" > $TMPDATA/file_b.txt
python3 scripts/dataset_register.py drift_test "$TMPDATA" \
  --role train --licence test --attribution "verify-drift test" \
  --no-update-registry --dry-run > $TMPSNAP

# Manually wire it into a temp sources.yaml entry
python3 - <<PY
import json, yaml
from pathlib import Path
snap_path = Path("eval/datasets/snapshots/drift_test-test.json")
snap_path.parent.mkdir(parents=True, exist_ok=True)
with open("$TMPSNAP") as f:
    snap = json.load(f)
snap["dataset_version"] = "test"
with snap_path.open("w") as f:
    json.dump(snap, f, indent=2)

src = Path("eval/datasets/sources.yaml")
with src.open() as f:
    data = yaml.safe_load(f)
data["sources"].append({
    "source_url": "dataset://drift_test",
    "role": "train", "licence": "test", "fetched_date": "2026-05-24",
    "attribution": "test", "datasets": ["drift_test"],
    "local_path": "$TMPDATA",
    "snapshot": str(snap_path),
    "dataset_version": "test",
})
with src.open("w") as f:
    yaml.safe_dump(data, f, sort_keys=False)
PY

echo "── verify clean (expect OK, exit 0):"
python3 scripts/dataset_verify.py drift_test && echo "exit=0 ✓"

echo
echo "── corrupting file_a.txt (one extra byte):"
echo "X" >> $TMPDATA/file_a.txt

echo "── verify corrupted (expect DRIFT, exit 2):"
set +e
python3 scripts/dataset_verify.py drift_test
rc=$?
set -e
echo "exit=$rc"
[ "$rc" = "2" ] || { echo "FAIL: expected exit 2, got $rc"; exit 1; }

# Cleanup
python3 - <<PY
import yaml
from pathlib import Path
src = Path("eval/datasets/sources.yaml")
with src.open() as f:
    data = yaml.safe_load(f)
data["sources"] = [e for e in data["sources"] if e.get("source_url") != "dataset://drift_test"]
with src.open("w") as f:
    f.write(
        "# eval/datasets/sources.yaml — role manifest + provenance registry\n"
        "# (auto-managed in part by scripts/dataset_register.py — comments\n"
        "# above individual entries may be lost on rewrite; see git history\n"
        "# for prior annotations or restore via ruamel.yaml round-trip.)\n\n"
    )
    yaml.safe_dump(data, f, sort_keys=False)
rm = Path("eval/datasets/snapshots/drift_test-test.json")
if rm.exists():
    rm.unlink()
PY

echo
echo "PASS — verify caught the corruption and exited 2."
