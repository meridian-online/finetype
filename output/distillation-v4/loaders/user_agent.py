#!/usr/bin/env python3
"""Per-type distilled data loader for technology.internet.user_agent (v17 spec ac-01).

Source:  https://github.com/ua-parser/uap-core
         Raw fixtures pulled from:
           - tests/test_ua.yaml      (~1,900 UA strings, browser/bot family fixtures)
           - tests/test_device.yaml  (~1,100 UA strings, device/brand/model fixtures)
           - tests/test_os.yaml      (~540 UA strings, operating-system fixtures)

License: Apache License 2.0
         https://github.com/ua-parser/uap-core/blob/master/LICENSE
         The regexes.yaml data is Copyright 2009 Google Inc. and released
         under Apache 2.0. The test fixtures inherit the repository license.

Access:  Public GitHub raw URLs, no authentication required. Cached to
         output/distillation-v4/loaders/_cache/ after first fetch.

Date accessed: 2026-04-20
Expected row count: ~3,500 raw fixtures → ≥1,000 unique UA strings.

Output:  output/distillation-v4/user_agent.csv
         Header: value,label
         Label:  technology.internet.user_agent  (canonical taxonomy key)

Run:     python output/distillation-v4/loaders/user_agent.py

Rationale (v17 spec, sourcing_table user_agent row):
  v16 eval failures: user_agent columns classified as jwt / docker_ref.
  Real UA fixtures give the model a corpus of the actual token shape
  instead of generator-only synthetic strings.

Spec reference:
  specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml (v1.3)
  acceptance_criteria: ac-01
"""

from __future__ import annotations

import csv
import sys
import urllib.request
from pathlib import Path

LABEL = "technology.internet.user_agent"
MIN_UNIQUE = 1000

# Canonical upstream: ua-parser/uap-core (Apache-2.0)
SOURCES = {
    "test_ua.yaml": "https://raw.githubusercontent.com/ua-parser/uap-core/master/tests/test_ua.yaml",
    "test_device.yaml": "https://raw.githubusercontent.com/ua-parser/uap-core/master/tests/test_device.yaml",
    "test_os.yaml": "https://raw.githubusercontent.com/ua-parser/uap-core/master/tests/test_os.yaml",
}

HERE = Path(__file__).resolve().parent
CACHE_DIR = HERE / "_cache" / "user_agent"
OUTPUT_CSV = HERE.parent / "user_agent.csv"


def fetch_source(name: str, url: str) -> str:
    """Download a raw fixture file, caching locally. Idempotent on re-run."""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    cached = CACHE_DIR / name
    if cached.exists() and cached.stat().st_size > 0:
        return cached.read_text(encoding="utf-8")
    print(f"[user_agent] fetching {url}", file=sys.stderr)
    req = urllib.request.Request(url, headers={"User-Agent": "finetype-distillation-v4"})
    with urllib.request.urlopen(req, timeout=60) as resp:  # noqa: S310 - fixed https URL
        data = resp.read().decode("utf-8")
    cached.write_text(data, encoding="utf-8")
    return data


def extract_ua_strings(yaml_text: str) -> list[str]:
    """Parse YAML fixture text and extract `user_agent_string` values.

    Intentionally avoids PyYAML dependency — the fixture format is
    line-oriented with one `user_agent_string: '...'` per test case.
    Handles single-quoted and double-quoted values; YAML-escaped
    doubled-single-quotes ('') become a single quote.
    """
    results: list[str] = []
    for line in yaml_text.splitlines():
        stripped = line.strip()
        # Match both "- user_agent_string: '...'" and "  user_agent_string: '...'"
        prefix_variants = ("user_agent_string:", "- user_agent_string:")
        for prefix in prefix_variants:
            if stripped.startswith(prefix):
                value = stripped[len(prefix):].strip()
                if not value:
                    break
                # Strip surrounding quote characters
                if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
                    quote = value[0]
                    inner = value[1:-1]
                    if quote == "'":
                        # YAML single-quote escaping: '' -> '
                        inner = inner.replace("''", "'")
                    else:
                        # Double-quote: minimal unescape for \\ and \"
                        inner = inner.replace('\\"', '"').replace("\\\\", "\\")
                    results.append(inner)
                else:
                    # Unquoted scalar
                    results.append(value)
                break
    return results


def main() -> int:
    all_uas: list[str] = []
    for name, url in SOURCES.items():
        try:
            text = fetch_source(name, url)
        except Exception as exc:  # noqa: BLE001 - fail loud, no retry
            print(f"[user_agent] ERROR fetching {name}: {exc}", file=sys.stderr)
            print(
                "[user_agent] If offline, download the three YAML files manually "
                f"into {CACHE_DIR} and re-run.",
                file=sys.stderr,
            )
            return 2
        extracted = extract_ua_strings(text)
        print(f"[user_agent] {name}: {len(extracted)} UA strings", file=sys.stderr)
        all_uas.extend(extracted)

    # Deduplicate while preserving first-seen order
    seen: set[str] = set()
    unique: list[str] = []
    for ua in all_uas:
        if ua and ua not in seen:
            seen.add(ua)
            unique.append(ua)

    print(
        f"[user_agent] total={len(all_uas)} unique={len(unique)} target>={MIN_UNIQUE}",
        file=sys.stderr,
    )

    if len(unique) < MIN_UNIQUE:
        print(
            f"[user_agent] FAIL: {len(unique)} unique values < target {MIN_UNIQUE}. "
            "Check source availability or invoke the fallback-to-generator clause "
            "in the v17 spec sourcing_table.",
            file=sys.stderr,
        )
        return 1

    OUTPUT_CSV.parent.mkdir(parents=True, exist_ok=True)
    with OUTPUT_CSV.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, quoting=csv.QUOTE_MINIMAL)
        writer.writerow(["value", "label"])
        for ua in unique:
            writer.writerow([ua, LABEL])

    print(f"[user_agent] wrote {len(unique)} rows -> {OUTPUT_CSV}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
