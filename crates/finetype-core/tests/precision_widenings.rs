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

/// Widening 3, measured over the internal eval corpus:
/// `eval/datasets/csv/finance_coverage.csv`, column `lei`.
///
/// This test tracks that all values in the eval corpus's `lei` column pass
/// validation, so a later drift in the fixture reddens here:
///
///   - 25 rows have a non-empty lei value;
///   - All 25 values pass LEI validation (pattern and checksum).
#[test]
fn pvc_finance_coverage_lei_validates() {
    let path = workspace_root().join("eval/datasets/csv/finance_coverage.csv");
    let csv =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));

    let mut lines = csv.lines();
    let header = lines.next().expect("the file has a header row");

    // Locate lei column
    let headers: Vec<&str> = header.split(',').collect();
    let lei_idx = headers
        .iter()
        .position(|&h| h == "lei")
        .unwrap_or_else(|| panic!("lei column exists in {}", path.display()));

    // Collect non-empty lei values
    let leis: Vec<&str> = lines
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| {
            let fields: Vec<&str> = l.split(',').collect();
            let lei = fields.get(lei_idx).copied().unwrap_or("").trim();
            if !lei.is_empty() {
                Some(lei)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        leis.len(),
        25,
        "lei column has 25 rows in {}; if this drifts, the fixture has been changed",
        path.display()
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
        "{} of {} rows in {} fail LEI validation, e.g. {:?}",
        invalid.len(),
        leis.len(),
        path.display(),
        invalid.first()
    );
}

const UUID_LABEL: &str = "representation.identifier.uuid";
const UUID_ACCEPT: &[&str] = &[
    "550e8400-e29b-41d4-a716-446655440000",
    "550E8400-E29B-41D4-A716-446655440000",
    "550e8400-E29B-41d4-a716-446655440000",
];
const UUID_REJECT: &[&str] = &[
    "550e8400e29b41d4a716446655440000",
    "550e8400-e29b-41d4-a716-44665544000g",
];

/// RFC 9562: the hex digits of the text form may be upper, lower or mixed case.
#[test]
fn uuid_accepts_source_forms() {
    assert_pattern_boundary(UUID_LABEL, UUID_ACCEPT, &[]);
}

#[test]
fn uuid_rejects_undefined_forms() {
    assert_pattern_boundary(UUID_LABEL, &[], UUID_REJECT);
}

/// Lowercase is the preferred form; the other cases are accepted with a note.
#[test]
fn uuid_grades_uppercase_and_mixed_case_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    assert_eq!(
        grade_value_for_label(UUID_ACCEPT[0], UUID_LABEL, &tax),
        Grade::Conforms
    );
    for value in &UUID_ACCEPT[1..] {
        match grade_value_for_label(value, UUID_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::NonCanonical, "{value}");
                assert_eq!(note.detail, "lowercase", "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}

const PERCENTAGE_LABEL: &str = "representation.numeric.percentage";
const PERCENTAGE_ACCEPT: &[&str] = &[
    "50%",
    "50 %",
    "50\u{00A0}%",
    "50\u{202F}%",
    "%50",
    "-12.5%",
    "12.5",
    "%-12.5",
    "-12.5 %",
];
const PERCENTAGE_REJECT: &[&str] = &["50%%", "%50%", "abc%", "50 % %"];

/// SI Brochure, French, German and Turkish typography: a space (ordinary,
/// no-break or narrow no-break) may precede the sign, or the sign may lead.
#[test]
fn percentage_accepts_source_forms() {
    assert_pattern_boundary(PERCENTAGE_LABEL, PERCENTAGE_ACCEPT, &[]);
}

#[test]
fn percentage_rejects_undefined_forms() {
    assert_pattern_boundary(PERCENTAGE_LABEL, &[], PERCENTAGE_REJECT);
}

/// The two non-ASCII spaces reach the published JSON Schema as literal characters.
#[test]
fn percentage_json_schema_pattern_has_literal_spaces() {
    let tax = load_taxonomy();
    let def = tax.get(PERCENTAGE_LABEL).expect("percentage defined");
    let schema = finetype_core::json_schema::emit_type_schema(PERCENTAGE_LABEL, def);
    let pattern = schema["pattern"].as_str().expect("pattern in schema");
    assert!(pattern.contains('\u{00A0}'), "{pattern:?}");
    assert!(pattern.contains('\u{202F}'), "{pattern:?}");
    assert!(!pattern.contains("\\u00A0"), "{pattern:?}");
}

const FILE_SIZE_LABEL: &str = "representation.file.file_size";
const FILE_SIZE_ACCEPT: &[&str] = &[
    "512", "1.5 kB", "1.5 KB", "2 TB", "3 PB", "4KiB", "3 GiB", "2 TiB", "1 PiB", "10M", "10 K",
    "7G", "2T", "1P", "100 B",
];
const FILE_SIZE_REJECT: &[&str] = &["1.5 XB", "MB", "5 Mb", "5 mb", "3 gib", "1 KIB", "2 EB"];

/// NIST/IEC and GNU coreutils: B-suffixed units, the `iB` units and the bare
/// size letters are all defined, with an optional space after the number.
#[test]
fn file_size_accepts_source_forms() {
    assert_pattern_boundary(FILE_SIZE_LABEL, FILE_SIZE_ACCEPT, &[]);
}

#[test]
fn file_size_rejects_undefined_forms() {
    assert_pattern_boundary(FILE_SIZE_LABEL, &[], FILE_SIZE_REJECT);
}

const LOCALE_CODE_LABEL: &str = "technology.code.locale_code";
const LOCALE_CODE_ACCEPT: &[&str] = &[
    "en",
    "en-US",
    "es-419",
    "zh-Hant-TW",
    "pt_BR",
    "eng",
    "zh-Hant",
    "EN-us",
    "zh-yue",
    "zh-cmn-Hans-CN",
];
const LOCALE_CODE_REJECT: &[&str] = &[
    "english",
    "e",
    "en-US-x",
    "en-419-US",
    "en-12",
    "en-Latn-Latn",
    "en-US-US",
];
/// Well-formed RFC 5646 variant subtags: variants are out of this type's scope.
const LOCALE_CODE_OUT_OF_SCOPE: &[&str] = &["en-1234", "de-1996"];

/// RFC 5646: a language subtag, an optional script subtag, an optional region
/// subtag (two letters or the three-digit UN M.49 code), joined by hyphens.
#[test]
fn locale_code_accepts_source_forms() {
    assert_pattern_boundary(LOCALE_CODE_LABEL, LOCALE_CODE_ACCEPT, &[]);
}

#[test]
fn locale_code_rejects_undefined_forms() {
    assert_pattern_boundary(LOCALE_CODE_LABEL, &[], LOCALE_CODE_REJECT);
}

/// The extended-language slot is checked for shape only, as the old rule's
/// second subtag was: membership is the substance guard's job.
#[test]
fn locale_code_extlang_slot_is_shape_only() {
    assert_pattern_boundary(LOCALE_CODE_LABEL, &["en-USA", "foo-bar"], &[]);
}

#[test]
fn locale_code_rejects_variant_subtags_as_out_of_scope() {
    assert_pattern_boundary(LOCALE_CODE_LABEL, &[], LOCALE_CODE_OUT_OF_SCOPE);
}

/// The hyphen is RFC 5646's separator; the underscore is the POSIX/Java form
/// and is accepted with a note.
#[test]
fn locale_code_grades_underscore_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in ["en", "en-US", "es-419", "zh-Hant-TW"] {
        assert_eq!(
            grade_value_for_label(value, LOCALE_CODE_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    match grade_value_for_label("pt_BR", LOCALE_CODE_LABEL, &tax) {
        Grade::Noted(note) => {
            assert_eq!(note.kind, NoteKind::Nonstandard);
            assert_eq!(note.detail, "hyphen separator");
        }
        other => panic!("pt_BR should be noted, got {other:?}"),
    }
}

const ABBREVIATED_MONTH_LABEL: &str = "datetime.date.abbreviated_month";
const ABBREVIATED_MONTH_ACCEPT: &[&str] = &[
    "Feb 4, 2023",
    "4 Feb 2023",
    "4 Feb 23",
    "Sept 4, 2023",
    "Feb. 4, 2023",
    "15 ene 2024",
    "15 Mai 2024",
    "4 janv. 2023",
    "12 d\u{e9}c. 2023",
    "4 f\u{e9}vr. 2023",
    "4 ao\u{fb}t 2023",
];
// `Febr 4 2023` is not rejected: the month token stays letter-based so other
// languages work, and a four-letter token (German `Juni`) cannot be told apart.
const ABBREVIATED_MONTH_REJECT: &[&str] = &["4 February 23x", "4 Fe 2023", "Feb 32x"];
/// Unicode CLDR en (Jan..Dec), en-GB (Sept), and the US military form with a
/// full stop after the month; a two-digit year in the day-first form.
#[test]
fn abbreviated_month_accepts_source_forms() {
    assert_pattern_boundary(ABBREVIATED_MONTH_LABEL, ABBREVIATED_MONTH_ACCEPT, &[]);
}

#[test]
fn abbreviated_month_rejects_undefined_forms() {
    assert_pattern_boundary(ABBREVIATED_MONTH_LABEL, &[], ABBREVIATED_MONTH_REJECT);
}

const RFC_3339_LABEL: &str = "datetime.timestamp.rfc_3339";
// RFC 3339 section 5.6: a numeric offset is a sign, two-digit hours, a colon and
// two-digit minutes; the fraction of a second is optional; section 5.6 also
// permits lower-case `t` and `z`.
const RFC_3339_ACCEPT: &[&str] = &[
    "2024-01-01 10:00:00Z",
    "2024-01-01 10:00:00.123+05:30",
    "2024-01-01 10:00:00z",
    "2024-01-01 10:00:00+05:30",
    "2024-01-01 10:00:00+0530",
    "2024-01-01 10:00:00 +0000",
    "2024-01-01 10:00:00 Z",
];
const RFC_3339_REJECT: &[&str] = &[
    "2024-01-01 10:00:00+5:30",
    "2024-01-01 10:00:00",
    "2024-01-01T10:00:00Z",
    "2024-01-01 10:00:00.+05:30",
];

#[test]
fn rfc_3339_accepts_source_forms() {
    assert_pattern_boundary(RFC_3339_LABEL, RFC_3339_ACCEPT, &[]);
}

#[test]
fn rfc_3339_rejects_undefined_forms() {
    assert_pattern_boundary(RFC_3339_LABEL, &[], RFC_3339_REJECT);
}

#[test]
fn rfc_3339_grades_offset_forms() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in [
        "2024-01-01 10:00:00Z",
        "2024-01-01 10:00:00.123+05:30",
        "2024-01-01 10:00:00z",
        "2024-01-01 10:00:00+05:30",
    ] {
        assert_eq!(
            grade_value_for_label(value, RFC_3339_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    for value in [
        "2024-01-01 10:00:00 +0000",
        "2024-01-01 10:00:00 Z",
        "2024-01-01 10:00:00+0530",
    ] {
        match grade_value_for_label(value, RFC_3339_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::Nonstandard, "{value}");
                assert_eq!(note.detail, "RFC 3339 offset", "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}

const ISO_8601_OFFSET_LABEL: &str = "datetime.timestamp.iso_8601_offset";
// ISO 8601 writes the offset as a sign, two-digit hours and optional minutes,
// with or without a colon; lower-case `t` is accepted.
const ISO_8601_OFFSET_ACCEPT: &[&str] = &[
    "2024-01-01T10:00:00+05:30",
    "2024-01-01T10:00:00+0530",
    "2024-01-01T10:00:00+05",
    "2024-01-01t10:00:00+05:30",
    "2024-01-01T10:00:00+5:30",
    "2024-01-01T10:00:00+0:53",
    "2024-01-01T10:00:00-08:00",
];
const ISO_8601_OFFSET_REJECT: &[&str] = &[
    "2024-01-01T10:00:00",
    "2024-01-01T10:00:00Z",
    "2024-01-01 10:00:00+05:30",
    "2024-01-01T10:00:00+123:45",
    "2024-01-01T10:00:00+05:3",
    "2024-01-01T10:00:00+053",
    "2024-01-01T10:00:00+530",
    "2024-01-01T10:00:00+5",
];

#[test]
fn iso_8601_offset_accepts_source_forms() {
    assert_pattern_boundary(ISO_8601_OFFSET_LABEL, ISO_8601_OFFSET_ACCEPT, &[]);
}

#[test]
fn iso_8601_offset_rejects_undefined_forms() {
    assert_pattern_boundary(ISO_8601_OFFSET_LABEL, &[], ISO_8601_OFFSET_REJECT);
}

#[test]
fn iso_8601_offset_grades_noncanonical_offsets_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in [
        "2024-01-01T10:00:00+05:30",
        "2024-01-01T10:00:00+05",
        "2024-01-01t10:00:00-08:00",
    ] {
        assert_eq!(
            grade_value_for_label(value, ISO_8601_OFFSET_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    for value in ["2024-01-01T10:00:00+5:30", "2024-01-01T10:00:00+0530"] {
        match grade_value_for_label(value, ISO_8601_OFFSET_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::Nonstandard, "{value}");
                assert_eq!(note.detail, "extended-format offset", "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}

// ── finance.currency.currency_code (ISO 4217, SIX lists one and three) ──

const CURRENCY_CODE_LABEL: &str = "finance.currency.currency_code";
const CURRENCY_CODE_ACCEPT: &[&str] = &["USD", "EUR", "XCG", "HRK", "BGN", "ZWL", "DEM"];
const CURRENCY_CODE_REJECT: &[&str] = &["ABC", "US", "USDD", "UDP", "TCP"];

#[test]
fn currency_code_accepts_source_forms() {
    assert_pattern_boundary(CURRENCY_CODE_LABEL, CURRENCY_CODE_ACCEPT, &[]);
}

#[test]
fn currency_code_rejects_undefined_forms() {
    assert_pattern_boundary(CURRENCY_CODE_LABEL, &[], CURRENCY_CODE_REJECT);
}

/// List-one codes conform; withdrawn codes validate with a note carrying
/// SIX's withdrawal month.
#[test]
fn currency_code_grades_withdrawn_codes_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in ["USD", "EUR", "XCG"] {
        assert_eq!(
            grade_value_for_label(value, CURRENCY_CODE_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    for (value, detail) in [
        ("HRK", "withdrawn 2023-01"),
        ("BGN", "withdrawn 2026-01"),
        ("ZWL", "withdrawn 2024-09"),
        ("DEM", "withdrawn 2002-03"),
    ] {
        match grade_value_for_label(value, CURRENCY_CODE_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::Withdrawn, "{value}");
                assert_eq!(note.detail, detail, "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}
