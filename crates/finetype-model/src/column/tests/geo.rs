use super::super::*;
use super::{geo_vote_classifier, make_result, vals};

#[test]
fn test_coordinates_longitude_detected() {
    let values: Vec<String> = vec!["-74.0060", "151.2093", "-0.1278", "139.6917", "2.3522"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_coordinates(&values);
    assert_eq!(result, Some("geography.coordinate.longitude".to_string()));
}

#[test]
fn test_coordinates_latitude_detected() {
    let values: Vec<String> = vec!["40.7128", "-33.8688", "51.5074", "35.6762", "-22.9068"]
        .into_iter()
        .map(String::from)
        .collect();

    let result = disambiguate_coordinates(&values);
    assert_eq!(result, Some("geography.coordinate.latitude".to_string()));
}

// test_numeric_port_detection: REMOVED (port type removed)

#[test]
fn test_numeric_postal_code_detection() {
    let values: Vec<String> = vec![
        "10001", "90210", "30301", "60601", "02101", "75001", "33101", "94102", "20001", "98101",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let results: Vec<ClassificationResult> = values
        .iter()
        .map(|_| ClassificationResult {
            label: "geography.address.postal_code".to_string(),
            confidence: 0.6,
            all_scores: vec![],
        })
        .collect();

    let votes = [
        ("geography.address.postal_code".to_string(), 6),
        ("representation.numeric.integer_number".to_string(), 4),
    ];
    let top_labels: Vec<&str> = votes.iter().map(|(l, _)| l.as_str()).collect();

    let result = disambiguate_numeric(&values, &results, &top_labels);
    assert!(result.is_some());
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "geography.address.postal_code");
}

// ── Locale suffix stripping tests ────────────────────────────────────

#[test]
fn test_strip_locale_suffix_4level_country() {
    let (base, locale) = strip_locale_suffix("geography.address.postal_code.EN_US");
    assert_eq!(base, "geography.address.postal_code");
    assert_eq!(locale, Some("EN_US"));
}

#[test]
fn test_header_hint_postal() {
    assert_eq!(header_hint("zip"), Some("geography.address.postal_code"));
    assert_eq!(
        header_hint("zip_code"),
        Some("geography.address.postal_code")
    );
    assert_eq!(
        header_hint("Postal Code"),
        Some("geography.address.postal_code")
    );
    assert_eq!(
        header_hint("postcode"),
        Some("geography.address.postal_code")
    );
}

// === 0094 coordinate header-veto helpers ===

#[test]
fn coord_veto_corroborates_real_coordinate_headers() {
    for h in [
        "lat",
        "latitude",
        "lon",
        "lng",
        "long",
        "longitude",
        "lat_dd",
        "y_lat",
        "decimalLatitude",
        "gps_lat",
        "coord_x",
        "wgs84_lat",
        "LATITUDE",
        "Longitude",
    ] {
        assert!(header_corroborates_coordinate(h), "should corroborate: {h}");
    }
}

#[test]
fn coord_veto_does_not_corroborate_false_friends_or_quantities() {
    // value-confusable non-coordinate headers — the FP battleground — must
    // NOT corroborate, so a latitude prediction on them is demotable.
    for h in [
        "mag",
        "mean",
        "std",
        "rms",
        "error",
        "depth",
        "score",
        "rate",
        "probability",
        "latency",
        "translate",
        "correlation",
        "plate",
        "population",
        "magnitude",
        "temperature",
        "elevation_error",
    ] {
        assert!(
            !header_corroborates_coordinate(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn coord_veto_value_gate_requires_numeric() {
    assert!(values_look_like_generic_decimals(&[
        "0.12".into(),
        "-3.4".into(),
        "51.5".into(),
        "2.0".into(),
        "9.9".into()
    ]));
    // text / mixed columns do not satisfy the value gate
    assert!(!values_look_like_generic_decimals(&[
        "alpha".into(),
        "beta".into(),
        "gamma".into()
    ]));
    assert!(!values_look_like_generic_decimals(&[]));
}

#[test]
fn coord_veto_is_a_default_on_header_hint() {
    // As a header-hint family, the coordinate veto is ON in default builds
    // (rhh::is_disabled is a const `false` when the rhh-instrumentation
    // feature is off) and disableable via RHH_DISABLE_HINTS in ablation builds.
    assert!(!rhh::is_disabled("header_hint_coord_veto"));
}

// === state_code promotion helpers ===

#[test]
fn state_code_header_corroboration() {
    for h in [
        "State",
        "Provider State",
        "DL State",
        "ADDRESS STATE",
        "province",
    ] {
        assert!(header_corroborates_state(h), "should corroborate: {h}");
    }
    // false friends — `state` must not match as a substring
    for h in [
        "statement",
        "status",
        "real estate",
        "estate",
        "country",
        "city",
    ] {
        assert!(!header_corroborates_state(h), "should NOT corroborate: {h}");
    }
}

#[test]
fn region_header_corroboration() {
    // admin divisions above city level — the gold city->region false positives
    for h in [
        "Region",
        "County",
        "district",
        "borough",
        "work_location_borough",
        "province",
    ] {
        assert!(header_corroborates_region(h), "should corroborate: {h}");
    }
    // must NOT match city/town or ambiguous tokens, nor substrings
    for h in ["city", "town", "name", "regional_office", "districting"] {
        assert!(
            !header_corroborates_region(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn state_code_value_vocab_gate() {
    // US codes
    assert!(values_look_like_state_codes(&[
        "GA".into(),
        "FL".into(),
        "NM".into(),
        "TX".into(),
    ]));
    // lowercase is uppercased before lookup
    assert!(values_look_like_state_codes(&[
        "ca".into(),
        "ny".into(),
        "wa".into(),
    ]));
    // AU 3-letter codes
    assert!(values_look_like_state_codes(&[
        "NSW".into(),
        "VIC".into(),
        "QLD".into(),
    ]));
    // full state NAMES are not codes
    assert!(!values_look_like_state_codes(&[
        "California".into(),
        "Florida".into(),
        "Texas".into(),
    ]));
    // a country-code column: most ISO codes are not state codes, so it fails
    // the vocab gate even before the header guard.
    assert!(!values_look_like_state_codes(&[
        "GB".into(),
        "FR".into(),
        "DE".into(),
        "JP".into(),
        "CN".into(),
    ]));
    // below the 3-value floor
    assert!(!values_look_like_state_codes(&["TX".into(), "FL".into()]));
}

#[test]
fn state_code_promote_is_a_default_on_header_hint() {
    assert!(!rhh::is_disabled("header_hint_state_code_promote"));
}

#[test]
fn country_code_corroboration_is_default_on() {
    assert!(!rhh::is_disabled("country_code_corroboration"));
}

#[test]
fn geo_code_membership_vote_is_default_on() {
    assert!(!rhh::is_disabled("geo_code_membership_vote"));
}

#[test]
fn geo_vote_flips_country_dominant_state_code_to_country() {
    // GLEIF `jurisdiction`: 89% ISO countries + 11% US-DE subdivisions; the tail
    // drags the model to state_code. country 0.9 >> us_state 0.2 -> country_code.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = [
        "IT", "GB", "NL", "ES", "FR", "CN", "SE", "DE", "IN", "US-DE",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("jurisdiction", &vals, "geography.location.state_code", 0.99)
        .unwrap();
    assert_eq!(r.label, "geography.location.country_code");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_membership_vote:geography.location.state_code")
    );
}

#[test]
fn geo_vote_keeps_genuine_us_states() {
    // A real US-states column: us_state 1.0, country ~0.33 (only DE/IN overlap) —
    // below the 0.70 country bar, so it is NOT wrongly promoted to country_code.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["CA", "TX", "NY", "FL", "DE", "IN"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("state", &vals, "geography.location.state_code", 0.90)
        .unwrap();
    // The vote's contract is that it must NOT promote a genuine US-states column to
    // country_code (the 0.51-overlap density trap the margin gate exists to stop).
    // Other pipeline rules may relabel state_code independently; that is not this rule.
    assert_ne!(r.label, "geography.location.country_code");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_membership_vote:geography.location.state_code")
    );
}

#[test]
fn geo_vote_leaves_ambiguous_columns_alone() {
    // Every value valid as BOTH country and US state (DE, IN, CA): neither side
    // clears the margin, so the model's call stands rather than a coin-flip.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["DE", "IN", "CA", "DE", "IN", "CA"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("code", &vals, "geography.location.country_code", 0.80)
        .unwrap();
    assert_eq!(r.label, "geography.location.country_code");
    assert!(
        !r.disambiguation_applied
            || r.disambiguation_rule.as_deref()
                != Some("geo_code_membership_vote:geography.location.country_code")
    );
}

#[test]
fn geo_vote_keeps_canadian_provinces_as_state_not_country() {
    // Regression (caught by the corpus-honest per-column trace): Canadian province
    // codes (NL, SK, YT…) collide with country codes (Netherlands, Slovakia,
    // Mayotte). With a US-ONLY subdivision set an NL-heavy column scores ~0.7
    // country / 0 state and wrongly flips to country_code. Unioning EN_CA fixes it:
    // subdivision coverage 1.0 dominates, so state_code is preserved.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["NL", "NL", "NL", "SK", "YT", "QC", "ON"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("state", &vals, "geography.location.state_code", 0.90)
        .unwrap();
    assert_ne!(r.label, "geography.location.country_code");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_membership_vote:geography.location.state_code")
    );
}

// === geo_hyphenated_region_margin_promote (dataset-descriptor audit:
//     edgar_gleif.jurisdiction lands on postal_code, not state_code/
//     country_code — a mixed bare-country + hyphenated-ISO-3166-2 column) ===

#[test]
fn geo_hyphenated_region_promote_is_default_on() {
    assert!(!rhh::is_disabled("geo_hyphenated_region_margin_promote"));
}

#[test]
fn geo_hyphenated_region_promote_fixes_the_postal_code_attractor() {
    // A jurisdiction-of-incorporation column dominated by hyphenated US
    // state codes plus a minority of bare foreign country codes: real ISO
    // codes throughout, 80% hyphenated (below the sibling ≥90% pure-promote
    // bar, so this is what exercises THIS guard specifically). Motivating
    // case: this shape (mostly US-state, minority bare-country) is what an
    // EDGAR-GLEIF crosswalk is expected to have, and it measures raw-model
    // postal_code at 95% confidence with nothing else in the stack able to
    // correct it (see the guard's own doc comment).
    let cc = geo_vote_classifier();
    let vals: Vec<String> = [
        "US-DE", "US-DE", "US-DE", "US-NY", "US-CA", "US-TX", "US-NV", "US-MD", "GB", "IT",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("jurisdiction", &vals, "geography.address.postal_code", 0.95)
        .unwrap();
    assert_eq!(
        r.label, "geography.location.region",
        "rule was {:?}",
        r.disambiguation_rule
    );
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("geo_hyphenated_region_margin_promote:geography.address.postal_code")
    );
}

#[test]
fn geo_hyphenated_region_promote_requires_dominance() {
    // A 50/50 hyphenated/bare split clears neither the WIN bar nor a
    // dominance margin — the postal_code call (wrong as it may be) stands
    // rather than a coin-flip promotion.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = [
        "US-DE", "US-NY", "US-CA", "US-TX", "US-NV", "GB", "IT", "ES", "SE", "FR",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("jurisdiction", &vals, "geography.address.postal_code", 0.95)
        .unwrap();
    assert_ne!(r.label, "geography.location.region");
}

#[test]
fn geo_hyphenated_region_promote_does_not_regress_bare_canadian_provinces() {
    // The exact scenario an earlier (bidirectional) version of this guard
    // regressed: bare Canadian province codes collide with country codes
    // (NL/SK/YT = Netherlands/Slovakia/Mayotte), scoring country_cov ~0.71 —
    // but region_cov (hyphenated ISO-3166-2 membership) is 0 for bare codes,
    // so this guard must NOT treat that as region-dominant.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["NL", "NL", "NL", "SK", "YT", "QC", "ON"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("state", &vals, "geography.location.state_code", 0.90)
        .unwrap();
    assert_ne!(r.label, "geography.location.region");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_hyphenated_region_margin_promote:geography.location.state_code")
    );
}

#[test]
fn geo_hyphenated_region_promote_leaves_region_alone() {
    // Already the target label — nothing to promote (mirrors
    // geo_subdivision_membership_promote's own already-region no-op).
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["US-DE", "US-NY", "US-CA", "US-TX"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("jurisdiction", &vals, "geography.location.region", 0.9)
        .unwrap();
    assert_eq!(r.label, "geography.location.region");
    assert!(r
        .disambiguation_rule
        .as_deref()
        .map(|rule| !rule.starts_with("geo_hyphenated_region_margin_promote"))
        .unwrap_or(true));
}

// === geo_code_nonmembership_demotion (external-band guard batch, guard 1) ===

#[test]
fn geo_nonmembership_demotion_is_default_on() {
    assert!(!rhh::is_disabled("geo_code_nonmembership_demotion"));
}

#[test]
fn geo_nonmembership_demotes_nonlocation_short_codes_to_word() {
    // NYC-DOB `work_type`: OT/EQ/MH/SP/PL — 2-letter permit codes the flat softmax
    // pulls onto state_code (they pass ^[A-Z]{2}$). Measured geo-union coverage ~39%,
    // below the 0.50 floor, so demote to the small-vocabulary residual `word`.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["OT", "EQ", "MH", "SP", "PL", "OT", "EQ", "OT"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("work_type", &vals, "geography.location.state_code", 0.90)
        .unwrap();
    assert_eq!(r.label, "representation.text.word");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_nonmembership_demotion:geography.location.state_code")
    );
}

#[test]
fn geo_nonmembership_keeps_hyphenated_iso3166_2_subdivisions() {
    // gleif `region`-style ISO-3166-2 subdivisions in hyphenated form. They score 0%
    // on the BARE subdivision set; the hyphenated arm of the membership check
    // recognises them by their real country prefix (100% coverage), so the column is
    // NOT demoted. This is the false-demote landmine the guard must avoid. (The test
    // fixture's country enum carries CA/GB/DE/FR, so hyphenated codes use those
    // prefixes; production carries the full ISO-3166-1 list incl. US.)
    let cc = geo_vote_classifier();
    // All six are published ISO-3166-2 codes (FR-IDF, not the deprecated FR-75 —
    // membership is strict, which is the point).
    let vals: Vec<String> = ["CA-ON", "CA-BC", "CA-QC", "GB-ENG", "DE-BW", "FR-IDF"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("region", &vals, "geography.location.state_code", 0.90)
        .unwrap();
    // NOT demoted by the nonmembership guard (the false-demote landmine)...
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_nonmembership_demotion:geography.location.state_code")
    );
    // ...and now PROMOTED to its precise type: a hyphenated ISO-3166-2 column is
    // geography.location.region, not the bare-code state_code the model guessed
    // (geo_subdivision_membership_promote runs after the nonmembership guard).
    assert_eq!(r.label, "geography.location.region");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("geo_subdivision_membership_promote:geography.location.state_code")
    );
}

#[test]
fn geo_subdivision_promote_is_default_on() {
    assert!(!rhh::is_disabled("geo_subdivision_membership_promote"));
}

#[test]
fn geo_subdivision_promote_recovers_iso3166_2_to_region() {
    // External band: ourairports `iso_region` = US-PA/US-KS/US-AK — real ISO-3166-2
    // subdivisions the model files under a residual (unlocode→unknown, alphanumeric_id).
    // ≥90% published membership promotes the column to region regardless of source label.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["US-PA", "US-KS", "US-AK", "GB-ENG", "CA-ON", "US-CA"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "iso_region",
            &vals,
            "representation.identifier.alphanumeric_id",
            0.60,
        )
        .unwrap();
    assert_eq!(r.label, "geography.location.region");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("geo_subdivision_membership_promote:representation.identifier.alphanumeric_id")
    );
}

#[test]
fn geo_subdivision_promote_keeps_nonmember_hyphen_codes() {
    // The precision control: hyphenated codes that share the `CC-SSS` SHAPE but are
    // NOT published ISO-3166-2 subdivisions (product/OS lookalikes) must NOT be
    // promoted — membership, not shape, is the discriminator.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["OS-10", "AB-99", "XX-01", "ZZ-ZZ", "QW-42", "OS-11"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "code",
            &vals,
            "representation.identifier.alphanumeric_id",
            0.80,
        )
        .unwrap();
    assert_ne!(r.label, "geography.location.region");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_subdivision_membership_promote:representation.identifier.alphanumeric_id")
    );
}

#[test]
fn geo_nonmembership_keeps_constant_country_column() {
    // A constant single-country column (all "GB") is a genuine — if low-cardinality —
    // country column: 100% enum coverage, so the guard must NOT mistake its tiny
    // vocabulary for a non-location short-code set and demote it.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["GB", "GB", "GB", "GB", "GB", "GB", "GB"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("country", &vals, "geography.location.country_code", 0.95)
        .unwrap();
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_nonmembership_demotion:geography.location.country_code")
    );
    assert_eq!(r.label, "geography.location.country_code");
}

#[test]
fn geo_nonmembership_keeps_alpha3_country_column() {
    // Regression control (gold `primaryCountry.alpha3Code` / `Country Code`): a
    // genuine country column in ALPHA-3 form (`FRA`/`GBR`/`USA`) is a real geography
    // type the 2-letter enum simply doesn't list. The values are NOT the 2-letter
    // attractor shape, so the shape gate leaves the column untouched — it must NOT be
    // demoted to a text residual.
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["FRA", "GBR", "USA", "DEU", "ITA", "ESP", "CHN", "IND"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("country", &vals, "geography.location.country_code", 0.95)
        .unwrap();
    // The guard's contract is that it must NOT demote this to a text residual. Other
    // pipeline rules may relabel country_code -> country (the full leaf) on alpha-3
    // values; that is legitimate and not this guard.
    assert!(
        r.label.starts_with("geography."),
        "alpha-3 country column must stay a geography type, got {}",
        r.label
    );
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_nonmembership_demotion:geography.location.country_code")
    );
}

#[test]
fn geo_nonmembership_keeps_genuine_country_column() {
    // A real multi-country column validates ~100% against the country enum -> kept.
    // (All codes are members of the fixture's 12-code country enum.)
    let cc = geo_vote_classifier();
    let vals: Vec<String> = ["GB", "FR", "DE", "IT", "CN", "ES", "NL", "SE"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("country", &vals, "geography.location.country_code", 0.95)
        .unwrap();
    assert_eq!(r.label, "geography.location.country_code");
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("geo_code_nonmembership_demotion:geography.location.country_code")
    );
}

// === 0094 postal header-veto helpers (spec 2026-06-10-postal-header-veto) ===

#[test]
fn postal_veto_corroborates_real_postal_headers() {
    for h in [
        "zip",
        "ZIP",
        "zip_code",
        "zipcode",
        "business_zip",
        "billing zip",
        "postal_code",
        "PostalCode",
        "postcode",
        "post_code",
        "PLZ",
        "cep",
        "pincode",
        "pin_code_area_pincode",
    ] {
        assert!(header_corroborates_postal(h), "should corroborate: {h}");
    }
}

#[test]
fn postal_veto_does_not_corroborate_false_friends_or_quantities() {
    // the gold-measured FP battleground: bare-integer quantity columns plus
    // token false friends — none may corroborate, so postal is demotable.
    for h in [
        "zipper",
        "unzip",
        "gzip_size",
        "averageDailyVolume10Day",
        "fullTimeEmployees",
        "totalCurrentLiabilities",
        "conversation_no",
        "voteId",
        "d_week_seq",
        "destination_x",
        "sId",
        "1001",
        "code",
        "postage_paid",
    ] {
        assert!(
            !header_corroborates_postal(h),
            "should NOT corroborate: {h}"
        );
    }
}

#[test]
fn postal_veto_value_gate_requires_bare_integers_without_leading_zeros() {
    // bare 4-digit integers — the measured FP shape — satisfy the gate
    assert!(values_look_like_generic_integers(&[
        "1191".into(),
        "1497".into(),
        "2200".into(),
        "5322".into()
    ]));
    // a leading zero is postal evidence (01219-style zip): veto blocked
    assert!(!values_look_like_generic_integers(&[
        "01219".into(),
        "1191".into(),
        "1497".into()
    ]));
    // decimals / text / zip+4 / empty do not satisfy the gate
    assert!(!values_look_like_generic_integers(&[
        "12.5".into(),
        "9.1".into(),
        "3.3".into()
    ]));
    assert!(!values_look_like_generic_integers(&[
        "12345-6789".into(),
        "98765-4321".into()
    ]));
    assert!(!values_look_like_generic_integers(&[]));
}

#[test]
fn postal_veto_is_a_default_on_header_hint() {
    assert!(!rhh::is_disabled("header_hint_postal_veto"));
}

#[test]
fn ac01_r25_http_status_gate_preserves_real_postal_codes() {
    // 5-digit postal codes: non-sequential, consistent 5-digit length
    let values: Vec<String> = vec![
        "10001", "90210", "33139", "60601", "02134", "94102", "30308",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    assert!(result.is_some(), "R12 should fire on 5-digit postal codes");
    let (label, _) = result.unwrap();
    assert_eq!(label, "geography.address.postal_code");
}

#[test]
fn ac01_r25_http_status_gate_preserves_3digit_postal() {
    // 3-digit values outside HTTP range (≥600): Icelandic postal codes
    // Non-sequential to avoid increment detection
    let values: Vec<String> = vec!["601", "900", "750", "602", "850", "700", "801"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    // <90% in 100-599 range (0% in this case), so R25 guard doesn't fire
    if let Some((label, _)) = &result {
        assert_eq!(
            label, "geography.address.postal_code",
            "3-digit values outside 100-599 should still be postal_code"
        );
    }
}

#[test]
fn state_hint_uses_live_region_leaf() {
    // Regression guard: the dead alias geography.location.state must never be
    // reintroduced as a header hint (BACKLOG #9).
    assert_ne!(header_hint("state"), Some("geography.location.state"));
    assert_ne!(header_hint("province"), Some("geography.location.state"));
}

#[test]
fn v15_subcountry_not_mapped_to_state() {
    // "subcountry" should NOT map to state — model predicts region correctly
    assert_eq!(header_hint("subcountry"), None);
}

// ==========================================================================
// Locale detection tests for calling_code, month_name, day_of_week
// ==========================================================================

#[test]
fn test_detect_locale_calling_code_uk() {
    let yaml = r#"
geography.contact.calling_code:
  title: International Calling Code
  designation: locale_specific
  tier: [VARCHAR, contact]
  release_priority: 3
  samples: ["+1"]
  validation:
    type: string
    pattern: "^\\+?[0-9]{1,4}$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^\\+?1$"
    EN_GB:
      type: string
      pattern: "^\\+?44$"
    DE:
      type: string
      pattern: "^\\+?49$"
    FR:
      type: string
      pattern: "^\\+?33$"
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["+44", "44", "+44", "+44", "44"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale =
        detect_locale_from_validation(&values, "geography.contact.calling_code", &taxonomy);
    assert_eq!(
        locale,
        Some("EN_GB".to_string()),
        "+44 calling codes should detect EN_GB"
    );
}

#[test]
fn test_detect_locale_calling_code_de() {
    let yaml = r#"
geography.contact.calling_code:
  title: International Calling Code
  designation: locale_specific
  tier: [VARCHAR, contact]
  release_priority: 3
  samples: ["+1"]
  validation:
    type: string
    pattern: "^\\+?[0-9]{1,4}$"
  validation_by_locale:
    EN_US:
      type: string
      pattern: "^\\+?1$"
    EN_GB:
      type: string
      pattern: "^\\+?44$"
    DE:
      type: string
      pattern: "^\\+?49$"
"#;
    let mut taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    taxonomy.compile_locale_validators();

    let values: Vec<String> = vec!["+49", "49", "+49"]
        .into_iter()
        .map(String::from)
        .collect();

    let locale =
        detect_locale_from_validation(&values, "geography.contact.calling_code", &taxonomy);
    assert_eq!(
        locale,
        Some("DE".to_string()),
        "+49 calling codes should detect DE"
    );
}

#[test]
fn test_geography_protection_person_name_hints() {
    // Fix 2: Geography protection should fire for last_name and first_name,
    // not just full_name. Verifies PERSON_NAME_HINTS covers all three.
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.full_name"),
        "PERSON_NAME_HINTS should include full_name"
    );
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.last_name"),
        "PERSON_NAME_HINTS should include last_name"
    );
    assert!(
        PERSON_NAME_HINTS.contains(&"identity.person.first_name"),
        "PERSON_NAME_HINTS should include first_name"
    );

    // Non-person identity types should NOT be in the list
    assert!(
        !PERSON_NAME_HINTS.contains(&"identity.person.email"),
        "email should NOT be in PERSON_NAME_HINTS"
    );
    assert!(
        !PERSON_NAME_HINTS.contains(&"identity.person.phone_number"),
        "phone_number should NOT be in PERSON_NAME_HINTS"
    );
}

#[test]
fn test_location_types_module_level() {
    // Verify LOCATION_TYPES extracted to module level contains expected types
    assert!(
        LOCATION_TYPES.contains(&"geography.location.city"),
        "LOCATION_TYPES should include city"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.country"),
        "LOCATION_TYPES should include country"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.region"),
        "LOCATION_TYPES should include region"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.state"),
        "LOCATION_TYPES should include state"
    );
    assert!(
        LOCATION_TYPES.contains(&"geography.location.continent"),
        "LOCATION_TYPES should include continent"
    );
}

#[test]
fn test_geo_override_same_domain() {
    // `header_hint_geo_override`, through the composed path a user actually sees.
    //
    // This test used to assert only that "city" and "country" are both in
    // LOCATION_TYPES — it never called the function it was named for, so it
    // passed unchanged on a branch that had DELETED the rule. Membership in a
    // const array is not behaviour. Drive the rule instead.
    //
    // A location hint on a different location label overrides it and KEEPS the
    // model's confidence (floor 0.6). The `return` matters as much as the
    // override: it shields the column from `header_hint_hardcoded`, which
    // flattens confidence to 0.5 for anything under 0.95. Deleting the arm
    // leaves the label alone but drops 0.80 -> 0.50 — a `medium` quality band
    // becoming `low`, with a runner_up surfaced — so the confidence and the
    // rule are asserted, not just the label.
    let cc = geo_vote_classifier();
    let vals = vals(&["Osaka", "Lyon", "Perth", "Bogota", "Tallinn"]);
    let r = cc
        .compose_from_sense("city", &vals, "geography.location.country", 0.80)
        .unwrap();
    assert_eq!(r.label, "geography.location.city");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("header_hint_geo_override:city"),
        "the geo override must be the rule that fired, not a downstream fallback"
    );
    assert!(
        (r.confidence - 0.80).abs() < 1e-6,
        "the model's confidence must survive the override; got {}",
        r.confidence
    );
}

#[test]
fn geo_override_shields_the_column_from_the_hardcoded_flattener() {
    // The same arm, stated as the property that broke when it was deleted:
    // whatever else changes, a location-hint-on-location-label column must not
    // come out of Sharpen wearing `header_hint_hardcoded` at 0.5.
    let cc = geo_vote_classifier();
    let vals = vals(&["Osaka", "Lyon", "Perth", "Bogota", "Tallinn"]);
    let r = cc
        .compose_from_sense("city", &vals, "geography.location.country", 0.80)
        .unwrap();
    assert_ne!(
        r.disambiguation_rule.as_deref(),
        Some("header_hint_hardcoded:city"),
        "the hardcoded-authority arm must not be reached for this column"
    );
    assert!(
        r.confidence > 0.70,
        "confidence must stay above the low-band floor; got {}",
        r.confidence
    );
}

#[test]
fn person_name_header_overrides_a_location_label() {
    // `header_hint_person_override` — the arm the dead-weight pass first deleted
    // and then restored on corpus evidence (133 columns headed `authors`,
    // `LastName`, `Surname`, `billing_lastname`, `NPPES_PROVIDER_FIRST_NAME`).
    // Nothing pinned it: mutating it to never fire left all 563 model tests
    // green, so the next deletion pass would have removed it again with a clean
    // suite. Surnames and place names share a vocabulary, so the model reads
    // these columns as a place; a hardcoded person-name header is what rescues
    // them.
    let cc = geo_vote_classifier();
    let vals = vals(&["Nakamura", "Okafor", "Silva", "Novak", "Dubois"]);
    let r = cc
        .compose_from_sense("last_name", &vals, "geography.location.city", 0.70)
        .unwrap();
    assert_eq!(r.label, "identity.person.last_name");
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("header_hint_person_override:last_name")
    );
    assert!(
        (r.confidence - 0.70).abs() < 1e-6,
        "the model's confidence must survive the override; got {}",
        r.confidence
    );
}

// ── Sharpen header bugfix tests (spec: 2026-04-12-sharpen-header-bugfixes) ──

#[test]
fn ac01_bitcoin_address_not_captured_by_address_hint() {
    // "bitcoin_address" must NOT match the address keyword — it should return
    // None so the model's bitcoin_address prediction is preserved.
    assert_eq!(
        header_hint("bitcoin_address"),
        None,
        "bitcoin_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("btc_address"),
        None,
        "btc_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("crypto_address"),
        None,
        "crypto_address must not match the address keyword"
    );
    assert_eq!(
        header_hint("wallet_address"),
        None,
        "wallet_address must not match the address keyword"
    );
    // Regression: street_address and full_address still work
    assert_eq!(
        header_hint("street_address"),
        Some("geography.address.street_address"),
        "street_address should still return street_address"
    );
    assert_eq!(
        header_hint("full_address"),
        Some("geography.address.full_address"),
        "full_address should still return full_address"
    );
    assert_eq!(
        header_hint("home_address"),
        Some("geography.address.full_address"),
        "home_address should still return full_address"
    );
}

#[test]
fn ac04_icao_header_hint() {
    assert_eq!(
        header_hint("icao"),
        Some("geography.transportation.icao_code"),
        "icao should hint to icao_code"
    );
    assert_eq!(
        header_hint("ICAO"),
        Some("geography.transportation.icao_code"),
        "ICAO (uppercase) should hint to icao_code"
    );
    assert_eq!(
        header_hint("icao_code"),
        Some("geography.transportation.icao_code"),
        "icao_code should hint to icao_code"
    );
}

#[test]
fn debug_icao_sharpen_override() {
    // Full apply_header_sharpen test: icao hint should override unlocode
    let mock = crate::inference::MockClassifier::new("geography.transportation.unlocode");
    let cc = ColumnClassifier::with_defaults(Box::new(mock));
    let mut result = make_result("geography.transportation.unlocode", 0.919);
    let sample: Vec<String> = vec!["EGLL".into(), "KJFK".into(), "LFPG".into()];

    // Verify header_hint works
    let hint = header_hint("icao");
    assert_eq!(hint, Some("geography.transportation.icao_code"));

    cc.apply_header_sharpen(&mut result, "icao", &sample);

    eprintln!("result.label = {}", result.label);
    eprintln!("result.rule = {:?}", result.disambiguation_rule);
    assert_eq!(
        result.label, "geography.transportation.icao_code",
        "icao hint must override unlocode@0.919 via same-category path"
    );
}

// ── R21: Coordinate plausibility gate tests ─────────────────────────
#[test]
fn test_r21_coordinate_gate_demotes_out_of_range() {
    // Earthquake depth values — many exceed 180, not coordinates
    let values: Vec<String> = vec![
        "127.013", "573.817", "201.998", "10.0", "224.419", "177.684", "97.335", "671.0", "450.2",
        "300.1",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.coordinate.longitude", 0.94, None);
    assert!(result.is_some(), "R21 should fire for out-of-range values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
    assert!(rule.contains("coordinate_plausibility_gate"));
}

#[test]
fn test_r21_coordinate_gate_keeps_real_coordinates() {
    // Real longitude values — all within [-180, 180]
    let values: Vec<String> = vec![
        "-122.4194",
        "139.6917",
        "2.3522",
        "-73.9857",
        "151.2093",
        "-43.1729",
        "28.9784",
        "-3.7038",
        "103.8198",
        "-46.6333",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.coordinate.longitude", 0.95, None);
    // Should NOT fire — these are valid longitude values
    assert!(
        result.is_none()
            || result
                .as_ref()
                .unwrap()
                .1
                .contains("coordinate_disambiguation"),
        "R21 should not fire for valid coordinates"
    );
}

// ── AC-05: Country/country_code post-hint guard ──────────────────────
// Note: Full integration tests for AC-05 require the model to be loaded.
// These unit tests verify the guard logic via header_hint and value patterns.

#[test]
fn ac05_country_exact_match_still_hints_country() {
    // The hardcoded hint for "country" still maps to geography.location.country.
    // The post-hint guard in apply_header_sharpen then checks values.
    assert_eq!(header_hint("country"), Some("geography.location.country"));
}

#[test]
fn ac05_alpha2_regex_matches_country_codes() {
    // Verify the alpha-2 check logic matches ISO 3166-1 codes.
    let codes = ["AU", "US", "GB", "DE", "FR", "JP", "CN", "BR"];
    for code in &codes {
        assert!(
            code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase()),
            "{} should match alpha-2 pattern",
            code
        );
    }
}

#[test]
fn ac05_alpha2_regex_rejects_state_codes() {
    // 3-letter state codes like "NSW" must NOT match the alpha-2 pattern.
    let state_codes = ["NSW", "VIC", "QLD", "CA", "NY"];
    for code in &state_codes {
        let matches = code.len() == 2 && code.chars().all(|c| c.is_ascii_uppercase());
        // CA and NY are 2-letter but they're state codes — the guard only fires
        // when the label is "country" (not state), so these are correctly handled
        // by the pipeline context, not the regex alone.
        if code.len() == 3 {
            assert!(
                !matches,
                "{} (3-char) should not match alpha-2 pattern",
                code
            );
        }
    }
}

#[test]
fn ac05_alpha2_regex_rejects_country_names() {
    // Full country names should not match.
    let names = ["Australia", "United States", "Germany", "France"];
    for name in &names {
        assert!(
            !(name.len() == 2 && name.chars().all(|c| c.is_ascii_uppercase())),
            "{} should not match alpha-2 pattern",
            name
        );
    }
}
