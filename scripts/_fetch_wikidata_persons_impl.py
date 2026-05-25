#!/usr/bin/env python3
"""Internal: SPARQL fetch of Wikidata person primitives. Driven by
scripts/fetch_wikidata_persons.sh per spec 2026-05-25-v22-boundary-training
ac-01. Not intended for direct invocation — the wrapper script handles
registration + verification.

Three outputs in --dest:
  given_names.tsv    qid\tlabel    (Q202444 'given name' instances)
  family_names.tsv   qid\tlabel    (Q101352 'family name' instances)
  persons.tsv        qid\tlabel    (Q-id-ranged sample of Q5 entities)

WDQS has a 60-second timeout per request. To stay safely under that
budget we chunk the Q5 sample by numeric Q-id range; given/family
queries return ~50k rows in one shot.

Idempotent: if a target TSV exists with non-empty content for the
current date, the corresponding SPARQL pass is skipped.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path

WDQS_URL = "https://query.wikidata.org/sparql"
USER_AGENT = (
    "FineType-fetch/1.0 (https://github.com/meridian-online/finetype; "
    "v22-boundary-training spec) - academic research"
)

GIVEN_NAME_QUERY = """
SELECT ?item ?itemLabel WHERE {
  ?item wdt:P31 wd:Q202444 .
  ?item rdfs:label ?itemLabel .
  FILTER(LANG(?itemLabel) = "en")
}
"""

# Q101352 (family name) has ~685k direct instances — too many for a
# single WDQS query, so we page by numeric Q-id range like the Q5 sample.
FAMILY_NAME_QUERY_TMPL = """
SELECT ?item ?itemLabel WHERE {{
  ?item wdt:P31 wd:Q101352 .
  ?item rdfs:label ?itemLabel .
  FILTER(LANG(?itemLabel) = "en")
  FILTER(
    xsd:integer(SUBSTR(STR(?item), {qid_offset})) >= {lo}
    && xsd:integer(SUBSTR(STR(?item), {qid_offset})) < {hi}
  )
}}
"""

# Q5 sample: paged by numeric Q-id range. Each page targets a window of
# QID_RANGE_STEP Q-ids; with Q5 instance density ~10% the typical page
# returns 5k-15k labels. We stop once persons_target is reached.
PERSONS_QUERY_TMPL = """
SELECT ?item ?itemLabel WHERE {{
  ?item wdt:P31 wd:Q5 .
  ?item rdfs:label ?itemLabel .
  FILTER(LANG(?itemLabel) = "en")
  FILTER(
    xsd:integer(SUBSTR(STR(?item), {qid_offset})) >= {lo}
    && xsd:integer(SUBSTR(STR(?item), {qid_offset})) < {hi}
  )
}}
"""
QID_PREFIX = "http://www.wikidata.org/entity/Q"
QID_OFFSET = len(QID_PREFIX) + 1  # SUBSTR is 1-indexed

# Reasonable ceiling — Wikidata Q-ids extend past 130M; we step in
# 50k windows and stop when persons_target is hit. Step is small so
# dense Q-id ranges (Q5 in particular) don't blow WDQS's response
# budget — early Q-ids are super-dense with notable people and a
# 100k-step query against Q1-Q100000 was reliably timing out with
# IncompleteRead errors.
QID_RANGE_STEP = 25_000  # small step keeps each query well under WDQS
                          # timeout and response-size limits
QID_RANGE_MAX = 130_000_000
# Skip the densest early Q-id band — Q1-Q1M has the most-notable
# historical people and the response volume per range is too large
# to ship cleanly over HTTP. We sample from Q1M+ where density is
# uniform (~10% Q5).
PERSONS_START_QID = 1_000_000
INTER_QUERY_SLEEP = 1.0
RETRY_ATTEMPTS = 3
RETRY_BACKOFF = 8.0


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--dest", type=Path, required=True)
    p.add_argument("--persons-target", type=int, default=50_000)
    return p.parse_args()


def sparql_query(query: str) -> list[dict]:
    """Run a single WDQS query via curl (urllib gave IncompleteRead errors
    on responses > ~1.5 MB; curl's HTTP/2 handling is more reliable for
    the streaming WDQS responses). Returns binding dicts."""
    last_exc: Exception | None = None
    for attempt in range(1, RETRY_ATTEMPTS + 1):
        try:
            proc = subprocess.run(
                [
                    "curl", "-sSL", "--fail", "-G",
                    "--http1.1",  # WDQS HTTP/2 occasionally returns
                                  # protocol errors mid-stream
                    "--data-urlencode", f"query={query}",
                    "-H", "Accept: application/sparql-results+json",
                    "-H", f"User-Agent: {USER_AGENT}",
                    "--max-time", "120",
                    WDQS_URL,
                ],
                check=True, capture_output=True, text=True,
            )
            payload = json.loads(proc.stdout)
            return payload["results"]["bindings"]
        except Exception as exc:  # noqa: BLE001
            last_exc = exc
            print(f"  retry {attempt}/{RETRY_ATTEMPTS} after error: "
                  f"{type(exc).__name__}: {exc}",
                  file=sys.stderr)
            time.sleep(RETRY_BACKOFF * attempt)
    raise RuntimeError(f"WDQS query failed after {RETRY_ATTEMPTS} attempts: {last_exc}")


def qid_from_uri(uri: str) -> str:
    return uri.rsplit("/", 1)[-1]


def write_rows(path: Path, rows: list[tuple[str, str]]) -> int:
    seen: set[str] = set()
    with path.open("w", encoding="utf-8") as f:
        f.write("qid\tlabel\n")
        for qid, label in rows:
            if not label or qid in seen:
                continue
            seen.add(qid)
            # Replace tab/newline in label to keep TSV well-formed
            clean = label.replace("\t", " ").replace("\n", " ").strip()
            if not clean:
                continue
            f.write(f"{qid}\t{clean}\n")
    return len(seen)


def has_content(path: Path) -> bool:
    return path.exists() and path.stat().st_size > 64  # > header line


def fetch_property_class(query: str, path: Path, label: str) -> int:
    if has_content(path):
        # Idempotent: count existing rows and return.
        with path.open() as f:
            n = sum(1 for _ in f) - 1
        print(f"  cached {label}: {path} ({n:,} rows)", file=sys.stderr)
        return n
    print(f"  query {label}...", file=sys.stderr)
    bindings = sparql_query(query)
    rows = [
        (qid_from_uri(b["item"]["value"]), b["itemLabel"]["value"])
        for b in bindings if "itemLabel" in b
    ]
    n = write_rows(path, rows)
    print(f"  wrote {label}: {n:,} rows -> {path}", file=sys.stderr)
    return n


def fetch_persons(path: Path, target: int) -> int:
    if has_content(path):
        with path.open() as f:
            n = sum(1 for _ in f) - 1
        print(f"  cached persons: {path} ({n:,} rows)", file=sys.stderr)
        return n
    print(f"  paged Q5 fetch (target {target:,}, start Q{PERSONS_START_QID})...",
          file=sys.stderr)
    collected: list[tuple[str, str]] = []
    lo = PERSONS_START_QID
    n_pages = 0
    while lo < QID_RANGE_MAX and len(collected) < target:
        hi = lo + QID_RANGE_STEP
        query = PERSONS_QUERY_TMPL.format(
            qid_offset=QID_OFFSET, lo=lo, hi=hi,
        )
        try:
            bindings = sparql_query(query)
        except RuntimeError as exc:
            # Skip a window that consistently times out — fairly common
            # for ranges with dense Q5 populations near low Q-ids.
            print(f"  skip Q{lo}-Q{hi}: {exc}", file=sys.stderr)
            lo = hi
            continue
        n_pages += 1
        before = len(collected)
        for b in bindings:
            if "itemLabel" not in b:
                continue
            collected.append((
                qid_from_uri(b["item"]["value"]),
                b["itemLabel"]["value"],
            ))
        delta = len(collected) - before
        print(f"  Q{lo}-Q{hi}: +{delta:,} (total {len(collected):,})",
              file=sys.stderr)
        lo = hi
        time.sleep(INTER_QUERY_SLEEP)
        if n_pages % 5 == 0:
            # Incremental write so partial progress survives interruption.
            write_rows(path, collected)
    n = write_rows(path, collected)
    print(f"  wrote persons: {n:,} rows ({n_pages} pages) -> {path}",
          file=sys.stderr)
    return n


def fetch_paged(path: Path, query_tmpl: str, label: str, target: int,
                step: int = QID_RANGE_STEP) -> int:
    """Paged Q-id-range fetch for a P31 class with too many direct
    instances to return in one query (e.g. family names ~685k)."""
    if has_content(path):
        with path.open() as f:
            n = sum(1 for _ in f) - 1
        print(f"  cached {label}: {path} ({n:,} rows)", file=sys.stderr)
        return n
    print(f"  paged {label} fetch (target {target:,}, step {step:,})...",
          file=sys.stderr)
    collected: list[tuple[str, str]] = []
    lo = 1
    n_pages = 0
    while lo < QID_RANGE_MAX and len(collected) < target:
        hi = lo + step
        query = query_tmpl.format(qid_offset=QID_OFFSET, lo=lo, hi=hi)
        try:
            bindings = sparql_query(query)
        except RuntimeError as exc:
            print(f"  skip {label} Q{lo}-Q{hi}: {exc}", file=sys.stderr)
            lo = hi
            continue
        n_pages += 1
        before = len(collected)
        for b in bindings:
            if "itemLabel" not in b:
                continue
            collected.append((
                qid_from_uri(b["item"]["value"]),
                b["itemLabel"]["value"],
            ))
        delta = len(collected) - before
        print(f"  {label} Q{lo}-Q{hi}: +{delta:,} (total {len(collected):,})",
              file=sys.stderr)
        lo = hi
        time.sleep(INTER_QUERY_SLEEP)
        if n_pages % 5 == 0:
            write_rows(path, collected)
    n = write_rows(path, collected)
    print(f"  wrote {label}: {n:,} rows ({n_pages} pages) -> {path}",
          file=sys.stderr)
    return n


def main() -> int:
    args = parse_args()
    args.dest.mkdir(parents=True, exist_ok=True)

    n_given = fetch_property_class(
        GIVEN_NAME_QUERY, args.dest / "given_names.tsv", "given_names")
    n_family = fetch_paged(
        args.dest / "family_names.tsv",
        FAMILY_NAME_QUERY_TMPL,
        "family_names",
        target=20_000,
        step=1_000_000,  # ~0.5% density → ~5k/page
    )
    # persons.tsv is best-effort — Q5 has ~10M instances and WDQS
    # frequently closes the connection mid-response for Q5-dense Q-id
    # ranges (curl exit 18 / IncompleteRead). The downstream generator
    # treats persons.tsv as optional and falls back to combinatorial
    # given × family if absent.
    persons_path = args.dest / "persons.tsv"
    if args.persons_target > 0:
        try:
            n_persons = fetch_persons(persons_path, args.persons_target)
        except Exception as exc:  # noqa: BLE001
            print(f"  persons.tsv fetch failed ({exc}); proceeding without it.",
                  file=sys.stderr)
            n_persons = 0
            if not persons_path.exists():
                persons_path.write_text("qid\tlabel\n")
    else:
        n_persons = 0
        persons_path.write_text("qid\tlabel\n")

    combos = n_given * n_family
    print(f"summary: given={n_given:,}  family={n_family:,}  "
          f"persons={n_persons:,}  combinations={combos:,}",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
