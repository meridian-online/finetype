//! ISO 3166-1 alpha-2 + US state alpha-2 enums for the R26 Sharpen rule.
//!
//! Used by `value_sharpen()` in `column.rs` to promote misclassified
//! country_code columns while avoiding false promotion of US-state columns
//! whose codes happen to coincide with ISO 3166 codes (CA, IL, GA, LA, MS,
//! MD, DE, IN, VA, KY, AL — 17+ codes overlap).
//!
//! Hardcoded rather than read from `labels/definitions_geography.yaml`
//! because the yaml enum is currently contaminated with US state and
//! Canadian province codes (see memory `taxonomy-country-code-enum-contamination`).
//! Cleaning the yaml is out-of-scope for v23; this constant is the
//! authoritative source for R26.

use std::collections::HashSet;
use std::sync::OnceLock;

/// Canonical ISO 3166-1 alpha-2 codes (249 entries).
///
/// Source: ISO 3166-1 alpha-2 as of 2024. Add new codes here on the rare
/// occasion ISO updates the standard (last full revision: South Sudan, 2011).
const ISO_3166_1_ALPHA_2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX",
    "AZ", "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ",
    "BR", "BS", "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK",
    "CL", "CM", "CN", "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM",
    "DO", "DZ", "EC", "EE", "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR",
    "GA", "GB", "GD", "GE", "GF", "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS",
    "GT", "GU", "GW", "GY", "HK", "HM", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN",
    "IO", "IQ", "IR", "IS", "IT", "JE", "JM", "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN",
    "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV",
    "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK", "ML", "MM", "MN", "MO", "MP", "MQ",
    "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA", "NC", "NE", "NF", "NG", "NI",
    "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG", "PH", "PK", "PL", "PM",
    "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW", "SA", "SB", "SC",
    "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS", "ST", "SV",
    "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO", "TR",
    "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

/// US state + DC + territory alpha-2 codes (56 entries).
///
/// 50 states + DC + 5 unincorporated territories (AS American Samoa,
/// GU Guam, MP Northern Mariana Islands, PR Puerto Rico, VI US Virgin
/// Islands). The territories also appear in ISO 3166-1 alpha-2 (they're
/// US-administered but listed as separate entries in ISO) — that's
/// expected overlap, not a bug.
const US_STATE_ALPHA_2: &[&str] = &[
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA",
    "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ",
    "NM", "NY", "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT",
    "VA", "WA", "WV", "WI", "WY", "DC", "AS", "GU", "MP", "PR", "VI",
];

/// O(1) ISO 3166-1 alpha-2 membership lookup. Initialised on first call.
pub fn iso_3166_alpha2_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| ISO_3166_1_ALPHA_2.iter().copied().collect())
}

/// O(1) US state alpha-2 membership lookup. Initialised on first call.
pub fn us_state_alpha2_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| US_STATE_ALPHA_2.iter().copied().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_set_size_matches_constant() {
        assert_eq!(iso_3166_alpha2_set().len(), ISO_3166_1_ALPHA_2.len());
        assert_eq!(ISO_3166_1_ALPHA_2.len(), 249);
    }

    #[test]
    fn us_state_set_size_matches_constant() {
        assert_eq!(us_state_alpha2_set().len(), US_STATE_ALPHA_2.len());
        assert_eq!(US_STATE_ALPHA_2.len(), 56);
    }

    #[test]
    fn overlap_codes_present_in_both() {
        // Sanity check: codes that should appear in both enums (the
        // ambiguous "state_overlap" set R26 must handle carefully).
        let iso = iso_3166_alpha2_set();
        let states = us_state_alpha2_set();
        for code in ["CA", "IL", "GA", "LA", "MS", "MD", "DE", "IN", "VA", "KY", "AL"] {
            assert!(iso.contains(code), "{code} missing from ISO set");
            assert!(states.contains(code), "{code} missing from US-state set");
        }
    }

    #[test]
    fn state_only_codes_absent_from_iso() {
        // The state_only signal R26 keys off — these are unambiguously
        // US state codes (no ISO 3166 collision). Note: PA=Panama and
        // NC=New Caledonia ARE in ISO, so they live in the overlap set —
        // do not put them here.
        let iso = iso_3166_alpha2_set();
        for code in ["NY", "TX", "FL", "OK", "OH", "MI", "WA", "WI", "OR", "AK"] {
            assert!(!iso.contains(code), "{code} unexpectedly present in ISO set");
        }
    }

    #[test]
    fn iso_only_codes_absent_from_states() {
        // Real ISO codes that don't collide with any US state. Note:
        // DE=Germany IS Delaware, IN=India IS Indiana, KY=Cayman IS
        // Kentucky — those are overlap codes, not iso-only.
        let states = us_state_alpha2_set();
        for code in ["GB", "FR", "JP", "BR", "AU", "MX", "ZA", "CN", "RU", "NZ"] {
            assert!(!states.contains(code), "{code} unexpectedly in US-state set");
        }
    }
}
