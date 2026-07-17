#!/usr/bin/env python3
"""Fetch the US-listed symbol sources and regenerate the us_tickers
membership set (labels/sets/us_tickers.txt).

The set is the UNION of two public sources:

1. SEC https://www.sec.gov/files/company_tickers.json — the canonical
   ticker→CIK map behind EDGAR (~10.4k entries, public domain, no auth
   beyond a descriptive User-Agent). Class shares / preferred use the SEC
   dash form (BRK-B, ABR-PD) — the form EDGAR's own ticker column carries.
2. Nasdaq Trader SymbolDirectory (public HTTP, no auth):
   nasdaqlisted.txt (`Symbol` column) + otherlisted.txt (`ACT Symbol`
   column). Covers what the SEC company map omits: ETFs, warrants (W),
   units (U), rights (R), preferred classes. ACT conventions keep their
   native punctuation (ABR$D, AAC.U). Test issues (`Test Issue` = Y) are
   dropped; the trailing `File Creation Time` footer row is dropped.

Honest limit: OTC/ADR `-F` forms and delisted (Q) symbols have no free
authoritative bulk list and are deliberately NOT chased — the
protein_sequence_length_veto guard covers that residual tail.

This is the CI/CD refresh entry point: run on a schedule, and open a PR when
the checked-in set changes. The build itself never touches the network — the
set is embedded from the committed .txt via include_str! (membership.rs).

Usage:
    python3 scripts/fetch_us_tickers.py            # fetch + rewrite the set
    python3 scripts/fetch_us_tickers.py --check     # exit 1 if the set is stale
"""
import json
import subprocess
import sys
from datetime import date
from pathlib import Path

SEC_URL = "https://www.sec.gov/files/company_tickers.json"
NASDAQ_URLS = [
    "https://www.nasdaqtrader.com/dynamic/SymDir/nasdaqlisted.txt",
    "https://www.nasdaqtrader.com/dynamic/SymDir/otherlisted.txt",
]
# SEC fair-access requires a descriptive User-Agent that includes a contact
# email, else 403. Two further WAF quirks learned the hard way: (1) it
# fingerprints and rejects urllib regardless of headers, so we shell out to
# curl (present on every CI runner); (2) it rejects multi-label email domains
# (a github noreply subdomain 403s) — a simple 2-label domain works, so we use
# the RFC-2606 reserved placeholder, the standard convention for SEC scripts.
UA = "FineType typeinference contact@example.com"
OUT = Path(__file__).resolve().parent.parent / "labels" / "sets" / "us_tickers.txt"


def curl(url: str) -> str:
    proc = subprocess.run(
        ["curl", "-sS", "--fail", "-A", UA, url],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"curl failed ({proc.returncode}) for {url}: {proc.stderr.strip()}")
    return proc.stdout


def fetch_sec_tickers(from_file: str | None = None) -> set[str]:
    if from_file:
        data = json.loads(Path(from_file).read_text())
    else:
        data = json.loads(curl(SEC_URL))
    # {"0": {"cik_str": ..., "ticker": "NVDA", "title": ...}, ...}
    return {
        row["ticker"].strip().upper()
        for row in data.values()
        if row.get("ticker", "").strip()
    }


def parse_symbol_directory(text: str) -> set[str]:
    """Parse a Nasdaq Trader SymbolDirectory pipe-delimited file.

    First row is the header (symbol column = `Symbol` or `ACT Symbol`); the
    trailing `File Creation Time: …` row is a footer, not a symbol. Rows with
    `Test Issue` = Y are exchange test symbols (ZAZZT/ZXIET…), not listings.
    """
    lines = [ln for ln in text.splitlines() if ln.strip()]
    header = [col.strip() for col in lines[0].split("|")]
    sym_idx = header.index("Symbol") if "Symbol" in header else header.index("ACT Symbol")
    test_idx = header.index("Test Issue") if "Test Issue" in header else None
    out: set[str] = set()
    for ln in lines[1:]:
        if ln.startswith("File Creation Time"):
            continue
        cols = ln.split("|")
        if len(cols) <= sym_idx:
            continue
        if test_idx is not None and len(cols) > test_idx and cols[test_idx].strip() == "Y":
            continue
        sym = cols[sym_idx].strip().upper()
        if sym:
            out.add(sym)
    return out


def fetch_nasdaq_tickers(from_files: list[str] | None = None) -> set[str]:
    texts = (
        [Path(p).read_text() for p in from_files]
        if from_files
        else [curl(url) for url in NASDAQ_URLS]
    )
    out: set[str] = set()
    for text in texts:
        out |= parse_symbol_directory(text)
    return out


def fetch_tickers(
    from_file: str | None = None, nasdaq_files: list[str] | None = None
) -> list[str]:
    return sorted(fetch_sec_tickers(from_file) | fetch_nasdaq_tickers(nasdaq_files))


def render(tickers: list[str]) -> str:
    header = (
        "# US-listed stock tickers — closed membership set for the\n"
        "# `membership: us_tickers` taxonomy directive on finance.securities.ticker.\n"
        "# Sources (UNION): SEC company_tickers.json (https://www.sec.gov/files/\n"
        "# company_tickers.json, public domain), the canonical ticker->CIK map behind\n"
        "# EDGAR; plus Nasdaq Trader SymbolDirectory nasdaqlisted.txt + otherlisted.txt\n"
        "# (Symbol / ACT Symbol columns, test issues + footer dropped) for the ETF /\n"
        "# warrant / unit / right / preferred coverage the SEC company map omits.\n"
        "# Extraction: DISTINCT upper-cased symbols, sorted. A ticker has no checksum\n"
        "# and its shape (^[A-Z]{1,7}$) confirms every short uppercase token, so list\n"
        "# membership is the substance (Precision Principle). Class shares keep each\n"
        "# source's native form (SEC dash BRK-B; ACT ABR$D / AAC.U); the >=90% column\n"
        "# guard tolerates residual-minority forms. OTC/ADR -F and delisted symbols\n"
        "# have no free authoritative bulk list and are deliberately not chased.\n"
        f"# Regenerate: scripts/fetch_us_tickers.py. Snapshot {date.today().isoformat()}, "
        f"{len(tickers)} tickers.\n"
    )
    return header + "\n".join(tickers) + "\n"


def main() -> int:
    check = "--check" in sys.argv
    # --from-file PATH: transform an already-downloaded company_tickers.json
    # instead of fetching (offline/CI-cache use, and to spare SEC's rate limit).
    from_file = None
    if "--from-file" in sys.argv:
        from_file = sys.argv[sys.argv.index("--from-file") + 1]
    # --from-nasdaq-files A B: already-downloaded nasdaqlisted.txt otherlisted.txt.
    nasdaq_files = None
    if "--from-nasdaq-files" in sys.argv:
        i = sys.argv.index("--from-nasdaq-files")
        nasdaq_files = sys.argv[i + 1 : i + 3]
    tickers = fetch_tickers(from_file, nasdaq_files)
    rendered = render(tickers)
    if check:
        current = OUT.read_text() if OUT.exists() else ""
        # Compare on the code lines only — the snapshot date in the header is
        # expected to differ and must not by itself flag the set as stale.
        def codes(text: str) -> list[str]:
            return [ln for ln in text.splitlines() if ln and not ln.startswith("#")]

        if codes(current) == codes(rendered):
            print(f"us_tickers.txt is up to date ({len(tickers)} tickers)")
            return 0
        print("us_tickers.txt is STALE — run scripts/fetch_us_tickers.py", file=sys.stderr)
        return 1
    OUT.write_text(rendered)
    print(f"wrote {OUT.relative_to(OUT.parents[2])} ({len(tickers)} tickers)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
