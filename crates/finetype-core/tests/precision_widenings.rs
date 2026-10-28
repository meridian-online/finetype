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
    "4 \u{d6}kt 2023",
];
// `Febr 4 2023` is not rejected: the month token stays letter-based so other
// languages work, and a four-letter token (German `Juni`) cannot be told apart.
// The multiplication and division signs sit inside Latin-1's letter block and are
// not letters.
const ABBREVIATED_MONTH_REJECT: &[&str] = &[
    "4 February 23x",
    "4 Fe 2023",
    "Feb 32x",
    "4 F\u{d7}b 2023",
    "4 F\u{f7}b 2023",
];
/// Unicode CLDR en (Jan..Dec), en-GB (Sept), and the dotted AP-style form with a
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

const USERNAME_LABEL: &str = "identity.person.username";

/// useradd(8): a name may end with `$` and may be up to 256 characters.
#[test]
fn username_accepts_source_forms() {
    let github = "a".repeat(39);
    let longest = "a".repeat(256);
    assert_pattern_boundary(
        USERNAME_LABEL,
        &["alice", "bob.smith", "svc$", &github, &longest],
        &[],
    );
}

#[test]
fn username_rejects_undefined_forms() {
    let too_long = "a".repeat(257);
    assert_pattern_boundary(
        USERNAME_LABEL,
        &[],
        &["ab", "al ice", "$alice", "a$b", &too_long],
    );
}

const BLOOD_TYPE_LABEL: &str = "identity.person.blood_type";

/// Rh nomenclature writes the negative type with a minus sign (U+2212) as well
/// as the ASCII hyphen.
#[test]
fn blood_type_accepts_source_forms() {
    assert_pattern_boundary(
        BLOOD_TYPE_LABEL,
        &["A+", "O-", "AB\u{2212}", "B\u{2212}", "AB", "O+"],
        &[],
    );
}

#[test]
fn blood_type_rejects_undefined_forms() {
    assert_pattern_boundary(BLOOD_TYPE_LABEL, &[], &["C+", "A++", "A\u{2013}"]);
}

#[test]
fn blood_type_json_schema_pattern_has_literal_minus_sign() {
    let tax = load_taxonomy();
    let def = tax.get(BLOOD_TYPE_LABEL).expect("blood_type defined");
    let schema = finetype_core::json_schema::emit_type_schema(BLOOD_TYPE_LABEL, def);
    let pattern = schema["pattern"].as_str().expect("pattern in schema");
    assert!(pattern.contains('\u{2212}'), "{pattern:?}");
    assert!(!pattern.contains("\\u2212"), "{pattern:?}");
}

const NPI_LABEL: &str = "identity.medical.npi";

/// A 10-digit number starting with `3` whose check digit passes the CMS
/// algorithm (Luhn over the 80840-prefixed number).
fn npi_valid_starting_with_3() -> String {
    (0..10)
        .map(|d| format!("300000123{d}"))
        .find(|v| finetype_core::checksum::npi(v))
        .expect("one check digit completes the base")
}

/// The CMS check-digit document: the published rule is ten digits plus the
/// check digit; first digits other than 1 or 2 are reserved for coordinated use.
#[test]
fn npi_accepts_source_forms() {
    let other_first_digit = npi_valid_starting_with_3();
    assert_pattern_boundary(NPI_LABEL, &["1234567893", &other_first_digit], &[]);
}

#[test]
fn npi_rejects_undefined_forms() {
    // `1234567890` has a bad check digit, which the checksum (not the pattern)
    // rejects; the pattern-level reject is the wrong length.
    assert_pattern_boundary(NPI_LABEL, &[], &["123456789", "12345678931", "12345678a3"]);
}

#[test]
fn npi_grades_other_first_digits_as_noted_and_bad_check_digit_as_failing() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    assert_eq!(
        grade_value_for_label("1234567893", NPI_LABEL, &tax),
        Grade::Conforms
    );
    match grade_value_for_label(&npi_valid_starting_with_3(), NPI_LABEL, &tax) {
        Grade::Noted(note) => {
            assert_eq!(note.kind, NoteKind::NonCanonical);
            assert_eq!(note.detail, "issued range (first digit 1 or 2)");
        }
        other => panic!("should be noted, got {other:?}"),
    }
    match grade_value_for_label("1234567890", NPI_LABEL, &tax) {
        Grade::Fails { constraint, .. } => assert_eq!(constraint, "checksum"),
        other => panic!("bad check digit should fail, got {other:?}"),
    }
}

#[test]
fn npi_detector_requires_issued_range_and_check_digit() {
    let detect = finetype_core::discrimination::detector_for(NPI_LABEL).expect("npi detector");
    assert!(detect("1234567893"));
    assert!(!detect(&npi_valid_starting_with_3()));
    assert!(!detect("1234567890"));
    assert!(!detect("123456789"));
}

const JSON_LABEL: &str = "container.object.json";

/// RFC 8259: insignificant whitespace, line breaks included, may surround and
/// sit inside the object.
#[test]
fn json_accepts_source_forms() {
    assert_pattern_boundary(
        JSON_LABEL,
        &["{\"a\": 1}", " {\"a\": 1} ", "{\n  \"a\": 1\n}"],
        &[],
    );
}

#[test]
fn json_rejects_undefined_forms() {
    assert_pattern_boundary(JSON_LABEL, &[], &["[1, 2]", "{\"a\": 1", "a {}"]);
}

const XML_LABEL: &str = "container.object.xml";

/// XML 1.0: an optional declaration, Misc (comments, processing instructions,
/// whitespace), an optional DOCTYPE with an internal subset, one root element
/// (an empty-element tag, or a start tag through its end tag, across lines),
/// then trailing Misc.
#[test]
fn xml_accepts_source_forms() {
    assert_pattern_boundary(
        XML_LABEL,
        &[
            "<root/>",
            "<br />",
            "<?xml version=\"1.0\"?><a>1</a>",
            "<a>\n  <b>1</b>\n</a>",
            "<?xml version=\"1.0\"?>\n<a>\n  <b>1</b>\n</a>",
            "<gr\u{f6}\u{df}e>1</gr\u{f6}\u{df}e>",
            "<!DOCTYPE note><note>x</note>",
            "<!DOCTYPE note [<!ENTITY e \"v\">]>\n<note>&e;</note>",
            "<?xml version=\"1.0\"?><!-- generated --><a>1</a>",
            "<a/><!-- end -->",
            "<a>1</a>\n<?pi data?>\n",
        ],
        &[],
    );
}

#[test]
fn xml_rejects_undefined_forms() {
    assert_pattern_boundary(XML_LABEL, &[], &["<a>1", "a <b/>", "<>", "<1a/>"]);
}

/// The detector keeps HTML documents out of discrimination: they conform to
/// the XML rule (XHTML is XML) but must not count as xml.
#[test]
fn xml_detector_keeps_out_html() {
    let detect = finetype_core::discrimination::detector_for(XML_LABEL).expect("xml detector");
    for bad in [
        "<!DOCTYPE html><html><body>x</body></html>",
        "<html>\n<body>x</body>\n</html>",
        "<!doctype HTML>\n<HTML lang=\"en\"><body>x</body></HTML>",
    ] {
        assert!(!detect(bad), "{bad:?}");
    }
    // A DOCTYPE with an internal subset is skipped to find the root.
    assert!(!detect(
        "<!DOCTYPE x [<!ENTITY e \"v\">]><html><b>1</b></html>"
    ));
    for ok in [
        "<a>1</a>",
        "<?xml version=\"1.0\"?><a>1</a>",
        "<!DOCTYPE x [<!ENTITY e \"v\">]><a>1</a>",
    ] {
        assert!(detect(ok), "{ok:?}");
    }
}

/// The newly accepted forms conform but do not count for discrimination, so
/// multi-line HTML fragments and empty-element tags are not promoted.
#[test]
fn xml_detector_keeps_the_old_single_line_shape() {
    let detect = finetype_core::discrimination::detector_for(XML_LABEL).expect("xml detector");
    for new_form in [
        "<root/>",
        "<a>\n  <b>1</b>\n</a>",
        "<p>Hello</p>\n<p>World</p>",
        "<a/><!-- end -->",
    ] {
        assert!(!detect(new_form), "{new_form:?}");
    }
}

/// Newly rejected, source-required: XML 1.0 is case-sensitive about
/// `<!DOCTYPE`, a root must be an element, and an end tag is `</Name S?>`.
#[test]
fn xml_rejects_forms_the_source_forbids() {
    assert_pattern_boundary(
        XML_LABEL,
        &[],
        &[
            "<!doctype note><a>1</a>",
            "<![CDATA[x]]><a>1</a>",
            // U+10000 in a name: rejected by the ECMA-262 ruling (no supplementary-plane name characters).
            "<a\u{10000}>1</a\u{10000}>",
            "<a>x</a b>",
            "<a>x</ a>",
            "<>x</>",
        ],
    );
}

const YAML_LABEL: &str = "container.object.yaml";

/// YAML 1.2.2: a document may open with `---`, a comment line, or a top-level
/// sequence; keys may contain `-` and `.`.
#[test]
fn yaml_accepts_source_forms() {
    assert_pattern_boundary(
        YAML_LABEL,
        &[
            "---\nname: x",
            "# c\nname: x",
            "- a\n- b",
            "my-key.v2: 1",
            "name: John\nage: 30",
        ],
        &[],
    );
}

#[test]
fn yaml_rejects_undefined_forms() {
    assert_pattern_boundary(
        YAML_LABEL,
        &[],
        &["#hashtag", "- just a bullet", "hello world", ": x"],
    );
}

/// The detector keeps the old `key:` first-line shape, so the newly accepted
/// forms conform in grading but do not count for discrimination.
#[test]
fn container_object_yaml_detector_keeps_old_shape() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(YAML_LABEL, &tax).expect("yaml rule");
    let detector = detector_for(YAML_LABEL);
    assert!(detector.is_some());
    assert!(passes_detector_or_rule(
        detector,
        &rule,
        "name: John\nage: 30"
    ));
    assert!(passes_detector_or_rule(detector, &rule, "user:"));
    for new_form in ["---\nname: x", "- a\n- b", "my-key.v2: 1", "# c\nname: x"] {
        assert!(rule.is_valid(new_form), "{new_form:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, new_form),
            "{new_form:?}"
        );
    }
}

/// The detector keeps the old pairs-only shape, so the newly accepted forms
/// conform in grading but do not count for discrimination.
#[test]
fn query_string_detector_keeps_old_shape() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(QUERY_STRING_LABEL, &tax).expect("query_string rule");
    let detector = detector_for(QUERY_STRING_LABEL);
    assert!(detector.is_some());
    for old_form in ["name=John&age=30", "q=", "my key=1"] {
        assert!(
            passes_detector_or_rule(detector, &rule, old_form),
            "{old_form:?}"
        );
    }
    for new_form in ["debug&page=2", "a=1&&b=2"] {
        assert!(rule.is_valid(new_form), "{new_form:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, new_form),
            "{new_form:?}"
        );
    }
}

const CSV_LABEL: &str = "container.object.csv";

/// RFC 4180: fields may be empty, and a quoted field may hold commas, doubled
/// quotes and line breaks.
#[test]
fn csv_accepts_source_forms() {
    assert_pattern_boundary(
        CSV_LABEL,
        &[
            "a,,c",
            "\"a,,b\",1",
            "\"say \"\"hi\"\"\",2",
            "John,30,true",
            "\"line\nbreak\",1",
        ],
        &[],
    );
}

#[test]
fn csv_rejects_undefined_forms() {
    assert_pattern_boundary(CSV_LABEL, &[], &["\"unterminated,1", "a\"b,c", "a,b\nc,d"]);
}

/// The detector is the published rule and either the old shape (every field
/// non-empty) or at least one quoted field, so the rule's empty fields do not
/// lift the type's discrimination rate on random text.
#[test]
fn csv_detector_keeps_empty_field_text_out() {
    let detect = finetype_core::discrimination::detector_for(CSV_LABEL).expect("csv detector");
    for ok in [
        "John,30,true",
        "\"a,,b\",1",
        "\"say \"\"hi\"\"\",2",
        "a,,\"c\"",
    ] {
        assert!(detect(ok), "{ok:?}");
    }
    for bad in ["a,,c", ",", "a,", "\"unterminated,1", "a\"b,c", "a,b\nc,d"] {
        assert!(!detect(bad), "{bad:?}");
    }
}

const QUERY_STRING_LABEL: &str = "container.key_value.query_string";

/// WHATWG URL Standard: bare names and empty sequences sit beside at least one
/// `name=value` pair.
#[test]
fn query_string_accepts_source_forms() {
    assert_pattern_boundary(
        QUERY_STRING_LABEL,
        &["debug&page=2", "a=1&&b=2", "q=", "name=John&age=30"],
        &[],
    );
}

#[test]
fn query_string_rejects_undefined_forms() {
    assert_pattern_boundary(
        QUERY_STRING_LABEL,
        &[],
        &[
            "debug",
            "&&",
            "=x",
            "hello world",
            "a == 1 && b == 2",
            "status=='x' && age>18",
            "AT&T price=5",
        ],
    );
}

const STATE_CODE_LABEL: &str = "geography.location.state_code";

/// ISO 3166-2:AU codes are two or three letters; USPS Publication 28
/// Appendix B lists the two-letter codes, military ones included.
#[test]
fn state_code_accepts_source_forms() {
    assert_pattern_boundary(STATE_CODE_LABEL, &["NSW", "ACT", "AE", "TX"], &[]);
}

#[test]
fn state_code_rejects_undefined_forms() {
    assert_pattern_boundary(STATE_CODE_LABEL, &[], &["NSWX", "N", "ny"]);
}

/// Publication 28 Appendix B names the freely associated states and the
/// military "states" beside the fifty states, so the US enum holds them.
#[test]
fn state_code_us_enum_has_the_pub_28_additions() {
    let tax = load_taxonomy();
    let def = tax.get(STATE_CODE_LABEL).expect("state_code");
    let us = def
        .validation_by_locale
        .as_ref()
        .and_then(|m| m.get("EN_US"))
        .and_then(|v| v.enum_values.as_ref())
        .expect("EN_US enum");
    for code in ["FM", "MH", "PW", "AA", "AE", "AP"] {
        assert!(us.iter().any(|c| c == code), "{code} missing from EN_US");
    }
}

/// The detector counts two uppercase letters, or an EN_AU three-letter code,
/// so three-letter airport and currency codes do not count for discrimination.
#[test]
fn state_code_detector_keeps_three_letter_codes_out() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(STATE_CODE_LABEL, &tax).expect("state_code rule");
    let detector = detector_for(STATE_CODE_LABEL);
    assert!(detector.is_some());
    for ok in ["TX", "AE", "NSW", "ACT", "QLD", "TAS", "VIC"] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for bad in ["USD", "JFK", "EUR", "NSWX", "N", "ny", "N1", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const H3_LABEL: &str = "geography.index.h3";

/// H3 docs and Snowflake output: a cell index is 15 hexadecimal digits, and
/// Snowflake writes it in upper case.
#[test]
fn h3_accepts_source_forms() {
    assert_pattern_boundary(H3_LABEL, &["881F1D4887FFFFF", "8928308280fffff"], &[]);
}

#[test]
fn h3_rejects_undefined_forms() {
    assert_pattern_boundary(H3_LABEL, &[], &["8928308280ffff", "8928308280fffffg"]);
}

const WKT_LABEL: &str = "geography.format.wkt";

/// OGC 06-103r4, 7.2.1 and 7.2.2: keywords are case-insensitive, and the
/// triangle, polyhedral-surface and TIN keywords are geometry types.
#[test]
fn wkt_accepts_source_forms() {
    assert_pattern_boundary(
        WKT_LABEL,
        &[
            "Point (10 10)",
            "point(1 2)",
            "TIN Z (((0 0 0, 0 1 0, 1 0 0, 0 0 0)))",
            "TRIANGLE ((0 0, 0 1, 1 0, 0 0))",
            "POLYGON EMPTY",
            "POLYHEDRALSURFACE Z (((0 0 0, 0 1 0, 1 1 0, 0 0 0)))",
        ],
        &[],
    );
}

#[test]
fn wkt_rejects_undefined_forms() {
    assert_pattern_boundary(WKT_LABEL, &[], &["POINTS (1 2)", "circle(1 2)"]);
}

/// The detector counts a keyword in any letter case with `EMPTY` or a coordinate
/// pair, so genuine mixed-case WKT counts and parenthesised text does not.
#[test]
fn wkt_detector_needs_a_coordinate_pair_or_empty() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(WKT_LABEL, &tax).expect("wkt rule");
    let detector = detector_for(WKT_LABEL);
    assert!(detector.is_some());
    for ok in [
        "POINT (30 10)",
        "POINT(30 10)",
        "POINT EMPTY",
        "POINT ZM (1 2 3 4)",
        "MULTIPOLYGON (((30 20, 45 40, 10 40, 30 20)))",
        "GEOMETRYCOLLECTION EMPTY",
        "Point (10 10)",
        "point(1 2)",
        "point(-1.5 2e0)",
        "TIN Z (((0 0 0, 0 1 0, 1 0 0, 0 0 0)))",
        "TRIANGLE ((0 0, 0 1, 1 0, 0 0))",
        "Polygon empty",
        "GEOMETRYCOLLECTION (POINT (1 2))",
        "GEOMETRYCOLLECTION (POINT EMPTY)",
    ] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    // The long s (U+017F) folds to `s` in a Unicode case-insensitive match; the
    // published rule rejects it, so the detector must too.
    assert!(!rule.is_valid("LINE\u{17f}TRING (1 2)"));
    assert!(!passes_detector_or_rule(
        detector,
        &rule,
        "LINE\u{17f}TRING (1 2)"
    ));
    for bad in [
        "Triangle (equilateral)",
        "Triangle (1999)",
        "Point (of sale)",
        "Tin (2 cans)",
        "Polygon (regular)",
        "Tin (small) 250 2pk",
        "Tin (empty)",
        "Point (empty lot)",
        "Tin (empty box)",
    ] {
        assert!(rule.is_valid(bad), "{bad:?} conforms");
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const PLUS_CODE_LABEL: &str = "geography.coordinate.plus_code";

/// Open Location Code specification: input is case-insensitive; a full code is
/// up to 15 digits, padded only before the `+` and in pairs, with nothing after
/// the `+` when padded, at most seven characters after it.
#[test]
fn plus_code_accepts_source_forms() {
    assert_pattern_boundary(
        PLUS_CODE_LABEL,
        &[
            "849VCWC8+R9",
            "8FVC9G8F+",
            "6P000000+",
            "849vcwc8+r9",
            "8FVC9G8F+6WXQRJH",
            "6PH57V00+",
            "6PH50000+",
        ],
        &[],
    );
}

#[test]
fn plus_code_rejects_undefined_forms() {
    assert_pattern_boundary(
        PLUS_CODE_LABEL,
        &[],
        &[
            // short code: no area part
            "CWC8+R9",
            // padded code with digits after the plus
            "7FG49W00+2V",
            // latitude above 90 degrees, longitude above 180 degrees
            "X49VCWC8+R9",
            "8WC8Q2PQ+QV",
            // eight characters after the plus
            "8FVC9G8F+6WXQRJHV",
            // padding must come in pairs and only before the plus
            "8FVC9G0F+",
            "8FVC9G000+",
            "8FVC0G8F+",
            // one character after the plus names no cell
            "849VCWC8+R",
        ],
    );
}

/// The detector counts the form the type was first defined by (upper case,
/// eight characters, a refinement of two or more), so padded and bare-plus
/// codes and lower-case input conform but do not count for discrimination.
#[test]
fn plus_code_detector_counts_the_first_defined_form() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(PLUS_CODE_LABEL, &tax).expect("plus_code rule");
    let detector = detector_for(PLUS_CODE_LABEL);
    assert!(detector.is_some());
    for ok in ["849VCWC8+R9", "87G8Q2PQ+QV", "8FVC9G8F+6WXQRJH"] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in ["8FVC9G8F+", "6P000000+", "849vcwc8+r9", "22000000+"] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["CWC8+R9", "7FG49W00+2V", "X49VCWC8+R9", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const MGRS_LABEL: &str = "geography.coordinate.mgrs";

/// FGDC-STD-011-2001: a grid zone designator (or a UPS band letter), a 100 km
/// square, and an equal-halved even count of digits, up to 16 (1 mm).
#[test]
fn mgrs_accepts_source_forms() {
    assert_pattern_boundary(
        MGRS_LABEL,
        &[
            "18SUJ2348306479",
            "18SUJ2348316806479498",
            "ZAH1234567890",
            "4QFJ1234567890",
            "18SUJ2306",
        ],
        &[],
    );
}

#[test]
fn mgrs_rejects_undefined_forms() {
    assert_pattern_boundary(
        MGRS_LABEL,
        &[],
        &[
            // odd digit count: easting and northing cannot be equal halves
            "18SUJ234",
            "18SUJ2348306",
            // no digits at all
            "18SUJ",
            // I and O are not band letters
            "18IUJ2348306479",
            "18OUJ2348306479",
            // 18 digits: past the 1 mm special application
            "18SUJ123456789012345678",
            // a UPS reference has no zone number
            "1ZAH1234567890",
        ],
    );
}

/// The detector counts a grid zone designator with a band letter and 2 to 10
/// digits, as the type was first defined, so UPS references and 12- to 16-digit
/// references conform but do not count; neither do code-like tokens that share
/// a UPS reference's letters.
#[test]
fn mgrs_detector_counts_the_first_defined_form() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(MGRS_LABEL, &tax).expect("mgrs rule");
    let detector = detector_for(MGRS_LABEL);
    assert!(detector.is_some());
    for ok in [
        "18SUJ2348306479",
        "33UUP0490171890",
        "4QFJ1234567890",
        "18SUJ23",
    ] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in ["ZAH1234567890", "18SUJ2348316806479498", "ABC1234", "BMW20"] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["18SUJ234", "18IUJ2348306479", "18SUJ", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const GEOHASH_LABEL: &str = "geography.coordinate.geohash";

/// PostGIS ST_GeoHash and BigQuery ST_GEOHASH: one to twenty characters of the
/// geohash alphabet.
#[test]
fn geohash_accepts_source_forms() {
    assert_pattern_boundary(
        GEOHASH_LABEL,
        &["dr5r", "9q8yyk8yuvrs4kz5qqvz", "9", "ezs42e"],
        &[],
    );
}

#[test]
fn geohash_rejects_undefined_forms() {
    assert_pattern_boundary(
        GEOHASH_LABEL,
        &[],
        &["9q8yyk8yuvrs4kz5qqvzq", "dr5a", "", "DR5R"],
    );
}

/// The detector counts six to twelve characters, so short tokens and long digit
/// strings do not count for discrimination.
#[test]
fn geohash_detector_counts_the_first_defined_form() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(GEOHASH_LABEL, &tax).expect("geohash rule");
    let detector = detector_for(GEOHASH_LABEL);
    assert!(detector.is_some());
    for ok in ["9q8yyk8yuv", "ezs42e", "dr5ru7c02wnm", "u4pruydqqvj"] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in [
        "dr5r",
        "bed",
        "9",
        "ezs42",
        "9q8yyk8yuvrs4kz5qqvz",
        "1700000000000",
        "4111111111111111",
    ] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["9q8yyk8yuvrs4kz5qqvzq", "dr5a00", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const STREET_SUFFIX_LABEL: &str = "geography.address.street_suffix";

/// UPU country sheets and Canada Post street-type abbreviations: the national
/// forms of the listed locales join the English enum.
#[test]
fn street_suffix_accepts_source_forms() {
    assert_pattern_boundary(
        STREET_SUFFIX_LABEL,
        &[
            "CRT", "LANE", "RUE", "AV", "BOUL", "BVD", "BD", "str", "Str.", "Crescent", "Terrace",
            "Close", "Street", "St", "Lane", "crt", "STR.",
        ],
        &[],
    );
}

#[test]
fn street_suffix_rejects_undefined_forms() {
    assert_pattern_boundary(
        STREET_SUFFIX_LABEL,
        &[],
        &["Strasse Nord", "XYZ", "", "Str.."],
    );
}

/// The detector keeps the English enum and the added full words, so the national
/// abbreviations, which are also country, currency and airport codes, conform
/// without counting.
#[test]
fn street_suffix_detector_keeps_out_abbreviations() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(STREET_SUFFIX_LABEL, &tax).expect("street_suffix rule");
    let detector = detector_for(STREET_SUFFIX_LABEL);
    assert!(detector.is_some());
    for ok in [
        "Street", "St", "ave", "BLVD", "Crescent", "Terrace", "Close", "Rue", "LANE",
    ] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in ["BD", "AV", "CRT", "BVD", "BOUL", "str", "Str."] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["Strasse Nord", "XYZ", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const COORDINATES_LABEL: &str = "geography.coordinate.coordinates";

/// DuckDB spatial and Frictionless Table Schema: a latitude and a longitude
/// separated by a comma (with optional spaces) or by whitespace.
#[test]
fn coordinates_accepts_source_forms() {
    assert_pattern_boundary(
        COORDINATES_LABEL,
        &[
            "48.8761373390584, 2.3680844979417",
            "40.7128,-74.0060",
            "35.6762 139.6917",
            "40.7128 ,-74.0060",
            "40.7128 , -74.0060",
            "35.6762  139.6917",
            "35.6762\t139.6917",
        ],
        &[],
    );
}

#[test]
fn coordinates_rejects_undefined_forms() {
    assert_pattern_boundary(
        COORDINATES_LABEL,
        &[],
        &[
            "40.7128,,-74.0060",
            "N40 44.0 W73 59.0",
            "40.7128",
            "40.7128,",
            "40.7128;-74.0060",
        ],
    );
}

const EMOJI_LABEL: &str = "representation.text.emoji";

/// UTS #51: keycap sequences, a digit, `#` or `*`, an optional U+FE0F and
/// U+20E3, join the symbol class.
#[test]
fn emoji_accepts_source_forms() {
    assert_pattern_boundary(
        EMOJI_LABEL,
        &[
            "1\u{fe0f}\u{20e3}",
            "#\u{fe0f}\u{20e3}",
            "*\u{fe0f}\u{20e3}",
            "1\u{20e3}",
            "\u{1f600}",
            "\u{2764}\u{fe0f}",
            "\u{1f600}1\u{fe0f}\u{20e3}",
        ],
        &[],
    );
}

#[test]
fn emoji_rejects_undefined_forms() {
    assert_pattern_boundary(
        EMOJI_LABEL,
        &[],
        &["1", "#", "a\u{20e3}", "10", "1\u{fe0f}", "12\u{20e3}"],
    );
}

const SMILES_LABEL: &str = "representation.scientific.smiles";

/// OpenSMILES: `*` (the wildcard atom) and `$` (the quadruple bond).
#[test]
fn smiles_accepts_source_forms() {
    assert_pattern_boundary(
        SMILES_LABEL,
        &[
            "[*]CC",
            "*C",
            "[Re]$[Re]",
            "CC(=O)Oc1ccccc1C(=O)O",
            "[Na+].[Cl-]",
            "C\\C=C/C",
        ],
        &[],
    );
}

#[test]
fn smiles_rejects_undefined_forms() {
    assert_pattern_boundary(SMILES_LABEL, &[], &["C C", "C;C", "C,C", "C{C}", ""]);
}

/// The detector keeps the old class, so `*` and `$` forms conform without
/// counting.
#[test]
fn smiles_detector_keeps_old_class() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(SMILES_LABEL, &tax).expect("smiles rule");
    let detector = detector_for(SMILES_LABEL);
    assert!(detector.is_some());
    for ok in ["CCO", "CC(=O)Oc1ccccc1C(=O)O", "[Na+].[Cl-]"] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in ["[*]CC", "*C", "[Re]$[Re]"] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["C C", "C;C", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
    // Over the rule's maxLength of 1000: the detector never widens the rule.
    let long = "C".repeat(1001);
    assert!(!passes_detector_or_rule(detector, &rule, &long));
}

const PROTEIN_LABEL: &str = "representation.scientific.protein_sequence";

/// IUPAC one-letter code and NCBI BLAST: B, Z, U, O and the gap `-` join the
/// 20 standard letters, X and `*`. J is not in the IUPAC table.
#[test]
fn protein_sequence_accepts_source_forms() {
    assert_pattern_boundary(
        PROTEIN_LABEL,
        &["MKVBZU", "MKV-LLI", "MKOVL", "MKVLLIVGS", "MKXV*"],
        &[],
    );
}

#[test]
fn protein_sequence_rejects_undefined_forms() {
    assert_pattern_boundary(
        PROTEIN_LABEL,
        &[],
        &["MKV LLI", "MKV1", "MKVJLI", "mkvlli", ""],
    );
}

/// The detector keeps the old class (the 20 standard letters, X and `*`), so
/// the ambiguity codes, rare residues and gap conform without counting.
#[test]
fn protein_sequence_detector_keeps_old_class() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(PROTEIN_LABEL, &tax).expect("protein rule");
    let detector = detector_for(PROTEIN_LABEL);
    assert!(detector.is_some());
    for ok in ["MKVLLIVGS", "ACDEFGHIKLMNPQRSTVWYXX*"] {
        assert!(passes_detector_or_rule(detector, &rule, ok), "{ok:?}");
    }
    for conforming in ["MKVBZU", "MKV-LLI", "MKOVL"] {
        assert!(rule.is_valid(conforming), "{conforming:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, conforming),
            "{conforming:?}"
        );
    }
    for bad in ["MKV LLI", "MKV1", "MKVJLI", ""] {
        assert!(!passes_detector_or_rule(detector, &rule, bad), "{bad:?}");
    }
}

const COLOR_RGB_LABEL: &str = "representation.format.color_rgb";

/// CSS Color 3 and 4: `rgb(`/`rgba(` with three integers or three percentages
/// and an optional alpha, comma-separated or space-separated with `/ alpha`.
/// The bare and parenthesised triples the old rule took stay.
#[test]
fn color_rgb_accepts_source_forms() {
    assert_pattern_boundary(
        COLOR_RGB_LABEL,
        &[
            "rgba(0, 0, 255, 0.5)",
            "rgba(0,0,255,.5)",
            "rgb(255, 0, 0, 50%)",
            "rgb(100% 0% 0% / 50%)",
            "rgb(255 0 0)",
            "rgb(255 128 0 / 0.5)",
            "rgb(12.5%, 0%, 100%)",
            "rgb(255, 0, 0)",
            "255,0,0",
            "(255, 0, 0)",
            // CSS Color 4 modern syntax: mixing, decimals and none.
            "rgb(100% 0 0)",
            "rgb(127.5 0 0)",
            "rgb(none 0 0)",
            "rgb(0 none 128 / none)",
            // whitespace before a comma; a leading-dot percentage.
            "rgb(255 , 0 , 0)",
            "rgb(1.5, 0, 0)",
            "rgb(.5% 0% 0%)",
        ],
        &[],
    );
}

#[test]
fn color_rgb_rejects_undefined_forms() {
    assert_pattern_boundary(
        COLOR_RGB_LABEL,
        &[],
        &[
            "rgb(255, 0)",
            "(0.1, 0.2, 0.5)",
            "rgb(255 0)",
            "rgb(255 0 0 0.5)",
            "rgb(255, 0, 0 / 0.5)",
            "rgb(255 0 0, 0.5)",
            "rgb(100%, 0, 0)",
            "rgb(none, 0, 0)",
            "rgba(0, 0, 255, )",
            "",
        ],
    );
}

const COLOR_HSL_LABEL: &str = "representation.format.color_hsl";

/// CSS Color 4 (MDN): the modern space-separated syntax with optional
/// `/ alpha`, `deg`, bare-number saturation and lightness, decimals and `none`;
/// the legacy comma syntax keeps percentages.
#[test]
fn color_hsl_accepts_source_forms() {
    assert_pattern_boundary(
        COLOR_HSL_LABEL,
        &[
            "hsl(120deg 75% 25% / 60%)",
            "hsl(540 0 0)",
            "hsl(120.5, 100%, 50%)",
            "hsl(none 100% 50%)",
            "hsl(120 none none / none)",
            "hsla(240, 100%, 50%, 0.5)",
            "hsla(240, 100%, 50%, 50%)",
            "hsl(120 75% 25%)",
            "hsl(.5 .5% .5%)",
            "hsl(120, 100%, 50%)",
            "hsl( 120 , 100% , 50% )",
        ],
        &[],
    );
}

#[test]
fn color_hsl_rejects_undefined_forms() {
    assert_pattern_boundary(
        COLOR_HSL_LABEL,
        &[],
        &[
            "hsl(120 75%)",
            "hsl(120, 75% 25%)",
            "hsl(120, 75, 25)",
            "hsl(120, none, 50%)",
            "hsl(120, 75%, 25% / 0.5)",
            "hsl(120 75% 25%, 0.5)",
            "hsl(120 75% 25% 0.5)",
            "hsl(120 75% 25% /)",
            "hsl(120 75% 25% 60%)",
            "hsla(120, 50%, 50%, 1.)",
            "hsl(deg 75% 25%)",
            "",
        ],
    );
}

const COLOR_HEX_LABEL: &str = "representation.format.color_hex";

/// CSS Color 4: `#` with 3, 4, 6 or 8 hex digits.
#[test]
fn color_hex_accepts_source_forms() {
    assert_pattern_boundary(
        COLOR_HEX_LABEL,
        &["#0000ffcc", "#f00c", "#FF0000", "#f00", "#0000FFCC"],
        &[],
    );
}

#[test]
fn color_hex_rejects_undefined_forms() {
    assert_pattern_boundary(
        COLOR_HEX_LABEL,
        &[],
        &[
            "0000ffcc", "f00c", "#12345", "#1234567", "#12", "12345", "#ggg", "2019", "20190101",
        ],
    );
}

/// The bare 3- and 6-digit forms stay accepted, with a `nonstandard` note for
/// the missing `#`; the `#` forms conform.
#[test]
fn color_hex_grades_bare_forms_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in ["#FF0000", "#f00", "#f00c", "#0000ffcc"] {
        assert_eq!(
            grade_value_for_label(value, COLOR_HEX_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    for value in ["FF0000", "f00"] {
        match grade_value_for_label(value, COLOR_HEX_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::Nonstandard, "{value}");
                assert_eq!(note.detail, "leading #", "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}

const IP_V6_LABEL: &str = "technology.internet.ip_v6";

/// RFC 4291 section 2.2: the full, `::`-compressed and IPv4-mixed forms.
#[test]
fn ip_v6_accepts_source_forms() {
    assert_pattern_boundary(
        IP_V6_LABEL,
        &[
            "2001:db8:85a3::8a2e:370:7334",
            "::1",
            "::",
            "fe80::1",
            "::ffff:192.0.2.128",
            "2001:0db8:85a3:0000:0000:8a2e:0370:7334",
            "1::",
            "::ffff:0:0",
            "1:2:3:4:5:6:7::",
            "::2:3:4:5:6:7:8",
            "1:2:3:4:5:6:1.2.3.4",
            "1::1.2.3.4",
            "1:2:3:4:5::1.2.3.4",
            "::1.2.3.4",
            "64:ff9b::192.0.2.33",
            "2001:DB8::ABCD",
        ],
        &[],
    );
}

/// Two `::`, a short group count without `::`, a 9-group value, a bad octet or
/// digit, and a zone ID are not RFC 4291 addresses.
#[test]
fn ip_v6_rejects_undefined_forms() {
    assert_pattern_boundary(
        IP_V6_LABEL,
        &[],
        &[
            "1::2::3",
            "12:34:56:78:9a:bc",
            "::ffff:192.0.2.256",
            "2001:db8::g",
            "1:2:3:4:5:6:7:8:9",
            "1:2:3:4:5:6:7::8",
            "::1:2:3:4:5:6:7:8",
            "12345::1",
            ":1",
            "1:",
            ":::",
            "fe80::1%eth0",
            "::ffff:192.0.2",
            "::ffff:01.2.3.4",
            "1:2:3:4:5:6:7:1.2.3.4",
            "1:2:3:4:5:6::1.2.3.4",
        ],
    );
}

const HOSTNAME_LABEL: &str = "technology.internet.hostname";

/// RFC 3696: an absolute name may end in one full stop.
#[test]
fn hostname_accepts_source_forms() {
    assert_pattern_boundary(
        HOSTNAME_LABEL,
        &[
            "example.com.",
            "localhost",
            "localhost.",
            "a.b.c.",
            "example.com",
        ],
        &[],
    );
}

#[test]
fn hostname_rejects_undefined_forms() {
    assert_pattern_boundary(
        HOSTNAME_LABEL,
        &[],
        &[
            "example..com",
            ".example.com",
            ".",
            "example.com..",
            "example .com.",
            "-a.com.",
        ],
    );
}

/// The detector counts a trailing full stop only after a dotted name, so a lone
/// word that ends a sentence conforms but is not counted for discrimination.
#[test]
fn hostname_detector_keeps_out_lone_words_with_a_stop() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(HOSTNAME_LABEL, &tax).expect("hostname rule");
    let detector = detector_for(HOSTNAME_LABEL);
    assert!(detector.is_some());
    for kept in ["example.com", "localhost", "example.com.", "a.bc."] {
        assert!(passes_detector_or_rule(detector, &rule, kept), "{kept:?}");
    }
    for word in [
        "done.",
        "inc.",
        "localhost.",
        "ok.",
        "a.m.",
        "p.m.",
        "e.g.",
        "u.s.",
    ] {
        assert!(rule.is_valid(word), "{word:?} conforms");
        assert!(!passes_detector_or_rule(detector, &rule, word), "{word:?}");
    }
}

const DATA_URI_LABEL: &str = "technology.internet.data_uri";

/// RFC 2397: the data may be empty; the Fetch Standard matches `base64`
/// case-insensitively.
#[test]
fn data_uri_accepts_source_forms() {
    assert_pattern_boundary(
        DATA_URI_LABEL,
        &[
            "data:,",
            "data:text/plain;BASE64,SGk=",
            "data:text/plain;Base64,SGk=",
            "data:text/plain;base64,",
            "data:;base64,SGk=",
            "data:text/plain;charset=utf-8,Hi",
            "data:image/png;base64,iVBORw0KGgo=",
        ],
        &[],
    );
}

#[test]
fn data_uri_rejects_undefined_forms() {
    assert_pattern_boundary(
        DATA_URI_LABEL,
        &[],
        &[
            "data:",
            "data:text/plain",
            "data:text/plain;base64",
            "data:text/plain;base63,SGk=",
            "text/plain;base64,SGk=",
            "DATA:,",
        ],
    );
}

const S3_URI_LABEL: &str = "technology.cloud.s3_uri";

/// AWS CLI S3Uri: an access-point ARN may stand in for the bucket.
#[test]
fn s3_uri_accepts_source_forms() {
    assert_pattern_boundary(
        S3_URI_LABEL,
        &[
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/myaccesspoint/mykey",
            "s3://my-bucket/data/file.csv",
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/myaccesspoint",
            "s3://arn:aws-cn:s3:cn-north-1:123456789012:accesspoint/my-ap/a/b.csv",
            "s3://arn:aws-us-gov:s3:us-gov-west-1:123456789012:accesspoint/my-ap/",
            "s3://my-bucket",
        ],
        &[],
    );
}

#[test]
fn s3_uri_rejects_undefined_forms() {
    assert_pattern_boundary(
        S3_URI_LABEL,
        &[],
        &[
            "s3://arn:aws:s3:us-west-2:1234:accesspoint/x",
            "s3://A",
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/x",
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/-bad-name",
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint/UPPER",
            "s3://arn:aws:s3:us-west-2:123456789012:accesspoint",
            "s3://arn:aws:s3:::my-bucket",
            "s3://arn:aws:ec2:us-west-2:123456789012:accesspoint/myaccesspoint",
            "s3://arn:azure:s3:us-west-2:123456789012:accesspoint/myaccesspoint",
        ],
    );
}

/// The bare `::` conforms (grading) but is not counted for discrimination.
#[test]
fn ip_v6_detector_keeps_out_the_bare_double_colon() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(IP_V6_LABEL, &tax).expect("ip_v6 rule");
    let detector = detector_for(IP_V6_LABEL);
    assert!(detector.is_some());
    assert!(rule.is_valid("::"));
    assert!(!passes_detector_or_rule(detector, &rule, "::"));
    for kept in ["::1", "1::", "fe80::1", "::ffff:192.0.2.128"] {
        assert!(passes_detector_or_rule(detector, &rule, kept), "{kept:?}");
    }
}

const VERSION_LABEL: &str = "technology.development.version";

/// Semantic Versioning 2.0.0: pre-release and build identifiers are dot-separated
/// runs of `[0-9A-Za-z-]`, so a hyphen may appear inside them.
#[test]
fn version_accepts_source_forms() {
    assert_pattern_boundary(
        VERSION_LABEL,
        &[
            "1.2.3-alpha-1",
            "1.0.0+build-7",
            "v1.2.3",
            "1.2.3-beta.1+exp.sha.5114f85",
            "1.0.0-0.3.7",
            "1.02.3",
        ],
        &[],
    );
}

#[test]
fn version_rejects_undefined_forms() {
    assert_pattern_boundary(
        VERSION_LABEL,
        &[],
        &[
            "1.2",
            "1.2.3-",
            "1.2.3-al pha",
            "1.2.3+",
            // Semantic Versioning requires every identifier to be non-empty; the old
            // rule took these.
            "1.2.3-alpha.",
            "1.2.3-a..b",
            "1.2.3-.x",
            "1.0.0+build.",
            "1.0.0+a..b",
        ],
    );
}

/// A leading zero in a numeric identifier is not Semantic Versioning, so it is
/// accepted with a note.
#[test]
fn version_grades_leading_zeros_as_noted() {
    use finetype_core::{grade_value_for_label, Grade, NoteKind};
    let tax = load_taxonomy();
    for value in ["1.2.3", "v1.2.3", "1.2.3-alpha-1", "1.0.0+build-7", "0.0.0"] {
        assert_eq!(
            grade_value_for_label(value, VERSION_LABEL, &tax),
            Grade::Conforms,
            "{value}"
        );
    }
    for value in ["1.02.3", "01.2.3", "1.2.3-01"] {
        match grade_value_for_label(value, VERSION_LABEL, &tax) {
            Grade::Noted(note) => {
                assert_eq!(note.kind, NoteKind::Nonstandard, "{value}");
                assert_eq!(note.detail, "no leading zeros", "{value}");
            }
            other => panic!("{value} should be noted, got {other:?}"),
        }
    }
}

const CALVER_LABEL: &str = "technology.development.calver";

/// calver.org: the month and the day are `MM`/`0M` and `DD`/`0D`, so a single
/// digit is a calendar version too.
#[test]
fn calver_accepts_source_forms() {
    assert_pattern_boundary(
        CALVER_LABEL,
        &[
            "2024.2.9",
            "2024.12.31",
            "2024.1",
            "2024.02",
            "2024.01.15.1",
        ],
        &[],
    );
}

#[test]
fn calver_rejects_undefined_forms() {
    assert_pattern_boundary(
        CALVER_LABEL,
        &[],
        &["24.2", "2024..1", "2024", "2024.", "2024.123", "2024.1x"],
    );
}

/// `2024.1` and `2020.5` conform but are also decimals, so the detector does not
/// count them; a two-digit month or a third component does.
#[test]
fn calver_detector_keeps_out_decimals() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(CALVER_LABEL, &tax).expect("calver rule");
    let detector = detector_for(CALVER_LABEL);
    assert!(detector.is_some());
    for kept in [
        "2024.02",
        "2024.12",
        "2024.2.9",
        "2024.12.31",
        "2024.01.15.1",
    ] {
        assert!(passes_detector_or_rule(detector, &rule, kept), "{kept:?}");
    }
    for out in ["2024.1", "2020.5", "1999.9", "2024.1.", "24.2", "2024"] {
        assert!(
            !passes_detector_or_rule(detector, &rule, out),
            "{out:?} is not counted"
        );
    }
    for decimal in ["2024.1", "2020.5"] {
        assert!(rule.is_valid(decimal), "{decimal:?} conforms");
    }
}

const DOCKER_REF_LABEL: &str = "technology.development.docker_ref";

/// OCI distribution spec: a tag is `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}`. OCI image
/// spec: a digest algorithm is `sha256`, `sha512` or `blake3`.
#[test]
fn docker_ref_accepts_source_forms() {
    let sha512 = format!("app@sha512:{}", "a".repeat(128));
    let blake3 = format!("app@blake3:{}", "b".repeat(64));
    let sha256_upper = format!("app@sha256:{}", "ABCDEF01".repeat(8));
    let tag_128 = format!("app:{}", "a".repeat(128));
    let with_tag_and_digest = format!("ghcr.io/org/app:v1@sha512:{}", "0".repeat(128));
    assert_pattern_boundary(
        DOCKER_REF_LABEL,
        &[
            "app:_dev",
            &sha512,
            &blake3,
            &sha256_upper,
            &tag_128,
            &with_tag_and_digest,
            "nginx:latest",
        ],
        &[],
    );
}

#[test]
fn docker_ref_rejects_undefined_forms() {
    let tag_129 = format!("app:{}", "a".repeat(129));
    let md5 = format!("app@md5:{}", "a".repeat(32));
    let short_sha512 = format!("app@sha512:{}", "a".repeat(64));
    let long_blake3 = format!("app@blake3:{}", "a".repeat(128));
    assert_pattern_boundary(
        DOCKER_REF_LABEL,
        &[],
        &[
            "app:.dev",
            "app:-dev",
            &tag_129,
            &md5,
            &short_sha512,
            &long_blake3,
        ],
    );
}

const JWT_LABEL: &str = "technology.cryptographic.jwt";

/// RFC 7519 section 6.1: the example Unsecured JWT (empty signature).
const JWT_UNSECURED: &str = "eyJhbGciOiJub25lIn0.eyJpc3MiOiJqb2UiLA0KICJleHAiOjEzMDA4MTkzODAsDQogImh0dHA6Ly9leGFtcGxlLmNvbS9pc19yb290Ijp0cnVlfQ.";

/// RFC 7516 appendix A.1.7: the complete JWE Compact Serialization example.
const JWT_JWE: &str = "eyJhbGciOiJSU0EtT0FFUCIsImVuYyI6IkEyNTZHQ00ifQ.OKOawDo13gRp2ojaHV7LFpZcgV7T6DVZKTyKOMTYUmKoTCVJRgckCL9kiMT03JGeipsEdY3mx_etLbbWSrFr05kLzcSr4qKAq7YN7e9jwQRb23nfa6c9d-StnImGyFDbSv04uVuxIp5Zms1gNxKKK2Da14B8S4rzVRltdYwam_lDp5XnZAYpQdb76FdIKLaVmqgfwX7XWRxv2322i-vDxRfqNzo_tETKzpVLzfiwQyeyPGLBIO56YJ7eObdv0je81860ppamavo35UgoRdbYaBcoh9QcfylQr66oc6vFWXRcZ_ZT2LawVCWTIy3brGPi6UklfCpIMfIjf7iGdXKHzg.48V1_ALb6US04U3b.5eym8TW_c8SuK0ltJ3rpYIzOeDQz7TALvtu6UG9oMo4vpzs9tX_EFShS8iB7j6jiSdiwkIr3ajwQzaBtQD_A.XFBoMYUZodetZdvTiFvSkQ";

const JWT_JWS: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

/// RFC 7519 allows an Unsecured JWT, which ends in a dot, and RFC 7516 defines the
/// five-part JWE compact form, whose encrypted key may be empty.
#[test]
fn jwt_accepts_source_forms() {
    let empty_key = JWT_JWE.replacen(
        ".OKOawDo13gRp2ojaHV7LFpZcgV7T6DVZKTyKOMTYUmKoTCVJRgckCL9kiMT03JGeipsEdY3mx_etLbbWSrFr05kLzcSr4qKAq7YN7e9jwQRb23nfa6c9d-StnImGyFDbSv04uVuxIp5Zms1gNxKKK2Da14B8S4rzVRltdYwam_lDp5XnZAYpQdb76FdIKLaVmqgfwX7XWRxv2322i-vDxRfqNzo_tETKzpVLzfiwQyeyPGLBIO56YJ7eObdv0je81860ppamavo35UgoRdbYaBcoh9QcfylQr66oc6vFWXRcZ_ZT2LawVCWTIy3brGPi6UklfCpIMfIjf7iGdXKHzg.",
        "..",
        1,
    );
    assert!(empty_key.contains(".."), "the encrypted key is empty");
    assert_pattern_boundary(
        JWT_LABEL,
        &[JWT_UNSECURED, JWT_JWE, &empty_key, JWT_JWS],
        &[],
    );
}

#[test]
fn jwt_rejects_undefined_forms() {
    let four_part = format!("{JWT_JWS}.eyJhbGciOiJub25lIn0");
    let two_part = "eyJhbGciOiJub25lIn0.eyJpc3MiOiJqb2UifQ";
    // A JWE is exactly five parts: four segments, or the first three of a JWE.
    let jwe_four: String = JWT_JWE.rsplitn(2, '.').last().unwrap().to_string();
    assert_pattern_boundary(
        JWT_LABEL,
        &[],
        &[
            "a.b.c",
            &four_part,
            two_part,
            &jwe_four,
            "...",
            "eyJhbGciOiJub25lIn0..",
            ".eyJpc3MiOiJqb2UifQ.",
        ],
    );
}

/// The detector counts the three-segment shape the type was first defined by, and
/// a JWE or an unsecured token only when its header decodes to JSON with `alg`;
/// dotted names that fit the widened rule conform but are not counted.
#[test]
fn jwt_detector_keeps_out_dotted_names() {
    use finetype_core::discrimination::{detector_for, passes_detector_or_rule};
    use finetype_core::validator::resolve_label_validator;
    let tax = load_taxonomy();
    let rule = resolve_label_validator(JWT_LABEL, &tax).expect("jwt rule");
    let detector = detector_for(JWT_LABEL);
    assert!(detector.is_some());
    for kept in [JWT_JWS, JWT_UNSECURED, JWT_JWE] {
        assert!(passes_detector_or_rule(detector, &rule, kept), "{kept:?}");
    }
    for not_counted in [
        "internationalization.configuration.",
        "springframework.web.configuration.mvcdispatcher.DispatcherServlet",
        "abcdefghijk.lmnopqrstuv.wxyzabcdefg.hijklmnopqr.stuvwxyzabc",
    ] {
        assert!(rule.is_valid(not_counted), "{not_counted:?} conforms");
        assert!(
            !passes_detector_or_rule(detector, &rule, not_counted),
            "{not_counted:?} is not counted"
        );
    }
}

const PG_SHORT_OFFSET_LABEL: &str = "datetime.timestamp.pg_short_offset";

/// PostgreSQL's ISO output (`DateStyle` ISO) writes the fraction only when it is
/// non-zero and trims trailing zeros, so one to six digits or none.
#[test]
fn pg_short_offset_accepts_source_forms() {
    assert_pattern_boundary(
        PG_SHORT_OFFSET_LABEL,
        &[
            "2022-01-06 18:00:00+03",
            "1997-12-17 07:37:16-08",
            "2022-01-06 18:00:00.5+03",
            "2024-01-15 14:30:00.123456-05",
            "2023-12-31 23:59:59.000000+00",
        ],
        &[],
    );
}

#[test]
fn pg_short_offset_rejects_undefined_forms() {
    assert_pattern_boundary(
        PG_SHORT_OFFSET_LABEL,
        &[],
        &[
            "2022-01-06 18:00:00+3",
            "2022-01-06 18:00:00.1234567+03",
            "2022-01-06 18:00:00.+03",
            "2022-01-06 18:00:00",
            "2022-01-06 18:00:00+03:00",
            "2022-01-06 18:00:00+0300",
            "2022-01-06T18:00:00+03",
        ],
    );
}

const ISO_DURATION_LABEL: &str = "datetime.duration.iso_8601";

/// ISO 8601-1 section 5.5.2.3: the smallest component may carry a decimal
/// fraction, with a comma (preferred) or a full stop; section 5.5.2.4 gives the
/// alternative format `PYYYY-MM-DDThh:mm:ss`.
#[test]
fn iso_duration_accepts_source_forms() {
    assert_pattern_boundary(
        ISO_DURATION_LABEL,
        &[
            "P0,5Y",
            "PT0.5S",
            "PT1H30,5M",
            "P0003-06-04T12:30:05",
            "P2W",
            "P0.5Y",
            "P1Y2,5M",
            "P1DT1.5H",
            "-P1,5D",
            "PT30M",
            "P1Y2M3DT4H5M6.5S",
            "P0001-12-30T24:59:59",
        ],
        &[],
    );
}

#[test]
fn iso_duration_rejects_undefined_forms() {
    assert_pattern_boundary(
        ISO_DURATION_LABEL,
        &[],
        &[
            "P0.5Y1M",
            "P1,5Y2M",
            "PT1.5H30M",
            "PT1.5H2.5M",
            "P0.5YT1H",
            "P",
            "PT",
            "P.5Y",
            "P1.Y",
            "PT0,S",
            "P0003-13-04T12:30:05",
            "P0003-06-31T12:30:05",
            "P0003-06-04T12:30",
            "P0003-06-04",
        ],
    );
}

/// ISO 8601-1 allows 60 as the minute or second of the alternative format. The rule
/// deliberately rejects it (a deviation from the standard) because the transform
/// cannot cast an interval with a 60 there; `notes` say so.
#[test]
fn iso_duration_rejects_sixty_in_the_alternative_format_by_design() {
    assert_pattern_boundary(
        ISO_DURATION_LABEL,
        &[],
        &["P0003-06-04T12:60:05", "P0003-06-04T12:30:60"],
    );
}
