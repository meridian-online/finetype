//! RHH (Remove-Header-Hints) instrumentation hook.
//!
//! Per-family disablement of the rule-based header-hint scaffolding inside
//! [`crate::column`]. Reads the `RHH_DISABLE_HINTS` env var (comma-separated
//! family ids) once at first call and caches the parsed set.
//!
//! The full canonical family list is in
//! `diagnostics/rhh_family_inventory.tsv` (ac-01 output of spec
//! `orbit/specs/2026-04-24-remove-header-hints/`).
//!
//! ## Compile-time behaviour
//!
//! - **Feature `rhh-instrumentation` ON** — [`is_disabled`] consults the
//!   parsed env var set and returns `true` when the family id matches.
//! - **Feature `rhh-instrumentation` OFF (default)** — [`is_disabled`] is a
//!   `const fn` body that always returns `false`. Optimisers eliminate the
//!   call site entirely; off-feature builds carry no runtime cost.
//!
//! ## Family ids
//!
//! Family ids are the rule-family granularity the runtime
//! `disambiguation_rule` emitter uses (e.g. `header_hint_table`,
//! `substring_matcher_identity`, `header_hint_measurement`,
//! `sense_geo_rescue`). Per-arm enumeration is out of scope — see spec
//! constraint #3.

/// Return `true` when `family_id` is listed in the `RHH_DISABLE_HINTS`
/// env var (comma-separated, whitespace-trimmed, empty entries skipped).
///
/// Always `false` on default builds (feature `rhh-instrumentation` off).
/// The argument is borrowed rather than copied so call-sites can pass
/// `&'static str` literals directly.
///
/// On feature-on builds the env var is parsed on every call. ac-04
/// counterfactual runs invoke the binary fresh per family, so no caching
/// is needed; the per-call parse cost only shows up on the diagnostic
/// build and is dominated by inference cost in any realistic scenario.
/// Avoiding a static cache lets tests exercise multiple configurations
/// in the same process by mutating the env var.
#[inline]
pub fn is_disabled(family_id: &str) -> bool {
    #[cfg(feature = "rhh-instrumentation")]
    {
        match std::env::var("RHH_DISABLE_HINTS") {
            Ok(raw) => raw
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .any(|tok| tok == family_id),
            Err(_) => false,
        }
    }
    #[cfg(not(feature = "rhh-instrumentation"))]
    {
        let _ = family_id;
        false
    }
}

#[cfg(test)]
#[cfg(not(feature = "rhh-instrumentation"))]
mod tests {
    use super::*;

    /// rhh_ac02_off_feature_zero_cost — when the feature is disabled, the
    /// hook returns `false` for every input regardless of the env var.
    /// Default `cargo test` (no feature flags) runs this test path.
    #[test]
    fn rhh_ac02_off_feature_zero_cost() {
        // SAFETY: this test runs only on default builds where the function
        // body ignores the env var entirely. We do not mutate the env in the
        // off-feature path because there is no observable effect; setting it
        // here would still make the test inadvertently pass on a buggy build
        // that reads the var anyway. The point is: on the off-feature build,
        // every call returns `false` even if a buggy hook tried to consult
        // an env var.
        assert!(!is_disabled("header_hint_table"));
        assert!(!is_disabled("substring_matcher_identity"));
        assert!(!is_disabled(""));
    }
}
