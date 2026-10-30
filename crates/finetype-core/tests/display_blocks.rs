//! The `display` blocks on number definitions match the design's table, and
//! the table covers every definition the classifier calls a number.

use finetype_core::display::{display_kind_for, Display, DisplayKind};
use finetype_core::Taxonomy;
use std::path::Path;

/// (key, number specifier, flags, fixed currency).
type Row = (
    &'static str,
    &'static str,
    &'static [&'static str],
    Option<&'static str>,
);

const MONEY: &[&str] = &["money"];
const FRACTION: &[&str] = &["fraction"];
const NONE: &[&str] = &[];

const NUMBER_TABLE: &[Row] = &[
    ("finance.currency.amount", ",.2f", MONEY, None),
    ("finance.currency.amount_comma", ",.2f", MONEY, None),
    ("finance.currency.amount_comma_suffix", ",.2f", MONEY, None),
    ("finance.currency.amount_space", ",.2f", MONEY, None),
    ("finance.currency.amount_code_prefix", ",.2f", MONEY, None),
    ("finance.currency.amount_multisym", ",.2f", MONEY, None),
    ("finance.currency.amount_neg_trailing", ",.2f", MONEY, None),
    ("finance.currency.amount_accounting", "(,.2f", MONEY, None),
    ("finance.currency.amount_lakh", ",.2f", MONEY, Some("INR")),
    (
        "finance.currency.amount_apostrophe",
        ",.2f",
        MONEY,
        Some("CHF"),
    ),
    ("finance.currency.amount_nodecimal", ",d", MONEY, None),
    ("finance.currency.amount_crypto", ",.8~f", MONEY, None),
    ("finance.rate.yield", "~%", FRACTION, None),
    ("finance.rate.basis_points", "~%", FRACTION, None),
    ("representation.numeric.percentage", "~%", FRACTION, None),
    ("representation.numeric.integer_number", ",d", NONE, None),
    ("representation.numeric.decimal_number", ",", NONE, None),
    (
        "representation.numeric.decimal_number_comma",
        ",",
        NONE,
        None,
    ),
    (
        "representation.numeric.scientific_notation",
        "~e",
        NONE,
        None,
    ),
    ("representation.numeric.si_number", "~s", NONE, None),
    ("representation.file.file_size", ",d", &["bytes"], None),
    ("representation.identifier.increment", "d", NONE, None),
    ("geography.coordinate.latitude", "~f", NONE, None),
    ("geography.coordinate.longitude", "~f", NONE, None),
    ("identity.person.height", ",~f", NONE, None),
    ("identity.person.weight", ",~f", NONE, None),
    ("datetime.component.year", "d", NONE, None),
];

fn taxonomy() -> Taxonomy {
    Taxonomy::from_directory(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../labels"))
        .expect("load taxonomy")
}

#[test]
fn number_blocks_match_the_design() {
    let t = taxonomy();
    assert_eq!(NUMBER_TABLE.len(), 27);
    for (key, number, flags, currency) in NUMBER_TABLE {
        let def = t
            .get(key)
            .unwrap_or_else(|| panic!("{key} not in taxonomy"));
        let expected = Display {
            number: Some((*number).to_string()),
            money: flags.contains(&"money"),
            fraction: flags.contains(&"fraction"),
            bytes: flags.contains(&"bytes"),
            currency: currency.map(str::to_string),
            ..Display::default()
        };
        assert_eq!(def.display.as_ref(), Some(&expected), "{key}");
    }
}

#[test]
fn every_number_definition_is_in_the_table() {
    let t = taxonomy();
    let missing: Vec<&String> = t
        .definitions()
        .filter(|(k, d)| display_kind_for(k, d) == Some(DisplayKind::Number))
        .map(|(k, _)| k)
        .filter(|k| !NUMBER_TABLE.iter().any(|(tk, ..)| tk == k))
        .collect();
    assert!(
        missing.is_empty(),
        "number definitions with no row: {missing:?}"
    );
}
