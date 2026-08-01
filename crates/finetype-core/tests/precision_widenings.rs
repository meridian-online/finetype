//! Regression tests for precision-corpus taxonomy widenings.
//!
//! Spec: `2026-04-28-validate-precision-corpus` (ac-10).
//!
//! This file pins the validators that ac-10 widens. Each widening MUST satisfy
//! MADR 0001 (precision principle): the widened pattern still rejects clearly
//! invalid input while accepting the previously-rejected real-world variants
//! surfaced by the iter-1 baseline. The tests use the cross-test fixture
//! pattern: an `ACCEPT` slice and a `REJECT` slice per widened type, exercised
//! against the live `labels/definitions_*.yaml` taxonomy through
//! `validate_value_for_label` — there is no second source of truth.
//!
//! Iter-1 widenings (≤5 per spec constraint):
//!
//!   1. `representation.numeric.decimal_number` — pattern gains an optional
//!      `[eE][+-]?[0-9]+` suffix so scientific-notation values like `6e-04`
//!      validate alongside plain decimals like `0.081541`. Surfaced by
//!      `us_baby_names.percent` (format_diversity in baseline).
//!
//! Later widenings:
//!
//!   2. `identity.industry.naics` — pattern gains a literal alternation branch
//!      for the three official hyphenated sector ranges (`31-33`, `44-45`,
//!      `48-49`) that Census publishes alongside the plain 2-6 digit codes.
//!      The branch is exhaustive by construction, NOT a general
//!      `(-[0-9]{2})?` suffix, which would admit non-ranges like `11-99`.
//!
//!   3. `finance.securities.lei` — the LOU prefix (characters 1-4) becomes
//!      `[A-Z0-9]` rather than `[0-9]`, which is what ISO 17442 defines it as.
//!      The check digits stay `[0-9]{2}` and the length stays 20, so the
//!      widening is confined to one of the three parts.
//!
//! When future iterations add widenings, append additional `pvc_widening_*`
//! tests in this file. The accept/reject fixtures double as living
//! documentation of the boundary each pattern guards.
//!
//! Run: `cargo test -p finetype-core --test precision_widenings`

use finetype_core::taxonomy::Taxonomy;
use finetype_core::validator::validate_value_for_label;
use std::path::PathBuf;

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (`crates/finetype-core`)
/// by popping two segments. Mirrors the helper in `precise_audit.rs`.
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> crates/
    p.pop(); // -> workspace root
    p
}

/// Load the live taxonomy from `labels/`. Compiles validators so the test
/// path mirrors production (cached `CompiledValidator`).
fn load_taxonomy() -> Taxonomy {
    let root = workspace_root();
    let labels_dir = root.join("labels");
    let mut tax = Taxonomy::from_directory(&labels_dir)
        .expect("load taxonomy from labels/ — is labels/ present in workspace?");
    tax.compile_validators();
    tax
}

/// Assert every value in `accept` validates against `label`, and every value
/// in `reject` does NOT. Failure messages name the offending value so the
/// `cargo test` output diagnoses the boundary that drifted.
fn assert_pattern_boundary(label: &str, accept: &[&str], reject: &[&str]) {
    let tax = load_taxonomy();

    for value in accept {
        let result = validate_value_for_label(value, label, &tax).unwrap_or_else(|e| {
            panic!("validate_value_for_label({label:?}, {value:?}) errored: {e:?}")
        });
        assert!(
            result.is_valid,
            "ACCEPT-set regression: value {:?} should validate against {} but failed with {:?}",
            value, label, result.errors
        );
    }

    for value in reject {
        let result = validate_value_for_label(value, label, &tax).unwrap_or_else(|e| {
            panic!("validate_value_for_label({label:?}, {value:?}) errored: {e:?}")
        });
        assert!(
            !result.is_valid,
            "REJECT-set regression: value {:?} should NOT validate against {} but passed — \
             precision principle (MADR 0001) violated, the widened pattern accepts garbage",
            value, label
        );
    }
}

/// ac-10 widening 1 — `representation.numeric.decimal_number` accepts
/// scientific-notation suffix.
///
/// ACCEPT-set: a mix of plain decimals (the prior contract) and the
/// scientific-notation variants that previously rejected. Includes both
/// `e` and `E`, both `+` and `-` exponent signs and unsigned, and both
/// integer- and decimal-mantissa cases.
///
/// REJECT-set: the textbook precision-violators — non-numeric tokens,
/// multi-dot strings, units glued onto numbers, mantissa-without-exponent
/// (`1e`), exponent-without-mantissa (`e5`), and trailing/leading garbage.
/// If any of these slip through, the regex has overshot.
#[test]
fn pvc_widening_decimal_number_accepts_scientific_notation() {
    assert_pattern_boundary(
        "representation.numeric.decimal_number",
        // ACCEPT
        &[
            // Pre-widening contract — must stay green.
            "3.14", "-0.5", "1000.001", "0.0001", "0", "-42", "0.081541",
            // Post-widening additions (the format-diversity gap).
            "6e-04", "6E-04", "1.5e10", "1.5E+10", "-2.3e+5", "-2.3E-5", "1e0", "1E0", "0e0",
            "1.0e-10",
        ],
        // REJECT
        &[
            "abc",     // non-numeric
            "",        // empty
            "1.2.3",   // multiple decimal points
            "12px",    // unit-suffixed
            "1e",      // mantissa with no exponent digits
            "e5",      // exponent with no mantissa
            "1e+",     // dangling exponent sign
            "1.5ee10", // double-e
            "--3",     // double sign
            "+3",      // explicit plus on mantissa is not in spec pattern
            "1.5e1.5", // fractional exponent
            " 3.14",   // leading whitespace
            "3.14 ",   // trailing whitespace
            "3,14",    // comma decimal (that's decimal_number_comma)
        ],
    );
}

/// Widening 2 — `identity.industry.naics` accepts the official hyphenated
/// sector ranges.
///
/// Census publishes exactly three sector ranges as hyphenated pairs: 31-33
/// (Manufacturing), 44-45 (Retail Trade) and 48-49 (Transportation and
/// Warehousing). They are real published NAICS sector codes and appear
/// verbatim in reference data, but the pre-widening pattern only admitted
/// sector-prefixed digit runs, so all three rejected.
///
/// ACCEPT-set: the three ranges, plus the prior contract (2-digit sector,
/// 4-digit industry group, 6-digit national industry) which must stay green.
///
/// REJECT-set: the guard against a sloppy widening. A general
/// `(-[0-9]{2})?` suffix would have accepted `11-99` — a digit pair that
/// looks like a range but is not one. The rest are the textbook
/// precision-violators: a dangling hyphen on either side, a range built from
/// non-sector operands, an over-long digit run, and a spaced form.
#[test]
fn pvc_widening_naics_accepts_hyphenated_sector_ranges() {
    assert_pattern_boundary(
        "identity.industry.naics",
        // ACCEPT
        &[
            // Pre-widening contract — must stay green: 2-digit sector,
            // 4-digit industry group, 6-digit national industry.
            "11", "2131", "541511", "92",
            // Post-widening additions — the three official sector ranges.
            "31-33", "44-45", "48-49",
        ],
        // REJECT
        &[
            "11-99",    // not a published sector range; the general-suffix trap
            "31-",      // dangling hyphen, no right operand
            "-33",      // dangling hyphen, no left operand
            "3133-44",  // range operands must be bare sectors
            "12345678", // longer than a 6-digit national industry
            "31 - 33",  // spaced form is not the published notation
        ],
    );
}

/// Widening 3 — `finance.securities.lei` accepts an alphanumeric LOU prefix.
///
/// ISO 17442 makes characters 1-4 of an LEI the LOU (Local Operating Unit)
/// prefix, and alphanumeric. The pre-widening pattern
/// `^[0-9]{4}[A-Z0-9]{14}[0-9]{2}$` required four digits there, so a
/// letter-prefixed LEI failed validation while passing the ISO 7064 check
/// digits the same taxonomy entry's `checksum:` directive names.
///
/// ACCEPT-set: the prior contract (digit-prefixed) plus values whose first
/// four characters are not all digits. Each carries valid ISO 7064 check
/// digits, asserted alongside the pattern below so a value that is merely
/// LEI-shaped cannot stand in for one that is an LEI.
///
/// REJECT-set: the boundary the widening must not overshoot — the check
/// digits stay numeric, the length stays 20, and the alphabet stays uppercase
/// ASCII. The alphabet cases are split by where the defect sits. Some put it
/// outside characters 1-4, where the segments this widening left alone reject
/// it; some put it inside, where the widened segment is what rejects it.
/// Without that second group `^.{4}[A-Z0-9]{14}[0-9]{2}$` passes this test.
#[test]
fn pvc_widening_lei_accepts_alphanumeric_lou_prefix() {
    let accept = [
        // Pre-widening contract — digit-prefixed, must stay green.
        "529900T8BM49AURSDO55",
        "213800WSGIIZCXF1P572",
        "549300MLUDYVRQOOXS22",
        // Post-widening additions — characters 1-4 are not all digits.
        "HWUPKR0MPOU8FGXBT394",
        "001GPB6A9XPE8XJICC14",
        "00EHHQ2ZHDCFXJCPCL46",
    ];

    let lei_checksum =
        finetype_core::checksum::resolve("lei").expect("the `lei` checksum scheme resolves");
    for value in &accept {
        assert!(
            lei_checksum(value),
            "ACCEPT-set value {value:?} does not carry valid ISO 7064 check digits, so it \
             cannot stand as evidence that the pattern accepts real LEIs"
        );
    }

    assert_pattern_boundary(
        "finance.securities.lei",
        &accept,
        // REJECT
        &[
            "",                      // empty
            "529900T8BM49AURSDO5",   // 19 characters
            "529900T8BM49AURSDO555", // 21 characters
            "HWUPKR0MPOU8FGXBT39A",  // check digits are numeric, positions 19-20
            "hwupkr0mpou8fgxbt394",  // lower case is not the LEI alphabet
            "HWUP-R0MPOU8FGXBT394",  // separators are not in the alphabet
            "HWUPKR0MPOU8FGXBT 94",  // embedded space
            "HWUPKR0MPOU8FGXBTé94",  // non-ASCII
            // Defect confined to characters 1-4. From character 5 onward each
            // of these matches the segments this widening left alone, so the
            // LOU prefix — the segment it moved — is what rejects them.
            "hWUPKR0MPOU8FGXBT394", // lower case at character 1
            "HW-PKR0MPOU8FGXBT394", // separator at character 3
            "HWU KR0MPOU8FGXBT394", // space at character 4
            "@#$%KR0MPOU8FGXBT394", // punctuation across characters 1-4
        ],
    );
}

/// Widening 3, measured over the registry slice this repository already ships:
/// `eval/datasets/gold_external/gleif_entities.csv`, column `lei`.
///
/// The counts are asserted rather than narrated, so a revert of the pattern or
/// a drift in the fixture reddens here:
///
///   - 200 rows;
///   - 171 of them have an all-digit LOU prefix, which is the subset the
///     pre-widening pattern `^[0-9]{4}[A-Z0-9]{14}[0-9]{2}$` could match;
///   - 200 validate against the widened pattern;
///   - 2 named values fail their own ISO 7064 check digits, which is what
///     keeps pattern (shape) and checksum (substance) distinguishable here.
#[test]
fn pvc_widening_lei_validates_the_committed_registry_slice() {
    let path = workspace_root().join("eval/datasets/gold_external/gleif_entities.csv");
    let csv =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    let mut lines = csv.lines();
    let header = lines.next().expect("the slice has a header row");
    assert_eq!(
        header.split(',').next(),
        Some("lei"),
        "`lei` is the first column of {}; this test reads field 0",
        path.display()
    );
    // Field 0 only, so a quoted `name` containing a comma cannot shift it.
    let leis: Vec<&str> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split(',').next().expect("a row has a first field"))
        .collect();

    assert_eq!(leis.len(), 200, "row count of the committed slice");

    let all_digit_prefix = leis
        .iter()
        .filter(|v| v.len() >= 4 && v.as_bytes()[..4].iter().all(u8::is_ascii_digit))
        .count();
    assert_eq!(
        all_digit_prefix,
        171,
        "rows the pre-widening pattern could match; the other {} are what this widening buys",
        leis.len() - all_digit_prefix
    );

    let tax = load_taxonomy();
    let invalid: Vec<&&str> = leis
        .iter()
        .filter(|v| {
            !validate_value_for_label(v, "finance.securities.lei", &tax)
                .expect("the lei leaf exists")
                .is_valid
        })
        .collect();
    assert!(
        invalid.is_empty(),
        "{} of {} rows fail the widened pattern, e.g. {:?}",
        invalid.len(),
        leis.len(),
        invalid.first()
    );

    let lei_checksum =
        finetype_core::checksum::resolve("lei").expect("the `lei` checksum scheme resolves");
    let checksum_failures: Vec<&str> = leis.iter().copied().filter(|v| !lei_checksum(v)).collect();
    assert_eq!(
        checksum_failures,
        ["0292001629A3Q7XJ0D13", "0292001684F9TE5J9417"],
        "the pattern is shape and the checksum is substance: these rows of {} match the \
         widened pattern and still fail ISO 7064",
        path.display()
    );
}
