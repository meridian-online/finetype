//! Type nominations: what a column IS, declared by the person who built it.
//!
//! A nomination is taken as given. FineType does not check it against the data
//! and does not let inference overturn it — the drift contract is `ft_validate`
//! against a schema, and it stays where it is. What a nomination buys is that
//! the label is *correct* rather than *guessed*, so everything the taxonomy
//! states about that label — its Frictionless type, its format, its length and
//! range bounds — is published rather than approximated.
//!
//! The file is JSON, conventionally `nominations.finetype.json`, though any
//! path is accepted:
//!
//! ```json
//! {
//!   "version": 1,
//!   "resources": {
//!     "naics": {
//!       "description": { "label": "representation.text.plain_text",
//!                        "why": "free-form NAICS descriptions" }
//!     }
//!   }
//! }
//! ```
//!
//! `resources` is required and keys on the **file stem** of each profiled
//! input. `version` is optional and defaults to `1`. `label` is required;
//! `why` is free text FineType records and never interprets.
//!
//! **Every departure from that shape is refused, by design.** A nomination
//! that is silently dropped becomes an inference, and the descriptor then ships
//! a guess under a heading that says somebody declared it — which is the exact
//! confusion the marker exists to prevent. So an unknown key, a missing
//! `label`, a label the taxonomy does not carry, a declared column the file
//! does not have and a declared stem no input matches are all errors that stop
//! the run before any profiling happens.

use anyhow::{bail, Result};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// The format version this build understands.
pub const SUPPORTED_VERSION: u64 = 1;

/// One declared column type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nomination {
    /// The taxonomy label this column is declared to be.
    pub label: String,
    /// Free text from the author. Recorded, never interpreted.
    pub why: Option<String>,
}

/// A parsed nominations file: resource stem → column name → nomination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Nominations {
    version: u64,
    resources: BTreeMap<String, BTreeMap<String, Nomination>>,
}

impl Nominations {
    /// The declared format version (`1` when the file omitted it).
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Whether any column at all is declared.
    pub fn is_empty(&self) -> bool {
        self.resources.values().all(|cols| cols.is_empty())
    }

    /// The nomination for one column of one resource, if there is one.
    ///
    /// A stem with no entry is profiled entirely by inference — that is the
    /// `--files` batch case, where a nominations file naming two of five
    /// inputs is a normal thing to write.
    pub fn get(&self, stem: &str, column: &str) -> Option<&Nomination> {
        self.resources.get(stem)?.get(column)
    }

    /// Every stem the file declares, sorted.
    pub fn stems(&self) -> impl Iterator<Item = &str> {
        self.resources.keys().map(String::as_str)
    }

    /// Read and validate a nominations file.
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("could not read --nominations {:?}: {}", path, e))?;
        Self::parse(&text, &path.display().to_string())
    }

    /// Validate and parse nominations JSON. `origin` names the file in errors.
    pub fn parse(text: &str, origin: &str) -> Result<Self> {
        let root: Value = serde_json::from_str(text)
            .map_err(|e| anyhow::anyhow!("--nominations {}: not valid JSON: {}", origin, e))?;

        let Some(obj) = root.as_object() else {
            bail!("--nominations {}: the top level of the file must be a JSON object with a `resources` key, found {}", origin, kind_of(&root));
        };

        for key in obj.keys() {
            if key != "version" && key != "resources" {
                bail!(
                    "--nominations {}: unknown key `{}` at `$.{}` — the file takes `version` and `resources` and nothing else",
                    origin, key, key
                );
            }
        }

        let version = match obj.get("version") {
            None => SUPPORTED_VERSION,
            Some(v) => {
                let Some(n) = v.as_u64() else {
                    bail!(
                        "--nominations {}: `$.version` must be an integer, found {}",
                        origin,
                        kind_of(v)
                    );
                };
                if n != SUPPORTED_VERSION {
                    bail!(
                        "--nominations {}: `$.version` is {}, and this build understands version {} only",
                        origin, n, SUPPORTED_VERSION
                    );
                }
                n
            }
        };

        let Some(resources) = obj.get("resources") else {
            bail!(
                "--nominations {}: required key `resources` is missing at `$.resources`",
                origin
            );
        };
        let Some(resources) = resources.as_object() else {
            bail!(
                "--nominations {}: `$.resources` must be an object keyed on resource stem, found {}",
                origin,
                kind_of(resources)
            );
        };

        let mut parsed: BTreeMap<String, BTreeMap<String, Nomination>> = BTreeMap::new();
        for (stem, columns) in resources {
            let Some(columns) = columns.as_object() else {
                bail!(
                    "--nominations {}: `$.resources[{:?}]` must be an object keyed on column name, found {}",
                    origin, stem, kind_of(columns)
                );
            };
            let mut stem_map = BTreeMap::new();
            for (column, spec) in columns {
                let at = format!("$.resources[{:?}][{:?}]", stem, column);
                let Some(spec) = spec.as_object() else {
                    bail!(
                        "--nominations {}: `{}` must be an object with a `label`, found {}",
                        origin,
                        at,
                        kind_of(spec)
                    );
                };
                for key in spec.keys() {
                    if key != "label" && key != "why" {
                        bail!(
                            "--nominations {}: unknown key `{}` at `{}.{}` — a nomination takes `label` and `why` and nothing else",
                            origin, key, at, key
                        );
                    }
                }
                let Some(label) = spec.get("label") else {
                    bail!(
                        "--nominations {}: `{}` is missing the required key `label` (`{}.label`)",
                        origin,
                        at,
                        at
                    );
                };
                let Some(label) = label.as_str() else {
                    bail!(
                        "--nominations {}: `{}.label` must be a string, found {}",
                        origin,
                        at,
                        kind_of(label)
                    );
                };
                let why = match spec.get("why") {
                    None => None,
                    Some(w) => {
                        let Some(w) = w.as_str() else {
                            bail!(
                                "--nominations {}: `{}.why` must be a string, found {}",
                                origin,
                                at,
                                kind_of(w)
                            );
                        };
                        Some(w.to_string())
                    }
                };
                stem_map.insert(
                    column.clone(),
                    Nomination {
                        label: label.to_string(),
                        why,
                    },
                );
            }
            parsed.insert(stem.clone(), stem_map);
        }

        Ok(Nominations {
            version,
            resources: parsed,
        })
    }

    /// Refuse a nominated label the taxonomy does not carry, and one whose
    /// declared Frictionless type the Data Package v2 profile does not admit.
    ///
    /// The asymmetry with inference is deliberate. An *inferred* `list` still
    /// emits: that is a mid-run model answer about data the caller cannot
    /// change, and aborting the profile would leave them nothing. A *nominated*
    /// `list` is a deliberate declaration made before any work starts, so it is
    /// refused at the point it is read, while the person is still holding the
    /// file they can fix.
    pub fn check_against_taxonomy(
        &self,
        taxonomy: &finetype_core::Taxonomy,
        origin: &str,
    ) -> Result<()> {
        use finetype_core::frictionless_vocabulary::is_profile_field_type;

        for (stem, columns) in &self.resources {
            for (column, nomination) in columns {
                let Some(def) = taxonomy.get(&nomination.label) else {
                    bail!(
                        "--nominations {}: column `{}` of `{}` is nominated as `{}`, which is not a label in the taxonomy",
                        origin, column, stem, nomination.label
                    );
                };
                let ftype = def
                    .frictionless
                    .as_ref()
                    .map(|f| f.ftype.as_str())
                    .unwrap_or("string");
                if !is_profile_field_type(ftype) {
                    bail!(
                        "--nominations {}: column `{}` of `{}` is nominated as `{}`, whose Frictionless type `{}` is not one the Data Package v2 profile admits — its field object is a fifteen-branch `oneOf` and `{}` is in none of them",
                        origin, column, stem, nomination.label, ftype, ftype
                    );
                }
            }
        }
        Ok(())
    }

    /// Refuse a declared stem no input file in this run matches.
    ///
    /// The reverse is not an error: an input with no stem in the file is
    /// profiled by inference, with no nomination and no warning.
    pub fn check_stems(&self, stems: &[String], origin: &str) -> Result<()> {
        for declared in self.resources.keys() {
            if !stems.iter().any(|s| s == declared) {
                bail!(
                    "--nominations {}: `{}` is declared but no file in this run has that stem (this run profiles: {})",
                    origin, declared, if stems.is_empty() { "nothing".to_string() } else { stems.join(", ") }
                );
            }
        }
        Ok(())
    }

    /// Refuse a declared column the file profiled under `stem` does not have.
    ///
    /// The same reasoning a curated key naming an absent column already gets:
    /// a rename silently drops the nomination, and the descriptor then ships
    /// the inferred label with no sign anything was ever asked for.
    pub fn check_columns(
        &self,
        stem: &str,
        headers: &[String],
        file: &Path,
        origin: &str,
    ) -> Result<()> {
        let Some(columns) = self.resources.get(stem) else {
            return Ok(());
        };
        for column in columns.keys() {
            if !headers.iter().any(|h| h == column) {
                bail!(
                    "--nominations {}: column `{}` is declared for `{}` but {:?} has no such column (its columns are: {})",
                    origin, column, stem, file, headers.join(", ")
                );
            }
        }
        Ok(())
    }
}

/// The JSON type name to print when a value is the wrong shape.
fn kind_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{
      "version": 1,
      "resources": {
        "naics": {
          "description": { "label": "representation.text.plain_text", "why": "prose" },
          "corpus": { "label": "representation.text.plain_text" }
        }
      }
    }"#;

    fn err(text: &str) -> String {
        Nominations::parse(text, "n.json")
            .expect_err("expected a refusal")
            .to_string()
    }

    #[test]
    fn a_well_formed_file_parses_and_version_defaults_to_one() {
        let n = Nominations::parse(GOOD, "n.json").expect("parses");
        assert_eq!(n.version(), 1);
        assert_eq!(
            n.get("naics", "description").map(|d| d.label.as_str()),
            Some("representation.text.plain_text")
        );
        assert_eq!(
            n.get("naics", "description").and_then(|d| d.why.as_deref()),
            Some("prose")
        );
        assert_eq!(
            n.get("naics", "corpus").and_then(|d| d.why.as_deref()),
            None
        );
        assert_eq!(n.get("naics", "absent"), None);
        assert_eq!(n.get("other", "corpus"), None);

        let no_version = r#"{"resources": {"s": {"c": {"label": "x"}}}}"#;
        assert_eq!(
            Nominations::parse(no_version, "n.json")
                .expect("parses")
                .version(),
            1
        );
    }

    #[test]
    fn an_unknown_key_is_named_with_its_json_path_at_every_level() {
        // Top level.
        let e = err(r#"{"resources": {}, "resource": {}}"#);
        assert!(e.contains("resource"), "{e}");
        assert!(e.contains("$.resource"), "{e}");

        // Inside a nomination — the typo that would otherwise become an
        // inference, since `labell` leaves no `label` to read.
        let e = err(r#"{"resources": {"s": {"c": {"label": "x", "lable": "y"}}}}"#);
        assert!(e.contains("lable"), "{e}");
        assert!(e.contains(r#"$.resources["s"]["c"].lable"#), "{e}");
    }

    #[test]
    fn a_column_without_a_label_is_named_with_its_path_and_the_missing_key() {
        let e = err(r#"{"resources": {"s": {"c": {"why": "no label here"}}}}"#);
        assert!(e.contains(r#"$.resources["s"]["c"]"#), "{e}");
        assert!(e.contains("label"), "{e}");
    }

    #[test]
    fn every_level_of_the_wrong_shape_is_refused() {
        assert!(err(r#"[]"#).contains("resources"));
        assert!(err(r#"{"resources": []}"#).contains("$.resources"));
        assert!(err(r#"{"resources": {"s": 3}}"#).contains(r#"$.resources["s"]"#));
        assert!(err(r#"{"resources": {"s": {"c": "label"}}}"#).contains(r#"["s"]["c"]"#));
        assert!(err(r#"{"resources": {"s": {"c": {"label": 3}}}}"#).contains(".label"));
        assert!(err(r#"{"resources": {"s": {"c": {"label": "x", "why": 3}}}}"#).contains(".why"));
        assert!(err(r#"{"version": "1", "resources": {}}"#).contains("$.version"));
        assert!(err(r#"{"version": 2, "resources": {}}"#).contains("$.version"));
        assert!(err(r#"not json"#).contains("not valid JSON"));
    }

    #[test]
    fn a_declared_stem_no_input_matches_is_refused_and_an_undeclared_input_is_not() {
        let n = Nominations::parse(GOOD, "n.json").unwrap();
        let e = n
            .check_stems(&["edgar".to_string()], "n.json")
            .expect_err("naics is declared and absent")
            .to_string();
        assert!(e.contains("naics"), "{e}");

        // An input with no stem in the file is inferred, silently.
        n.check_stems(&["naics".to_string(), "edgar".to_string()], "n.json")
            .expect("an undeclared input is not an error");
    }

    #[test]
    fn a_declared_column_the_file_does_not_have_is_refused() {
        let n = Nominations::parse(GOOD, "n.json").unwrap();
        let headers = vec!["corpus".to_string(), "code".to_string()];
        let e = n
            .check_columns("naics", &headers, Path::new("naics.csv"), "n.json")
            .expect_err("description is declared and absent")
            .to_string();
        assert!(e.contains("description"), "{e}");
        assert!(e.contains("naics"), "{e}");
        assert!(e.contains("naics.csv"), "{e}");

        let headers = vec!["corpus".to_string(), "description".to_string()];
        n.check_columns("naics", &headers, Path::new("naics.csv"), "n.json")
            .expect("every declared column is present");
        // A stem with no entry has nothing to check.
        n.check_columns("other", &[], Path::new("other.csv"), "n.json")
            .expect("an undeclared stem has no columns to check");
    }

    #[test]
    fn an_unknown_label_and_a_type_outside_the_profile_are_both_refused() {
        let taxonomy = finetype_core::Taxonomy::from_yaml(
            r#"
representation.text.plain_text:
  broad_type: VARCHAR
  frictionless:
    type: string
  validation:
    type: string
    minLength: 1
    maxLength: 65536
container.array.comma_separated:
  broad_type: VARCHAR
  frictionless:
    type: list
  validation:
    type: string
"#,
        )
        .expect("test taxonomy parses");

        let good = r#"{"resources": {"s": {"c": {"label": "representation.text.plain_text"}}}}"#;
        Nominations::parse(good, "n.json")
            .unwrap()
            .check_against_taxonomy(&taxonomy, "n.json")
            .expect("a known, profile-typed label is accepted");

        let unknown = r#"{"resources": {"s": {"c": {"label": "identity.person.emial"}}}}"#;
        let e = Nominations::parse(unknown, "n.json")
            .unwrap()
            .check_against_taxonomy(&taxonomy, "n.json")
            .expect_err("an unknown label is refused")
            .to_string();
        assert!(e.contains("identity.person.emial"), "{e}");
        assert!(e.contains('c'), "{e}");

        let listy = r#"{"resources": {"s": {"c": {"label": "container.array.comma_separated"}}}}"#;
        let e = Nominations::parse(listy, "n.json")
            .unwrap()
            .check_against_taxonomy(&taxonomy, "n.json")
            .expect_err("a `list` type is refused")
            .to_string();
        assert!(e.contains("list"), "{e}");
        assert!(e.contains("container.array.comma_separated"), "{e}");
    }
}
