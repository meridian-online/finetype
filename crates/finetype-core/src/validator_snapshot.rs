//! A frozen, serialisable copy of the taxonomy's `validation` blocks.
//!
//! A model is trained against the rules as they stood at training time. The
//! snapshot is written beside the model so inference can compile those rules
//! rather than whatever the live taxonomy says today.

use crate::taxonomy::{Taxonomy, Validation};
use crate::validator::{CompiledValidator, ValidatorError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

/// File name of the snapshot inside a model directory.
pub const SNAPSHOT_FILE: &str = "validation_snapshot.json";

const SNAPSHOT_FORMAT: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorSnapshot {
    pub format: u32,
    /// Every label key the snapshot covers, in the order given.
    pub keys: Vec<String>,
    /// Validation block per label key. A type with no validation is absent.
    pub validations: BTreeMap<String, Validation>,
}

impl ValidatorSnapshot {
    /// Freeze the validation blocks of `keys` as the taxonomy has them now.
    pub fn from_taxonomy(tax: &Taxonomy, keys: &[String]) -> Self {
        let validations = keys
            .iter()
            .filter_map(|k| {
                let validation = tax.get(k)?.validation.clone()?;
                Some((k.clone(), validation))
            })
            .collect();
        Self {
            format: SNAPSHOT_FORMAT,
            keys: keys.to_vec(),
            validations,
        }
    }

    /// Parse a snapshot, refusing an unknown format, duplicate keys, or a rule
    /// for a label the snapshot does not list.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ValidatorError> {
        let bad = |detail: String| ValidatorError::SchemaCompilationError {
            label: SNAPSHOT_FILE.to_string(),
            detail,
        };
        let snapshot: Self = serde_json::from_slice(bytes).map_err(|e| bad(e.to_string()))?;
        if snapshot.format != SNAPSHOT_FORMAT {
            return Err(bad(format!(
                "unsupported format {} (expected {SNAPSHOT_FORMAT})",
                snapshot.format
            )));
        }
        let listed: HashSet<&String> = snapshot.keys.iter().collect();
        if listed.len() != snapshot.keys.len() {
            return Err(bad("duplicate keys".to_string()));
        }
        if let Some(stray) = snapshot.validations.keys().find(|k| !listed.contains(k)) {
            return Err(bad(format!("validation for unlisted label {stray}")));
        }
        Ok(snapshot)
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("a snapshot of plain data serialises")
    }

    /// Compile every frozen rule. Unlike the live taxonomy, which skips a bad
    /// rule, a frozen artefact fails loudly on the first that does not compile.
    pub fn compile(&self) -> Result<HashMap<String, CompiledValidator>, ValidatorError> {
        self.validations
            .iter()
            .map(|(label, v)| Ok((label.clone(), CompiledValidator::new_for_label(v, label)?)))
            .collect()
    }

    /// Every label the snapshot covers, including those with no rule.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.keys.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "representation.identifier.uuid";

    fn labels_dir() -> Option<&'static str> {
        ["labels", "../../labels"]
            .into_iter()
            .find(|d| std::path::Path::new(d).is_dir())
    }

    fn live_taxonomy() -> Taxonomy {
        let mut tax = Taxonomy::from_directory(labels_dir().expect("labels/ not found"))
            .expect("labels/ parses");
        tax.compile_validators();
        tax
    }

    #[test]
    fn snapshot_round_trips_and_compiles_like_live() {
        let tax = live_taxonomy();
        let snapshot = ValidatorSnapshot::from_taxonomy(&tax, tax.labels());
        let snapshot = ValidatorSnapshot::from_json(snapshot.to_json_pretty().as_bytes()).unwrap();
        let compiled = snapshot.compile().unwrap();

        for label in [
            UUID,
            "finance.currency.currency_code",
            "representation.numeric.percentage",
        ] {
            let live = tax.get_validator(label).unwrap();
            let mut values: Vec<String> = tax
                .get(label)
                .unwrap()
                .samples
                .iter()
                .filter_map(|s| match s {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    other => serde_yaml::to_string(other)
                        .ok()
                        .map(|s| s.trim().to_string()),
                })
                .collect();
            assert!(!values.is_empty(), "{label} has no samples");
            values.push("not-a-value".to_string());
            for v in values {
                assert_eq!(
                    compiled[label].is_valid(&v),
                    live.is_valid(&v),
                    "{label} disagrees on {v:?}"
                );
            }
        }
    }

    #[test]
    fn snapshot_keeps_keys_without_a_rule() {
        let tax = live_taxonomy();
        // Every live type has a rule, so a key the taxonomy lacks stands in for one.
        let keyless = "test.no.rule".to_string();
        let mut keys = tax.labels().to_vec();
        keys.push(keyless.clone());
        let snapshot = ValidatorSnapshot::from_taxonomy(&tax, &keys);
        let snapshot = ValidatorSnapshot::from_json(snapshot.to_json_pretty().as_bytes()).unwrap();
        assert!(snapshot.keys().any(|k| *k == keyless));
        assert!(!snapshot.validations.contains_key(&keyless));
        assert_eq!(snapshot.keys, keys);
    }

    fn refusal(json: &str) -> String {
        ValidatorSnapshot::from_json(json.as_bytes())
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn from_json_refuses_wrong_format() {
        let msg = refusal(r#"{"format":2,"keys":["a"],"validations":{}}"#);
        assert!(msg.contains("unsupported format"), "{msg}");
    }

    #[test]
    fn from_json_refuses_duplicate_keys() {
        let msg = refusal(r#"{"format":1,"keys":["a","a"],"validations":{}}"#);
        assert!(msg.contains("duplicate"), "{msg}");
    }

    #[test]
    fn from_json_refuses_stray_validation() {
        let msg = refusal(r#"{"format":1,"keys":["a"],"validations":{"b":{"type":"string"}}}"#);
        assert!(msg.contains("unlisted label b"), "{msg}");
    }

    #[test]
    fn snapshot_ignores_later_taxonomy_edits() {
        // A small synthetic taxonomy, so a future edit to a published rule
        // cannot break what this proves.
        const KEY: &str = "test.frozen.hex_code";
        let rule = |class: &str| {
            format!("{KEY}:\n  validation:\n    type: string\n    pattern: \"^[{class}]{{4}}$\"\n")
        };
        let mut tax = Taxonomy::from_yaml(&rule("0-9a-fA-F")).unwrap();
        tax.compile_validators();
        let snapshot = ValidatorSnapshot::from_taxonomy(&tax, tax.labels());

        // The live rule is later tightened to lower case only.
        let mut edited_tax = Taxonomy::from_yaml(&rule("0-9a-f")).unwrap();
        edited_tax.compile_validators();

        assert!(!edited_tax.get_validator(KEY).unwrap().is_valid("ABCD"));
        assert!(snapshot.compile().unwrap()[KEY].is_valid("ABCD"));
    }
}
