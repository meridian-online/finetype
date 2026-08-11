//! Fit a published `pattern` constraint to the values a column actually holds.
//!
//! A taxonomy definition's `validation.pattern` describes the *idealised* form of
//! a type. Real columns of that type carry legitimate variants the idealised form
//! rejects: a country-code column that is mostly ISO 3166-1 alpha-2 with a
//! minority of ISO 3166-2 subdivision codes, an identifier column whose codes are
//! sometimes all-digit and sometimes all-letter. Publishing the idealised pattern
//! verbatim tells a consumer running a schema validator that correct data is
//! broken — the constraint describes the type, not the table it is attached to.
//!
//! One rule, applied to every column:
//!
//! > **Widen the constraint when the observed variants are few and enumerable.
//! > Omit it otherwise.**
//!
//! The outcome differs between columns because the data differs, never because a
//! column was granted a policy of its own. Re-labelling a column so its canonical
//! pattern happens to fit is the other way to make a violation disappear, and it
//! is not this: the label is decided by inference and is left exactly as it is.
//!
//! Both arms cost something, which is why the threshold below is the whole
//! decision. Widening without a bound ends at a pattern that admits any string —
//! a field a consumer reads as a guarantee while it guarantees nothing. Omitting
//! without a bound throws away constraints that were nearly right.
//!
//! ## What "observed" means
//!
//! The values handed to [`fit_pattern_to_observed`] are the profiler's sample of
//! the column, not the column. A widened pattern therefore admits every *sampled*
//! variant; a variant that appears only in unsampled rows is not admitted and
//! still validates as a violation. Widening from a sample is the same bet the
//! label itself is made on.

use std::collections::BTreeSet;

/// How many distinct observed shapes may be folded into a widened pattern before
/// the constraint is dropped instead — the bound on "few and enumerable".
///
/// **Four.** Two real columns set the floor. A country-code column mixing alpha-2
/// with `US-DE`-shaped subdivision codes contributes one extra shape, three if
/// the data also carries three-letter (`GB-ENG`) and numeric (`IT-21`) suffixes.
/// A four-character legal-form code column, whose codes are legally allowed to be
/// all-digit or all-letter, contributes two. Four clears both with headroom, and
/// each count is pinned by a fixture in this module's tests rather than asserted
/// here.
///
/// Four also sets the ceiling: an emitted pattern is the canonical pattern plus
/// at most four anchored alternatives, which a person reading a descriptor can
/// still enumerate. At five and up the alternation stops being a rule and becomes
/// a transcription of the column — and since every added alternative admits more
/// junk, the bound is the only thing keeping "widened" from sliding into "admits
/// anything".
pub const MAX_OBSERVED_SHAPES: usize = 4;

/// Longest rendered shape, in characters, that may enter a published pattern.
///
/// A shape is one alternative in the emitted regex, so a 400-character shape
/// derived from one long free-text value would be published as a rule nobody can
/// read and nothing but that value can satisfy. A value whose shape exceeds this
/// is treated as un-renderable and forces the constraint to be omitted.
pub const MAX_SHAPE_CHARS: usize = 120;

/// What to publish for a column's `pattern` constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternFit {
    /// Every observed value satisfies the canonical pattern (or nothing was
    /// observed that contradicts it) — publish it unchanged.
    AsIs(String),
    /// Some observed values do not satisfy the canonical pattern, and their
    /// shapes are few enough to enumerate — publish the widened pattern, which
    /// contains the canonical one verbatim as its first alternative.
    Widened {
        /// The pattern to publish.
        pattern: String,
        /// The type's canonical pattern, which this one widens.
        canonical: String,
    },
    /// Some observed values do not satisfy the canonical pattern and their shapes
    /// are too many or too long to enumerate — publish no pattern.
    Omit {
        /// The type's canonical pattern, which was dropped.
        canonical: String,
    },
}

impl PatternFit {
    /// The pattern to publish, or `None` when the constraint is dropped.
    pub fn published(&self) -> Option<&str> {
        match self {
            PatternFit::AsIs(p) => Some(p),
            PatternFit::Widened { pattern, .. } => Some(pattern),
            PatternFit::Omit { .. } => None,
        }
    }

    /// The type's canonical pattern, whatever the outcome.
    pub fn canonical(&self) -> &str {
        match self {
            PatternFit::AsIs(p) => p,
            PatternFit::Widened { canonical, .. } | PatternFit::Omit { canonical } => canonical,
        }
    }

    /// `"widened"` / `"omitted"`, or `None` when the canonical pattern survived
    /// unchanged and there is nothing to report.
    pub fn outcome(&self) -> Option<&'static str> {
        match self {
            PatternFit::AsIs(_) => None,
            PatternFit::Widened { .. } => Some("widened"),
            PatternFit::Omit { .. } => Some("omitted"),
        }
    }
}

/// Reconcile a canonical `pattern` with the values observed in one column.
///
/// Values that are empty strings are skipped: a schema `pattern` does not apply
/// to a null, and an empty cell reaches here as an empty string.
///
/// Matching uses the same `jsonschema` engine as
/// [`crate::validator::CompiledValidator`], so a value this function counts as
/// satisfying the pattern is one that engine accepts — including patterns using
/// lookaround, which the `regex` crate cannot compile.
pub fn fit_pattern_to_observed(pattern: &str, values: &[String]) -> PatternFit {
    let Some(canonical_matcher) = compile(pattern) else {
        // A pattern that will not compile cannot be measured against anything.
        // Dropping it on that basis would remove a constraint for a reason that
        // has nothing to do with the data in front of us.
        return PatternFit::AsIs(pattern.to_string());
    };

    let observed = || values.iter().filter(|v| !v.is_empty());

    let mut shapes: BTreeSet<String> = BTreeSet::new();
    for value in observed() {
        if canonical_matcher(value) {
            continue;
        }
        let Some(shape) = observed_shape(value) else {
            return PatternFit::Omit {
                canonical: pattern.to_string(),
            };
        };
        shapes.insert(shape);
        if shapes.len() > MAX_OBSERVED_SHAPES {
            return PatternFit::Omit {
                canonical: pattern.to_string(),
            };
        }
    }

    if shapes.is_empty() {
        return PatternFit::AsIs(pattern.to_string());
    }

    match widen(pattern, &shapes, values) {
        Some(widened) => PatternFit::Widened {
            pattern: widened,
            canonical: pattern.to_string(),
        },
        None => PatternFit::Omit {
            canonical: pattern.to_string(),
        },
    }
}

/// Assemble the widened pattern, or `None` if the result is not publishable.
///
/// The canonical pattern stays verbatim as the first alternative, so the widened
/// pattern is a superset of it by construction and a reader can see what was
/// added. The shapes arrive ordered, so the same column profiled twice emits the
/// same string.
///
/// The alternation is built by hand out of escaped literals and character
/// classes, so **whether it compiles, and whether it admits what was observed,
/// are checked rather than assumed**. Either failure returns `None` and the
/// caller omits the constraint: a constraint that does not fit is the defect
/// being fixed, and shipping a broken one in its place would be worse than
/// shipping none.
fn widen(canonical: &str, shapes: &BTreeSet<String>, values: &[String]) -> Option<String> {
    let mut widened = String::from(canonical);
    for shape in shapes {
        widened.push_str("|^");
        widened.push_str(shape);
        widened.push('$');
    }
    let matcher = compile(&widened)?;
    values
        .iter()
        .filter(|v| !v.is_empty())
        .all(|v| matcher(v))
        .then_some(widened)
}

/// Compile `pattern` into a value predicate, or `None` if it will not compile.
fn compile(pattern: &str) -> Option<impl Fn(&str) -> bool> {
    let schema = serde_json::json!({ "type": "string", "pattern": pattern });
    let validator = jsonschema::validator_for(&schema).ok()?;
    Some(move |value: &str| validator.is_valid(&serde_json::Value::String(value.to_string())))
}

/// Render one observed value as an anchorable regex body describing its shape,
/// or `None` when it cannot be rendered.
///
/// A run of adjacent alphanumerics becomes one character class covering the
/// classes present in that run, quantified by the run's exact length: `US` is
/// `[A-Z]{2}`, `H7C4` is `[A-Z0-9]{4}`. Anything else is a literal, escaped where
/// it is a metacharacter, and it ends the run: `US-DE` is `[A-Z]{2}-[A-Z]{2}`.
///
/// Merging letters and digits inside one alphanumeric run is what keeps an
/// identifier column enumerable — the interleaving of letters and digits within a
/// code is not structure a consumer can rely on, whereas a punctuation boundary
/// is, so punctuation splits runs and letter/digit alternation does not. Without
/// the merge, `8888` / `XJHM` / `H7C4` / `2HBR` are four unrelated shapes and the
/// column is dropped for variety it does not have.
///
/// Returns `None` for a value that is not ASCII, holds a control character, or
/// renders longer than [`MAX_SHAPE_CHARS`]. Each of those forces the omit arm:
/// this synthesises plain ASCII classes only, and will not publish a Unicode
/// class it did not derive or a rule too long to read.
fn observed_shape(value: &str) -> Option<String> {
    if !value.is_ascii() {
        return None;
    }
    let bytes = value.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_alphanumeric() {
            let (mut upper, mut lower, mut digit) = (false, false, false);
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphanumeric() {
                match bytes[i] {
                    b'A'..=b'Z' => upper = true,
                    b'a'..=b'z' => lower = true,
                    _ => digit = true,
                }
                i += 1;
            }
            out.push_str(character_class(upper, lower, digit));
            let run = i - start;
            if run > 1 {
                out.push_str(&format!("{{{run}}}"));
            }
        } else {
            let c = bytes[i] as char;
            if c.is_ascii_control() {
                return None;
            }
            if is_regex_metacharacter(c) {
                out.push('\\');
            }
            out.push(c);
            i += 1;
        }
        // `out` is ASCII by construction — classes, escapes and ASCII literals —
        // so its byte length is its character count.
        if out.len() > MAX_SHAPE_CHARS {
            return None;
        }
    }
    Some(out)
}

/// The character class covering exactly the ASCII classes present in one
/// alphanumeric run. At least one of the three is always set by the caller; the
/// all-false arm is unreachable and renders the widest class rather than panic.
fn character_class(upper: bool, lower: bool, digit: bool) -> &'static str {
    match (upper, lower, digit) {
        (true, false, false) => "[A-Z]",
        (false, true, false) => "[a-z]",
        (false, false, true) => "[0-9]",
        (true, true, false) => "[A-Za-z]",
        (true, false, true) => "[A-Z0-9]",
        (false, true, true) => "[a-z0-9]",
        _ => "[A-Za-z0-9]",
    }
}

/// Is this character special outside a character class, so a literal occurrence
/// of it has to be escaped? Deliberately narrow: escaping a character that is
/// already literal (a space, `-`, `/`, `#`) is an escape sequence some engines
/// reject, and the emitted pattern has to compile everywhere it is read.
fn is_regex_metacharacter(c: char) -> bool {
    matches!(
        c,
        '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical pattern for an ISO 3166-1 alpha-2 country code, as the
    /// taxonomy carries it.
    const COUNTRY_CODE: &str = "^[A-Z]{2}$";

    /// The canonical pattern for a mixed letter-and-digit identifier: at least
    /// one letter and at least one digit, in either order.
    const ALPHANUMERIC_ID: &str = "^[A-Za-z][A-Za-z0-9 /_.#-]*[0-9][A-Za-z0-9 /_.#-]*$|^[0-9][A-Za-z0-9 /_.#-]*[A-Za-z][A-Za-z0-9 /_.#-]*$";

    fn vals(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// Does the pattern this fit publishes accept `value`?
    fn accepts(fit: &PatternFit, value: &str) -> bool {
        let published = fit.published().expect("fit publishes a pattern");
        compile(published).expect("published pattern compiles")(value)
    }

    #[test]
    fn a_column_that_satisfies_its_type_publishes_the_canonical_pattern() {
        let fit = fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US", "FR", "JP"]));
        assert_eq!(fit, PatternFit::AsIs(COUNTRY_CODE.to_string()));
        assert_eq!(fit.outcome(), None);
    }

    #[test]
    fn an_unobserved_column_publishes_the_canonical_pattern() {
        // Nothing was measured, so there is nothing to widen around and no
        // evidence the canonical pattern is wrong.
        assert_eq!(
            fit_pattern_to_observed(COUNTRY_CODE, &[]),
            PatternFit::AsIs(COUNTRY_CODE.to_string())
        );
    }

    #[test]
    fn empty_values_are_nulls_and_do_not_widen_anything() {
        assert_eq!(
            fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US", "", "FR"])),
            PatternFit::AsIs(COUNTRY_CODE.to_string())
        );
    }

    /// The motivating case: a correctly-labelled country-code column that is
    /// mostly alpha-2 with a minority of subdivision codes. One extra shape.
    #[test]
    fn a_country_code_column_with_subdivisions_widens_to_admit_them() {
        let fit = fit_pattern_to_observed(
            COUNTRY_CODE,
            &vals(&["US", "FR", "US-DE", "US-CA", "CA-ON", "JP"]),
        );
        assert_eq!(
            fit,
            PatternFit::Widened {
                pattern: "^[A-Z]{2}$|^[A-Z]{2}-[A-Z]{2}$".to_string(),
                canonical: COUNTRY_CODE.to_string(),
            }
        );
        assert_eq!(fit.outcome(), Some("widened"));

        // Everything observed is admitted …
        for observed in ["US", "FR", "US-DE", "US-CA", "CA-ON", "JP"] {
            assert!(accepts(&fit, observed), "{observed} should be admitted");
        }
        // … and the widened pattern is still a filter, not decoration.
        for junk in ["us-de", "USA", "US-DELAWARE", "U", "US-", "12-34", ""] {
            assert!(!accepts(&fit, junk), "{junk} should still be rejected");
        }
    }

    /// The second motivating case: four-character legal-form codes, legally
    /// all-digit or all-letter, against a pattern demanding both. Two extra
    /// shapes, and only because a run of letters and digits is one class.
    #[test]
    fn an_identifier_column_widens_to_its_all_digit_and_all_letter_codes() {
        let fit = fit_pattern_to_observed(
            ALPHANUMERIC_ID,
            &vals(&["8888", "XJHM", "H7C4", "2HBR", "AXSB", "9999"]),
        );
        assert_eq!(
            fit.published(),
            Some(format!("{ALPHANUMERIC_ID}|^[0-9]{{4}}$|^[A-Z]{{4}}$").as_str())
        );
        for observed in ["8888", "XJHM", "H7C4", "2HBR", "AXSB", "9999"] {
            assert!(accepts(&fit, observed), "{observed} should be admitted");
        }
        for junk in ["88888", "999", "xjhm", "XJHMQ"] {
            assert!(!accepts(&fit, junk), "{junk} should still be rejected");
        }
    }

    /// Merging letters and digits inside one alphanumeric run is what decides
    /// widen-vs-omit on an identifier column, not just how a shape reads. These
    /// six codes are one shape merged and six unmerged — past the threshold.
    #[test]
    fn one_run_of_letters_and_digits_is_one_shape() {
        let fit = fit_pattern_to_observed(
            COUNTRY_CODE,
            &vals(&["US", "H7C4", "2HBR", "AB12", "1A2B", "X999", "88AA"]),
        );
        assert_eq!(
            fit,
            PatternFit::Widened {
                pattern: "^[A-Z]{2}$|^[A-Z0-9]{4}$".to_string(),
                canonical: COUNTRY_CODE.to_string(),
            }
        );
    }

    /// The threshold itself: exactly `MAX_OBSERVED_SHAPES` distinct shapes still
    /// widens, one more drops the constraint. The fixture is mixed-format by
    /// construction — each value contributes a shape no other value has.
    #[test]
    fn the_threshold_is_the_line_between_widening_and_omitting() {
        assert_eq!(MAX_OBSERVED_SHAPES, 4);
        let at_threshold = vals(&["US", "US-DE", "US-DEL", "US-1", "US-12-3"]);
        assert_eq!(at_threshold.len() - 1, MAX_OBSERVED_SHAPES);
        let fit = fit_pattern_to_observed(COUNTRY_CODE, &at_threshold);
        assert_eq!(fit.outcome(), Some("widened"));
        for observed in at_threshold.iter() {
            assert!(accepts(&fit, observed), "{observed} should be admitted");
        }

        let mut over_threshold = at_threshold.clone();
        over_threshold.push("US-DELA".to_string());
        assert_eq!(
            fit_pattern_to_observed(COUNTRY_CODE, &over_threshold),
            PatternFit::Omit {
                canonical: COUNTRY_CODE.to_string()
            }
        );
    }

    #[test]
    fn a_free_text_column_drops_the_constraint_rather_than_transcribing_itself() {
        let fit = fit_pattern_to_observed(
            COUNTRY_CODE,
            &vals(&[
                "United States of America",
                "France",
                "Japan",
                "The Netherlands",
                "Cote d'Ivoire",
            ]),
        );
        assert_eq!(fit.published(), None);
        assert_eq!(fit.outcome(), Some("omitted"));
        assert_eq!(fit.canonical(), COUNTRY_CODE);
    }

    #[test]
    fn a_non_ascii_value_drops_the_constraint() {
        // One shape by count, but not one this renders: no Unicode class is
        // synthesised, so the omit arm takes it.
        assert_eq!(observed_shape("CÔ"), None);
        assert_eq!(
            fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US", "FR", "CÔ"])),
            PatternFit::Omit {
                canonical: COUNTRY_CODE.to_string()
            }
        );
    }

    #[test]
    fn an_unreadably_long_shape_drops_the_constraint() {
        // A single value, so one shape by count — rejected on rendered length.
        // Each `a` renders as a five-character class and each `-` as itself, so
        // the shape is six times as long as the value.
        let long = "a-".repeat(MAX_SHAPE_CHARS);
        assert!(observed_shape(&long).is_none());
        assert_eq!(
            fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US", &long])),
            PatternFit::Omit {
                canonical: COUNTRY_CODE.to_string()
            }
        );
    }

    #[test]
    fn a_pattern_that_will_not_compile_is_published_unchanged() {
        let broken = "^[A-Z{2}$";
        assert!(compile(broken).is_none());
        assert_eq!(
            fit_pattern_to_observed(broken, &vals(&["US", "US-DE"])),
            PatternFit::AsIs(broken.to_string())
        );
    }

    #[test]
    fn a_literal_metacharacter_in_a_value_is_escaped_into_the_shape() {
        // `.` and `+` are literal in the data and must not become regex syntax.
        let fit = fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US", "A.B+C"]));
        assert_eq!(fit.published(), Some("^[A-Z]{2}$|^[A-Z]\\.[A-Z]\\+[A-Z]$"));
        assert!(accepts(&fit, "A.B+C"));
        assert!(!accepts(&fit, "AXBYC"));
    }

    fn shape_set(shapes: &[&str]) -> BTreeSet<String> {
        shapes.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_widened_pattern_that_will_not_compile_is_refused() {
        // What a renderer bug would produce: an unclosed character class. It has
        // to be caught here rather than published, so the caller omits instead.
        assert_eq!(
            widen(COUNTRY_CODE, &shape_set(&["[A-Z"]), &vals(&["US"])),
            None
        );
    }

    #[test]
    fn a_widened_pattern_that_rejects_an_observed_value_is_refused() {
        // The shape does not describe the value it came from — a widened pattern
        // that still fails the data is the defect, not the fix.
        assert_eq!(
            widen(
                COUNTRY_CODE,
                &shape_set(&["[0-9]{2}"]),
                &vals(&["US", "us"])
            ),
            None
        );
        // …and the same call with a shape that does describe it is published.
        assert_eq!(
            widen(
                COUNTRY_CODE,
                &shape_set(&["[a-z]{2}"]),
                &vals(&["US", "us"])
            ),
            Some("^[A-Z]{2}$|^[a-z]{2}$".to_string())
        );
    }

    #[test]
    fn shapes_are_ordered_so_the_same_column_emits_the_same_pattern() {
        // Same values in two orders, and the exact string, so this pins the
        // ordering rather than only that two calls agree.
        let expected = "^[A-Z]{2}$|^[0-9]{4}$|^[A-Z]{2}-[A-Z]{2}$|^[a-z]{2}$";
        let forward = fit_pattern_to_observed(COUNTRY_CODE, &vals(&["US-DE", "1234", "ab"]));
        let backward = fit_pattern_to_observed(COUNTRY_CODE, &vals(&["ab", "1234", "US-DE"]));
        assert_eq!(forward.published(), Some(expected));
        assert_eq!(forward, backward);
    }

    #[test]
    fn shape_rendering_covers_each_class_and_the_run_merge() {
        assert_eq!(observed_shape("US").as_deref(), Some("[A-Z]{2}"));
        assert_eq!(observed_shape("us").as_deref(), Some("[a-z]{2}"));
        assert_eq!(observed_shape("42").as_deref(), Some("[0-9]{2}"));
        assert_eq!(observed_shape("Us").as_deref(), Some("[A-Za-z]{2}"));
        assert_eq!(observed_shape("U2").as_deref(), Some("[A-Z0-9]{2}"));
        assert_eq!(observed_shape("u2").as_deref(), Some("[a-z0-9]{2}"));
        assert_eq!(observed_shape("Us2").as_deref(), Some("[A-Za-z0-9]{3}"));
        assert_eq!(observed_shape("X").as_deref(), Some("[A-Z]"));
        assert_eq!(
            observed_shape("SKU-12345").as_deref(),
            Some("[A-Z]{3}-[0-9]{5}")
        );
    }
}
