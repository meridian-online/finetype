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
    ];
    if GEO_SET.contains(&expected) && GEO_SET.contains(&predicted) {
        return true;
    }
    // entity_name satisfies full_name GT label
    if expected == "identity.person.full_name" && predicted == "representation.text.entity_name" {
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
    fn test_entity_name_satisfies_full_name() {
        assert!(is_label_match(
            "representation.text.entity_name",
            "identity.person.full_name"
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
            "datetime.date.us_slash",
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
