//! Answer, for every `samples:` value in a labels directory, whether it satisfies
//! its own leaf's `validation.pattern`.
//!
//! This exists so a gate written in Python can reach the pattern code the product
//! actually uses instead of carrying a second implementation of it — the same
//! reason `checksum_samples.rs` exists for check digits. Two parts of the product
//! are borrowed whole and neither is reimplemented here:
//!
//! * `Taxonomy::from_directory` reads the YAML, so escape decoding, block
//!   scalars and flow collections resolve to the same strings the shipped
//!   taxonomy carries. A hand-rolled reader would have to decode `\\u2013` and
//!   `\\\\d` itself to compare anything.
//! * `CompiledValidator` applies the pattern, so the regex dialect is
//!   jsonschema's (`fancy-regex`), not Python's `re`. Ten of the patterns in
//!   `labels/` use `\\p{…}`, which Python's `re` cannot compile at all, and two
//!   use lookaround — a Python reimplementation would refuse or disagree on
//!   those twelve.
//!
//! `scripts/check_sample_patterns.py` is the caller.
//!
//! SCOPE: `validation.pattern` and nothing else. `minLength`, `maxLength`,
//! `enum`, `minimum` and `maximum` are stripped before the validator is built,
//! so a verdict here is about the pattern alone. `validation_by_locale` is not
//! consulted: the base `validation` is the universal fallback the product itself
//! reaches for through `Taxonomy::get_validator`, and on the six leaves that
//! carry per-locale schemas it is the superset.
//!
//! Protocol — arguments in, one record per line out, tab-separated:
//!
//! ```text
//!     representation.identifier.numeric_code<TAB>2<TAB>ok<TAB>"06"
//! ```
//!
//! The fields are the leaf key, the 0-based index of the value in the leaf's
//! `samples:` sequence, the verdict, and the sample as a JSON string. Records are
//! emitted for EVERY sample of EVERY leaf, including leaves with no pattern, so a
//! caller can cross-check its own reading of the same files against this one; a
//! caller that only ever hears about failures cannot tell a clean tree from a
//! reader that stopped reading. A leaf with NO samples gets one record at index
//! `-1` for the same reason: without it a leaf that has lost its samples and a
//! leaf that was never read are the same silence. Verdicts:
//!
//! * `ok` — the leaf has a pattern and the sample satisfies it.
//! * `fail` — the leaf has a pattern and the sample does not satisfy it.
//! * `bad-pattern` — the leaf has a pattern that does not compile. The product
//!   drops such a validator with `.ok()` and validates nothing, so this is
//!   reported rather than passed.
//! * `no-pattern` — the leaf has no `validation.pattern`.
//! * `non-string` — the sample is not a YAML string (a bare `840` is an integer).
//! * `no-samples` — index `-1`: the leaf has a pattern and publishes nothing to
//!   exercise it.
//! * `no-samples-no-pattern` — index `-1`: the leaf publishes no samples and has
//!   no pattern either.
//!
//! Exit 0 when every record was emitted, whatever the verdicts — this reports,
//! the caller decides. Exit 2 when the taxonomy cannot be loaded at all.

use finetype_core::taxonomy::{Taxonomy, Validation};
use finetype_core::validator::CompiledValidator;
use std::io::{self, Write};
use std::process;

fn usage() -> ! {
    eprintln!("usage: validate-samples --labels DIR");
    process::exit(2);
}

/// The leaf's validation with everything but `pattern` removed.
///
/// A verdict has to be attributable to the pattern. `minLength` and friends are
/// live on many of the same leaves, and folding them in would let a length
/// failure be reported as a pattern failure.
fn pattern_only(validation: &Validation) -> Option<Validation> {
    let pattern = validation.pattern.clone()?;
    Some(Validation {
        schema_type: Some("string".to_string()),
        pattern: Some(pattern),
        min_length: None,
        max_length: None,
        minimum: None,
        maximum: None,
        enum_values: None,
    })
}

/// The sample as a string, for the samples this tool can answer for.
///
/// `Definition::samples` is a `Vec<serde_yaml::Value>`, so an unquoted `840` is
/// an integer and a nested mapping is a mapping. Only a string is a published
/// example of a VARCHAR value, and only a string is what a pattern applies to.
fn as_string(value: &serde_yaml::Value) -> Option<&str> {
    value.as_str()
}

fn json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut labels_dir: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--labels" => {
                index += 1;
                match args.get(index) {
                    Some(value) => labels_dir = Some(value.clone()),
                    None => usage(),
                }
            }
            _ => usage(),
        }
        index += 1;
    }
    let Some(labels_dir) = labels_dir else {
        usage()
    };

    let taxonomy = match Taxonomy::from_directory(&labels_dir) {
        Ok(taxonomy) => taxonomy,
        Err(err) => {
            eprintln!("validate-samples: cannot load {labels_dir}: {err}");
            process::exit(2);
        }
    };

    let mut out = String::new();
    // `Taxonomy::labels()` is sorted; `definitions()` iterates a HashMap and is
    // not. Sorted output keeps a diff of two runs readable.
    for label in taxonomy.labels() {
        let Some(definition) = taxonomy.get(label) else {
            continue;
        };
        let compiled = definition
            .validation
            .as_ref()
            .and_then(pattern_only)
            .map(|validation| CompiledValidator::new_for_label(&validation, label));
        if definition.samples.is_empty() {
            let verdict = if compiled.is_some() {
                "no-samples"
            } else {
                "no-samples-no-pattern"
            };
            out.push_str(&format!("{label}\t-1\t{verdict}\t\"\"\n"));
            continue;
        }
        for (position, sample) in definition.samples.iter().enumerate() {
            // Pattern-ness is decided before string-ness, so a leaf this tool
            // has no pattern for reports `no-pattern` for all of its samples
            // whatever they are. `non-string` is reserved for the case the
            // caller can act on: a leaf that HAS a pattern publishing a sample
            // no pattern can be applied to.
            let (verdict, text) = match (&compiled, as_string(sample)) {
                (None, text) => ("no-pattern", text.unwrap_or("")),
                (Some(_), None) => ("non-string", ""),
                (Some(Err(_)), Some(text)) => ("bad-pattern", text),
                (Some(Ok(validator)), Some(text)) => (
                    if validator.is_valid(text) {
                        "ok"
                    } else {
                        "fail"
                    },
                    text,
                ),
            };
            out.push_str(&format!(
                "{label}\t{position}\t{verdict}\t{}\n",
                json_string(text)
            ));
        }
    }

    if let Err(err) = io::stdout().write_all(out.as_bytes()) {
        eprintln!("validate-samples: cannot write stdout: {err}");
        process::exit(2);
    }
}
