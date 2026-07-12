#!/usr/bin/env python3
"""Fetch the SEC's official ticker→CIK map and regenerate the us_tickers
membership set (labels/sets/us_tickers.txt).

Source: https://www.sec.gov/files/company_tickers.json — the SEC's canonical
list of US-listed company symbols (the same table behind EDGAR), ~9.3k
entries, public domain, no auth beyond a descriptive User-Agent.

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

URL = "https://www.sec.gov/files/company_tickers.json"
# SEC fair-access requires a descriptive User-Agent that includes a contact
# email, else 403. Two further WAF quirks learned the hard way: (1) it
# fingerprints and rejects urllib regardless of headers, so we shell out to
# curl (present on every CI runner); (2) it rejects multi-label email domains
# (a github noreply subdomain 403s) — a simple 2-label domain works, so we use
# the RFC-2606 reserved placeholder, the standard convention for SEC scripts.
UA = "FineType typeinference contact@example.com"
OUT = Path(__file__).resolve().parent.parent / "labels" / "sets" / "us_tickers.txt"


def fetch_tickers(from_file: str | None = None) -> list[str]:
    if from_file:
        data = json.loads(Path(from_file).read_text())
    else:
        proc = subprocess.run(
            ["curl", "-sS", "--fail", "-A", UA, URL],
            capture_output=True,
            text=True,
            timeout=60,
        )
        if proc.returncode != 0:
            raise RuntimeError(f"curl failed ({proc.returncode}): {proc.stderr.strip()}")
        data = json.loads(proc.stdout)
    # {"0": {"cik_str": ..., "ticker": "NVDA", "title": ...}, ...}
    tickers = {
        row["ticker"].strip().upper()
        for row in data.values()
        if row.get("ticker", "").strip()
    }
    return sorted(tickers)


def render(tickers: list[str]) -> str:
    header = (
        "# US-listed stock tickers — closed membership set for the\n"
        "# `membership: us_tickers` taxonomy directive on finance.securities.ticker.\n"
        "# Source: SEC company_tickers.json (https://www.sec.gov/files/company_tickers.json,\n"
        "# public domain), the canonical ticker->CIK map behind EDGAR.\n"
        "# Extraction: DISTINCT upper-cased `ticker` field, sorted. A ticker has no\n"
        "# checksum and its shape (^[A-Z]{1,7}$) confirms every short uppercase token,\n"
        "# so list membership is the substance (Precision Principle). Class shares keep\n"
        "# the SEC dash form (BRK-B); the >=90% column guard tolerates dot-form minorities.\n"
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
    tickers = fetch_tickers(from_file)
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
