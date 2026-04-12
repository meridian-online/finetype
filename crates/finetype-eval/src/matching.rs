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
    // Timestamp sub-types are interchangeable
    if expected.starts_with("datetime.timestamp.") && predicted.starts_with("datetime.timestamp.") {
        return true;
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
    const DMY_MDY_SET: &[&str] = &[
        "datetime.date.dmy_dash",
        "datetime.date.mdy_dash",
    ];
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
    if expected == "technology.internet.ip_v4_with_port"
        && predicted == "technology.internet.ip_v4"
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
    fn test_timestamp_interchangeable() {
        assert!(is_label_match(
            "datetime.timestamp.iso_8601",
            "datetime.timestamp.iso_8601_microseconds"
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
