/// Check whether two timestamp subtypes belong to the same format family.
/// Types within a family are precision or offset variants of each other —
/// predicting iso_8601 when the GT is iso_8601_milliseconds is acceptable,
/// but predicting iso_8601 when the GT is clf or mdy_12h is not.
fn is_same_timestamp_family(a: &str, b: &str) -> bool {
    const FAMILIES: &[&[&str]] = &[
        // ISO 8601 family (precision and offset variants, including RFC 3339)
        &[
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.iso_8601_compact",
            "datetime.timestamp.iso_8601_microseconds",
            "datetime.timestamp.iso_8601_offset",
            "datetime.timestamp.iso_8601_milliseconds",
            "datetime.timestamp.iso_8601_millis_offset",
            "datetime.timestamp.iso_8601_micros_offset",
            "datetime.timestamp.rfc_3339",
            "datetime.timestamp.iso_microseconds",
            "datetime.timestamp.iso_space_zulu",
        ],
        // SQL family (precision and offset variants)
        &[
            "datetime.timestamp.sql_standard",
            "datetime.timestamp.sql_microseconds",
            "datetime.timestamp.sql_milliseconds",
            "datetime.timestamp.sql_microseconds_offset",
        ],
        // RFC 2822 family
        &[
            "datetime.timestamp.rfc_2822",
            "datetime.timestamp.rfc_2822_ordinal",
        ],
        // US MDY family (12h/24h variants)
        &[
            "datetime.timestamp.mdy_12h",
            "datetime.timestamp.mdy_24h",
        ],
        // EU DMY family (separator variants)
        &[
            "datetime.timestamp.dmy_hm",
            "datetime.timestamp.dot_dmy_24h",
        ],
        // YMD non-ISO family (separator variants)
        &[
            "datetime.timestamp.slash_ymd_24h",
            "datetime.timestamp.dot_ymd_24h",
        ],
    ];

    FAMILIES.iter().any(|family| family.contains(&a) && family.contains(&b))
}

/// Check label match with interchangeability rules (matches eval_profile.sql).
pub fn is_label_match(predicted: &str, expected: &str) -> bool {
    if predicted == expected {
        return true;
    }
    // Boolean sub-types are interchangeable
    if expected.starts_with("representation.boolean.")
        && predicted.starts_with("representation.boolean.")
    {
        return true;
    }
    // Time sub-types are interchangeable
    if expected.starts_with("datetime.time.") && predicted.starts_with("datetime.time.") {
        return true;
    }
    // Timestamp sub-types: only format-compatible families are interchangeable.
    // Previously all datetime.timestamp.* were treated as equivalent, which masked
    // real errors (e.g., iso_8601 predicted for clf or mdy_12h counted as correct).
    if expected.starts_with("datetime.timestamp.") && predicted.starts_with("datetime.timestamp.") {
        return is_same_timestamp_family(predicted, expected);
    }
    // Geographic hierarchy interchangeable
    const GEO_SET: &[&str] = &[
        "geography.location.region",
        "geography.location.state",
        "geography.location.continent",
        "geography.location.country",
    ];
    if GEO_SET.contains(&expected) && GEO_SET.contains(&predicted) {
        return true;
    }
    // entity_name satisfies full_name GT label
    if expected == "identity.person.full_name" && predicted == "representation.text.entity_name" {
        return true;
    }
    // DMY/MDY dash dates are inherently ambiguous (DD-MM vs MM-DD)
    const DMY_MDY_SET: &[&str] = &["datetime.date.dmy_dash", "datetime.date.mdy_dash"];
    if DMY_MDY_SET.contains(&expected) && DMY_MDY_SET.contains(&predicted) {
        return true;
    }
    // Coordinate subtypes: latitude/longitude vs coordinates (isolated decimals need header)
    const COORD_SET: &[&str] = &[
        "geography.coordinate.latitude",
        "geography.coordinate.longitude",
        "geography.coordinate.coordinates",
    ];
    if COORD_SET.contains(&expected) && COORD_SET.contains(&predicted) {
        return true;
    }
    // Hash subtypes: git_sha (40-char SHA-1) is a valid specific subtype of hash
    const HASH_SET: &[&str] = &[
        "technology.cryptographic.hash",
        "technology.development.git_sha",
    ];
    if HASH_SET.contains(&expected) && HASH_SET.contains(&predicted) {
        return true;
    }
    // JSON subtypes: geojson is a specific subtype of json
    if (expected == "container.object.json" && predicted == "geography.format.geojson")
        || (expected == "geography.format.geojson" && predicted == "container.object.json")
    {
        return true;
    }
    // IP hierarchy: ip_v4 captures the core format of ip_v4_with_port
    if expected == "technology.internet.ip_v4_with_port" && predicted == "technology.internet.ip_v4"
    {
        return true;
    }
    // Categorical is a valid generic parent for http_method and measurement_unit
    if (expected == "technology.internet.http_method"
        || expected == "representation.scientific.measurement_unit")
        && predicted == "representation.discrete.categorical"
    {
        return true;
    }
    false
}

/// Check domain match with interchangeability rules.
pub fn is_domain_match(predicted: &str, expected_label: &str, expected_domain: &str) -> bool {
    let pred_domain = predicted.split('.').next().unwrap_or("");
    if pred_domain == expected_domain {
        return true;
    }
    // entity_name in representation domain satisfies identity-domain "name" GT
    if expected_label == "identity.person.full_name"
        && predicted == "representation.text.entity_name"
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(is_label_match("datetime.date.iso", "datetime.date.iso"));
    }

    #[test]
    fn test_boolean_interchangeable() {
        assert!(is_label_match(
            "representation.boolean.binary",
            "representation.boolean.true_false"
        ));
    }

    #[test]
    fn test_time_interchangeable() {
        assert!(is_label_match(
            "datetime.time.hms_24h",
            "datetime.time.hm_24h"
        ));
    }

    #[test]
    fn test_timestamp_iso_family_interchangeable() {
        // Within ISO family — precision variants are interchangeable
        assert!(is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.iso_8601_microseconds"
        ));
        assert!(is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.rfc_3339"
        ));
        assert!(is_label_match(
            "datetime.timestamp.iso_8601_milliseconds",
            "datetime.timestamp.iso_8601_offset"
        ));
    }

    #[test]
    fn test_timestamp_sql_family_interchangeable() {
        assert!(is_label_match(
            "datetime.timestamp.sql_standard",
            "datetime.timestamp.sql_microseconds"
        ));
    }

    #[test]
    fn test_timestamp_cross_family_not_interchangeable() {
        // ISO vs CLF — different format families
        assert!(!is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.clf"
        ));
        // ISO vs MDY — different format families
        assert!(!is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.mdy_12h"
        ));
        // ISO vs DMY — different format families
        assert!(!is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.dmy_hm"
        ));
        // ISO vs syslog — different format families
        assert!(!is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.syslog_bsd"
        ));
        // CLF vs syslog — both standalone, not interchangeable
        assert!(!is_label_match(
            "datetime.timestamp.clf",
            "datetime.timestamp.syslog_bsd"
        ));
    }

    #[test]
    fn test_timestamp_mdy_family_interchangeable() {
        assert!(is_label_match(
            "datetime.timestamp.mdy_12h",
            "datetime.timestamp.mdy_24h"
        ));
    }

    #[test]
    fn test_geo_interchangeable() {
        assert!(is_label_match(
            "geography.location.region",
            "geography.location.state"
        ));
    }

    #[test]
    fn test_geo_country_interchangeable() {
        assert!(is_label_match(
            "geography.location.region",
            "geography.location.country"
        ));
        assert!(is_label_match(
            "geography.location.country",
            "geography.location.region"
        ));
    }

    #[test]
    fn test_entity_name_satisfies_full_name() {
        assert!(is_label_match(
            "representation.text.entity_name",
            "identity.person.full_name"
        ));
    }

    #[test]
    fn test_dmy_mdy_interchangeable() {
        assert!(is_label_match(
            "datetime.date.dmy_dash",
            "datetime.date.mdy_dash"
        ));
        assert!(is_label_match(
            "datetime.date.mdy_dash",
            "datetime.date.dmy_dash"
        ));
    }

    #[test]
    fn test_coordinate_subtypes_interchangeable() {
        assert!(is_label_match(
            "geography.coordinate.coordinates",
            "geography.coordinate.latitude"
        ));
        assert!(is_label_match(
            "geography.coordinate.coordinates",
            "geography.coordinate.longitude"
        ));
        assert!(is_label_match(
            "geography.coordinate.latitude",
            "geography.coordinate.coordinates"
        ));
    }

    #[test]
    fn test_hash_git_sha_interchangeable() {
        assert!(is_label_match(
            "technology.development.git_sha",
            "technology.cryptographic.hash"
        ));
        assert!(is_label_match(
            "technology.cryptographic.hash",
            "technology.development.git_sha"
        ));
    }

    #[test]
    fn test_json_geojson_interchangeable() {
        assert!(is_label_match(
            "geography.format.geojson",
            "container.object.json"
        ));
        assert!(is_label_match(
            "container.object.json",
            "geography.format.geojson"
        ));
    }

    #[test]
    fn test_ip_v4_satisfies_ip_v4_with_port() {
        assert!(is_label_match(
            "technology.internet.ip_v4",
            "technology.internet.ip_v4_with_port"
        ));
        // But not the reverse — ip_v4_with_port doesn't satisfy plain ip_v4
        assert!(!is_label_match(
            "technology.internet.ip_v4_with_port",
            "technology.internet.ip_v4"
        ));
    }

    #[test]
    fn test_categorical_satisfies_http_method() {
        assert!(is_label_match(
            "representation.discrete.categorical",
            "technology.internet.http_method"
        ));
    }

    #[test]
    fn test_categorical_satisfies_measurement_unit() {
        assert!(is_label_match(
            "representation.discrete.categorical",
            "representation.scientific.measurement_unit"
        ));
    }

    #[test]
    fn test_no_false_match() {
        assert!(!is_label_match(
            "datetime.date.iso",
            "datetime.time.hms_24h"
        ));
    }

    #[test]
    fn test_domain_match() {
        assert!(is_domain_match(
            "datetime.date.iso",
            "datetime.date.mdy_slash",
            "datetime"
        ));
    }

    #[test]
    fn test_entity_name_domain_match() {
        assert!(is_domain_match(
            "representation.text.entity_name",
            "identity.person.full_name",
            "identity"
        ));
    }
}
