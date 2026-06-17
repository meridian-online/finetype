//! Enum-domain detection — choice 0102 (patch increment), spec
//! `2026-06-17-enum-domain-emission`.
//!
//! A column's OBSERVED bounded value domain, detected from full-column structure
//! and reported as a DESCRIPTIVE `x-finetype-enum` extension — NOT the
//! validation-enforced JSON-Schema `enum` keyword (an open domain there re-creates
//! the `enum_overfit` failure mode, card 0014). Decoupled from the semantic label
//! (0102): any bounded, non-denylisted column gets its domain regardless of type
//! (a `country_code` column is an enum AND a country). Shared by the CLI and MCP
//! emitters so the MCP shadows the CLI — one policy, not two.
//!
//! Predicate (calibrated, spec ac-00): `distinct <= max_distinct` AND
//! `distinct/rows <= max_ratio` AND the label is not denylisted. Character-shape
//! cohesion is EMITTED as an analyst score, NOT gated on — the ac-00 ablation
//! showed gating on it costs designed-enum coverage that `ratio` already covers.

use std::collections::BTreeSet;

/// A detected open enum domain, emitted descriptively (never as a closed
/// validation constraint in the patch).
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDomain {
    /// Sorted observed distinct values (the domain).
    pub domain: Vec<String>,
    /// Distinct non-empty value count (full column where the caller has it).
    pub distinct: usize,
    /// Non-empty row count.
    pub rows: usize,
    /// Character-shape cohesion in `[0, 1]` — fraction of distinct values sharing
    /// the dominant collapsed character-class shape. Emitted as an analyst signal.
    pub cohesion: f32,
    /// Always `true` in the patch — the domain is OPEN (may grow). Closed-dictionary
    /// recognition is deferred (0.7.0).
    pub open: bool,
}

/// Thresholds for the bounded-domain gate.
pub struct EnumConfig {
    pub max_distinct: usize,
    pub max_ratio: f32,
}

impl Default for EnumConfig {
    fn default() -> Self {
        // ac-00 calibration: cap 32 (matches the historical enum_threshold),
        // ratio 0.5 separates designed enums / bounded domains from open text.
        Self {
            max_distinct: 32,
            max_ratio: 0.5,
        }
    }
}

/// Label families where an enumerated value-domain is meaningless, so no enum is
/// emitted even when cardinality is low (ac-00: load-bearing — numbers are
/// shape-cohesive, so cohesion cannot exclude them; only this type denylist can).
/// A NEGATIVE exclusion, not a positive "only categorical" gate — that is the
/// 0102 decoupling.
pub fn is_enum_denylisted(label: &str) -> bool {
    label.starts_with("representation.numeric.")
        || label.starts_with("geography.coordinate.")
        || label.starts_with("datetime.")
        || label.starts_with("representation.identifier.")
        || label == "technology.internet.url"
}

/// Labels eligible for the CONSERVATIVE, validation-enforced JSON-Schema `enum`
/// KEYWORD (a closed constraint `validate` enforces). Kept deliberately narrow —
/// categorical + boolean — to avoid `enum_overfit` (card 0014): a closed `enum`
/// on an open domain rejects legitimate new values. The OPEN domain goes to the
/// descriptive `x-finetype-enum` extension instead (`detect_enum_domain`). Shared
/// so the CLI and MCP emit the SAME conservative keyword (the MCP shadows the CLI,
/// not a separate feature set). Moved here from the CLI's `enum_emission.rs` so
/// both surfaces depend on one policy.
pub fn label_is_enum_keyword_eligible(label: &str) -> bool {
    matches!(
        label,
        "representation.discrete.categorical"
            | "representation.boolean.binary"
            | "representation.boolean.initials"
            | "representation.boolean.terms"
    )
}

/// Collapsed character-class signature: runs of upper/lower/digit collapse to
/// `U`/`l`/`d`; other characters (space, punctuation) are kept literally.
/// `"US" -> "U"`, `"active" -> "l"`, `"conv_5" -> "l_d"`, `"San Jose" -> "Ul Ul"`.
fn shape_sig(v: &str) -> String {
    let mut out = String::new();
    let mut last: Option<char> = None;
    for ch in v.chars() {
        let class = if ch.is_ascii_uppercase() {
            'U'
        } else if ch.is_ascii_lowercase() {
            'l'
        } else if ch.is_ascii_digit() {
            'd'
        } else {
            ch // keep punctuation/space literally, never collapsed
        };
        // collapse consecutive runs of the same collapsing class
        if matches!(class, 'U' | 'l' | 'd') && last == Some(class) {
            continue;
        }
        out.push(class);
        last = Some(class);
    }
    out
}

/// Character-shape cohesion over the distinct values: the fraction sharing the
/// dominant collapsed shape. `1.0` for a uniform set, lower for mixed shapes.
pub fn cohesion(distinct: &[String]) -> f32 {
    if distinct.len() < 2 {
        return 1.0;
    }
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in distinct {
        *counts.entry(shape_sig(v)).or_insert(0) += 1;
    }
    let dominant = counts.values().copied().max().unwrap_or(0);
    dominant as f32 / distinct.len() as f32
}

/// Detect a column's open enum domain, or `None` when it is not a bounded
/// non-denylisted domain. `values` are the column's values (full column where the
/// caller has it); empty/whitespace values are ignored.
pub fn detect_enum_domain(label: &str, values: &[String], cfg: &EnumConfig) -> Option<EnumDomain> {
    if is_enum_denylisted(label) {
        return None;
    }
    let mut rows = 0usize;
    let mut set: BTreeSet<&str> = BTreeSet::new();
    for v in values {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        rows += 1;
        set.insert(t);
    }
    let distinct = set.len();
    if rows == 0 || distinct == 0 || distinct > cfg.max_distinct {
        return None;
    }
    if distinct as f32 / rows as f32 > cfg.max_ratio {
        return None;
    }
    let domain: Vec<String> = set.into_iter().map(|s| s.to_string()).collect();
    let coh = cohesion(&domain);
    Some(EnumDomain {
        distinct,
        rows,
        cohesion: coh,
        open: true,
        domain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn fires_on_designed_enum_any_label() {
        // A country_code column (NOT categorical) still gets its domain — the 0102
        // decoupling. {US,GB,FR} repeated.
        let vals = col(&["US", "GB", "FR", "US", "GB", "US", "FR", "GB", "US", "FR"]);
        let e = detect_enum_domain(
            "geography.location.country_code",
            &vals,
            &EnumConfig::default(),
        )
        .expect("bounded cohesive domain should fire");
        assert_eq!(e.domain, vec!["FR", "GB", "US"]);
        assert_eq!(e.distinct, 3);
        assert!(e.open);
        assert!((e.cohesion - 1.0).abs() < 1e-6, "all 2-upper share a shape");
    }

    #[test]
    fn denylist_excludes_numeric_datetime_id_url() {
        let small_ints = col(&["1", "2", "3", "1", "2", "3", "1", "2"]);
        assert!(detect_enum_domain(
            "representation.numeric.integer_number",
            &small_ints,
            &EnumConfig::default()
        )
        .is_none());
        let lat = col(&["1.0", "2.0", "3.0", "1.0", "2.0"]);
        assert!(detect_enum_domain(
            "geography.coordinate.latitude",
            &lat,
            &EnumConfig::default()
        )
        .is_none());
        let url = col(&["http://a.com", "http://b.com", "http://a.com"]);
        assert!(
            detect_enum_domain("technology.internet.url", &url, &EnumConfig::default()).is_none()
        );
    }

    #[test]
    fn ratio_rejects_near_unique() {
        // 6 distinct over 6 rows = ratio 1.0 > 0.5 — not a repeating domain.
        let vals = col(&["alpha", "bravo", "charlie", "delta", "echo", "foxtrot"]);
        assert!(
            detect_enum_domain("representation.text.word", &vals, &EnumConfig::default()).is_none()
        );
    }

    #[test]
    fn cardinality_cap_rejects_open_set() {
        // 40 distinct cities over 80 rows — exceeds the cap even at low ratio.
        let mut v = Vec::new();
        for i in 0..40 {
            v.push(format!("city{i}"));
            v.push(format!("city{i}"));
        }
        assert!(
            detect_enum_domain("geography.location.city", &v, &EnumConfig::default()).is_none()
        );
    }

    #[test]
    fn cohesion_distinguishes_uniform_from_mixed() {
        assert!((cohesion(&col(&["active", "inactive", "pending"])) - 1.0).abs() < 1e-6);
        // mixed shapes: word, number, date, name -> 4 distinct shapes -> 0.25
        let mixed = cohesion(&col(&["yes", "42", "2021-01-01", "John"]));
        assert!(
            mixed <= 0.30,
            "heterogeneous set should score low, got {mixed}"
        );
    }
}
