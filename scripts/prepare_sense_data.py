#!/usr/bin/env python3
"""Prepare training data for Sense model.

Combines SOTAB columns (web tables) with profile eval columns (real-world
datasets with descriptive headers) to train a production Sense model that
handles diverse column headers.

Data sources:
  1. SOTAB CTA — ~31k columns, no meaningful headers (integer indices)
  2. Profile eval — ~120 columns with real headers (city, country, latitude, ...)
  3. Synthetic headers — plausible column names generated from SOTAB GT labels

Usage:
    # Spike mode (SOTAB only, original behaviour)
    python3 scripts/prepare_sense_data.py --output data/sense_spike

    # Production mode (SOTAB + profile eval + synthetic headers)
    python3 scripts/prepare_sense_data.py --include-profile --synthetic-headers \
        --output data/sense_prod

Requires: duckdb, numpy, pyyaml
"""

import argparse
import csv
import json
import random
from collections import Counter, defaultdict
from pathlib import Path

import duckdb

# ── Label mappings ──────────────────────────────────────────────────

# Map SOTAB GT labels → 6 broad categories
BROAD_CATEGORY_MAP = {
    # ENTITY - named things
    "Organization": "entity", "Person": "entity", "Person/name": "entity",
    "Place": "entity", "Place/name": "entity", "MusicArtistAT": "entity",
    "LocalBusiness/name": "entity", "Hotel/name": "entity",
    "Restaurant/name": "entity", "Brand": "entity", "SportsTeam": "entity",
    "EducationalOrganization": "entity", "MusicGroup": "entity",
    "Museum/name": "entity", "MusicAlbum": "entity",
    "MusicRecording/name": "entity", "Event/name": "entity",
    "Book/name": "entity", "Recipe/name": "entity", "Movie/name": "entity",
    "CreativeWork/name": "entity", "SportsEvent/name": "entity",
    "TVEpisode/name": "entity", "MusicAlbum/name": "entity",
    "JobPosting/name": "entity", "CreativeWork": "entity",
    "CreativeWorkSeries": "entity", "Product/name": "entity",
    "ProductModel": "entity",
    # FORMAT - structured identifiers, codes, URLs
    "URL": "format", "email": "format", "telephone": "format",
    "faxNumber": "format", "postalCode": "format", "IdentifierAT": "format",
    "identifierNameAP": "format", "unitCode": "format",
    "CategoryCode": "format",
    # TEMPORAL - dates, times, durations
    "Date": "temporal", "DateTime": "temporal", "Duration": "temporal",
    "Time": "temporal", "DayOfWeek": "temporal", "openingHours": "temporal",
    "workHours": "temporal",
    # NUMERIC - numbers, measurements, quantities
    "Number": "numeric", "Integer": "numeric", "Mass": "numeric",
    "Distance": "numeric", "Energy": "numeric", "weight": "numeric",
    "QuantitativeValue": "numeric", "price": "numeric",
    "priceRange": "numeric", "currency": "numeric",
    "MonetaryAmount": "numeric", "CoordinateAT": "numeric",
    "Rating": "numeric", "typicalAgeRange": "numeric",
    # GEOGRAPHIC - locations, addresses
    "addressLocality": "geographic", "addressRegion": "geographic",
    "Country": "geographic", "streetAddress": "geographic",
    "PostalAddress": "geographic",
    # TEXT - free text, categories, enums, descriptions
    "Text": "text", "category": "text", "ItemAvailability": "text",
    "ItemList": "text", "Review": "text", "EventStatusType": "text",
    "BookFormatType": "text", "Language": "text", "Thing": "text",
    "GenderType": "text", "EventAttendanceModeEnumeration": "text",
    "OccupationalExperienceRequirements": "text", "unitText": "text",
    "OfferItemCondition": "text", "Boolean": "text",
    "paymentAccepted": "text", "Photograph": "text", "Offer": "text",
    "Action": "text", "DeliveryMethod": "text", "RestrictedDiet": "text",
    "Product": "text", "LocationFeatureSpecification": "text",
    "audience": "text", "MusicRecording": "text", "WarrantyPromise": "text",
    "EducationalOccupationalCredential": "text",
}

# Map entity GT labels → 4 subtypes
ENTITY_SUBTYPE_MAP = {
    "Person": "person", "Person/name": "person", "MusicArtistAT": "person",
    "Place": "place", "Place/name": "place", "Hotel/name": "place",
    "Restaurant/name": "place", "Museum/name": "place",
    "Organization": "organization", "LocalBusiness/name": "organization",
    "Brand": "organization", "SportsTeam": "organization",
    "EducationalOrganization": "organization", "MusicGroup": "organization",
    "MusicAlbum": "creative_work", "MusicRecording/name": "creative_work",
    "Event/name": "creative_work", "Book/name": "creative_work",
    "Recipe/name": "creative_work", "Movie/name": "creative_work",
    "CreativeWork/name": "creative_work", "SportsEvent/name": "creative_work",
    "TVEpisode/name": "creative_work", "MusicAlbum/name": "creative_work",
    "JobPosting/name": "creative_work", "CreativeWork": "creative_work",
    "CreativeWorkSeries": "creative_work", "Product/name": "creative_work",
    "ProductModel": "creative_work",
}

BROAD_CATEGORIES = ["entity", "format", "temporal", "numeric", "geographic", "text"]
ENTITY_SUBTYPES = ["person", "place", "organization", "creative_work"]


# ── FineType label → broad category (mirrors Rust LabelCategoryMap) ──
# This is the authoritative mapping from label_category_map.rs.
# The domain-based heuristic in the spike was approximate and incorrect
# for ~20 types (e.g. identity.person.email should be FORMAT not ENTITY).

FINETYPE_TO_BROAD = {}

_TEMPORAL_LABELS = [
    "datetime.component.century", "datetime.component.day_of_month",
    "datetime.component.day_of_week", "datetime.component.month_name",
    "datetime.component.periodicity", "datetime.component.year",
    "datetime.date.abbreviated_month", "datetime.date.compact_dmy",
    "datetime.date.compact_mdy", "datetime.date.compact_ymd",
    "datetime.date.eu_dot", "datetime.date.eu_slash", "datetime.date.iso",
    "datetime.date.iso_week", "datetime.date.julian",
    "datetime.date.long_full_month", "datetime.date.ordinal",
    "datetime.date.short_dmy", "datetime.date.short_mdy",
    "datetime.date.short_ymd", "datetime.date.us_slash",
    "datetime.date.weekday_abbreviated_month",
    "datetime.date.weekday_full_month", "datetime.duration.iso_8601",
    "datetime.epoch.unix_microseconds", "datetime.epoch.unix_milliseconds",
    "datetime.epoch.unix_seconds", "datetime.offset.iana",
    "datetime.offset.utc", "datetime.time.hm_12h", "datetime.time.hm_24h",
    "datetime.time.hms_12h", "datetime.time.hms_24h", "datetime.time.iso",
    "datetime.timestamp.american", "datetime.timestamp.american_24h",
    "datetime.timestamp.european", "datetime.timestamp.iso_8601",
    "datetime.timestamp.iso_8601_compact",
    "datetime.timestamp.iso_8601_microseconds",
    "datetime.timestamp.iso_8601_offset",
    "datetime.timestamp.iso_microseconds", "datetime.timestamp.rfc_2822",
    "datetime.timestamp.rfc_2822_ordinal", "datetime.timestamp.rfc_3339",
    "datetime.timestamp.sql_standard",
]
_NUMERIC_LABELS = [
    "identity.person.age", "identity.person.height", "identity.person.weight",
    "representation.file.file_size", "representation.numeric.decimal_number",
    "representation.numeric.increment", "representation.numeric.integer_number",
    "representation.numeric.percentage",
    "representation.numeric.scientific_notation",
    "representation.numeric.si_number", "technology.hardware.ram_size",
    "technology.hardware.screen_size", "technology.internet.http_status_code",
    "technology.internet.port",
]
_GEOGRAPHIC_LABELS = [
    "geography.address.full_address", "geography.address.postal_code",
    "geography.address.street_name", "geography.address.street_number",
    "geography.address.street_suffix", "geography.contact.calling_code",
    "geography.coordinate.coordinates", "geography.coordinate.latitude",
    "geography.coordinate.longitude", "geography.location.city",
    "geography.location.continent", "geography.location.country",
    "geography.location.country_code", "geography.location.region",
    "geography.transportation.iata_code", "geography.transportation.icao_code",
]
_ENTITY_LABELS = [
    "identity.person.blood_type", "identity.person.first_name",
    "identity.person.full_name", "identity.person.gender",
    "identity.person.gender_code", "identity.person.gender_symbol",
    "identity.person.last_name", "identity.person.username",
    "representation.text.entity_name",
]
_FORMAT_LABELS = [
    "container.array.comma_separated", "container.array.pipe_separated",
    "container.array.semicolon_separated",
    "container.array.whitespace_separated", "container.key_value.form_data",
    "container.key_value.query_string", "container.object.csv",
    "container.object.json", "container.object.json_array",
    "container.object.xml", "container.object.yaml",
    "identity.medical.dea_number", "identity.medical.ndc",
    "identity.medical.npi", "identity.payment.bitcoin_address",
    "identity.payment.credit_card_expiration_date",
    "identity.payment.credit_card_number", "identity.payment.cusip",
    "identity.payment.cvv", "identity.payment.ethereum_address",
    "identity.payment.isin", "identity.payment.lei",
    "identity.payment.paypal_email", "identity.payment.sedol",
    "identity.payment.swift_bic", "identity.person.email",
    "identity.person.password", "identity.person.phone_number",
    "representation.code.alphanumeric_id",
    "representation.scientific.dna_sequence",
    "representation.scientific.protein_sequence",
    "representation.scientific.rna_sequence",
    "representation.text.color_hex", "representation.text.color_rgb",
    "technology.code.doi", "technology.code.ean", "technology.code.imei",
    "technology.code.isbn", "technology.code.issn",
    "technology.code.locale_code", "technology.code.pin",
    "technology.internet.hostname", "technology.internet.ip_v4",
    "technology.internet.ip_v4_with_port", "technology.internet.ip_v6",
    "technology.internet.mac_address", "technology.internet.url",
    "technology.internet.user_agent",
]
_TEXT_LABELS = [
    "identity.payment.credit_card_network",
    "identity.payment.currency_code", "identity.payment.currency_symbol",
    "representation.boolean.binary", "representation.boolean.initials",
    "representation.boolean.terms", "representation.discrete.categorical",
    "representation.discrete.ordinal", "representation.file.excel_format",
    "representation.file.extension", "representation.file.mime_type",
    "representation.scientific.measurement_unit",
    "representation.scientific.metric_prefix", "representation.text.emoji",
    "representation.text.paragraph", "representation.text.plain_text",
    "representation.text.sentence", "representation.text.word",
    "technology.cryptographic.hash", "technology.cryptographic.token_hex",
    "technology.cryptographic.token_urlsafe",
    "technology.cryptographic.uuid", "technology.development.calver",
    "technology.development.os", "technology.development.programming_language",
    "technology.development.software_license",
    "technology.development.stage", "technology.development.version",
    "technology.internet.http_method", "technology.internet.top_level_domain",
]

for _l in _TEMPORAL_LABELS:
    FINETYPE_TO_BROAD[_l] = "temporal"
for _l in _NUMERIC_LABELS:
    FINETYPE_TO_BROAD[_l] = "numeric"
for _l in _GEOGRAPHIC_LABELS:
    FINETYPE_TO_BROAD[_l] = "geographic"
for _l in _ENTITY_LABELS:
    FINETYPE_TO_BROAD[_l] = "entity"
for _l in _FORMAT_LABELS:
    FINETYPE_TO_BROAD[_l] = "format"
for _l in _TEXT_LABELS:
    FINETYPE_TO_BROAD[_l] = "text"

# Entity subtype for profile eval person columns
PROFILE_ENTITY_SUBTYPES = {
    "identity.person.full_name": "person",
    "identity.person.first_name": "person",
    "identity.person.last_name": "person",
    "identity.person.username": "person",
    "identity.person.gender": "person",
    "identity.person.gender_code": "person",
    "identity.person.gender_symbol": "person",
    "identity.person.blood_type": "person",
    "representation.text.entity_name": "organization",  # conservative default
}


# ── Synthetic header templates for SOTAB GT labels ──────────────────
# Maps each SOTAB Schema.org GT label to plausible column name variations.
# Used to teach the model diverse header → category associations.

SYNTHETIC_HEADERS = {
    # ENTITY
    "Person": ["name", "full_name", "person_name", "person", "contact", "contact_name"],
    "Person/name": ["name", "full_name", "person_name", "person", "Name"],
    "MusicArtistAT": ["artist", "artist_name", "performer", "musician", "singer"],
    "Organization": ["organization", "org_name", "company", "company_name", "org"],
    "LocalBusiness/name": ["business_name", "company", "store", "shop_name", "business"],
    "Hotel/name": ["hotel", "hotel_name", "accommodation", "lodging", "property"],
    "Restaurant/name": ["restaurant", "restaurant_name", "dining", "eatery"],
    "Brand": ["brand", "brand_name", "manufacturer", "make"],
    "SportsTeam": ["team", "team_name", "club", "squad"],
    "EducationalOrganization": ["school", "university", "institution", "college"],
    "MusicGroup": ["band", "group", "band_name", "ensemble"],
    "Museum/name": ["museum", "museum_name", "gallery", "exhibit"],
    "MusicAlbum": ["album", "album_name", "album_title", "release"],
    "MusicAlbum/name": ["album", "album_name", "album_title", "release"],
    "MusicRecording/name": ["song", "track", "track_name", "song_name", "recording"],
    "Event/name": ["event", "event_name", "activity", "occasion"],
    "Book/name": ["book", "book_title", "title", "publication"],
    "Recipe/name": ["recipe", "recipe_name", "dish", "meal"],
    "Movie/name": ["movie", "movie_title", "film", "film_name"],
    "CreativeWork/name": ["work", "title", "creative_work", "work_name"],
    "CreativeWork": ["work", "title", "creative_work", "content"],
    "CreativeWorkSeries": ["series", "series_name", "show", "franchise"],
    "SportsEvent/name": ["match", "game", "event", "fixture", "competition"],
    "TVEpisode/name": ["episode", "episode_name", "show", "program"],
    "JobPosting/name": ["job", "job_title", "position", "role", "vacancy"],
    "Product/name": ["product", "product_name", "item", "article"],
    "ProductModel": ["model", "model_name", "product_model", "variant"],
    "Place": ["place", "place_name", "venue", "site"],
    "Place/name": ["place", "place_name", "venue", "location_name"],
    # FORMAT
    "URL": ["url", "website", "link", "web_address", "homepage", "URL"],
    "email": ["email", "email_address", "contact_email", "Email", "e_mail"],
    "telephone": ["phone", "telephone", "phone_number", "contact_phone", "tel", "mobile"],
    "faxNumber": ["fax", "fax_number", "fax_no"],
    "postalCode": ["postal_code", "zip", "zip_code", "postcode", "ZipCode"],
    "IdentifierAT": ["id", "identifier", "code", "ID", "ref"],
    "identifierNameAP": ["identifier_name", "id_name", "ref_name"],
    "unitCode": ["unit_code", "unit", "units", "uom"],
    "CategoryCode": ["category_code", "cat_code", "code", "classification"],
    # TEMPORAL
    "Date": ["date", "created_date", "event_date", "Date", "start_date", "end_date",
             "birth_date", "due_date", "publish_date"],
    "DateTime": ["datetime", "timestamp", "created_at", "updated_at", "date_time",
                 "modified_at", "last_updated"],
    "Duration": ["duration", "length", "time_span", "runtime", "elapsed"],
    "Time": ["time", "start_time", "end_time", "Time", "clock_time"],
    "DayOfWeek": ["day", "day_of_week", "weekday", "day_name"],
    "openingHours": ["hours", "opening_hours", "business_hours", "schedule"],
    "workHours": ["work_hours", "shift", "working_hours"],
    # NUMERIC
    "Number": ["number", "count", "quantity", "amount", "total", "value", "num"],
    "Integer": ["integer", "count", "number", "qty", "int"],
    "Mass": ["mass", "weight", "weight_kg", "mass_kg", "grams"],
    "Distance": ["distance", "length", "distance_km", "range", "miles"],
    "Energy": ["energy", "calories", "energy_kj", "power", "watts"],
    "weight": ["weight", "weight_kg", "weight_lbs", "mass", "wt"],
    "QuantitativeValue": ["value", "quantity", "amount", "measurement", "metric"],
    "price": ["price", "cost", "amount", "Price", "unit_price", "total_price"],
    "priceRange": ["price_range", "cost_range", "pricing", "price_bracket"],
    "currency": ["currency", "currency_code", "curr", "monetary_unit"],
    "MonetaryAmount": ["amount", "monetary_amount", "payment", "total", "sum"],
    "CoordinateAT": ["coordinate", "lat", "lng", "coord", "latitude", "longitude"],
    "Rating": ["rating", "score", "stars", "review_score", "rank"],
    "typicalAgeRange": ["age_range", "age", "target_age", "age_group"],
    # GEOGRAPHIC
    "addressLocality": ["city", "locality", "town", "City", "municipality", "place"],
    "addressRegion": ["region", "state", "province", "Region", "area", "district"],
    "Country": ["country", "nation", "Country", "country_name", "state"],
    "streetAddress": ["address", "street_address", "street", "Address", "location"],
    "PostalAddress": ["postal_address", "mailing_address", "full_address", "address"],
    # TEXT
    "Text": ["text", "description", "content", "notes", "details", "info"],
    "category": ["category", "type", "class", "group", "Category", "kind"],
    "ItemAvailability": ["availability", "in_stock", "status", "stock_status"],
    "ItemList": ["items", "list", "item_list", "entries"],
    "Review": ["review", "feedback", "comment", "Review", "testimonial"],
    "EventStatusType": ["status", "event_status", "state"],
    "BookFormatType": ["format", "book_format", "edition_type"],
    "Language": ["language", "lang", "Language", "locale"],
    "Thing": ["thing", "item", "object", "entity"],
    "GenderType": ["gender", "sex", "Gender"],
    "EventAttendanceModeEnumeration": ["attendance_mode", "event_type", "format"],
    "OccupationalExperienceRequirements": ["experience", "requirements", "qualifications"],
    "unitText": ["unit", "unit_text", "measurement_unit", "uom"],
    "OfferItemCondition": ["condition", "item_condition", "quality"],
    "Boolean": ["is_active", "flag", "enabled", "active", "boolean"],
    "paymentAccepted": ["payment", "payment_method", "pay_type"],
    "Photograph": ["photo", "image", "picture", "photograph"],
    "Offer": ["offer", "deal", "promotion", "discount"],
    "Action": ["action", "activity", "operation", "task"],
    "DeliveryMethod": ["delivery", "shipping_method", "delivery_type"],
    "RestrictedDiet": ["diet", "dietary", "restriction", "food_preference"],
    "Product": ["product", "item", "Product", "goods"],
    "LocationFeatureSpecification": ["feature", "amenity", "facility"],
    "audience": ["audience", "target_audience", "demographic"],
    "MusicRecording": ["recording", "track", "song", "audio"],
    "WarrantyPromise": ["warranty", "guarantee", "coverage"],
    "EducationalOccupationalCredential": ["credential", "certification", "qualification"],
}


def sample_values(values: list[str], max_n: int, seed: int = 42) -> list[str]:
    """Sample up to max_n values using stratified frequency-weighted sampling.

    Strategy: take top-K most frequent values to preserve distribution signal,
    then fill remaining slots with random diverse values. This gives the model
    both common patterns and rare variants.
    """
    if len(values) <= max_n:
        return values

    rng = random.Random(seed)

    # Count frequencies
    freq = Counter(values)
    unique_vals = list(freq.keys())

    # Take top half by frequency
    top_k = max_n // 2
    by_freq = sorted(unique_vals, key=lambda v: freq[v], reverse=True)
    selected = set(by_freq[:top_k])

    # Fill remaining with random diverse values (not already selected)
    remaining = [v for v in unique_vals if v not in selected]
    fill_n = max_n - len(selected)
    if fill_n > 0 and remaining:
        rng.shuffle(remaining)
        selected.update(remaining[:fill_n])

    # If still short (fewer unique values than max_n), add duplicates
    result = list(selected)
    if len(result) < max_n:
        extras = [v for v in values if v not in selected]
        rng.shuffle(extras)
        result.extend(extras[: max_n - len(result)])

    return result[:max_n]


def load_sotab_columns(
    parquet_path: Path,
) -> list[dict]:
    """Load SOTAB columns and group values."""
    con = duckdb.connect()
    rows = con.execute(
        f"""SELECT table_name, col_index, gt_label, col_value
            FROM read_parquet('{parquet_path}')"""
    ).fetchall()
    con.close()

    # Group by column
    columns: dict[tuple[str, int], list[str]] = defaultdict(list)
    gt_labels: dict[tuple[str, int], str] = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = (table_name, int(col_index))
        if col_value is not None:
            columns[key].append(str(col_value))
        gt_labels[key] = gt_label

    result = []
    for key, values in columns.items():
        table_name, col_index = key
        gt_label = gt_labels[key]
        broad_cat = BROAD_CATEGORY_MAP.get(gt_label)
        if broad_cat is None:
            continue  # Skip unmapped labels

        entity_subtype = ENTITY_SUBTYPE_MAP.get(gt_label)

        result.append({
            "table_name": table_name,
            "col_index": col_index,
            "gt_label": gt_label,
            "broad_category": broad_cat,
            "entity_subtype": entity_subtype,  # None for non-entity columns
            "values": values,
        })

    return result


def load_profile_columns(
    datasets_dir: Path,
    manifest_path: Path,
    schema_mapping_path: Path,
) -> list[dict]:
    """Load profile eval columns with real headers and authoritative category mapping.

    Uses FINETYPE_TO_BROAD (mirrors Rust LabelCategoryMap) for accurate
    broad category assignment instead of the approximate domain-based heuristic.
    """
    import yaml

    # Load schema mapping for gt_label → finetype_label lookup
    with open(schema_mapping_path) as f:
        data = yaml.safe_load(f)

    label_to_ft = {}
    for entry in data["mappings"]:
        ft = entry.get("finetype_label")
        if ft:
            label_to_ft[entry["gt_label"]] = ft

    # Load manifest
    manifest_rows = []
    with open(manifest_path) as f:
        reader = csv.DictReader(f)
        for row in reader:
            manifest_rows.append(row)

    result = []
    skipped = []
    for row in manifest_rows:
        dataset = row["dataset"]
        file_path = Path(row["file_path"])
        col_name = row["column_name"]
        gt_label = row["gt_label"]

        # Resolve file path
        if not file_path.is_absolute():
            file_path = datasets_dir / file_path

        if not file_path.exists():
            skipped.append(f"{dataset}.{col_name}: file not found ({file_path})")
            continue

        # Read column values
        try:
            con = duckdb.connect()
            values = con.execute(
                f"""SELECT CAST("{col_name}" AS VARCHAR)
                    FROM read_csv_auto('{file_path}')
                    WHERE "{col_name}" IS NOT NULL"""
            ).fetchall()
            con.close()
            values = [str(v[0]) for v in values if v[0] is not None]
        except Exception as e:
            skipped.append(f"{dataset}.{col_name}: read error ({e})")
            continue

        if not values:
            skipped.append(f"{dataset}.{col_name}: no values")
            continue

        # Map gt_label → finetype_label → broad category (authoritative)
        ft_label = label_to_ft.get(gt_label, "")
        broad_cat = FINETYPE_TO_BROAD.get(ft_label)
        if broad_cat is None:
            # Fallback: conservative domain-based guess
            if ft_label:
                domain = ft_label.split(".")[0]
                broad_cat = {
                    "datetime": "temporal", "geography": "geographic",
                    "container": "format",
                }.get(domain, "text")
            else:
                broad_cat = "text"

        # Entity subtype for person columns
        entity_subtype = PROFILE_ENTITY_SUBTYPES.get(ft_label)

        result.append({
            "table_name": f"profile_{dataset}",
            "col_index": 0,
            "gt_label": gt_label,
            "broad_category": broad_cat,
            "entity_subtype": entity_subtype,
            "header": col_name,  # Profile eval has real column names!
            "values": values,
        })

    if skipped:
        print(f"  Skipped {len(skipped)} columns:")
        for s in skipped[:5]:
            print(f"    {s}")
        if len(skipped) > 5:
            print(f"    ... and {len(skipped) - 5} more")

    return result


def assign_synthetic_headers(
    columns: list[dict],
    header_fraction: float,
    rng: random.Random,
) -> int:
    """Assign synthetic headers to a fraction of headerless columns.

    Returns the number of columns that received a synthetic header.
    """
    assigned = 0
    for col in columns:
        if col.get("header") is not None:
            continue  # Already has a header (e.g. profile eval)

        if rng.random() >= header_fraction:
            continue  # Leave headerless (maintains no-header capacity)

        templates = SYNTHETIC_HEADERS.get(col["gt_label"])
        if templates:
            col["header"] = rng.choice(templates)
            assigned += 1

    return assigned


def main():
    parser = argparse.ArgumentParser(description="Prepare Sense model training data")
    parser.add_argument(
        "--max-values", type=int, default=50,
        help="Max values to sample per column (default: 50)",
    )
    parser.add_argument(
        "--output", type=str, default="data/sense_spike",
        help="Output directory (default: data/sense_spike)",
    )
    parser.add_argument(
        "--seed", type=int, default=42,
        help="Random seed (default: 42)",
    )
    parser.add_argument(
        "--val-fraction", type=float, default=0.2,
        help="Validation fraction for SOTAB (default: 0.2)",
    )
    parser.add_argument(
        "--sotab-dir", type=str,
        default=str(Path.home() / "datasets/sotab/cta"),
        help="SOTAB CTA directory",
    )
    # Production mode flags
    parser.add_argument(
        "--include-profile", action="store_true",
        help="Include profile eval columns with real headers",
    )
    parser.add_argument(
        "--profile-repeat", type=int, default=50,
        help="Repeat profile eval columns N times (default: 50)",
    )
    parser.add_argument(
        "--synthetic-headers", action="store_true",
        help="Generate synthetic headers for SOTAB columns",
    )
    parser.add_argument(
        "--header-fraction", type=float, default=0.5,
        help="Fraction of SOTAB columns to give synthetic headers (default: 0.5)",
    )
    parser.add_argument(
        "--datasets-dir", type=str,
        default=str(Path.home() / "datasets"),
        help="Datasets directory for profile eval (default: ~/datasets)",
    )
    parser.add_argument(
        "--manifest", type=str,
        default="eval/datasets/manifest.csv",
        help="Profile eval manifest (default: eval/datasets/manifest.csv)",
    )
    parser.add_argument(
        "--schema-mapping", type=str,
        default="eval/schema_mapping.yaml",
        help="Schema mapping YAML (default: eval/schema_mapping.yaml)",
    )
    args = parser.parse_args()

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    sotab_dir = Path(args.sotab_dir)
    rng = random.Random(args.seed)

    # ── Load SOTAB columns ──
    val_parquet = sotab_dir / "validation" / "column_values.parquet"
    print(f"Loading SOTAB validation from {val_parquet}...")
    columns = load_sotab_columns(val_parquet)
    print(f"  Loaded {len(columns)} columns")

    test_parquet = sotab_dir / "test" / "column_values.parquet"
    if test_parquet.exists():
        print(f"Loading SOTAB test from {test_parquet}...")
        test_cols = load_sotab_columns(test_parquet)
        print(f"  Loaded {len(test_cols)} columns")
        columns.extend(test_cols)

    # ── Assign synthetic headers to SOTAB ──
    if args.synthetic_headers:
        n_assigned = assign_synthetic_headers(columns, args.header_fraction, rng)
        n_with_header = sum(1 for c in columns if c.get("header") is not None)
        print(f"\nSynthetic headers: assigned {n_assigned} headers "
              f"({n_with_header}/{len(columns)} columns now have headers)")

    # ── Load profile eval columns ──
    profile_cols = []
    if args.include_profile:
        print(f"\nLoading profile eval columns...")
        profile_cols = load_profile_columns(
            Path(args.datasets_dir),
            Path(args.manifest),
            Path(args.schema_mapping),
        )
        print(f"  Loaded {len(profile_cols)} columns with real headers")

        # Show profile category distribution
        profile_cats = Counter(c["broad_category"] for c in profile_cols)
        print("  Category distribution:")
        for cat in BROAD_CATEGORIES:
            print(f"    {cat:12s}: {profile_cats.get(cat, 0):3d}")

    # ── Category distribution (SOTAB) ──
    cat_counts = Counter(c["broad_category"] for c in columns)
    print(f"\nSOTAB broad category distribution ({len(columns)} total):")
    for cat in BROAD_CATEGORIES:
        print(f"  {cat:12s}: {cat_counts.get(cat, 0):5d}")

    entity_cols = [c for c in columns if c["broad_category"] == "entity"]
    sub_counts = Counter(c["entity_subtype"] for c in entity_cols)
    print(f"\nEntity subtype distribution ({len(entity_cols)} entity columns):")
    for sub in ENTITY_SUBTYPES:
        print(f"  {sub:15s}: {sub_counts.get(sub, 0):5d}")

    # ── Sample values ──
    print(f"\nSampling up to {args.max_values} values per column...")
    all_cols = columns + profile_cols
    for col in all_cols:
        col["sampled_values"] = sample_values(
            col["values"], args.max_values, seed=args.seed
        )
        if "header" not in col:
            col["header"] = None

    # ── Train/val split ──
    # SOTAB: stratified 80/20 split (val set monitors training progress)
    # Profile eval: ALL go into training (repeated N times)
    # Real acceptance test is profile_eval.sh on the Rust pipeline
    print(f"\nSplitting SOTAB train/val ({1-args.val_fraction:.0%}/{args.val_fraction:.0%})...")

    by_category: dict[str, list[dict]] = defaultdict(list)
    for col in columns:
        by_category[col["broad_category"]].append(col)

    train_cols = []
    val_cols = []
    for cat, cat_cols in by_category.items():
        rng.shuffle(cat_cols)
        split_idx = int(len(cat_cols) * (1 - args.val_fraction))
        train_cols.extend(cat_cols[:split_idx])
        val_cols.extend(cat_cols[split_idx:])

    # Add profile eval columns to training (repeated N times)
    if profile_cols and args.profile_repeat > 0:
        n_profile_added = len(profile_cols) * args.profile_repeat
        for _ in range(args.profile_repeat):
            train_cols.extend(profile_cols)
        print(f"  Added {n_profile_added} profile eval rows "
              f"({len(profile_cols)} columns × {args.profile_repeat} repeats)")

    rng.shuffle(train_cols)
    rng.shuffle(val_cols)

    print(f"  Train: {len(train_cols)}, Val: {len(val_cols)}")

    # ── Header coverage stats ──
    train_with_header = sum(1 for c in train_cols if c.get("header") is not None)
    val_with_header = sum(1 for c in val_cols if c.get("header") is not None)
    print(f"  Headers in train: {train_with_header}/{len(train_cols)} "
          f"({train_with_header * 100 / max(len(train_cols), 1):.1f}%)")
    print(f"  Headers in val: {val_with_header}/{len(val_cols)} "
          f"({val_with_header * 100 / max(len(val_cols), 1):.1f}%)")

    # ── Write output ──
    def write_jsonl(path: Path, data: list[dict]):
        with open(path, "w") as f:
            for item in data:
                record = {
                    "table_name": item["table_name"],
                    "col_index": item["col_index"],
                    "gt_label": item["gt_label"],
                    "broad_category": item["broad_category"],
                    "broad_category_idx": BROAD_CATEGORIES.index(item["broad_category"]),
                    "entity_subtype": item["entity_subtype"],
                    "entity_subtype_idx": (
                        ENTITY_SUBTYPES.index(item["entity_subtype"])
                        if item["entity_subtype"]
                        else -1
                    ),
                    "header": item.get("header"),
                    "values": item["sampled_values"],
                    "n_original_values": len(item["values"]),
                }
                f.write(json.dumps(record, ensure_ascii=False) + "\n")

    train_path = output_dir / "train.jsonl"
    val_path = output_dir / "val.jsonl"

    write_jsonl(train_path, train_cols)
    write_jsonl(val_path, val_cols)

    # Write metadata
    meta = {
        "broad_categories": BROAD_CATEGORIES,
        "entity_subtypes": ENTITY_SUBTYPES,
        "max_values": args.max_values,
        "seed": args.seed,
        "n_train": len(train_cols),
        "n_val": len(val_cols),
        "include_profile": args.include_profile,
        "profile_repeat": args.profile_repeat if args.include_profile else 0,
        "n_profile_columns": len(profile_cols),
        "synthetic_headers": args.synthetic_headers,
        "header_fraction": args.header_fraction if args.synthetic_headers else 0,
        "broad_category_map": BROAD_CATEGORY_MAP,
        "entity_subtype_map": ENTITY_SUBTYPE_MAP,
    }
    with open(output_dir / "meta.json", "w") as f:
        json.dump(meta, f, indent=2)

    print(f"\nWrote {train_path} ({len(train_cols)} columns)")
    print(f"Wrote {val_path} ({len(val_cols)} columns)")
    print(f"Wrote {output_dir / 'meta.json'}")

    # ── Summary stats ──
    train_cat = Counter(c["broad_category"] for c in train_cols)
    val_cat = Counter(c["broad_category"] for c in val_cols)
    print("\nCategory split:")
    print(f"  {'Category':12s} {'Train':>6s} {'Val':>6s}")
    for cat in BROAD_CATEGORIES:
        print(f"  {cat:12s} {train_cat.get(cat, 0):6d} {val_cat.get(cat, 0):6d}")


if __name__ == "__main__":
    main()
