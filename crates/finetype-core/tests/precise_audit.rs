//! ac-01b — taxonomy audit for `Validation::is_precise()`.
//!
//! Walks every definition in `labels/definitions_*.yaml` and emits a
//! sorted TSV to
//! `.orbit/specs/2026-04-21-sharpen-demotion-guard/precise_audit.tsv`
//! with columns `(type_key, has_enum, pattern, is_precise)`.
//!
//! The TSV is diffable in PRs — regenerated on every CI run — and is
//! the empirical audit of which taxonomy entries the demotion guard
//! would accept vs reject. Patterns sampled from `is_precise=false`
//! rows anchor ac-01's unit tests against real-world evidence.
//!
//! Run: `cargo test -p finetype-core --test precise_audit -- --nocapture`
//!
//! Spec: .orbit/specs/2026-04-21-sharpen-demotion-guard/spec.yaml (ac-01b)
//! MADR: .orbit/choices/0059-demotion-guard-over-promotion.md

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
    let out_path = root.join(".orbit/specs/2026-04-21-sharpen-demotion-guard/precise_audit.tsv");

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

    // Spec verification — 240 rows ± 1, non-zero on both sides.
    let n = keys.len();
    assert!(
        (239..=241).contains(&n),
        "expected 240 ± 1 taxonomy rows, got {n}"
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
