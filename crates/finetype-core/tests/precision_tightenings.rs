//! Regression tests for taxonomy validators that were TIGHTENED because a
//! shape-only pattern confirmed everything of its shape.
//!
//! Sibling of `precision_widenings.rs`, opposite direction. A widening has to
//! prove it still rejects garbage; a tightening has to prove it still accepts
//! the real family. Both serve the Precision Principle: "a validation that
//! confirms 90% of random input is not a validation".
//!
//! Each tightening gets an `ACCEPT` slice and a `REJECT` slice exercised
//! against the live `labels/definitions_*.yaml` taxonomy through
//! `validate_value_for_label` — there is no second source of truth.
//!
//! Run: `cargo test -p finetype-core --test precision_tightenings`

use finetype_core::taxonomy::Taxonomy;
use finetype_core::validator::validate_value_for_label;
use std::path::PathBuf;

/// Workspace root, derived from `CARGO_MANIFEST_DIR` (`crates/finetype-core`)
/// by popping two segments. Mirrors the helper in `precision_widenings.rs`.
fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // -> crates/
    p.pop(); // -> workspace root
    p
}

/// Load the live taxonomy from `labels/`, validators compiled so the test path
/// mirrors production.
fn load_taxonomy() -> Taxonomy {
    let root = workspace_root();
    let labels_dir = root.join("labels");
    let mut tax = Taxonomy::from_directory(&labels_dir)
        .expect("load taxonomy from labels/ — is labels/ present in workspace?");
    tax.compile_validators();
    tax
}

/// Assert every value in `accept` validates against `label`, and every value in
/// `reject` does not. Failure messages name the offending value so the test
/// output diagnoses which side of the boundary drifted.
fn assert_pattern_boundary(label: &str, accept: &[&str], reject: &[&str]) {
    let tax = load_taxonomy();

    for value in accept {
        let result = validate_value_for_label(value, label, &tax).unwrap_or_else(|e| {
            panic!("validate_value_for_label({label:?}, {value:?}) errored: {e:?}")
        });
        assert!(
            result.is_valid,
            "ACCEPT-set regression: value {:?} should validate against {} but failed with {:?} \
             — the tightening has cut into the real family",
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
             the validator is back to confirming its shape instead of its substance",
            value, label
        );
    }
}

/// Tightening 1 — `datetime.date.compact_ymd` carries year/month/day windows,
/// not just eight-digit shape.
///
/// `^\d{8}$` confirmed every eight-digit token, so a financial figure and a
/// surrogate key both validated as a date and the validation veto had nothing
/// to push back with. Every value in the REJECT set below is a REAL corpus
/// value from a column the engine typed `datetime.date.compact_ymd`, taken from
/// the gittables corpus pass — they are the false-positive family, not
/// hypotheticals, and between them they exercise each of the three windows
/// independently:
///
/// - year window (`19|20`): `grossProfit` 71132000, `sharesOutstanding`
///   25012600, `goodWill` 48479000, `intangibleAssets` 16514000, NBA `GAME_ID`
///   21601092, `PostId` 18502653 — all have a leading pair that is not a
///   plausible century, and each has an in-range month/day tail, so ONLY the
///   year window rejects them.
/// - month window (`0[1-9]|1[0-2]`): `researchDevelopment` 20184000 (month 40),
///   `wc` 20106460 (month 64), `changeToLiabilities` 19845000 (month 50),
///   `sellingGeneralAdministrative` 19022167 (month 21), `PostId` 19855204
///   (month 52) — plausible year, impossible month.
/// - day window (`0[1-9]|[12]\d|3[01]`): `marketCap` 19390490 (day 90),
///   `commonStock` 20361000 (day 00), `grossProfit` 20100500 (day 00),
///   `longTermDebt` 19571000 (day 00) — plausible year AND month, impossible
///   day. Without the day window these four still validate.
///
/// ACCEPT keeps the real family: the two taxonomy `samples`, the corpus date
/// columns the fix must not touch (`date` 20151115, `CLM_FROM_DT` 20080707,
/// `FirstAddedDate` 20171231, `game_date` 20161107, `end_date` 19800417), the
/// century edges 19000101/20991231, and a leap day. Day 31 in a 30-day month
/// (20240931) is deliberately in ACCEPT — the windows are ranges, not a
/// calendar, and that is the documented contract.
#[test]
fn ptc_tightening_compact_ymd_rejects_eight_digit_figures_and_keys() {
    assert_pattern_boundary(
        "datetime.date.compact_ymd",
        // ACCEPT — the real family. Every one of these is a genuine YYYYMMDD
        // date; if the tightening rejects any, it has over-shot.
        &[
            // The taxonomy's own declared samples.
            "20240115", "20191231",
            // Real corpus columns typed compact_ymd that must survive.
            "20151115", // `date`, the constant-date column beside marketCap/ebit
            "20080707", // `CLM_FROM_DT`
            "20171231", // `FirstAddedDate`
            "20161107", // `game_date`
            "20200319", // `data_ricevimento_tampone`
            "20190514", // `FirstAddedDate`
            "19800417", // `end_date`
            "20061209", // `reviewed`
            // Boundaries of each window.
            "19000101", // first accepted year, month 01, day 01
            "20991231", // last accepted year, month 12, day 31
            "19700101", // the epoch
            "20000229", // leap day
            "20240931", // day 31 of a 30-day month — accepted by contract
        ],
        // REJECT — real corpus values from columns the engine mis-typed as
        // compact_ymd. Grouped by the window that catches each one.
        &[
            // Year window.
            "71132000", // grossProfit
            "25012600", // sharesOutstanding
            "48479000", // goodWill
            "16514000", // intangibleAssets
            "15800000", // ebit
            "21601092", // NBA GAME_ID
            "18502653", // PostId
            "10169207", // end_id
            "31038000", // cash
            // Month window (plausible year, impossible month).
            "20184000", // researchDevelopment — month 40
            "20106460", // wc — month 64
            "19845000", // changeToLiabilities — month 50
            "19022167", // sellingGeneralAdministrative — month 21
            "19855204", // PostId — month 52
            "20200000", // sharesOutstanding — month 00
            "19748000", // otherLiab — month 80
            // Day window (plausible year AND month, impossible day).
            "19390490", // marketCap — day 90
            "20361000", // commonStock — day 00
            "20100500", // grossProfit — day 00
            "19571000", // longTermDebt — day 00
            // Shape violators — the prior contract, which must stay green.
            "2024011",   // seven digits
            "202401155", // nine digits
            "2024-01-15",
            "",
            "abcdefgh",
        ],
    );
}

/// The tightening has to bite where it matters: through the HARD VALIDATION
/// VETO, which is what actually rewrites a column's emitted type.
///
/// `datetime.date.compact_ymd` is on the veto-safe allowlist
/// (`labels/veto_safe.txt`), so a sub-50%-pass-rate column is hard-vetoed off
/// the label. Under `^\d{8}$` a financial column passes at 100% and the veto is
/// inert — which is exactly why the false positive shipped. This test asserts
/// the veto's own verdict on a real column of financial figures, and asserts it
/// stays inert on a real column of dates.
///
/// Values are corpus samples from financial-statement tables (`grossProfit`,
/// `sharesOutstanding`, `goodWill`, `marketCap`, `intangibleAssets`) and from a
/// `date` column beside them — the pair the shape-only validator could not
/// separate.
#[test]
fn ptc_compact_ymd_veto_fires_on_financial_column_and_stays_inert_on_dates() {
    let tax = load_taxonomy();
    let safe = finetype_core::audited_safe_labels();
    assert!(
        safe.contains("datetime.date.compact_ymd"),
        "compact_ymd must stay on the veto-safe allowlist or the tightening cannot bite"
    );

    let financial = [
        "71132000", "70976000", "70504000", "64512000", "25012600", "48479000", "19390490",
        "16514000",
    ];
    let opts: Vec<Option<&str>> = financial.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_ymd", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of eight-digit financial figures must be HARD-VETOED off compact_ymd; \
         pass_rate = {:?}",
        veto.pass_rate
    );

    let dates = [
        "20151115", "20161130", "20171231", "20180331", "20190630", "20200930", "20211231",
        "20240115",
    ];
    let opts: Vec<Option<&str>> = dates.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_ymd", &opts, &tax, &safe);
    assert!(
        !veto.vetoed,
        "a column of genuine YYYYMMDD dates must NOT be vetoed off compact_ymd; \
         pass_rate = {:?}",
        veto.pass_rate
    );
    assert_eq!(
        veto.pass_rate,
        Some(1.0),
        "genuine dates must pass the tightened validator at 100%"
    );
}
