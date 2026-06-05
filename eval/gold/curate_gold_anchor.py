#!/usr/bin/env python3
"""Curate the gold eval anchor (spec 2026-06-05-gold-eval-anchor, ac-02/ac-03).

Discovery + INDEPENDENT labelling over the corpus-pass columns parquet. The YDF
and Sense predictions are used ONLY to find candidate columns (discovery); the
gold label is assigned by a hand-authored labelling function that inspects the
column header and value shape — never by copying the lens prediction. See
eval/gold/README.md for the independence contract.

Sanitisation: the emitted fixture stores NO raw third-party values. Columns are
identified by (file_content_sha256, column_name); provenance is expressed as
header tokens + pattern-match counts, not echoed values.

Run:  ../gittables/.venv/bin/python curate_gold_anchor.py \
          --columns ../gittables/corpus_pass/columns.parquet \
          --out gold_eval_anchor.tsv
"""
from __future__ import annotations

import argparse
import re
import sys
from collections import defaultdict
from pathlib import Path

import pyarrow.parquet as pq

SEP = "│"  # │ — the corpus sample-value separator

# Canonical taxonomy type-ids (verified against the corpus + definitions).
T_ALNUM = "representation.identifier.alphanumeric_id"
T_ISO6346 = "geography.transportation.iso6346"
T_MGRS = "geography.coordinate.mgrs"
T_COUNTRY = "geography.location.country_code"
T_CATEGORICAL = "representation.discrete.categorical"
T_LAT = "geography.coordinate.latitude"
T_LON = "geography.coordinate.longitude"
T_DECIMAL = "representation.numeric.decimal_number"
T_YEAR = "datetime.component.year"
T_INT = "representation.numeric.integer_number"

# Oracle tie-breaker patterns (hand-authored; independent of YDF/Sense/B2).
RE_ISO6346 = re.compile(r"^[A-Z]{3}[UJZ]\d{7}$")
RE_MGRS = re.compile(r"^\d{1,2}[C-X][A-Z]{2}\d+$")
RE_ALNUM_TOKEN = re.compile(r"^[A-Za-z][A-Za-z0-9_\-]*\d[A-Za-z0-9_\-]*$")
RE_2LETTER = re.compile(r"^[A-Za-z]{2}$")
RE_3LETTER = re.compile(r"^[A-Za-z]{3}$")
RE_4DIGIT = re.compile(r"^\d{4}$")

HDR_ID = re.compile(r"(_id$|^id$|msg|stock|cluster|uuid|guid|hash|code$|key$)", re.I)
HDR_COUNTRY = re.compile(r"(country|nation|\bcc\b|iso.?3166|iso_country)", re.I)
HDR_TEAM_EXCH = re.compile(r"(team|abbrev|exchange|ticker|venue|league|club)", re.I)
HDR_LAT = re.compile(r"(^|[^a-z])(lat|latitude)([^a-z]|$)", re.I)
HDR_LON = re.compile(r"(^|[^a-z])(lon|lng|long|longitude)([^a-z]|$)", re.I)
HDR_YEAR = re.compile(r"(year|^yr$|season|fiscal)", re.I)
HDR_NONTEMPORAL = re.compile(r"(_id$|^id$|count|num|qty|code|rank|index|score)", re.I)
HDR_NONGEO_NUMERIC = re.compile(
    r"(ratio|score|rate|index|temp|depth|prob|weight|coef|kl|loss|mean|std)", re.I
)


def values_of(raw: str) -> list[str]:
    if not raw:
        return []
    return [v for v in raw.split(SEP) if v != ""]


def as_floats(vals: list[str]) -> list[float]:
    out = []
    for v in vals:
        try:
            out.append(float(v))
        except ValueError:
            return []  # any non-float => not a clean numeric column
    return out


def relativise(path: str) -> str:
    """Strip the local prefix; keep the gittables-relative path. No secrets in a
    file path, but the local home/username need not be vendored into a tracked
    fixture."""
    m = re.search(r"(gittables/.*)$", path)
    if m:
        return m.group(1)
    return Path(path).name


# --- Per-family independent labelling functions -------------------------------
# Each returns (curated_label, provenance) or None to drop an ambiguous column.


def label_family_a(header: str, vals: list[str]) -> tuple[str, str] | None:
    """Tight-code (iso6346/mgrs) vs alphanumeric_id."""
    n = len(vals)
    iso = sum(1 for v in vals if RE_ISO6346.match(v))
    mgrs = sum(1 for v in vals if RE_MGRS.match(v))
    if iso and iso == n:
        return T_ISO6346, f"all {n} match ^[A-Z]{{3}}[UJZ]\\d{{7}}$ (genuine iso6346)"
    if mgrs and mgrs == n:
        return T_MGRS, f"all {n} match MGRS grid pattern (genuine mgrs)"
    alnum = sum(1 for v in vals if RE_ALNUM_TOKEN.match(v))
    if alnum == n and iso == 0 and mgrs == 0 and (HDR_ID.search(header) or alnum == n):
        return (
            T_ALNUM,
            f"header '{header}'; {alnum}/{n} alnum-id tokens; "
            f"0 match iso6346/mgrs tight pattern",
        )
    return None


def label_family_b(header: str, vals: list[str]) -> tuple[str, str] | None:
    """country_code vs categorical (team/exchange codes). Disambiguated by
    value-LENGTH (2 vs 3) + header semantics — NOT by ISO vocabulary membership,
    which would make this a B2 sieve and breach independence."""
    n = len(vals)
    two = sum(1 for v in vals if RE_2LETTER.match(v))
    three = sum(1 for v in vals if RE_3LETTER.match(v))
    if two == n and HDR_COUNTRY.search(header):
        return (
            T_COUNTRY,
            f"header '{header}' (geographic); all {n} are 2-letter codes "
            f"(alpha-2 shape; not ISO-vocab lookup)",
        )
    if three == n and HDR_TEAM_EXCH.search(header):
        return (
            T_CATEGORICAL,
            f"header '{header}' (team/exchange); all {n} are 3-letter codes "
            f"(not 2-letter country shape)",
        )
    return None


def label_family_c(
    header: str, vals: list[str], sense: str
) -> tuple[str, str] | None:
    """latitude vs longitude vs temperature(decimal_number). Header + range."""
    floats = as_floats(vals)
    if not floats:
        return None
    lo, hi = min(floats), max(floats)
    if HDR_LAT.search(header) and -90.0 <= lo and hi <= 90.0:
        return T_LAT, f"header '{header}' (lat); all values in [-90,90]"
    if HDR_LON.search(header) and -180.0 <= lo and hi <= 180.0:
        return T_LON, f"header '{header}' (lon); all values in [-180,180]"
    # Float column a lens calls a coordinate but the header is plainly a
    # measurement/score and there is NO temperature type => decimal_number.
    if (
        sense in (T_LAT, T_LON)
        and not HDR_LAT.search(header)
        and not HDR_LON.search(header)
        and HDR_NONGEO_NUMERIC.search(header)
    ):
        return (
            T_DECIMAL,
            f"header '{header}' (non-geographic measurement); float column "
            f"mis-sensed as coordinate; no temperature type => decimal_number",
        )
    return None


def label_family_d(header: str, vals: list[str]) -> tuple[str, str] | None:
    """component.year vs integer_number. Header + plausible-year range."""
    n = len(vals)
    four = sum(1 for v in vals if RE_4DIGIT.match(v))
    if four != n:
        return None
    ys = [int(v) for v in vals]
    in_year_range = all(1500 <= y <= 2100 for y in ys)
    if HDR_YEAR.search(header) and in_year_range:
        return T_YEAR, f"header '{header}' (temporal); all {n} 4-digit in [1500,2100]"
    if HDR_NONTEMPORAL.search(header) or not in_year_range:
        return (
            T_INT,
            f"header '{header}' (non-temporal/out-of-range); 4-digit integers "
            f"=> integer_number not year",
        )
    return None


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--columns", required=True, type=Path)
    ap.add_argument("--out", required=True, type=Path)
    ap.add_argument("--per-label-cap", type=int, default=30)
    args = ap.parse_args()

    tbl = pq.read_table(
        args.columns,
        columns=[
            "file_path",
            "file_content_sha256",
            "column_name",
            "sense_prediction",
            "ydf_prediction",
            "sample_values_truncated",
            "is_trivial",
        ],
    )
    rows = tbl.to_pylist()
    print(f"read {len(rows)} columns from {args.columns}", file=sys.stderr)

    # (family, curated_label) -> list of emitted rows; dedup by identity.
    buckets: dict[tuple[str, str], list[dict]] = defaultdict(list)
    seen: set[tuple[str, str]] = set()

    for r in rows:
        if r.get("is_trivial"):
            continue
        header = (r.get("column_name") or "").strip()
        ydf = r.get("ydf_prediction") or ""
        sense = r.get("sense_prediction") or ""
        vals = values_of(r.get("sample_values_truncated") or "")
        if not header or len(vals) < 3:
            continue
        identity = (r.get("file_content_sha256") or "", header)
        if identity in seen:
            continue

        labelled: tuple[str, str] | None = None
        family: str | None = None
        if "iso6346" in ydf or "mgrs" in ydf:
            family = "A_tight_code_vs_alnum"
            labelled = label_family_a(header, vals)
        elif "country_code" in ydf:
            family = "B_country_vs_categorical"
            labelled = label_family_b(header, vals)
        elif sense in (T_LAT, T_LON) or HDR_LAT.search(header) or HDR_LON.search(header):
            family = "C_lat_lon_temperature"
            labelled = label_family_c(header, vals, sense)
        elif "component.year" in ydf or "component.year" in sense:
            family = "D_year_vs_integer"
            labelled = label_family_d(header, vals)

        if labelled is None or family is None:
            continue
        curated, provenance = labelled
        key = (family, curated)
        if len(buckets[key]) >= args.per_label_cap:
            continue
        seen.add(identity)
        buckets[key].append(
            {
                "family": family,
                "file_path": relativise(r.get("file_path") or ""),
                "file_content_sha256": r.get("file_content_sha256") or "",
                "column_name": header,
                "curated_label": curated,
                "ydf_context": ydf,
                "sense_context": sense,
                "labeller": "curate_gold_anchor.py:label_" + family.split("_")[0].lower(),
                "provenance": provenance,
            }
        )

    out_rows = [row for key in sorted(buckets) for row in buckets[key]]
    header_cols = [
        "family",
        "file_path",
        "file_content_sha256",
        "column_name",
        "curated_label",
        "ydf_context",
        "sense_context",
        "labeller",
        "provenance",
    ]
    with args.out.open("w") as fh:
        fh.write("\t".join(header_cols) + "\n")
        for row in out_rows:
            fh.write("\t".join(str(row[c]).replace("\t", " ") for c in header_cols) + "\n")

    print(f"wrote {len(out_rows)} gold rows -> {args.out}", file=sys.stderr)
    # Per (family, label) tally for the floor audit.
    for key in sorted(buckets):
        print(f"  {key[0]:28s} {key[1]:45s} {len(buckets[key])}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
