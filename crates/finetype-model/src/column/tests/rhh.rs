use super::super::*;

// ── RHH instrumentation tests (ac-02) ───────────────────────────────
//
// Default build: prove behaviour is unchanged — header_hint() returns the
// same family-tag for every header it used to fire on. This is the
// "default cargo test compiles and passes" half of ac-02 verification.

// Default-build invariants — gated to default builds only because the
// on-feature test (`rhh_ac02_on_feature_disable_scenarios`) mutates
// `RHH_DISABLE_HINTS`, and Cargo runs unit tests concurrently within a
// single test binary. The on-feature test is self-contained: it sets
// env vars to assert the disable mechanic, then restores them. These
// baseline tests instead assert that on a default build (feature off),
// the env var is read-through-noop and behaviour is unchanged.

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_email_match_table_unchanged() {
    // "email" in the match table → identity.person.email
    assert_eq!(header_hint("email"), Some("identity.person.email"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_phone_substring_unchanged() {
    // "phone" only fires through the substring matcher (not in match table)
    assert_eq!(header_hint("phone"), Some("identity.person.phone_number"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_zip_geography_unchanged() {
    assert_eq!(header_hint("zip"), Some("geography.address.postal_code"));
}

#[test]
#[cfg(not(feature = "rhh-instrumentation"))]
fn rhh_ac02_default_build_env_var_ignored() {
    // Even if RHH_DISABLE_HINTS is set, default builds ignore it because
    // rhh::is_disabled compiles to a constant `false`.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "substring_matcher_identity");
    }
    let result = header_hint("phone");
    unsafe {
        std::env::remove_var("RHH_DISABLE_HINTS");
    }
    assert_eq!(result, Some("identity.person.phone_number"));
}

// On-feature tests — gated behind `rhh-instrumentation`. Default
// `cargo test` (no feature) skips these entirely. Run with:
//   cargo test -p finetype-model --features rhh-instrumentation rhh_ac02
//
// SAFETY: env-var mutation is process-global. These tests run
// sequentially and each test sets, asserts, and unsets the env var
// within its own body. They must NOT run concurrently with anything
// else that reads RHH_DISABLE_HINTS — Cargo serialises tests within a
// single test binary by default for unit tests, and the workspace
// configures `--test-threads=1` is not required because each test
// restores RHH_DISABLE_HINTS to its prior state on exit.

/// All on-feature scenarios in one test so the shared `RHH_DISABLE_HINTS`
/// env var is mutated by exactly one thread at a time. Splitting this
/// into multiple `#[test]` functions caused parallel-test interference
/// with the unconditional `rhh_ac02_default_build_*` tests above.
#[test]
#[cfg(feature = "rhh-instrumentation")]
fn rhh_ac02_on_feature_disable_scenarios() {
    let prior = std::env::var("RHH_DISABLE_HINTS").ok();

    let restore = |prior: &Option<String>| match prior {
        Some(v) => unsafe { std::env::set_var("RHH_DISABLE_HINTS", v) },
        None => unsafe { std::env::remove_var("RHH_DISABLE_HINTS") },
    };

    // Scenario 1: disable substring_matcher_identity → "phone" no
    // longer fires (it lives only in the identity substring matcher).
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "substring_matcher_identity");
    }
    assert_eq!(
        header_hint("phone"),
        None,
        "disabling substring_matcher_identity should silence phone hint"
    );

    // Scenario 2: disabling identity must not silence technology hits.
    // (Same env var still set from scenario 1.)
    assert_eq!(
        header_hint("ipv6"),
        Some("technology.internet.ip_v6"),
        "disabling identity must not affect technology"
    );

    // Scenario 3: disable header_hint_table + substring_matcher_identity
    // simultaneously → "email" (which lives in the exact-match table)
    // returns None because the match table is gated and the substring
    // fallback for identity is also gated.
    unsafe {
        std::env::set_var(
            "RHH_DISABLE_HINTS",
            "header_hint_table,substring_matcher_identity",
        );
    }
    assert_eq!(
        header_hint("email"),
        None,
        "match_table+identity disable should silence email"
    );

    // Scenario 4: empty env var = no families disabled → baseline.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "");
    }
    assert_eq!(
        header_hint("phone"),
        Some("identity.person.phone_number"),
        "empty disable list must restore baseline"
    );

    // Scenario 5: whitespace + empty entries are tolerated.
    unsafe {
        std::env::set_var("RHH_DISABLE_HINTS", "  substring_matcher_identity ,, ,");
    }
    assert_eq!(
        header_hint("phone"),
        None,
        "whitespace and empty tokens must parse cleanly"
    );

    restore(&prior);
}
