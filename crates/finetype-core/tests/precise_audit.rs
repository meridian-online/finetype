//! ac-01b — taxonomy audit for `Validation::is_precise()`.
//!
//! Walks every definition in `labels/definitions_*.yaml` and emits a
//! sorted TSV to
//! `tests/fixtures/precise_audit.tsv`
//! with columns `(type_key, has_enum, pattern, is_precise)`.
//!
//! The TSV is diffable in PRs — regenerated on every CI run — and is
//! the empirical audit of which taxonomy entries the demotion guard
//! would accept vs reject. Patterns sampled from `is_precise=false`
//! rows anchor ac-01's unit tests against real-world evidence.
//!
//! Run: `cargo test -p finetype-core --test precise_audit -- --nocapture`
//!
//! Decision: recorded in a private planning repo (demotion guard over promotion, MADR 0059)

use finetype_core::taxonomy::Taxonomy;
use std::path::PathBuf;

/// Resolve the workspace root from this crate's manifest dir.
///
/// `CARGO_MANIFEST_DIR` is `.../finetype/crates/finetype-core` during
/// tests; the workspace root is two `..` up.
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> crates/
    p.pop(); // -> workspace root
    p
}

#[test]
fn dgd_ac01b_emit_precise_audit_tsv() {
    let root = workspace_root();
    let labels_dir = root.join("labels");
    let out_path = root.join("tests/fixtures/precise_audit.tsv");

    let taxonomy = Taxonomy::from_directory(&labels_dir)
        .expect("load taxonomy from labels/ — is labels/ present in workspace?");

    // Stable ordering for diffable output.
    let mut keys: Vec<String> = taxonomy.labels().to_vec();
    keys.sort();

    let mut rows: Vec<String> = Vec::with_capacity(keys.len() + 1);
    rows.push("type_key\thas_enum\tpattern\tis_precise".to_string());

    let mut n_precise_true = 0usize;
    let mut n_precise_false = 0usize;

    for key in &keys {
        let def = taxonomy.get(key).expect("taxonomy.get(key) returns Some");
        let (has_enum, pattern_raw, is_precise) = match &def.validation {
            Some(v) => {
                let has_enum = v
                    .enum_values
                    .as_ref()
                    .map(|e| !e.is_empty())
                    .unwrap_or(false);
                let pattern = v.pattern.clone().unwrap_or_default();
                (has_enum, pattern, v.is_precise())
            }
            None => (false, String::new(), false),
        };

        if is_precise {
            n_precise_true += 1;
        } else {
            n_precise_false += 1;
        }

        // TSV-escape the pattern: replace tabs and newlines (should be
        // absent in practice, but defence in depth).
        let pattern_esc = pattern_raw.replace('\t', "\\t").replace('\n', "\\n");

        rows.push(format!(
            "{}\t{}\t{}\t{}",
            key, has_enum, pattern_esc, is_precise
        ));
    }

    let contents = rows.join("\n") + "\n";

    // Ensure parent dir exists (spec dir is under orbit/, always present
    // in-repo; no-op in normal runs but keeps the test robust).
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create spec dir");
    }

    std::fs::write(&out_path, &contents)
        .unwrap_or_else(|e| panic!("write {}: {}", out_path.display(), e));

    // Spec verification — 251 rows ± 1, non-zero on both sides (250 prior +
    // technology.filesystem.filename, reservoir-mining sweep 2026-07-14).
    let n = keys.len();
    assert!(
        (250..=252).contains(&n),
        "expected 251 ± 1 taxonomy rows, got {n}"
    );
    assert!(
        n_precise_true >= 1,
        "expected at least 1 is_precise=true row"
    );
    assert!(
        n_precise_false >= 1,
        "expected at least 1 is_precise=false row"
    );

    eprintln!(
        "precise_audit: {n} rows written to {} ({} precise, {} imprecise)",
        out_path.display(),
        n_precise_true,
        n_precise_false,
    );
}

/// The registry patterns that are deliberately a prefix of the value they
/// validate, each with the value it is a prefix of. Every other
/// `validation.pattern` in `labels/definitions_*.yaml` ends in `$`.
///
/// JSON Schema's `pattern` is a search, not a full match, so an unanchored
/// pattern accepts any value that merely starts well — `R$ 1 apple` passed
/// `finance.currency.amount_multisym` until it gained its end anchor. These
/// three are right as they are: the value runs on past anything a pattern could
/// name, so an end anchor would reject the real family.
const UNANCHORED_PREFIXES: &[(&str, &str)] = &[
    (
        "container.object.yaml",
        "a YAML mapping, whose `key: value` lines run on past the first key",
    ),
    (
        "geography.format.wkt",
        "a WKT geometry, whose coordinate list runs on past the opening parenthesis",
    ),
    (
        "technology.internet.user_agent",
        "a User-Agent header, whose product and comment tokens run on past the leading product",
    ),
];

/// True when `pattern` ends in an unescaped `$`: an even run of backslashes
/// (including none) before it, so a literal `\$` at the end is not an anchor.
fn ends_anchored(pattern: &str) -> bool {
    match pattern.strip_suffix('$') {
        None => false,
        Some(rest) => (rest.len() - rest.trim_end_matches('\\').len()) % 2 == 0,
    }
}

/// `(type_key, pattern)` for every definition whose `validation.pattern` does
/// not end in an unescaped `$`, sorted by key. A definition with no pattern is
/// not listed: there is nothing there to run on.
fn unanchored_patterns(taxonomy: &Taxonomy) -> Vec<(String, String)> {
    let mut keys: Vec<&String> = taxonomy.labels().iter().collect();
    keys.sort();
    keys.into_iter()
        .filter_map(|key| {
            let pattern = taxonomy.get(key)?.validation.as_ref()?.pattern.as_deref()?;
            (!ends_anchored(pattern)).then(|| (key.clone(), pattern.to_string()))
        })
        .collect()
}

#[test]
fn registry_patterns_end_anchored_unless_a_recorded_prefix() {
    let root = workspace_root();
    let taxonomy = Taxonomy::from_directory(root.join("labels"))
        .expect("load taxonomy from labels/ — is labels/ present in workspace?");

    let found = unanchored_patterns(&taxonomy);

    let unrecorded: Vec<String> = found
        .iter()
        .filter(|(key, _)| !UNANCHORED_PREFIXES.iter().any(|(k, _)| k == key))
        .map(|(key, pattern)| format!("  {key}: {pattern}"))
        .collect();
    assert!(
        unrecorded.is_empty(),
        "validation.pattern without an end anchor, so it accepts any value that merely starts \
         well. Anchor it with `$`, or record it in UNANCHORED_PREFIXES with the value it is a \
         prefix of:\n{}",
        unrecorded.join("\n")
    );

    // A recorded exception that has since been anchored or removed is a stale
    // entry: the list would keep excusing a pattern nothing needs to excuse.
    let stale: Vec<&str> = UNANCHORED_PREFIXES
        .iter()
        .map(|(key, _)| *key)
        .filter(|key| !found.iter().any(|(k, _)| k == key))
        .collect();
    assert!(
        stale.is_empty(),
        "UNANCHORED_PREFIXES records {stale:?}, which no longer has an unanchored \
         validation.pattern — delete the entry"
    );
}

#[test]
fn the_anchor_audit_names_an_unanchored_pattern_it_is_given() {
    fn definition(key: &str, pattern: &str) -> String {
        format!(
            r#"
{key}:
  title: "t"
  description: "d"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: VARCHAR
  format_string: null
  transform: null
  transform_ext: null
  decompose: null
  validation:
    type: string
    pattern: "{pattern}"
  tier: [VARCHAR, text]
  release_priority: 5
  aliases: []
  samples:
    - "x"
  references: null
  notes: null
"#
        )
    }
    let report = |key: &str, pattern: &str| {
        let tax = Taxonomy::from_yaml(&definition(key, pattern)).expect("test YAML parses");
        unanchored_patterns(&tax)
    };

    // A fourth unanchored pattern is reported, with its key and its pattern.
    assert_eq!(
        report("text.sample.fourth", "^abc[0-9]"),
        vec![("text.sample.fourth".to_string(), "^abc[0-9]".to_string())]
    );
    // An anchored pattern is not.
    assert!(report("text.sample.anchored", "^abc[0-9]+$").is_empty());
    // A literal `\$` at the end is not an anchor, but `\\$` (an escaped
    // backslash, then the anchor) is.
    assert_eq!(report("text.sample.escaped", "^abc\\\\$").len(), 1);
    assert!(report("text.sample.double", "^abc\\\\\\\\$").is_empty());
}
