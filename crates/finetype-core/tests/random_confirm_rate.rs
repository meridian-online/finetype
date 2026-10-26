//! How much random input each type's rule confirms.
//!
//! A type's published rule is meant to agree with its defining source, and a rule
//! that does can still confirm a large share of random input. This test measures
//! that over three seeded populations and records it in
//! `tests/fixtures/random_confirm_rate.tsv` (columns `label`, `published_ascii`,
//! `discrimination_ascii`, `published_digits`, `discrimination_digits`,
//! `published_tokens`, `discrimination_tokens`):
//!
//! - ascii: 10,000 printable-ASCII strings of length 1-40
//! - digits: 10,000 digit-only strings of length 1-20
//! - tokens: 10,000 identifier-like strings, drawn in equal parts from common
//!   words, hex strings of 8, 32, 40 or 64 characters, UUIDs in either case,
//!   random alphanumeric strings of 6-20 characters, three uppercase letters,
//!   and 10- or 13-digit numbers
//!
//! `published_*` is the share the type's published rule (pattern, length, numeric
//! bounds and enum, as `CompiledValidator::is_valid` evaluates them; no checksum or
//! membership check) confirms. `discrimination_*` is the share
//! `value_passes_for_discrimination` confirms, which is the same until a detector
//! exists for the label. Labels with no validation are omitted.
//!
//! Gate: `discrimination_ascii` must not exceed 0.5, since numeric types accept
//! digit strings by design and the digit and token populations are informational
//! (recorded and compared against the committed TSV, with no limit of their own).
//! For every label and population, `discrimination_*` must not exceed
//! `published_*`: a detector is stricter than the published rule by design. Labels
//! already above 0.5 when the test landed are listed in `BASELINE_EXCEPTIONS`
//! with the rate then measured. A listed label fails in three cases: its rate
//! rises above the recorded one, its rate falls to 0.5 or below (remove it from
//! the list), or it produces no row (remove it too).
//!
//! The test compares against the committed TSV and fails on any difference, so a
//! rule change regenerates it on purpose:
//! `UPDATE_RANDOM_CONFIRM_RATE=1 cargo test -p finetype-core --test random_confirm_rate`

use finetype_core::discrimination::value_passes_for_discrimination;
use finetype_core::taxonomy::Taxonomy;
use finetype_core::validator::resolve_label_validator;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::path::PathBuf;

const POPULATION: usize = 10_000;

/// Labels whose rate on random printable ASCII was already above 0.5 when this
/// test landed, each with that rate. They may not rise.
const BASELINE_EXCEPTIONS: &[(&str, f64)] = &[
    ("container.array.comma_separated", 0.9770),
    ("container.array.pipe_separated", 0.9751),
    ("container.array.semicolon_separated", 0.9781),
    ("container.array.whitespace_separated", 0.9763),
    ("container.object.csv", 0.9770),
    ("geography.address.full_address", 0.7765),
    ("geography.address.street_name", 0.9773),
    ("geography.coordinate.longitude", 0.9999),
    ("geography.location.city", 1.0000),
    ("geography.location.country", 0.9773),
    ("geography.location.region", 0.9773),
    ("identity.person.password", 1.0000),
    ("representation.discrete.ordinal", 0.7454),
    ("representation.text.entity_name", 0.9773),
    ("representation.text.plain_text", 1.0000),
];

/// Rate tolerance for the exceptions, so 4-decimal rounding in the list never fails.
const TOLERANCE: f64 = 0.0001;

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn ascii_population() -> Vec<String> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    (0..POPULATION)
        .map(|_| {
            // The explicit integer type fixes the random stream the TSV was generated from.
            let len = rng.gen_range(1u32..=40);
            (0..len)
                .map(|_| rng.gen_range(0x20u8..=0x7e) as char)
                .collect()
        })
        .collect()
}

fn digit_population() -> Vec<String> {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    (0..POPULATION)
        .map(|_| {
            // The explicit integer type fixes the random stream the TSV was generated from.
            let len = rng.gen_range(1u32..=20);
            (0..len)
                .map(|_| (b'0' + rng.gen_range(0..10u8)) as char)
                .collect()
        })
        .collect()
}

const WORDS: &[&str] = &[
    "apple", "table", "river", "green", "orange", "paper", "street", "yes", "no", "name", "total",
    "open", "closed", "pending", "active", "north", "london", "alpha", "sample", "value",
];

fn token_population() -> Vec<String> {
    const HEX: &[u8] = b"0123456789abcdef";
    const ALNUM: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    (0..POPULATION)
        .map(|_| {
            // The explicit integer types fix the random stream the TSV was generated from.
            match rng.gen_range(0u32..6) {
                0 => WORDS[rng.gen_range(0..WORDS.len())].to_string(),
                1 => {
                    let len = [8usize, 32, 40, 64][rng.gen_range(0..4usize)];
                    (0..len)
                        .map(|_| HEX[rng.gen_range(0..HEX.len())] as char)
                        .collect()
                }
                2 => {
                    let hex: String = (0..32)
                        .map(|_| HEX[rng.gen_range(0..HEX.len())] as char)
                        .collect();
                    let uuid = format!(
                        "{}-{}-{}-{}-{}",
                        &hex[0..8],
                        &hex[8..12],
                        &hex[12..16],
                        &hex[16..20],
                        &hex[20..32]
                    );
                    if rng.gen_range(0u32..2) == 0 {
                        uuid
                    } else {
                        uuid.to_uppercase()
                    }
                }
                3 => {
                    let len = rng.gen_range(6u32..=20);
                    (0..len)
                        .map(|_| ALNUM[rng.gen_range(0..ALNUM.len())] as char)
                        .collect()
                }
                4 => (0..3)
                    .map(|_| (b'A' + rng.gen_range(0..26u8)) as char)
                    .collect(),
                _ => {
                    let len = if rng.gen_range(0u32..2) == 0 { 10 } else { 13 };
                    (0..len)
                        .map(|_| (b'0' + rng.gen_range(0..10u8)) as char)
                        .collect()
                }
            }
        })
        .collect()
}

fn rate(population: &[String], pass: impl Fn(&str) -> bool) -> f64 {
    population.iter().filter(|v| pass(v)).count() as f64 / population.len() as f64
}

struct Row {
    label: String,
    published_ascii: f64,
    discrimination_ascii: f64,
    published_digits: f64,
    discrimination_digits: f64,
    published_tokens: f64,
    discrimination_tokens: f64,
}

#[test]
fn random_confirm_rate() {
    let root = workspace_root();
    let mut tax = Taxonomy::from_directory(root.join("labels")).expect("load labels");
    tax.compile_validators();

    let ascii = ascii_population();
    let digits = digit_population();
    let tokens = token_population();

    let mut labels: Vec<String> = tax.labels().to_vec();
    labels.sort();

    let mut rows = Vec::new();
    for label in &labels {
        let has_validation = tax.get(label).is_some_and(|d| d.validation.is_some());
        let Some(rule) = resolve_label_validator(label, &tax).filter(|_| has_validation) else {
            continue;
        };
        rows.push(Row {
            label: label.clone(),
            published_ascii: rate(&ascii, |v| rule.is_valid(v)),
            discrimination_ascii: rate(&ascii, |v| value_passes_for_discrimination(v, label, &tax)),
            published_digits: rate(&digits, |v| rule.is_valid(v)),
            discrimination_digits: rate(&digits, |v| {
                value_passes_for_discrimination(v, label, &tax)
            }),
            published_tokens: rate(&tokens, |v| rule.is_valid(v)),
            discrimination_tokens: rate(&tokens, |v| {
                value_passes_for_discrimination(v, label, &tax)
            }),
        });
    }

    let mut tsv = String::from(
        "label\tpublished_ascii\tdiscrimination_ascii\tpublished_digits\tdiscrimination_digits\tpublished_tokens\tdiscrimination_tokens\n",
    );
    for r in &rows {
        tsv.push_str(&format!(
            "{}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\n",
            r.label,
            r.published_ascii,
            r.discrimination_ascii,
            r.published_digits,
            r.discrimination_digits,
            r.published_tokens,
            r.discrimination_tokens
        ));
    }

    let path = root.join("tests/fixtures/random_confirm_rate.tsv");
    if std::env::var_os("UPDATE_RANDOM_CONFIRM_RATE").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).expect("create fixtures dir");
        std::fs::write(&path, &tsv).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }

    let mut failures = Vec::new();
    for r in &rows {
        for (population, published, discrimination) in [
            ("ascii", r.published_ascii, r.discrimination_ascii),
            ("digits", r.published_digits, r.discrimination_digits),
            ("tokens", r.published_tokens, r.discrimination_tokens),
        ] {
            if discrimination > published {
                failures.push(format!(
                    "{}: discrimination {discrimination:.4} exceeds published {published:.4} on {population}",
                    r.label
                ));
            }
        }
        let rate = r.discrimination_ascii;
        match BASELINE_EXCEPTIONS.iter().find(|(l, _)| *l == r.label) {
            Some((_, recorded)) if rate > recorded + TOLERANCE => failures.push(format!(
                "{}: {rate:.4} rose above its recorded baseline {recorded:.4}",
                r.label
            )),
            None if rate > 0.5 => failures.push(format!(
                "{}: confirms {rate:.4} of random printable ASCII (limit 0.5); add a detector",
                r.label
            )),
            _ => {}
        }
    }
    for (label, recorded) in BASELINE_EXCEPTIONS {
        match rows.iter().find(|r| r.label == *label) {
            None => failures.push(format!(
                "{label}: listed in BASELINE_EXCEPTIONS but produced no row; remove it"
            )),
            Some(r) if r.discrimination_ascii <= 0.5 => failures.push(format!(
                "{label}: now {:.4} (recorded {recorded:.4}), at or below 0.5; remove it from BASELINE_EXCEPTIONS",
                r.discrimination_ascii
            )),
            _ => {}
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));

    let committed = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        committed == tsv,
        "{} is out of date; regenerate it with UPDATE_RANDOM_CONFIRM_RATE=1",
        path.display()
    );
}
