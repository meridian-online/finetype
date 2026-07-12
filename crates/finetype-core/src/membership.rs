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

/// A closed-set membership predicate: `true` when the value belongs to the set.
type MembershipFn = fn(&str) -> bool;

const ICAO_AIRPORTS_RAW: &str = include_str!("../data/sets/icao_airport_codes.txt");
const IATA_AIRPORTS_RAW: &str = include_str!("../data/sets/iata_airport_codes.txt");
const NAICS_CODES_RAW: &str = include_str!("../data/sets/naics_codes.txt");
const TLD_CODES_RAW: &str = include_str!("../data/sets/tld_codes.txt");
const UNLOCODE_CODES_RAW: &str = include_str!("../data/sets/unlocode_codes.txt");
const ISO_3166_2_CODES_RAW: &str = include_str!("../data/sets/iso_3166_2_codes.txt");
const US_TICKERS_RAW: &str = include_str!("../data/sets/us_tickers.txt");

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

fn naics_codes_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(NAICS_CODES_RAW))
}

fn tld_codes_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(TLD_CODES_RAW))
}

fn unlocode_codes_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(UNLOCODE_CODES_RAW))
}

fn iso_3166_2_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(ISO_3166_2_CODES_RAW))
}

fn us_tickers_set() -> &'static HashSet<&'static str> {
    static SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    SET.get_or_init(|| parse_set(US_TICKERS_RAW))
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

/// `true` when the value (trimmed) is a published NAICS 2022 code at any
/// level (2-digit sector through 6-digit national industry).
pub fn naics_codes(value: &str) -> bool {
    naics_codes_set().contains(value.trim())
}

/// `true` when the value (trimmed, case-folded to lowercase) is an IANA
/// delegated top-level domain. TLDs are conventionally lowercase and the set
/// is stored lowercase (unlike the airport sets, which fold to uppercase);
/// punycode IDN TLDs (`xn--…`) are members, Unicode IDN forms are not.
pub fn tld_codes(value: &str) -> bool {
    tld_codes_set().contains(value.trim().to_lowercase().as_str())
}

/// `true` when the value (trimmed, case-folded to uppercase) is a published
/// UN/LOCODE (2-letter ISO-3166-1 country + 3-char location). The shape
/// `^[A-Z]{2}[A-Z2-9]{3}$` confirms every 5-char uppercase token — a stock
/// ticker or arbitrary code that clears the shape is not a location; the
/// published list is what distinguishes them (company-reference audit ticker
/// finding).
pub fn unlocode(value: &str) -> bool {
    unlocode_codes_set().contains(value.trim().to_uppercase().as_str())
}

/// `true` when the value (trimmed, case-folded to uppercase) is a published
/// ISO 3166-2 subdivision code (`CC-SSS`: an ISO-3166-1 country prefix and a
/// 1-3 char subdivision suffix, e.g. `US-PA`, `GB-ENG`). The shape
/// `^[A-Z]{2}-[A-Z0-9]{1,3}$` confirms many non-subdivision hyphenated codes
/// (product/OS/locale tokens) — the published list is what distinguishes a
/// real subdivision, so `geo_subdivision_membership_promote` keys on membership,
/// not shape (external-band iso_region → unknown finding).
pub fn iso_3166_2(value: &str) -> bool {
    iso_3166_2_set().contains(value.trim().to_uppercase().as_str())
}

/// `true` when the value (trimmed, case-folded to uppercase) is a US-listed
/// stock ticker. A ticker has no check digit and its shape `^[A-Z]{1,7}$`
/// confirms every short uppercase token — a US state code (`MA`, `TX`), a
/// currency, or an arbitrary code all clear it — so the published SEC list is
/// the substance (company-reference audit `ticker` → state_code finding). The
/// two-letter state/ticker collision (15 of 50 state codes are also tickers)
/// is why the promote guard also gates on a ticker-ish header, not membership
/// alone. Set is US-only for now (SEC `company_tickers.json`); other venues
/// compose additively behind the same `finance.securities.ticker` type.
pub fn us_tickers(value: &str) -> bool {
    us_tickers_set().contains(value.trim().to_uppercase().as_str())
}

/// Every named membership set as `(directive_name, predicate)`, in a stable
/// order. Single source of truth: [`resolve`] looks up its arm here, and
/// diagnostics that must check a value or column against *all* sets (e.g. the
/// multi-set collision analysis behind the set-vs-set voting question) iterate
/// it — so the two can never drift. Enrol a new set-backed type by adding its
/// file and one row here.
pub fn all_sets() -> &'static [(&'static str, MembershipFn)] {
    &[
        ("icao_airports", icao_airports),
        ("iata_airports", iata_airports),
        ("naics_codes", naics_codes),
        ("tld", tld_codes),
        ("unlocode", unlocode),
        ("iso_3166_2", iso_3166_2),
        ("us_tickers", us_tickers),
    ]
}

/// Resolve a `membership:` directive name to its membership function.
///
/// Returns `None` for an unknown name so the caller can surface the typo.
/// Add a row to [`all_sets`] (plus the set file) to enrol a new set-backed type.
pub fn resolve(name: &str) -> Option<MembershipFn> {
    all_sets().iter().find(|(n, _)| *n == name).map(|(_, f)| *f)
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
        assert!(resolve("naics_codes").is_some());
        assert!(resolve("tld").is_some());
        assert!(resolve("unlocode").is_some());
        assert!(resolve("iso_3166_2").is_some());
        assert!(resolve("us_tickers").is_some());
        assert!(resolve("no_such_set").is_none());
    }

    #[test]
    fn resolve_covers_every_all_sets_entry() {
        // resolve() is defined over all_sets(); guarantee they stay in lock-step
        // so a diagnostic iterating all_sets and the guard calling resolve agree.
        for (name, _) in all_sets() {
            assert!(resolve(name).is_some(), "{name} must resolve");
        }
    }

    #[test]
    fn unlocode_set_matches_real_locodes_and_rejects_tickers() {
        for code in ["USLAX", "GBLON", "DEHAM", "SGSIN", "NLRTM", " uslax "] {
            assert!(unlocode(code), "{code} should be a known UN/LOCODE");
        }
        // 5-char uppercase tokens that clear the shape `^[A-Z]{2}[A-Z2-9]{3}$`
        // but are stock tickers, not locations — the collision this set breaks
        // (dataset-descriptor audit: EDGAR `ticker` → unlocode). Shorter tickers
        // (AAPL/TXG) never match the 5-char shape; these 5-char ones do.
        for ticker in ["GOOGL", "BRKAX", "AMZNX", "ZZZZZ"] {
            assert!(!unlocode(ticker), "{ticker} must not validate as UN/LOCODE");
        }
    }

    #[test]
    fn tld_set_matches_real_domains_and_rejects_words() {
        // Real delegated TLDs (case-insensitive) including a punycode IDN.
        for code in ["com", "org", "UK", "io", " net ", "xn--p1ai"] {
            assert!(tld_codes(code), "{code} should be an IANA TLD");
        }
        // Same lowercase-word shape the `^[a-z]{2,}$` pattern confirms, but not
        // delegated TLDs — the collision this set breaks. (The new-gTLD program
        // delegated many dictionary words — `.data`, `.app`, `.blog` — so the
        // probes are verified non-members, not just "looks like a word".)
        for word in ["notatld", "zzzz", "qwerty", "foobar", "xyzzy"] {
            assert!(!tld_codes(word), "{word} must not validate as a TLD");
        }
    }

    #[test]
    fn iso_3166_2_matches_real_subdivisions_and_rejects_lookalikes() {
        // Real ISO-3166-2 codes across several countries (case-insensitive).
        for code in ["US-PA", "GB-ENG", "CA-ON", "AU-NSW", "DE-BW", " us-ca "] {
            assert!(iso_3166_2(code), "{code} should be an ISO-3166-2 code");
        }
        // Same `CC-SSS` shape, NOT real subdivisions — the collision this set
        // breaks (external band: product/OS/locale hyphen-codes → geography).
        for code in ["US-ZZ", "XX-01", "AB-99", "ZZ-ZZ", "OS-10"] {
            assert!(!iso_3166_2(code), "{code} must not validate as ISO-3166-2");
        }
    }

    #[test]
    fn us_tickers_match_real_symbols_and_reject_nonmembers() {
        // Real US-listed symbols (case-insensitive) incl. a dash class-share.
        for code in ["AAPL", "NVDA", "GOOGL", "BRK-B", " msft ", "tsla"] {
            assert!(us_tickers(code), "{code} should be a US ticker");
        }
        // Same short-uppercase shape, NOT listed symbols — the collisions this
        // set exists to break: a UN/LOCODE-shaped token, an arbitrary code, and
        // an unassigned symbol. (State codes like MA/TX genuinely ARE tickers,
        // so the header gate — not membership — resolves the state column.)
        for non in ["ZZZZZ", "QQQQXY", "NOTATICKER"] {
            assert!(!us_tickers(non), "{non} must not validate as a US ticker");
        }
    }

    #[test]
    fn naics_set_matches_real_codes_and_rejects_lookalikes() {
        for code in ["11", "92", "541511", "236220", "722511"] {
            assert!(naics_codes(code), "{code} should be a NAICS code");
        }
        // Same digit-shape, not NAICS: sector prefix 10/93-99 or unassigned
        // 6-digit combinations.
        for code in ["100000", "999999", "541599999", "05", "541510"] {
            assert!(!naics_codes(code), "{code} must not validate as NAICS");
        }
    }
}
