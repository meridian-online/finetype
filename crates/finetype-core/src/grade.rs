//! Per-value grade: conforms, conforms with a note, or fails.
//!
//! `validate_value_for_label` answers pass/fail. The grade adds the middle
//! case: a value the published rule accepts that a type's `validation_notes`
//! still flags, such as an upper-case UUID or a withdrawn currency code.

use crate::taxonomy::{Taxonomy, ValidationNotes};
use crate::validator::validate_value_for_label;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Why an accepted value carries a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    /// A form the source allows but does not prefer.
    NonCanonical,
    /// A value the source has withdrawn.
    Withdrawn,
    /// A real-world form the source does not define.
    Nonstandard,
}

/// A note on an accepted value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub kind: NoteKind,
    /// `"lowercase"`, `"withdrawn 2023-01"`, …
    pub detail: String,
}

/// A `validation_notes` block (or a schema property's note extensions) that
/// cannot be compiled: an unknown kind or a pattern that is not a regex.
#[derive(Error, Debug)]
#[error("invalid validation notes: {0}")]
pub struct NotesError(pub String);

/// The note rule compiled once: historic values first (case-folded, like enum
/// matching), then a miss on the canonical pattern. The single home of that
/// rule, used by `grade_value_for_label` and by the table validator.
#[derive(Debug, Clone)]
pub struct NotesMatcher {
    /// Lower-cased withdrawn value → `until`; the first listing wins.
    historic: HashMap<String, String>,
    canonical: Option<(NoteKind, String, regex::Regex)>,
}

impl NotesMatcher {
    /// Compile `notes`; fails if the canonical pattern is not a valid regex.
    pub fn compile(notes: &ValidationNotes) -> Result<Self, NotesError> {
        let mut historic = HashMap::new();
        for h in &notes.historic {
            historic
                .entry(h.value.to_lowercase())
                .or_insert_with(|| h.until.clone());
        }
        let canonical = match &notes.canonical {
            Some(c) => {
                let re = regex::Regex::new(&c.pattern)
                    .map_err(|e| NotesError(format!("canonical pattern: {e}")))?;
                Some((c.kind, c.name.clone(), re))
            }
            None => None,
        };
        Ok(Self {
            historic,
            canonical,
        })
    }

    /// Build from a schema property's `x-finetype-canonical` /
    /// `x-finetype-historic` extensions, as `emit_table_schema` writes them.
    /// `Ok(None)` when the property carries neither.
    pub fn from_schema_property(prop: &serde_json::Value) -> Result<Option<Self>, NotesError> {
        let canonical = prop.get("x-finetype-canonical");
        let historic = prop.get("x-finetype-historic");
        if canonical.is_none() && historic.is_none() {
            return Ok(None);
        }
        let notes = ValidationNotes {
            canonical: canonical
                .map(|c| serde_json::from_value(c.clone()))
                .transpose()
                .map_err(|e| NotesError(format!("x-finetype-canonical: {e}")))?,
            historic: historic
                .map(|h| serde_json::from_value(h.clone()))
                .transpose()
                .map_err(|e| NotesError(format!("x-finetype-historic: {e}")))?
                .unwrap_or_default(),
        };
        Self::compile(&notes).map(Some)
    }

    /// The note an accepted `value` carries, if any.
    pub fn note(&self, value: &str) -> Option<Note> {
        if let Some(until) = self.historic.get(&value.to_lowercase()) {
            return Some(Note {
                kind: NoteKind::Withdrawn,
                detail: format!("withdrawn {until}"),
            });
        }
        match &self.canonical {
            Some((kind, name, re)) if !re.is_match(value) => Some(Note {
                kind: *kind,
                detail: name.clone(),
            }),
            _ => None,
        }
    }
}

/// The grade of one value against one type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Grade {
    /// Not a value: empty, whitespace or a null token. Validation counts it as null.
    Skipped,
    Conforms,
    Noted(Note),
    Fails {
        constraint: String,
        message: String,
    },
}

/// Grade `value` against `label`'s published rule and notes.
///
/// Order: skipped, then the published rule (shape, check digit, membership),
/// then historic values, then the canonical form. A label with no rule, or one
/// the taxonomy does not know, has nothing to fail and conforms.
pub fn grade_value_for_label(value: &str, label: &str, tax: &Taxonomy) -> Grade {
    if value.trim().is_empty() || value.trim().eq_ignore_ascii_case("null") {
        return Grade::Skipped;
    }
    let Some(def) = tax.get(label) else {
        return Grade::Conforms;
    };
    if def.validation.is_none() {
        return Grade::Conforms;
    }

    // The published rule first; a value it rejects keeps its existing token.
    if let Ok(result) = validate_value_for_label(value, label, tax) {
        if let Some(err) = result.errors.first() {
            return Grade::Fails {
                constraint: err.check.to_string(),
                message: err.message.clone(),
            };
        }
    }
    // Check digit and set membership only judge values of the right shape.
    if let Some(verify) = def.checksum.as_deref().and_then(crate::checksum::resolve) {
        if !verify(value) {
            return Grade::Fails {
                constraint: "checksum".into(),
                message: format!("'{value}' fails its check digit"),
            };
        }
    }
    if let Some(member) = def
        .membership
        .as_deref()
        .and_then(crate::membership::resolve)
    {
        if !member(value) {
            return Grade::Fails {
                constraint: "membership".into(),
                message: format!("'{value}' is not in the published code list"),
            };
        }
    }

    let Some(notes) = &def.validation_notes else {
        return Grade::Conforms;
    };
    // The taxonomy's compiled cache when it has one, else compile for this call.
    // A pattern that does not compile is rejected by `Taxonomy::validate`.
    let owned;
    let matcher = match tax.get_notes_matcher(label) {
        Some(m) => m,
        None => match NotesMatcher::compile(notes) {
            Ok(m) => {
                owned = m;
                &owned
            }
            Err(_) => return Grade::Conforms,
        },
    };
    match matcher.note(value) {
        Some(note) => Grade::Noted(note),
        None => Grade::Conforms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const YAML: &str = r#"
identity.test.uuid:
  validation:
    type: string
    pattern: '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}$'
  validation_notes:
    canonical:
      kind: non_canonical
      name: lowercase
      pattern: '^[^A-F]*$'
finance.test.currency:
  validation:
    type: string
    enum: [EUR, HRK, USD]
  validation_notes:
    canonical:
      kind: nonstandard
      name: uppercase
      pattern: '^[A-Z]+$'
    historic:
      - { value: HRK, until: "2023-01" }
identity.test.isbnish:
  checksum: luhn
  validation:
    type: string
    pattern: '^[0-9]+$'
technology.test.code:
  membership: tld
  validation:
    type: string
    pattern: '^[a-z]+$'
"#;

    fn tax() -> Taxonomy {
        Taxonomy::from_yaml(YAML).unwrap()
    }

    #[test]
    fn grade_conforms() {
        let t = tax();
        assert_eq!(
            grade_value_for_label("abcdef01-1234", "identity.test.uuid", &t),
            Grade::Conforms
        );
        assert_eq!(
            grade_value_for_label("EUR", "finance.test.currency", &t),
            Grade::Conforms
        );
    }

    #[test]
    fn grade_noncanonical_upper_uuid() {
        assert_eq!(
            grade_value_for_label("ABCDEF01-1234", "identity.test.uuid", &tax()),
            Grade::Noted(Note {
                kind: NoteKind::NonCanonical,
                detail: "lowercase".into()
            })
        );
    }

    #[test]
    fn grade_withdrawn_hrk() {
        assert_eq!(
            grade_value_for_label("HRK", "finance.test.currency", &tax()),
            Grade::Noted(Note {
                kind: NoteKind::Withdrawn,
                detail: "withdrawn 2023-01".into()
            })
        );
    }

    #[test]
    fn grade_fails_keeps_constraint_token() {
        match grade_value_for_label("nope", "identity.test.uuid", &tax()) {
            Grade::Fails { constraint, .. } => assert_eq!(constraint, "pattern"),
            other => panic!("expected Fails, got {other:?}"),
        }
        match grade_value_for_label("GBP", "finance.test.currency", &tax()) {
            Grade::Fails { constraint, .. } => assert_eq!(constraint, "enum"),
            other => panic!("expected Fails, got {other:?}"),
        }
    }

    #[test]
    fn grade_fails_checksum_and_membership() {
        let t = tax();
        assert_eq!(
            grade_value_for_label("79927398713", "identity.test.isbnish", &t),
            Grade::Conforms
        );
        match grade_value_for_label("79927398710", "identity.test.isbnish", &t) {
            Grade::Fails { constraint, .. } => assert_eq!(constraint, "checksum"),
            other => panic!("expected Fails, got {other:?}"),
        }
        assert_eq!(
            grade_value_for_label("com", "technology.test.code", &t),
            Grade::Conforms
        );
        match grade_value_for_label("zzzzzz", "technology.test.code", &t) {
            Grade::Fails { constraint, .. } => assert_eq!(constraint, "membership"),
            other => panic!("expected Fails, got {other:?}"),
        }
    }

    #[test]
    fn grade_empty_and_null_are_skipped() {
        let t = tax();
        for v in ["", "  ", "NULL", "null"] {
            assert_eq!(
                grade_value_for_label(v, "identity.test.uuid", &t),
                Grade::Skipped,
                "{v:?}"
            );
        }
    }

    #[test]
    fn grade_historic_wins_over_canonical() {
        // HRK is upper-case (canonical) and withdrawn: the withdrawal is the note.
        assert!(matches!(
            grade_value_for_label("HRK", "finance.test.currency", &tax()),
            Grade::Noted(Note {
                kind: NoteKind::Withdrawn,
                ..
            })
        ));
        // A lower-case current code is not canonical here, so it is nonstandard.
        assert!(matches!(
            grade_value_for_label("usd", "finance.test.currency", &tax()),
            Grade::Noted(Note {
                kind: NoteKind::Nonstandard,
                ..
            })
        ));
    }

    #[test]
    fn check_rejects_historic_value_not_accepted() {
        let t = Taxonomy::from_yaml(
            r#"
finance.test.currency:
  validation: { type: string, enum: [EUR, USD] }
  validation_notes:
    historic: [ { value: HRK, until: "2023-01" } ]
"#,
        )
        .unwrap();
        assert!(t.validate().is_err());
    }

    #[test]
    fn check_rejects_bad_canonical_pattern_and_failing_sample() {
        let t = Taxonomy::from_yaml(
            r#"
a.b.c:
  validation: { type: string, pattern: '^[a-z]+$' }
  samples: ["abc", "ABC"]
  validation_notes:
    canonical: { kind: non_canonical, name: x, pattern: '(' }
"#,
        )
        .unwrap();
        let errs = t.validate().unwrap_err();
        assert!(
            errs.iter().any(|e| e.contains("canonical.pattern")),
            "{errs:?}"
        );
        assert!(errs.iter().any(|e| e.contains("sample `ABC`")), "{errs:?}");
    }

    #[test]
    fn check_accepts_well_formed_notes() {
        assert!(tax().validate().is_ok());
    }

    #[test]
    fn note_kind_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&NoteKind::NonCanonical).unwrap(),
            "\"non_canonical\""
        );
    }
}
