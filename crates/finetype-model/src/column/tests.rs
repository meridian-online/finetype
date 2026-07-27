use super::*;

/// Minimal taxonomy carrying the country_code enum + the state_code EN_US enum,
/// with DE/IN/CA deliberately valid as BOTH (the collision the vote must survive).
fn geo_vote_classifier() -> ColumnClassifier {
    let yaml = r#"
geography.location.country_code:
  title: Country Code
  designation: universal
  tier: [VARCHAR, location]
  samples: ["US"]
  validation:
    type: string
    pattern: '^[A-Z]{2}$'
    enum: ["DE", "IT", "GB", "NL", "ES", "FR", "IN", "CN", "SE", "CA", "SK", "YT"]
geography.location.state_code:
  title: State Code
  designation: locale_specific
  tier: [VARCHAR, location]
  samples: ["CA"]
  validation:
    type: string
    pattern: '^[A-Z]{2}$'
  validation_by_locale:
    EN_US:
      type: string
      enum: ["DE", "IN", "CA", "TX", "NY", "FL"]
    EN_CA:
      type: string
      enum: ["NL", "SK", "YT", "QC", "ON", "BC", "MB"]
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    let mut cc =
        ColumnClassifier::with_defaults(Box::new(crate::inference::MockClassifier::new("unknown")));
    cc.set_taxonomy(tax);
    cc
}

/// Helper: create a ColumnResult for threshold tests.
fn make_result(label: &str, confidence: f32) -> ColumnResult {
    ColumnResult {
        label: label.to_string(),
        confidence,
        vote_distribution: vec![(label.to_string(), 1.0)],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 10,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    }
}

// ── full_name → username value veto (spec 2026-06-17-full-name-username-veto) ──

fn vals(xs: &[&str]) -> Vec<String> {
    xs.iter().map(|s| s.to_string()).collect()
}

fn jwt_guard_taxonomy() -> Taxonomy {
    let yaml = r#"
technology.cryptographic.jwt:
  title: JSON Web Token
  designation: universal
  tier: [VARCHAR, cryptographic]
  samples: ["eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.sig"]
  validation:
    type: string
    pattern: '^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$'
"#;
    let mut tax = Taxonomy::from_yaml(yaml).unwrap();
    tax.compile_validators();
    tax
}

mod attractor;
mod boolean;
mod datetime;
mod geo;
mod header;
mod identity;
mod misc;
mod numeric;
mod parallel_order;
mod rhh;
mod sharpen;
mod substance;
mod validation_gate;
