//! Per-type discrimination detectors.
//!
//! A type's published rule (its taxonomy `validation`) is written to agree with
//! its defining source, and for some types that rule confirms a large share of
//! random input. Where it does, Sharpen and the validation veto need a stricter
//! per-value test to decide whether a column really is the type. A detector is
//! that test; it is internal and never published.
//!
//! A label with no detector falls through to its published rule.

use crate::taxonomy::{Taxonomy, Validation};
use crate::validator::{resolve_label_validator, validate_value, CompiledValidator};

/// The detector for `label`, when one exists. Stricter than the published rule.
pub fn detector_for(label: &str) -> Option<fn(&str) -> bool> {
    match label {
        "identity.person.username" => Some(is_username_shape),
        "identity.medical.npi" => Some(is_issued_npi),
        "technology.identifier.tsid" => Some(is_tsid_shape),
        "container.object.csv" => Some(is_csv_discrimination_shape),
        "container.object.yaml" => Some(is_yaml_discrimination_shape),
        "container.object.xml" => Some(is_xml_discrimination_shape),
        "container.key_value.query_string" => Some(is_query_string_discrimination_shape),
        "geography.location.state_code" => Some(is_state_code_discrimination_shape),
        "geography.format.wkt" => Some(is_wkt_discrimination_shape),
        _ => None,
    }
}

/// Whether `value` passes: the detector when there is one, else the compiled
/// published rule. The one per-value form every discrimination site shares, so a
/// site cannot grow its own reading of "detector else rule".
pub fn passes_detector_or_rule(
    detector: Option<fn(&str) -> bool>,
    rule: &CompiledValidator,
    value: &str,
) -> bool {
    detector.map_or_else(|| rule.is_valid(value), |d| d(value))
}

/// As [`passes_detector_or_rule`], for a site whose taxonomy has no compiled
/// cache: the published rule is compiled per call. A rule that fails to compile
/// counts the value as passing, as `validate_value`'s callers have always read it.
pub fn passes_detector_or_uncompiled_rule(
    detector: Option<fn(&str) -> bool>,
    rule: &Validation,
    value: &str,
) -> bool {
    detector.map_or_else(
        || {
            validate_value(value, rule)
                .map(|r| r.is_valid)
                .unwrap_or(true)
        },
        |d| d(value),
    )
}

/// `username`: 3 to 32 characters of `[a-zA-Z0-9_.-]`, optionally followed by a
/// single `$`. The published rule follows useradd(8) (up to 256 characters),
/// which would confirm 36-character UUIDs and 40/64-character hashes; this
/// keeps the shape analysts meet in web-service names.
fn is_username_shape(value: &str) -> bool {
    let body = value.strip_suffix('$').unwrap_or(value);
    let n = body.chars().count();
    (3..=32).contains(&n)
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Whether `value` is one RFC 4180 record: `field(,field)*`, where a field is
/// either quoted (any text, a doubled quote standing for one) or unquoted
/// (no comma, quote or line break). The same set as the published rule.
fn is_csv_record(value: &str) -> bool {
    let b = value.as_bytes();
    let mut i = 0;
    loop {
        if b.get(i) == Some(&b'"') {
            i += 1;
            loop {
                match b.get(i) {
                    None => return false,
                    Some(b'"') if b.get(i + 1) == Some(&b'"') => i += 2,
                    Some(b'"') => break,
                    Some(_) => i += 1,
                }
            }
            i += 1;
        } else {
            while let Some(c) = b.get(i) {
                match c {
                    b',' => break,
                    b'"' | b'\r' | b'\n' => return false,
                    _ => i += 1,
                }
            }
        }
        match b.get(i) {
            None => return true,
            Some(b',') => i += 1,
            Some(_) => return false,
        }
    }
}

/// `csv`: the published rule (an RFC 4180 record, whose fields may be empty)
/// and either every field non-empty (the shape the type was first defined by)
/// or at least one quoted field. The published rule alone would confirm most
/// random text with a comma in it, since an empty field is allowed; this keeps
/// column discrimination at the rate it had.
fn is_csv_discrimination_shape(value: &str) -> bool {
    is_csv_record(value) && (value.contains('"') || value.split(',').all(|f| !f.is_empty()))
}

/// The published `container.object.xml` rule. A copy, because the library
/// cannot read `labels/` when built from the crates.io package; the test
/// `xml_rule_copy_matches_the_taxonomy` keeps it equal to the taxonomy's.
const XML_RULE: &str = "^(?:<\\?xml[^>]*\\?>)?(?:<!--[\\s\\S]*?-->|<\\?[\\s\\S]*?\\?>|\\s+)*(?:<!DOCTYPE\\s[^\\[>]*(?:\\[[\\s\\S]*?\\]\\s*)?>(?:<!--[\\s\\S]*?-->|<\\?[\\s\\S]*?\\?>|\\s+)*)?(?:<[A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}][A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}\\-.0-9\u{B7}\u{300}-\u{36F}\u{203F}-\u{2040}]*(?:\\s[^<>]*)?/>|<[A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}][A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}\\-.0-9\u{B7}\u{300}-\u{36F}\u{203F}-\u{2040}]*(?:\\s[^<>]*)?>[\\s\\S]*</[A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}][A-Za-z_:\u{C0}-\u{D6}\u{D8}-\u{F6}\u{F8}-\u{2FF}\u{370}-\u{37D}\u{37F}-\u{1FFF}\u{200C}\u{200D}\u{2070}-\u{218F}\u{2C00}-\u{2FEF}\u{3001}-\u{D7FF}\u{F900}-\u{FDCF}\u{FDF0}-\u{FFFD}\\-.0-9\u{B7}\u{300}-\u{36F}\u{203F}-\u{2040}]*\\s*>)(?:<!--[\\s\\S]*?-->|<\\?[\\s\\S]*?\\?>|\\s+)*$";

/// The shape the type had before the rule followed XML 1.0: single line,
/// `<...>...</...>`.
const XML_OLD_SHAPE: &str = "^<.*>.*</.*>$";

fn xml_regexes() -> Option<&'static (regex::Regex, regex::Regex)> {
    static RULES: std::sync::OnceLock<Option<(regex::Regex, regex::Regex)>> =
        std::sync::OnceLock::new();
    RULES
        .get_or_init(|| {
            Some((
                regex::Regex::new(XML_RULE).ok()?,
                regex::Regex::new(XML_OLD_SHAPE).ok()?,
            ))
        })
        .as_ref()
}

/// The name of the root element: the first tag after any declaration,
/// processing instructions, comments and DOCTYPE (with its internal subset).
fn xml_root_name(value: &str) -> Option<&str> {
    let mut rest = value.trim_start();
    loop {
        let end = if rest.starts_with("<?") {
            rest.find("?>")? + 2
        } else if rest.starts_with("<!--") {
            rest.find("-->")? + 3
        } else if rest.starts_with("<!") {
            // A declaration ends at the first `>` outside an internal subset.
            let mut depth = 0usize;
            let mut end = None;
            for (i, c) in rest.char_indices() {
                match c {
                    '[' => depth += 1,
                    ']' => depth = depth.saturating_sub(1),
                    '>' if depth == 0 => {
                        end = Some(i + 1);
                        break;
                    }
                    _ => {}
                }
            }
            end?
        } else {
            break;
        };
        rest = rest[end..].trim_start();
    }
    let rest = rest.strip_prefix('<')?;
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(rest.len());
    Some(&rest[..end])
}

/// `xml`: the published rule, the single-line `<...>...</...>` shape the type
/// had first, and neither an `html` root element (in any case) nor a
/// `<!DOCTYPE html` declaration. The published rule is shaped by XML 1.0, so it
/// also accepts multi-line text, XHTML and any `<html>` page or HTML fragment;
/// the type is a ceded leaf, so without this a column of HTML could be
/// relabelled xml. Those forms conform but are not counted for discrimination.
fn is_xml_discrimination_shape(value: &str) -> bool {
    let Some((rule, old_shape)) = xml_regexes() else {
        return false;
    };
    rule.is_match(value)
        && old_shape.is_match(value)
        && !value
            .get(..value.len().min(2048))
            .unwrap_or(value)
            .to_ascii_lowercase()
            .contains("<!doctype html")
        && !xml_root_name(value).is_some_and(|n| n.eq_ignore_ascii_case("html"))
}

#[cfg(test)]
mod xml_rule_copy {
    #[test]
    fn xml_rule_copy_matches_the_taxonomy() {
        let doc: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../../labels/definitions_container.yaml"))
                .expect("parse container definitions");
        let published = doc["container.object.xml"]["validation"]["pattern"]
            .as_str()
            .expect("xml pattern");
        assert_eq!(super::XML_RULE, published);
    }
}

/// `yaml`: a first line of the form `key:` with a key of letters, digits and
/// underscores, the shape the type was first defined by. Every such value also
/// passes the published rule, which additionally takes `---`-opened and
/// comment-led documents and multi-line sequences; those conform but are not
/// counted for discrimination.
fn is_yaml_discrimination_shape(value: &str) -> bool {
    let mut chars = value.chars();
    if !matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_') {
        return false;
    }
    chars
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_'))
        .is_some_and(|c| c == ':')
}

/// `query_string`: only `name=value` pairs joined by `&`, every name non-empty
/// and every sequence present, the shape the type was first defined by. Every
/// such value also passes the published rule, which additionally takes bare
/// names and empty sequences; those conform but are not counted for
/// discrimination.
fn is_query_string_discrimination_shape(value: &str) -> bool {
    value.split('&').all(|seq| {
        seq.split_once('=')
            .is_some_and(|(name, _)| !name.is_empty())
    })
}

/// The three-letter `EN_AU` state and territory codes of `state_code`'s
/// `validation_by_locale`. A copy, because the library cannot read `labels/`
/// when built from the crates.io package; the test
/// `state_code_au_codes_match_the_taxonomy` keeps it equal to the taxonomy's.
const STATE_CODE_AU_THREE_LETTER: [&str; 5] = ["ACT", "NSW", "QLD", "TAS", "VIC"];

/// `state_code`: two uppercase letters, or one of the `EN_AU` three-letter codes.
/// The published rule takes any two or three uppercase letters (ISO 3166-2:AU
/// codes run to three), which would confirm three-letter airport and currency
/// codes; this keeps column discrimination at the shapes state codes take. Every
/// such value also passes the published rule.
fn is_state_code_discrimination_shape(value: &str) -> bool {
    let b = value.as_bytes();
    (b.len() == 2 && b.iter().all(u8::is_ascii_uppercase))
        || STATE_CODE_AU_THREE_LETTER.contains(&value)
}

#[cfg(test)]
mod state_code_copy {
    #[test]
    fn state_code_au_codes_match_the_taxonomy() {
        let doc: serde_yaml::Value =
            serde_yaml::from_str(include_str!("../../../labels/definitions_geography.yaml"))
                .expect("parse geography definitions");
        let au = doc["geography.location.state_code"]["validation_by_locale"]["EN_AU"]["enum"]
            .as_sequence()
            .expect("EN_AU enum");
        let mut three: Vec<&str> = au
            .iter()
            .filter_map(|c| c.as_str())
            .filter(|c| c.len() == 3)
            .collect();
        three.sort_unstable();
        assert_eq!(three, super::STATE_CODE_AU_THREE_LETTER);
    }
}

/// What `wkt` counts for discrimination: a geometry keyword in any letter case,
/// an optional `Z`/`M`/`ZM`, then `EMPTY` or a `(` followed, somewhere inside,
/// by a coordinate pair (a number, white space, a number).
const WKT_DETECTOR_SHAPE: &str = "(?is)^(?:point|linestring|polygon|multipoint|multilinestring|multipolygon|geometrycollection|triangle|polyhedralsurface|tin)\\s*(?:zm|z|m)?\\s*(?:empty|\\(.*?[-+]?\\d+(?:\\.\\d+)?\\s+[-+]?[.\\d])";

/// `wkt`: a keyword in any case followed by `EMPTY` or a body holding at least one
/// coordinate pair. The published rule stops at the opening parenthesis, so
/// `Triangle (1999)` and `Point (of sale)` conform; with no coordinate pair they
/// do not count for discrimination or for a ceded recovery. Every value counted
/// here also passes the published rule.
fn is_wkt_discrimination_shape(value: &str) -> bool {
    static SHAPE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    SHAPE
        .get_or_init(|| regex::Regex::new(WKT_DETECTOR_SHAPE).expect("wkt detector shape"))
        .is_match(value)
}

/// `tsid`: the published rule (13 Crockford base32 characters, first character
/// at most F) and at least one letter. The published rule is source-shaped, so it
/// accepts every 13-digit number (the first character may be any hex digit),
/// which covers every 13-digit Unix-millisecond timestamp, 2001 to 2286; a real
/// TSID is all digits about one time in a million, so requiring a letter keeps
/// column discrimination off those.
fn is_tsid_shape(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 13
        && b[0].is_ascii_hexdigit()
        && b[1..].iter().all(|c| {
            c.is_ascii_digit()
                || (c.is_ascii_alphabetic()
                    && !matches!(c.to_ascii_uppercase(), b'I' | b'L' | b'O' | b'U'))
        })
        && b.iter().any(u8::is_ascii_alphabetic)
}

/// `npi`: a number in the issued range (first digit 1 or 2) whose check digit
/// passes. The published rule follows the CMS check-digit document, which leaves
/// other first digits reserved rather than invalid, so it accepts any ten digits;
/// this keeps column discrimination at the range NPIs are issued from.
fn is_issued_npi(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() == 10
        && matches!(b[0], b'1' | b'2')
        && b.iter().all(u8::is_ascii_digit)
        && crate::checksum::npi(value)
}

/// Whether `value` passes for `label`: the detector when there is one, else the
/// published rule. A label with no published rule (or an unknown one) never
/// passes, detector or not, matching the veto. The rule is resolved from the taxonomy's compiled cache or compiled on a
/// miss (the same resolution `resolve_label_validator` gives `ceded_leaf_recovery`).
///
/// The profile call sites keep their own validator lookup and call
/// `detector_for` directly, so a failed lookup behaves at each as it did before;
/// this form serves callers with no lookup of their own.
pub fn value_passes_for_discrimination(value: &str, label: &str, tax: &Taxonomy) -> bool {
    let Some(rule) = resolve_label_validator(label, tax) else {
        return false;
    };
    passes_detector_or_rule(detector_for(label), &rule, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn taxonomy(compiled: bool) -> Taxonomy {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.pop();
        let mut tax = Taxonomy::from_directory(dir.join("labels")).expect("load labels");
        if compiled {
            tax.compile_validators();
        }
        tax
    }

    #[test]
    fn username_detector_keeps_the_old_shape_and_allows_trailing_dollar() {
        let detect = detector_for("identity.person.username").expect("username detector");
        for ok in ["alice", "bob.smith", "svc$", "abc", &"a".repeat(32)] {
            assert!(detect(ok), "{ok}");
        }
        for bad in [
            "ab",
            "ab$",
            "a$b",
            "$alice",
            "al ice",
            "123e4567-e89b-12d3-a456-426614174000",
            &"a".repeat(40),
            &"a".repeat(33),
        ] {
            assert!(!detect(bad), "{bad}");
        }
    }

    /// The locale branches of the attractor sites consult only the published
    /// rule, so a detector label must never reach them.
    #[test]
    fn detector_labels_have_no_locale_rules_the_detector_could_contradict() {
        let tax = taxonomy(false);
        let labels = [
            "identity.person.username",
            "identity.medical.npi",
            "technology.identifier.tsid",
            "container.object.csv",
            "container.object.yaml",
            "container.key_value.query_string",
            "container.object.xml",
            "geography.location.state_code",
            "geography.format.wkt",
        ];
        // `state_code` has locale rules. Its locale enums hold only shapes its
        // detector counts (two letters, or an EN_AU three-letter code), so the
        // locale branches, which consult the enums alone, cannot disagree.
        let with_locale_rules = ["geography.location.state_code"];
        for label in labels {
            assert!(detector_for(label).is_some(), "{label} has a detector");
            let def = tax.get(label).expect("label in taxonomy");
            if with_locale_rules.contains(&label) {
                let by_locale = def.validation_by_locale.as_ref().expect("locale rules");
                for (locale, rule) in by_locale {
                    assert!(
                        rule.pattern.is_none(),
                        "{label} {locale} has a pattern the detector could contradict"
                    );
                    for code in rule.enum_values.as_ref().expect("locale enum") {
                        assert!(
                            detector_for(label).expect("detector")(code),
                            "{label} {locale} code {code} not counted by the detector"
                        );
                    }
                }
                continue;
            }
            assert!(
                def.validation_by_locale.is_none()
                    || def
                        .validation_by_locale
                        .as_ref()
                        .is_some_and(|m| m.is_empty()),
                "{label} has validation_by_locale"
            );
            // The fast path reads `detector_for` (xml and wkt are fast-path
            // leaves), so only the locale branches need the check above.
        }
        // Every registered detector is covered by the list above.
        for (id, _) in tax.definitions() {
            if detector_for(id).is_some() {
                assert!(labels.contains(&id.as_str()), "{id} not listed");
            }
        }
    }

    #[test]
    fn tsid_detector_is_the_published_rule_plus_a_letter() {
        let tax = taxonomy(true);
        let detect = detector_for("technology.identifier.tsid").expect("tsid detector");
        for ok in ["0JMVQ049NFN6K", "0jmvq049nfn6k", "000000000000A"] {
            assert!(detect(ok), "{ok}");
        }
        for bad in [
            "1700000000000",
            "0000000000000",
            "GAWE5HZP3SKTK",
            "0AWE5HZP3SKTU",
            "0JMVQ049NFN6",
            "0190a1b2c3d4e5f6a7b8c9d0e1f2a3b4",
        ] {
            assert!(!detect(bad), "{bad}");
        }
        // A detector never accepts what the published rule rejects.
        let rule = resolve_label_validator("technology.identifier.tsid", &tax).expect("rule");
        for v in [
            "GAWE5HZP3SKTK",
            "0AWE5HZP3SKTU",
            "0JMVQ049NFN6",
            "0JMVQ049NFN6KK",
        ] {
            assert!(!detect(v) && !rule.is_valid(v), "{v}");
        }
    }

    #[test]
    fn no_detector_means_published_rule() {
        let label = "representation.identifier.uuid";
        assert!(detector_for(label).is_none());
        let samples = [
            "123e4567-e89b-12d3-a456-426614174000",
            "00000000-0000-0000-0000-000000000000",
            "123e4567e89b12d3a456426614174000",
            "not-a-uuid",
            "",
        ];
        for compiled in [true, false] {
            let tax = taxonomy(compiled);
            let rule = tax
                .get(label)
                .and_then(|d| d.validation.as_ref())
                .map(|v| crate::validator::CompiledValidator::new(v).unwrap())
                .expect("uuid has a validation");
            for s in samples {
                assert_eq!(
                    value_passes_for_discrimination(s, label, &tax),
                    rule.is_valid(s),
                    "{s:?} (compiled cache: {compiled})"
                );
            }
        }
    }
}
