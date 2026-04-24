//! amvg_sample — deterministic sample printer for the amount-variant-generators
//! diagnostics (orbit/specs/2026-04-24-amount-variant-generators).
//!
//! Usage:
//!   cargo run --example amvg_sample -- <key> <count> <seed>
//!
//! Prints `count` values for taxonomy `key`, one per line, using a seeded RNG.
//! This is the deterministic surface ac-02 (Jaccard) and ac-04 (confidence)
//! Python orchestrators consume — so sample shapes are bit-reproducible across
//! runs given the same (key, count, seed).

use std::env;
use std::path::PathBuf;

use finetype_core::generator::Generator;
use finetype_core::taxonomy::Taxonomy;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: amvg_sample <key> <count> <seed>");
        std::process::exit(2);
    }
    let key = &args[1];
    let count: usize = args[2]
        .parse()
        .expect("count must be a non-negative integer");
    let seed: u64 = args[3]
        .parse()
        .expect("seed must be a non-negative integer");

    // Taxonomy lives at repo-root-relative `labels/`. Resolve from CARGO_MANIFEST_DIR
    // so the example binary works from any cwd.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let labels_dir = PathBuf::from(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("labels"))
        .expect("resolve labels dir");

    let taxonomy = Taxonomy::from_directory(&labels_dir)
        .unwrap_or_else(|e| panic!("load taxonomy from {}: {e}", labels_dir.display()));

    let mut generator = Generator::with_seed(taxonomy, seed);
    for _ in 0..count {
        match generator.generate_value(key) {
            Ok(v) => println!("{v}"),
            Err(e) => {
                eprintln!("generate error for key={key}: {e}");
                std::process::exit(1);
            }
        }
    }
}
