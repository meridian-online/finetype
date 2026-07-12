use super::super::*;
use super::jwt_guard_taxonomy;

#[test]
fn checksum_substance_guard_is_default_on() {
    assert!(!rhh::is_disabled("checksum_substance_guard"));
}

#[test]
fn isbn_checksum_distinguishes_genuine_from_lookalikes() {
    // The check-digit math now lives in the canonical crate::checksum module
    // (wired into the validator via `checksum: isbn`); the guard delegates to
    // it. Genuine ISBNs pass; same-length financial figures the model
    // mislabels as ISBN fail.
    use finetype_core::checksum::isbn;
    for v in [
        "0306406152",
        "043942089X",
        "9780306406157",
        "978-3-16-148410-0",
    ] {
        assert!(isbn(v), "should be valid ISBN: {v}");
    }
    for v in ["5150000128", "6965100000", "7586000000", "1041000000"] {
        assert!(!isbn(v), "should NOT be valid ISBN: {v}");
    }
    assert!(!isbn("-1617000000"));
}

#[test]
fn v15_upc_maps_to_upc_not_ean() {
    // "upc" should map to identity.commerce.upc, not ean
    assert_eq!(header_hint("upc"), Some("identity.commerce.upc"));
    // "ean" still maps to ean
    assert_eq!(header_hint("ean"), Some("identity.commerce.ean"));
    // "barcode" still maps to ean
    assert_eq!(header_hint("barcode"), Some("identity.commerce.ean"));
}

#[test]
fn test_hs_code_float_parseability() {
    // HS codes with 3 segments don't parse as float
    let codes = [
        "6204.62.40", // 3 segments → NOT float-parseable
        "8471.30.10",
        "8471.30", // 2 segments → float-parseable (8471.30)
        "6204.62",
    ];
    let per_value: Vec<[f32; FEATURE_DIM]> = codes.iter().map(|s| extract_features(s)).collect();
    let cf = aggregate_features(&per_value);

    // is_float should be < 1.0 (2 of 4 don't parse as float)
    assert!(
        cf.mean[feature_idx::IS_FLOAT] < 1.0,
        "HS codes with 3-segment entries should have is_float < 1.0, got {}",
        cf.mean[feature_idx::IS_FLOAT]
    );
    // digit_ratio should be high
    assert!(
        cf.mean[feature_idx::DIGIT_RATIO] > 0.7,
        "HS codes should have high digit ratio, got {}",
        cf.mean[feature_idx::DIGIT_RATIO]
    );
}

// R20: HS code validation gate — demotes hs_code when values are plain decimals
#[test]
fn test_r20_hs_code_gate_demotes_plain_decimals() {
    // Model predicts hs_code but values are plain decimals (pe_ratio, sepal_length, etc.)
    let values: Vec<String> = vec![
        "3.14", "0.887", "-12.5", "100.0", "0.003", "45.67", "1.23", "99.9",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_some(), "R20 should fire on plain decimals");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
    assert!(rule.starts_with("hs_code_validation_gate"));
}

#[test]
fn test_r20_hs_code_gate_keeps_real_hs_codes() {
    // Real HS codes should NOT be demoted
    let values: Vec<String> = vec![
        "8471.30",
        "8471.30.00",
        "6204.62",
        "8517.12",
        "0901.21",
        "2204.10",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_none(), "R20 should NOT fire on real HS codes");
}

#[test]
fn test_is_hs_code_format() {
    // Valid HS codes
    assert!(is_hs_code_format("8471.30"));
    assert!(is_hs_code_format("8471.30.00"));
    assert!(is_hs_code_format("0901.21.00.10"));
    assert!(is_hs_code_format("847130")); // undotted 6-digit
    assert!(is_hs_code_format("84713000")); // undotted 8-digit

    // Invalid — plain decimals
    assert!(!is_hs_code_format("3.14"));
    assert!(!is_hs_code_format("0.887"));
    assert!(!is_hs_code_format("-12.5"));
    assert!(!is_hs_code_format("100.0"));
    assert!(!is_hs_code_format("45.67"));

    // Invalid — too short
    assert!(!is_hs_code_format("123"));
    assert!(!is_hs_code_format("12.34"));

    // Invalid — negative
    assert!(!is_hs_code_format("-8471.30"));
}

// ── R22: UPC digit-count gate tests ───────────────────────────────
#[test]
fn test_r22_upc_gate_corrects_to_ean() {
    // EAN-13 values (13 digits) misclassified as UPC (12 digits)
    let values: Vec<String> = vec![
        "1794213764625",
        "4293423898067",
        "6324920385397",
        "3683935437077",
        "5078019484874",
        "8706648142321",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.999, None);
    assert!(result.is_some(), "R22 should fire for 13-digit values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.commerce.ean");
    assert!(rule.contains("upc_digit_count_gate"));
}

#[test]
fn test_r22_upc_gate_demotes_wrong_length() {
    // 10-digit values (e.g., NPI) misclassified as UPC
    let values: Vec<String> = vec![
        "1966662179",
        "6579926978",
        "2527909147",
        "9953906342",
        "2157414996",
        "6989529491",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.94, None);
    assert!(result.is_some(), "R22 should fire for non-12-digit values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.identifier.numeric_code");
    assert!(rule.contains("upc_digit_count_gate"));
}

#[test]
fn test_r22_upc_gate_keeps_real_upc() {
    // Real UPC values (12 digits)
    let values: Vec<String> = vec![
        "012345678905",
        "036000291452",
        "070330507227",
        "042100005264",
        "040000000068",
        "041570056103",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.upc", 0.99, None);
    // Should NOT fire — these are valid UPC values
    assert!(
        result.is_none() || !result.as_ref().unwrap().1.contains("upc_digit_count_gate"),
        "R22 should not fire for valid 12-digit UPC"
    );
}

// ── R23: ISIN format gate tests ───────────────────────────────────
#[test]
fn test_r23_isin_gate_corrects_isrc() {
    // ISIN values misclassified as ISRC
    let values: Vec<String> = vec![
        "US0378331005",
        "GB0002634946",
        "DE0007164600",
        "JP3435000009",
        "FR0000120271",
        "CA0585861085",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.commerce.isrc", 0.97, None);
    assert!(result.is_some(), "R23 should fire for ISIN-format values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "finance.securities.isin");
    assert!(rule.contains("isin_format_gate"));
}

// ── R24: ISSN/EIN dash-position gate tests ────────────────────────
#[test]
fn test_r24_issn_gate_corrects_ein() {
    // ISSN values misclassified as EIN
    let values: Vec<String> = vec![
        "1781-2253",
        "8371-5342",
        "6910-7471",
        "2908-3721",
        "8987-7548",
        "4149-8688",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "identity.government.ein", 0.91, None);
    assert!(result.is_some(), "R24 should fire for ISSN-format values");
    let (label, rule) = result.unwrap();
    assert_eq!(label, "identity.commerce.issn");
    assert!(rule.contains("issn_format_gate"));
}

#[test]
fn test_is_isin_format() {
    assert!(is_isin_format("US0378331005"));
    assert!(is_isin_format("GB0002634946"));
    assert!(is_isin_format("AU000000BHP4"));
    assert!(is_isin_format("NL0011540547"));

    // Invalid — ISRC values (letters in positions 2-4 = registrant code)
    assert!(!is_isin_format("SE3YX3859059")); // ISRC: positions 2-4 = "3YX"
    assert!(!is_isin_format("NLEK47515013")); // ISRC: positions 2-4 = "EK4"
    assert!(!is_isin_format("CAHRM7311593")); // ISRC: positions 2-4 = "HRM"
                                              // Invalid — other formats
    assert!(!is_isin_format("US-Z03-98-12345")); // ISRC with dashes
    assert!(!is_isin_format("1234567890AB")); // starts with digits
    assert!(!is_isin_format("USABC")); // too short
    assert!(!is_isin_format("us0378331005")); // lowercase
}

#[test]
fn test_is_issn_format() {
    assert!(is_issn_format("1781-2253"));
    assert!(is_issn_format("0317-839X")); // X check digit
    assert!(is_issn_format("0000-0000"));

    // Invalid — EIN format (dash at position 2)
    assert!(!is_issn_format("12-3456789"));
    // Invalid — too short/long
    assert!(!is_issn_format("1234-567"));
    assert!(!is_issn_format("12345-6789"));
    // Invalid — no dash
    assert!(!is_issn_format("12345678"));
}

// ── AC-07(b): F3 removal ─────────────────────────────────────────────

#[test]
fn ac07b_r20_still_validates_hs_codes() {
    // R20 (HS code validation gate) must still work as the sole backstop
    // after F3 removal. Model-predicted hs_code with valid values should pass.
    let values: Vec<String> = vec![
        "8471.30",
        "8471.30.00",
        "6204.62",
        "8517.12",
        "0901.21",
        "2204.10",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(
        result.is_none(),
        "R20 should keep valid hs_code predictions after F3 removal"
    );
}

#[test]
fn ac07b_r20_still_demotes_false_hs_codes() {
    // R20 must still demote hs_code when values are plain decimals.
    let values: Vec<String> = vec![
        "3.14", "0.887", "-12.5", "100.0", "0.003", "45.67", "1.23", "99.9",
    ]
    .into_iter()
    .map(String::from)
    .collect();

    let result = value_sharpen(&values, "geography.transportation.hs_code", 0.95, None);
    assert!(result.is_some(), "R20 should still demote false hs_code");
    let (label, _) = result.unwrap();
    assert_eq!(label, "representation.numeric.decimal_number");
}

// ── isbn_header_recovery (BACKLOG #6b) ──

#[test]
fn isbn_header_recovery_promotes_checked_isbn() {
    let yaml = r#"
identity.commerce.isbn:
  title: ISBN
  designation: universal
  tier: [VARCHAR, commerce]
  release_priority: 3
  samples: ["0306406152"]
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    // ISBN header + check-digit-valid ISBN-10s funnelled into numeric_code → recovered.
    let isbns: Vec<String> = vec!["0306406152", "0140449132", "043942089X", "0201633612"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "Primary ISBN10",
            &isbns,
            "representation.identifier.numeric_code",
            1.0,
        )
        .unwrap();
    assert_eq!(r.label, "identity.commerce.isbn", "isbn recovery must fire");
}

#[test]
fn isbn_header_recovery_declines_failed_checksum_and_no_header() {
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy({
        let mut t = Taxonomy::from_yaml(
            "identity.commerce.isbn:\n  title: ISBN\n  designation: universal\n  tier: [VARCHAR, commerce]\n  samples: [\"0306406152\"]\n",
        )
        .unwrap();
        t.compile_validators();
        t
    });

    // ISBN header but financial integers that FAIL the check digit → untouched.
    let bad: Vec<String> = vec!["5150000128", "5150000129", "5150000130"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "Primary ISBN10",
            &bad,
            "representation.numeric.integer_number",
            1.0,
        )
        .unwrap();
    assert_ne!(
        r.label, "identity.commerce.isbn",
        "must decline failed checksum"
    );

    // Valid ISBNs but NO isbn header → untouched (header gate is load-bearing).
    let isbns: Vec<String> = vec!["0306406152", "0140449132", "043942089X"]
        .into_iter()
        .map(String::from)
        .collect();
    let r2 = cc
        .compose_from_sense(
            "book_code",
            &isbns,
            "representation.identifier.numeric_code",
            1.0,
        )
        .unwrap();
    assert_ne!(
        r2.label, "identity.commerce.isbn",
        "must decline without header"
    );
}

// ── ceded_leaf_recovery (model label-space reshape ac-3) ──

#[test]
fn ceded_leaf_recovery_reasserts_uuid_over_relocated_label() {
    // The reshaped model can't emit uuid, so it relocates a uuid column onto a
    // digit/hex neighbour (alphanumeric_id here, ac-2 drift). The value-based recovery
    // re-asserts uuid from the conclusive validator, regardless of the wrong label.
    let yaml = r#"
representation.identifier.uuid:
  title: UUID
  designation: universal
  tier: [VARCHAR, identifier]
  validation:
    type: string
    pattern: "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
  samples: ["550e8400-e29b-41d4-a716-446655440000"]
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let uuids: Vec<String> = vec![
        "550e8400-e29b-41d4-a716-446655440000",
        "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
        "6ba7b811-9dad-11d1-80b4-00c04fd430c8",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "token",
            &uuids,
            "representation.identifier.alphanumeric_id",
            1.0,
        )
        .unwrap();
    assert_eq!(
        r.label, "representation.identifier.uuid",
        "ceded_leaf_recovery must re-assert uuid from values"
    );
}

#[test]
fn ceded_leaf_recovery_declines_non_validating_values() {
    // Same eligible leaf (uuid), but the values are NOT uuids — recovery must NOT fire
    // (no over-emission onto non-matching columns).
    let yaml = r#"
representation.identifier.uuid:
  title: UUID
  designation: universal
  tier: [VARCHAR, identifier]
  validation:
    type: string
    pattern: "^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
  samples: ["550e8400-e29b-41d4-a716-446655440000"]
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let words: Vec<String> = vec!["apple", "banana", "cherry", "date"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("fruit", &words, "representation.text.word", 1.0)
        .unwrap();
    assert_ne!(
        r.label, "representation.identifier.uuid",
        "ceded_leaf_recovery must not fire on non-validating values"
    );
}

// ── isin_checksum_recovery ──

const ISIN_ISRC_YAML: &str = r#"
finance.securities.isin:
  title: ISIN
  designation: universal
  tier: [VARCHAR, payment]
  checksum: isin
  validation:
    type: string
    pattern: "^[A-Z]{2}[A-Z0-9]{9}[0-9]$"
  samples: ["US0378331005"]
identity.commerce.isrc:
  title: ISRC
  designation: universal
  tier: [VARCHAR, code]
  validation:
    type: string
    pattern: "^[A-Z]{2}[A-Z0-9]{3}\\d{7}$"
  samples: ["USRC17607839"]
"#;

#[test]
fn isin_checksum_recovery_reasserts_isin_over_isrc() {
    // A digit-tailed ISIN matches ISRC's 12-char pattern, so the regex-only
    // ceded_leaf_recovery lands a real ISIN column on isrc. The ISIN check digit
    // (invisible to the regex validator) is the discriminator that corrects it.
    let mut tax = Taxonomy::from_yaml(ISIN_ISRC_YAML).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let isins: Vec<String> = vec![
        "US0378331005",
        "GB0002634946",
        "JP3633400001",
        "DE0007164600",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "token",
            &isins,
            "representation.identifier.alphanumeric_id",
            1.0,
        )
        .unwrap();
    assert_eq!(
        r.label, "finance.securities.isin",
        "isin_checksum_recovery must correct a valid-ISIN column off isrc"
    );
}

#[test]
fn isin_checksum_recovery_declines_bad_checksum() {
    // ISIN-shaped values with broken check digits are not ISINs — must not
    // promote. Real ISRCs fail the ISIN checksum identically, so their isrc
    // recovery is preserved.
    let mut tax = Taxonomy::from_yaml(ISIN_ISRC_YAML).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let bad: Vec<String> = vec![
        "US0378331000",
        "GB0002634940",
        "JP3633400000",
        "DE0007164601",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "token",
            &bad,
            "representation.identifier.alphanumeric_id",
            1.0,
        )
        .unwrap();
    assert_ne!(
        r.label, "finance.securities.isin",
        "isin_checksum_recovery must not promote checksum-invalid values"
    );
}

// ── model-blind certainty-recovery guards (cusip/sedol/dea/imei/unlocode/cpt/
//    hs_code/color_rgb) — each recovers a leaf the 244-dim model cannot predict ──

// Minimal taxonomy carrying the eight recovered leaves (patterns match the live
// definitions so `label_validates_sample` behaves as it does in production).
const CERTAINTY_LEAVES_YAML: &str = r#"
finance.securities.cusip:
  title: CUSIP
  designation: universal
  tier: [VARCHAR, payment]
  checksum: cusip
  validation:
    type: string
    pattern: "^[A-Z0-9]{8}[0-9]$"
  samples: ["037833100"]
finance.securities.sedol:
  title: SEDOL
  designation: universal
  tier: [VARCHAR, payment]
  checksum: sedol
  validation:
    type: string
    pattern: "^[B-DF-HJ-NP-TV-Z0-9]{6}[0-9]$"
  samples: ["0263494"]
identity.medical.dea_number:
  title: DEA
  designation: universal
  tier: [VARCHAR, medical]
  checksum: dea
  validation:
    type: string
    pattern: "^[ABFMPRabfmpr][A-Za-z]\\d{7}$"
  samples: ["AB1234563"]
identity.medical.cpt:
  title: CPT
  designation: universal
  tier: [VARCHAR, medical]
  validation:
    type: string
    pattern: "^\\d{5}$|^\\d{4}[FTU]$"
  samples: ["99213"]
technology.code.imei:
  title: IMEI
  designation: universal
  tier: [VARCHAR, code]
  checksum: luhn
  validation:
    type: string
    pattern: "^[0-9]{15}$"
  samples: ["490154203237518"]
geography.transportation.hs_code:
  title: HS code
  designation: universal
  tier: [VARCHAR, transportation]
  validation:
    type: string
    pattern: "^\\d{4}\\.?\\d{2}(\\.?\\d{2}){0,2}$"
  samples: ["8471.30"]
geography.transportation.unlocode:
  title: UN/LOCODE
  designation: universal
  tier: [VARCHAR, transportation]
  membership: unlocode
  validation:
    type: string
    pattern: "^[A-Z]{2}[A-Z2-9]{3}$"
  samples: ["USLAX"]
representation.format.color_rgb:
  title: Color (RGB)
  designation: universal
  tier: [VARCHAR, text]
  validation:
    type: string
    pattern: "^(?:rgb)?\\(?([0-9]{1,3}),\\s*([0-9]{1,3}),\\s*([0-9]{1,3})\\)?$"
  samples: ["rgb(255, 0, 0)"]
"#;

fn certainty_cc() -> ColumnClassifier {
    let mut tax = Taxonomy::from_yaml(CERTAINTY_LEAVES_YAML).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    cc
}

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn recovered_label(header: &str, values: &[&str], sense: &str) -> String {
    certainty_cc()
        .compose_from_sense(header, &v(values), sense, 1.0)
        .unwrap()
        .label
}

#[test]
fn cusip_checksum_recovery_promotes_valid_cusips() {
    // Real CUSIPs land on the word/alnum attractor; the mod-10 check digit recovers.
    let l = recovered_label(
        "id",
        &["037833100", "17275R102", "594918104"],
        "representation.text.word",
    );
    assert_eq!(l, "finance.securities.cusip");
}

#[test]
fn cusip_checksum_recovery_declines_bad_checksum() {
    // 9-char alnum IDs that fit the shape but fail the check digit are not CUSIPs.
    let l = recovered_label(
        "id",
        &["037833101", "17275R103", "594918105"],
        "representation.text.word",
    );
    assert_ne!(l, "finance.securities.cusip");
}

#[test]
fn cusip_checksum_recovery_declines_constant_column() {
    // Regression: gold col `phkey` is a CONSTANT column of one repeated 9-digit key
    // (484158167) that coincidentally passes the weak mod-10 CUSIP check, so a 100%
    // pass rate fired the guard. The distinct-value cardinality gate rejects it — a
    // rare checksum type needs distributional evidence, not one repeated coincidence.
    let l = recovered_label(
        "phkey",
        &["484158167", "484158167", "484158167", "484158167"],
        "datetime.epoch.unix_seconds",
    );
    assert_ne!(l, "finance.securities.cusip");
}

#[test]
fn sedol_checksum_recovery_promotes_valid_sedols() {
    let l = recovered_label(
        "code",
        &["0263494", "B0WNLY7", "3134865"],
        "representation.identifier.numeric_code",
    );
    assert_eq!(l, "finance.securities.sedol");
}

#[test]
fn sedol_checksum_recovery_declines_plain_numeric_codes() {
    // Bare 7-digit numeric codes clear the shape but not the weighted check digit.
    let l = recovered_label(
        "code",
        &["1234567", "7654321", "1112223"],
        "representation.identifier.numeric_code",
    );
    assert_ne!(l, "finance.securities.sedol");
}

#[test]
fn dea_checksum_recovery_promotes_valid_dea_numbers() {
    let l = recovered_label(
        "id",
        &["AB1234563", "BS4567890", "FH1357905"],
        "representation.identifier.alphanumeric_id",
    );
    assert_eq!(l, "identity.medical.dea_number");
}

#[test]
fn dea_checksum_recovery_declines_bad_checksum() {
    // Same DEA shape, wrong check digit (last digit bumped) — not real DEA numbers.
    let l = recovered_label(
        "id",
        &["AB1234564", "BS4567891", "FH1357906"],
        "representation.identifier.alphanumeric_id",
    );
    assert_ne!(l, "identity.medical.dea_number");
}

#[test]
fn imei_checksum_recovery_promotes_with_header() {
    let l = recovered_label(
        "imei",
        &["490154203237518", "352043068649148", "011245002151205"],
        "representation.numeric.integer_number",
    );
    assert_eq!(l, "technology.code.imei");
}

#[test]
fn imei_checksum_recovery_requires_header() {
    // Same valid IMEIs, but a non-imei header — must NOT promote (headerless Luhn
    // is not self-precise).
    let l = recovered_label(
        "device_serial",
        &["490154203237518", "352043068649148", "011245002151205"],
        "representation.numeric.integer_number",
    );
    assert_ne!(l, "technology.code.imei");
}

#[test]
fn imei_checksum_recovery_declines_amex_card_column() {
    // 15-digit American Express card numbers are Luhn-valid BY CONSTRUCTION; only
    // the header gate keeps this off live payment data.
    let l = recovered_label(
        "card_number",
        &["378282246310005", "371449635398431", "378734493671000"],
        "representation.numeric.integer_number",
    );
    assert_ne!(l, "technology.code.imei");
}

#[test]
fn cpt_procedure_recovery_promotes_with_header() {
    let l = recovered_label(
        "cpt",
        &["99213", "29580", "2029F", "0307T", "99214"],
        "representation.text.word",
    );
    assert_eq!(l, "identity.medical.cpt");
}

#[test]
fn cpt_procedure_recovery_declines_generic_code_header() {
    // A 5-digit ZIP column headed with a GENERIC `code` token must not be promoted
    // — CPT admits only the distinctive `cpt`/`procedure` token, unlike naics.
    let l = recovered_label(
        "code",
        &["99213", "10001", "94103", "60601", "30301"],
        "representation.identifier.numeric_code",
    );
    assert_ne!(l, "identity.medical.cpt");
}

#[test]
fn hs_code_header_recovery_promotes_with_header() {
    let l = recovered_label(
        "hs_code",
        &[
            "8471.30",
            "6110.20.20",
            "0901.11.00.10",
            "8517.12",
            "3004.90.92",
        ],
        "representation.text.word",
    );
    assert_eq!(l, "geography.transportation.hs_code");
}

#[test]
fn hs_code_header_recovery_declines_year_column() {
    // Bare 4-digit years pass is_hs_code_format's loose branch; the median-length
    // floor (>=6) rejects them even under a coincidental customs header.
    let l = recovered_label(
        "hs",
        &["2019", "2020", "2021", "2022", "2023"],
        "representation.numeric.integer_number",
    );
    assert_ne!(l, "geography.transportation.hs_code");
}

#[test]
fn unlocode_membership_recovery_promotes_members() {
    let l = recovered_label(
        "port",
        &["USLAX", "GBLON", "DEHAM", "SGSIN", "NLRTM"],
        "representation.text.word",
    );
    assert_eq!(l, "geography.transportation.unlocode");
}

#[test]
fn unlocode_membership_recovery_declines_non_members() {
    // 5-char uppercase tokens that are not published UN/LOCODEs (the ticker/SKU
    // attractor) must not be promoted — membership, not shape, is the gate.
    let l = recovered_label(
        "code",
        &["ZZZZZ", "QQQQQ", "XKXKX", "VWVWV", "JKJKJ"],
        "representation.text.word",
    );
    assert_ne!(l, "geography.transportation.unlocode");
}

#[test]
fn unlocode_membership_recovery_declines_constant_column() {
    // Regression (corpus-honest spot-check, cert-guard batch): a fund `symbol` column of
    // one repeated ticker (`FRMUF`) — like a `city` column of `Essen` or a `Namespace`
    // column of `Debug` — coincidentally matches a single UN/LOCODE entry, since the
    // 110k-entry set makes 5-char collisions common, and fired the guard at 100%
    // membership. 57 of 58 corpus promotions were such constant columns. The distinct
    // gate rejects them: this rare location-code type needs distributional evidence, not
    // one repeated coincidence (mirrors cusip_checksum_recovery_declines_constant_column).
    let l = recovered_label(
        "symbol",
        &["FRMUF", "FRMUF", "FRMUF", "FRMUF"],
        "representation.text.entity_name",
    );
    assert_ne!(l, "geography.transportation.unlocode");
}

// ── ticker_membership_recovery (company-reference external band) ──

#[test]
fn ticker_membership_recovery_promotes_symbols() {
    // EDGAR `ticker` was over-emitted as state_code; a ticker-headed column of
    // US-listed symbols promotes to the membership-backed ticker leaf.
    let l = recovered_label(
        "ticker",
        &["AAPL", "MSFT", "NVDA", "TSLA", "AMZN"],
        "geography.location.state_code",
    );
    assert_eq!(l, "finance.securities.ticker");
}

#[test]
fn ticker_membership_recovery_requires_ticker_header() {
    // Header gate is load-bearing: these ARE real tickers (>=90% membership),
    // but under a `state` header they must NOT promote — 15 of 50 state codes
    // are also tickers, so only the header separates a ticker from a state list.
    let l = recovered_label(
        "state",
        &["AAPL", "MSFT", "NVDA", "TSLA", "AMZN"],
        "geography.location.state_code",
    );
    assert_ne!(l, "finance.securities.ticker");
}

#[test]
fn ticker_membership_recovery_declines_non_members() {
    // Ticker-headed, but the values are not US-listed symbols (arbitrary tokens):
    // membership, not the header alone, is the gate.
    let l = recovered_label(
        "ticker",
        &["ZZZZZ", "QQQQXY", "NOTATCKR", "WResReW", "FooBarBz"],
        "representation.text.word",
    );
    assert_ne!(l, "finance.securities.ticker");
}

#[test]
fn ticker_membership_recovery_declines_constant_column() {
    // Constant column (one repeated real ticker) matches at 100% but carries no
    // distributional evidence — the >=3-distinct gate rejects it (unlocode lesson).
    let l = recovered_label(
        "symbol",
        &["AAPL", "AAPL", "AAPL", "AAPL"],
        "representation.text.word",
    );
    assert_ne!(l, "finance.securities.ticker");
}

// ── org_name_geography_demotion (external band seam 1c) ──

#[test]
fn org_name_geography_demotion_demotes_org_column() {
    // gleif `name` was over-emitted as region; org/fund suffixes are the self-precise
    // tell (a place is never named "… PLC" / "… Fund" / "… LP").
    let l = recovered_label(
        "name",
        &[
            "ICON STOCKBROKERS LIMITED",
            "NIGERIAN BREWERIES PLC",
            "Oakmark International Fund",
            "Hutchin Hill Capital, LP",
            "Vanguard Fiduciary Trust Company",
        ],
        "geography.location.region",
    );
    assert_eq!(l, "representation.text.entity_name");
}

#[test]
fn org_name_geography_demotion_keeps_place_names() {
    // Real region NAMES under a region header carry ZERO org suffixes — must stay
    // geography (the precision boundary: no header/membership signal protects these,
    // only the value-suffix ratio does).
    let l = recovered_label(
        "region",
        &[
            "California",
            "Texas",
            "Florida",
            "Illinois",
            "Ohio",
            "Virginia",
        ],
        "geography.location.region",
    );
    assert_eq!(l, "geography.location.region");
}

#[test]
fn org_name_geography_demotion_keeps_bare_geo_codes() {
    // Corpus spot-check regression: `AB` (Alberta) / `NV` (Nevada) / `SE` (Sweden)
    // are 2-letter COMPANY forms (Aktiebolag / Naamloze Vennootschap / Societas
    // Europaea) AND geo codes. A bare-code state/country column must NOT be demoted —
    // the ≥2-token rule (a suffix must be part of a multi-word NAME) is what excludes it.
    // The regression is specifically "not demoted to entity_name" — the column must
    // stay geographic (a sibling geo guard may normalise state_code<->region, which
    // is not this guard's concern).
    let state = recovered_label(
        "state",
        &["AB", "AB", "NV", "NV", "AB", "NV"],
        "geography.location.state_code",
    );
    assert!(
        state.starts_with("geography."),
        "bare state codes must stay geographic, got {state}"
    );
    let country = recovered_label(
        "country_code",
        &["SE", "SE", "AG", "SE", "AG", "SE"],
        "geography.location.country_code",
    );
    assert!(
        country.starts_with("geography."),
        "bare country codes must stay geographic, got {country}"
    );
}

#[test]
fn org_name_geography_demotion_keeps_address_columns() {
    // 33k-gate spot-check regression: a real street-address column is legitimately
    // multi-word free text whose tokens collide with org suffixes — `4th Street SE`
    // (SE = South-East), `Royal Trust Tower` (a building), `Bairro Asa` (a Brasília
    // district, not the Norwegian ASA form). These are 100% of the observed FPs and
    // all live in `geography.address.*`. Gating the guard on place-NAME leaves means
    // an address leaf is never demoted, regardless of the false-friend token count.
    for base in [
        "geography.address.full_address",
        "geography.address.street_name",
    ] {
        let l = recovered_label(
            "address1",
            &[
                "6143 - 4th Street SE",
                "1331 Macleod Trail SE",
                "Royal Trust Tower Suite 4500",
                "725 11th Avenue Boulevard SE",
            ],
            base,
        );
        // Must stay an address leaf — not demoted to entity_name. (A sibling
        // address-composition rule may normalise street_name<->full_address; that is
        // not this guard's concern — the regression is specifically "not demoted".)
        assert!(
            l.starts_with("geography.address."),
            "address column carrying directional/building tokens must NOT be demoted (base {base}, got {l})"
        );
    }
}

// ── tld_geography_recovery (external band: majestic TLD → continent) ──

#[test]
fn tld_geography_recovery_promotes_tlds() {
    let l = recovered_label(
        "TLD",
        &["com", "org", "net", "edu", "gov", "uk", "io"],
        "geography.location.continent",
    );
    assert_eq!(l, "technology.internet.top_level_domain");
}

#[test]
fn tld_geography_recovery_requires_tld_header() {
    // ccTLDs are >=90% TLD-set members but under a `country` header this is a
    // country column, not a domain column — the header gate must block it.
    let l = recovered_label(
        "country",
        &["uk", "de", "fr", "jp", "cn", "it"],
        "geography.location.continent",
    );
    assert_ne!(l, "technology.internet.top_level_domain");
}

#[test]
fn color_rgb_recovery_promotes_anchored_rgb() {
    let l = recovered_label(
        "colour",
        &[
            "rgb(255, 0, 0)",
            "rgb(0, 255, 0)",
            "rgba(0, 0, 255, 0.5)",
            "rgb(128, 128, 128)",
        ],
        "representation.text.word",
    );
    assert_eq!(l, "representation.format.color_rgb");
}

#[test]
fn color_rgb_recovery_declines_bare_triples() {
    // Bare comma triples are genuinely ambiguous (coordinate / comma_separated /
    // word); without the literal rgb( prefix they must not be promoted.
    let l = recovered_label(
        "colour",
        &["255, 0, 0", "0, 255, 0", "128, 128, 128", "12, 45, 90"],
        "representation.text.word",
    );
    assert_ne!(l, "representation.format.color_rgb");
}

// ── unlocode_format_veto (BACKLOG #11) ──

#[test]
fn unlocode_format_veto_demotes_contradicting_values() {
    // unlocode over UK-postcode values (space + digits — forbidden by the unlocode
    // shape) → demoted. With no postal locale defined, the safe fallback is unknown.
    let yaml = r#"
geography.transportation.unlocode:
  title: UN/LOCODE
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["USNYC"]
  validation:
    type: string
    pattern: '^[A-Z]{2}[A-Z2-9]{3}$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let postcodes: Vec<String> = vec!["CM13 3GF", "SW1A 1AA", "EC1A 1BB"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("loc", &postcodes, "geography.transportation.unlocode", 0.97)
        .unwrap();
    assert_ne!(
        r.label, "geography.transportation.unlocode",
        "contradicting values must demote the unlocode overcall"
    );
}

#[test]
fn unlocode_format_veto_keeps_valid_unlocodes() {
    // Genuine UN/LOCODEs pass the shape → untouched.
    let yaml = r#"
geography.transportation.unlocode:
  title: UN/LOCODE
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["USNYC"]
  validation:
    type: string
    pattern: '^[A-Z]{2}[A-Z2-9]{3}$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let codes: Vec<String> = vec!["USNYC", "DEHAM", "NLRTM", "GBLON"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("loc", &codes, "geography.transportation.unlocode", 0.97)
        .unwrap();
    assert_eq!(r.label, "geography.transportation.unlocode");
}

// ── membership_substance_guard (company-reference audit W2) ──

#[test]
fn membership_substance_guard_is_default_on() {
    assert!(!rhh::is_disabled("membership_substance_guard"));
}

#[test]
fn membership_guard_demotes_tickers_off_icao() {
    // The reproduced ticker→icao failure: 4-letter stock tickers pass the
    // shape pattern ^[A-Z]{4}$ 100% (validation CONFIRMS the wrong label and
    // disarms the attractor demotion), but they are not in the published ICAO
    // airport list — membership is the substance the shape cannot supply. The
    // header here is `ident` (a real airport-column name), NOT `ticker`/`symbol`:
    // this isolates membership_substance_guard's icao demotion, since a
    // ticker/symbol-headed column of real tickers now routes to the dedicated
    // ticker_membership_recovery promote instead (its own tests).
    let yaml = r#"
geography.transportation.icao_code:
  title: ICAO Airport Code
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["KJFK"]
  validation:
    type: string
    pattern: '^[A-Z]{4}$'
  membership: icao_airports
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let tickers: Vec<String> = vec![
        "AAPL", "MSFT", "TSLA", "NVDA", "AMZN", "META", "NFLX", "ORCL", "ADBE", "INTC",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "ident",
            &tickers,
            "geography.transportation.icao_code",
            0.93,
        )
        .unwrap();
    assert_ne!(
        r.label, "geography.transportation.icao_code",
        "non-member tickers must demote the icao overcall"
    );
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("membership_substance_guard:geography.transportation.icao_code")
    );
}

#[test]
fn jwt_substance_guard_is_default_on() {
    assert!(!rhh::is_disabled("jwt_substance_guard"));
}

#[test]
fn jwt_guard_demotes_nonjwt_dotted_strings() {
    // Three-base64url-segment strings that pass the SHAPE pattern but whose
    // header is not JSON-with-alg — the corpus over-emission (paths/prose/tokens).
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(jwt_guard_taxonomy());
    // Underscore tokens: pass the base64url 3-segment shape, fail is_jwt (header
    // is not JSON-with-alg), and are NOT valid hostnames (so no downstream
    // hostname recovery re-labels the demoted `unknown`).
    let vals: Vec<String> = vec![
        "tok_aaaa1.pay_bbbb2.sig_cccc3",
        "tok_dddd4.pay_eeee5.sig_ffff6",
        "tok_gggg7.pay_hhhh8.sig_iiii9",
        "tok_jjjj0.pay_kkkk1.sig_llll2",
        "tok_mmmm3.pay_nnnn4.sig_oooo5",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("token", &vals, "technology.cryptographic.jwt", 0.9)
        .unwrap();
    assert_ne!(
        r.label, "technology.cryptographic.jwt",
        "non-JWT dotted strings must demote off jwt"
    );
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("jwt_substance_guard")
    );
}

#[test]
fn jwt_guard_keeps_real_tokens() {
    // Genuine JWTs (header decodes to JSON with `alg`) → untouched.
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(jwt_guard_taxonomy());
    let vals: Vec<String> = vec![
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
        "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJhdXRoLmV4YW1wbGUuY29tIn0.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIyIn0.aaaa",
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIzIn0.bbbb",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("token", &vals, "technology.cryptographic.jwt", 0.9)
        .unwrap();
    assert_eq!(
        r.label, "technology.cryptographic.jwt",
        "genuine JWTs must be kept"
    );
}

#[test]
fn membership_guard_keeps_real_airport_codes() {
    // Genuine ICAO columns are list members → untouched.
    let yaml = r#"
geography.transportation.icao_code:
  title: ICAO Airport Code
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["KJFK"]
  validation:
    type: string
    pattern: '^[A-Z]{4}$'
  membership: icao_airports
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let codes: Vec<String> = vec!["KJFK", "KLAX", "LFPG", "EGLL", "RJTT", "YSSY"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "airport",
            &codes,
            "geography.transportation.icao_code",
            0.93,
        )
        .unwrap();
    assert_eq!(r.label, "geography.transportation.icao_code");
}

#[test]
fn membership_guard_demotes_tickers_off_unlocode() {
    // dataset-descriptor audit (EDGAR `ticker`): stock tickers clear the
    // UN/LOCODE shape `^[A-Z]{2}[A-Z2-9]{3}$` (and the model calls the column
    // unlocode at 0.90), but they are not in the published UNECE list. The
    // 5-char shape is why the shape-only validator fix never touched it;
    // membership is the substance that demotes the overcall.
    let yaml = r#"
geography.transportation.unlocode:
  title: UN/LOCODE
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["USLAX"]
  validation:
    type: string
    pattern: '^[A-Z]{2}[A-Z2-9]{3}$'
  membership: unlocode
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    // Real 5-char tickers / fund symbols, all verified non-members of the set.
    let tickers: Vec<String> = vec![
        "GOOGL", "BRKAX", "AMZNX", "ZVZZT", "VTSAX", "FXAIX", "SWPPX", "VFIAX", "VBIAX", "QQQQY",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "ticker",
            &tickers,
            "geography.transportation.unlocode",
            0.90,
        )
        .unwrap();
    assert_ne!(
        r.label, "geography.transportation.unlocode",
        "non-member tickers must demote the unlocode overcall"
    );
    assert_eq!(
        r.disambiguation_rule.as_deref(),
        Some("membership_substance_guard:geography.transportation.unlocode")
    );
}

#[test]
fn membership_guard_keeps_real_unlocodes() {
    // Genuine UN/LOCODE columns are list members → untouched.
    let yaml = r#"
geography.transportation.unlocode:
  title: UN/LOCODE
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["USLAX"]
  validation:
    type: string
    pattern: '^[A-Z]{2}[A-Z2-9]{3}$'
  membership: unlocode
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let codes: Vec<String> = vec!["USLAX", "GBLON", "DEHAM", "SGSIN", "NLRTM", "USNYC"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("port", &codes, "geography.transportation.unlocode", 0.93)
        .unwrap();
    assert_eq!(r.label, "geography.transportation.unlocode");
}

#[test]
fn membership_guard_demotes_nonmember_codes_off_iata() {
    // iata's set covers ~52% of the 3-letter space, so unlike icao the guard
    // only decides columns that skew non-member (a major-currency column is
    // ~65% real airports — GBP/JPY/CHF/AUD — and is deliberately KEPT by the
    // ≥50% bar; demote-only means that is no worse than the shape-only status
    // quo). These are verified non-members: the guard must act.
    let yaml = r#"
geography.transportation.iata_code:
  title: IATA Airport Code
  designation: universal
  tier: [VARCHAR, transportation]
  samples: ["JFK"]
  validation:
    type: string
    pattern: '^[A-Z]{3}$'
  membership: iata_airports
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    let codes: Vec<String> = vec![
        "USD", "EUR", "NZD", "INR", "RUB", "NKE", "PFE", "CVX", "XOM",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("code", &codes, "geography.transportation.iata_code", 0.9)
        .unwrap();
    assert_ne!(
        r.label, "geography.transportation.iata_code",
        "non-member codes must demote the iata overcall"
    );
}

// ── R33 entity_prose_override + numeric_code_header_recovery + swift_bic R32
//    (company-reference audit, gold-priced batch) ──

#[test]
fn entity_prose_override_demotes_prose_to_plain_text() {
    // Sentence-like description prose asserted as entity_name → plain_text.
    let values: Vec<String> = vec![
        "Type 2 diabetes mellitus without complications",
        "Provides consulting and advisory services to businesses",
        "Chronic obstructive pulmonary disease unspecified",
        "Offers cloud software for enterprise resource planning",
        "Alcohol dependence uncomplicated with related disorders",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.text.entity_name", 0.93, None);
    let (label, rule) = result.expect("prose should demote off entity_name");
    assert_eq!(label, "representation.text.plain_text");
    assert!(rule.starts_with("entity_prose_override:"));
}

#[test]
fn entity_prose_override_keeps_org_names_with_connectors() {
    // Genuine org names — including connector-word and long Title-Case names —
    // must NOT demote (the measured zero-false-fire bar).
    let values: Vec<String> = vec![
        "Bank of America Corporation",
        "McKinsey & Company",
        "Fidelity Advisor Leveraged Company Stock Fund",
        "University of California Press",
        "Museum of Modern Art",
        "Procter & Gamble Co",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.text.entity_name", 0.93, None);
    assert!(
        result.is_none(),
        "org names with lowercase connectors must not demote off entity_name"
    );
}

#[test]
fn entity_prose_override_keeps_species_binomials() {
    // Two-token species names carry one lowercase content word — below both
    // the median-token and prose-value bars.
    let values: Vec<String> = vec![
        "Homo sapiens",
        "Mus musculus",
        "Rattus norvegicus",
        "Danio rerio",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.text.entity_name", 0.93, None);
    assert!(result.is_none(), "species binomials must stay entity_name");
}

#[test]
fn numeric_code_recovery_restores_naics_and_cik() {
    // F5 demotes no-leading-zero numeric_code to integer_number; a code-ish
    // header restores it (0094 corroboration). NAICS + CIK shapes.
    let yaml = r#"
representation.identifier.numeric_code:
  title: "Numeric Code"
  validation:
    type: string
    pattern: '^[0-9]+$'
  tier: [VARCHAR, identifier]
  release_priority: 4
  samples: ["00120"]
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    // Realistic CIK column (SEC registrant ids): repeated values keep the
    // numeric_sequential_detection promotion from grabbing the column as
    // `increment` before this guard can see it. (A naics-headed column
    // continues past numeric_code to the identity.industry.naics leaf via
    // naics_industry_recovery — covered by its own tests.)
    let ciks: Vec<String> = vec![
        "320193", "1652044", "789019", "1018724", "1318605", "320193", "789019", "1067983",
        "1652044", "1318605", "320193", "104169",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("cik", &ciks, "representation.numeric.integer_number", 0.8)
        .unwrap();
    assert_eq!(
        r.label, "representation.identifier.numeric_code",
        "rule was {:?}",
        r.disambiguation_rule
    );
    assert!(r
        .disambiguation_rule
        .as_deref()
        .unwrap_or("")
        .starts_with("numeric_code_header_recovery:"));
}

#[test]
fn numeric_code_recovery_leaves_quantities_and_postal_alone() {
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    // Quantity header: no code token → untouched.
    let counts: Vec<String> = vec!["1200", "845", "23000", "410"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "employees",
            &counts,
            "representation.numeric.integer_number",
            0.8,
        )
        .unwrap();
    assert_eq!(r.label, "representation.numeric.integer_number");
    // Postal header: `zip_code` tokenises to a bare `code` but the postal
    // token vetoes the match — this guard must NOT claim the column (other
    // postal machinery may legitimately relabel it).
    let zips: Vec<String> = vec!["90210", "10001", "60601", "94102"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "zip_code",
            &zips,
            "representation.numeric.integer_number",
            0.8,
        )
        .unwrap();
    assert_ne!(r.label, "representation.identifier.numeric_code");
    assert!(!r
        .disambiguation_rule
        .as_deref()
        .unwrap_or("")
        .starts_with("numeric_code_header_recovery:"));
}

#[test]
fn test_schema_fail_demotion_names_off_swift_bic() {
    // Normalized-name columns the model wears swift_bic on (measured on real
    // GLEIF data at pass_rate 0.007) → demote via R32.
    let values: Vec<String> = (0..30)
        .map(|i| format!("ACME HOLDINGS INTERNATIONAL {}", i))
        .collect();
    let yaml = r#"
finance.banking.swift_bic:
  title: "SWIFT/BIC"
  validation:
    type: string
    pattern: "^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}([A-Z0-9]{3})?$"
  tier: [VARCHAR, banking]
  release_priority: 3
  samples: ["DEUTDEFF"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "finance.banking.swift_bic", 0.9, Some(&taxonomy));
    let (label, rule) = result.expect("name strings should demote off swift_bic");
    assert_eq!(label, "representation.identifier.alphanumeric_id");
    assert!(rule.starts_with("schema_fail_demotion:"));
}

#[test]
fn test_schema_fail_demotion_keeps_real_swift_bic() {
    let values: Vec<String> = vec![
        "DEUTDEFF",
        "CHASUS33",
        "BARCGB22",
        "BNPAFRPP",
        "CITIUS33XXX",
        "HSBCHKHH",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let yaml = r#"
finance.banking.swift_bic:
  title: "SWIFT/BIC"
  validation:
    type: string
    pattern: "^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}([A-Z0-9]{3})?$"
  tier: [VARCHAR, banking]
  release_priority: 3
  samples: ["DEUTDEFF"]
"#;
    let taxonomy = Taxonomy::from_yaml(yaml).unwrap();
    let result = value_sharpen(&values, "finance.banking.swift_bic", 0.9, Some(&taxonomy));
    assert!(
        result.is_none(),
        "real BIC values must not be demoted off swift_bic"
    );
}

// ── naics_industry_recovery (company-reference audit W3) ──

#[test]
fn naics_recovery_is_default_on() {
    assert!(!rhh::is_disabled("naics_industry_recovery"));
}

#[test]
fn naics_recovery_promotes_naics_headed_member_columns() {
    // A NAICS column arrives here as integer_number (F5) or numeric_code
    // (header recovery); the leaf guard promotes on naics header + >=90%
    // membership in the published Census list.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let naics: Vec<String> = vec![
        "541511", "236220", "722511", "621111", "445110", "541511", "722511", "236220", "541512",
        "928120",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense(
            "NAICS Code",
            &naics,
            "representation.numeric.integer_number",
            0.7,
        )
        .unwrap();
    assert_eq!(
        r.label, "identity.industry.naics",
        "rule was {:?}",
        r.disambiguation_rule
    );
}

#[test]
fn naics_recovery_requires_the_header_gate() {
    // Same member values under a quantity header must NOT promote — sector
    // codes are value-identical with small integers; the header is load-bearing.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let vals: Vec<String> = vec!["11", "21", "23", "31", "42", "44", "11", "21"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "team_size",
            &vals,
            "representation.numeric.integer_number",
            0.7,
        )
        .unwrap();
    assert_ne!(r.label, "identity.industry.naics");
}

#[test]
fn naics_recovery_requires_membership() {
    // naics-headed column whose values are NOT in the published list (bad
    // sector prefixes) must not promote.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let vals: Vec<String> = vec!["100000", "999999", "050000", "130000", "990000"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense(
            "naics_code",
            &vals,
            "representation.numeric.integer_number",
            0.7,
        )
        .unwrap();
    assert_ne!(r.label, "identity.industry.naics");
}

#[test]
fn s_expression_recovery_is_default_on() {
    assert!(!rhh::is_disabled("s_expression_recovery"));
}

#[test]
fn s_expression_recovery_promotes_parse_trees() {
    // Parse trees reach the guard as container.array.comma_separated (the Penn
    // comma-tokens `(, ,)` fool the delimiter detector). No header gate — the
    // balanced-nested-paren structure is self-precise.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let trees: Vec<String> = vec![
        "(ROOT (S (NP (NN dog)) (VP (VBZ runs))))",
        "(ROOT (FRAG (INTJ (UH uh)) (, ,) (NP (NN yeah))))",
        "(S (NP (PRP I)) (VP (VBP think) (SBAR (S (NP (PRP it))))))",
        "(ROOT (SINV (ADVP (RB so)) (, ,) (SBARQ (WHNP (WP what)))))",
        "(program (call (id print) (string hi)))",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("parse_tree", &trees, "container.array.comma_separated", 0.8)
        .unwrap();
    assert_eq!(
        r.label, "container.object.s_expression",
        "rule was {:?}",
        r.disambiguation_rule
    );
}

#[test]
fn s_expression_recovery_declines_genuine_comma_lists() {
    // A real comma-separated list must NOT be pulled into s_expression.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let lists: Vec<String> = vec!["apple,banana,cherry", "red,green,blue", "1,2,3,4"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("tags", &lists, "container.array.comma_separated", 0.8)
        .unwrap();
    assert_ne!(r.label, "container.object.s_expression");
}

#[test]
fn s_expression_recovery_tolerates_truncated_trees() {
    // Long parse trees may reach the guard clipped mid-tree (open, unbalanced);
    // the truncation-tolerant structural check still promotes them.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let trees: Vec<String> = vec![
        "(ROOT (SINV (ADVP (RB so)) (, ,) (SBARQ (INTJ (UH uh",
        "(ROOT (S (NP (NP (DT the) (NN cat)) (PP (IN on) (NP (DT the",
        "(S (INTJ (UH Uh)) (, ,) (NP-SBJ (PRP I)) (VP (VBP think) (SBAR",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("trees", &trees, "container.array.comma_separated", 0.8)
        .unwrap();
    assert_eq!(
        r.label, "container.object.s_expression",
        "rule was {:?}",
        r.disambiguation_rule
    );
}

#[test]
fn naics_recovery_admits_bare_code_header_for_long_codes() {
    // The real product surface: a 6-digit NAICS column under a bare `code`
    // header. Membership at 6 digits is decisive; the generic-code tier admits it.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let vals: Vec<String> = vec![
        "541511", "236220", "722511", "621111", "445110", "541512", "722511", "236220",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let r = cc
        .compose_from_sense("code", &vals, "representation.numeric.integer_number", 0.7)
        .unwrap();
    assert_eq!(
        r.label, "identity.industry.naics",
        "rule was {:?}",
        r.disambiguation_rule
    );
}

#[test]
fn naics_recovery_rejects_bare_code_header_for_sector_length_values() {
    // 2-digit values 11-92 are ALL valid sectors — a rating-like column headed
    // `code` must not promote; only the distinctive `naics` token admits
    // sector-level codes.
    let cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    let vals: Vec<String> = vec!["11", "22", "31", "44", "55", "62", "72", "81", "11", "44"]
        .into_iter()
        .map(String::from)
        .collect();
    let r = cc
        .compose_from_sense("code", &vals, "representation.numeric.integer_number", 0.7)
        .unwrap();
    assert_ne!(r.label, "identity.industry.naics");
}
