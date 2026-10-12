#!/usr/bin/env python3
"""Write each date-pattern locale's default numbering system from CLDR.

A CLDR date pattern says where the day, month and year go, not which digits
write them: ar-SA's d/M/y is written ١٥/١/٢٠٢٤, because its default numbering
system is `arab`, not `latn`. This table says which locales write ASCII digits
by default, so a reader of cldr_date_patterns.tsv can tell.

Reads data/cldr/json/cldr-numbers-full (fetched by scripts/download_cldr.sh)
and writes data/cldr/cldr_numbering_systems.tsv: one row per locale in
cldr_date_patterns.tsv, sorted by locale. Standard library only.

Usage: ./scripts/download_cldr.sh && python3 scripts/extract_cldr_numbering.py
"""

import csv
import json
import sys
from pathlib import Path

CLDR = Path(__file__).resolve().parent.parent / "data" / "cldr"
NUMBERS = CLDR / "json" / "cldr-numbers-full" / "main"
OUT = CLDR / "cldr_numbering_systems.tsv"


def main() -> int:
    if not NUMBERS.is_dir():
        print(f"{NUMBERS} is missing: run scripts/download_cldr.sh first", file=sys.stderr)
        return 1
    with (CLDR / "cldr_date_patterns.tsv").open(newline="") as f:
        locales = sorted({r["locale"] for r in csv.DictReader(f, delimiter="\t")})
    rows = []
    for locale in locales:
        path = NUMBERS / locale / "numbers.json"
        if not path.exists():
            print(f"{locale}: no numbers.json in cldr-numbers-full", file=sys.stderr)
            return 1
        numbers = json.loads(path.read_text())["main"][locale]["numbers"]
        rows.append((locale, numbers["defaultNumberingSystem"]))
    with OUT.open("w", newline="") as f:
        w = csv.writer(f, delimiter="\t", lineterminator="\n")
        w.writerow(["locale", "default_numbering_system"])
        w.writerows(rows)
    other = sum(1 for _, ns in rows if ns != "latn")
    print(f"wrote {len(rows)} locales to {OUT.relative_to(CLDR.parent.parent)}; {other} default to digits other than latn")
    return 0


if __name__ == "__main__":
    sys.exit(main())
