//! Emit, for every leaf in a labels directory, the SQL the registry publishes for
//! it and the samples it publishes beside that SQL.
//!
//! This exists so a gate written in Python can run the registry's expressions on
//! the strings the product actually ships, rather than on a second reading of the
//! YAML. `Taxonomy::from_directory` is borrowed whole, so escape decoding, block
//! scalars and flow collections resolve exactly as they do for `finetype taxonomy
//! --full`. That matters here more than for the other two oracles: a transform
//! written as a folded block scalar keeps a `令` escape as six literal
//! characters, and a reader that decoded it would run an expression the product
//! does not publish and pass it.
//!
//! `scripts/check_registry_expressions.py` is the caller.
//!
//! SCOPE: `transform`, `decompose` and `samples`, and nothing else. They are
//! emitted as parsed, with no judgement: the caller decides which shapes it can
//! run and refuses the rest.
//!
//! Protocol — arguments in, one JSON object per line out, one line per leaf in
//! `Taxonomy::labels()` order:
//!
//! ```text
//!     {"label":"…","transform":"…"|null,"decompose":{…}|"…"|null,"samples":[…]}
//! ```
//!
//! Every leaf gets a line, including a leaf with neither field, so a caller can
//! cross-check the leaf count against its own reading; a caller that only hears
//! about leaves with SQL cannot tell a clean tree from a reader that stopped.
//!
//! Exit status: 0 when every leaf was emitted, 2 on a usage error or a labels
//! directory `Taxonomy::from_directory` refuses.

use finetype_core::taxonomy::Taxonomy;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::process;

fn usage() -> ! {
    eprintln!("usage: registry-expressions --labels DIR");
    process::exit(2);
}

fn to_json(value: &serde_yaml::Value) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let labels_dir = match args.as_slice() {
        [flag, dir] if flag == "--labels" => dir.clone(),
        _ => usage(),
    };

    let taxonomy = match Taxonomy::from_directory(&labels_dir) {
        Ok(taxonomy) => taxonomy,
        Err(err) => {
            eprintln!("registry-expressions: cannot load {labels_dir}: {err}");
            process::exit(2);
        }
    };

    let mut out = String::new();
    for label in taxonomy.labels() {
        let Some(definition) = taxonomy.get(label) else {
            continue;
        };
        let record = json!({
            "label": label,
            "transform": definition.transform,
            "decompose": definition.decompose.as_ref().map(to_json),
            "samples": definition.samples.iter().map(to_json).collect::<Vec<_>>(),
        });
        out.push_str(&record.to_string());
        out.push('\n');
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    if handle.write_all(out.as_bytes()).is_err() || handle.flush().is_err() {
        process::exit(2);
    }
}
