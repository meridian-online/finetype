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
/// A previous doc-comment here filed 71132000, 25012600, 48479000, 16514000,
/// 15800000, 21601092, 18502653, 10169207 and 31038000 under a year window,
/// claiming they "all have a leading pair that is not a plausible century … so
/// ONLY the year window rejects them". That was false for all nine — eight die
/// on the month, one on the day — and it is what disguised the coverage gap:
/// with the year window deleted the REJECT set stayed 100% rejecting, so no
/// test could see the year policy at all.
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

/// Tightening 2 — `datetime.date.compact_dmy` carries day and month windows,
/// not just eight-digit shape.
///
/// THE DEFECT IS OLDER THAN THE CHANGE IT WAS BLAMED ON. This tightening was
/// opened on the premise that tightening the year-first leaf had RELOCATED an
/// eight-digit false positive onto this one — a low-confidence integer becoming
/// a high-confidence date. A four-sided probe refutes that
/// (`scripts/probe_compact_date_residual.sh`,
/// `docs/compact-date-residual.tsv`): the released binary and the PARENT of the
/// year-first tightening agree record for record, and three column families
/// already emitted `datetime.date.compact_dmy` at high confidence with a
/// `%d%m%Y` strptime attached before that change existed.
///
///   fixture                      released       after the year-first fix
///   ymd_reject_set               0.9878 high    0.9064 high
///   sequential_ids               0.8341 medium  0.8110 medium
///   round_hundred_share_counts   0.9999 high    0.9996 high
///
/// The year-first change moved the confidence slightly and the label not at
/// all. So this is not a repair of a regression — it is the first fix the
/// day-first leaf has had, and it is worth more than the premise claimed: three
/// families, not one, stop shipping a confident wrong date.
///
/// The windows sit on the FIRST two fields here, because the ordering is
/// day-month-year: day `0[1-9]|[12]\d|3[01]`, month `0[1-9]|1[0-2]`.
///
/// THE YEAR IS `\d{4}` AND CARRIES NO WINDOW, for the reason a reviewer proved
/// on the year-first leaf: a century window rejects genuine nineteenth-century
/// dates, strips the `strptime` transform, and ships an archival date column as
/// a confident integer. That is this same defect pointed the other way. The
/// price of the widest year window that would still bite (`1000-2099`) was
/// MEASURED rather than assumed — `docs/compact-dmy-corpus-family.json`: it
/// puts 30 more columns below the veto threshold and takes 54 day-first
/// candidate columns with it, `clm_from_dt`, `clm_thru_dt` and `ship_date`
/// among them. Not worth it.
///
/// Every value in REJECT is a REAL corpus value from a column the engine typed
/// `datetime.date.compact_dmy` in a 33,250-table pass, grouped by the window
/// that actually catches it — each field re-read, not assumed:
///
/// - day window: `totalCashFromOperatingActivities` 91095000 (d91),
///   `otherAssets` 48120000 (d48), `depreciation` 39082000 (d39),
///   `treasuryStock` 50128000 (d50), `sharesOutstanding` 32038200 (d32),
///   `inventory` 34065000 (d34), `commonStock` 58100000 (d58).
/// - month window: `ID` 10825933 (m82), `ID` 10722218 (m72), `GAME_ID`
///   12000020 (m00), `sharesOutstanding` 17250000 (m25), `minorityInterest`
///   17705000 (m70), `depreciation` 17452000 (m45), `minorityInterest`
///   18841000 (m84) and 16400000 (m40).
/// - both windows: `GAME_ID` 21600200 (d21 legal, m60), `minorityInterest`
///   46200000 (d46, m20), `otherAssets` 59096000 (d59, m09 legal).
#[test]
fn ptc_tightening_compact_dmy_rejects_eight_digit_figures_and_keys() {
    assert_pattern_boundary(
        "datetime.date.compact_dmy",
        // ACCEPT — the real family. Genuine DD-MM-YYYY renderings, which must
        // survive: DD-MM-YYYY is the ordering most of the world writes.
        &[
            // The taxonomy's own declared samples.
            "15012024", "31122019",
            // Ordinary day-first dates.
            "28021996", "17081947", "23111963", "09051945", "25121980",
            // NINETEENTH- AND EIGHTEENTH-CENTURY DATES. These are the values
            // that refute a century year window on this leaf, the same way an
            // 1865-1872 column refuted one on `compact_ymd`. Re-introduce
            // `(19|20)\d{2}` here and these redden.
            "01031865", "30061815", "14071789", "04071776",
            // Boundaries of the day and month windows.
            "01011900", // day 01, month 01 — the low corner
            "31121799", // day 31, month 12 — the high corner
            "29022000", // leap day
            "31041999", // day 31 of a 30-day month — accepted by contract
            // A real corpus value whose day and month are BOTH legal. It is a
            // financial figure, not a date, and the windows cannot tell —
            // see `ptc_compact_dmy_names_the_residual_the_windows_cannot_reach`.
            "19022167", // sellingGeneralAdministrative — day 19, month 02
        ],
        // REJECT — real corpus values from columns the engine typed
        // compact_dmy, grouped by the window that catches each one.
        &[
            // Day window (impossible day).
            "91095000", // totalCashFromOperatingActivities — day 91
            "48120000", // otherAssets — day 48
            "39082000", // depreciation — day 39
            "50128000", // treasuryStock — day 50
            "32038200", // sharesOutstanding — day 32
            "34065000", // inventory — day 34
            "58100000", // commonStock — day 58
            "37031000", // depreciation — day 37
            // Month window (legal day, impossible month).
            "10825933", // ID — month 82
            "10722218", // ID — month 72
            "12000020", // GAME_ID — month 00
            "17250000", // sharesOutstanding — month 25
            "17705000", // minorityInterest — month 70
            "17452000", // depreciation — month 45
            "18841000", // minorityInterest — month 84
            "16400000", // minorityInterest — month 40
            // Both windows.
            "21600200", // GAME_ID — day 21 legal, month 60
            "46200000", // minorityInterest — day 46, month 20
            "59096000", // otherAssets — day 59, month 09 legal
            // Shape violators — the prior contract, which must stay green.
            "1501202",   // seven digits
            "150120240", // nine digits
            "15-01-2024",
            "",
            "abcdefgh",
            // The year field is unwindowed but still four DIGITS: this is the
            // only reject-side constraint the year carries, so it gets a case.
            "1501202a", // day 15 and month 01 are both valid; the year is not
        ],
    );
}

/// Every window in the shipped day-first pattern is pinned by a real corpus
/// value that ONLY that window rejects — proved by a twin differing in that
/// field alone.
///
/// The sibling test on `compact_ymd` exists because the first revision of that
/// change had a REJECT set no single-window mutation could redden, so deleting
/// a window left the file green — the fifteenth instance in this repo of a
/// structural guard passing on broken code. Same construction here: each case
/// is a PAIR. Delete the window and the reject half fails; over-tighten it and
/// the accept half fails. Neither half recomputes anything from the pattern —
/// both go through `validate_value_for_label`.
#[test]
fn ptc_compact_dmy_each_window_is_pinned_by_a_value_only_it_rejects() {
    // (field, rejected real corpus value, twin differing ONLY in that field)
    let pairs: [(&str, &str, &str); 5] = [
        // DAY. 91-09-5000 → 19-09-5000: same month, same year, legal day.
        ("day", "91095000", "19095000"), // totalCashFromOperatingActivities
        // 48-12-0000 → 18-12-0000.
        ("day", "48120000", "18120000"), // otherAssets
        // 32-03-8200 → 23-03-8200.
        ("day", "32038200", "23038200"), // sharesOutstanding
        // MONTH. 10-82-5933 → 10-08-5933: same day, same year, legal month.
        ("month", "10825933", "10085933"), // ID
        // 10-72-2218 → 10-07-2218.
        ("month", "10722218", "10072218"), // ID
    ];

    let tax = load_taxonomy();
    let label = "datetime.date.compact_dmy";
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
    // both sides. Genuine historical day-first dates must validate (a century
    // window reddens this), and a non-numeric year must not.
    for historical in ["01031865", "30061815", "14071789", "04071776", "01010999"] {
        let r = validate_value_for_label(historical, label, &tax).expect("validate historical");
        assert!(
            r.is_valid,
            "the year field must stay unwindowed: genuine historical date {historical:?} \
             failed with {:?} — a century window strips the date transform off real archival \
             columns and ships them as confident integers",
            r.errors
        );
    }
    for not_a_year in ["1501202a", "150120-4", "15012 24"] {
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
/// AND THE ALLOWLIST IS HALF THE CHANGE. `datetime.date.compact_dmy` was NOT on
/// `labels/veto_safe.txt` while both its siblings were, so its validator's
/// verdict was ADVISORY: the profile path computed a 0.0 pass rate and then let
/// the label stand, `strptime` transform and all. The tightened pattern alone
/// leaves that intact — measured, not argued: a column of eight real
/// round-hundred share counts emitted `datetime.date.compact_dmy` at confidence
/// 0.98, `validation_pass_rate` 0.0, `validation_advisory_low` true, transform
/// `strptime({col}, '%d%m%Y')::DATE`. A validator whose verdict nothing acts on
/// is not a validator. Delete the allowlist line and this test reddens on its
/// first assertion.
///
/// Four columns, each pinning a different clause AT THE VETO LAYER:
///
/// - `financial` — vetoed. Inert under a shape-only validator.
/// - `id` — vetoed only by the MONTH window (every value has day 10).
/// - `shares` — vetoed only by the DAY window (every value has month 03).
/// - `day_first` — inert. A century year window would veto these and ship an
///   archival date column as a confident integer.
#[test]
fn ptc_compact_dmy_veto_fires_on_financial_column_and_stays_inert_on_day_first_dates() {
    let tax = load_taxonomy();
    let safe = finetype_core::audited_safe_labels();
    assert!(
        safe.contains("datetime.date.compact_dmy"),
        "compact_dmy must be on the veto-safe allowlist or the tightening cannot bite: its \
         pass rate stays ADVISORY and the label ships with its strptime transform however \
         many values the validator rejects. Both siblings (compact_mdy, compact_ymd) are on \
         it; this leaf was omitted because the both-lens sweep had ZERO agreement columns for \
         it, not because it was measured brittle."
    );

    // Real corpus round-hundred share counts and balance-sheet figures — the
    // family that survived the year-first tightening by moving onto this label.
    let financial = [
        "17250000", "17705000", "18841000", "16400000", "16700000", "46200000", "45400000",
        "17452000",
    ];
    let opts: Vec<Option<&str>> = financial.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_dmy", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of eight-digit financial figures must be HARD-VETOED off compact_dmy; \
         pass_rate = {:?}",
        veto.pass_rate
    );

    // A real corpus `ID` column of sequential surrogate keys. Every value has
    // day 10 and month 82, so the MONTH window alone is what vetoes it.
    let id = [
        "10825933", "10825932", "10825931", "10825930", "10825929", "10825928", "10825927",
        "10825926",
    ];
    let opts: Vec<Option<&str>> = id.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_dmy", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of surrogate keys with a legal day and an impossible month must be \
         HARD-VETOED off compact_dmy — this is the MONTH window's own case; pass_rate = {:?}",
        veto.pass_rate
    );

    // A real corpus `sharesOutstanding` column. Every value has month 03, so
    // the DAY window alone is what vetoes it.
    let shares = ["32038200", "32038200", "32038200", "32038200"];
    let opts: Vec<Option<&str>> = shares.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_dmy", &opts, &tax, &safe);
    assert!(
        veto.vetoed,
        "a column of figures with a legal month and an impossible day must be HARD-VETOED off \
         compact_dmy — this is the DAY window's own case; pass_rate = {:?}",
        veto.pass_rate
    );

    // Genuine day-first dates, including three from before 1900. The veto must
    // stay inert: this is the half of the contract a century year window
    // breaks.
    let day_first = [
        "15012024", "31122019", "28021996", "17081947", "01031865", "30061815", "14071789",
        "04071776",
    ];
    let opts: Vec<Option<&str>> = day_first.iter().map(|v| Some(*v)).collect();
    let veto =
        finetype_core::evaluate_validation_veto("datetime.date.compact_dmy", &opts, &tax, &safe);
    assert!(
        !veto.vetoed,
        "a column of genuine DD-MM-YYYY dates must NOT be vetoed off compact_dmy — a century \
         year window strips its date transform and ships it as a confident integer; \
         pass_rate = {:?}",
        veto.pass_rate
    );
    assert_eq!(
        veto.pass_rate,
        Some(1.0),
        "genuine day-first dates, nineteenth- and eighteenth-century ones included, must pass \
         the tightened validator at 100%"
    );
}

/// What this tightening does NOT reach, named so nobody reads the leaf as clean.
///
/// The day-first ordering puts the two windowed fields in the HIGH-order digits.
/// A surrogate key whose leading four digits happen to read as a legal day and
/// month therefore validates at 1.0 however implausible the remaining four are —
/// the range validator has no opinion about the year, by design. The year-first
/// sibling does not have this weakness: there the windows land on the LOW-order
/// digits, which is exactly where sequential keys vary, so they bite.
///
/// `10026387` is a real corpus value from an `ID` column of descending
/// sequential keys that types `datetime.date.compact_dmy` at confidence 0.9954
/// with a `%d%m%Y` transform attached, on BOTH sides of this change. It reads
/// 10-02-6387. This test asserts the limit as a fact about the pattern rather
/// than freezing the wrong emitted label as an expectation: if a later change
/// makes the value reject, this test reddens and should be deleted along with
/// the caveat in the taxonomy comment.
#[test]
fn ptc_compact_dmy_names_the_residual_the_windows_cannot_reach() {
    let tax = load_taxonomy();
    let label = "datetime.date.compact_dmy";
    // Real corpus `ID` values. The first two are reachable by neither window
    // (10-02-…); the last two ARE reached, by the month window (10-82-…,
    // 10-32-…), so this case states both halves of the limit rather than only
    // the embarrassing one.
    for surrogate_key in ["10026387", "10026386", "10825933", "10322168"] {
        assert_eq!(
            surrogate_key.len(),
            8,
            "{surrogate_key:?} is not an eight-digit corpus value — a typo here would silently \
             weaken the case"
        );
        let r = validate_value_for_label(surrogate_key, label, &tax).expect("validate key");
        let day: u32 = surrogate_key[..2].parse().expect("day digits");
        let month: u32 = surrogate_key[2..4].parse().expect("month digits");
        let reachable = !(1..=31).contains(&day) || !(1..=12).contains(&month);
        assert_eq!(
            r.is_valid, !reachable,
            "{surrogate_key:?} (day {day}, month {month}): a day-first range validator can only \
             reject an eight-digit surrogate key whose LEADING four digits are not a legal \
             day and month. This test states that limit; it is not an endorsement."
        );
    }
}

/// The audited-safe allowlist and the script that generates it must not drift.
///
/// `labels/veto_safe.txt` is generated by `scripts/false_veto_sweep.py` and says
/// so in its own header, but the sweep needs a 315 MB corpus and a `_before`
/// snapshot, so in practice a label is added by editing the generator's
/// exception list and applying the same line to the file. Nothing enforced that
/// the two agreed. This test does: every label the generator declares as an
/// exception must be in the shipped allowlist, so a re-run of the sweep cannot
/// silently drop one and a hand-edit cannot silently add one.
#[test]
fn ptc_veto_safe_allowlist_contains_every_generator_declared_exception() {
    let root = workspace_root();
    let generator = std::fs::read_to_string(root.join("scripts/false_veto_sweep.py"))
        .expect("read scripts/false_veto_sweep.py");
    let safe = finetype_core::audited_safe_labels();

    let mut declared: Vec<String> = Vec::new();
    for constant in ["VETO_SAFE_EXCEPTIONS", "GATE_VALIDATED_EXCEPTIONS"] {
        let start = generator
            .find(&format!("{constant} = ["))
            .unwrap_or_else(|| panic!("{constant} not found in the generator"));
        let end = generator[start..]
            .find(']')
            .unwrap_or_else(|| panic!("{constant} list is unterminated"));
        for chunk in generator[start..start + end].split('"').skip(1).step_by(2) {
            declared.push(chunk.to_string());
        }
    }
    assert!(
        declared.len() >= 4,
        "parsed only {declared:?} from the generator — the constant format changed and this \
         test is no longer reading it"
    );
    for label in &declared {
        assert!(
            safe.contains(label),
            "{label} is declared an exception in scripts/false_veto_sweep.py but is missing \
             from labels/veto_safe.txt — the generator and its output have drifted"
        );
    }
}
