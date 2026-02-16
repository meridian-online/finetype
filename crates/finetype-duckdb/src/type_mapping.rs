//! Maps finetype semantic labels to recommended DuckDB types.
//!
//! This module provides the `to_duckdb_type()` function which maps each of the
//! 151 finetype labels to the most appropriate DuckDB logical type. These mappings
//! represent the optimal CAST target for each detected type.

/// Map a finetype label to the recommended DuckDB logical type.
///
/// Returns the DuckDB type name (e.g., "INET", "UUID", "TIMESTAMP") that best
/// represents the semantic type. Falls back to "VARCHAR" for unrecognized labels.
pub fn to_duckdb_type(label: &str) -> &'static str {
    match label {
        // ── datetime.date ──────────────────────────────────────────────
        "datetime.date.iso"
        | "datetime.date.us_slash"
        | "datetime.date.eu_slash"
        | "datetime.date.eu_dot"
        | "datetime.date.long_full_month"
        | "datetime.date.abbreviated_month"
        | "datetime.date.weekday_full_month"
        | "datetime.date.weekday_abbreviated_month"
        | "datetime.date.ordinal"
        | "datetime.date.julian"
        | "datetime.date.iso_week"
        | "datetime.date.compact_ymd"
        | "datetime.date.compact_mdy"
        | "datetime.date.compact_dmy"
        | "datetime.date.short_ymd"
        | "datetime.date.short_mdy"
        | "datetime.date.short_dmy" => "DATE",

        // ── datetime.time ──────────────────────────────────────────────
        "datetime.time.hm_24h"
        | "datetime.time.hms_24h"
        | "datetime.time.hm_12h"
        | "datetime.time.hms_12h"
        | "datetime.time.iso" => "TIME",

        // ── datetime.timestamp ─────────────────────────────────────────
        "datetime.timestamp.iso_8601"
        | "datetime.timestamp.iso_8601_compact"
        | "datetime.timestamp.iso_8601_microseconds"
        | "datetime.timestamp.iso_microseconds"
        | "datetime.timestamp.american"
        | "datetime.timestamp.american_24h"
        | "datetime.timestamp.european"
        | "datetime.timestamp.sql_standard" => "TIMESTAMP",

        "datetime.timestamp.iso_8601_offset"
        | "datetime.timestamp.rfc_2822"
        | "datetime.timestamp.rfc_2822_ordinal"
        | "datetime.timestamp.rfc_3339" => "TIMESTAMPTZ",

        // ── datetime.epoch ─────────────────────────────────────────────
        "datetime.epoch.unix_seconds" => "BIGINT",
        "datetime.epoch.unix_milliseconds" => "BIGINT",
        "datetime.epoch.unix_microseconds" => "BIGINT",

        // ── datetime.duration ──────────────────────────────────────────
        "datetime.duration.iso_8601" => "INTERVAL",

        // ── datetime.component ─────────────────────────────────────────
        "datetime.component.year" => "INTEGER",
        "datetime.component.day_of_month" => "INTEGER",
        "datetime.component.century" => "VARCHAR",
        "datetime.component.day_of_week" => "VARCHAR",
        "datetime.component.month_name" => "VARCHAR",
        "datetime.component.periodicity" => "VARCHAR",

        // ── datetime.offset ────────────────────────────────────────────
        "datetime.offset.utc" | "datetime.offset.iana" => "VARCHAR",

        // ── technology.internet ────────────────────────────────────────
        "technology.internet.ip_v4"
        | "technology.internet.ip_v6"
        | "technology.internet.ip_v4_with_port" => "INET",
        "technology.internet.mac_address" => "VARCHAR",
        "technology.internet.url"
        | "technology.internet.uri"
        | "technology.internet.slug"
        | "technology.internet.hostname"
        | "technology.internet.top_level_domain"
        | "technology.internet.user_agent" => "VARCHAR",
        "technology.internet.http_method" => "VARCHAR",
        "technology.internet.http_status_code" => "SMALLINT",
        "technology.internet.port" => "SMALLINT",

        // ── technology.cryptographic ───────────────────────────────────
        "technology.cryptographic.uuid" => "UUID",
        "technology.cryptographic.hash"
        | "technology.cryptographic.token_hex"
        | "technology.cryptographic.token_urlsafe" => "VARCHAR",

        // ── technology.development ─────────────────────────────────────
        "technology.development.boolean" => "BOOLEAN", // legacy label
        "technology.development.version"
        | "technology.development.calver"
        | "technology.development.programming_language"
        | "technology.development.software_license"
        | "technology.development.os"
        | "technology.development.stage" => "VARCHAR",

        // ── technology.hardware ────────────────────────────────────────
        "technology.hardware.ram_size" => "BIGINT",
        "technology.hardware.screen_size" => "DOUBLE",
        "technology.hardware.cpu" | "technology.hardware.generation" => "VARCHAR",

        // ── technology.code ────────────────────────────────────────────
        "technology.code.ean"
        | "technology.code.isbn"
        | "technology.code.issn"
        | "technology.code.imei"
        | "technology.code.locale_code"
        | "technology.code.pin" => "VARCHAR",

        // ── geography.coordinate ───────────────────────────────────────
        "geography.coordinate.latitude" | "geography.coordinate.longitude" => "DOUBLE",
        "geography.coordinate.coordinates" => "POINT",

        // ── geography.location ─────────────────────────────────────────
        "geography.location.city"
        | "geography.location.country"
        | "geography.location.country_code"
        | "geography.location.continent"
        | "geography.location.region" => "VARCHAR",

        // ── geography.address ──────────────────────────────────────────
        "geography.address.full_address" | "geography.address.street_name" => "VARCHAR",
        "geography.address.street_number" => "INTEGER",
        "geography.address.postal_code" | "geography.address.street_suffix" => "VARCHAR",

        // ── geography.contact ──────────────────────────────────────────
        "geography.contact.calling_code" => "VARCHAR",

        // ── geography.transportation ───────────────────────────────────
        "geography.transportation.iata_code" | "geography.transportation.icao_code" => "VARCHAR",

        // ── identity.person ────────────────────────────────────────────
        "identity.person.first_name"
        | "identity.person.last_name"
        | "identity.person.full_name"
        | "identity.person.username"
        | "identity.person.email"
        | "identity.person.phone_number"
        | "identity.person.password"
        | "identity.person.gender"
        | "identity.person.gender_code"
        | "identity.person.gender_symbol"
        | "identity.person.nationality"
        | "identity.person.occupation"
        | "identity.person.blood_type" => "VARCHAR",
        "identity.person.age" => "SMALLINT",
        "identity.person.height" | "identity.person.weight" => "DOUBLE",

        // ── identity.academic ──────────────────────────────────────────
        "identity.academic.degree" | "identity.academic.university" => "VARCHAR",

        // ── identity.payment ───────────────────────────────────────────
        "identity.payment.credit_card_number"
        | "identity.payment.credit_card_network"
        | "identity.payment.credit_card_expiration_date"
        | "identity.payment.cvv"
        | "identity.payment.bitcoin_address"
        | "identity.payment.ethereum_address"
        | "identity.payment.paypal_email" => "VARCHAR",

        // ── representation.numeric ─────────────────────────────────────
        "representation.numeric.integer_number" | "representation.numeric.increment" => "BIGINT",
        "representation.numeric.decimal_number"
        | "representation.numeric.percentage"
        | "representation.numeric.scientific_notation" => "DOUBLE",

        // ── representation.text ────────────────────────────────────────
        "representation.text.word"
        | "representation.text.sentence"
        | "representation.text.plain_text"
        | "representation.text.emoji"
        | "representation.text.color_hex"
        | "representation.text.color_rgb" => "VARCHAR",

        // ── representation.file ────────────────────────────────────────
        "representation.file.file_size" => "BIGINT",
        "representation.file.extension" | "representation.file.mime_type" => "VARCHAR",

        // ── representation.scientific ──────────────────────────────────
        "representation.scientific.dna_sequence"
        | "representation.scientific.rna_sequence"
        | "representation.scientific.protein_sequence"
        | "representation.scientific.measurement_unit"
        | "representation.scientific.metric_prefix" => "VARCHAR",

        // ── representation.boolean ───────────────────────────────────
        "representation.boolean.binary"
        | "representation.boolean.initials"
        | "representation.boolean.terms" => "BOOLEAN",

        // ── container.object ───────────────────────────────────────────
        "container.object.json" | "container.object.json_array" => "JSON",
        "container.object.xml" | "container.object.yaml" | "container.object.csv" => "VARCHAR",

        // ── container.array ────────────────────────────────────────────
        "container.array.comma_separated"
        | "container.array.pipe_separated"
        | "container.array.semicolon_separated"
        | "container.array.whitespace_separated" => "VARCHAR",

        // ── container.key_value ────────────────────────────────────────
        "container.key_value.query_string" | "container.key_value.form_data" => "VARCHAR",

        // ── Fallback ───────────────────────────────────────────────────
        _ => "VARCHAR",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_types() {
        assert_eq!(to_duckdb_type("technology.internet.ip_v4"), "INET");
        assert_eq!(to_duckdb_type("technology.cryptographic.uuid"), "UUID");
        assert_eq!(to_duckdb_type("datetime.date.iso"), "DATE");
        assert_eq!(to_duckdb_type("datetime.timestamp.rfc_3339"), "TIMESTAMPTZ");
        assert_eq!(to_duckdb_type("container.object.json"), "JSON");
        assert_eq!(
            to_duckdb_type("representation.numeric.integer_number"),
            "BIGINT"
        );
        assert_eq!(to_duckdb_type("geography.coordinate.latitude"), "DOUBLE");
        assert_eq!(to_duckdb_type("technology.development.boolean"), "BOOLEAN");
        // NNFT-075: new boolean subtypes
        assert_eq!(to_duckdb_type("representation.boolean.binary"), "BOOLEAN");
        assert_eq!(to_duckdb_type("representation.boolean.initials"), "BOOLEAN");
        assert_eq!(to_duckdb_type("representation.boolean.terms"), "BOOLEAN");
    }

    #[test]
    fn test_unknown_fallback() {
        assert_eq!(to_duckdb_type("some.unknown.type"), "VARCHAR");
        assert_eq!(to_duckdb_type(""), "VARCHAR");
    }
}
