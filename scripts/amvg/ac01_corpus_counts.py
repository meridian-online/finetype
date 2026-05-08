#!/usr/bin/env python3
"""ac-01: Corpus-count-per-subtype table.

Tallies rows per label in the v16 blend FTMB and writes:
  diagnostics/corpus_counts.tsv    — 12 rows (11 amount subtypes + plain amount)
  diagnostics/v16_corpus_hash.txt  — SHA256 hex of the FTMB

Implementation note: FTMB v3/v4 is a group-keyed format — reimplementing the
parser here would duplicate logic from scripts/read_ftmb.py and risk silent
drift. Instead we shell out to read_ftmb.py --stats (which prints a label-count
histogram) and regex-parse its output. This keeps scripts/read_ftmb.py as the
single source of truth for the FTMB format.

Constraints (spec v1.2 ac-01):
  - Corpus pinned at output/multibranch-training/v16-blend-70-30.ftmb
  - Hash file contains sha256 hex (64 chars) of that FTMB
  - corpus_counts.tsv has 12 rows with exactly the expected labels
"""
import hashlib
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC_DIR = REPO_ROOT / ".orbit/specs/2026-04-24-amount-variant-generators"
DIAG_DIR = SPEC_DIR / "diagnostics"
FTMB = REPO_ROOT / "output/multibranch-training/v16-blend-70-30.ftmb"
READ_FTMB = REPO_ROOT / "scripts/read_ftmb.py"

TARGET_SUBTYPES = [
    "finance.currency.amount",  # control
    "finance.currency.amount_accounting",
    "finance.currency.amount_apostrophe",
    "finance.currency.amount_code_prefix",
    "finance.currency.amount_comma",
    "finance.currency.amount_comma_suffix",
    "finance.currency.amount_crypto",
    "finance.currency.amount_lakh",
    "finance.currency.amount_multisym",
    "finance.currency.amount_neg_trailing",
    "finance.currency.amount_nodecimal",
    "finance.currency.amount_space",
]

# Matches lines like "  finance.currency.amount_code_prefix: 308"
LABEL_LINE = re.compile(r"^\s+([a-z_]+\.[a-z_0-9]+\.[a-z_0-9]+):\s+(\d+)\s*$")


def sha256_of(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(8 * 1024 * 1024):
            h.update(chunk)
    return h.hexdigest()


def label_counts_via_stats(ftmb: Path) -> dict[str, int]:
    """Invoke read_ftmb.py --stats and parse its label histogram."""
    cmd = [sys.executable, str(READ_FTMB), str(ftmb), "--stats"]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True)
    counts: dict[str, int] = {}
    for line in out.stdout.splitlines():
        m = LABEL_LINE.match(line)
        if m:
            counts[m.group(1)] = int(m.group(2))
    if not counts:
        sys.exit("read_ftmb.py --stats produced no label lines")
    return counts


def main() -> None:
    if not FTMB.exists():
        sys.exit(f"FIXTURE MISSING: {FTMB}")
    if not READ_FTMB.exists():
        sys.exit(f"TOOL MISSING: {READ_FTMB}")

    digest = sha256_of(FTMB)
    counts = label_counts_via_stats(FTMB)

    DIAG_DIR.mkdir(parents=True, exist_ok=True)
    (DIAG_DIR / "v16_corpus_hash.txt").write_text(digest + "\n")

    lines = ["subtype\trow_count\n"]
    for sub in TARGET_SUBTYPES:
        lines.append(f"{sub}\t{counts.get(sub, 0)}\n")
    (DIAG_DIR / "corpus_counts.tsv").write_text("".join(lines))

    target_total = sum(counts.get(s, 0) for s in TARGET_SUBTYPES)
    grand_total = sum(counts.values())
    print(f"hash: {digest}")
    print(f"grand_total_records: {grand_total}")
    print(f"target_subtype_total: {target_total}")
    for sub in TARGET_SUBTYPES:
        print(f"  {sub}: {counts.get(sub, 0)}")


if __name__ == "__main__":
    main()
