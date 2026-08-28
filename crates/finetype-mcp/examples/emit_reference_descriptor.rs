//! Print a Data Package descriptor to stdout so the **emitted bytes** can be
//! put through the reference implementation, `frictionless`.
//!
//! Nothing in CI runs `frictionless` — it is a Python package and this is a
//! Rust workspace — so the gate that ships is
//! `crates/finetype-mcp/tests/conformance.rs`, which asks
//! `finetype-core::frictionless_vocabulary` the same question the reference
//! implementation asks. This example is how that gate is confirmed to be
//! asking it correctly, by hand, against the real thing:
//!
//! ```text
//! cargo run -p finetype-mcp --example emit_reference_descriptor > dp.json
//! uv run --with frictionless==5.19.0 python -c \
//!   'from frictionless import Package; Package("dp.json"); print("ACCEPTED")'
//! ```
//!
//! With no arguments it emits one field per taxonomy label. With arguments it
//! emits the named columns, each `name=label`, which is how a descriptor
//! already published can be re-emitted through the current engine and re-tested.

use finetype_core::Taxonomy;
use finetype_mcp::datapackage::{emit_datapackage, DatapackageColumn, ResourceMeta};

fn main() {
    let taxonomy = Taxonomy::embedded().expect("embedded taxonomy");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let columns: Vec<(String, String)> = if args.is_empty() {
        taxonomy
            .labels()
            .iter()
            .enumerate()
            .map(|(i, label)| (format!("c{i}"), label.clone()))
            .collect()
    } else {
        args.iter()
            .map(|arg| match arg.split_once('=') {
                Some((name, label)) => (name.to_string(), label.to_string()),
                None => {
                    eprintln!("expected `name=label`, got `{arg}`");
                    std::process::exit(2);
                }
            })
            .collect()
    };

    // No observed values: the worst case for the constraint router, because an
    // unobserved column publishes the type's canonical `pattern` verbatim
    // rather than a fitted or omitted one.
    let empty: Vec<String> = Vec::new();
    let cols: Vec<DatapackageColumn<'_>> = columns
        .iter()
        .map(|(name, label)| DatapackageColumn {
            name,
            label,
            values: &empty,
            confidence: Some(0.9),
            locale: None,
        })
        .collect();

    let meta = ResourceMeta {
        name: "reference".into(),
        path: "reference.csv".into(),
        format: "csv".into(),
        mediatype: "text/csv".into(),
        encoding: Some("utf-8".into()),
        bytes: 0,
        hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
        created: "2026-08-28T00:00:00Z".into(),
    };

    let descriptor = emit_datapackage(&cols, &meta, &taxonomy, 32);
    println!(
        "{}",
        serde_json::to_string_pretty(&descriptor).expect("serialise descriptor")
    );
}
