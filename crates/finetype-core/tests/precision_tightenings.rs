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

/// Tightening 1 — `datetime.date.compact_ymd` carries month and day windows,
/// not just eight-digit shape.
///
/// `^\d{8}$` confirmed every eight-digit token, so a financial figure and a
/// surrogate key both validated as a date and the validation veto had nothing
/// to push back with. Every value in the REJECT set below is a REAL corpus
/// value from a column the engine typed `datetime.date.compact_ymd`, taken from
/// the gittables corpus pass — they are the false-positive family, not
/// hypotheticals.
///
/// THE YEAR IS `\d{4}` AND CARRIES NO WINDOW. An earlier revision of this
/// change shipped `(19|20)\d{2}` and a reviewer refuted it through the CLI: an
/// archival YYYYMMDD column of 1865–1872 values lost the label, dropped to
/// `representation.numeric.integer_number` at pass rate 0.0, and had its
/// `strptime` transform stripped — the same confident-and-wrong failure the
/// tightening exists to stop, pointed the other way. The sibling `compact_ym`
/// has likewise always been `^\d{4}(0[1-9]|1[0-2])$`, month window only. The
/// month and day windows are what reject eight-digit financial figures, and
/// they do the whole job: grouped by the window that ACTUALLY catches each
/// value —
///
/// - month window (`0[1-9]|1[0-2]`): `researchDevelopment` 20184000 (m40),
///   `wc` 20106460 (m64), `changeToLiabilities` 19845000 (m50),
///   `sellingGeneralAdministrative` 19022167 (m21), `PostId` 19855204 (m52),
///   `sharesOutstanding` 20200000 (m00), `otherLiab` 19748000 (m80),
///   `grossProfit` 71132000 (m20), `sharesOutstanding` 25012600 (m26),
///   `goodWill` 48479000 (m90), `intangibleAssets` 16514000 (m40), `ebit`
///   15800000 (m00), `PostId` 18502653 (m26), `end_id` 10169207 (m92), `cash`
///   31038000 (m80).
/// - day window (`0[1-9]|[12]\d|3[01]`): `marketCap` 19390490 (d90),
///   `commonStock` 20361000 (d00), `grossProfit` 20100500 (d00),
///   `longTermDebt` 19571000 (d00), NBA `GAME_ID` 21601092 (m10 valid, d92).
///
/// A previous doc-comment here claimed the first nine of those "all have a
/// leading pair that is not a plausible century … so ONLY the year window
/// rejects them". That was false for all nine, and it is what disguised the
/// coverage gap: with the year window deleted the REJECT set stayed 100%
/// rejecting, so no test could see the year policy at all.
/// `ptc_compact_ymd_each_window_is_pinned_by_a_value_only_it_rejects` below
/// pins each window by construction instead of by hope.
///
/// ACCEPT keeps the real family: the two taxonomy `samples`, the corpus date
/// columns the fix must not touch (`date` 20151115, `CLM_FROM_DT` 20080707,
/// `FirstAddedDate` 20171231, `game_date` 20161107, `end_date` 19800417),
/// nineteenth-century archival dates (18651115 … 18721231), the epoch, and a
/// leap day. Day 31 in a 30-day month (20240931) is deliberately in ACCEPT —
/// the windows are ranges, not a calendar, and that is the documented
/// contract.
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
            // NINETEENTH-CENTURY ARCHIVAL DATES. These are the values a
            // reviewer used to refute the `(19|20)\d{2}` year window: under it
            // the whole column dropped to an integer with its date transform
            // stripped. Re-introduce a century window and these redden.
            "18651115", "18661130", "18671231", "18680331", "18690630", "18700930", "18711231",
            "18720115",
            // Boundaries of the month and day windows.
            "19000101", // month 01, day 01 — the low corner
            "20991231", // month 12, day 31 — the high corner
            "19700101", // the epoch
            "20000229", // leap day
            "20240931", // day 31 of a 30-day month — accepted by contract
        ],
        // REJECT — real corpus values from columns the engine mis-typed as
        // compact_ymd. Grouped by the window that ACTUALLY catches each one:
        // every month/day field is re-read here, not assumed.
        &[
            // Month window (impossible month).
            "71132000", // grossProfit — month 20
            "25012600", // sharesOutstanding — month 26
            "48479000", // goodWill — month 90
            "16514000", // intangibleAssets — month 40
            "15800000", // ebit — month 00
            "18502653", // PostId — month 26
            "10169207", // end_id — month 92
            "31038000", // cash — month 80
            "20184000", // researchDevelopment — month 40
            "20106460", // wc — month 64
            "19845000", // changeToLiabilities — month 50
            "19022167", // sellingGeneralAdministrative — month 21
            "19855204", // PostId — month 52
            "20200000", // sharesOutstanding — month 00
            "19748000", // otherLiab — month 80
            // Day window (valid month, impossible day).
            "21601092", // NBA GAME_ID — month 10, day 92
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
            // The year field is unwindowed but still four DIGITS: this is the
            // only reject-side constraint the year carries, so it gets a case.
            "18a51115", // month 11 and day 15 are both valid; the year is not
        ],
    );
}

/// Every window in the shipped pattern is pinned by a value that ONLY that
/// window rejects — proved by a twin that differs in that field alone.
///
/// This is the test the first revision of this change did not have. Its REJECT
/// set contained no value that survived deleting the year window and no value
/// whose rejection was attributable to a single field, so mutating the shipped
/// validator to drop a window left the whole file green. Here each case is a
/// PAIR: a real corpus value that must reject, and a twin differing only in the
/// named field that must accept. Delete the window and the reject half fails;
/// over-tighten it and the accept half fails. Neither half recomputes anything
/// from the pattern — both go through `validate_value_for_label`.
#[test]
fn ptc_compact_ymd_each_window_is_pinned_by_a_value_only_it_rejects() {
    // (field, rejected real corpus value, twin differing ONLY in that field)
    let pairs: [(&str, &str, &str); 5] = [
        // MONTH. 1985-52-04 → 1985-02-04: same year, same day, legal month.
        ("month", "19855204", "19850204"), // PostId
        // 1016-92-07 → 1016-09-07.
        ("month", "10169207", "10160907"), // end_id
        // DAY. 1939-04-90 → 1939-04-19: same year, same month, legal day.
        ("day", "19390490", "19390419"), // marketCap
        // 2160-10-92 → 2160-10-29.
        ("day", "21601092", "21601029"), // NBA GAME_ID
        // 2036-10-00 → 2036-10-01.
        ("day", "20361000", "20361001"), // commonStock
    ];

    let tax = load_taxonomy();
    let label = "datetime.date.compact_ymd";
    for (field, bad, twin) in pairs {
        let bad_r = validate_value_for_label(bad, label, &tax).expect("validate bad");
        let twin_r = validate_value_for_label(twin, label, &tax).expect("validate twin");
        assert!(
            !bad_r.is_valid,
            "{field} window is not doing its job: {bad:?} validated against {label}"
        );
        assert!(
            twin_r.is_valid,
            "{field} window is over-tight: {twin:?} differs from {bad:?} only in the {field} \
             field and is a legal date, but it failed with {:?}",
            twin_r.errors
        );
    }

    // THE YEAR CARRIES NO WINDOW, and that is the shipped policy — pinned from
    // both sides. A nineteenth-century date must validate (a century window
    // reddens this), and a non-numeric year must not (the four-digit shape
    // still binds).
    for historical in ["18651115", "18700930", "18721231", "17760704", "12000101"] {
        let r = validate_value_for_label(historical, label, &tax).expect("validate historical");
        assert!(
            r.is_valid,
            "the year field must stay unwindowed: genuine historical date {historical:?} \
             failed with {:?} — a century window strips the date transform off real archival \
             columns and ships them as confident integers",
            r.errors
        );
    }
    for not_a_year in ["18a51115", "-8651115", "186 1115"] {
        let r = validate_value_for_label(not_a_year, label, &tax).expect("validate non-year");
        assert!(
            !r.is_valid,
            "the year field must still be four digits: {not_a_year:?} validated against {label}"
        );
    }
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
///
/// Four columns, each pinning a different clause AT THE VETO LAYER, which is
/// the layer that rewrites a type:
///
/// - `financial` — vetoed. Inert under a shape-only validator.
/// - `prompt_id` — vetoed only by the MONTH window (every value has a legal
///   day). Delete the month window and this column passes at 1.0 and ships as
///   a date.
/// - `game_id` — vetoed only by the DAY window (every value has month 09 or
///   10). Delete the day window and it passes at 1.0.
/// - `dates` / `historical` — inert. Under a `(19|20)\d{2}` year window the
///   nineteenth-century column is hard-vetoed at pass rate 0.0 and ships as an
///   integer with no date transform, which is the same defect wearing the
///   other hat.
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

    // `Prompt ID`, a real corpus identifier column. Every value has a legal day
    // (18, 19, 22, 25, 26, 27, 10, 11) and an impossible month (45, 72), so the
    // MONTH window alone is what vetoes it.
    let prompt_id = [
        "43064518", "43064519", "43064525", "43064522", "43064526", "43064527", "43197210",
        "43197211",
    ];
    let opts: Vec<Option<&str>> = prompt_id.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_ymd", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of surrogate keys with legal days and impossible months must be HARD-VETOED \
         off compact_ymd — this is the MONTH window's own case; pass_rate = {:?}",
        veto.pass_rate
    );

    // NBA `GAME_ID`, a real corpus column. Every value carries month 09 or 10,
    // so the DAY window alone is what vetoes it.
    let game_id = [
        "21601092", "21601077", "21601062", "21601045", "21601037", "21601020", "21601003",
        "21600990",
    ];
    let opts: Vec<Option<&str>> = game_id.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_ymd", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of game identifiers with legal months and impossible days must be HARD-VETOED \
         off compact_ymd — this is the DAY window's own case; pass_rate = {:?}",
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

    // The nineteenth-century column the reviewer used to refute the year
    // window. Same assertions, 150 years earlier.
    let historical = [
        "18651115", "18661130", "18671231", "18680331", "18690630", "18700930", "18711231",
        "18720115",
    ];
    let opts: Vec<Option<&str>> = historical.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_ymd", &opts, &tax, &safe);
    assert!(
        !veto.vetoed,
        "a column of nineteenth-century YYYYMMDD dates must NOT be vetoed off compact_ymd — \
         a century year window strips its date transform and ships it as a confident integer; \
         pass_rate = {:?}",
        veto.pass_rate
    );
    assert_eq!(
        veto.pass_rate,
        Some(1.0),
        "nineteenth-century dates must pass the validator at 100%"
    );
}
