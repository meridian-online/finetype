#!/usr/bin/env python3
"""Fetch the place-name gazetteer backing the `region_nonmembership_veto` guard.

The guard demotes a `geography.location.region` overcall to text when the
values are NOT real place names (usgs `net`/`type`, gleif `category`, seattle
`checkouttype` — short catalog codes the model reaches `region` for). To do that
safely it must recognise a REAL region column (US states, world provinces,
country names) and leave it alone. This gazetteer is that recognition set.

Source: GeoNames (https://www.geonames.org/, CC-BY 4.0):
  - admin1CodesASCII.txt  — first-level admin divisions (states/provinces/regions)
                            worldwide; native + ASCII name.
  - countryInfo.txt       — country names.
  - cities15000.txt       — cities with population >= 15,000 (~34k); native + ASCII
                            name. Without cities the veto wrongly demotes real
                            city/county/`City, State` columns (measured 15% FP on
                            an admin1-only gazetteer) — the guard also composite-
                            matches `,`/`(` parts so `Durham County, NC` clears.

Names only, NOT codes: bare 2-letter codes (state/country) collide with the
catalog codes the veto must catch (seismic `net` = ak/tx/nc are also US state
codes), so the guard keys on NAMES here plus the separate ISO-3166-2 hyphenated
code roster — never bare codes.

Like scripts/fetch_us_tickers.py this is a scheduled CI/CD refresh into a
checked-in set; the build stays hermetic (include_str! from the committed file).

Usage:
  python3 scripts/fetch_place_names.py                 # fetch + write the set
  python3 scripts/fetch_place_names.py --admin1 A --countries C   # from cached files
  python3 scripts/fetch_place_names.py --check         # verify the committed set parses
"""
import argparse
import io
import subprocess
import sys
import zipfile
from pathlib import Path

ADMIN1_URL = "https://download.geonames.org/export/dump/admin1CodesASCII.txt"
COUNTRY_URL = "https://download.geonames.org/export/dump/countryInfo.txt"
CITIES_URL = "https://download.geonames.org/export/dump/cities15000.zip"
OUT = Path(__file__).resolve().parent.parent / "labels" / "sets" / "place_names.txt"

HEADER = """\
# Place-name gazetteer -- region/state/province names + country names, for the
# `region_nonmembership_veto` model guard. The guard demotes a
# geography.location.region OVERCALL to text when <50% of a column's distinct
# values are real places (a place-NAME here OR an ISO-3166-2 hyphenated code) and
# the header is not a strong region header -- so it needs to recognise a genuine
# region column (US states, world provinces, countries) and leave it alone.
# Source: GeoNames (https://www.geonames.org/, CC-BY 4.0) admin1CodesASCII.txt +
# countryInfo.txt + cities15000.txt (pop >= 15,000). NAMES ONLY, never bare
# 2-letter codes -- those collide with the catalog codes the veto must catch
# (seismic `net` ak/tx/nc are US state codes); the guard pairs this with the
# separate ISO-3166-2 code roster and a bare state-code check. Cities are included
# because an admin1-only set wrongly demotes real city/county/`City, State`
# columns (measured 15% FP); the guard also composite-matches `,`/`(` parts.
# Extraction: admin1 native + ASCII names, country names, city native + ASCII
# names; lower-cased, >=2 chars, sorted, de-duplicated. Refreshed by
# scripts/fetch_place_names.py (scheduled CI/CD download; build hermetic via
# include_str!).
"""


def curl(url: str) -> bytes:
    # GeoNames serves plain files; curl keeps this consistent with fetch_us_tickers.py.
    r = subprocess.run(
        ["curl", "-sS", "--max-time", "120", url],
        capture_output=True,
    )
    if r.returncode != 0:
        sys.exit(f"fetch failed for {url}: {r.stderr.decode(errors='replace').strip()}")
    return r.stdout


def cities_text(zip_bytes: bytes) -> str:
    with zipfile.ZipFile(io.BytesIO(zip_bytes)) as z:
        return z.read("cities15000.txt").decode("utf-8", errors="replace")


def extract(admin1_text: str, country_text: str, cities_txt: str) -> list[str]:
    names: set[str] = set()

    def add(nm: str) -> None:
        nm = nm.strip().lower()
        if len(nm) >= 2:
            names.add(nm)

    for line in admin1_text.splitlines():
        parts = line.split("\t")
        if len(parts) >= 3:
            add(parts[1])  # native
            add(parts[2])  # ASCII
    for line in country_text.splitlines():
        if line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) >= 5:
            add(parts[4])  # Country name column
    for line in cities_txt.splitlines():
        parts = line.split("\t")
        if len(parts) >= 3:
            add(parts[1])  # name
            add(parts[2])  # asciiname
    return sorted(names)


def write_set(names: list[str]) -> None:
    with open(OUT, "w") as f:
        f.write(HEADER)
        for n in names:
            f.write(n + "\n")
    print(f"wrote {len(names)} place names -> {OUT}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--admin1", help="path to a cached admin1CodesASCII.txt")
    ap.add_argument("--countries", help="path to a cached countryInfo.txt")
    ap.add_argument("--cities", help="path to a cached cities15000.txt")
    ap.add_argument("--check", action="store_true", help="verify the committed set parses")
    args = ap.parse_args()

    if args.check:
        n = sum(1 for ln in open(OUT) if ln.strip() and not ln.startswith("#"))
        print(f"{OUT}: {n} place names")
        sys.exit(0 if n > 30000 else f"suspiciously small set: {n}")

    admin1_text = open(args.admin1).read() if args.admin1 else curl(ADMIN1_URL).decode(
        "utf-8", errors="replace"
    )
    country_text = open(args.countries).read() if args.countries else curl(COUNTRY_URL).decode(
        "utf-8", errors="replace"
    )
    cities_txt = open(args.cities).read() if args.cities else cities_text(curl(CITIES_URL))
    names = extract(admin1_text, country_text, cities_txt)
    if len(names) < 30000:
        sys.exit(f"refusing to write a suspiciously small set ({len(names)})")
    write_set(names)


if __name__ == "__main__":
    main()
