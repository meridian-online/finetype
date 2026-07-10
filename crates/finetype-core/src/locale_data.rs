//! Locale-specific data tables for synthetic data generation.
//!
//! Provides per-locale first names, last names, country names, city names,
//! month names, weekday names, street data, and postal code formats.
//!
//! Locales follow the taxonomy convention:
//! - Base locales: EN, DE, FR, ES, IT, NL, PL, RU, JA, ZH, KO, AR
//! - Regional EN: EN_AU, EN_GB, EN_CA, EN_US
//! - CLDR-sourced: BG, CS, DA, EL, ET, FI, HR, HU, LT, LV,
//!   NO, PT, PT_BR, RO, SK, SL, SV, TR, UK

mod address;
mod datetime;
mod geography;
mod names;

pub use address::*;
pub use datetime::*;
pub use geography::*;
pub use names::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_locales_have_names() {
        let locales = [
            "EN", "EN_US", "EN_AU", "EN_GB", "EN_CA", "DE", "FR", "ES", "IT", "NL", "PL", "RU",
            "JA", "ZH", "KO", "AR",
        ];
        for locale in &locales {
            assert!(
                !first_names(locale).is_empty(),
                "No first names for {}",
                locale
            );
            assert!(
                !last_names(locale).is_empty(),
                "No last names for {}",
                locale
            );
        }
    }

    #[test]
    fn test_all_locales_have_months() {
        let locales = [
            "EN", "DE", "FR", "ES", "IT", "NL", "PL", "RU", "JA", "ZH", "KO", "AR",
        ];
        for locale in &locales {
            assert_eq!(
                month_names(locale).len(),
                12,
                "Wrong month count for {}",
                locale
            );
            assert_eq!(
                month_abbreviations(locale).len(),
                12,
                "Wrong abbrev count for {}",
                locale
            );
        }
    }

    #[test]
    fn test_all_locales_have_weekdays() {
        let locales = [
            "EN", "DE", "FR", "ES", "IT", "NL", "PL", "RU", "JA", "ZH", "KO", "AR",
        ];
        for locale in &locales {
            assert_eq!(
                weekday_names(locale).len(),
                7,
                "Wrong weekday count for {}",
                locale
            );
            assert_eq!(
                weekday_abbreviations(locale).len(),
                7,
                "Wrong weekday abbrev count for {}",
                locale
            );
        }
    }

    #[test]
    fn test_base_locale() {
        assert_eq!(base_locale("EN_AU"), "EN");
        assert_eq!(base_locale("EN_US"), "EN");
        assert_eq!(base_locale("DE"), "DE");
        assert_eq!(base_locale("FR"), "FR");
    }
}
