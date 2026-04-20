#!/usr/bin/env python3
"""Per-type distilled data loader for identity.medical.loinc (v17 spec ac-01).

Source:  NIH National Library of Medicine — Clinical Tables LOINC API
         https://clinicaltables.nlm.nih.gov/apidoc/loinc_items/v3/doc.html
         Endpoint: https://clinicaltables.nlm.nih.gov/api/loinc_items/v3/search
         This is the NLM's public REST search service over the full LOINC
         table. No authentication, no click-through, free for research &
         production use.

License: LOINC codes are Copyright © Regenstrief Institute, Inc.
         LOINC Terms of Use (https://loinc.org/terms-of-use/) permit
         royalty-free redistribution with attribution. Key obligations:
           - Include the LOINC copyright notice and link with any product
             that uses LOINC content.
           - Do not modify field names or contents of Group 1 Artifacts.
           - Do not use LOINC to promulgate a competing identifier standard.
         The attribution string required downstream is:
           "This material contains content from LOINC (http://loinc.org).
            LOINC is copyright © Regenstrief Institute, Inc."
         Carry this string into SOURCES.md (spec ac-03).

Access:  Public HTTPS, no API key required. Paged via &count=500&offset=N.
         Cached to output/distillation-v4/loaders/_cache/ on first fetch.

Date accessed: 2026-04-20
Expected row count: ≥1,000 unique LOINC codes (format: NNNNN-N).

Output:  output/distillation-v4/loinc.csv
         Header: value,label
         Label:  identity.medical.loinc  (canonical taxonomy key)

Run:     python output/distillation-v4/loaders/loinc.py

Rationale (v17 spec, sourcing_table loinc row):
  LOINC has no v16 eval failure; this is quality-prevention for future
  data. The v16 distilled rows for loinc were mislabeled noise, so the
  type was dropped from distilled training entirely. This loader restores
  real LOINC codes to the corpus.

Sampling strategy:
  The Clinical Tables search endpoint requires a `terms=` query. We sweep
  single-letter prefixes over the long-common-name field to cover the
  code space broadly without cherry-picking by medical specialty. Each
  page returns up to 500 rows; we deduplicate by code.

Spec reference:
  specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml (v1.3)
  acceptance_criteria: ac-01
"""

from __future__ import annotations

import csv
import json
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path
from urllib.error import HTTPError

LABEL = "identity.medical.loinc"
MIN_UNIQUE = 1000

BASE_URL = "https://clinicaltables.nlm.nih.gov/api/loinc_items/v3/search"
# Single-letter sweep — each letter returns a fresh slice of the LOINC
# universe (matched against long common names). We iterate until we have
# enough unique codes.
QUERY_TERMS = list("abcdefghijklmnopqrstuvwxyz")
PAGE_SIZE = 500  # NLM API max per request
MAX_OFFSET = 7000  # NLM API rejects offset > ~7000 with HTTP 400; stop there

LOINC_CODE_RE = re.compile(r"^\d{1,5}-\d$")

HERE = Path(__file__).resolve().parent
CACHE_DIR = HERE / "_cache" / "loinc"
OUTPUT_CSV = HERE.parent / "loinc.csv"


def fetch_page(term: str, offset: int) -> list[str]:
    """Fetch one search page and return the list of LOINC codes it contains.

    Returns [] on HTTP 400 (NLM rejects offsets past its internal cap —
    this marks end of the result set for the term, not a real error).
    """
    params = {
        "terms": term,
        "maxList": str(PAGE_SIZE),
        "offset": str(offset),
        "df": "LOINC_NUM",
    }
    url = f"{BASE_URL}?{urllib.parse.urlencode(params)}"
    cache_key = f"{term}_{offset}.json"
    cached = CACHE_DIR / cache_key
    CACHE_DIR.mkdir(parents=True, exist_ok=True)

    if cached.exists() and cached.stat().st_size > 0:
        body = cached.read_text(encoding="utf-8")
    else:
        req = urllib.request.Request(url, headers={"User-Agent": "finetype-distillation-v4"})
        try:
            with urllib.request.urlopen(req, timeout=60) as resp:  # noqa: S310 - fixed https URL
                body = resp.read().decode("utf-8")
        except HTTPError as exc:
            if exc.code == 400:
                # Offset past NLM's internal cap — treat as empty page.
                return []
            raise
        cached.write_text(body, encoding="utf-8")
        # Be polite to NLM — small delay between uncached calls
        time.sleep(0.2)

    try:
        payload = json.loads(body)
    except json.JSONDecodeError as exc:
        print(f"[loinc] JSON decode error for term={term} offset={offset}: {exc}", file=sys.stderr)
        return []

    # API shape: [total_count, [codes...], null_or_extra, [[code], ...]]
    if not isinstance(payload, list) or len(payload) < 2:
        return []
    codes = payload[1]
    if not isinstance(codes, list):
        return []
    return [c for c in codes if isinstance(c, str) and LOINC_CODE_RE.match(c)]


def main() -> int:
    unique: set[str] = set()
    ordered: list[str] = []

    for term in QUERY_TERMS:
        if len(unique) >= MIN_UNIQUE * 2:
            # Plenty of headroom; no need to keep sweeping.
            break
        offset = 0
        stalled_pages = 0
        while offset <= MAX_OFFSET:
            try:
                codes = fetch_page(term, offset)
            except Exception as exc:  # noqa: BLE001
                print(
                    f"[loinc] ERROR fetching term={term} offset={offset}: {exc}",
                    file=sys.stderr,
                )
                print(
                    "[loinc] If offline, pre-populate the cache at "
                    f"{CACHE_DIR} with page JSON responses from the NLM API "
                    "and re-run. See module docstring for the URL format.",
                    file=sys.stderr,
                )
                return 2
            if not codes:
                break
            new_before = len(unique)
            for c in codes:
                if c not in unique:
                    unique.add(c)
                    ordered.append(c)
            added = len(unique) - new_before
            if added == 0:
                stalled_pages += 1
                if stalled_pages >= 2:
                    break
            else:
                stalled_pages = 0
            print(
                f"[loinc] term={term} offset={offset} +{added} total={len(unique)}",
                file=sys.stderr,
            )
            offset += PAGE_SIZE

    print(
        f"[loinc] final unique={len(unique)} target>={MIN_UNIQUE}",
        file=sys.stderr,
    )

    if len(unique) < MIN_UNIQUE:
        print(
            f"[loinc] FAIL: {len(unique)} unique codes < target {MIN_UNIQUE}. "
            "Check NLM API availability or invoke the fallback-to-generator "
            "clause in the v17 spec sourcing_table.",
            file=sys.stderr,
        )
        return 1

    OUTPUT_CSV.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_CSV.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, quoting=csv.QUOTE_MINIMAL)
        writer.writerow(["value", "label"])
        for code in ordered:
            writer.writerow([code, LABEL])

    print(f"[loinc] wrote {len(ordered)} rows -> {OUTPUT_CSV}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
