//! Closed-set membership for enumerable identifier types (Precision Principle).
//!
//! Some identifier types are drawn from a published, enumerable code list —
//! a column of arbitrary 4-letter tokens the same *shape* as an ICAO airport
//! code is not a column of airport codes (`^[A-Z]{4}$` confirms every
//! 4-letter stock ticker). Where a check digit exists the `checksum:`
//! directive supplies substance; where the type's substance IS list
//! membership, this module supplies it.
//!
//! Sets live in `labels/sets/*.txt` (one code per line, `#` comments,
//! provenance header) and are embedded at compile time. Each future
//! set-backed type enrols by adding its file, one entry in [`resolve`], and a
//! one-line `membership:` directive in its YAML — no new bespoke veto. Small
//! sets (≲200 entries, e.g. ISO-4217 currency codes) stay inline YAML
//! `enum`s; this module is for lists too large to live readably in a
//! definition file.
//!
//! Like `checksum:`, membership is deliberately NOT folded into the compiled
//! `validation` validator: the generic schema-demotion rules also consult
//! that validator and would demote non-member columns to a worse fallback
//! than the dedicated guard's target (see `checksum_substance_guard`). The
//! validator stays shape-only; the model's `membership_substance_guard` owns
//! the set check. Lookups trim and case-fold to UPPERCASE.

use std::collections::HashSet;
use std::sync::OnceLock;

const ICAO_AIRPORTS_RAW: &str = include_str!("../../../labels/sets/icao_airport_codes.txt");
const IATA_AIRPORTS_RAW: &str = include_str!("../../../labels/sets/iata_airport_codes.txt");

fn parse_set(raw: &'static str) -> HashSet<&'static str> {
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect()
}

fn icao_airports_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(ICAO_AIRPORTS_RAW))
}

fn iata_airports_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(IATA_AIRPORTS_RAW))
}

/// `true` when the value (trimmed, case-folded to uppercase) is a published
/// ICAO airport code.
pub fn icao_airports(value: &str) -> bool {
    icao_airports_set().contains(value.trim().to_uppercase().as_str())
}

/// `true` when the value (trimmed, case-folded to uppercase) is a published
/// IATA airport code.
pub fn iata_airports(value: &str) -> bool {
    iata_airports_set().contains(value.trim().to_uppercase().as_str())
}

/// Resolve a `membership:` directive name to its membership function.
///
/// Returns `None` for an unknown name so the caller can surface the typo.
/// Add a match arm here (plus the set file) to enrol a new set-backed type.
pub fn resolve(name: &str) -> Option<fn(&str) -> bool> {
    match name {
        "icao_airports" => Some(icao_airports),
        "iata_airports" => Some(iata_airports),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icao_set_loads_and_matches_known_airports() {
        for code in ["KJFK", "KLAX", "LFPG", "EGLL", "RJTT", "YSSY"] {
            assert!(icao_airports(code), "{code} should be a known ICAO code");
        }
    }

    #[test]
    fn icao_rejects_ticker_shaped_nonmembers() {
        // 4-letter stock tickers pass the shape pattern ^[A-Z]{4}$ but are not
        // airports — the collision this set exists to break (company-reference
        // audit W2). GOOG/LRCX are excluded from the probe: amusingly, both ARE
        // published ICAO idents.
        for ticker in ["AAPL", "MSFT", "TSLA", "NVDA", "AMZN", "META", "NFLX"] {
            assert!(!icao_airports(ticker), "{ticker} must not validate as ICAO");
        }
    }

    #[test]
    fn iata_set_loads_and_matches_known_airports() {
        for code in ["JFK", "LAX", "CDG", "LHR", "HND", "SYD"] {
            assert!(iata_airports(code), "{code} should be a known IATA code");
        }
    }

    #[test]
    fn iata_rejects_nonmember_codes() {
        // The IATA set covers ~52% of the 3-letter space (9,056/17,576), so
        // membership is a weaker discriminator than ICAO's 2.2% density — many
        // currency codes and tickers ARE real airports (GBP, JPY, CHF, AUD,
        // IBM, SAP…). The guard is demote-only with a ≥50% keep bar, so dense
        // collisions are left alone (no worse than the shape-only status quo);
        // these probes are verified NON-members it can act on.
        for code in ["USD", "EUR", "NZD", "INR", "RUB", "NKE", "PFE", "CVX"] {
            assert!(!iata_airports(code), "{code} must not validate as IATA");
        }
    }

    #[test]
    fn lookups_trim_and_case_fold() {
        assert!(icao_airports(" kjfk "));
        assert!(iata_airports("jfk"));
    }

    #[test]
    fn comments_and_blanks_are_not_members() {
        assert!(!icao_airports("#"));
        assert!(!icao_airports(""));
    }

    #[test]
    fn resolve_known_and_unknown() {
        assert!(resolve("icao_airports").is_some());
        assert!(resolve("iata_airports").is_some());
        assert!(resolve("no_such_set").is_none());
    }
}
