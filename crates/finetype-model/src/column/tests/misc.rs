use super::super::*;

// test_age_column_not_detected_as_port: REMOVED (port type removed)
// test_age_column_with_mixed_values_not_port: REMOVED (port type removed)

#[test]
fn test_empty_column() {
    // Just test the ColumnResult for empty case
    let result = ColumnResult {
        label: "unknown".to_string(),
        confidence: 0.0,
        vote_distribution: vec![],
        disambiguation_applied: false,
        disambiguation_rule: None,
        samples_used: 0,
        detected_locale: None,
        is_generic: false,
        column_features: None,
    };
    assert_eq!(result.label, "unknown");
    assert_eq!(result.samples_used, 0);
    assert_eq!(result.detected_locale, None);
}

// === currency-amount bare-number veto ===

#[test]
fn amount_bare_number_gate() {
    // bare accounting integers (the false-positive shape) → demote, integer
    let (bare, dec) = values_look_like_bare_numbers(&[
        "84000000".into(),
        "-14638000".into(),
        "795000000".into(),
        "0".into(),
    ]);
    assert!(bare && !dec);
    // bare decimals → demote, to decimal
    let (bare, dec) = values_look_like_bare_numbers(&[
        "-270269883.0".into(),
        "396458184.0".into(),
        "2701751944.0".into(),
    ]);
    assert!(bare && dec);
    // currency-symbol / formatted money → NOT bare (kept as amount)
    let (bare, _) =
        values_look_like_bare_numbers(&["£45.17".into(), "£23.88".into(), "£35.02".into()]);
    assert!(!bare);
    let (bare, _) = values_look_like_bare_numbers(&[
        "EUR 4 459 807".into(),
        "EUR 4 626 565".into(),
        "EUR 4 652 581".into(),
    ]);
    assert!(!bare);
    // below the 3-value floor
    let (bare, _) = values_look_like_bare_numbers(&["100".into(), "200".into()]);
    assert!(!bare);
}

#[test]
fn amount_bare_number_veto_is_default_on() {
    assert!(!rhh::is_disabled("amount_bare_number_veto"));
}

#[test]
fn url_bare_number_veto_is_default_on() {
    assert!(!rhh::is_disabled("url_bare_number_veto"));
}

#[test]
fn url_bare_number_gate() {
    // 0/1/-1 flag columns the model mislabels as url → bare integers, demote
    let (bare, dec) =
        values_look_like_bare_numbers(&["0".into(), "1".into(), "0".into(), "-1".into()]);
    assert!(bare && !dec);
    // genuine URLs are non-numeric → not bare, kept as url
    let (bare, _) = values_look_like_bare_numbers(&[
        "https://example.com/a".into(),
        "http://foo.org".into(),
        "https://bar.net/x".into(),
    ]);
    assert!(!bare);
}

#[test]
fn values_are_clearly_non_url_separates_ids_from_urls() {
    // spec 2026-06-25-sharpen-stage-audit: the url header-hint corroboration uses a
    // value-SHAPE test (not the validator) so it keeps all three url forms gold
    // counts as url, and fires only on positive evidence of non-url-ness.
    let v = |xs: &[&str]| xs.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    // bare ids / prose / flags -> clearly non-url (block the hint)
    assert!(values_are_clearly_non_url(&v(&[
        "msg32812262",
        "msg32929450",
        "msg11"
    ])));
    assert!(values_are_clearly_non_url(&v(&["Yes", "Yes", "Yes", "No"])));
    // real urls — all three forms — are NOT clearly non-url (hint may stand)
    assert!(!values_are_clearly_non_url(&v(&[
        "http://a.com/x",
        "https://b.io/y",
        "http://c.org/z"
    ])));
    assert!(!values_are_clearly_non_url(&v(&[
        "//cdn.a.io/x.js",
        "//cdn.b.io/y.css",
        "//c.io/z.js"
    ])));
    assert!(!values_are_clearly_non_url(&v(&[
        "/partner/x.asp?id=1",
        "/partner/y.asp?id=2",
        "/partner/z.asp?id=3"
    ])));
    // too few values -> inconclusive: a SINGLE clearly-non-url value still returns
    // false, so the verdict comes from the count floor alone (a `msg…` id would be
    // "clearly non-url" with >=3 values — see above — but one truncated compose
    // sample is not enough evidence to block the hint). Uses a non-url value on
    // purpose: a url value here would pass for the wrong reason (it's url-shaped),
    // masking the floor.
    assert!(!values_are_clearly_non_url(&v(&["msg32812262"])));
    // and the same id WITH enough values IS clearly non-url — proving it was the
    // count, not the shape, that spared the single-value case.
    assert!(values_are_clearly_non_url(&v(&[
        "msg32812262",
        "msg32929450",
        "msg11"
    ])));
}
