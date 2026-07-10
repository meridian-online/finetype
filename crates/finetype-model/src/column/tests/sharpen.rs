use super::super::*;

#[test]
fn test_strip_locale_suffix_4level_universal() {
    let (base, locale) = strip_locale_suffix("representation.boolean.binary.UNIVERSAL");
    assert_eq!(base, "representation.boolean.binary");
    assert_eq!(locale, Some("UNIVERSAL"));
}

#[test]
fn test_strip_locale_suffix_3level_unchanged() {
    let (base, locale) = strip_locale_suffix("geography.address.postal_code");
    assert_eq!(base, "geography.address.postal_code");
    assert_eq!(locale, None);
}

#[test]
fn test_strip_locale_suffix_short_locale() {
    let (base, locale) = strip_locale_suffix("geography.location.city.EN");
    assert_eq!(base, "geography.location.city");
    assert_eq!(locale, Some("EN"));
}

#[test]
fn test_strip_locale_suffix_no_false_positive_on_type() {
    // "iso" is lowercase — should NOT be treated as a locale suffix
    let (base, locale) = strip_locale_suffix("datetime.date.iso");
    assert_eq!(base, "datetime.date.iso");
    assert_eq!(locale, None);
}

#[test]
fn test_strip_locale_suffix_no_false_positive_on_short_label() {
    // Only two parts — the last part should not be treated as locale
    let (base, locale) = strip_locale_suffix("representation.EN");
    assert_eq!(base, "representation.EN");
    assert_eq!(locale, None);
}

// === keyword guard tests ===

#[test]
fn ac01_r25_http_status_gate_fires_on_status_codes() {
    // 3-digit HTTP status codes in 100-599 should NOT be converted to postal_code.
    // R25 is now a guard inside R12's disambiguate_numeric — when the model
    // predicts integer_number and values are 3-digit 100-599, R12 should NOT
    // override to postal_code.
    let values: Vec<String> = vec!["200", "404", "500", "301", "503"]
        .into_iter()
        .map(String::from)
        .collect();
    // When model predicts integer_number, R12 fires but R25 guard blocks postal_code
    let result = value_sharpen(&values, "representation.numeric.integer_number", 0.8, None);
    // Should NOT return postal_code
    if let Some((label, _)) = &result {
        assert_ne!(
            label, "geography.address.postal_code",
            "R25 guard should prevent postal_code for HTTP status codes"
        );
    }
}

#[test]
fn v15_region_not_mapped_to_state() {
    // "region" should NOT map to state — model handles this correctly
    assert_eq!(header_hint("region"), None);
    // "state"/"province" now map to the LIVE region leaf (state is a retired alias)
    assert_eq!(header_hint("state"), Some("geography.location.region"));
    assert_eq!(header_hint("province"), Some("geography.location.region"));
}

// ── R32: text-family low-cardinality vocabulary override ────────────

#[test]
fn r32_vocab_overrides_word() {
    // A status vocabulary asserted as free text is a categorical.
    let values: Vec<String> = vec![
        "active", "inactive", "active", "pending", "active", "inactive",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    let result = value_sharpen(&values, "representation.text.word", 0.9, None);
    assert!(
        result.is_some(),
        "R32 should fire on a small repeated vocab"
    );
    let (label, rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
    assert!(rule.starts_with("text_vocab_override:"));
}

#[test]
fn r32_preserves_distinct_free_text() {
    // Species names / free words are mostly distinct — never a vocab.
    let values: Vec<String> = vec![
        "Dichocarpum",
        "adiantifolium",
        "sutchuenense",
        "arisanense",
        "auriculatum",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for label in [
        "representation.text.word",
        "representation.text.entity_name",
        "representation.text.plain_text",
    ] {
        assert!(
            value_sharpen(&values, label, 0.9, None).is_none(),
            "R32 must not fire on distinct free text ({label})"
        );
    }
}

#[test]
fn r32_preserves_constant_and_short_columns() {
    let constant: Vec<String> = vec!["s"; 6].into_iter().map(String::from).collect();
    assert!(
        value_sharpen(&constant, "representation.text.word", 0.9, None).is_none(),
        "single distinct value is not a vocabulary"
    );
    let short: Vec<String> = vec!["a".into(), "b".into(), "a".into()];
    assert!(
        value_sharpen(&short, "representation.text.word", 0.9, None).is_none(),
        "below the n>=4 floor"
    );
}

#[test]
fn r32_out_of_scope_labels_untouched() {
    // Excluded by design: city/region (legitimately low-cardinality
    // vocabularies exist) and — measured by the corpus-honest gate
    // round 1 — entity_name/plain_text, where the oracle refuted
    // 3,752/2,115 moves (repeating manufacturer names ARE entity_name;
    // repeated boilerplate IS plain_text).
    let values: Vec<String> = vec![
        "Sydney",
        "Melbourne",
        "Sydney",
        "Sydney",
        "Melbourne",
        "Sydney",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for label in [
        "geography.location.city",
        "representation.text.entity_name",
        "representation.text.plain_text",
    ] {
        let result = value_sharpen(&values, label, 0.9, None);
        if let Some((_, rule)) = &result {
            assert!(
                !rule.starts_with("text_vocab_override:"),
                "R32 must not fire on {label}"
            );
        }
    }
}

/// Integration test: verify that semantic hint classifier influences column classification.
/// Skips if Model2Vec model files are not present.
#[test]
fn test_classify_column_with_semantic_hint() {
    use crate::semantic::SemanticHintClassifier;

    let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("models")
        .join("model2vec");

    if !model_dir.join("model.safetensors").exists() {
        eprintln!("Skipping semantic column integration test: models/model2vec not found");
        return;
    }

    let semantic = SemanticHintClassifier::load(&model_dir).unwrap();

    // Create a mock classifier that delegates value-level inference
    // We use a simple stub here — the semantic hint should override generic
    // value predictions when the header name is semantically clear.
    let base_classifier =
        crate::inference::MockClassifier::new("representation.numeric.decimal_number");
    let column_classifier = ColumnClassifier::with_semantic_hint(
        Box::new(base_classifier),
        ColumnConfig::default(),
        semantic,
    );

    // The base classifier always returns decimal_number, but the semantic hint
    // for "weight_kg" should override to identity.person.weight
    let values: Vec<String> = vec!["72.5", "85.0", "63.2", "90.1"]
        .into_iter()
        .map(String::from)
        .collect();
    let result = column_classifier
        .classify_column_with_header(&values, "weight_kg")
        .unwrap();
    assert_eq!(
        result.label, "identity.person.weight",
        "Semantic hint for 'weight_kg' should override generic decimal_number"
    );

    // Generic column names should NOT override (semantic hint returns None)
    let result2 = column_classifier
        .classify_column_with_header(&values, "col1")
        .unwrap();
    assert_eq!(
        result2.label, "representation.numeric.decimal_number",
        "Generic 'col1' should not trigger semantic override"
    );
}

// ── Sharpen-specific tests (AC-2, AC-3) ──────────────────────────────
//
// These tests exercise the multi-branch Sharpen functions directly,
// using single-entry vote distributions that simulate multi-branch output.

// AC-2: feature_sharpen — F2 fires on hostname without docker_ref in votes
#[test]
fn test_sharpen_f2_hostname_high_slash_segments_no_docker_vote() {
    // Multi-branch predicts "hostname" but column has high slash segments
    // (e.g., "docker.io/library/nginx:latest"). With multi-branch single-entry
    // votes, docker_ref never appears as runner-up — F2 must fire on feature
    // threshold alone.
    let mut result = ColumnResult {
        label: "technology.internet.hostname".to_string(),
        confidence: 0.85,
        vote_distribution: vec![("technology.internet.hostname".to_string(), 0.85)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::SEGMENT_COUNT_SLASH] = 2.0; // ≥ 1.5 threshold

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "technology.development.docker_ref",
        "F2 should fire on hostname with high slash segments even without docker_ref in votes"
    );
    assert!(result.disambiguation_applied);
    assert!(result
        .disambiguation_rule
        .as_ref()
        .unwrap()
        .starts_with("feature_slash_segments"));
}

#[test]
fn test_sharpen_f2_hostname_low_slash_segments_stays() {
    // hostname with low slash segments should NOT trigger F2
    let mut result = ColumnResult {
        label: "technology.internet.hostname".to_string(),
        confidence: 0.85,
        vote_distribution: vec![("technology.internet.hostname".to_string(), 0.85)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::SEGMENT_COUNT_SLASH] = 0.5; // below 1.5

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "technology.internet.hostname",
        "F2 should NOT fire when slash segments are below threshold"
    );
    assert!(!result.disambiguation_applied);
}

// AC-2: feature_sharpen — F6 falls back to categorical with empty votes
#[test]
fn test_sharpen_f6_extension_to_categorical_empty_votes() {
    // Multi-branch predicts "file.extension" with single-entry votes
    // and short alphabetic values — F6 should fallback to categorical
    let mut result = ColumnResult {
        label: "representation.file.extension".to_string(),
        confidence: 0.75,
        vote_distribution: vec![("representation.file.extension".to_string(), 0.75)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 50,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };

    let mut cf = ColumnFeatures::empty();
    cf.mean[feature_idx::LENGTH] = 2.5; // ≤ 4.0
    cf.mean[feature_idx::SEGMENT_COUNT_DOT] = 0.0; // < 1.1
    cf.mean[feature_idx::ALPHA_RATIO] = 0.95; // ≥ 0.8

    feature_sharpen(&mut result, &cf);

    assert_eq!(
        result.label, "representation.text.word",
        "F6 should fallback to categorical with single-entry votes"
    );
    assert!(result.disambiguation_applied);
}

// AC-3: value_sharpen — R11 categorical with single-entry votes
#[test]
fn test_sharpen_r11_categorical_single_vote() {
    let values: Vec<String> = vec!["red", "blue", "green", "red", "blue"]
        .into_iter()
        .map(String::from)
        .collect();

    // Low cardinality values with a text-like label should trigger categorical
    let result = value_sharpen(&values, "identity.person.first_name", 0.70, None);

    assert!(
        result.is_some(),
        "R11 should fire for low-cardinality categorical values"
    );
    let (label, _rule) = result.unwrap();
    assert_eq!(label, "representation.text.word");
}

// ── compose_from_sense (corpus-honest gate fast path, spec 2026-06-27) ──

#[test]
fn compose_from_sense_runs_sharpen() {
    // compose_from_sense must run the REAL Sharpen stack on an injected Sense label
    // (the gate fast path). Proven empirically at 99.4% native parity; this pins the
    // behaviour so a refactor of the Sharpen sequence can't silently make it a no-op.
    let yaml = r#"
technology.internet.url:
  title: URL
  designation: universal
  tier: [VARCHAR, internet]
  release_priority: 3
  samples: ["https://x.com/a"]
  validation:
    type: string
    pattern: '^https?://[^\s]+$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    // Injected Sense = plain_text (demoted); the url recovery reader must restore url.
    let urls: Vec<String> = vec![
        "https://bitcointalk.org/index.php?topic=1".to_string(),
        "https://bitcointalk.org/index.php?topic=2".to_string(),
        "https://bitcointalk.org/index.php?topic=3".to_string(),
    ];
    let r = cc
        .compose_from_sense("parent_id", &urls, "representation.text.plain_text", 1.0)
        .unwrap();
    assert_eq!(r.label, "technology.internet.url", "url recovery must fire");

    // A genuine prose residual stays put — no spurious recovery.
    let prose: Vec<String> = vec![
        "just some prose here".to_string(),
        "and more words today".to_string(),
        "a third line of text".to_string(),
    ];
    let r2 = cc
        .compose_from_sense("notes", &prose, "representation.text.plain_text", 1.0)
        .unwrap();
    assert_eq!(r2.label, "representation.text.plain_text");
}

// ── structured_string_refinement (spec 2026-06-19-plain-text-type-discovery) ──

#[test]
fn test_structured_string_refinement() {
    let yaml = r#"
technology.filesystem.windows_path:
  title: Windows Path
  designation: universal
  tier: [VARCHAR, filesystem]
  release_priority: 3
  samples: ["C:\\x"]
  validation:
    type: string
    pattern: '^([A-Za-z]:\\|\\\\)[^\r\n]*$'
technology.internet.message_id:
  title: Message ID
  designation: universal
  tier: [VARCHAR, internet]
  release_priority: 3
  samples: ["<a@b>"]
  validation:
    type: string
    pattern: '^<[^<>@\s]+@[^<>@\s]+>$'
technology.code.qualified_name:
  title: Qualified Name
  designation: universal
  tier: [VARCHAR, code]
  release_priority: 3
  samples: ["a.b.c"]
  validation:
    type: string
    pattern: '^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*){2,}$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);

    let mk = |label: &str| ColumnResult {
        label: label.to_string(),
        confidence: 0.3,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 3,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    let run = |cc: &ColumnClassifier, label: &str, vals: &[&str]| {
        let mut r = mk(label);
        let s: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
        cc.structured_string_refinement(&mut r, &s);
        r.label
    };

    // Fires from the residual labels.
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &[r"C:\a\b.sys", r"D:\x\y.cs"]
        ),
        "technology.filesystem.windows_path"
    );
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &["<a.b@thyme>", "<c.d@thyme>"]
        ),
        "technology.internet.message_id"
    );
    assert_eq!(
        run(&cc, "representation.text.word", &["com.a.B", "org.c.D"]),
        "technology.code.qualified_name"
    );
    // windows_path also recovers from a path/locator misprediction (unambiguous validator).
    assert_eq!(
        run(
            &cc,
            "technology.internet.urn",
            &[r"C:\a\b.sys", r"D:\x\y.cs"]
        ),
        "technology.filesystem.windows_path"
    );
    // qualified_name must NOT eat a confident hostname (structural overlap).
    assert_eq!(
        run(
            &cc,
            "technology.internet.hostname",
            &["www.bbc.co.uk", "api.github.com"]
        ),
        "technology.internet.hostname"
    );
    // Prose is left alone (no validator passes).
    assert_eq!(
        run(
            &cc,
            "representation.text.plain_text",
            &["a sentence here", "more prose now"]
        ),
        "representation.text.plain_text"
    );
}
