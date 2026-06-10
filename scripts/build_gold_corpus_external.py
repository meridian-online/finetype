#!/usr/bin/env python3
"""Extract the external share of the gold-corpus candidate sample.

Spec 2026-06-10-human-verified-gold-corpus ac-02 (external share). Sources are
the six vendored + registered datasets under eval/datasets/gold_external/
(snapshots in eval/datasets/snapshots/gold-*.json, sources.yaml role=gold —
see eval/gold/gold_corpus_external_plan.md for licences and rationale).

One fixture row per (file, column), same schema as the GitTables share
(gold_corpus_candidates.tsv) with bucket=external. The stratum_label is the
CANDIDATE stratum the column informs — the human-verified label may differ
(that disagreement is signal, e.g. OpenFlights `country` holds country NAMES,
not codes; Majestic `Domain` holds bare domains, not URLs — boundary cases by
design). No raw values in the fixture; lenses read the vendored CSVs directly.

file_content_sha256 is the sha256 of the vendored file, matching the
registered snapshot — `dataset_verify.py` guards drift.

Usage: python3 scripts/build_gold_corpus_external.py
"""
from __future__ import annotations
import csv
import hashlib
from pathlib import Path

ROOT = Path("eval/datasets/gold_external")
OUT = Path("eval/gold/gold_corpus_candidates_external.tsv")

# file -> (registry name, licence, {column -> candidate stratum})
MAPPING: dict[str, tuple[str, str, dict[str, str]]] = {
    "openflights_airports.csv": ("gold-openflights-airports", "ODbL-1.0", {
        "utc_offset": "datetime.offset.utc",
        "tz_database_time_zone": "datetime.offset.utc",   # tz NAME — near-miss by design
        "latitude": "geography.coordinate.latitude",
        "longitude": "geography.coordinate.longitude",
        "altitude": "representation.numeric.integer_number",
        "city": "geography.location.city",
        "country": "geography.location.country_code",     # country NAMES — boundary case
        "iata": "representation.identifier.alphanumeric_id",
        "icao": "representation.identifier.alphanumeric_id",
        "dst": "representation.discrete.categorical",
        "type": "representation.discrete.categorical",
        "name": "representation.text.plain_text",
    }),
    "usgs_earthquakes_202605.csv": ("gold-usgs-earthquakes", "US-PD", {
        "latitude": "geography.coordinate.latitude",
        "longitude": "geography.coordinate.longitude",
        "depth": "representation.numeric.decimal_number",
        "mag": "representation.numeric.decimal_number",
        "time": "datetime.date.iso",
        "updated": "datetime.date.iso",
        "magType": "representation.discrete.categorical",
        "status": "representation.discrete.categorical",
        "net": "representation.discrete.categorical",
        "id": "representation.identifier.alphanumeric_id",
        "place": "representation.text.plain_text",
    }),
    "seattle_checkouts_sample.csv": ("gold-seattle-checkouts", "PDDL-1.0", {
        "isbn": "identity.commerce.isbn",
        "checkoutyear": "datetime.component.year",
        "publicationyear": "datetime.component.year",     # messy free-text years
        "checkouts": "representation.numeric.integer_number",
        "checkoutmonth": "representation.numeric.integer_number",
        "materialtype": "representation.discrete.categorical",
        "usageclass": "representation.discrete.categorical",
        "checkouttype": "representation.discrete.categorical",
        "title": "representation.text.plain_text",
        "creator": "representation.text.plain_text",
        "publisher": "representation.text.plain_text",
    }),
    "nyc_dob_permits_sample.csv": ("gold-nyc-dob-permits", "NYC-OpenData", {
        "zip_code": "geography.address.postal_code",
        "gis_latitude": "geography.coordinate.latitude",
        "gis_longitude": "geography.coordinate.longitude",
        "borough": "geography.location.region",
        "permit_status": "representation.discrete.categorical",
        "permit_type": "representation.discrete.categorical",
        "job_type": "representation.discrete.categorical",
        "work_type": "representation.discrete.categorical",
        "filing_status": "representation.discrete.categorical",
        "filing_date": "datetime.date.iso",                # actual format adjudicated
        "issuance_date": "datetime.date.iso",
        "expiration_date": "datetime.date.iso",
        "job_start_date": "datetime.date.iso",
        "job__": "representation.identifier.alphanumeric_id",
        "street_name": "representation.text.plain_text",
        "gis_nta_name": "geography.location.region",
    }),
    "nyc_payroll_sample.csv": ("gold-nyc-payroll", "NYC-OpenData", {
        "base_salary": "finance.currency.amount",
        "regular_gross_paid": "finance.currency.amount",
        "total_ot_paid": "finance.currency.amount",
        "total_other_pay": "finance.currency.amount",
        "fiscal_year": "datetime.component.year",
        "agency_start_date": "datetime.date.iso",
        "work_location_borough": "geography.location.region",
        "pay_basis": "representation.discrete.categorical",
        "leave_status_as_of_june_30": "representation.discrete.categorical",
        "agency_name": "representation.discrete.categorical",
        "title_description": "representation.text.plain_text",
        "regular_hours": "representation.numeric.decimal_number",
        "ot_hours": "representation.numeric.decimal_number",
    }),
    "ourairports_airports.csv": ("gold-ourairports", "US-PD", {
        "latitude_deg": "geography.coordinate.latitude",
        "longitude_deg": "geography.coordinate.longitude",
        "elevation_ft": "representation.numeric.integer_number",
        "continent": "representation.discrete.categorical",
        "iso_country": "geography.location.country_code",   # real ISO codes
        "iso_region": "geography.location.region",          # ISO 3166-2 codes
        "municipality": "geography.location.city",
        "scheduled_service": "representation.discrete.categorical",
        "type": "representation.discrete.categorical",
        "icao_code": "representation.identifier.alphanumeric_id",
        "iata_code": "representation.identifier.alphanumeric_id",
        "gps_code": "representation.identifier.alphanumeric_id",
        "home_link": "technology.internet.url",              # real full URLs
        "wikipedia_link": "technology.internet.url",
        "name": "representation.text.plain_text",
    }),
    "chicago_crimes_sample.csv": ("gold-chicago-crimes", "Chicago-OpenData", {
        "latitude": "geography.coordinate.latitude",
        "longitude": "geography.coordinate.longitude",
        "x_coordinate": "geography.coordinate.longitude",    # state-plane feet — near-miss by design
        "y_coordinate": "geography.coordinate.latitude",     # near-miss by design
        "date": "datetime.date.iso",                          # mdy + 12h clock — format adjudicated
        "updated_on": "datetime.date.iso",
        "year": "datetime.component.year",
        "primary_type": "representation.discrete.categorical",
        "location_description": "representation.discrete.categorical",
        "fbi_code": "representation.identifier.alphanumeric_id",
        "iucr": "representation.identifier.alphanumeric_id",
        "case_number": "representation.identifier.alphanumeric_id",
        "arrest": "representation.discrete.categorical",      # booleans — boundary case
        "ward": "representation.numeric.integer_number",
        "block": "representation.text.plain_text",
    }),
    "sf_businesses_sample.csv": ("gold-sf-businesses", "PDDL-1.0", {
        "business_zip": "geography.address.postal_code",
        "city": "geography.location.city",
        "state": "geography.location.region",
        "dba_start_date": "datetime.date.iso",
        "location_start_date": "datetime.date.iso",
        "data_as_of": "datetime.date.iso",
        "naic_code": "representation.identifier.alphanumeric_id",
        "naic_code_description": "representation.discrete.categorical",
        "neighborhoods_analysis_boundaries": "representation.discrete.categorical",
        "supervisor_district": "representation.numeric.integer_number",
        "certificate_number": "representation.identifier.alphanumeric_id",
        "ttxid": "representation.identifier.alphanumeric_id",
    }),
    "uk_price_paid_sample.csv": ("gold-uk-price-paid", "OGL-3.0", {
        "price": "finance.currency.amount",
        "postcode": "geography.address.postal_code",          # UK format
        "date_of_transfer": "datetime.date.iso",
        "town_city": "geography.location.city",
        "district": "geography.location.region",
        "county": "geography.location.region",
        "property_type": "representation.discrete.categorical",
        "duration": "representation.discrete.categorical",
        "transaction_id": "representation.identifier.alphanumeric_id",
        "street": "representation.text.plain_text",
    }),
    "majestic_million_top20k.csv": ("gold-majestic-million", "CC-BY-3.0", {
        "TLD": "technology.internet.top_level_domain",
        "IDN_TLD": "technology.internet.top_level_domain",
        "Domain": "technology.internet.url",               # bare domains — boundary case
        "IDN_Domain": "technology.internet.url",
        "GlobalRank": "representation.numeric.integer_number",
        "TldRank": "representation.numeric.integer_number",
        "RefSubNets": "representation.numeric.integer_number",
        "RefIPs": "representation.numeric.integer_number",
    }),
}

FIELDS = ["stratum_label", "tier", "bucket", "file_path", "file_content_sha256",
          "column_name", "sense_context", "ydf_context", "source", "licence",
          "curated_label", "labeller", "provenance"]


def main() -> int:
    rows = []
    for fname, (registry, licence, cols) in MAPPING.items():
        path = ROOT / fname
        sha = hashlib.sha256(path.read_bytes()).hexdigest()
        with path.open() as fh:
            header = next(csv.reader(fh))
        missing = [c for c in cols if c not in header]
        if missing:
            raise SystemExit(f"{fname}: mapped columns absent: {missing}")
        for col, stratum in cols.items():
            rows.append({
                "stratum_label": stratum, "tier": "external", "bucket": "external",
                "file_path": str(path), "file_content_sha256": sha,
                "column_name": col, "sense_context": "", "ydf_context": "",
                "source": registry, "licence": licence,
                "curated_label": "", "labeller": "",
                "provenance": "build_gold_corpus_external.py; snapshot-registered "
                              f"({registry}); candidate stratum, label adjudicated later",
            })
    with OUT.open("w", newline="") as fh:
        w = csv.DictWriter(fh, fieldnames=FIELDS, delimiter="\t")
        w.writeheader()
        w.writerows(rows)
    per = {}
    for r in rows:
        per[r["stratum_label"]] = per.get(r["stratum_label"], 0) + 1
    for k in sorted(per):
        print(f"{per[k]:3d}  {k}")
    print(f"\nwrote {len(rows)} external candidates -> {OUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
