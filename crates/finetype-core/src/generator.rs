//! Synthetic data generation for all type definitions.
//!
//! Generates synthetic training data using taxonomy keys:
//! `domain.category.type` (e.g., `datetime.timestamp.iso_8601`).
//!
//! Each generator produces strings that match the transformation contract
//! defined in the YAML specification.

use crate::locale_data;
use crate::taxonomy::{Designation, Taxonomy};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rand::prelude::*;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Unknown label: {0}")]
    UnknownLabel(String),
    #[error("Generator not implemented for: {0}")]
    NotImplemented(String),
}

/// Phone number format selection for training data diversity.
enum PhoneFmt {
    National,
    International,
    E164,
}

/// A generated sample with its label.
#[derive(Debug, Clone)]
pub struct Sample {
    pub text: String,
    pub label: String,
}

/// Data generator for creating synthetic training samples.
pub struct Generator {
    taxonomy: Taxonomy,
    rng: StdRng,
    /// Current locale for locale-aware generation (set during generate_all_localized).
    locale: Option<String>,
}

impl Generator {
    /// Create a new generator with the given taxonomy.
    pub fn new(taxonomy: Taxonomy) -> Self {
        Self {
            taxonomy,
            rng: StdRng::from_entropy(),
            locale: None,
        }
    }

    /// Create a generator with a fixed seed for reproducibility.
    pub fn with_seed(taxonomy: Taxonomy, seed: u64) -> Self {
        Self {
            taxonomy,
            rng: StdRng::seed_from_u64(seed),
            locale: None,
        }
    }

    /// Generate samples for all labels at a given priority level.
    pub fn generate_all(&mut self, min_priority: u8, samples_per_label: usize) -> Vec<Sample> {
        let keys: Vec<String> = self
            .taxonomy
            .at_priority(min_priority)
            .into_iter()
            .map(|(k, _)| k.clone())
            .collect();

        let mut all_samples = Vec::new();

        for key in keys {
            for _ in 0..samples_per_label {
                if let Ok(text) = self.generate_value(&key) {
                    all_samples.push(Sample {
                        text,
                        label: key.clone(),
                    });
                }
            }
        }

        all_samples
    }

    /// Generate samples with 4-level labels (domain.category.type.LOCALE).
    ///
    /// For locale_specific types, generates `samples_per_label` samples for EACH locale.
    /// For universal/broad types, generates samples with `.UNIVERSAL` suffix.
    pub fn generate_all_localized(
        &mut self,
        min_priority: u8,
        samples_per_label: usize,
    ) -> Vec<Sample> {
        let entries: Vec<(String, Designation, Vec<String>)> = self
            .taxonomy
            .at_priority(min_priority)
            .into_iter()
            .map(|(k, d)| (k.clone(), d.designation.clone(), d.locales.clone()))
            .collect();

        let mut all_samples = Vec::new();

        for (key, designation, locales) in &entries {
            match designation {
                Designation::LocaleSpecific => {
                    // Generate per-locale samples with 4-level labels
                    for locale in locales {
                        let label = format!("{}.{}", key, locale);
                        self.locale = Some(locale.clone());
                        for _ in 0..samples_per_label {
                            if let Ok(text) = self.generate_value(key) {
                                all_samples.push(Sample {
                                    text,
                                    label: label.clone(),
                                });
                            }
                        }
                    }
                    self.locale = None;
                }
                _ => {
                    // Universal and broad types get .UNIVERSAL suffix
                    let label = format!("{}.UNIVERSAL", key);
                    for _ in 0..samples_per_label {
                        if let Ok(text) = self.generate_value(key) {
                            all_samples.push(Sample {
                                text,
                                label: label.clone(),
                            });
                        }
                    }
                }
            }
        }

        all_samples
    }

    /// Generate a single value for a key (domain.category.type).
    pub fn generate_value(&mut self, key: &str) -> Result<String, GeneratorError> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 3 {
            return Err(GeneratorError::UnknownLabel(key.to_string()));
        }

        let (domain, category, type_name) = (parts[0], parts[1], parts[2]);

        match domain {
            "datetime" => self.gen_datetime(category, type_name),
            "technology" => self.gen_technology(category, type_name),
            "identity" => self.gen_identity(category, type_name),
            "geography" => self.gen_geography(category, type_name),
            "representation" => self.gen_representation(category, type_name),
            "container" => self.gen_container(category, type_name),
            _ => Err(GeneratorError::UnknownLabel(key.to_string())),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: datetime (46 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_datetime(&mut self, category: &str, type_name: &str) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── timestamp (12 types) ─────────────────────────────────────
            ("timestamp", "iso_8601") => Ok(self
                .random_datetime()
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string()),
            ("timestamp", "iso_8601_compact") => {
                Ok(self.random_datetime().format("%Y%m%dT%H%M%S").to_string())
            }
            ("timestamp", "iso_8601_microseconds") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1000000);
                Ok(format!("{}.{:06}Z", dt.format("%Y-%m-%dT%H:%M:%S"), micros))
            }
            ("timestamp", "iso_8601_offset") => {
                let dt = self.random_datetime();
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}{:+03}:00",
                    dt.format("%Y-%m-%dT%H:%M:%S"),
                    offset_h
                ))
            }
            ("timestamp", "rfc_2822") => Ok(self
                .random_datetime()
                .format("%a, %d %b %Y %H:%M:%S +0000")
                .to_string()),
            ("timestamp", "rfc_2822_ordinal") => {
                let dt = self.random_datetime();
                let day = dt.day();
                let ord = match day % 10 {
                    1 if day != 11 => "st",
                    2 if day != 12 => "nd",
                    3 if day != 13 => "rd",
                    _ => "th",
                };
                Ok(format!(
                    "{}{} {} +0000",
                    day,
                    ord,
                    dt.format("%b %Y %H:%M:%S")
                ))
            }
            ("timestamp", "rfc_3339") => {
                // RFC 3339 uses SPACE separator (vs ISO 8601 which uses T)
                let dt = self.random_datetime();
                let offset_h = self.rng.gen_range(-12i32..=12);
                Ok(format!(
                    "{}{:+03}:00",
                    dt.format("%Y-%m-%d %H:%M:%S"),
                    offset_h
                ))
            }
            ("timestamp", "american") => Ok(self
                .random_datetime()
                .format("%m/%d/%Y %I:%M %p")
                .to_string()),
            ("timestamp", "american_24h") => Ok(self
                .random_datetime()
                .format("%m/%d/%Y %H:%M:%S")
                .to_string()),
            ("timestamp", "european") => {
                Ok(self.random_datetime().format("%d/%m/%Y %H:%M").to_string())
            }
            ("timestamp", "iso_microseconds") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1000000);
                Ok(format!("{}.{:06}", dt.format("%Y-%m-%dT%H:%M:%S"), micros))
            }
            ("timestamp", "sql_standard") => Ok(self
                .random_datetime()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()),

            // ── date (17 types) ──────────────────────────────────────────
            ("date", "iso") => Ok(self.random_datetime().format("%Y-%m-%d").to_string()),
            ("date", "us_slash") => Ok(self.random_datetime().format("%m/%d/%Y").to_string()),
            ("date", "eu_slash") => Ok(self.random_datetime().format("%d/%m/%Y").to_string()),
            ("date", "eu_dot") => Ok(self.random_datetime().format("%d.%m.%Y").to_string()),
            ("date", "compact_ymd") => {
                let dt = self.random_datetime();
                Ok(format!("{}{:02}{:02}", dt.year(), dt.month(), dt.day()))
            }
            ("date", "compact_mdy") => {
                let dt = self.random_datetime();
                Ok(format!("{:02}{:02}{}", dt.month(), dt.day(), dt.year()))
            }
            ("date", "compact_dmy") => {
                let dt = self.random_datetime();
                Ok(format!("{:02}{:02}{}", dt.day(), dt.month(), dt.year()))
            }
            ("date", "short_ymd") => Ok(self.random_datetime().format("%y-%m-%d").to_string()),
            ("date", "short_mdy") => Ok(self.random_datetime().format("%m-%d-%y").to_string()),
            ("date", "short_dmy") => Ok(self.random_datetime().format("%d-%m-%y").to_string()),
            ("date", "abbreviated_month") => {
                let dt = self.random_datetime();
                let abbrevs = locale_data::month_abbreviations(self.current_locale());
                let month_abbr = abbrevs[(dt.month0() as usize) % abbrevs.len()];
                Ok(format!("{} {:02}, {}", month_abbr, dt.day(), dt.year()))
            }
            ("date", "long_full_month") => {
                let dt = self.random_datetime();
                let months = locale_data::month_names(self.current_locale());
                let month_name = months[(dt.month0() as usize) % months.len()];
                Ok(format!("{} {:02}, {}", month_name, dt.day(), dt.year()))
            }
            ("date", "weekday_abbreviated_month") => {
                let dt = self.random_datetime();
                let weekdays = locale_data::weekday_names(self.current_locale());
                let abbrevs = locale_data::month_abbreviations(self.current_locale());
                let weekday =
                    weekdays[(dt.weekday().num_days_from_monday() as usize) % weekdays.len()];
                let month_abbr = abbrevs[(dt.month0() as usize) % abbrevs.len()];
                Ok(format!(
                    "{}, {:02} {} {}",
                    weekday,
                    dt.day(),
                    month_abbr,
                    dt.year()
                ))
            }
            ("date", "weekday_full_month") => {
                let dt = self.random_datetime();
                let weekdays = locale_data::weekday_names(self.current_locale());
                let months = locale_data::month_names(self.current_locale());
                let weekday =
                    weekdays[(dt.weekday().num_days_from_monday() as usize) % weekdays.len()];
                let month_name = months[(dt.month0() as usize) % months.len()];
                Ok(format!(
                    "{}, {:02} {} {}",
                    weekday,
                    dt.day(),
                    month_name,
                    dt.year()
                ))
            }
            ("date", "ordinal") => Ok(format!(
                "{}-{:03}",
                self.rng.gen_range(2020..2030),
                self.rng.gen_range(1..366)
            )),
            ("date", "julian") => Ok(format!(
                "{:02}-{:03}",
                self.rng.gen_range(20..30),
                self.rng.gen_range(1..366)
            )),
            ("date", "iso_week") => Ok(format!(
                "{}-W{:02}",
                self.rng.gen_range(2020..2030),
                self.rng.gen_range(1..53)
            )),

            // ── time (5 types) ───────────────────────────────────────────
            ("time", "iso") => {
                let dt = self.random_datetime();
                let micros = self.rng.gen_range(0..1000000);
                Ok(format!("{}.{:06}", dt.format("%H:%M:%S"), micros))
            }
            ("time", "hms_24h") => Ok(self.random_datetime().format("%H:%M:%S").to_string()),
            ("time", "hm_24h") => Ok(self.random_datetime().format("%H:%M").to_string()),
            ("time", "hms_12h") => Ok(self.random_datetime().format("%I:%M:%S %p").to_string()),
            ("time", "hm_12h") => Ok(self.random_datetime().format("%I:%M %p").to_string()),

            // ── epoch (3 types) ──────────────────────────────────────────
            ("epoch", "unix_seconds") => Ok(self
                .rng
                .gen_range(1_000_000_000i64..2_000_000_000)
                .to_string()),
            ("epoch", "unix_milliseconds") => Ok(self
                .rng
                .gen_range(1_000_000_000_000i64..2_000_000_000_000)
                .to_string()),
            ("epoch", "unix_microseconds") => Ok(self
                .rng
                .gen_range(1_000_000_000_000_000i64..2_000_000_000_000_000)
                .to_string()),

            // ── offset (2 types) ─────────────────────────────────────────
            ("offset", "utc") => {
                let h = self.rng.gen_range(-12i32..=14);
                Ok(format!("UTC {:+03}:00", h))
            }
            ("offset", "iana") => {
                let tzs = [
                    "America/New_York",
                    "America/Los_Angeles",
                    "America/Chicago",
                    "Europe/London",
                    "Europe/Paris",
                    "Europe/Berlin",
                    "Asia/Tokyo",
                    "Asia/Shanghai",
                    "Asia/Singapore",
                    "Australia/Sydney",
                    "Pacific/Auckland",
                    "Africa/Cairo",
                ];
                Ok(tzs[self.rng.gen_range(0..tzs.len())].to_string())
            }

            // ── duration (1 type) ────────────────────────────────────────
            ("duration", "iso_8601") => {
                let h = self.rng.gen_range(0..24);
                let m = self.rng.gen_range(0..60);
                let s = self.rng.gen_range(0..60);
                if h > 0 {
                    Ok(format!("PT{}H{}M{}S", h, m, s))
                } else if m > 0 {
                    Ok(format!("PT{}M{}S", m, s))
                } else {
                    Ok(format!("PT{}S", s))
                }
            }

            // ── component (6 types) ──────────────────────────────────────
            ("component", "year") => {
                // Weighted distribution: modern years most common, then historical, then future
                let year = if self.rng.gen_bool(0.60) {
                    // Modern era (60%): 1900-2025
                    self.rng.gen_range(1900..2026)
                } else if self.rng.gen_bool(0.625) {
                    // Historical (25% of total): 1000-1900
                    self.rng.gen_range(1000..1900)
                } else {
                    // Future (15% of total): 2026-2100
                    self.rng.gen_range(2026..2101)
                };
                Ok(year.to_string())
            }
            ("component", "month_name") => {
                let months = locale_data::month_names(self.current_locale());
                Ok(months[self.rng.gen_range(0..months.len())].to_string())
            }
            ("component", "day_of_month") => Ok(self.rng.gen_range(1u32..=31).to_string()),
            ("component", "day_of_week") => {
                let days = locale_data::weekday_names(self.current_locale());
                Ok(days[self.rng.gen_range(0..days.len())].to_string())
            }
            ("component", "century") => {
                let centuries = ["XVIII", "XIX", "XX", "XXI"];
                Ok(centuries[self.rng.gen_range(0..4)].to_string())
            }
            ("component", "periodicity") => {
                let periods = [
                    "Once",
                    "Daily",
                    "Weekly",
                    "Biweekly",
                    "Monthly",
                    "Quarterly",
                    "Yearly",
                    "Never",
                ];
                Ok(periods[self.rng.gen_range(0..periods.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "datetime.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: technology (34 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_technology(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── internet (13 types) ──────────────────────────────────────
            ("internet", "ip_v4") => Ok(format!(
                "{}.{}.{}.{}",
                self.rng.gen_range(1..255),
                self.rng.gen_range(0..255),
                self.rng.gen_range(0..255),
                self.rng.gen_range(1..255)
            )),
            ("internet", "ip_v4_with_port") => Ok(format!(
                "{}.{}.{}.{}:{}",
                self.rng.gen_range(1..255),
                self.rng.gen_range(0..255),
                self.rng.gen_range(0..255),
                self.rng.gen_range(1..255),
                self.rng.gen_range(1024..65535)
            )),
            ("internet", "ip_v6") => {
                let groups: Vec<String> = (0..8)
                    .map(|_| format!("{:04x}", self.rng.gen_range(0u16..65535)))
                    .collect();
                Ok(groups.join(":"))
            }
            ("internet", "mac_address") => {
                let octets: Vec<String> = (0..6)
                    .map(|_| format!("{:02x}", self.rng.gen::<u8>()))
                    .collect();
                Ok(octets.join(":"))
            }
            ("internet", "url") => {
                let tlds = ["com", "org", "net", "io", "dev", "co", "app"];
                let words: Vec<String> = (0..self.rng.gen_range(1..3))
                    .map(|_| self.random_word())
                    .collect();
                let domain = words.join("");
                let tld = tlds[self.rng.gen_range(0..tlds.len())];
                let path_segments: Vec<String> = (0..self.rng.gen_range(1..4))
                    .map(|_| self.random_word())
                    .collect();
                Ok(format!(
                    "https://{}.{}/{}",
                    domain,
                    tld,
                    path_segments.join("/")
                ))
            }
            ("internet", "uri") => {
                let schemes = ["https", "http", "ftp", "mailto", "ssh"];
                let scheme = schemes[self.rng.gen_range(0..schemes.len())];
                if scheme == "mailto" {
                    Ok(format!(
                        "mailto:{}@{}.com",
                        self.random_word(),
                        self.random_word()
                    ))
                } else {
                    Ok(format!(
                        "{}://{}.com/{}",
                        scheme,
                        self.random_word(),
                        self.random_word()
                    ))
                }
            }
            ("internet", "hostname") => {
                let tlds = ["com", "org", "net", "io", "dev"];
                Ok(format!(
                    "{}.{}",
                    self.random_word(),
                    tlds[self.rng.gen_range(0..tlds.len())]
                ))
            }
            ("internet", "port") => {
                // Weighted toward common/well-known ports to distinguish from generic integers
                if self.rng.gen_bool(0.6) {
                    // Well-known ports
                    let common = [
                        22, 25, 53, 80, 110, 143, 443, 465, 587, 993, 995, 3306, 3389, 5432, 5672,
                        5900, 6379, 8080, 8443, 8888, 9090, 9200, 9300, 27017,
                    ];
                    Ok(common[self.rng.gen_range(0..common.len())].to_string())
                } else if self.rng.gen_bool(0.5) {
                    // Registered ports (1024-49151)
                    Ok(self.rng.gen_range(1024..49152).to_string())
                } else {
                    // Ephemeral ports (49152-65535)
                    Ok(self.rng.gen_range(49152..65535).to_string())
                }
            }
            ("internet", "top_level_domain") => {
                let tlds = [
                    "com", "org", "net", "io", "dev", "edu", "gov", "mil", "co.uk", "com.au",
                ];
                Ok(tlds[self.rng.gen_range(0..tlds.len())].to_string())
            }
            ("internet", "slug") => {
                let words: Vec<String> = (0..self.rng.gen_range(2..6))
                    .map(|_| self.random_word())
                    .collect();
                Ok(words.join("-"))
            }
            ("internet", "user_agent") => {
                let agents = [
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
                    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
                    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
                    "curl/8.4.0",
                    "python-requests/2.31.0",
                    "Go-http-client/2.0",
                ];
                Ok(agents[self.rng.gen_range(0..agents.len())].to_string())
            }
            ("internet", "http_method") => {
                let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
                Ok(methods[self.rng.gen_range(0..methods.len())].to_string())
            }
            ("internet", "http_status_code") => {
                let codes = [
                    200, 201, 204, 301, 302, 304, 400, 401, 403, 404, 405, 500, 502, 503,
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }

            // ── cryptographic (4 types) ──────────────────────────────────
            ("cryptographic", "uuid") => Ok(Uuid::new_v4().to_string()),
            ("cryptographic", "hash") => {
                // Generate MD5 (32), SHA1 (40), or SHA256 (64) length hashes
                let lengths = [32, 40, 64];
                let len = lengths[self.rng.gen_range(0..3)];
                Ok(self.gen_hex_string(len))
            }
            ("cryptographic", "token_hex") => {
                // Generate hex tokens at lengths that DON'T match standard hash sizes
                // Avoid: 32 (MD5), 40 (SHA1), 64 (SHA256), 128 (SHA512)
                let hash_lengths = [32, 40, 64];
                let mut len = self.rng.gen_range(16..48);
                while hash_lengths.contains(&len) {
                    len = self.rng.gen_range(16..48);
                }
                Ok(self.gen_hex_string(len))
            }
            ("cryptographic", "token_urlsafe") => {
                // Base64url alphabet: A-Z, a-z, 0-9, -, _
                // Must include - and _ to distinguish from base58 (bitcoin_address)
                let base64url = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
                let len = self.rng.gen_range(22..44);
                let mut token: String = (0..len)
                    .map(|_| base64url.as_bytes()[self.rng.gen_range(0..64)] as char)
                    .collect();
                // Ensure at least one - or _ to distinguish from alphanumeric-only strings
                if !token.contains('-') && !token.contains('_') {
                    let pos = self.rng.gen_range(0..token.len());
                    let special = if self.rng.gen_bool(0.5) { '-' } else { '_' };
                    token.replace_range(pos..pos + 1, &special.to_string());
                }
                Ok(token)
            }

            // ── code (7 types) ───────────────────────────────────────────
            ("code", "isbn") => {
                if self.rng.gen_bool(0.6) {
                    // ISBN-13 (60% of samples)
                    let prefix = if self.rng.gen_bool(0.8) { "978" } else { "979" };
                    let group = self.rng.gen_range(0..9);
                    let publisher = self.rng.gen_range(10000..99999);
                    let title = self.rng.gen_range(100..999);
                    let digits = format!("{}{}{:05}{:03}", prefix, group, publisher, title);
                    let check = self.isbn13_check_digit(&digits);
                    if self.rng.gen_bool(0.6) {
                        // With hyphens
                        Ok(format!(
                            "{}-{}-{:05}-{:03}-{}",
                            prefix, group, publisher, title, check
                        ))
                    } else {
                        // Without hyphens (bare digits)
                        Ok(format!("{}{}", digits, check))
                    }
                } else {
                    // ISBN-10 (40% of samples)
                    let group = self.rng.gen_range(0..9);
                    let publisher = self.rng.gen_range(1000..99999);
                    let title = self.rng.gen_range(10..999);
                    let body = format!("{}{:05}{:03}", group, publisher, title);
                    let check = self.isbn10_check_digit(&body);
                    if self.rng.gen_bool(0.6) {
                        // With hyphens
                        Ok(format!("{}-{:05}-{:03}-{}", group, publisher, title, check))
                    } else {
                        // Without hyphens
                        Ok(format!("{}{}", body, check))
                    }
                }
            }
            ("code", "imei") => {
                // Generate Luhn-valid 15-digit IMEI with realistic TAC prefixes
                // TAC = Type Allocation Code (8 digits identifying manufacturer/model)
                let tacs = [
                    "35332509", "35391109", "35404909", "35648409", // Apple
                    "35290611", "35397710", "35466210", "35195410", // Samsung
                    "35816110", "35837910", "35455610", "35260810", // Google
                    "86109003", "86637303", "86813603", "86930804", // Huawei
                    "86876103", "35780008", "35928509", "35455307", // OnePlus/Sony/LG
                ];
                let tac = tacs[self.rng.gen_range(0..tacs.len())];
                // 6 random serial digits
                let serial: String = (0..6)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                let partial = format!("{}{}", tac, serial); // 14 digits
                let check = self.luhn_check_digit(&partial);
                Ok(format!("{}{}", partial, check))
            }
            ("code", "ean") => {
                if self.rng.gen_bool(0.7) {
                    // EAN-13 with realistic GS1 country prefixes
                    let gs1_prefixes = [
                        "000", "001", "030", "040", // US/Canada
                        "300", "310", "350", "370", // France
                        "400", "410", "420", "440", // Germany
                        "450", "459", // Japan
                        "500", "509", // UK
                        "690", "694", "699", // China
                        "880", // South Korea
                        "890", // India
                        "930", "940", // Australia
                    ];
                    let prefix = gs1_prefixes[self.rng.gen_range(0..gs1_prefixes.len())];
                    let remaining = 12 - prefix.len();
                    let body: String = (0..remaining)
                        .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                        .collect();
                    let partial = format!("{}{}", prefix, body);
                    let check = self.ean_check_digit(&partial);
                    Ok(format!("{}{}", partial, check))
                } else {
                    // EAN-8: 7 digits + check digit
                    let body: String = (0..7)
                        .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                        .collect();
                    let check = self.ean_check_digit(&body);
                    Ok(format!("{}{}", body, check))
                }
            }
            ("code", "issn") => {
                let check_chars = "0123456789X";
                let check = check_chars.chars().nth(self.rng.gen_range(0..11)).unwrap();
                Ok(format!(
                    "{:04}-{:03}{}",
                    self.rng.gen_range(1000..9999),
                    self.rng.gen_range(100..999),
                    check
                ))
            }
            ("code", "doi") => {
                // DOI format: 10.XXXX/suffix
                // Realistic registrant codes from major publishers
                let registrants = [
                    "1038",  // Nature
                    "1016",  // Elsevier
                    "1126",  // Science (AAAS)
                    "1145",  // ACM
                    "1109",  // IEEE
                    "1002",  // Wiley
                    "1007",  // Springer
                    "1371",  // PLOS
                    "1073",  // PNAS
                    "1186",  // BioMed Central
                    "3389",  // Frontiers
                    "1021",  // ACS (chemistry)
                    "48550", // arXiv
                    "5281",  // Zenodo
                    "1000",  // generic
                    "7554",  // eLife
                ];
                let reg = registrants[self.rng.gen_range(0..registrants.len())];

                // Generate realistic suffixes
                let suffix = match self.rng.gen_range(0..5) {
                    0 => {
                        // Journal style: journal.year.identifier
                        let journals = ["nature", "science", "cell", "lancet", "nphys", "nmat"];
                        let journal = journals[self.rng.gen_range(0..journals.len())];
                        format!("{}{:05}", journal, self.rng.gen_range(10000..99999))
                    }
                    1 => {
                        // Elsevier/journal path style: j.journal.year.month.day
                        format!(
                            "j.{}.{}.{:02}.{:03}",
                            ["cell", "neuron", "jmb", "jtbi", "amc"][self.rng.gen_range(0..5)],
                            self.rng.gen_range(2000..2026),
                            self.rng.gen_range(1..13),
                            self.rng.gen_range(1..100)
                        )
                    }
                    2 => {
                        // arXiv style: arXiv.YYMM.NNNNN
                        format!(
                            "arXiv.{:02}{:02}.{:05}",
                            self.rng.gen_range(18..26),
                            self.rng.gen_range(1..13),
                            self.rng.gen_range(10..99999)
                        )
                    }
                    3 => {
                        // Simple alphanumeric
                        let len = self.rng.gen_range(5..12);
                        let chars: String = (0..len)
                            .map(|_| {
                                let c = self.rng.gen_range(0..36);
                                if c < 10 {
                                    (b'0' + c) as char
                                } else {
                                    (b'a' + c - 10) as char
                                }
                            })
                            .collect();
                        chars
                    }
                    _ => {
                        // Structured with slashes: s12345-678-90123-4
                        format!(
                            "s{:05}-{:03}-{:05}-{}",
                            self.rng.gen_range(10000..99999),
                            self.rng.gen_range(0..999),
                            self.rng.gen_range(10000..99999),
                            self.rng.gen_range(0..9)
                        )
                    }
                };
                Ok(format!("10.{}/{}", reg, suffix))
            }
            ("code", "locale_code") => {
                let codes = [
                    "en", "en-US", "en-GB", "en-AU", "en-CA", "fr", "fr-FR", "fr-CA", "de",
                    "de-DE", "de-AT", "es", "es-ES", "es-MX", "it", "it-IT", "ja", "ja-JP", "ko",
                    "ko-KR", "zh", "zh-CN", "zh-TW", "pt", "pt-BR", "ru", "ru-RU", "nl", "nl-NL",
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }
            ("code", "pin") => {
                let len = if self.rng.gen_bool(0.7) { 4 } else { 6 };
                Ok(format!(
                    "{:0width$}",
                    self.rng.gen_range(0..10u32.pow(len)),
                    width = len as usize
                ))
            }

            // ── development (8 types) ────────────────────────────────────
            ("development", "version") => {
                let major = self.rng.gen_range(0..20);
                let minor = self.rng.gen_range(0..50);
                let patch = self.rng.gen_range(0..100);
                let prefix = if self.rng.gen_bool(0.3) { "v" } else { "" };
                let pre = if self.rng.gen_bool(0.2) {
                    let tags = ["-alpha", "-beta", "-rc.1", "-dev"];
                    tags[self.rng.gen_range(0..tags.len())]
                } else {
                    ""
                };
                Ok(format!("{}{}.{}.{}{}", prefix, major, minor, patch, pre))
            }
            ("development", "calver") => {
                let y = self.rng.gen_range(2020..2026);
                let m = self.rng.gen_range(1..13);
                if self.rng.gen_bool(0.5) {
                    let d = self.rng.gen_range(1..29);
                    Ok(format!("{}.{:02}.{:02}", y, m, d))
                } else {
                    Ok(format!("{}.{:02}", y, m))
                }
            }
            ("development", "programming_language") => {
                let langs = [
                    "Python",
                    "JavaScript",
                    "TypeScript",
                    "Java",
                    "C++",
                    "C#",
                    "Go",
                    "Rust",
                    "PHP",
                    "Ruby",
                    "Kotlin",
                    "Swift",
                    "Scala",
                    "Haskell",
                    "R",
                    "Julia",
                    "MATLAB",
                    "Perl",
                    "Lua",
                    "Elixir",
                ];
                Ok(langs[self.rng.gen_range(0..langs.len())].to_string())
            }
            ("development", "software_license") => {
                let licenses = [
                    "MIT",
                    "Apache-2.0",
                    "GPL-3.0",
                    "GPL-2.0",
                    "BSD-3-Clause",
                    "BSD-2-Clause",
                    "ISC",
                    "MPL-2.0",
                    "LGPL-3.0",
                    "AGPL-3.0",
                    "Unlicense",
                    "CC0-1.0",
                ];
                Ok(licenses[self.rng.gen_range(0..licenses.len())].to_string())
            }
            ("development", "stage") => {
                let stages = [
                    "Alpha",
                    "Beta",
                    "Release Candidate",
                    "Stable",
                    "LTS",
                    "Deprecated",
                ];
                Ok(stages[self.rng.gen_range(0..stages.len())].to_string())
            }
            ("development", "os") => {
                let oses = [
                    "Windows 10",
                    "Windows 11",
                    "macOS",
                    "Ubuntu",
                    "Fedora",
                    "Debian",
                    "Arch Linux",
                    "CentOS",
                    "iOS",
                    "Android",
                ];
                Ok(oses[self.rng.gen_range(0..oses.len())].to_string())
            }
            ("development", "boolean") => {
                let bools = [
                    "true", "false", "yes", "no", "1", "0", "True", "False", "YES", "NO",
                ];
                Ok(bools[self.rng.gen_range(0..bools.len())].to_string())
            }

            // ── hardware (4 types) ───────────────────────────────────────
            ("hardware", "cpu") => {
                let cpus = [
                    "Intel Core i9-14900K",
                    "Intel Core i7-14700K",
                    "Intel Core i5-14600K",
                    "AMD Ryzen 9 7950X",
                    "AMD Ryzen 7 7700X",
                    "AMD Ryzen 5 7600X",
                    "Apple M3 Pro",
                    "Apple M3 Max",
                    "Apple M2 Ultra",
                    "Qualcomm Snapdragon 8 Gen 3",
                ];
                Ok(cpus[self.rng.gen_range(0..cpus.len())].to_string())
            }
            ("hardware", "ram_size") => {
                let sizes = [
                    "4GB", "8GB", "16GB", "32GB", "64GB", "128GB", "256GB", "512MB",
                ];
                Ok(sizes[self.rng.gen_range(0..sizes.len())].to_string())
            }
            ("hardware", "screen_size") => {
                let sizes = [
                    "13.3\"", "14\"", "15.6\"", "16\"", "24\"", "27\"", "32\"", "34\"",
                ];
                Ok(sizes[self.rng.gen_range(0..sizes.len())].to_string())
            }
            ("hardware", "generation") => {
                let gens = [
                    "1st Generation",
                    "2nd Generation",
                    "3rd Generation",
                    "4th Generation",
                    "5th Generation",
                    "Gen 3",
                    "Gen 4",
                    "Gen 5",
                    "Rev 2",
                    "v3",
                ];
                Ok(gens[self.rng.gen_range(0..gens.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "technology.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: identity (32 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_identity(&mut self, category: &str, type_name: &str) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── person (16 types) ────────────────────────────────────────
            ("person", "full_name") => {
                let first = self.random_first_name();
                let last = self.random_last_name();
                let locale = self.current_locale();
                // East Asian: LastName FirstName order
                if matches!(locale, "JA" | "ZH" | "KO") {
                    return Ok(format!("{}{}", last, first));
                }
                // Generate diverse name formats for better model training.
                // This helps the char-CNN distinguish names from user_agent strings
                // by exposing it to formats like "LastName, Title. FirstName"
                // which have similar character patterns (commas, dots, mixed case).
                let format_idx = self.rng.gen_range(0..10);
                match format_idx {
                    // "FirstName LastName" (basic — most common)
                    0 | 1 | 2 => Ok(format!("{} {}", first, last)),
                    // "LastName, FirstName" (CSV/database style)
                    3 | 4 => Ok(format!("{}, {}", last, first)),
                    // "LastName, Title. FirstName" (Titanic style)
                    5 => {
                        let titles = ["Mr.", "Mrs.", "Ms.", "Dr.", "Rev.", "Prof."];
                        let title = titles[self.rng.gen_range(0..titles.len())];
                        Ok(format!("{}, {} {}", last, title, first))
                    }
                    // "LASTNAME, FIRSTNAME" (all caps)
                    6 => Ok(format!("{}, {}", last.to_uppercase(), first.to_uppercase())),
                    // "Title FirstName LastName" (title prefix)
                    7 => {
                        let titles = ["Dr.", "Mr.", "Mrs.", "Ms.", "Prof.", "Sir"];
                        let title = titles[self.rng.gen_range(0..titles.len())];
                        Ok(format!("{} {} {}", title, first, last))
                    }
                    // "FirstName M. LastName" (middle initial)
                    8 => {
                        let initial = (b'A' + self.rng.gen_range(0..26)) as char;
                        Ok(format!("{} {}. {}", first, initial, last))
                    }
                    // "LastName, Title. FirstName MiddleName" (formal with middle name)
                    _ => {
                        let titles = ["Mr.", "Mrs.", "Miss", "Dr.", "Rev."];
                        let title = titles[self.rng.gen_range(0..titles.len())];
                        let middle = self.random_first_name();
                        Ok(format!("{}, {} {} {}", last, title, first, middle))
                    }
                }
            }
            ("person", "first_name") => Ok(self.random_first_name()),
            ("person", "last_name") => Ok(self.random_last_name()),
            ("person", "email") => {
                let first = self.random_first_name().to_lowercase();
                let last = self.random_last_name().to_lowercase();
                let domains = [
                    "gmail.com",
                    "yahoo.com",
                    "outlook.com",
                    "example.com",
                    "company.org",
                ];
                let sep = [".", "_", ""][self.rng.gen_range(0..3)];
                let num = if self.rng.gen_bool(0.3) {
                    self.rng.gen_range(1..99).to_string()
                } else {
                    String::new()
                };
                Ok(format!(
                    "{}{}{}{}@{}",
                    first,
                    sep,
                    last,
                    num,
                    domains[self.rng.gen_range(0..domains.len())]
                ))
            }
            ("person", "phone_number") => self.gen_phone_number(),
            ("person", "username") => {
                let first = self.random_first_name().to_lowercase();
                let seps = [".", "_", "-", ""];
                let sep = seps[self.rng.gen_range(0..seps.len())];
                let suffix = if self.rng.gen_bool(0.5) {
                    self.rng.gen_range(1..999).to_string()
                } else {
                    self.random_word()
                };
                Ok(format!("{}{}{}", first, sep, suffix))
            }
            ("person", "password") => {
                use rand::distributions::Alphanumeric;
                let len = self.rng.gen_range(8..20);
                let mut pass: String = (&mut self.rng)
                    .sample_iter(Alphanumeric)
                    .take(len)
                    .map(|b| b as char)
                    .collect();
                // Add special chars
                let specials = "!@#$%^&*()_+-=[]{}|;:',.<>?";
                let pos = self.rng.gen_range(0..pass.len());
                let special = specials
                    .chars()
                    .nth(self.rng.gen_range(0..specials.len()))
                    .unwrap();
                pass.insert(pos, special);
                Ok(pass)
            }
            ("person", "gender") => {
                let genders = ["Male", "Female", "Non-binary", "Other", "Prefer not to say"];
                Ok(genders[self.rng.gen_range(0..genders.len())].to_string())
            }
            ("person", "gender_code") => {
                let codes = ["M", "F", "X"];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }
            ("person", "gender_symbol") => {
                // Unicode gender symbols only — NOT emoji codepoints
                let symbols = ["♂", "♀", "⚧", "⚪"];
                Ok(symbols[self.rng.gen_range(0..symbols.len())].to_string())
            }
            ("person", "nationality") => {
                let nationalities = locale_data::nationalities(self.current_locale());
                Ok(nationalities[self.rng.gen_range(0..nationalities.len())].to_string())
            }
            ("person", "blood_type") => {
                let types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
                Ok(types[self.rng.gen_range(0..types.len())].to_string())
            }
            ("person", "height") => {
                if self.rng.gen_bool(0.6) {
                    // Metric
                    Ok(format!("{} cm", self.rng.gen_range(150..200)))
                } else {
                    // Imperial
                    let feet = self.rng.gen_range(5..7);
                    let inches = self.rng.gen_range(0..12);
                    Ok(format!("{}'{:02}\"", feet, inches))
                }
            }
            ("person", "weight") => {
                if self.rng.gen_bool(0.6) {
                    Ok(format!("{} kg", self.rng.gen_range(45..120)))
                } else {
                    Ok(format!("{} lbs", self.rng.gen_range(100..265)))
                }
            }
            ("person", "age") => Ok(self.rng.gen_range(1..100).to_string()),
            ("person", "occupation") => {
                let jobs = [
                    "Software Engineer",
                    "Data Scientist",
                    "Product Manager",
                    "Designer",
                    "Teacher",
                    "Nurse",
                    "Accountant",
                    "Lawyer",
                    "Chef",
                    "Pilot",
                    "Architect",
                    "Pharmacist",
                    "Marketing Manager",
                    "Sales Representative",
                    "Researcher",
                ];
                Ok(jobs[self.rng.gen_range(0..jobs.len())].to_string())
            }
            // ── payment (7 types) ────────────────────────────────────────
            ("payment", "credit_card_number") => {
                // Generate Luhn-valid card numbers with correct IIN prefixes per network
                let (prefix, total_len) = match self.rng.gen_range(0u8..4) {
                    0 => {
                        // Visa: starts with 4, 16 digits
                        ("4".to_string(), 16)
                    }
                    1 => {
                        // Mastercard: starts with 51-55, 16 digits
                        let mc = self.rng.gen_range(51..=55);
                        (mc.to_string(), 16)
                    }
                    2 => {
                        // Amex: starts with 34 or 37, 15 digits
                        let amex = if self.rng.gen_bool(0.5) { "34" } else { "37" };
                        (amex.to_string(), 15)
                    }
                    _ => {
                        // Discover: starts with 6011, 16 digits
                        ("6011".to_string(), 16)
                    }
                };
                let random_digits = total_len - prefix.len() - 1; // -1 for check digit
                let body: String = (0..random_digits)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                let partial = format!("{}{}", prefix, body);
                let check = self.luhn_check_digit(&partial);
                Ok(format!("{}{}", partial, check))
            }
            ("payment", "credit_card_expiration_date") => {
                let month = self.rng.gen_range(1..13);
                let year = self.rng.gen_range(25..32);
                Ok(format!("{:02}/{:02}", month, year))
            }
            ("payment", "cvv") => {
                if self.rng.gen_bool(0.85) {
                    Ok(format!("{:03}", self.rng.gen_range(100..999)))
                } else {
                    // Amex 4-digit CID
                    Ok(format!("{:04}", self.rng.gen_range(1000..9999)))
                }
            }
            ("payment", "credit_card_network") => {
                let networks = [
                    "Visa",
                    "Mastercard",
                    "Amex",
                    "Discover",
                    "Diners Club",
                    "JCB",
                ];
                Ok(networks[self.rng.gen_range(0..networks.len())].to_string())
            }
            ("payment", "bitcoin_address") => {
                let base58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
                let prefix_choice = self.rng.gen_range(0..3);
                match prefix_choice {
                    0 => {
                        // P2PKH (1...)
                        let chars: String = (0..33)
                            .map(|_| base58.chars().nth(self.rng.gen_range(0..58)).unwrap())
                            .collect();
                        Ok(format!("1{}", chars))
                    }
                    1 => {
                        // P2SH (3...)
                        let chars: String = (0..33)
                            .map(|_| base58.chars().nth(self.rng.gen_range(0..58)).unwrap())
                            .collect();
                        Ok(format!("3{}", chars))
                    }
                    _ => {
                        // Bech32 (bc1...)
                        let bech32_chars = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
                        let chars: String = (0..39)
                            .map(|_| bech32_chars.chars().nth(self.rng.gen_range(0..32)).unwrap())
                            .collect();
                        Ok(format!("bc1{}", chars))
                    }
                }
            }
            ("payment", "ethereum_address") => Ok(format!("0x{}", self.gen_hex_string(40))),
            ("payment", "paypal_email") => {
                // PayPal-distinctive patterns: paypal.com domains, business patterns, pp- prefixes
                let first = self.random_first_name().to_lowercase();
                let last = self.random_last_name().to_lowercase();
                match self.rng.gen_range(0u8..5) {
                    0 => {
                        // PayPal business email
                        Ok(format!("{}@paypal.com", first))
                    }
                    1 => {
                        // PayPal-style pp- prefix
                        Ok(format!("pp-{}.{}@paypal.com", first, last))
                    }
                    2 => {
                        // PayPal merchant pattern
                        let word = self.random_word();
                        Ok(format!("{}-payments@paypal.com", word))
                    }
                    3 => {
                        // PayPal service address
                        let services = ["service", "payments", "billing", "merchant", "seller"];
                        let svc = services[self.rng.gen_range(0..services.len())];
                        Ok(format!("{}.{}@{}.paypal.com", first, last, svc))
                    }
                    _ => {
                        // PayPal-linked email with paypal subdomain
                        Ok(format!(
                            "paypal-{}{}@{}.com",
                            first,
                            self.rng.gen_range(1..999),
                            self.random_word()
                        ))
                    }
                }
            }

            // ── payment: finance identifiers (7 types) ──────────────────
            ("payment", "isin") => {
                // ISIN: 2-letter country code + 9 alphanumeric NSIN + 1 Luhn check digit
                let countries = [
                    "US", "GB", "DE", "FR", "JP", "CA", "AU", "CH", "NL", "SE", "IT", "ES", "HK",
                    "SG", "KR", "BR", "IN", "ZA", "MX", "NO", "DK", "FI", "BE", "AT", "IE", "LU",
                    "NZ", "TW", "IL", "PT",
                ];
                let country = countries[self.rng.gen_range(0..countries.len())];
                // NSIN: 9 alphanumeric characters (digits + uppercase letters)
                let alphanum = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
                let nsin: String = (0..9)
                    .map(|_| alphanum.as_bytes()[self.rng.gen_range(0..alphanum.len())] as char)
                    .collect();
                let body = format!("{}{}", country, nsin);
                let check = self.isin_check_digit(&body);
                Ok(format!("{}{}", body, check))
            }
            ("payment", "cusip") => {
                // CUSIP: 6 issuer chars + 2 issue chars + 1 check digit
                let alphanum = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
                let body: String = (0..8)
                    .map(|_| {
                        if self.rng.gen_bool(0.6) {
                            // Bias toward digits for realistic look
                            (b'0' + self.rng.gen_range(0..10)) as char
                        } else {
                            alphanum.as_bytes()[self.rng.gen_range(0..alphanum.len())] as char
                        }
                    })
                    .collect();
                let check = self.cusip_check_digit(&body);
                Ok(format!("{}{}", body, check))
            }
            ("payment", "sedol") => {
                // SEDOL: 6 chars (consonants + digits, no vowels) + 1 weighted check digit
                let sedol_chars = "0123456789BCDFGHJKLMNPQRSTVWXYZ";
                let body: String = (0..6)
                    .map(|_| {
                        sedol_chars.as_bytes()[self.rng.gen_range(0..sedol_chars.len())] as char
                    })
                    .collect();
                let check = self.sedol_check_digit(&body);
                Ok(format!("{}{}", body, check))
            }
            ("payment", "swift_bic") => {
                // SWIFT/BIC: 4 bank letters + 2 country letters + 2 location + optional 3 branch
                let countries = [
                    "US", "GB", "DE", "FR", "CH", "JP", "AU", "SG", "HK", "NL", "IT", "ES", "CA",
                    "SE", "NO", "DK", "BE", "AT", "IE", "LU",
                ];
                let bank: String = (0..4)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                let country = countries[self.rng.gen_range(0..countries.len())];
                let location: String = (0..2)
                    .map(|_| {
                        if self.rng.gen_bool(0.7) {
                            (b'A' + self.rng.gen_range(0..26)) as char
                        } else {
                            (b'0' + self.rng.gen_range(0..10)) as char
                        }
                    })
                    .collect();
                if self.rng.gen_bool(0.4) {
                    // 11-char with branch code
                    let branch: String = (0..3)
                        .map(|_| {
                            if self.rng.gen_bool(0.7) {
                                (b'A' + self.rng.gen_range(0..26)) as char
                            } else {
                                (b'0' + self.rng.gen_range(0..10)) as char
                            }
                        })
                        .collect();
                    Ok(format!("{}{}{}{}", bank, country, location, branch))
                } else {
                    // 8-char (head office)
                    Ok(format!("{}{}{}", bank, country, location))
                }
            }
            ("payment", "lei") => {
                // LEI: 4-digit LOU prefix + 14 alphanumeric entity + 2 check digits (ISO 7064)
                let lou_prefixes = [
                    "5299", "2138", "5493", "3358", "8945", "9598", "7245", "6354", "3157", "2549",
                    "5067", "8156",
                ];
                let lou = lou_prefixes[self.rng.gen_range(0..lou_prefixes.len())];
                let alphanum = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
                let entity: String = (0..14)
                    .map(|_| alphanum.as_bytes()[self.rng.gen_range(0..alphanum.len())] as char)
                    .collect();
                let body = format!("{}{}", lou, entity);
                let check = self.lei_check_digits(&body);
                Ok(format!("{}{}", body, check))
            }
            ("payment", "currency_code") => {
                let codes = [
                    "USD", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD", "CNY", "HKD", "NZD", "SEK",
                    "NOK", "DKK", "SGD", "KRW", "INR", "BRL", "ZAR", "MXN", "TWD", "THB", "IDR",
                    "MYR", "PHP", "PLN", "CZK", "HUF", "TRY", "ILS", "AED", "SAR", "RUB", "CLP",
                    "COP", "PEN", "ARS", "EGP", "NGN", "KES", "GHS",
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }
            ("payment", "currency_symbol") => {
                let symbols = [
                    "$", "€", "£", "¥", "₹", "₩", "₿", "₽", "₺", "₴", "₸", "₡", "₵", "₫", "₭", "₮",
                    "₱", "₲", "₳", "₦", "৳", "฿", "₪", "﷼", "₢", "₣", "₤", "₧", "₯", "₰",
                ];
                Ok(symbols[self.rng.gen_range(0..symbols.len())].to_string())
            }

            // ── academic (2 types) ───────────────────────────────────────
            ("academic", "degree") => {
                let degrees = [
                    "Bachelor of Science",
                    "Bachelor of Arts",
                    "Master of Science",
                    "Master of Arts",
                    "Master of Business Administration",
                    "Doctor of Philosophy",
                    "Associate Degree",
                    "Juris Doctor",
                    "Doctor of Medicine",
                ];
                Ok(degrees[self.rng.gen_range(0..degrees.len())].to_string())
            }
            ("academic", "university") => {
                let unis = [
                    "Harvard University",
                    "Stanford University",
                    "MIT",
                    "Oxford University",
                    "Cambridge University",
                    "ETH Zurich",
                    "University of Tokyo",
                    "Caltech",
                    "Princeton University",
                    "Yale University",
                    "Columbia University",
                    "UC Berkeley",
                    "University of Melbourne",
                    "Sorbonne University",
                ];
                Ok(unis[self.rng.gen_range(0..unis.len())].to_string())
            }

            // ── medical (3 types) ──────────────────────────────────────
            ("medical", "npi") => {
                // NPI: 10 digits starting with 1 or 2, with Luhn check digit
                // Generate first 9 digits (prefix + 8 random)
                let prefix: u64 = if self.rng.gen_bool(0.7) { 1 } else { 2 };
                let middle: u64 = self.rng.gen_range(0..100_000_000);
                let partial = prefix * 100_000_000 + middle; // 9 digits
                // Calculate Luhn check digit: prepend 80840, append 0 as placeholder
                let test_str = format!("80840{}0", partial);
                let digits: Vec<u32> = test_str
                    .chars()
                    .map(|c| c.to_digit(10).unwrap())
                    .collect();
                let mut sum: u32 = 0;
                for (i, &d) in digits.iter().rev().enumerate() {
                    if i % 2 == 0 {
                        let doubled = d * 2;
                        sum += if doubled > 9 { doubled - 9 } else { doubled };
                    } else {
                        sum += d;
                    }
                }
                let check = (10 - (sum % 10)) % 10;
                Ok(format!("{}{}", partial, check))
            }
            ("medical", "dea_number") => {
                // DEA: 2 letters + 7 digits with check digit
                let reg_types = ['A', 'B', 'F', 'M'];
                let first = reg_types[self.rng.gen_range(0..reg_types.len())];
                let second = (b'A' + self.rng.gen_range(0..26u8)) as char;
                // Generate 6 digits for positions 1-6
                let d: Vec<u32> = (0..6).map(|_| self.rng.gen_range(0..10u32)).collect();
                // Check digit: (d1+d3+d5) + 2*(d2+d4+d6), last digit of sum
                let odd_sum = d[0] + d[2] + d[4];
                let even_sum = d[1] + d[3] + d[5];
                let check = (odd_sum + 2 * even_sum) % 10;
                Ok(format!(
                    "{}{}{}{}{}{}{}{}{}",
                    first, second, d[0], d[1], d[2], d[3], d[4], d[5], check
                ))
            }
            ("medical", "ndc") => {
                // NDC: 3 formats (4-4-2, 5-3-2, 5-4-1)
                let format_choice = self.rng.gen_range(0..4);
                match format_choice {
                    0 => {
                        // 4-4-2
                        let a = self.rng.gen_range(0..10000u32);
                        let b = self.rng.gen_range(0..10000u32);
                        let c = self.rng.gen_range(0..100u32);
                        Ok(format!("{:04}-{:04}-{:02}", a, b, c))
                    }
                    1 => {
                        // 5-3-2
                        let a = self.rng.gen_range(0..100000u32);
                        let b = self.rng.gen_range(0..1000u32);
                        let c = self.rng.gen_range(0..100u32);
                        Ok(format!("{:05}-{:03}-{:02}", a, b, c))
                    }
                    2 => {
                        // 5-4-1
                        let a = self.rng.gen_range(0..100000u32);
                        let b = self.rng.gen_range(0..10000u32);
                        let c = self.rng.gen_range(0..10u32);
                        Ok(format!("{:05}-{:04}-{}", a, b, c))
                    }
                    _ => {
                        // 11-digit no dashes
                        let n: u64 = self.rng.gen_range(0..100_000_000_000u64);
                        Ok(format!("{:011}", n))
                    }
                }
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "identity.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: geography (16 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_geography(&mut self, category: &str, type_name: &str) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── location (5 types) ───────────────────────────────────────
            ("location", "country") => {
                let countries = locale_data::countries(self.current_locale());
                Ok(countries[self.rng.gen_range(0..countries.len())].to_string())
            }
            ("location", "country_code") => {
                let codes = [
                    "US", "GB", "CA", "AU", "DE", "FR", "JP", "CN", "IN", "BR", "MX", "IT", "ES",
                    "KR", "RU", "NL", "CH", "SE", "NO", "DK",
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }
            ("location", "continent") => {
                let continents = locale_data::continents(self.current_locale());
                Ok(continents[self.rng.gen_range(0..continents.len())].to_string())
            }
            ("location", "region") => {
                let regions = locale_data::states(self.current_locale());
                Ok(regions[self.rng.gen_range(0..regions.len())].to_string())
            }
            ("location", "city") => {
                let cities = locale_data::cities(self.current_locale());
                Ok(cities[self.rng.gen_range(0..cities.len())].to_string())
            }

            // ── address (5 types) ────────────────────────────────────────
            ("address", "full_address") => {
                let num = self.rng.gen_range(1..9999);
                let streets = locale_data::street_names(self.current_locale());
                let cities = locale_data::cities(self.current_locale());
                let street = streets[self.rng.gen_range(0..streets.len())];
                let city = cities[self.rng.gen_range(0..cities.len())];
                Ok(format!("{} {}, {}", num, street, city))
            }
            ("address", "street_number") => {
                if self.rng.gen_bool(0.9) {
                    Ok(self.rng.gen_range(1..9999).to_string())
                } else {
                    let suffix = ['A', 'B', 'C'][self.rng.gen_range(0..3)];
                    Ok(format!("{}{}", self.rng.gen_range(1..999), suffix))
                }
            }
            ("address", "street_name") => {
                let names = locale_data::street_names(self.current_locale());
                Ok(names[self.rng.gen_range(0..names.len())].to_string())
            }
            ("address", "street_suffix") => {
                let suffixes = locale_data::street_suffixes(self.current_locale());
                Ok(suffixes[self.rng.gen_range(0..suffixes.len())].to_string())
            }
            ("address", "postal_code") => self.gen_postal_code(),

            // ── coordinate (3 types) ─────────────────────────────────────
            ("coordinate", "latitude") => {
                let lat = (self.rng.gen::<f64>() - 0.5) * 180.0;
                Ok(format!("{:.4}", lat))
            }
            ("coordinate", "longitude") => {
                let lon = (self.rng.gen::<f64>() - 0.5) * 360.0;
                Ok(format!("{:.4}", lon))
            }
            ("coordinate", "coordinates") => {
                let lat = (self.rng.gen::<f64>() - 0.5) * 180.0;
                let lon = (self.rng.gen::<f64>() - 0.5) * 360.0;
                Ok(format!("{:.4},{:.4}", lat, lon))
            }

            // ── transportation (2 types) ─────────────────────────────────
            ("transportation", "iata_code") => {
                let code: String = (0..3)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                Ok(code)
            }
            ("transportation", "icao_code") => {
                let code: String = (0..4)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                Ok(code)
            }

            // ── contact (1 type) ─────────────────────────────────────────
            ("contact", "calling_code") => {
                let codes = locale_data::calling_codes(self.current_locale());
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "geography.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: representation (19 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_representation(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── numeric (5 types) ────────────────────────────────────────
            ("numeric", "integer_number") => Ok(self.rng.gen_range(-100000i64..100000).to_string()),
            ("numeric", "decimal_number") => {
                let val = (self.rng.gen::<f64>() - 0.5) * 2000.0;
                let precision = self.rng.gen_range(1..8);
                Ok(format!("{:.prec$}", val, prec = precision))
            }
            ("numeric", "scientific_notation") => {
                let mantissa = self.rng.gen::<f64>() * 9.0 + 1.0;
                let exponent = self.rng.gen_range(-15i32..15);
                let e_char = if self.rng.gen_bool(0.5) { 'e' } else { 'E' };
                Ok(format!("{:.2}{}{:+}", mantissa, e_char, exponent))
            }
            ("numeric", "percentage") => {
                let val = self.rng.gen::<f64>() * 100.0;
                if self.rng.gen_bool(0.7) {
                    Ok(format!("{:.1}%", val))
                } else {
                    Ok(format!("{:.2}%", val))
                }
            }
            ("numeric", "increment") => Ok(self.rng.gen_range(1..100000).to_string()),
            ("numeric", "si_number") => {
                let suffixes = ['K', 'k', 'M', 'm', 'B', 'b', 'T', 't'];
                let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];
                let prefixes = ["", "$", "€", "£", "+", "-"];
                let prefix = if self.rng.gen_bool(0.3) {
                    prefixes[self.rng.gen_range(0..prefixes.len())]
                } else {
                    ""
                };
                let value: f64 = match suffix.to_ascii_uppercase() {
                    'K' => self.rng.gen_range(1.0..999.9),
                    'M' => self.rng.gen_range(1.0..999.9),
                    'B' => self.rng.gen_range(1.0..99.9),
                    'T' => self.rng.gen_range(1.0..9.9),
                    _ => self.rng.gen_range(1.0..999.9),
                };
                // Choose precision: 0 (whole), 1, or 2 decimal places
                let precision = self.rng.gen_range(0..3);
                if precision == 0 {
                    Ok(format!("{}{}{}", prefix, value as u64, suffix))
                } else {
                    Ok(format!("{}{:.prec$}{}", prefix, value, suffix, prec = precision))
                }
            }

            // ── text (5 types) ───────────────────────────────────────────
            ("text", "plain_text") => {
                let words: Vec<String> = (0..self.rng.gen_range(5..25))
                    .map(|_| self.random_word())
                    .collect();
                Ok(words.join(" "))
            }
            ("text", "sentence") => {
                let mut words: Vec<String> = (0..self.rng.gen_range(5..15))
                    .map(|_| self.random_word())
                    .collect();
                // Capitalize first word
                if let Some(first) = words.first_mut() {
                    let mut chars = first.chars();
                    if let Some(c) = chars.next() {
                        *first = c.to_uppercase().collect::<String>() + chars.as_str();
                    }
                }
                let ending = [".", "!", "?"][self.rng.gen_range(0..3)];
                Ok(format!("{}{}", words.join(" "), ending))
            }
            ("text", "word") => Ok(self.random_word()),
            ("text", "color_hex") => {
                let r = self.rng.gen::<u8>();
                let g = self.rng.gen::<u8>();
                let b = self.rng.gen::<u8>();
                if self.rng.gen_bool(0.8) {
                    Ok(format!("#{:02X}{:02X}{:02X}", r, g, b))
                } else {
                    Ok(format!("{:02x}{:02x}{:02x}", r, g, b))
                }
            }
            ("text", "color_rgb") => {
                let r = self.rng.gen_range(0..256);
                let g = self.rng.gen_range(0..256);
                let b = self.rng.gen_range(0..256);
                if self.rng.gen_bool(0.6) {
                    Ok(format!("rgb({}, {}, {})", r, g, b))
                } else {
                    Ok(format!("{}, {}, {}", r, g, b))
                }
            }
            ("text", "emoji") => {
                let emojis = [
                    "\u{1f600}",
                    "\u{1f602}",
                    "\u{1f923}",
                    "\u{1f60a}",
                    "\u{1f60d}",
                    "\u{1f970}",
                    "\u{1f60e}",
                    "\u{1f914}",
                    "\u{1f622}",
                    "\u{1f621}",
                    "\u{1f389}",
                    "\u{1f525}",
                    "\u{2764}\u{fe0f}",
                    "\u{1f44d}",
                    "\u{1f44e}",
                    "\u{1f680}",
                    "\u{1f4bb}",
                    "\u{1f4f1}",
                    "\u{1f30d}",
                    "\u{26a1}",
                    "\u{2705}",
                    "\u{274c}",
                    "\u{2b50}",
                    "\u{1f3b8}",
                ];
                Ok(emojis[self.rng.gen_range(0..emojis.len())].to_string())
            }

            // ── file (4 types) ───────────────────────────────────────────
            ("file", "extension") => {
                let exts = [
                    "txt", "pdf", "docx", "xlsx", "csv", "json", "xml", "html", "js", "py", "rs",
                    "go", "java", "cpp", "md", "yaml", "png", "jpg", "gif", "svg", "mp4", "mp3",
                    "zip", "gz",
                ];
                Ok(exts[self.rng.gen_range(0..exts.len())].to_string())
            }
            ("file", "mime_type") => {
                let types = [
                    "text/plain",
                    "text/html",
                    "text/css",
                    "text/csv",
                    "application/json",
                    "application/xml",
                    "application/pdf",
                    "application/javascript",
                    "application/octet-stream",
                    "image/png",
                    "image/jpeg",
                    "image/gif",
                    "image/svg+xml",
                    "audio/mpeg",
                    "audio/wav",
                    "video/mp4",
                    "video/webm",
                    "multipart/form-data",
                ];
                Ok(types[self.rng.gen_range(0..types.len())].to_string())
            }
            ("file", "file_size") => {
                let units = ["B", "KB", "MB", "GB"];
                let unit = units[self.rng.gen_range(0..units.len())];
                let size = if unit == "B" {
                    self.rng.gen_range(1..1024).to_string()
                } else if self.rng.gen_bool(0.5) {
                    format!("{:.1}", self.rng.gen::<f64>() * 999.0 + 0.1)
                } else {
                    self.rng.gen_range(1..999).to_string()
                };
                Ok(format!("{} {}", size, unit))
            }
            ("file", "excel_format") => {
                // Excel custom number format strings
                let choice = self.rng.gen_range(0..8);
                let fmt = match choice {
                    // Number formats
                    0 => {
                        let decimals = self.rng.gen_range(0..5);
                        if decimals == 0 {
                            "#,##0".to_string()
                        } else {
                            format!("#,##0.{}", "0".repeat(decimals))
                        }
                    }
                    // Currency formats
                    1 => {
                        let symbols = ["$", "€", "£", "¥"];
                        let sym = symbols[self.rng.gen_range(0..symbols.len())];
                        let decimals = self.rng.gen_range(0..3);
                        if decimals == 0 {
                            format!("{}#,##0", sym)
                        } else {
                            format!("{}#,##0.{}", sym, "0".repeat(decimals))
                        }
                    }
                    // Percentage formats
                    2 => {
                        let decimals = self.rng.gen_range(0..4);
                        if decimals == 0 {
                            "0%".to_string()
                        } else {
                            format!("0.{}%", "0".repeat(decimals))
                        }
                    }
                    // Date formats
                    3 => {
                        let fmts = [
                            "mm/dd/yyyy", "dd/mm/yyyy", "yyyy-mm-dd", "m/d/yy",
                            "d-mmm-yy", "d-mmm", "mmm-yy", "dd-mmm-yyyy",
                            "yyyy/mm/dd", "mm-dd-yyyy",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // Time formats
                    4 => {
                        let fmts = [
                            "h:mm:ss AM/PM", "h:mm AM/PM", "h:mm:ss",
                            "hh:mm:ss", "hh:mm", "mm:ss", "h:mm:ss.00",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // Scientific notation
                    5 => {
                        let decimals = self.rng.gen_range(1..5);
                        format!("0.{}E+00", "0".repeat(decimals))
                    }
                    // Fraction formats
                    6 => {
                        let fmts = ["# ?/?", "# ??/??", "# ???/???", "# ?/2", "# ?/4", "# ?/8"];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // Conditional / multi-section formats
                    _ => {
                        let fmts = [
                            "#,##0.00;(#,##0.00)",
                            "#,##0.00;[Red](#,##0.00)",
                            "$#,##0.00;($#,##0.00)",
                            "0.00;-0.00;0",
                            "#,##0;-#,##0;\"--\"",
                            "[>100]#,##0;[Red]#,##0",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                };
                Ok(fmt)
            }

            // ── scientific (5 types) ─────────────────────────────────────
            ("scientific", "dna_sequence") => {
                let len = self.rng.gen_range(8..30);
                let bases = ['A', 'T', 'G', 'C'];
                let seq: String = (0..len).map(|_| bases[self.rng.gen_range(0..4)]).collect();
                Ok(seq)
            }
            ("scientific", "rna_sequence") => {
                let len = self.rng.gen_range(8..30);
                let bases = ['A', 'U', 'G', 'C'];
                let seq: String = (0..len).map(|_| bases[self.rng.gen_range(0..4)]).collect();
                Ok(seq)
            }
            ("scientific", "protein_sequence") => {
                let len = self.rng.gen_range(10..50);
                let amino = "ACDEFGHIKLMNPQRSTVWY";
                let seq: String = (0..len)
                    .map(|_| amino.chars().nth(self.rng.gen_range(0..20)).unwrap())
                    .collect();
                Ok(seq)
            }
            ("scientific", "measurement_unit") => {
                let units = [
                    "meter", "kilogram", "second", "ampere", "kelvin", "mole", "candela", "hertz",
                    "newton", "joule", "watt", "pascal", "liter", "gram", "m", "kg", "s", "A", "K",
                    "mol", "cd", "Hz", "N", "J", "W", "Pa", "L", "g",
                ];
                Ok(units[self.rng.gen_range(0..units.len())].to_string())
            }
            ("scientific", "metric_prefix") => {
                let prefixes = [
                    "yotta", "zetta", "exa", "peta", "tera", "giga", "mega", "kilo", "hecto",
                    "deca", "deci", "centi", "milli", "micro", "nano", "pico", "femto", "atto",
                ];
                Ok(prefixes[self.rng.gen_range(0..prefixes.len())].to_string())
            }

            // ── discrete (2 types) ────────────────────────────────────────
            ("discrete", "categorical") => {
                // Generate values that look like typical categorical column entries.
                // Multiple "vocabularies" to give the model diverse categorical patterns.
                let vocab_idx = self.rng.gen_range(0..12);
                let vocab: &[&str] = match vocab_idx {
                    0 => &["male", "female"],
                    1 => &["yes", "no", "maybe"],
                    2 => &["active", "inactive", "pending", "suspended"],
                    3 => &["red", "blue", "green", "yellow", "orange", "purple"],
                    4 => &["S", "C", "Q"],
                    5 => &["A", "B", "C", "D"],
                    6 => &["cat", "dog", "bird", "fish", "hamster"],
                    7 => &["left", "right", "center"],
                    8 => &["small", "medium", "large"],
                    9 => &["north", "south", "east", "west"],
                    10 => &["pass", "fail"],
                    _ => &["Type A", "Type B", "Type C", "Type D", "Type E"],
                };
                Ok(vocab[self.rng.gen_range(0..vocab.len())].to_string())
            }
            ("discrete", "ordinal") => {
                // Generate values that look like ordinal/ranked entries.
                let vocab_idx = self.rng.gen_range(0..10);
                let vocab: &[&str] = match vocab_idx {
                    0 => &["low", "medium", "high"],
                    1 => &["low", "medium", "high", "critical"],
                    2 => &["poor", "fair", "good", "very good", "excellent"],
                    3 => &["1st", "2nd", "3rd", "4th", "5th"],
                    4 => &["A", "B", "C", "D", "F"],
                    5 => &["freshman", "sophomore", "junior", "senior"],
                    6 => &["★", "★★", "★★★", "★★★★", "★★★★★"],
                    7 => &["I", "II", "III", "IV", "V"],
                    8 => &["none", "mild", "moderate", "severe"],
                    _ => &["beginner", "intermediate", "advanced", "expert"],
                };
                Ok(vocab[self.rng.gen_range(0..vocab.len())].to_string())
            }

            // ── code (1 type) ─────────────────────────────────────────────
            ("code", "alphanumeric_id") => {
                // Generate mixed letter+digit identifier patterns.
                let pattern_idx = self.rng.gen_range(0..10);
                match pattern_idx {
                    // PREFIX-NNNNN (product/ticket codes)
                    0 => {
                        let prefixes = ["SKU", "REF", "LOT", "INV", "PO", "WO", "TKT", "ORD"];
                        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                        let num = self.rng.gen_range(1000..99999);
                        Ok(format!("{}-{:05}", prefix, num))
                    }
                    // L-NNN (cabin/seat codes like C85, B28, A5)
                    1 => {
                        let letter = (b'A' + self.rng.gen_range(0..8)) as char;
                        let num = self.rng.gen_range(1..200);
                        Ok(format!("{}{}", letter, num))
                    }
                    // LL-NNNN (flight/batch codes)
                    2 => {
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let num = self.rng.gen_range(100..9999);
                        Ok(format!("{}{}-{}", l1, l2, num))
                    }
                    // LLL NNN (license plates)
                    3 => {
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l3 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let num = self.rng.gen_range(100..999);
                        Ok(format!("{}{}{} {}", l1, l2, l3, num))
                    }
                    // L/N NNNNN (Titanic ticket style)
                    4 => {
                        let letter = (b'A' + self.rng.gen_range(0..10)) as char;
                        let num = self.rng.gen_range(10000..99999);
                        Ok(format!("{}/{} {}", letter, self.rng.gen_range(1..9), num))
                    }
                    // LLNN-NNN (product batch)
                    5 => {
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let n1 = self.rng.gen_range(10..99);
                        let n2 = self.rng.gen_range(100..999);
                        Ok(format!("{}{}{}-{}", l1, l2, n1, n2))
                    }
                    // PREFIX-YYYY-NNNN (year-based lot numbers)
                    6 => {
                        let prefixes = ["LOT", "BATCH", "RUN", "SN"];
                        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                        let year = self.rng.gen_range(2018..2026);
                        let seq = self.rng.gen_range(1..9999);
                        Ok(format!("{}-{}-{:04}", prefix, year, seq))
                    }
                    // NNN-LL (zone codes)
                    7 => {
                        let num = self.rng.gen_range(100..999);
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        Ok(format!("{}-{}{}", num, l1, l2))
                    }
                    // L.NNN.NN (part numbers)
                    8 => {
                        let letter = (b'A' + self.rng.gen_range(0..8)) as char;
                        let n1 = self.rng.gen_range(100..999);
                        let n2 = self.rng.gen_range(10..99);
                        Ok(format!("{}.{}.{}", letter, n1, n2))
                    }
                    // LLLNNNNN (compact alphanumeric)
                    _ => {
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l3 = (b'a' + self.rng.gen_range(0..26)) as char;
                        let num = self.rng.gen_range(10000..99999);
                        Ok(format!("{}{}{}{}", l1, l2, l3, num))
                    }
                }
            }

            // ── logical (1 type) ──────────────────────────────────────────
            ("logical", "boolean") => {
                let vals = ["true", "false", "yes", "no", "1", "0", "True", "False",
                            "YES", "NO", "on", "off", "ON", "OFF"];
                Ok(vals[self.rng.gen_range(0..vals.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "representation.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: container (11 types)
    // ═══════════════════════════════════════════════════════════════════════════

    fn gen_container(&mut self, category: &str, type_name: &str) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── object (5 types) ─────────────────────────────────────────
            ("object", "json") => {
                let templates = [
                    format!(
                        r#"{{"name":"{}","age":{},"active":{}}}"#,
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.rng.gen_bool(0.7)
                    ),
                    format!(
                        r#"{{"id":{},"email":"{}@{}.com","role":"{}"}}"#,
                        self.rng.gen_range(1..10000),
                        self.random_first_name().to_lowercase(),
                        self.random_word(),
                        ["admin", "user", "moderator"][self.rng.gen_range(0..3)]
                    ),
                    format!(
                        r#"{{"product":"{}","price":{:.2},"currency":"{}"}}"#,
                        self.random_word(),
                        self.rng.gen::<f64>() * 999.0 + 0.01,
                        ["USD", "EUR", "GBP", "JPY"][self.rng.gen_range(0..4)]
                    ),
                    format!(
                        r#"{{"lat":{:.4},"lon":{:.4},"label":"{}"}}"#,
                        (self.rng.gen::<f64>() - 0.5) * 180.0,
                        (self.rng.gen::<f64>() - 0.5) * 360.0,
                        self.random_word()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "json_array") => {
                let templates = [
                    format!(
                        "[{},{},{}]",
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(1..100)
                    ),
                    format!(
                        r#"["{}","{}","{}"]"#,
                        self.random_word(),
                        self.random_word(),
                        self.random_word()
                    ),
                    format!(
                        r#"[{{"id":{},"name":"{}"}},{{"id":{},"name":"{}"}}]"#,
                        self.rng.gen_range(1..100),
                        self.random_first_name(),
                        self.rng.gen_range(1..100),
                        self.random_first_name()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "xml") => {
                let name = self.random_first_name();
                let age = self.rng.gen_range(18..80);
                let templates = [
                    format!("<person><name>{}</name><age>{}</age></person>", name, age),
                    format!(
                        "<item id=\"{}\"><title>{}</title><price>{:.2}</price></item>",
                        self.rng.gen_range(1..1000),
                        self.random_word(),
                        self.rng.gen::<f64>() * 100.0
                    ),
                    format!(
                        "<record><field name=\"status\">{}</field></record>",
                        ["active", "inactive", "pending"][self.rng.gen_range(0..3)]
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "yaml") => {
                let templates = [
                    format!(
                        "name: {}\nage: {}\nactive: {}",
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.rng.gen_bool(0.7)
                    ),
                    format!(
                        "server:\n  host: {}.com\n  port: {}\n  ssl: true",
                        self.random_word(),
                        self.rng.gen_range(80..9000)
                    ),
                    format!(
                        "database:\n  driver: {}\n  name: {}",
                        ["postgres", "mysql", "sqlite"][self.rng.gen_range(0..3)],
                        self.random_word()
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }
            ("object", "csv") => {
                let templates = [
                    format!(
                        "{},{},{},{}",
                        self.random_first_name(),
                        self.rng.gen_range(18..80),
                        self.random_first_name().to_lowercase() + "@example.com",
                        ["active", "inactive"][self.rng.gen_range(0..2)]
                    ),
                    format!(
                        "{},{:.2},{},{}",
                        self.random_word(),
                        self.rng.gen::<f64>() * 100.0,
                        self.rng.gen_range(1..1000),
                        ["USD", "EUR", "GBP"][self.rng.gen_range(0..3)]
                    ),
                ];
                Ok(templates[self.rng.gen_range(0..templates.len())].clone())
            }

            // ── array (4 types) ──────────────────────────────────────────
            ("array", "comma_separated") => {
                let count = self.rng.gen_range(3..8);
                if self.rng.gen_bool(0.5) {
                    // Words
                    let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                    Ok(items.join(","))
                } else {
                    // Numbers
                    let items: Vec<String> = (0..count)
                        .map(|_| self.rng.gen_range(1..100).to_string())
                        .collect();
                    Ok(items.join(","))
                }
            }
            ("array", "pipe_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                Ok(items.join("|"))
            }
            ("array", "semicolon_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                Ok(items.join(";"))
            }
            ("array", "whitespace_separated") => {
                let count = self.rng.gen_range(3..8);
                let items: Vec<String> = (0..count).map(|_| self.random_word()).collect();
                if self.rng.gen_bool(0.7) {
                    Ok(items.join(" "))
                } else {
                    Ok(items.join("\t"))
                }
            }

            // ── key_value (2 types) ──────────────────────────────────────
            ("key_value", "query_string") => {
                let count = self.rng.gen_range(2..5);
                let pairs: Vec<String> = (0..count)
                    .map(|_| format!("{}={}", self.random_word(), self.random_word()))
                    .collect();
                Ok(pairs.join("&"))
            }
            ("key_value", "form_data") => {
                let fields = [
                    ("username", self.random_first_name().to_lowercase()),
                    ("email", format!("{}@example.com", self.random_word())),
                    (
                        "password",
                        format!("pass{}", self.rng.gen_range(1000..9999)),
                    ),
                ];
                let count = self.rng.gen_range(2..=3);
                let pairs: Vec<String> = fields[..count]
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect();
                Ok(pairs.join("&"))
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "container.{}.{}",
                category, type_name
            ))),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SHARED HELPERS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Compute Luhn check digit for a string of digits.
    /// Returns the single digit (0-9) that, when appended, makes the number Luhn-valid.
    fn luhn_check_digit(&self, digits: &str) -> u8 {
        let sum: u32 = digits
            .bytes()
            .rev()
            .enumerate()
            .map(|(i, b)| {
                let mut d = (b - b'0') as u32;
                if i % 2 == 0 {
                    d *= 2;
                    if d > 9 {
                        d -= 9;
                    }
                }
                d
            })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    /// Compute EAN check digit (weighted sum with alternating weights 1 and 3).
    /// Works for both EAN-13 (12 input digits) and EAN-8 (7 input digits).
    fn ean_check_digit(&self, digits: &str) -> u8 {
        let sum: u32 = digits
            .bytes()
            .enumerate()
            .map(|(i, b)| {
                let d = (b - b'0') as u32;
                if i % 2 == 0 {
                    d
                } else {
                    d * 3
                }
            })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    /// Compute ISBN-13 check digit (same algorithm as EAN-13).
    /// Input: 12-digit string. Returns single check digit 0-9.
    fn isbn13_check_digit(&self, digits: &str) -> u8 {
        self.ean_check_digit(digits)
    }

    /// Compute ISBN-10 check digit.
    /// Input: 9-digit string. Returns check character ('0'-'9' or 'X').
    fn isbn10_check_digit(&self, digits: &str) -> char {
        let sum: u32 = digits
            .bytes()
            .enumerate()
            .map(|(i, b)| {
                let d = (b - b'0') as u32;
                d * (10 - i as u32)
            })
            .sum();
        let remainder = (11 - (sum % 11)) % 11;
        if remainder == 10 {
            'X'
        } else {
            (b'0' + remainder as u8) as char
        }
    }

    /// Convert alphanumeric character to numeric value for ISIN/CUSIP/SEDOL.
    /// Digits 0-9 map to 0-9, letters A-Z map to 10-35.
    fn alpha_to_num(c: char) -> u32 {
        if c.is_ascii_digit() {
            (c as u32) - ('0' as u32)
        } else {
            (c.to_ascii_uppercase() as u32) - ('A' as u32) + 10
        }
    }

    /// Compute ISIN check digit using Luhn algorithm on alpha-to-numeric expanded string.
    /// Each letter is expanded to two digits (A=10, B=11, ..., Z=35), then standard Luhn.
    fn isin_check_digit(&self, body: &str) -> u8 {
        // Expand alphanumeric to digit string
        let expanded: String = body
            .chars()
            .flat_map(|c| {
                let val = Self::alpha_to_num(c);
                if val >= 10 {
                    vec![(val / 10) as u8 + b'0', (val % 10) as u8 + b'0']
                } else {
                    vec![val as u8 + b'0']
                }
            })
            .map(|b| b as char)
            .collect();
        self.luhn_check_digit(&expanded)
    }

    /// Compute CUSIP check digit.
    /// Characters at even positions (0-indexed) have face value;
    /// characters at odd positions are doubled. Sum mod 10.
    fn cusip_check_digit(&self, body: &str) -> u8 {
        let sum: u32 = body
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let mut val = Self::alpha_to_num(c);
                if i % 2 == 1 {
                    val *= 2;
                }
                (val / 10) + (val % 10)
            })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    /// Compute SEDOL check digit using weights [1, 3, 1, 7, 3, 9].
    /// Letters: B=11, C=12, ..., Z=35 (vowels skipped in valid SEDOLs but
    /// the algorithm still maps them if present).
    fn sedol_check_digit(&self, body: &str) -> u8 {
        let weights = [1u32, 3, 1, 7, 3, 9];
        let sum: u32 = body
            .chars()
            .zip(weights.iter())
            .map(|(c, &w)| Self::alpha_to_num(c) * w)
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    /// Compute LEI check digits using ISO 7064 Mod 97-10 (same as IBAN).
    /// Returns a 2-character string (e.g., "55", "72", "02").
    fn lei_check_digits(&self, body: &str) -> String {
        // Convert letters to numbers: A=10, B=11, ..., Z=35
        let expanded: String = body
            .chars()
            .flat_map(|c| {
                let val = Self::alpha_to_num(c);
                if val >= 10 {
                    format!("{}", val).chars().collect::<Vec<_>>()
                } else {
                    vec![c]
                }
            })
            .collect();
        // Append "00" for check digit calculation
        let with_zeros = format!("{}00", expanded);
        // Compute mod 97 on the large number (process in chunks to avoid overflow)
        let mut remainder: u64 = 0;
        for chunk in with_zeros.as_bytes().chunks(9) {
            let s: String = chunk.iter().map(|&b| b as char).collect();
            let combined = format!("{}{}", remainder, s);
            remainder = combined.parse::<u64>().unwrap_or(0) % 97;
        }
        let check = 98 - remainder;
        format!("{:02}", check)
    }

    fn random_datetime(&mut self) -> NaiveDateTime {
        let year = self.rng.gen_range(2015..2030);
        let month = self.rng.gen_range(1..=12);
        let day = self.rng.gen_range(1..=28);
        let hour = self.rng.gen_range(0..24);
        let minute = self.rng.gen_range(0..60);
        let second = self.rng.gen_range(0..60);
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    fn gen_hex_string(&mut self, char_count: usize) -> String {
        (0..char_count / 2)
            .map(|_| format!("{:02x}", self.rng.gen::<u8>()))
            .collect()
    }

    fn random_word(&mut self) -> String {
        let words = [
            "apple", "banana", "cherry", "data", "engine", "format", "graph", "hash", "index",
            "join", "kernel", "lambda", "matrix", "node", "object", "parse", "query", "route",
            "schema", "table", "union", "value", "widget", "xenon", "yield", "zone", "alpha",
            "beta", "gamma", "delta", "echo", "foxtrot", "golf", "hotel", "india", "juliet",
            "kilo", "lima", "mike", "november", "oscar", "papa", "quebec", "romeo", "sierra",
            "tango", "uniform", "victor", "whiskey", "xray", "yankee", "zulu", "red", "blue",
            "green", "orange", "purple", "silver", "golden", "dark", "light", "fast", "slow",
            "big", "small", "new", "old", "north", "south", "east", "west", "spring", "summer",
            "autumn", "winter", "sun", "moon", "star", "cloud",
        ];
        words[self.rng.gen_range(0..words.len())].to_string()
    }

    fn random_first_name(&mut self) -> String {
        let names = locale_data::first_names(self.current_locale());
        names[self.rng.gen_range(0..names.len())].to_string()
    }

    fn random_last_name(&mut self) -> String {
        let names = locale_data::last_names(self.current_locale());
        names[self.rng.gen_range(0..names.len())].to_string()
    }

    /// Get the current locale, defaulting to "EN" if not set.
    fn current_locale(&self) -> &str {
        self.locale.as_deref().unwrap_or("EN")
    }

    /// Generate a phone number for the current locale.
    /// Produces NATIONAL, INTERNATIONAL, and E164 formats with realistic
    /// spacing and punctuation derived from Google's libphonenumber data.
    /// Covers 40+ country/region formats across all locales.
    fn gen_phone_number(&mut self) -> Result<String, GeneratorError> {
        let locale = self.current_locale().to_string();

        // Format selection: ~35% national, ~40% international, ~25% E164
        let fmt_roll = self.rng.gen_range(0..20);
        let fmt = if fmt_roll < 7 {
            PhoneFmt::National
        } else if fmt_roll < 15 {
            PhoneFmt::International
        } else {
            PhoneFmt::E164
        };

        match locale.as_str() {
            "EN_US" | "EN_CA" | "EN" => {
                // NANP: US, Canada — (AAA) AAA-AAAA / +1 AAA-AAA-AAAA
                let area = self.rng.gen_range(200..999);
                let exc = self.rng.gen_range(200..999);
                let sub = self.rng.gen_range(1000..9999);
                Ok(match fmt {
                    PhoneFmt::National => format!("({:03}) {:03}-{:04}", area, exc, sub),
                    PhoneFmt::International => format!("+1 {:03}-{:03}-{:04}", area, exc, sub),
                    PhoneFmt::E164 => format!("+1{:03}{:03}{:04}", area, exc, sub),
                })
            }
            "EN_GB" => {
                // UK: landline (02x) or mobile (07xxx)
                if self.rng.gen_bool(0.5) {
                    // Mobile: 07AAA AAAAAA
                    let p = self.rng.gen_range(700..799);
                    let a = self.rng.gen_range(100..999);
                    let b = self.rng.gen_range(100..999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{:03}0 {:03}{:03}", p, a, b),
                        PhoneFmt::International => format!("+44 {:03}0 {:03}{:03}", p, a, b),
                        PhoneFmt::E164 => format!("+44{:03}0{:03}{:03}", p, a, b),
                    })
                } else {
                    // London landline: 020 AAAA AAAA
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("020 {:04} {:04}", a, b),
                        PhoneFmt::International => format!("+44 20 {:04} {:04}", a, b),
                        PhoneFmt::E164 => format!("+4420{:04}{:04}", a, b),
                    })
                }
            }
            "EN_AU" => {
                // Australia: mobile 04AA AAA AAA or landline (0A) AAAA AAAA
                if self.rng.gen_bool(0.6) {
                    let p = self.rng.gen_range(10..99);
                    let a = self.rng.gen_range(100..999);
                    let b = self.rng.gen_range(100..999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("04{:02} {:03} {:03}", p, a, b),
                        PhoneFmt::International => format!("+61 4{:02} {:03} {:03}", p, a, b),
                        PhoneFmt::E164 => format!("+614{:02}{:03}{:03}", p, a, b),
                    })
                } else {
                    let area = [2, 3, 7, 8][self.rng.gen_range(0..4)];
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("(0{}) {:04} {:04}", area, a, b),
                        PhoneFmt::International => format!("+61 {} {:04} {:04}", area, a, b),
                        PhoneFmt::E164 => format!("+61{}{:04}{:04}", area, a, b),
                    })
                }
            }
            "DE" => {
                // Germany: mobile 01512 AAAAAAA or landline 030 AAAAAA
                if self.rng.gen_bool(0.6) {
                    let p = self.rng.gen_range(150..179);
                    let sub = self.rng.gen_range(1000000..9999999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{:03} {:07}", p, sub),
                        PhoneFmt::International => format!("+49 {:03} {:07}", p, sub),
                        PhoneFmt::E164 => format!("+49{:03}{:07}", p, sub),
                    })
                } else {
                    let areas = [30, 40, 69, 89, 211, 221, 351, 511, 711, 911];
                    let area = areas[self.rng.gen_range(0..areas.len())];
                    let sub = self.rng.gen_range(100000..999999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{} {:06}", area, sub),
                        PhoneFmt::International => format!("+49 {} {:06}", area, sub),
                        PhoneFmt::E164 => format!("+49{}{:06}", area, sub),
                    })
                }
            }
            "FR" => {
                // France: 06 AA AA AA AA (mobile) or 01-05 (landline)
                let p = if self.rng.gen_bool(0.6) {
                    self.rng.gen_range(6..8) // mobile
                } else {
                    self.rng.gen_range(1..6) // landline
                };
                let (a, b, c, d) = (
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                );
                Ok(match fmt {
                    PhoneFmt::National => {
                        format!("0{} {:02} {:02} {:02} {:02}", p, a, b, c, d)
                    }
                    PhoneFmt::International => {
                        format!("+33 {} {:02} {:02} {:02} {:02}", p, a, b, c, d)
                    }
                    PhoneFmt::E164 => format!("+33{}{:02}{:02}{:02}{:02}", p, a, b, c, d),
                })
            }
            "ES" => {
                // Spain: mobile 6AA AA AA AA or landline 9AA AA AA AA
                let p = if self.rng.gen_bool(0.6) {
                    self.rng.gen_range(600..699) // mobile
                } else {
                    self.rng.gen_range(910..989) // landline
                };
                let (a, b, c) = (
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                );
                Ok(match fmt {
                    PhoneFmt::National => format!("{:03} {:02} {:02} {:02}", p, a, b, c),
                    PhoneFmt::International => {
                        format!("+34 {:03} {:02} {:02} {:02}", p, a, b, c)
                    }
                    PhoneFmt::E164 => format!("+34{:03}{:02}{:02}{:02}", p, a, b, c),
                })
            }
            "IT" => {
                // Italy: mobile 3AA AAA AAAA or landline 02 AAAA AAAA
                if self.rng.gen_bool(0.6) {
                    let p = self.rng.gen_range(310..399);
                    let a = self.rng.gen_range(100..999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("{:03} {:03} {:04}", p, a, b),
                        PhoneFmt::International => {
                            format!("+39 {:03} {:03} {:04}", p, a, b)
                        }
                        PhoneFmt::E164 => format!("+39{:03}{:03}{:04}", p, a, b),
                    })
                } else {
                    let areas = [2, 6, 11, 51, 55, 81, 91];
                    let area = areas[self.rng.gen_range(0..areas.len())];
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{} {:04} {:04}", area, a, b),
                        PhoneFmt::International => {
                            format!("+39 0{} {:04} {:04}", area, a, b)
                        }
                        PhoneFmt::E164 => format!("+390{}{:04}{:04}", area, a, b),
                    })
                }
            }
            "NL" => {
                // Netherlands: mobile 06 AAAAAAAA or landline 0AA AAA AAAA
                if self.rng.gen_bool(0.6) {
                    let a = self.rng.gen_range(10000000..99999999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("06 {:08}", a),
                        PhoneFmt::International => format!("+31 6 {:08}", a),
                        PhoneFmt::E164 => format!("+316{:08}", a),
                    })
                } else {
                    let area = self.rng.gen_range(10..99);
                    let a = self.rng.gen_range(100..999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{:02} {:03} {:04}", area, a, b),
                        PhoneFmt::International => {
                            format!("+31 {:02} {:03} {:04}", area, a, b)
                        }
                        PhoneFmt::E164 => format!("+31{:02}{:03}{:04}", area, a, b),
                    })
                }
            }
            "PL" => {
                // Poland: mobile 5AA AAA AAA or landline AA AAA AA AA
                let p = self.rng.gen_range(500..899);
                let a = self.rng.gen_range(100..999);
                let b = self.rng.gen_range(100..999);
                Ok(match fmt {
                    PhoneFmt::National => format!("{:03} {:03} {:03}", p, a, b),
                    PhoneFmt::International => {
                        format!("+48 {:03} {:03} {:03}", p, a, b)
                    }
                    PhoneFmt::E164 => format!("+48{:03}{:03}{:03}", p, a, b),
                })
            }
            "RU" => {
                // Russia: mobile 8 (9AA) AAA-AA-AA or +7 9AA AAA-AA-AA
                let p = self.rng.gen_range(900..999);
                let (a, b, c) = (
                    self.rng.gen_range(100..999),
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(10..99),
                );
                Ok(match fmt {
                    PhoneFmt::National => {
                        format!("8 ({:03}) {:03}-{:02}-{:02}", p, a, b, c)
                    }
                    PhoneFmt::International => {
                        format!("+7 {:03} {:03}-{:02}-{:02}", p, a, b, c)
                    }
                    PhoneFmt::E164 => format!("+7{:03}{:03}{:02}{:02}", p, a, b, c),
                })
            }
            "JA" => {
                // Japan: mobile 090-AAAA-AAAA or landline 03-AAAA-AAAA
                if self.rng.gen_bool(0.6) {
                    let p = [70, 80, 90][self.rng.gen_range(0..3)];
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{}-{:04}-{:04}", p, a, b),
                        PhoneFmt::International => {
                            format!("+81 {}-{:04}-{:04}", p, a, b)
                        }
                        PhoneFmt::E164 => format!("+81{}{:04}{:04}", p, a, b),
                    })
                } else {
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("03-{:04}-{:04}", a, b),
                        PhoneFmt::International => format!("+81 3-{:04}-{:04}", a, b),
                        PhoneFmt::E164 => format!("+813{:04}{:04}", a, b),
                    })
                }
            }
            "ZH" => {
                // China: mobile 131 AAAA AAAA or landline 010 AAAA AAAA
                if self.rng.gen_bool(0.6) {
                    let p = self.rng.gen_range(130..199);
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("{:03} {:04} {:04}", p, a, b),
                        PhoneFmt::International => {
                            format!("+86 {:03} {:04} {:04}", p, a, b)
                        }
                        PhoneFmt::E164 => format!("+86{:03}{:04}{:04}", p, a, b),
                    })
                } else {
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("010 {:04} {:04}", a, b),
                        PhoneFmt::International => format!("+86 10 {:04} {:04}", a, b),
                        PhoneFmt::E164 => format!("+8610{:04}{:04}", a, b),
                    })
                }
            }
            "KO" => {
                // Korea: mobile 010-AAAA-AAAA or landline 02-AAA-AAAA
                if self.rng.gen_bool(0.6) {
                    let p = [10, 11, 16, 17, 18, 19][self.rng.gen_range(0..6)];
                    let a = self.rng.gen_range(1000..9999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("0{}-{:04}-{:04}", p, a, b),
                        PhoneFmt::International => {
                            format!("+82 {}-{:04}-{:04}", p, a, b)
                        }
                        PhoneFmt::E164 => format!("+82{}{:04}{:04}", p, a, b),
                    })
                } else {
                    let a = self.rng.gen_range(100..999);
                    let b = self.rng.gen_range(1000..9999);
                    Ok(match fmt {
                        PhoneFmt::National => format!("02-{:03}-{:04}", a, b),
                        PhoneFmt::International => format!("+82 2-{:03}-{:04}", a, b),
                        PhoneFmt::E164 => format!("+822{:03}{:04}", a, b),
                    })
                }
            }
            "AR" => {
                // Arabic locale covers Saudi Arabia, UAE, Egypt
                let region = self.rng.gen_range(0..3);
                match region {
                    0 => {
                        // Saudi Arabia: mobile 05A AAA AAAA
                        let p = self.rng.gen_range(50..59);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::International => {
                                format!("+966 {:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+966{:02}{:03}{:04}", p, a, b),
                        })
                    }
                    1 => {
                        // UAE: mobile 050 AAA AAAA
                        let p = self.rng.gen_range(50..56);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::International => {
                                format!("+971 {:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+971{:02}{:03}{:04}", p, a, b),
                        })
                    }
                    _ => {
                        // Egypt: mobile 010 AAAAAAAA
                        let p = self.rng.gen_range(10..15);
                        let a = self.rng.gen_range(10000000..99999999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:08}", p, a),
                            PhoneFmt::International => {
                                format!("+20 {:02} {:08}", p, a)
                            }
                            PhoneFmt::E164 => format!("+20{:02}{:08}", p, a),
                        })
                    }
                }
            }
            _ => {
                // Default: randomly pick from 20+ diverse international formats
                // Covers BR, MX, IN, TH, MY, SG, PH, ID, TW, HK, NZ, IE,
                // SE, NO, DK, CH, AT, BE, PT, TR, IL, GR, ZA, NG, KE, CL, CO, AR, FI
                let region = self.rng.gen_range(0..30);
                match region {
                    0 => {
                        // Brazil: (AA) AAAAA-AAAA
                        let a = self.rng.gen_range(11..99);
                        let b = self.rng.gen_range(90000..99999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("({:02}) {:05}-{:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+55 {:02} {:05}-{:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+55{:02}{:05}{:04}", a, b, c),
                        })
                    }
                    1 => {
                        // Mexico: AAA AAA AAAA
                        let a = self.rng.gen_range(200..999);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("{:03} {:03} {:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+52 {:03} {:03} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+52{:03}{:03}{:04}", a, b, c),
                        })
                    }
                    2 => {
                        // India: 0AAAAA AAAAA
                        let a = self.rng.gen_range(70000..99999);
                        let b = self.rng.gen_range(10000..99999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:05} {:05}", a, b),
                            PhoneFmt::International => {
                                format!("+91 {:05} {:05}", a, b)
                            }
                            PhoneFmt::E164 => format!("+91{:05}{:05}", a, b),
                        })
                    }
                    3 => {
                        // Thailand: 0AA AAA AAAA
                        let a = self.rng.gen_range(80..99);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:03} {:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+66 {:02} {:03} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+66{:02}{:03}{:04}", a, b, c),
                        })
                    }
                    4 => {
                        // Malaysia: 0AA-AAA AAAA
                        let a = self.rng.gen_range(12..19);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02}-{:03} {:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+60 {:02}-{:03} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+60{:02}{:03}{:04}", a, b, c),
                        })
                    }
                    5 => {
                        // Singapore: AAAA AAAA
                        let a = self.rng.gen_range(6000..9999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("{:04} {:04}", a, b),
                            PhoneFmt::International => format!("+65 {:04} {:04}", a, b),
                            PhoneFmt::E164 => format!("+65{:04}{:04}", a, b),
                        })
                    }
                    6 => {
                        // Philippines: 0AAA AAA AAAA
                        let a = self.rng.gen_range(900..999);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("(0{:03}) {:03} {:04}", a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+63 {:03} {:03} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+63{:03}{:03}{:04}", a, b, c),
                        })
                    }
                    7 => {
                        // Indonesia: 0AAA-AAA-AAA
                        let a = self.rng.gen_range(811..899);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(100..999);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:03}-{:03}-{:03}", a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+62 {:03}-{:03}-{:03}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+62{:03}{:03}{:03}", a, b, c),
                        })
                    }
                    8 => {
                        // Taiwan: 09AA AAA AAA
                        let a = self.rng.gen_range(10..99);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(100..999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("09{:02} {:03} {:03}", a, b, c),
                            PhoneFmt::International => {
                                format!("+886 9{:02} {:03} {:03}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+8869{:02}{:03}{:03}", a, b, c),
                        })
                    }
                    9 => {
                        // Hong Kong: AAAA AAAA
                        let a = self.rng.gen_range(5000..9999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("{:04} {:04}", a, b),
                            PhoneFmt::International => format!("+852 {:04} {:04}", a, b),
                            PhoneFmt::E164 => format!("+852{:04}{:04}", a, b),
                        })
                    }
                    10 => {
                        // New Zealand: 0AA AAA AAAA
                        let a = self.rng.gen_range(21..29);
                        let b = self.rng.gen_range(100..999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:03} {:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+64 {:02} {:03} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+64{:02}{:03}{:04}", a, b, c),
                        })
                    }
                    11 => {
                        // Ireland: 085 AAA AAAA
                        let p = [83, 85, 86, 87, 89][self.rng.gen_range(0..5)];
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:03} {:04}", p, a, b),
                            PhoneFmt::International => {
                                format!("+353 {:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+353{:02}{:03}{:04}", p, a, b),
                        })
                    }
                    12 => {
                        // Sweden: 070-AAA AA AA
                        let p = self.rng.gen_range(70..79);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(10..99);
                        let c = self.rng.gen_range(10..99);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:02}-{:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+46 {:02} {:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::E164 => format!("+46{:02}{:03}{:02}{:02}", p, a, b, c),
                        })
                    }
                    13 => {
                        // Norway: AA AA AA AA
                        let (a, b, c, d) = (
                            self.rng.gen_range(40..99),
                            self.rng.gen_range(10..99),
                            self.rng.gen_range(10..99),
                            self.rng.gen_range(10..99),
                        );
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("{:02} {:02} {:02} {:02}", a, b, c, d)
                            }
                            PhoneFmt::International => {
                                format!("+47 {:02} {:02} {:02} {:02}", a, b, c, d)
                            }
                            PhoneFmt::E164 => format!("+47{:02}{:02}{:02}{:02}", a, b, c, d),
                        })
                    }
                    14 => {
                        // Denmark: AA AA AA AA
                        let (a, b, c, d) = (
                            self.rng.gen_range(20..99),
                            self.rng.gen_range(10..99),
                            self.rng.gen_range(10..99),
                            self.rng.gen_range(10..99),
                        );
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("{:02} {:02} {:02} {:02}", a, b, c, d)
                            }
                            PhoneFmt::International => {
                                format!("+45 {:02} {:02} {:02} {:02}", a, b, c, d)
                            }
                            PhoneFmt::E164 => format!("+45{:02}{:02}{:02}{:02}", a, b, c, d),
                        })
                    }
                    15 => {
                        // Switzerland: 078 AAA AA AA
                        let p = self.rng.gen_range(76..79);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(10..99);
                        let c = self.rng.gen_range(10..99);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:02} {:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+41 {:02} {:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::E164 => format!("+41{:02}{:03}{:02}{:02}", p, a, b, c),
                        })
                    }
                    16 => {
                        // Austria: 0664 AAAAAA
                        let p = [664, 676, 680, 699][self.rng.gen_range(0..4)];
                        let a = self.rng.gen_range(100000..999999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:03} {:06}", p, a),
                            PhoneFmt::International => format!("+43 {:03} {:06}", p, a),
                            PhoneFmt::E164 => format!("+43{:03}{:06}", p, a),
                        })
                    }
                    17 => {
                        // Belgium: 0450 AA AA AA
                        let p = self.rng.gen_range(450..499);
                        let (a, b, c) = (
                            self.rng.gen_range(0..99),
                            self.rng.gen_range(10..99),
                            self.rng.gen_range(10..99),
                        );
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:03} {:02} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+32 {:03} {:02} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::E164 => format!("+32{:03}{:02}{:02}{:02}", p, a, b, c),
                        })
                    }
                    18 => {
                        // Portugal: 912 AAA AAA
                        let p = self.rng.gen_range(910..969);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(100..999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("{:03} {:03} {:03}", p, a, b),
                            PhoneFmt::International => {
                                format!("+351 {:03} {:03} {:03}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+351{:03}{:03}{:03}", p, a, b),
                        })
                    }
                    19 => {
                        // Turkey: (0AAA) AAA AA AA
                        let p = self.rng.gen_range(500..559);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(10..99);
                        let c = self.rng.gen_range(10..99);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("(0{:03}) {:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::International => {
                                format!("+90 {:03} {:03} {:02} {:02}", p, a, b, c)
                            }
                            PhoneFmt::E164 => {
                                format!("+90{:03}{:03}{:02}{:02}", p, a, b, c)
                            }
                        })
                    }
                    20 => {
                        // Israel: 050-AAA-AAAA
                        let p = [50, 52, 53, 54, 58][self.rng.gen_range(0..5)];
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02}-{:03}-{:04}", p, a, b),
                            PhoneFmt::International => {
                                format!("+972 {:02}-{:03}-{:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+972{:02}{:03}{:04}", p, a, b),
                        })
                    }
                    21 => {
                        // Greece: 69A AAA AAAA
                        let p = self.rng.gen_range(690..699);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("{:03} {:03} {:04}", p, a, b),
                            PhoneFmt::International => {
                                format!("+30 {:03} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+30{:03}{:03}{:04}", p, a, b),
                        })
                    }
                    22 => {
                        // South Africa: 071 AAA AAAA
                        let p = self.rng.gen_range(71..84);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:03} {:04}", p, a, b),
                            PhoneFmt::International => {
                                format!("+27 {:02} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+27{:02}{:03}{:04}", p, a, b),
                        })
                    }
                    23 => {
                        // Nigeria: 0802 AAA AAAA
                        let p = self.rng.gen_range(800..909);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => {
                                format!("0{:03} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::International => {
                                format!("+234 {:03} {:03} {:04}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+234{:03}{:03}{:04}", p, a, b),
                        })
                    }
                    24 => {
                        // Kenya: 0712 AAAAAA
                        let p = self.rng.gen_range(710..799);
                        let a = self.rng.gen_range(100000..999999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:03} {:06}", p, a),
                            PhoneFmt::International => format!("+254 {:03} {:06}", p, a),
                            PhoneFmt::E164 => format!("+254{:03}{:06}", p, a),
                        })
                    }
                    25 => {
                        // Chile: (A) AAAA AAAA
                        let a = self.rng.gen_range(2..9);
                        let b = self.rng.gen_range(1000..9999);
                        let c = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("({}) {:04} {:04}", a, b, c),
                            PhoneFmt::International => {
                                format!("+56 {} {:04} {:04}", a, b, c)
                            }
                            PhoneFmt::E164 => format!("+56{}{:04}{:04}", a, b, c),
                        })
                    }
                    26 => {
                        // Colombia: (AAA) AAAAAAA
                        let a = self.rng.gen_range(300..399);
                        let b = self.rng.gen_range(1000000..9999999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("({:03}) {:07}", a, b),
                            PhoneFmt::International => format!("+57 {:03} {:07}", a, b),
                            PhoneFmt::E164 => format!("+57{:03}{:07}", a, b),
                        })
                    }
                    27 => {
                        // Argentina: 011 15-AAAA-AAAA
                        let a = self.rng.gen_range(1000..9999);
                        let b = self.rng.gen_range(1000..9999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("011 15-{:04}-{:04}", a, b),
                            PhoneFmt::International => {
                                format!("+54 9 11 {:04}-{:04}", a, b)
                            }
                            PhoneFmt::E164 => format!("+5491{:04}{:04}", a, b),
                        })
                    }
                    28 => {
                        // Finland: 041 AAAAAAA
                        let p = [41, 44, 45, 50][self.rng.gen_range(0..4)];
                        let a = self.rng.gen_range(1000000..9999999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:02} {:07}", p, a),
                            PhoneFmt::International => format!("+358 {:02} {:07}", p, a),
                            PhoneFmt::E164 => format!("+358{:02}{:07}", p, a),
                        })
                    }
                    _ => {
                        // Vietnam: 0912 AAA AAA
                        let p = self.rng.gen_range(900..999);
                        let a = self.rng.gen_range(100..999);
                        let b = self.rng.gen_range(100..999);
                        Ok(match fmt {
                            PhoneFmt::National => format!("0{:03} {:03} {:03}", p, a, b),
                            PhoneFmt::International => {
                                format!("+84 {:03} {:03} {:03}", p, a, b)
                            }
                            PhoneFmt::E164 => format!("+84{:03}{:03}{:03}", p, a, b),
                        })
                    }
                }
            }
        }
    }

    /// Generate a postal code for the current locale.
    fn gen_postal_code(&mut self) -> Result<String, GeneratorError> {
        let fmt = locale_data::postal_format(self.current_locale());
        match fmt {
            "US" => {
                if self.rng.gen_bool(0.7) {
                    Ok(format!("{:05}", self.rng.gen_range(10000..99999)))
                } else {
                    Ok(format!(
                        "{:05}-{:04}",
                        self.rng.gen_range(10000..99999),
                        self.rng.gen_range(1000..9999)
                    ))
                }
            }
            "UK" => Ok(format!(
                "{}{}{} {}{}{}",
                (b'A' + self.rng.gen_range(0..26)) as char,
                self.rng.gen_range(1..9),
                (b'A' + self.rng.gen_range(0..26)) as char,
                self.rng.gen_range(1..9),
                (b'A' + self.rng.gen_range(0..26)) as char,
                (b'A' + self.rng.gen_range(0..26)) as char,
            )),
            "AU" => Ok(format!("{:04}", self.rng.gen_range(2000..9999))),
            "CA" => Ok(format!(
                "{}{}{} {}{}{}",
                (b'A' + self.rng.gen_range(0..26)) as char,
                self.rng.gen_range(1..9),
                (b'A' + self.rng.gen_range(0..26)) as char,
                self.rng.gen_range(1..9),
                (b'A' + self.rng.gen_range(0..26)) as char,
                self.rng.gen_range(1..9),
            )),
            "DE" | "FR" | "ES" | "IT" => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),
            "NL" => Ok(format!(
                "{:04} {}{}",
                self.rng.gen_range(1000..9999),
                (b'A' + self.rng.gen_range(0..26)) as char,
                (b'A' + self.rng.gen_range(0..26)) as char,
            )),
            "PL" => Ok(format!(
                "{:02}-{:03}",
                self.rng.gen_range(10..99),
                self.rng.gen_range(100..999)
            )),
            "RU" => Ok(format!("{:06}", self.rng.gen_range(100000..999999))),
            "JP" => Ok(format!(
                "{:03}-{:04}",
                self.rng.gen_range(100..999),
                self.rng.gen_range(1000..9999)
            )),
            "CN" => Ok(format!("{:06}", self.rng.gen_range(100000..999999))),
            "KR" => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),
            _ => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_taxonomy() -> Taxonomy {
        Taxonomy::from_yaml(
            r#"
test.test.test:
  title: "Test"
  designation: universal
  locales: [UNIVERSAL]
  broad_type: VARCHAR
  release_priority: 1
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_datetime_iso_8601() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("datetime.timestamp.iso_8601").unwrap();
        assert!(val.contains('T'));
        assert!(val.ends_with('Z'));
        assert_eq!(val.len(), 20);
    }

    #[test]
    fn test_datetime_date_us_slash() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("datetime.date.us_slash").unwrap();
        assert_eq!(val.len(), 10);
        assert!(val.contains('/'));
    }

    #[test]
    fn test_technology_ipv4() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("technology.internet.ip_v4").unwrap();
        assert_eq!(val.split('.').count(), 4);
    }

    #[test]
    fn test_technology_uuid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("technology.cryptographic.uuid").unwrap();
        assert_eq!(val.len(), 36);
        assert_eq!(val.split('-').count(), 5);
    }

    #[test]
    fn test_identity_email() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("identity.person.email").unwrap();
        assert!(val.contains('@'));
        assert!(val.contains('.'));
    }

    #[test]
    fn test_identity_phone() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("identity.person.phone_number").unwrap();
        assert!(!val.is_empty());
    }

    #[test]
    fn test_geography_latitude() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("geography.coordinate.latitude").unwrap();
        let lat: f64 = val.parse().unwrap();
        assert!((-90.0..=90.0).contains(&lat));
    }

    #[test]
    fn test_representation_integer() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen
            .generate_value("representation.numeric.integer_number")
            .unwrap();
        let _: i64 = val.parse().unwrap();
    }

    #[test]
    fn test_representation_hex_color() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("representation.text.color_hex").unwrap();
        assert!(val.len() == 7 || val.len() == 6);
    }

    #[test]
    fn test_container_json() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen.generate_value("container.object.json").unwrap();
        assert!(val.starts_with('{'));
        assert!(val.ends_with('}'));
    }

    #[test]
    fn test_container_query_string() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let val = gen
            .generate_value("container.key_value.query_string")
            .unwrap();
        assert!(val.contains('='));
        assert!(val.contains('&'));
    }

    #[test]
    fn test_all_domains_have_generators() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        // Test one type from each domain
        assert!(gen.generate_value("datetime.timestamp.iso_8601").is_ok());
        assert!(gen.generate_value("technology.internet.ip_v4").is_ok());
        assert!(gen.generate_value("identity.person.email").is_ok());
        assert!(gen.generate_value("geography.location.country").is_ok());
        assert!(gen
            .generate_value("representation.numeric.integer_number")
            .is_ok());
        assert!(gen.generate_value("container.object.json").is_ok());
    }

    #[test]
    fn test_unknown_label_returns_error() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        assert!(gen.generate_value("nonexistent.type.foo").is_err());
    }

    #[test]
    fn test_credit_card_luhn_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen
                .generate_value("identity.payment.credit_card_number")
                .unwrap();
            // Verify Luhn validity
            assert!(
                luhn::valid(&val),
                "Credit card number {} failed Luhn check",
                val
            );
            // Verify correct lengths
            assert!(
                val.len() == 15 || val.len() == 16,
                "Credit card length {} unexpected for {}",
                val.len(),
                val
            );
            // Verify correct prefixes
            let first = val.chars().next().unwrap();
            assert!(
                matches!(first, '3' | '4' | '5' | '6'),
                "Unexpected credit card prefix: {}",
                val
            );
        }
    }

    #[test]
    fn test_imei_luhn_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("technology.code.imei").unwrap();
            assert_eq!(val.len(), 15, "IMEI should be 15 digits: {}", val);
            assert!(luhn::valid(&val), "IMEI {} failed Luhn check", val);
        }
    }

    #[test]
    fn test_ean_check_digit_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("technology.code.ean").unwrap();
            assert!(
                val.len() == 8 || val.len() == 13,
                "EAN length {} unexpected for {}",
                val.len(),
                val
            );
            // Verify EAN check digit
            let (body, check_str) = val.split_at(val.len() - 1);
            let expected_check = {
                let sum: u32 = body
                    .bytes()
                    .enumerate()
                    .map(|(i, b)| {
                        let d = (b - b'0') as u32;
                        if i % 2 == 0 {
                            d
                        } else {
                            d * 3
                        }
                    })
                    .sum();
                ((10 - (sum % 10)) % 10) as u8
            };
            let actual_check = check_str.bytes().next().unwrap() - b'0';
            assert_eq!(
                actual_check, expected_check,
                "EAN {} has invalid check digit (expected {}, got {})",
                val, expected_check, actual_check
            );
        }
    }

    #[test]
    fn test_credit_card_network_prefixes() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let mut saw_visa = false;
        let mut saw_mc = false;
        let mut saw_amex = false;
        let mut saw_discover = false;
        for _ in 0..200 {
            let val = gen
                .generate_value("identity.payment.credit_card_number")
                .unwrap();
            if val.starts_with('4') && val.len() == 16 {
                saw_visa = true;
            }
            if val.starts_with("51")
                || val.starts_with("52")
                || val.starts_with("53")
                || val.starts_with("54")
                || val.starts_with("55")
            {
                saw_mc = true;
            }
            if val.starts_with("34") || val.starts_with("37") {
                saw_amex = true;
            }
            if val.starts_with("6011") {
                saw_discover = true;
            }
        }
        assert!(saw_visa, "Should generate Visa cards");
        assert!(saw_mc, "Should generate Mastercard cards");
        assert!(saw_amex, "Should generate Amex cards");
        assert!(saw_discover, "Should generate Discover cards");
    }

    #[test]
    fn test_phone_number_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let mut valid_count = 0;
        let mut national_count = 0;
        let mut intl_count = 0;
        let mut e164_count = 0;
        let total = 200;
        // Test across different locales for diversity
        let locales = ["EN_US", "EN_GB", "EN_AU", "DE", "FR", "ES", "JA"];
        for (i, _) in (0..total).enumerate() {
            gen.locale = Some(locales[i % locales.len()].to_string());
            let val = gen.generate_value("identity.person.phone_number").unwrap();
            // All phone numbers should contain digits
            assert!(
                val.chars().any(|c| c.is_ascii_digit()),
                "Phone number should contain digits: {}",
                val
            );
            // Classify format type
            if val.starts_with('+') {
                if val.contains(' ') || val.contains('-') || val.contains('(') {
                    intl_count += 1;
                } else {
                    e164_count += 1;
                }
            } else {
                national_count += 1;
            }
            // Parse with phonenumber crate (try with + prefix for national numbers)
            if val.starts_with('+') {
                if let Ok(number) = phonenumber::parse(None, &val) {
                    if phonenumber::is_valid(&number) {
                        valid_count += 1;
                    }
                }
            }
        }
        gen.locale = None;
        // At least 50% of +prefixed numbers should pass strict validation
        let intl_total = intl_count + e164_count;
        if intl_total > 0 {
            let valid_pct = valid_count as f64 / intl_total as f64 * 100.0;
            assert!(
                valid_pct >= 50.0,
                "Only {:.0}% of international phone numbers passed validation ({}/{})",
                valid_pct,
                valid_count,
                intl_total
            );
        }
        // Verify format diversity: all three formats should appear
        assert!(national_count > 0, "Should generate NATIONAL format numbers");
        assert!(intl_count > 0, "Should generate INTERNATIONAL format numbers");
        assert!(e164_count > 0, "Should generate E164 format numbers");
    }

    #[test]
    fn test_phone_number_locale_routing() {
        let mut gen = Generator::with_seed(test_taxonomy(), 99);

        // Helper: generate many samples and check that at least one has the expected prefix
        fn locale_produces_prefix(gen: &mut Generator, locale: &str, prefix: &str, national_prefix: &str) {
            gen.locale = Some(locale.to_string());
            let mut saw_intl = false;
            let mut saw_national = false;
            for _ in 0..30 {
                let val = gen.generate_value("identity.person.phone_number").unwrap();
                if val.starts_with(prefix) {
                    saw_intl = true;
                }
                if val.starts_with(national_prefix) || val.starts_with('(') {
                    saw_national = true;
                }
            }
            assert!(
                saw_intl || saw_national,
                "{} should produce {} or {} numbers in 30 samples",
                locale, prefix, national_prefix
            );
        }

        // US locale → +1 or (AAA) format
        locale_produces_prefix(&mut gen, "EN_US", "+1", "(");

        // GB locale → +44 or 0 format
        locale_produces_prefix(&mut gen, "EN_GB", "+44", "0");

        // AU locale → +61 or 0 format
        locale_produces_prefix(&mut gen, "EN_AU", "+61", "0");

        // DE locale → +49 or 0 format
        locale_produces_prefix(&mut gen, "DE", "+49", "0");

        // FR locale → +33 or 0 format
        locale_produces_prefix(&mut gen, "FR", "+33", "0");

        // JA locale → +81 or 0 format
        locale_produces_prefix(&mut gen, "JA", "+81", "0");

        gen.locale = None;
    }

    #[test]
    fn test_locale_aware_names() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);

        // German names
        gen.locale = Some("DE".to_string());
        let first = gen.generate_value("identity.person.first_name").unwrap();
        let de_names = locale_data::first_names("DE");
        assert!(
            de_names.contains(&first.as_str()),
            "DE first name '{}' not in German name list",
            first
        );

        // Japanese names
        gen.locale = Some("JA".to_string());
        let first = gen.generate_value("identity.person.first_name").unwrap();
        let ja_names = locale_data::first_names("JA");
        assert!(
            ja_names.contains(&first.as_str()),
            "JA first name '{}' not in Japanese name list",
            first
        );

        gen.locale = None;
    }

    #[test]
    fn test_locale_aware_months() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);

        // French month names
        gen.locale = Some("FR".to_string());
        let month = gen.generate_value("datetime.component.month_name").unwrap();
        let fr_months = locale_data::month_names("FR");
        assert!(
            fr_months.contains(&month.as_str()),
            "FR month '{}' not in French month list",
            month
        );

        gen.locale = None;
    }

    #[test]
    fn test_localized_generation() {
        let taxonomy = Taxonomy::from_directory("../../labels").unwrap();
        let mut gen = Generator::with_seed(taxonomy, 42);

        let samples = gen.generate_all_localized(5, 2);
        assert!(!samples.is_empty(), "Should generate localized samples");

        // All labels should be 4-level (domain.category.type.LOCALE)
        for sample in &samples {
            let parts: Vec<&str> = sample.label.split('.').collect();
            assert_eq!(parts.len(), 4, "Label should be 4-level: {}", sample.label);
        }

        // Should have UNIVERSAL labels
        let universal_count = samples
            .iter()
            .filter(|s| s.label.ends_with(".UNIVERSAL"))
            .count();
        assert!(universal_count > 0, "Should have UNIVERSAL-suffixed labels");

        // Should have locale labels (not UNIVERSAL)
        let locale_count = samples
            .iter()
            .filter(|s| !s.label.ends_with(".UNIVERSAL"))
            .count();
        assert!(locale_count > 0, "Should have locale-specific labels");
    }

    #[test]
    fn test_isin_check_digit_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("identity.payment.isin").unwrap();
            assert_eq!(val.len(), 12, "ISIN should be 12 chars: {}", val);
            assert!(
                val[..2].chars().all(|c| c.is_ascii_uppercase()),
                "ISIN should start with 2 letters: {}",
                val
            );
            // Verify ISIN check digit by recomputing
            let body = &val[..11];
            let expected = gen.isin_check_digit(body);
            let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
            assert_eq!(
                actual, expected,
                "ISIN {} has invalid check digit (expected {}, got {})",
                val, expected, actual
            );
        }
    }

    #[test]
    fn test_isin_known_values() {
        // Verify against known real ISINs
        let gen = Generator::with_seed(test_taxonomy(), 42);
        // Apple Inc: US0378331005
        assert_eq!(gen.isin_check_digit("US037833100"), 5);
        // SAP SE: DE0007164600
        assert_eq!(gen.isin_check_digit("DE000716460"), 0);
    }

    #[test]
    fn test_cusip_check_digit_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("identity.payment.cusip").unwrap();
            assert_eq!(val.len(), 9, "CUSIP should be 9 chars: {}", val);
            let body = &val[..8];
            let expected = gen.cusip_check_digit(body);
            let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
            assert_eq!(
                actual, expected,
                "CUSIP {} has invalid check digit (expected {}, got {})",
                val, expected, actual
            );
        }
    }

    #[test]
    fn test_sedol_check_digit_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("identity.payment.sedol").unwrap();
            assert_eq!(val.len(), 7, "SEDOL should be 7 chars: {}", val);
            // No vowels allowed in SEDOL
            assert!(
                !val[..6].contains(['A', 'E', 'I', 'O', 'U']),
                "SEDOL should not contain vowels: {}",
                val
            );
            let body = &val[..6];
            let expected = gen.sedol_check_digit(body);
            let actual = val.chars().last().unwrap().to_digit(10).unwrap() as u8;
            assert_eq!(
                actual, expected,
                "SEDOL {} has invalid check digit (expected {}, got {})",
                val, expected, actual
            );
        }
    }

    #[test]
    fn test_swift_bic_format() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        let mut saw_8char = false;
        let mut saw_11char = false;
        for _ in 0..100 {
            let val = gen.generate_value("identity.payment.swift_bic").unwrap();
            assert!(
                val.len() == 8 || val.len() == 11,
                "SWIFT/BIC should be 8 or 11 chars: {} (len={})",
                val,
                val.len()
            );
            // First 4 chars must be letters (bank code)
            assert!(
                val[..4].chars().all(|c| c.is_ascii_uppercase()),
                "SWIFT bank code should be uppercase letters: {}",
                val
            );
            // Chars 5-6 must be letters (country code)
            assert!(
                val[4..6].chars().all(|c| c.is_ascii_uppercase()),
                "SWIFT country code should be uppercase letters: {}",
                val
            );
            if val.len() == 8 {
                saw_8char = true;
            }
            if val.len() == 11 {
                saw_11char = true;
            }
        }
        assert!(saw_8char, "Should generate 8-char SWIFT codes");
        assert!(saw_11char, "Should generate 11-char SWIFT codes");
    }

    #[test]
    fn test_lei_check_digits_valid() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..100 {
            let val = gen.generate_value("identity.payment.lei").unwrap();
            assert_eq!(val.len(), 20, "LEI should be 20 chars: {}", val);
            // Verify check digits by recomputing
            let body = &val[..18];
            let expected = gen.lei_check_digits(body);
            let actual = &val[18..];
            assert_eq!(
                actual, expected,
                "LEI {} has invalid check digits (expected {}, got {})",
                val, expected, actual
            );
        }
    }

    #[test]
    fn test_currency_code_format() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..50 {
            let val = gen
                .generate_value("identity.payment.currency_code")
                .unwrap();
            assert_eq!(val.len(), 3, "Currency code should be 3 chars: {}", val);
            assert!(
                val.chars().all(|c| c.is_ascii_uppercase()),
                "Currency code should be uppercase: {}",
                val
            );
        }
    }

    #[test]
    fn test_currency_symbol_format() {
        let mut gen = Generator::with_seed(test_taxonomy(), 42);
        for _ in 0..50 {
            let val = gen
                .generate_value("identity.payment.currency_symbol")
                .unwrap();
            assert!(!val.is_empty(), "Currency symbol should not be empty");
            // Should be short (1-3 chars typically)
            assert!(
                val.len() <= 4,
                "Currency symbol should be short: {} (len={})",
                val,
                val.len()
            );
        }
    }
}
