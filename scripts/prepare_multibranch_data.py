#!/usr/bin/env python3
"""Prepare feature-vector training data for the multi-branch model.

Reads distilled data (sherlock_distilled.csv.gz) and synthetic data
(from finetype generate), blends them per-type, extracts 4 feature
branches via `finetype extract-features`, and writes a binary .ftmb file.

Supports two output formats:
  - v2 (legacy): flat list of records, no table grouping
  - v3 (default): records grouped by source table with sibling header metadata,
    enabling sibling-context attention during training

The 4 feature branches are:
  1. char:   960-dim character distribution features
  2. embed:  512-dim Model2Vec embedding aggregation over values
  3. stats:   27-dim column-level statistics
  4. header: 128-dim Model2Vec embedding of the column header

Usage:
    python3 scripts/prepare_multibranch_data.py [OPTIONS]

Options:
    --distilled PATH        Distilled CSV (default: output/distillation-v3/sherlock_distilled.csv.gz)
    --finetype PATH         finetype binary (default: ./target/release/finetype)
    --output PATH           Output binary file (default: output/multibranch-training/blend-50-50.ftmb)
    --label-remap PATH      Label remap JSON (default: data/label_remap.json)
    --samples-per-type N    Blend cap: max columns per type after blending (default: 1200)
    --synthetic-columns N   Synthetic columns to generate per type (default: 1200)
    --ratio-distilled F     Distilled ratio 0.0-1.0 (default: 0.5)
    --min-values N          Min values per column (default: 5)
    --seed N                Random seed (default: 42)
    --workers N             Parallel feature extraction workers (default: 4)
    --format v2|v3          Output format version (default: v3)
    --dry-run               Show counts without extracting features
    --skip-preflight        Skip preflight extraction check
    -h, --help              Show help
"""

import csv
import gzip
import json
import os
import random
import struct
import subprocess
import sys
import tempfile
import time
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed

# ═══════════════════════════════════════════════════════════════════════════════
# Constants
# ═══════════════════════════════════════════════════════════════════════════════

CHAR_DIM = 960
EMBED_DIM = 512
STATS_DIM = 27
HEADER_DIM = 128
MAGIC = b"FTMB"
VERSION_V2 = 2
VERSION_V3 = 3

# Column-level types that may cause negative transfer (same as prepare_spike_data.py)
COLUMN_LEVEL_TYPES = {
    "representation.discrete.categorical",
    "representation.discrete.ordinal",
    "representation.identifier.increment",
}


# ═══════════════════════════════════════════════════════════════════════════════
# Table templates for synthetic table assembly (v3)
#
# Each template defines a realistic table archetype: a list of type keys that
# commonly co-occur in real datasets. During assembly, 1-3 random types from
# other domains are added as controlled noise, producing tables of 5-15 columns.
# ═══════════════════════════════════════════════════════════════════════════════

TABLE_TEMPLATES = {
    "person_record": [
        "identity.person.email", "identity.person.full_name",
        "identity.person.phone_number", "geography.location.city",
        "geography.address.postal_code", "identity.person.height",
        "identity.person.gender",
    ],
    "customer_profile": [
        "identity.person.full_name", "identity.person.email",
        "geography.address.street_name", "geography.location.city",
        "geography.location.region", "geography.address.postal_code",
        "geography.location.country", "identity.person.phone_number",
    ],
    "financial_txn": [
        "finance.currency.amount", "finance.banking.iban",
        "datetime.timestamp.iso_8601", "representation.text.entity_name",
        "finance.currency.currency_code",
    ],
    "credit_card_txn": [
        "finance.payment.credit_card_number", "finance.currency.amount",
        "datetime.timestamp.iso_8601", "representation.text.entity_name",
        "identity.person.full_name", "finance.currency.currency_code",
    ],
    "geo_dataset": [
        "geography.coordinate.latitude", "geography.coordinate.longitude",
        "geography.location.city", "geography.location.country",
        "geography.location.region",
    ],
    "geo_places": [
        "representation.text.entity_name", "geography.coordinate.latitude",
        "geography.coordinate.longitude", "geography.address.full_address",
        "geography.address.postal_code", "geography.location.region",
    ],
    "web_log": [
        "technology.internet.ip_v4", "technology.internet.url",
        "technology.internet.user_agent", "datetime.timestamp.iso_8601",
    ],
    "web_analytics": [
        "technology.internet.url", "technology.internet.hostname",
        "datetime.timestamp.iso_8601", "representation.numeric.integer_number",
        "technology.internet.user_agent", "technology.internet.ip_v4",
    ],
    "product_catalog": [
        "representation.identifier.alphanumeric_id",
        "representation.text.entity_name", "finance.currency.amount",
        "representation.text.sentence",
    ],
    "ecommerce_order": [
        "representation.identifier.alphanumeric_id",
        "identity.person.full_name", "identity.person.email",
        "finance.currency.amount", "datetime.date.iso",
        "representation.numeric.integer_number",
    ],
    "employee_hr": [
        "identity.person.full_name", "identity.person.email",
        "representation.text.entity_name", "identity.person.phone_number",
        "datetime.date.iso", "finance.currency.amount",
        "identity.person.gender",
    ],
    "network_inventory": [
        "technology.internet.ip_v4", "technology.internet.ip_v6",
        "technology.internet.mac_address", "technology.internet.hostname",
        "representation.text.entity_name",
    ],
    "event_calendar": [
        "representation.text.entity_name", "datetime.date.iso",
        "datetime.time.iso", "representation.text.sentence",
        "geography.location.city",
    ],
    "scientific_data": [
        "representation.numeric.decimal_number",
        "representation.numeric.decimal_number",
        "representation.text.entity_name", "datetime.date.iso",
        "representation.numeric.percentage",
    ],
    "user_accounts": [
        "identity.person.username", "identity.person.email",
        "identity.person.password", "datetime.timestamp.iso_8601",
        "representation.identifier.uuid",
    ],
    "securities_data": [
        "finance.securities.isin", "finance.securities.cusip",
        "finance.currency.amount", "finance.rate.yield",
        "datetime.date.iso", "representation.text.entity_name",
    ],
    "file_registry": [
        "representation.file.extension", "representation.file.mime_type",
        "technology.cryptographic.hash", "representation.numeric.integer_number",
        "datetime.timestamp.iso_8601",
    ],
    "address_book": [
        "identity.person.full_name", "geography.address.full_address",
        "identity.person.phone_number", "identity.person.email",
        "geography.location.city", "geography.location.country",
    ],
    "survey_results": [
        "representation.numeric.integer_number",
        "representation.numeric.percentage",
        "representation.text.entity_name", "representation.boolean.terms",
        "representation.text.sentence", "datetime.date.iso",
    ],
    "api_logs": [
        "technology.internet.url", "technology.internet.ip_v4",
        "datetime.timestamp.iso_8601", "container.object.json",
        "representation.numeric.integer_number",
        "representation.file.mime_type",
    ],
}

# All unique types referenced in templates (for noise injection)
_TEMPLATE_TYPES = set()
for _types in TABLE_TEMPLATES.values():
    _TEMPLATE_TYPES.update(_types)


# ═══════════════════════════════════════════════════════════════════════════════
# Header variation mapping
#
# Realistic column name variations for synthetic data. Prevents the model from
# cheating on perfect type-key headers. Each type maps to a list of plausible
# column names an analyst might use. For types not in this mapping, we generate
# variations programmatically from the type key's leaf name.
# ═══════════════════════════════════════════════════════════════════════════════

HEADER_VARIATIONS = {
    # ─── Identity domain ──────────────────────────────────────────
    "identity.person.email": [
        "email", "email_address", "e-mail", "contact_email", "Email",
        "EMAIL_ADDR", "emailAddress", "user_email", "mail", "email_id",
    ],
    "identity.person.full_name": [
        "name", "full_name", "fullName", "customer_name", "Name",
        "FULL_NAME", "person_name", "contact_name", "display_name", "client_name",
    ],
    "identity.person.first_name": [
        "first_name", "firstName", "fname", "given_name", "First Name",
        "FIRST_NAME", "first", "givenName", "forename", "name_first",
    ],
    "identity.person.last_name": [
        "last_name", "lastName", "lname", "surname", "Last Name",
        "LAST_NAME", "family_name", "familyName", "last", "name_last",
    ],
    "identity.person.username": [
        "username", "user_name", "userName", "login", "Username",
        "USERNAME", "user_id", "screen_name", "handle", "account_name",
    ],
    "identity.person.gender": [
        "gender", "sex", "Gender", "GENDER", "gender_code", "m_f",
    ],
    "identity.person.age": [
        "age", "Age", "AGE", "person_age", "customer_age", "age_years",
    ],
    "identity.person.job_title": [
        "job_title", "title", "position", "job", "Job Title",
        "JOB_TITLE", "jobTitle", "role", "occupation", "designation",
    ],
    "identity.person.phone_number": [
        "phone", "phone_number", "phoneNumber", "telephone", "Phone",
        "PHONE", "tel", "mobile", "contact_phone", "phone_no",
    ],
    "identity.account.password": [
        "password", "passwd", "pass", "Password", "PASSWORD", "pwd",
        "secret", "user_password", "hashed_password", "pw",
    ],
    "identity.account.ssn": [
        "ssn", "social_security", "SSN", "social_security_number",
        "ss_number", "soc_sec", "ssn_number", "tax_id",
    ],
    "identity.identifier.uuid": [
        "uuid", "id", "UUID", "guid", "unique_id", "identifier",
        "record_id", "entity_id", "uid", "external_id",
    ],
    "identity.identifier.alphanumeric_id": [
        "id", "code", "ref", "reference", "ID", "record_id",
        "identifier", "ref_code", "external_ref", "item_id",
    ],
    # ─── Geography domain ─────────────────────────────────────────
    "geography.location.city": [
        "city", "City", "CITY", "city_name", "town", "municipality",
        "cityName", "location_city", "metro", "place",
    ],
    "geography.location.state": [
        "state", "State", "STATE", "state_name", "province",
        "state_code", "region", "stateName", "state_province",
    ],
    "geography.location.country": [
        "country", "Country", "COUNTRY", "country_name", "nation",
        "country_code", "countryName", "country_of_origin", "nationality",
    ],
    "geography.location.region": [
        "region", "Region", "REGION", "area", "district", "zone",
        "territory", "geo_region", "location_region",
    ],
    "geography.address.street_address": [
        "address", "street_address", "addr", "Address", "ADDRESS",
        "street", "address_line_1", "mailing_address", "location",
        "street_addr",
    ],
    "geography.address.full_address": [
        "full_address", "address", "complete_address", "Full Address",
        "FULL_ADDRESS", "mailing_address", "postal_address", "location",
    ],
    "geography.coordinate.latitude": [
        "latitude", "lat", "Latitude", "LAT", "geo_lat", "y",
        "lat_deg", "location_lat", "coord_lat",
    ],
    "geography.coordinate.longitude": [
        "longitude", "lon", "lng", "Longitude", "LON", "geo_lon", "x",
        "long", "location_lon", "coord_lon",
    ],
    "geography.postal.zip_code": [
        "zip", "zip_code", "zipcode", "postal_code", "Zip Code",
        "ZIP", "postcode", "postalCode", "zip_postal",
    ],
    "geography.coordinate.geohash": [
        "geohash", "geo_hash", "Geohash", "GEOHASH", "location_hash",
    ],
    # ─── Datetime domain ──────────────────────────────────────────
    "datetime.timestamp.iso_8601": [
        "timestamp", "datetime", "created_at", "updated_at", "Timestamp",
        "TIMESTAMP", "date_time", "event_time", "ts", "created",
    ],
    "datetime.date.iso_date": [
        "date", "Date", "DATE", "event_date", "start_date", "end_date",
        "birth_date", "created_date", "dateValue", "record_date",
    ],
    "datetime.time.iso_time": [
        "time", "Time", "TIME", "event_time", "start_time", "end_time",
        "clock_time", "timeValue", "scheduled_time",
    ],
    "datetime.date.us_date": [
        "date", "Date", "DATE", "mm_dd_yyyy", "us_date", "event_date",
        "start_date", "end_date", "date_us",
    ],
    "datetime.date.eu_date": [
        "date", "Date", "DATE", "dd_mm_yyyy", "eu_date", "event_date",
        "start_date", "end_date", "date_eu",
    ],
    "datetime.component.year": [
        "year", "Year", "YEAR", "yr", "fiscal_year", "birth_year",
        "start_year", "end_year", "year_value",
    ],
    "datetime.component.month_name": [
        "month", "Month", "MONTH", "month_name", "mon", "calendar_month",
    ],
    "datetime.component.day_of_week": [
        "day", "day_of_week", "weekday", "Day", "DAY", "dow",
    ],
    "datetime.duration.iso_duration": [
        "duration", "Duration", "DURATION", "elapsed", "time_span",
        "period", "interval",
    ],
    # ─── Finance domain ───────────────────────────────────────────
    "finance.monetary.usd": [
        "price", "amount", "cost", "Price", "PRICE", "total",
        "revenue", "salary", "fee", "balance", "payment",
    ],
    "finance.monetary.eur": [
        "price", "amount", "cost", "Price", "betrag", "preis",
        "montant", "prix", "total", "payment_eur",
    ],
    "finance.monetary.gbp": [
        "price", "amount", "cost", "Price", "total", "payment_gbp",
        "fee", "salary", "balance",
    ],
    "finance.identifier.iban": [
        "iban", "IBAN", "bank_account", "account_number", "iban_number",
        "Iban", "account_iban",
    ],
    "finance.identifier.credit_card": [
        "card_number", "credit_card", "cc_number", "card", "Card Number",
        "CC", "card_no", "creditCard", "payment_card",
    ],
    "finance.identifier.cusip": [
        "cusip", "CUSIP", "security_id", "cusip_number", "fund_cusip",
    ],
    "finance.identifier.isin": [
        "isin", "ISIN", "security_id", "isin_code", "instrument_id",
    ],
    "finance.identifier.ticker": [
        "ticker", "symbol", "stock_symbol", "Ticker", "TICKER",
        "stock", "equity_symbol", "trading_symbol",
    ],
    # ─── Technology domain ────────────────────────────────────────
    "technology.network.ipv4": [
        "ip", "ip_address", "ipv4", "IP", "IP_ADDRESS", "ipAddress",
        "source_ip", "dest_ip", "client_ip", "server_ip",
    ],
    "technology.network.ipv6": [
        "ipv6", "ip_address", "IPv6", "IPV6", "ipv6_address",
        "source_ipv6", "dest_ipv6",
    ],
    "technology.network.mac_address": [
        "mac", "mac_address", "MAC", "MAC_ADDRESS", "macAddress",
        "hardware_address", "nic_address", "physical_address",
    ],
    "technology.web.url": [
        "url", "URL", "link", "website", "web_address", "href",
        "page_url", "source_url", "endpoint",
    ],
    "technology.web.domain_name": [
        "domain", "hostname", "domain_name", "Domain", "DOMAIN",
        "host", "server_name", "site",
    ],
    "technology.web.user_agent": [
        "user_agent", "userAgent", "User-Agent", "ua", "USER_AGENT",
        "browser", "client_agent",
    ],
    "technology.web.mime_type": [
        "mime_type", "content_type", "mimeType", "MIME", "media_type",
        "Content-Type", "file_type",
    ],
    "technology.file.file_path": [
        "path", "file_path", "filepath", "Path", "FILE_PATH",
        "filePath", "full_path", "directory",
    ],
    "technology.file.file_extension": [
        "extension", "ext", "file_ext", "Extension", "FILE_EXT",
        "file_extension", "file_type",
    ],
    "technology.crypto.sha256": [
        "hash", "sha256", "SHA256", "checksum", "digest", "file_hash",
        "content_hash",
    ],
    "technology.crypto.md5": [
        "md5", "MD5", "hash", "md5_hash", "checksum", "md5sum",
        "file_md5",
    ],
    "technology.version.semver": [
        "version", "Version", "VERSION", "ver", "release", "app_version",
        "api_version", "semver",
    ],
    # ─── Representation domain ────────────────────────────────────
    "representation.text.free_text": [
        "description", "text", "comment", "notes", "Description",
        "TEXT", "remarks", "body", "content", "message",
    ],
    "representation.text.sentence": [
        "sentence", "text", "message", "comment", "Sentence",
        "description", "note",
    ],
    "representation.text.entity_name": [
        "name", "entity", "company", "organization", "Name",
        "org_name", "entity_name", "business_name", "label",
    ],
    "representation.numeric.integer": [
        "count", "quantity", "number", "amount", "Count",
        "QUANTITY", "num", "total", "value", "size",
    ],
    "representation.numeric.decimal_number": [
        "value", "amount", "score", "rate", "Value",
        "AMOUNT", "decimal", "measurement", "reading",
    ],
    "representation.numeric.percentage": [
        "percentage", "pct", "percent", "rate", "Percentage",
        "PCT", "ratio", "share", "completion_rate",
    ],
    "representation.boolean.boolean": [
        "active", "enabled", "is_active", "flag", "Active",
        "ENABLED", "status", "boolean", "is_valid", "approved",
    ],
    "representation.encoding.json_string": [
        "data", "json", "payload", "config", "Data",
        "JSON", "metadata", "properties", "attributes",
    ],
    "representation.encoding.base64": [
        "encoded", "base64", "data", "content", "Base64",
        "BASE64", "encoded_data", "blob", "payload",
    ],
    "representation.color.hex_color": [
        "color", "colour", "hex_color", "Color", "COLOR",
        "bg_color", "text_color", "fill_color", "hex",
    ],
    # ─── Container domain ─────────────────────────────────────────
    "container.tabular.csv_row": [
        "row", "data", "record", "line", "csv_row", "Row",
    ],
    "container.tabular.tsv_row": [
        "row", "data", "record", "line", "tsv_row", "Row",
    ],
    "container.markup.html_fragment": [
        "html", "content", "body", "markup", "HTML",
        "html_content", "snippet", "template",
    ],
}


def _generate_fallback_header_variations(type_key):
    """Generate header variations from the type key's leaf name.

    Used for types not in the curated HEADER_VARIATIONS mapping.
    Produces underscore, camelCase, space, and abbreviated forms.
    """
    # Extract leaf: "identity.person.email" -> "email"
    leaf = type_key.rsplit(".", 1)[-1]

    variations = [leaf]

    # Underscore form is the leaf itself (e.g., "zip_code")
    # Title case
    variations.append(leaf.replace("_", " ").title().replace(" ", ""))

    # UPPER_CASE
    variations.append(leaf.upper())

    # Space-separated title case
    if "_" in leaf:
        variations.append(leaf.replace("_", " ").title())
        # CamelCase
        parts = leaf.split("_")
        variations.append(parts[0] + "".join(p.capitalize() for p in parts[1:]))
        # First word only (abbreviation)
        variations.append(parts[0])

    # Capitalize first letter
    variations.append(leaf.capitalize())

    # Kebab-case
    variations.append(leaf.replace("_", "-"))

    # Deduplicate while preserving order
    seen = set()
    unique = []
    for v in variations:
        if v not in seen:
            seen.add(v)
            unique.append(v)

    return unique


def get_header_for_type(type_key, rng):
    """Return a random realistic header name for a given type key.

    Uses curated variations when available, falls back to programmatic
    generation from the leaf name.
    """
    variations = HEADER_VARIATIONS.get(type_key)
    if not variations:
        variations = _generate_fallback_header_variations(type_key)
    return rng.choice(variations)


# ═══════════════════════════════════════════════════════════════════════════════
# Data loading (reused from prepare_spike_data.py)
# ═══════════════════════════════════════════════════════════════════════════════


def load_taxonomy_types(finetype_bin):
    """Get the full list of 250 taxonomy types from finetype."""
    result = subprocess.run(
        [finetype_bin, "taxonomy", "--output", "csv"],
        capture_output=True,
        text=True,
        check=True,
    )
    types = set()
    reader = csv.DictReader(result.stdout.splitlines())
    for row in reader:
        key = row.get("key", "").strip()
        if key:
            types.add(key)
    return types


def load_label_remap(remap_path):
    """Load label remap table from JSON file.

    Returns: dict mapping non-canonical labels to canonical taxonomy equivalents.
    """
    if not os.path.exists(remap_path):
        print(f"  No label remap file at {remap_path}", file=sys.stderr)
        return {}

    with open(remap_path) as f:
        remap = json.load(f)

    # Remove comment keys
    remap.pop("_comment", None)
    return remap


def load_distilled_columns(distilled_path, min_values, label_remap=None):
    """Load distilled data as columns (groups of values per type).

    Each column is a (values, header) tuple where header is the original
    column_name from the Sherlock corpus.

    Also returns a flat ordered list of (label, values, header) for all
    qualifying rows, preserving CSV row order. This is used for proximity-
    based table grouping in v3 format (adjacent rows in the Sherlock corpus
    likely came from the same source table).

    Args:
        label_remap: dict mapping non-canonical labels to canonical equivalents.

    Returns:
        columns_by_type: dict[str, list[tuple[list[str], str]]] — each type has
                         a list of (values, header) tuples.
        ordered_columns: list[tuple[str, list[str], str]] — all qualifying
                         columns in CSV row order as (label, values, header).
        stats: dict with counts for logging
    """
    columns_by_type = defaultdict(list)
    ordered_columns = []
    label_remap = label_remap or {}
    stats = {
        "total_rows": 0,
        "qualifying_rows": 0,
        "sparse_rows": 0,
        "parse_errors": 0,
        "empty_label": 0,
        "excluded_column_types": 0,
        "remapped_labels": 0,
        "total_values": 0,
    }

    opener = gzip.open if distilled_path.endswith(".gz") else open
    with opener(distilled_path, "rt", newline="") as f:
        reader = csv.DictReader(f)
        for row in reader:
            stats["total_rows"] += 1
            label = row.get("final_label", "").strip()

            if not label:
                stats["empty_label"] += 1
                continue

            # Apply label remap before any filtering
            if label in label_remap:
                label = label_remap[label]
                stats["remapped_labels"] += 1

            # Skip column-level types
            if label in COLUMN_LEVEL_TYPES:
                stats["excluded_column_types"] += 1
                continue

            try:
                vals = json.loads(row.get("sample_values", "[]"))
            except (json.JSONDecodeError, TypeError):
                stats["parse_errors"] += 1
                continue

            n = len(vals) if isinstance(vals, list) else 0
            if n < min_values:
                stats["sparse_rows"] += 1
                continue

            stats["qualifying_rows"] += 1
            # Keep as a column (list of values) with its original header
            clean_vals = [str(v).strip() for v in vals if str(v).strip()]
            header = row.get("column_name", "").strip()
            if len(clean_vals) >= min_values:
                columns_by_type[label].append((clean_vals, header))
                ordered_columns.append((label, clean_vals, header))
                stats["total_values"] += len(clean_vals)

    return dict(columns_by_type), ordered_columns, stats


def generate_synthetic_columns(finetype_bin, synthetic_columns_per_type, seed, min_values):
    """Generate synthetic training data via finetype generate, grouped as columns.

    Each column gets a realistic header variation (not the type key) to prevent
    the model from cheating on perfect type-key headers.

    Args:
        synthetic_columns_per_type: target number of columns per type. Each column
            has ~100 values, so we generate synthetic_columns_per_type * 100 values
            per type via `finetype generate --samples`.

    Returns: dict[str, list[tuple[list[str], str]]] — each type has a list of
             (values, header) tuples
    """
    # Generate enough values to produce the target number of columns
    # Each column is ~100 values, so we need N * 100 values per type
    values_per_type = synthetic_columns_per_type * 100

    rng = random.Random(seed + 7)  # Offset seed for header variation

    with tempfile.NamedTemporaryFile(suffix=".ndjson", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        subprocess.run(
            [
                finetype_bin,
                "generate",
                "--samples",
                str(values_per_type),
                "--seed",
                str(seed),
                "--output",
                tmp_path,
            ],
            capture_output=True,
            text=True,
            check=True,
        )

        values_by_type = defaultdict(list)
        with open(tmp_path) as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                rec = json.loads(line)
                values_by_type[rec["classification"]].append(rec["text"])

        # Convert to columns: chunk each type's values into column-sized groups
        # Each column gets a random realistic header variation
        columns_by_type = {}
        for type_key, values in values_by_type.items():
            if len(values) < min_values:
                continue
            # Create columns of ~100 values each (matching typical column sizes)
            col_size = min(100, len(values))
            columns = []
            for i in range(0, len(values), col_size):
                chunk = values[i : i + col_size]
                if len(chunk) >= min_values:
                    header = get_header_for_type(type_key, rng)
                    columns.append((chunk, header))
            columns_by_type[type_key] = columns

        return columns_by_type
    finally:
        os.unlink(tmp_path)


def blend_columns(distilled, synthetic, ratio_distilled, samples_per_type, rng):
    """Blend distilled and synthetic column data per-type with capping.

    Each column is a (values, header) tuple.

    samples_per_type: target number of columns per type
    ratio_distilled: float 0.0-1.0 (e.g. 0.5 means 50% distilled)
    Returns: dict[str, list[tuple[list[str], str]]]
    """
    all_types = set(distilled.keys()) | set(synthetic.keys())
    blended = {}

    for type_key in sorted(all_types):
        d_cols = distilled.get(type_key, [])
        s_cols = synthetic.get(type_key, [])

        target_d = int(samples_per_type * ratio_distilled)
        target_s = samples_per_type - target_d

        # Cap at available, no oversampling. Fill remainder from other source.
        if len(d_cols) < target_d:
            actual_d = len(d_cols)
            actual_s = min(len(s_cols), samples_per_type - actual_d)
        elif len(s_cols) < target_s:
            actual_s = len(s_cols)
            actual_d = min(len(d_cols), samples_per_type - actual_s)
        else:
            actual_d = target_d
            actual_s = target_s

        picked_d = rng.sample(d_cols, actual_d) if actual_d <= len(d_cols) else d_cols[:]
        picked_s = rng.sample(s_cols, actual_s) if actual_s <= len(s_cols) else s_cols[:]

        combined = picked_d + picked_s
        rng.shuffle(combined)
        if combined:
            blended[type_key] = combined

    return blended


# ═══════════════════════════════════════════════════════════════════════════════
# Table assembly (v3)
# ═══════════════════════════════════════════════════════════════════════════════


def assemble_synthetic_tables(synthetic_columns_by_type, rng, taxonomy_types,
                              noise_min=1, noise_max=3):
    """Assemble synthetic columns into table groups using TABLE_TEMPLATES.

    Each assembled table:
    1. Picks a template
    2. For each type in template: picks one synthetic column (if available)
    3. Adds 1-3 random columns from other domains (controlled noise)
    4. Final table size: 5-15 columns

    Returns: list of TableGroup, each being:
        [(label, values, header), ...] — the columns in the table
    """
    # Build a pool of available synthetic columns per type (as a queue)
    pool = {}
    for type_key, cols in synthetic_columns_by_type.items():
        pool[type_key] = list(cols)  # copy so we can pop

    # All available types for noise injection (excluding column-level types)
    noise_candidates = [t for t in taxonomy_types if t not in COLUMN_LEVEL_TYPES]

    tables = []
    template_names = list(TABLE_TEMPLATES.keys())

    # Keep cycling through templates until we run low on synthetic columns
    max_tables = sum(len(c) for c in pool.values()) // 3  # rough upper bound
    table_count = 0

    while table_count < max_tables:
        template_name = template_names[table_count % len(template_names)]
        template_types = TABLE_TEMPLATES[template_name]

        table_columns = []
        for type_key in template_types:
            if type_key in pool and pool[type_key]:
                values, header = pool[type_key].pop()
                table_columns.append((type_key, values, header))

        if len(table_columns) < 2:
            # Not enough columns for this template, skip
            table_count += 1
            # Check if we've exhausted all pools
            total_remaining = sum(len(c) for c in pool.values())
            if total_remaining < 3:
                break
            continue

        # Add 1-3 noise columns from random types
        n_noise = rng.randint(noise_min, noise_max)
        noise_types = rng.sample(noise_candidates,
                                 min(n_noise * 3, len(noise_candidates)))
        added_noise = 0
        for nt in noise_types:
            if added_noise >= n_noise:
                break
            if nt in pool and pool[nt]:
                values, header = pool[nt].pop()
                table_columns.append((nt, values, header))
                added_noise += 1

        # Enforce 5-15 column limit
        if len(table_columns) > 15:
            table_columns = table_columns[:15]

        if len(table_columns) >= 2:
            rng.shuffle(table_columns)
            tables.append(table_columns)

        table_count += 1

    # Any remaining columns that didn't fit into templates: group into
    # random tables of 5-10 columns
    remaining = []
    for type_key, cols in pool.items():
        for values, header in cols:
            remaining.append((type_key, values, header))
    rng.shuffle(remaining)

    i = 0
    while i < len(remaining):
        left = len(remaining) - i
        if left < 2:
            # Append remaining to last group
            if tables and left > 0:
                tables[-1].extend(remaining[i:])
            break
        group_size = rng.randint(min(5, left), min(10, left))
        tables.append(remaining[i:i + group_size])
        i += group_size

    return tables


def group_distilled_by_proximity(ordered_columns, rng, group_min=5, group_max=15):
    """Group distilled columns by proximity in CSV row order.

    Adjacent rows in the Sherlock corpus likely came from the same source
    table, so we use proximity-based grouping as a fallback when source_file
    is unavailable.

    Strategy: walk through ordered_columns and cut groups at random sizes
    between group_min and group_max. This preserves the adjacency signal
    while producing variable-sized groups.

    Returns: list of TableGroup, each being:
        [(label, values, header), ...] — the columns in the table
    """
    if not ordered_columns:
        return []

    tables = []
    i = 0
    while i < len(ordered_columns):
        group_size = rng.randint(group_min, group_max)
        group = ordered_columns[i:i + group_size]
        if len(group) >= 2:
            tables.append(group)
        else:
            # Append single remaining column to last group if possible
            if tables:
                tables[-1].extend(group)
        i += group_size

    return tables


# ═══════════════════════════════════════════════════════════════════════════════
# Feature extraction
# ═══════════════════════════════════════════════════════════════════════════════


def extract_features(finetype_bin, values, header=None):
    """Call `finetype extract-features` to get feature vectors for a column.

    Returns: dict with 'char', 'embed', 'stats', 'header_features' arrays,
             or None on failure.
    """
    cmd = [finetype_bin, "extract-features", "--json"]
    if header:
        cmd.extend(["--header", header])

    try:
        result = subprocess.run(
            cmd,
            input=json.dumps(values),
            capture_output=True,
            text=True,
            timeout=60,
        )
        if result.returncode != 0:
            return None
        return json.loads(result.stdout.strip())
    except (subprocess.TimeoutExpired, json.JSONDecodeError, Exception) as e:
        print(f"  Warning: feature extraction failed: {e}", file=sys.stderr)
        return None


# ═══════════════════════════════════════════════════════════════════════════════
# Binary file I/O — FTMB v2 and v3
# ═══════════════════════════════════════════════════════════════════════════════


def write_ftmb_v2(path, records):
    """Write records to a .ftmb v2 binary file.

    records: list of (label, char_features, embed_features, stats_features, header_features)

    Header (28 bytes):
        magic: b"FTMB" (4 bytes)
        version: uint32 = 2
        n_records: uint64
        char_dim: uint16 = 960
        embed_dim: uint16 = 512
        stats_dim: uint16 = 27
        header_dim: uint16 = 128

    Each record:
        label_len: uint16
        label: bytes (UTF-8 type key)
        char_features: 960 x float32 (little-endian)
        embed_features: 512 x float32 (little-endian)
        stats_features: 27 x float32 (little-endian)
        header_features: 128 x float32 (little-endian)
    """
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        # Header (28 bytes)
        f.write(MAGIC)
        f.write(struct.pack("<I", VERSION_V2))
        f.write(struct.pack("<Q", len(records)))
        f.write(struct.pack("<HHHH", CHAR_DIM, EMBED_DIM, STATS_DIM, HEADER_DIM))

        for label, char_feat, embed_feat, stats_feat, header_feat in records:
            label_bytes = label.encode("utf-8")
            f.write(struct.pack("<H", len(label_bytes)))
            f.write(label_bytes)
            f.write(struct.pack(f"<{CHAR_DIM}f", *char_feat))
            f.write(struct.pack(f"<{EMBED_DIM}f", *embed_feat))
            f.write(struct.pack(f"<{STATS_DIM}f", *stats_feat))
            f.write(struct.pack(f"<{HEADER_DIM}f", *header_feat))


def write_ftmb_v3(path, table_groups):
    """Write table-grouped records to a .ftmb v3 binary file.

    table_groups: list of TableGroupRecord, each being:
        {
            "sibling_headers": [str, ...],  -- all headers in the table
            "records": [
                {
                    "label": str,
                    "column_index": int,  -- index into sibling_headers
                    "char": [float, ...],
                    "embed": [float, ...],
                    "stats": [float, ...],
                    "header": [float, ...],
                },
                ...
            ]
        }

    File layout (28-byte header):
        4B  magic "FTMB"
        4B  version (3, LE u32)
        8B  n_records (total, LE u64)
        2B  char_dim (u16) = 960
        2B  embed_dim (u16) = 512
        2B  stats_dim (u16) = 27
        2B  header_dim (u16) = 128
        2B  n_groups (u16) = number of table groups
        2B  reserved (0)

    Per table group:
        2B  n_columns (u16) — records in this group
        2B  n_sibling_headers (u16) — number of sibling header strings
        For each sibling header:
            2B  header_len (u16)
            *B  header_bytes (UTF-8)
        For each record (n_columns times):
            2B  label_len (u16)
            *B  label_bytes
            2B  column_index (u16) — index into this group's sibling_headers
            char_dim*4B  char_features (f32 LE)
            embed_dim*4B embed_features (f32 LE)
            stats_dim*4B stats_features (f32 LE)
            header_dim*4B header_features (f32 LE, raw Model2Vec)
    """
    n_records = sum(len(g["records"]) for g in table_groups)
    n_groups = len(table_groups)

    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        # File header (28 bytes)
        f.write(MAGIC)
        f.write(struct.pack("<I", VERSION_V3))
        f.write(struct.pack("<Q", n_records))
        f.write(struct.pack("<HHHH", CHAR_DIM, EMBED_DIM, STATS_DIM, HEADER_DIM))
        f.write(struct.pack("<HH", n_groups, 0))  # n_groups + reserved

        # Table groups
        for group in table_groups:
            sibling_headers = group["sibling_headers"]
            records = group["records"]

            f.write(struct.pack("<HH", len(records), len(sibling_headers)))

            # Write sibling header strings
            for hdr in sibling_headers:
                hdr_bytes = hdr.encode("utf-8")
                f.write(struct.pack("<H", len(hdr_bytes)))
                f.write(hdr_bytes)

            # Write records
            for rec in records:
                label_bytes = rec["label"].encode("utf-8")
                f.write(struct.pack("<H", len(label_bytes)))
                f.write(label_bytes)
                f.write(struct.pack("<H", rec["column_index"]))
                f.write(struct.pack(f"<{CHAR_DIM}f", *rec["char"]))
                f.write(struct.pack(f"<{EMBED_DIM}f", *rec["embed"]))
                f.write(struct.pack(f"<{STATS_DIM}f", *rec["stats"]))
                f.write(struct.pack(f"<{HEADER_DIM}f", *rec["header"]))


def read_ftmb(path):
    """Read a .ftmb binary file (v1, v2, or v3).

    v1: returns list of (label, char_feat, embed_feat, stats_feat) tuples
    v2: returns list of (label, char_feat, embed_feat, stats_feat, header_feat) tuples
    v3: returns list of (label, char_feat, embed_feat, stats_feat, header_feat) tuples
        (flattened from table groups — use read_ftmb_v3 for grouped data)
    """
    with open(path, "rb") as f:
        magic = f.read(4)
        assert magic == MAGIC, f"Bad magic: {magic}"
        (version,) = struct.unpack("<I", f.read(4))
        assert version in (1, 2, 3), f"Unknown version: {version}"

        if version == 3:
            # Read v3 grouped format, flatten to record list
            groups = _read_ftmb_v3_groups(f)
            records = []
            for group in groups:
                for rec in group["records"]:
                    records.append((
                        rec["label"], rec["char"], rec["embed"],
                        rec["stats"], rec["header"],
                    ))
            return records

        (n_records,) = struct.unpack("<Q", f.read(8))
        char_dim, embed_dim, stats_dim = struct.unpack("<HHH", f.read(6))

        if version == 1:
            _padding = f.read(2)
            header_dim = 0
        else:
            (header_dim,) = struct.unpack("<H", f.read(2))

        records = []
        for _ in range(n_records):
            (label_len,) = struct.unpack("<H", f.read(2))
            label = f.read(label_len).decode("utf-8")
            char_feat = list(struct.unpack(f"<{char_dim}f", f.read(char_dim * 4)))
            embed_feat = list(struct.unpack(f"<{embed_dim}f", f.read(embed_dim * 4)))
            stats_feat = list(struct.unpack(f"<{stats_dim}f", f.read(stats_dim * 4)))
            if header_dim > 0:
                header_feat = list(struct.unpack(f"<{header_dim}f", f.read(header_dim * 4)))
                records.append((label, char_feat, embed_feat, stats_feat, header_feat))
            else:
                records.append((label, char_feat, embed_feat, stats_feat))

        return records


def _read_ftmb_v3_groups(f):
    """Read v3 table groups from an open file (after magic+version already read).

    Returns: list of group dicts, each with 'sibling_headers' and 'records'.
    """
    (n_records,) = struct.unpack("<Q", f.read(8))
    char_dim, embed_dim, stats_dim, header_dim = struct.unpack("<HHHH", f.read(8))
    n_groups, _reserved = struct.unpack("<HH", f.read(4))

    groups = []
    for _ in range(n_groups):
        n_columns, n_sibling_headers = struct.unpack("<HH", f.read(4))

        sibling_headers = []
        for _ in range(n_sibling_headers):
            (hdr_len,) = struct.unpack("<H", f.read(2))
            hdr = f.read(hdr_len).decode("utf-8")
            sibling_headers.append(hdr)

        records = []
        for _ in range(n_columns):
            (label_len,) = struct.unpack("<H", f.read(2))
            label = f.read(label_len).decode("utf-8")
            (column_index,) = struct.unpack("<H", f.read(2))
            char_feat = list(struct.unpack(f"<{char_dim}f", f.read(char_dim * 4)))
            embed_feat = list(struct.unpack(f"<{embed_dim}f", f.read(embed_dim * 4)))
            stats_feat = list(struct.unpack(f"<{stats_dim}f", f.read(stats_dim * 4)))
            header_feat = list(struct.unpack(f"<{header_dim}f", f.read(header_dim * 4)))
            records.append({
                "label": label,
                "column_index": column_index,
                "char": char_feat,
                "embed": embed_feat,
                "stats": stats_feat,
                "header": header_feat,
            })

        groups.append({
            "sibling_headers": sibling_headers,
            "records": records,
        })

    return groups


# ═══════════════════════════════════════════════════════════════════════════════
# Main
# ═══════════════════════════════════════════════════════════════════════════════


def run_preflight_check(finetype_bin, blended, num_types=10, cols_per_type=5):
    """Run a quick feature extraction on a sample to catch failures early.

    Returns True if preflight passes, False if any extraction fails.
    """
    print(f"\nPreflight: extracting features for {num_types} types x {cols_per_type} cols...")
    sample_types = list(sorted(blended.keys()))[:num_types]
    total = 0
    errors = 0
    start = time.time()

    for type_key in sample_types:
        cols = blended[type_key][:cols_per_type]
        for col_values, header in cols:
            features = extract_features(finetype_bin, col_values, header=header)
            total += 1
            if features is None:
                errors += 1
                print(f"  FAIL: {type_key} ({len(col_values)} values)", file=sys.stderr)
            elif "header_features" not in features:
                errors += 1
                print(f"  FAIL: {type_key} missing header_features", file=sys.stderr)

    elapsed = time.time() - start
    if errors > 0:
        print(f"  Preflight FAILED: {errors}/{total} extractions failed ({elapsed:.1f}s)")
        return False
    else:
        rate = total / elapsed if elapsed > 0 else 0
        print(f"  Preflight PASSED: {total}/{total} OK ({elapsed:.1f}s, {rate:.1f} cols/sec)")
        return True


def _extract_and_validate(finetype_bin, type_key, col_values, header):
    """Extract features for a single column and validate dimensions.

    Returns: (type_key, char, embed, stats, header_feat) or None on failure.
    """
    features = extract_features(finetype_bin, col_values, header=header)
    if features is None:
        return None

    char_feat = features.get("char", [0.0] * CHAR_DIM)
    embed_feat = features.get("embed", [0.0] * EMBED_DIM)
    stats_feat = features.get("stats", [0.0] * STATS_DIM)
    header_feat = features.get("header_features", [0.0] * HEADER_DIM)

    # Validate dimensions
    for name, feat, expected in [
        ("char", char_feat, CHAR_DIM),
        ("embed", embed_feat, EMBED_DIM),
        ("stats", stats_feat, STATS_DIM),
        ("header", header_feat, HEADER_DIM),
    ]:
        if len(feat) != expected:
            print(f"  Warning: {type_key} {name} dim {len(feat)} != {expected}",
                  file=sys.stderr)
            return None

    return type_key, char_feat, embed_feat, stats_feat, header_feat


def _run_v2_pipeline(blended, finetype_bin, output_path, workers, start_time):
    """Run v2 flat extraction and write pipeline.

    Returns: (records, errors) tuple.
    """
    total_blended = sum(len(cols) for cols in blended.values())
    print(f"\nExtracting features for {total_blended} columns (workers={workers})...")

    work_items = []
    for type_key in sorted(blended.keys()):
        for col_values, header in blended[type_key]:
            work_items.append((type_key, col_values, header))

    records = []
    errors = 0

    def process_item(item):
        type_key, col_values, header = item
        return _extract_and_validate(finetype_bin, type_key, col_values, header)

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(process_item, item): item for item in work_items}
        for i, future in enumerate(as_completed(futures)):
            result = future.result()

            if result is None:
                errors += 1
                continue

            records.append(result)

            # Progress
            done = i + 1
            if done % 100 == 0 or done == len(work_items):
                elapsed = time.time() - start_time
                rate = done / elapsed if elapsed > 0 else 0
                eta = (len(work_items) - done) / rate if rate > 0 else 0
                print(
                    f"  [{done}/{len(work_items)}] {rate:.1f} cols/sec, "
                    f"ETA {eta/60:.1f}min, {errors} errors",
                    file=sys.stderr,
                )

    print(f"\nWriting {len(records)} records to {output_path}...")
    write_ftmb_v2(output_path, records)
    return records, errors


def _run_v3_pipeline(synthetic, ordered_distilled, finetype_bin, output_path,
                     workers, ratio_distilled, samples_per_type,
                     taxonomy_types, rng, start_time):
    """Run v3 table-grouped extraction and write pipeline.

    1. Assemble synthetic columns into table groups
    2. Group distilled columns by proximity
    3. Extract features per-table-group (preserving group structure)
    4. Interleave synthetic and distilled groups
    5. Write FTMB v3

    Returns: (n_records, n_groups, errors) tuple.
    """
    # ─── Assemble synthetic tables ──────────────────────────────
    print("\nAssembling synthetic columns into table groups...")
    synthetic_tables = assemble_synthetic_tables(synthetic, rng, taxonomy_types)
    syn_cols = sum(len(t) for t in synthetic_tables)
    print(f"  {len(synthetic_tables)} synthetic tables ({syn_cols} columns)")

    # ─── Group distilled columns by proximity ────────────────────
    print("Grouping distilled columns by CSV proximity...")
    # Note: source_file is empty for all Sherlock rows, so we use proximity-
    # based grouping: adjacent rows in the CSV likely came from the same
    # source table in the Sherlock corpus.
    distilled_tables = group_distilled_by_proximity(ordered_distilled, rng)
    dist_cols = sum(len(t) for t in distilled_tables)
    print(f"  {len(distilled_tables)} distilled tables ({dist_cols} columns)")

    # ─── Apply ratio cap ─────────────────────────────────────────
    # Target: ratio_distilled of total groups should be distilled
    total_target = min(
        len(synthetic_tables) + len(distilled_tables),
        int(samples_per_type * len(taxonomy_types) / 8),  # rough cap
    )
    target_dist = int(total_target * ratio_distilled)
    target_syn = total_target - target_dist

    if len(distilled_tables) > target_dist:
        rng.shuffle(distilled_tables)
        distilled_tables = distilled_tables[:target_dist]
    if len(synthetic_tables) > target_syn:
        rng.shuffle(synthetic_tables)
        synthetic_tables = synthetic_tables[:target_syn]

    all_tables = distilled_tables + synthetic_tables
    rng.shuffle(all_tables)

    total_columns = sum(len(t) for t in all_tables)
    print(f"\n  Total: {len(all_tables)} table groups ({total_columns} columns)")

    # ─── Extract features per table group ──────────────────────
    print(f"\nExtracting features for {total_columns} columns in "
          f"{len(all_tables)} groups (workers={workers})...")

    # We extract features for all columns, then assemble into v3 groups.
    # Build a flat list of (group_idx, col_idx, label, values, header) items
    work_items = []
    for g_idx, table in enumerate(all_tables):
        for c_idx, (label, values, header) in enumerate(table):
            work_items.append((g_idx, c_idx, label, values, header))

    # Extract features in parallel
    # Results keyed by (group_idx, col_idx) -> extracted features
    results_map = {}
    errors = 0

    def process_item(item):
        g_idx, c_idx, label, values, header = item
        result = _extract_and_validate(finetype_bin, label, values, header)
        return g_idx, c_idx, header, result

    with ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(process_item, item): item for item in work_items}
        for i, future in enumerate(as_completed(futures)):
            g_idx, c_idx, header, result = future.result()

            if result is None:
                errors += 1
            else:
                results_map[(g_idx, c_idx)] = (header, result)

            # Progress
            done = i + 1
            if done % 100 == 0 or done == len(work_items):
                elapsed = time.time() - start_time
                rate = done / elapsed if elapsed > 0 else 0
                eta = (len(work_items) - done) / rate if rate > 0 else 0
                print(
                    f"  [{done}/{len(work_items)}] {rate:.1f} cols/sec, "
                    f"ETA {eta/60:.1f}min, {errors} errors",
                    file=sys.stderr,
                )

    # ─── Assemble v3 table groups ──────────────────────────────
    print("\nAssembling v3 table groups...")
    v3_groups = []
    total_records = 0

    for g_idx, table in enumerate(all_tables):
        # Collect all successfully extracted columns for this group
        group_columns = []
        for c_idx in range(len(table)):
            if (g_idx, c_idx) in results_map:
                header, result = results_map[(g_idx, c_idx)]
                label, char_f, embed_f, stats_f, header_f = result
                group_columns.append((label, header, char_f, embed_f, stats_f, header_f))

        if len(group_columns) < 2:
            # Skip groups with fewer than 2 successful columns
            continue

        # Build sibling headers list (all headers in this group)
        sibling_headers = [col[1] for col in group_columns]

        # Build records with column_index pointing into sibling_headers
        records = []
        for col_idx, (label, header, char_f, embed_f, stats_f, header_f) in enumerate(group_columns):
            records.append({
                "label": label,
                "column_index": col_idx,
                "char": char_f,
                "embed": embed_f,
                "stats": stats_f,
                "header": header_f,
            })

        v3_groups.append({
            "sibling_headers": sibling_headers,
            "records": records,
        })
        total_records += len(records)

    # ─── Write v3 binary file ──────────────────────────────────
    print(f"\nWriting {total_records} records in {len(v3_groups)} groups to {output_path}...")
    write_ftmb_v3(output_path, v3_groups)

    return total_records, len(v3_groups), errors


def main():
    args = sys.argv[1:]

    # Defaults
    distilled_path = "output/distillation-v3/sherlock_distilled.csv.gz"
    finetype_bin = "./target/release/finetype"
    output_path = "output/multibranch-training/blend-50-50.ftmb"
    label_remap_path = "data/label_remap.json"
    samples_per_type = 1200
    synthetic_columns_per_type = 1200
    ratio_distilled = 0.5
    min_values = 5
    seed = 42
    workers = 4
    dry_run = False
    skip_preflight = False
    output_format = "v3"

    i = 0
    while i < len(args):
        if args[i] == "--distilled":
            distilled_path = args[i + 1]
            i += 2
        elif args[i] == "--finetype":
            finetype_bin = args[i + 1]
            i += 2
        elif args[i] == "--output":
            output_path = args[i + 1]
            i += 2
        elif args[i] == "--label-remap":
            label_remap_path = args[i + 1]
            i += 2
        elif args[i] == "--samples-per-type":
            samples_per_type = int(args[i + 1])
            i += 2
        elif args[i] == "--synthetic-columns":
            synthetic_columns_per_type = int(args[i + 1])
            i += 2
        elif args[i] == "--ratio-distilled":
            ratio_distilled = float(args[i + 1])
            i += 2
        elif args[i] == "--min-values":
            min_values = int(args[i + 1])
            i += 2
        elif args[i] == "--seed":
            seed = int(args[i + 1])
            i += 2
        elif args[i] == "--workers":
            workers = int(args[i + 1])
            i += 2
        elif args[i] == "--format":
            output_format = args[i + 1]
            if output_format not in ("v2", "v3"):
                print(f"ERROR: --format must be 'v2' or 'v3', got '{output_format}'",
                      file=sys.stderr)
                sys.exit(1)
            i += 2
        elif args[i] == "--dry-run":
            dry_run = True
            i += 1
        elif args[i] == "--skip-preflight":
            skip_preflight = True
            i += 1
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    version = VERSION_V3 if output_format == "v3" else VERSION_V2
    rng = random.Random(seed)

    print(f"FTMB output format: {output_format}")

    # ─── Load taxonomy ─────────────────────────────────────────────
    print("Loading taxonomy...")
    taxonomy_types = load_taxonomy_types(finetype_bin)
    print(f"  {len(taxonomy_types)} taxonomy types")

    # ─── Load label remap ─────────────────────────────────────────
    print(f"\nLoading label remap from {label_remap_path}...")
    label_remap = load_label_remap(label_remap_path)
    if label_remap:
        print(f"  {len(label_remap)} remap entries loaded")
    else:
        print("  No remap table (using labels as-is)")

    # ─── Load distilled data ───────────────────────────────────────
    print(f"\nLoading distilled data (min_values={min_values})...")
    distilled, ordered_distilled, d_stats = load_distilled_columns(
        distilled_path, min_values, label_remap
    )
    print(f"  {d_stats['total_rows']} total rows")
    print(f"  {d_stats['qualifying_rows']} qualifying rows")
    print(f"  {d_stats['sparse_rows']} sparse rows (skipped)")
    print(f"  {d_stats['parse_errors']} parse errors (skipped)")
    print(f"  {d_stats['empty_label']} empty labels (skipped)")
    print(f"  {d_stats['excluded_column_types']} column-level types (excluded)")
    print(f"  {d_stats['remapped_labels']} labels remapped to canonical")
    total_d_cols = sum(len(cols) for cols in distilled.values())
    print(f"  {total_d_cols} columns across {len(distilled)} types")
    print(f"  {d_stats['total_values']} individual values")

    # ─── Generate synthetic data ───────────────────────────────────
    print(f"\nGenerating synthetic data ({synthetic_columns_per_type} columns/type, {synthetic_columns_per_type * 100} values/type)...")
    synthetic = generate_synthetic_columns(finetype_bin, synthetic_columns_per_type, seed, min_values)
    total_s_cols = sum(len(cols) for cols in synthetic.values())
    print(f"  {total_s_cols} columns across {len(synthetic)} types")

    # ─── Validate labels ───────────────────────────────────────────
    bad_distilled = set(distilled.keys()) - taxonomy_types
    if bad_distilled:
        print(f"\n  WARNING: {len(bad_distilled)} distilled types not in taxonomy:")
        for t in sorted(bad_distilled):
            print(f"    {t} ({len(distilled[t])} columns)")
        for t in bad_distilled:
            del distilled[t]
        # Also filter ordered_distilled
        ordered_distilled = [
            (label, vals, hdr) for label, vals, hdr in ordered_distilled
            if label not in bad_distilled
        ]

    # ─── Blend (for v2) or assemble tables (for v3) ──────────────
    if output_format == "v2":
        print(f"\nBlending data ({ratio_distilled:.0%} distilled, {1-ratio_distilled:.0%} synthetic)...")
        blended = blend_columns(distilled, synthetic, ratio_distilled, samples_per_type, rng)
        total_blended = sum(len(cols) for cols in blended.values())
        print(f"  {total_blended} columns across {len(blended)} types")
    else:
        # For v3, we do blending implicitly via table assembly + ratio cap
        blended = blend_columns(distilled, synthetic, ratio_distilled, samples_per_type, rng)
        total_blended = sum(len(cols) for cols in blended.values())

    # ─── Type coverage summary ─────────────────────────────────────
    distilled_types = set(distilled.keys())
    synthetic_types = set(synthetic.keys())
    blended_types = set(blended.keys())
    missing_types = taxonomy_types - blended_types

    print(f"\n{'='*60}")
    print(f"Type coverage:")
    print(f"  Taxonomy:   {len(taxonomy_types)} types")
    print(f"  Distilled:  {len(distilled_types)} types ({total_d_cols} columns)")
    print(f"  Synthetic:  {len(synthetic_types)} types ({total_s_cols} columns)")
    print(f"  Blended:    {len(blended_types)} types ({total_blended} columns)")
    print(f"  Missing:    {len(missing_types)} types (no source)")
    if missing_types and len(missing_types) <= 10:
        for t in sorted(missing_types):
            print(f"    {t}")
    print(f"{'='*60}")

    if dry_run:
        print(f"\n[DRY RUN] Would extract features for {total_blended} columns")
        print(f"  Output: {output_path}")
        print(f"  FTMB version: {version}")
        if output_format == "v3":
            print(f"  Table grouping: synthetic (templates) + distilled (proximity)")
        total_dim = CHAR_DIM + EMBED_DIM + STATS_DIM + HEADER_DIM
        print(f"  Record size: {2 + 30 + total_dim*4} bytes (avg)")
        est_size_mb = total_blended * total_dim * 4 / (1024 * 1024)
        print(f"  Estimated file size: ~{est_size_mb:.0f} MB")
        return

    # ─── Preflight extraction check ──────────────────────────────
    if not skip_preflight:
        if not run_preflight_check(finetype_bin, blended):
            print("\nAborting: preflight extraction failed.", file=sys.stderr)
            sys.exit(1)

    start_time = time.time()

    # ─── Run format-specific pipeline ─────────────────────────────
    if output_format == "v2":
        records, errors = _run_v2_pipeline(
            blended, finetype_bin, output_path, workers, start_time
        )
        n_records = len(records)
        n_groups = 0
        type_counts = Counter(r[0] for r in records)
    else:
        n_records, n_groups, errors = _run_v3_pipeline(
            synthetic, ordered_distilled, finetype_bin, output_path,
            workers, ratio_distilled, samples_per_type,
            taxonomy_types, rng, start_time,
        )
        type_counts = None  # v3 groups don't easily yield per-type counts

    file_size = os.path.getsize(output_path)
    print(f"  File size: {file_size / (1024*1024):.1f} MB")

    # ─── Summary ──────────────────────────────────────────────────
    elapsed = time.time() - start_time

    print(f"\n{'='*60}")
    print(f"Summary:")
    print(f"  FTMB version:    {version}")
    print(f"  Records written: {n_records}")
    if n_groups > 0:
        print(f"  Table groups:    {n_groups}")
        print(f"  Avg group size:  {n_records / n_groups:.1f} columns")
    if type_counts:
        print(f"  Types covered:   {len(type_counts)}")
    print(f"  Extraction errors: {errors}")
    print(f"  Time: {elapsed:.1f}s ({elapsed/60:.1f}min)")
    print(f"  Output: {output_path}")
    if output_format == "v3":
        print(f"  Distilled grouping: proximity-based (source_file empty in Sherlock)")
        print(f"  Synthetic grouping: domain-based table templates ({len(TABLE_TEMPLATES)} templates)")
    print(f"{'='*60}")

    # Write manifest
    manifest_path = output_path.replace(".ftmb", ".manifest.json")
    manifest = {
        "ftmb_version": version,
        "format": output_format,
        "seed": seed,
        "samples_per_type": samples_per_type,
        "ratio_distilled": ratio_distilled,
        "min_values": min_values,
        "distilled_source": distilled_path,
        "taxonomy_types": len(taxonomy_types),
        "blended_types": len(blended_types),
        "records_written": n_records,
        "errors": errors,
        "dimensions": {
            "char": CHAR_DIM,
            "embed": EMBED_DIM,
            "stats": STATS_DIM,
            "header": HEADER_DIM,
        },
    }
    if n_groups > 0:
        manifest["table_groups"] = n_groups
        manifest["avg_group_size"] = round(n_records / n_groups, 1)
        manifest["distilled_grouping_strategy"] = "proximity"
        manifest["synthetic_grouping_strategy"] = "table_templates"
        manifest["n_table_templates"] = len(TABLE_TEMPLATES)
    if type_counts:
        manifest["types_covered"] = len(type_counts)
        manifest["type_counts"] = dict(type_counts.most_common())

    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"Manifest: {manifest_path}")
    print("Done.")


if __name__ == "__main__":
    main()
