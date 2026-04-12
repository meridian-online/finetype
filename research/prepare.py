"""
FineType autoresearch — data preparation, feature extraction, and evaluation.

This file is FIXED. The autoresearch agent must NOT modify it.

Handles:
  - HuggingFace dataset download (sherlock-annotated + finetype-synthetic)
  - Column assembly from NDJSON synthetic data
  - Feature extraction via `finetype extract-features` subprocess
  - Train/val/test dataloaders (PyTorch tensors)
  - Validation accuracy evaluation
  - Profile eval (confirmation-tier metric against 29 eval CSV datasets)

Usage: imported by train.py
"""

import csv
import json
import os
import random
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

import torch
from torch.utils.data import Dataset, DataLoader

# ═══════════════════════════════════════════════════════════════════════════════
# Constants (not modifiable)
# ═══════════════════════════════════════════════════════════════════════════════

FINETYPE_BIN = os.environ.get("FINETYPE_BIN", "/workspace/bin/finetype")
TIME_BUDGET = 600  # 10 minutes in seconds
CHAR_DIM = 960
EMBED_DIM = 512
STATS_DIM = 27
HEADER_DIM = 128
TOTAL_DIM = CHAR_DIM + EMBED_DIM + STATS_DIM + HEADER_DIM  # 1627
MAX_COLUMN_VALUES = 100  # Sample up to 100 values per column
EVAL_TOKENS = None  # Evaluate on full validation set

# HuggingFace dataset identifiers
HF_SHERLOCK = "meridian-online/sherlock-annotated"
HF_SYNTHETIC = "meridian-online/finetype-synthetic"

# Local fallback paths (RunPod workspace)
LOCAL_SHERLOCK = "/workspace/datasets/sherlock-annotated"
LOCAL_SYNTHETIC = "/workspace/datasets/finetype-synthetic"

# Cache directory
CACHE_DIR = Path(os.environ.get("FINETYPE_CACHE", "/workspace/cache/finetype-research"))

# Repo root (for profile eval)
REPO_ROOT = Path(__file__).resolve().parent.parent

# Profile eval dataset names — must be excluded from training to prevent contamination
EVAL_DATASET_NAMES = {
    "api_users_json", "books_catalog", "codes_and_ids", "countries",
    "covid_timeseries", "datetime_formats", "datetime_formats_extended",
    "earthquakes_2024", "ecommerce_orders", "ecommerce_orders_json",
    "financial_data", "geography_data", "iris", "medical_records",
    "multilingual", "network_logs", "new_finance", "new_geography",
    "new_identity", "new_representation", "new_technology",
    "people_directory", "scientific_measurements", "server_logs_json",
    "sports_events", "tech_systems", "titanic", "us_states",
    "weather_stations_json", "world_cities",
}


# ═══════════════════════════════════════════════════════════════════════════════
# Header variation mapping
#
# Realistic column name variations for synthetic data. Prevents the model from
# cheating on perfect type-key headers. Each type maps to a list of plausible
# column names an analyst might use. For types not in this mapping, we generate
# variations programmatically from the type key's leaf name.
# ═══════════════════════════════════════════════════════════════════════════════

HEADER_VARIATIONS = {
    # --- Identity domain ---
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
    # --- Geography domain ---
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
    "geography.address.full_address": [
        "full_address", "address", "complete_address", "Full Address",
        "FULL_ADDRESS", "mailing_address", "postal_address", "location",
        "street_address", "addr", "street", "address_line_1", "street_addr",
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
    # --- Datetime domain ---
    "datetime.timestamp.iso_8601": [
        "timestamp", "datetime", "created_at", "updated_at", "Timestamp",
        "TIMESTAMP", "date_time", "event_time", "ts", "created",
    ],
    "datetime.date.iso": [
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
    # --- Finance domain ---
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
    # --- Technology domain ---
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
    # --- Representation domain ---
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
    # --- Container domain ---
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
    leaf = type_key.rsplit(".", 1)[-1]
    variations = [leaf]
    variations.append(leaf.replace("_", " ").title().replace(" ", ""))
    variations.append(leaf.upper())
    if "_" in leaf:
        variations.append(leaf.replace("_", " ").title())
        parts = leaf.split("_")
        variations.append(parts[0] + "".join(p.capitalize() for p in parts[1:]))
        variations.append(parts[0])
    variations.append(leaf.capitalize())
    variations.append(leaf.replace("_", "-"))
    seen = set()
    unique = []
    for v in variations:
        if v not in seen:
            seen.add(v)
            unique.append(v)
    return unique


def get_header_for_type(type_key, rng):
    """Return a random realistic header name for a given type key."""
    variations = HEADER_VARIATIONS.get(type_key)
    if not variations:
        variations = _generate_fallback_header_variations(type_key)
    return rng.choice(variations)


# ═══════════════════════════════════════════════════════════════════════════════
# Feature extraction — calls `finetype extract-features` as subprocess
# ═══════════════════════════════════════════════════════════════════════════════


def extract_features(finetype_bin, values, header=None):
    """Call `finetype extract-features` to get feature vectors for a column.

    Args:
        finetype_bin: Path to the finetype binary.
        values: List of string values (the column data).
        header: Optional column header name.

    Returns:
        dict with 'char' (960-dim), 'embed' (512-dim), 'stats' (27-dim),
        'header_features' (128-dim) arrays, or None on failure.
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
# Data loading — HuggingFace datasets with local fallback
# ═══════════════════════════════════════════════════════════════════════════════


def _load_sherlock_annotated():
    """Load sherlock-annotated dataset.

    Returns list of dicts: [{"values": [...], "header": str, "label": str}, ...]
    Each record is one column (already column-level in the source).
    """
    # Try HuggingFace first
    try:
        from datasets import load_dataset
        ds = load_dataset(HF_SHERLOCK, split="train")
        records = []
        for row in ds:
            records.append({
                "values": row["values"] if isinstance(row["values"], list) else json.loads(row["values"]),
                "header": row.get("header", row.get("col_name", "")),
                "label": row["label"],
            })
        print(f"Loaded {len(records)} columns from HuggingFace ({HF_SHERLOCK})")
        return records
    except Exception as e:
        print(f"HuggingFace download failed ({e}), trying local fallback...", file=sys.stderr)

    # Local fallback — look for NDJSON or CSV files
    local_path = Path(LOCAL_SHERLOCK)
    if not local_path.exists():
        print(f"WARNING: No sherlock-annotated data found at {local_path}", file=sys.stderr)
        return []

    records = []
    for f in sorted(local_path.glob("*.ndjson")) + sorted(local_path.glob("*.jsonl")):
        with open(f) as fh:
            for line in fh:
                if line.strip():
                    row = json.loads(line)
                    records.append({
                        "values": row["values"] if isinstance(row["values"], list) else json.loads(row["values"]),
                        "header": row.get("header", row.get("col_name", "")),
                        "label": row["label"],
                    })
    if records:
        print(f"Loaded {len(records)} columns from local ({local_path})")
    else:
        # Try CSV fallback
        for f in sorted(local_path.glob("*.csv")) + sorted(local_path.glob("*.csv.gz")):
            import gzip as gz
            opener = gz.open if str(f).endswith(".gz") else open
            with opener(f, "rt") as fh:
                reader = csv.DictReader(fh)
                for row in reader:
                    values = row.get("values", "[]")
                    if isinstance(values, str):
                        values = json.loads(values)
                    records.append({
                        "values": values,
                        "header": row.get("header", row.get("col_name", "")),
                        "label": row["label"],
                    })
        if records:
            print(f"Loaded {len(records)} columns from local CSV ({local_path})")

    return records


def _load_synthetic_ndjson():
    """Load finetype-synthetic dataset (NDJSON format: {text, classification} per value).

    Returns list of dicts: [{"values": [...], "header": str, "label": str}, ...]
    Values are assembled into column-level records by grouping on classification.
    """
    raw_values = defaultdict(list)  # classification -> list of text values

    # Try HuggingFace first
    try:
        from datasets import load_dataset
        ds = load_dataset(HF_SYNTHETIC, split="train")
        for row in ds:
            raw_values[row["classification"]].append(row["text"])
        print(f"Loaded {sum(len(v) for v in raw_values.values())} synthetic values "
              f"across {len(raw_values)} types from HuggingFace ({HF_SYNTHETIC})")
    except Exception as e:
        print(f"HuggingFace download failed ({e}), trying local fallback...", file=sys.stderr)

        # Local fallback
        local_path = Path(LOCAL_SYNTHETIC)
        if not local_path.exists():
            print(f"WARNING: No synthetic data found at {local_path}", file=sys.stderr)
            return []

        for f in sorted(local_path.glob("*.ndjson")) + sorted(local_path.glob("*.jsonl")):
            with open(f) as fh:
                for line in fh:
                    if line.strip():
                        row = json.loads(line)
                        raw_values[row["classification"]].append(row["text"])

        if not raw_values:
            # Try parquet fallback
            for f in sorted(local_path.glob("*.parquet")):
                try:
                    import pyarrow.parquet as pq
                    table = pq.read_table(f)
                    for row in table.to_pylist():
                        raw_values[row["classification"]].append(row["text"])
                except ImportError:
                    print("WARNING: pyarrow not available for parquet fallback", file=sys.stderr)

        if raw_values:
            print(f"Loaded {sum(len(v) for v in raw_values.values())} synthetic values "
                  f"across {len(raw_values)} types from local ({local_path})")

    if not raw_values:
        return []

    # Assemble into column-level records
    return _assemble_columns_from_values(raw_values)


def _assemble_columns_from_values(values_by_type, columns_per_type=10,
                                   values_per_column=MAX_COLUMN_VALUES, seed=42):
    """Assemble value-level data into column-level records.

    For each type, create multiple synthetic columns by sampling N values
    and assigning realistic header names.

    Args:
        values_by_type: dict mapping type label -> list of text values
        columns_per_type: How many synthetic columns to create per type
        values_per_column: How many values per column (default MAX_COLUMN_VALUES)
        seed: Random seed for reproducibility

    Returns:
        list of {"values": [...], "header": str, "label": str}
    """
    rng = random.Random(seed)
    records = []

    for type_key, all_values in sorted(values_by_type.items()):
        if len(all_values) < 5:
            continue  # Skip types with too few values

        n_columns = min(columns_per_type, max(1, len(all_values) // values_per_column))

        for _ in range(n_columns):
            n_sample = min(values_per_column, len(all_values))
            sampled = rng.sample(all_values, n_sample) if n_sample < len(all_values) else list(all_values)
            header = get_header_for_type(type_key, rng)
            records.append({
                "values": sampled,
                "header": header,
                "label": type_key,
            })

    print(f"Assembled {len(records)} synthetic columns from {len(values_by_type)} types")
    return records


# ═══════════════════════════════════════════════════════════════════════════════
# Data pipeline: merge, split, extract features, build dataloaders
# ═══════════════════════════════════════════════════════════════════════════════


def _build_label_mapping(records):
    """Build a deterministic label-to-index mapping from all records.

    Returns:
        label_to_idx: dict mapping label string to integer index
        idx_to_label: dict mapping integer index to label string
    """
    labels = sorted(set(r["label"] for r in records))
    label_to_idx = {label: idx for idx, label in enumerate(labels)}
    idx_to_label = {idx: label for label, idx in label_to_idx.items()}
    return label_to_idx, idx_to_label


def _merge_datasets(sherlock_records, synthetic_records, synthetic_ratio=0.5, seed=42):
    """Merge sherlock and synthetic records with configurable mix ratio.

    Args:
        sherlock_records: Column-level records from sherlock-annotated
        synthetic_records: Column-level records from finetype-synthetic
        synthetic_ratio: Fraction of final dataset that should be synthetic (0.0 to 1.0)
        seed: Random seed

    Returns:
        Merged list of records
    """
    rng = random.Random(seed)

    if not sherlock_records and not synthetic_records:
        raise ValueError("No data loaded from either source!")

    if not sherlock_records:
        print("WARNING: No sherlock data, using synthetic only", file=sys.stderr)
        return synthetic_records

    if not synthetic_records:
        print("WARNING: No synthetic data, using sherlock only", file=sys.stderr)
        return sherlock_records

    # Target: balance by ratio
    total_sherlock = len(sherlock_records)
    if synthetic_ratio > 0:
        target_synthetic = int(total_sherlock * synthetic_ratio / (1 - synthetic_ratio))
    else:
        target_synthetic = 0

    if target_synthetic > len(synthetic_records):
        # Not enough synthetic data — use all of it
        sampled_synthetic = synthetic_records
    else:
        sampled_synthetic = rng.sample(synthetic_records, target_synthetic)

    merged = sherlock_records + sampled_synthetic
    rng.shuffle(merged)

    print(f"Merged dataset: {len(sherlock_records)} sherlock + {len(sampled_synthetic)} synthetic "
          f"= {len(merged)} total (target ratio: {synthetic_ratio:.0%} synthetic)")
    return merged


def _split_data(records, train_frac=0.8, val_frac=0.1, seed=42):
    """Split records into train/val/test (80/10/10).

    Filters out eval dataset names to prevent train/eval contamination.
    """
    rng = random.Random(seed)
    rng.shuffle(records)

    n = len(records)
    n_train = int(n * train_frac)
    n_val = int(n * val_frac)

    train = records[:n_train]
    val = records[n_train:n_train + n_val]
    test = records[n_train + n_val:]

    print(f"Split: {len(train)} train / {len(val)} val / {len(test)} test")
    return train, val, test


class ColumnFeatureDataset(Dataset):
    """PyTorch dataset of pre-extracted column features.

    Each item is (features_tensor[1627], label_index).
    """

    def __init__(self, features, labels):
        """
        Args:
            features: list of 1627-dim float lists/arrays
            labels: list of integer label indices
        """
        assert len(features) == len(labels)
        self.features = torch.tensor(features, dtype=torch.float32)
        self.labels = torch.tensor(labels, dtype=torch.long)

    def __len__(self):
        return len(self.labels)

    def __getitem__(self, idx):
        return self.features[idx], self.labels[idx]


def _extract_all_features(records, finetype_bin=None):
    """Extract features for all records using finetype binary.

    Args:
        records: list of {"values": [...], "header": str, "label": str}
        finetype_bin: Path to finetype binary (default: FINETYPE_BIN)

    Returns:
        features: list of 1627-dim float lists (parallel to records)
        valid_indices: list of indices where extraction succeeded
    """
    if finetype_bin is None:
        finetype_bin = FINETYPE_BIN

    features = []
    valid_indices = []
    failed = 0
    t0 = time.time()

    for i, record in enumerate(records):
        if (i + 1) % 100 == 0:
            elapsed = time.time() - t0
            rate = (i + 1) / elapsed
            remaining = (len(records) - i - 1) / rate
            print(f"  Extracting features: {i+1}/{len(records)} "
                  f"({rate:.1f}/s, ~{remaining:.0f}s remaining)", end="\r")

        values = record["values"][:MAX_COLUMN_VALUES]
        header = record.get("header")

        feat = extract_features(finetype_bin, values, header=header)
        if feat is None:
            failed += 1
            continue

        # Concatenate all branches into a single 1627-dim vector
        char_feat = feat.get("char", [0.0] * CHAR_DIM)
        embed_feat = feat.get("embed", [0.0] * EMBED_DIM)
        stats_feat = feat.get("stats", [0.0] * STATS_DIM)
        header_feat = feat.get("header_features", [0.0] * HEADER_DIM)

        # Validate dimensions
        if len(char_feat) != CHAR_DIM:
            char_feat = (char_feat + [0.0] * CHAR_DIM)[:CHAR_DIM]
        if len(embed_feat) != EMBED_DIM:
            embed_feat = (embed_feat + [0.0] * EMBED_DIM)[:EMBED_DIM]
        if len(stats_feat) != STATS_DIM:
            stats_feat = (stats_feat + [0.0] * STATS_DIM)[:STATS_DIM]
        if len(header_feat) != HEADER_DIM:
            header_feat = (header_feat + [0.0] * HEADER_DIM)[:HEADER_DIM]

        combined = char_feat + embed_feat + stats_feat + header_feat
        assert len(combined) == TOTAL_DIM, f"Expected {TOTAL_DIM}, got {len(combined)}"

        features.append(combined)
        valid_indices.append(i)

    print()  # Clear the \r line
    elapsed = time.time() - t0
    print(f"Feature extraction: {len(features)}/{len(records)} succeeded, "
          f"{failed} failed, {elapsed:.1f}s")
    return features, valid_indices


def _load_or_extract_cached(records, label_to_idx, split_name, finetype_bin=None):
    """Load cached features or extract fresh ones.

    Caches to CACHE_DIR/{split_name}_features.pt and {split_name}_labels.pt
    """
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    feat_cache = CACHE_DIR / f"{split_name}_features.pt"
    label_cache = CACHE_DIR / f"{split_name}_labels.pt"

    if feat_cache.exists() and label_cache.exists():
        print(f"Loading cached {split_name} features from {CACHE_DIR}")
        features_t = torch.load(feat_cache, weights_only=True)
        labels_t = torch.load(label_cache, weights_only=True)
        return ColumnFeatureDataset(features_t, labels_t)

    print(f"Extracting {split_name} features ({len(records)} records)...")
    features, valid_indices = _extract_all_features(records, finetype_bin)

    # Filter features to match valid labels
    valid_features = []
    valid_labels = []
    for feat, idx in zip(features, valid_indices):
        label = records[idx]["label"]
        if label in label_to_idx:
            valid_features.append(feat)
            valid_labels.append(label_to_idx[label])

    features_t = torch.tensor(valid_features, dtype=torch.float32)
    labels_t = torch.tensor(valid_labels, dtype=torch.long)

    torch.save(features_t, feat_cache)
    torch.save(labels_t, label_cache)
    print(f"Cached {split_name} features to {CACHE_DIR}")

    return ColumnFeatureDataset(features_t, labels_t)


# ═══════════════════════════════════════════════════════════════════════════════
# Public API — called by train.py
# ═══════════════════════════════════════════════════════════════════════════════


# Global state populated by prepare_data()
_label_to_idx = {}
_idx_to_label = {}
_train_dataset = None
_val_dataset = None
_test_dataset = None
N_CLASSES = 0  # Set after data loading


def prepare_data(synthetic_ratio=0.5, seed=42, finetype_bin=None):
    """Load data, extract features, build datasets. Call once at startup.

    Args:
        synthetic_ratio: Fraction of dataset that should be synthetic (0.0-1.0)
        seed: Random seed for reproducibility
        finetype_bin: Path to finetype binary (default: FINETYPE_BIN env var)
    """
    global _label_to_idx, _idx_to_label, _train_dataset, _val_dataset, _test_dataset, N_CLASSES

    print("=" * 70)
    print("FineType autoresearch — data preparation")
    print("=" * 70)

    # Load raw data
    print("\n--- Loading sherlock-annotated ---")
    sherlock = _load_sherlock_annotated()

    print("\n--- Loading finetype-synthetic ---")
    synthetic = _load_synthetic_ndjson()

    # Merge
    print("\n--- Merging datasets ---")
    merged = _merge_datasets(sherlock, synthetic, synthetic_ratio=synthetic_ratio, seed=seed)

    # Build label mapping
    _label_to_idx, _idx_to_label = _build_label_mapping(merged)
    N_CLASSES = len(_label_to_idx)
    print(f"\nLabel mapping: {N_CLASSES} classes")

    # Export label mapping for reference
    mapping_path = CACHE_DIR / "label_mapping.json"
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    with open(mapping_path, "w") as f:
        json.dump({"label_to_idx": _label_to_idx, "idx_to_label": {str(k): v for k, v in _idx_to_label.items()}},
                  f, indent=2)
    print(f"Label mapping saved to {mapping_path}")

    # Split
    print("\n--- Splitting data ---")
    train_records, val_records, test_records = _split_data(merged, seed=seed)

    # Extract features (with caching)
    print("\n--- Extracting features ---")
    _train_dataset = _load_or_extract_cached(train_records, _label_to_idx, "train", finetype_bin)
    _val_dataset = _load_or_extract_cached(val_records, _label_to_idx, "val", finetype_bin)
    _test_dataset = _load_or_extract_cached(test_records, _label_to_idx, "test", finetype_bin)

    print(f"\nDatasets ready: train={len(_train_dataset)}, val={len(_val_dataset)}, test={len(_test_dataset)}")
    print(f"N_CLASSES={N_CLASSES}")
    print("=" * 70)


def make_dataloader(split="train", batch_size=64, shuffle=None, num_workers=0):
    """Create a DataLoader for the specified split.

    Args:
        split: "train", "val", or "test"
        batch_size: Batch size
        shuffle: Whether to shuffle (default: True for train, False for val/test)
        num_workers: DataLoader workers

    Returns:
        torch.utils.data.DataLoader
    """
    if _train_dataset is None:
        raise RuntimeError("Call prepare_data() before make_dataloader()")

    ds = {"train": _train_dataset, "val": _val_dataset, "test": _test_dataset}[split]
    if shuffle is None:
        shuffle = (split == "train")

    return DataLoader(
        ds,
        batch_size=batch_size,
        shuffle=shuffle,
        num_workers=num_workers,
        pin_memory=True,
        drop_last=(split == "train"),
    )


# ═══════════════════════════════════════════════════════════════════════════════
# Evaluation
# ═══════════════════════════════════════════════════════════════════════════════


def evaluate_accuracy(model, dataloader, device=None):
    """Evaluate model on a dataloader, returning accuracy and loss.

    Args:
        model: PyTorch model (features -> logits)
        dataloader: DataLoader yielding (features, labels)
        device: torch device (default: auto-detect)

    Returns:
        (val_accuracy: float, val_loss: float)
    """
    if device is None:
        device = next(model.parameters()).device

    model.eval()
    total_correct = 0
    total_samples = 0
    total_loss = 0.0
    n_batches = 0

    with torch.no_grad():
        for features, labels in dataloader:
            features = features.to(device)
            labels = labels.to(device)

            logits = model(features)
            loss = torch.nn.functional.cross_entropy(logits, labels)

            preds = logits.argmax(dim=-1)
            total_correct += (preds == labels).sum().item()
            total_samples += labels.size(0)
            total_loss += loss.item()
            n_batches += 1

    accuracy = total_correct / total_samples if total_samples > 0 else 0.0
    avg_loss = total_loss / n_batches if n_batches > 0 else float("inf")

    return accuracy, avg_loss


def profile_eval(model, finetype_bin=None, repo_root=None, device=None):
    """Run profile eval: test model against the 29 evaluation CSV datasets.

    This is the confirmation-tier metric matching `eval/profile_eval.sh`.
    Loads manifest.csv, reads each annotated column from its CSV file,
    extracts features, runs model inference, and compares to ground truth.

    Args:
        model: PyTorch model (features -> logits)
        finetype_bin: Path to finetype binary (default: FINETYPE_BIN)
        repo_root: Path to repo root (default: auto-detect)
        device: torch device

    Returns:
        (label_correct, label_total, domain_correct, domain_total)
    """
    if finetype_bin is None:
        finetype_bin = FINETYPE_BIN
    if repo_root is None:
        repo_root = REPO_ROOT
    if device is None:
        device = next(model.parameters()).device

    repo_root = Path(repo_root)
    manifest_path = repo_root / "eval" / "datasets" / "manifest.csv"

    if not manifest_path.exists():
        print(f"WARNING: manifest not found at {manifest_path}", file=sys.stderr)
        return (0, 0, 0, 0)

    # Load schema mapping for GT label -> FineType label resolution
    schema_mapping = _load_schema_mapping(repo_root)

    # Load manifest
    manifest_entries = []
    with open(manifest_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            manifest_entries.append(row)

    print(f"Profile eval: {len(manifest_entries)} manifest entries")

    # Group by file to avoid re-reading the same CSV
    entries_by_file = defaultdict(list)
    for entry in manifest_entries:
        entries_by_file[entry["file_path"]].append(entry)

    model.eval()
    label_correct = 0
    label_total = 0
    domain_correct = 0
    domain_total = 0

    for file_path_str, entries in entries_by_file.items():
        file_path = repo_root / file_path_str
        if not file_path.exists():
            print(f"  SKIP: {file_path} not found", file=sys.stderr)
            continue

        # Read the file
        columns = _read_file_columns(file_path)
        if columns is None:
            continue

        for entry in entries:
            col_name = entry["column_name"]
            gt_label = entry["gt_label"]

            # Handle nested column names (e.g., "address.city")
            if col_name not in columns:
                continue

            values = columns[col_name][:MAX_COLUMN_VALUES]
            if not values:
                continue

            # Extract features
            feat = extract_features(finetype_bin, values, header=col_name)
            if feat is None:
                continue

            # Build feature vector
            char_feat = feat.get("char", [0.0] * CHAR_DIM)
            embed_feat = feat.get("embed", [0.0] * EMBED_DIM)
            stats_feat = feat.get("stats", [0.0] * STATS_DIM)
            header_feat = feat.get("header_features", [0.0] * HEADER_DIM)

            combined = char_feat + embed_feat + stats_feat + header_feat
            if len(combined) != TOTAL_DIM:
                combined = (combined + [0.0] * TOTAL_DIM)[:TOTAL_DIM]

            features_t = torch.tensor([combined], dtype=torch.float32).to(device)

            # Inference
            with torch.no_grad():
                logits = model(features_t)
                pred_idx = logits.argmax(dim=-1).item()

            if pred_idx not in _idx_to_label:
                continue

            pred_label = _idx_to_label[pred_idx]
            pred_domain = pred_label.split(".")[0] if "." in pred_label else pred_label

            # Resolve GT label to FineType label via schema mapping
            gt_finetype = schema_mapping.get(gt_label, {})
            expected_label = gt_finetype.get("finetype_label")
            expected_domain = gt_finetype.get("finetype_domain")

            # Label accuracy
            label_total += 1
            if expected_label and pred_label == expected_label:
                label_correct += 1

            # Domain accuracy
            domain_total += 1
            if expected_domain and pred_domain == expected_domain:
                domain_correct += 1

    print(f"Profile eval: {label_correct}/{label_total} label, "
          f"{domain_correct}/{domain_total} domain")
    return (label_correct, label_total, domain_correct, domain_total)


def _load_schema_mapping(repo_root):
    """Load eval/schema_mapping.yaml for GT label resolution.

    Returns dict: gt_label -> {"finetype_label": str, "finetype_domain": str}
    """
    mapping_path = repo_root / "eval" / "schema_mapping.yaml"
    if not mapping_path.exists():
        return {}

    # Try yaml first, fall back to manual parsing
    try:
        import yaml
        with open(mapping_path) as f:
            data = yaml.safe_load(f)
        result = {}
        for entry in data.get("mappings", []):
            gt = entry.get("gt_label")
            if gt:
                result[gt] = {
                    "finetype_label": entry.get("finetype_label"),
                    "finetype_domain": entry.get("finetype_domain"),
                }
        return result
    except ImportError:
        # Manual YAML parsing for the simple structure we need
        result = {}
        current_gt = None
        with open(mapping_path) as f:
            for line in f:
                stripped = line.strip()
                if stripped.startswith("- gt_label:"):
                    current_gt = stripped.split(":", 1)[1].strip()
                elif stripped.startswith("finetype_label:") and current_gt:
                    val = stripped.split(":", 1)[1].strip()
                    if val == "null" or val == "~":
                        val = None
                    if current_gt not in result:
                        result[current_gt] = {}
                    result[current_gt]["finetype_label"] = val
                elif stripped.startswith("finetype_domain:") and current_gt:
                    val = stripped.split(":", 1)[1].strip()
                    if val == "null" or val == "~":
                        val = None
                    if current_gt not in result:
                        result[current_gt] = {}
                    result[current_gt]["finetype_domain"] = val
        return result


def _read_file_columns(file_path):
    """Read a CSV/JSON/NDJSON file and return columns as dict of lists.

    Returns: {"col_name": [values...], ...} or None on failure.
    """
    file_path = Path(file_path)
    suffix = file_path.suffix.lower()

    try:
        if suffix == ".csv":
            columns = defaultdict(list)
            with open(file_path, newline="", encoding="utf-8", errors="replace") as f:
                reader = csv.DictReader(f)
                for row in reader:
                    for key, value in row.items():
                        if key is not None and value is not None:
                            columns[key].append(value)
            return dict(columns)

        elif suffix == ".json":
            with open(file_path, encoding="utf-8") as f:
                data = json.load(f)
            if isinstance(data, list):
                columns = defaultdict(list)
                for item in data:
                    if isinstance(item, dict):
                        _flatten_dict(item, columns, prefix="")
                return dict(columns)

        elif suffix == ".ndjson" or suffix == ".jsonl":
            columns = defaultdict(list)
            with open(file_path, encoding="utf-8") as f:
                for line in f:
                    if line.strip():
                        item = json.loads(line)
                        if isinstance(item, dict):
                            _flatten_dict(item, columns, prefix="")
            return dict(columns)

    except Exception as e:
        print(f"  WARNING: Failed to read {file_path}: {e}", file=sys.stderr)
        return None

    return None


def _flatten_dict(d, columns, prefix=""):
    """Flatten a nested dict into dot-separated column paths."""
    for key, value in d.items():
        full_key = f"{prefix}.{key}" if prefix else key
        if isinstance(value, dict):
            _flatten_dict(value, columns, prefix=full_key)
        else:
            columns[full_key].append(str(value) if value is not None else "")
