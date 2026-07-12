//! Generators for the `finance` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: finance (3 new types: iban, amount, amount_comma)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_finance(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── banking ──────────────────────────────────────────────────
            ("banking", "iban") => {
                // IBAN: 2-letter country code + 2 check digits + BBAN
                // Country-specific BBAN lengths (digits only for simplicity)
                let iban_specs: &[(&str, usize)] = &[
                    ("GB", 18), // GB: 4 bank + 14 account
                    ("DE", 18), // DE: 8 bank + 10 account
                    ("FR", 23), // FR: 10 bank + 11 account + 2 key
                    ("NL", 14), // NL: 4 bank + 10 account
                    ("ES", 20), // ES: 8 bank + 2 check + 10 account
                    ("IT", 23), // IT: 1 check + 10 bank + 12 account
                    ("BE", 12), // BE: 3 bank + 7 account + 2 check
                    ("AT", 16), // AT: 5 bank + 11 account
                    ("CH", 17), // CH: 5 bank + 12 account
                    ("PT", 21), // PT: 8 bank + 11 account + 2 check
                    ("SE", 20), // SE: 3 bank + 17 account
                    ("NO", 11), // NO: 4 bank + 6 account + 1 check
                    ("DK", 14), // DK: 4 bank + 10 account
                    ("FI", 14), // FI: 6 bank + 7 account + 1 check
                    ("LU", 16), // LU: 3 bank + 13 account
                    ("IE", 18), // IE: 4 bank + 6 branch + 8 account
                ];
                let &(country, bban_len) = &iban_specs[self.rng.gen_range(0..iban_specs.len())];
                // Generate BBAN (digits, with some countries using letters for bank code)
                let bban: String = if country == "GB" || country == "IE" {
                    // UK/Ireland: 4 alpha bank code + rest digits
                    let bank: String = (0..4)
                        .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                        .collect();
                    let digits: String = (0..(bban_len - 4))
                        .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                        .collect();
                    format!("{}{}", bank, digits)
                } else {
                    // Most countries: all digits in BBAN
                    (0..bban_len)
                        .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                        .collect()
                };
                // Calculate mod-97 check digits
                let check = self.iban_check_digits(country, &bban);
                Ok(format!("{}{}{}", country, check, bban))
            }
            ("banking", "swift_bic") => {
                // SWIFT/BIC generator.
                // Broadened ISO 3166-1 alpha-2 coverage (~130 codes
                // from the active SWIFT country list), biased toward major
                // financial centres to mirror real-world frequency.
                //
                // Structure: 4-letter bank + 2-letter country (ISO 3166-1
                // alpha-2) + 2-char location (alphanumeric) + optional 3-char
                // branch (alphanumeric). "XXX" branch is the documented
                // head-office marker and is emitted with small probability.
                const ISO_3166_ALPHA2: &[&str] = &[
                    "AD", "AE", "AF", "AG", "AL", "AM", "AO", "AR", "AT", "AU", "AZ", "BA", "BB",
                    "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BN", "BO", "BR", "BS", "BT", "BW",
                    "BY", "BZ", "CA", "CD", "CF", "CG", "CH", "CI", "CL", "CM", "CN", "CO", "CR",
                    "CU", "CV", "CY", "CZ", "DE", "DJ", "DK", "DO", "DZ", "EC", "EE", "EG", "ER",
                    "ES", "ET", "FI", "FJ", "FR", "GA", "GB", "GE", "GH", "GM", "GN", "GQ", "GR",
                    "GT", "GW", "GY", "HK", "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN",
                    "IQ", "IR", "IS", "IT", "JM", "JO", "JP", "KE", "KG", "KH", "KR", "KW", "KY",
                    "KZ", "LA", "LB", "LC", "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA",
                    "MC", "MD", "ME", "MG", "MK", "ML", "MM", "MN", "MO", "MR", "MT", "MU", "MV",
                    "MW", "MX", "MY", "MZ", "NA", "NE", "NG", "NI", "NL", "NO", "NP", "NZ", "OM",
                    "PA", "PE", "PG", "PH", "PK", "PL", "PT", "PY", "QA", "RO", "RS", "RU", "RW",
                    "SA", "SB", "SC", "SD", "SE", "SG", "SI", "SK", "SL", "SN", "SO", "SR", "SV",
                    "SY", "SZ", "TD", "TG", "TH", "TJ", "TM", "TN", "TO", "TR", "TT", "TW", "TZ",
                    "UA", "UG", "US", "UY", "UZ", "VC", "VE", "VG", "VN", "VU", "WS", "YE", "ZA",
                    "ZM", "ZW",
                ];
                // Major financial centres — sampled with 40% probability to
                // keep the distribution realistic.
                const MAJORS: &[&str] = &[
                    "US", "GB", "DE", "FR", "CH", "JP", "AU", "SG", "HK", "NL", "IT", "ES", "CA",
                    "SE", "BE", "AT", "IE", "LU",
                ];
                let country = if self.rng.gen_bool(0.4) {
                    MAJORS[self.rng.gen_range(0..MAJORS.len())]
                } else {
                    ISO_3166_ALPHA2[self.rng.gen_range(0..ISO_3166_ALPHA2.len())]
                };
                let bank: String = (0..4)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                // Location: alphanumeric; bias toward letters to look realistic.
                let location: String = (0..2)
                    .map(|_| {
                        if self.rng.gen_bool(0.75) {
                            (b'A' + self.rng.gen_range(0..26)) as char
                        } else {
                            (b'0' + self.rng.gen_range(0..10)) as char
                        }
                    })
                    .collect();
                // 45% emit 11-char (8-char head office + 3-char branch).
                if self.rng.gen_bool(0.45) {
                    // 15% of branched codes use the documented "XXX" marker
                    // (head office by convention when 11-char form is emitted).
                    let branch: String = if self.rng.gen_bool(0.15) {
                        "XXX".to_string()
                    } else {
                        (0..3)
                            .map(|_| {
                                if self.rng.gen_bool(0.7) {
                                    (b'A' + self.rng.gen_range(0..26)) as char
                                } else {
                                    (b'0' + self.rng.gen_range(0..10)) as char
                                }
                            })
                            .collect()
                    };
                    Ok(format!("{}{}{}{}", bank, country, location, branch))
                } else {
                    Ok(format!("{}{}{}", bank, country, location))
                }
            }

            // ── currency ─────────────────────────────────────────────────
            ("currency", "amount") => {
                // US format: $1,234.56
                let symbols = ["$", "£", "¥"];
                let symbol = symbols[self.rng.gen_range(0..symbols.len())];
                let r = self.rng.gen::<f64>();
                let (integer_part, cents) = if r < 0.3 {
                    // Large amounts
                    (
                        self.rng.gen_range(1_000i64..10_000_000),
                        self.rng.gen_range(0..100u32),
                    )
                } else if r < 0.5 {
                    // Small amounts
                    (self.rng.gen_range(0i64..100), self.rng.gen_range(0..100u32))
                } else {
                    // Medium amounts
                    (
                        self.rng.gen_range(100i64..100_000),
                        self.rng.gen_range(0..100u32),
                    )
                };
                // Format integer part with comma thousands separators
                let int_str = integer_part.to_string();
                let mut with_sep = String::new();
                for (i, ch) in int_str.chars().rev().enumerate() {
                    if i > 0 && i % 3 == 0 {
                        with_sep.push(',');
                    }
                    with_sep.push(ch);
                }
                let formatted_int: String = with_sep.chars().rev().collect();
                let neg_style = self.rng.gen::<f64>();
                if neg_style < 0.05 {
                    // Accounting notation: parenthesized negative
                    Ok(format!("({}{}.{:02})", symbol, formatted_int, cents))
                } else if neg_style < 0.15 {
                    // Standard negative
                    Ok(format!("-{}{}.{:02}", symbol, formatted_int, cents))
                } else {
                    Ok(format!("{}{}.{:02}", symbol, formatted_int, cents))
                }
            }
            ("currency", "amount_comma") => {
                // EU format: €1.234,56
                let r = self.rng.gen::<f64>();
                let (integer_part, cents) = if r < 0.3 {
                    (
                        self.rng.gen_range(1_000i64..10_000_000),
                        self.rng.gen_range(0..100u32),
                    )
                } else if r < 0.5 {
                    (self.rng.gen_range(0i64..100), self.rng.gen_range(0..100u32))
                } else {
                    (
                        self.rng.gen_range(100i64..100_000),
                        self.rng.gen_range(0..100u32),
                    )
                };
                // Format integer part with period thousands separators
                let int_str = integer_part.to_string();
                let mut with_sep = String::new();
                for (i, ch) in int_str.chars().rev().enumerate() {
                    if i > 0 && i % 3 == 0 {
                        with_sep.push('.');
                    }
                    with_sep.push(ch);
                }
                let formatted_int: String = with_sep.chars().rev().collect();
                let is_negative = self.rng.gen_bool(0.1);
                let symbol_pos = self.rng.gen_range(0..3); // 0=prefix, 1=suffix, 2=prefix
                match symbol_pos {
                    0 | 2 => {
                        if is_negative {
                            Ok(format!("-€{},{:02}", formatted_int, cents))
                        } else {
                            Ok(format!("€{},{:02}", formatted_int, cents))
                        }
                    }
                    _ => {
                        if is_negative {
                            Ok(format!("-{},{:02} €", formatted_int, cents))
                        } else {
                            Ok(format!("{},{:02} €", formatted_int, cents))
                        }
                    }
                }
            }

            // ── currency metadata (moved from identity.payment) ──────────
            ("currency", "currency_code") => {
                let codes = [
                    "USD", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD", "CNY", "HKD", "NZD", "SEK",
                    "NOK", "DKK", "SGD", "KRW", "INR", "BRL", "ZAR", "MXN", "TWD", "THB", "IDR",
                    "MYR", "PHP", "PLN", "CZK", "HUF", "TRY", "ILS", "AED", "SAR", "RUB", "CLP",
                    "COP", "PEN", "ARS", "EGP", "NGN", "KES", "GHS",
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }
            ("currency", "currency_symbol") => {
                let symbols = [
                    "$", "€", "£", "¥", "₹", "₩", "₿", "₽", "₺", "₴", "₸", "₡", "₵", "₫", "₭", "₮",
                    "₱", "₲", "₳", "₦", "৳", "฿", "₪", "﷼", "₢", "₣", "₤", "₧", "₯", "₰",
                ];
                Ok(symbols[self.rng.gen_range(0..symbols.len())].to_string())
            }

            // ── payment (moved from identity.payment) ────────────────────
            ("payment", "credit_card_number") => self.gen_identity("payment", "credit_card_number"),

            // ── securities (moved from identity.payment) ─────────────────
            ("securities", "cusip") => self.gen_identity("payment", "cusip"),
            ("securities", "isin") => self.gen_identity("payment", "isin"),
            ("securities", "sedol") => self.gen_identity("payment", "sedol"),
            ("securities", "lei") => self.gen_identity("payment", "lei"),
            ("securities", "figi") => {
                // FIGI: 2 consonants + 'G' + 8 alphanumeric (no vowels) + 1 check digit (0-9)
                let consonants = "BCDFGHJKLMNPQRSTVWXYZ";
                let alphanum_no_vowels = "BCDFGHJKLMNPQRSTVWXYZ0123456789";
                let prefix: String = (0..2)
                    .map(|_| consonants.as_bytes()[self.rng.gen_range(0..consonants.len())] as char)
                    .collect();
                let body: String = (0..8)
                    .map(|_| {
                        alphanum_no_vowels.as_bytes()
                            [self.rng.gen_range(0..alphanum_no_vowels.len())]
                            as char
                    })
                    .collect();
                // First 11 chars = provider + 'G' + body; the 12th is the real
                // check digit (OMG FIGI Modulus-10). Shared with the validator
                // so generated FIGIs pass their own `checksum: figi` guard.
                let body11 = format!("{}G{}", prefix, body);
                let first11: Vec<char> = body11.chars().collect();
                let check = crate::checksum::figi_check_digit(&first11)
                    .expect("FIGI body is alphanumeric by construction");
                Ok(format!("{}{}", body11, check))
            }
            ("securities", "ticker") => {
                // Ticker: 1-5 uppercase letters (real symbols cluster at 3-4),
                // ~15% with a class-share suffix (-A). Shape-valid for the
                // `^[A-Z]{1,7}([./-][A-Z]{1,4})?$` pattern; the substance is
                // `membership: us_tickers`, not shape, so shape-valid suffices
                // for the taxonomy check.
                let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
                let len = self.rng.gen_range(1..=5);
                let base: String = (0..len)
                    .map(|_| letters.as_bytes()[self.rng.gen_range(0..letters.len())] as char)
                    .collect();
                if self.rng.gen_range(0..100) < 15 {
                    let cls = letters.as_bytes()[self.rng.gen_range(0..letters.len())] as char;
                    Ok(format!("{}-{}", base, cls))
                } else {
                    Ok(base)
                }
            }

            // ── banking (aba_routing, bsb) ────────────────────────────────
            ("banking", "aba_routing") => {
                // ABA routing: valid prefix (01-12, 21-32, 61-72, 80) + 6 digits + checksum
                let prefixes = [
                    "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "21",
                    "22", "23", "24", "25", "26", "27", "28", "29", "30", "31", "32", "61", "62",
                    "63", "64", "65", "66", "67", "68", "69", "70", "71", "72", "80",
                ];
                let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                let mid: String = (0..6)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                let digits_str = format!("{}{}", prefix, mid);
                let digits: Vec<u32> = digits_str.bytes().map(|b| (b - b'0') as u32).collect();
                let weights = [3u32, 7, 1, 3, 7, 1, 3, 7];
                let sum: u32 = digits.iter().zip(weights.iter()).map(|(d, w)| d * w).sum();
                let check = (10 - (sum % 10)) % 10;
                Ok(format!("{}{}{}", prefix, mid, check))
            }
            ("banking", "bsb") => {
                // BSB: ###-### format
                let bank_codes = [
                    "012", "013", "014", "033", "034", "035", "062", "063", "064", "082", "083",
                    "084",
                ];
                let bank = bank_codes[self.rng.gen_range(0..bank_codes.len())];
                let branch: String = (0..3)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                Ok(format!("{}-{}", bank, branch))
            }

            // ── crypto (moved from identity.payment) ─────────────────────
            ("crypto", "bitcoin_address") => self.gen_identity("payment", "bitcoin_address"),
            ("crypto", "ethereum_address") => self.gen_identity("payment", "ethereum_address"),

            // ── new currency types (11 types) ────────────────────────────
            ("currency", "amount_accounting") => {
                // ($1,234.56) for negatives, $1,234.56 for positives
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, ',');
                let is_negative = self.rng.gen_bool(0.4);
                if is_negative {
                    Ok(format!("(${}.{:02})", formatted, cents))
                } else {
                    Ok(format!("${}.{:02}", formatted, cents))
                }
            }
            ("currency", "amount_comma_suffix") => {
                // 1.234,56 € — period thousands, comma decimal, single-char symbol suffix
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, '.');
                let symbols = ["€", "£", "¥", "₹"];
                let sym = symbols[self.rng.gen_range(0..symbols.len())];
                let is_negative = self.rng.gen_bool(0.1);
                if is_negative {
                    Ok(format!("-{},{:02} {}", formatted, cents, sym))
                } else {
                    Ok(format!("{},{:02} {}", formatted, cents, sym))
                }
            }
            ("currency", "amount_space") => {
                // 1 234,56 € — space thousands, comma decimal, single-char symbol suffix
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, ' ');
                let symbols = ["€", "£", "¥", "₹"];
                let sym = symbols[self.rng.gen_range(0..symbols.len())];
                let is_negative = self.rng.gen_bool(0.1);
                if is_negative {
                    Ok(format!("-{},{:02} {}", formatted, cents, sym))
                } else {
                    Ok(format!("{},{:02} {}", formatted, cents, sym))
                }
            }
            ("currency", "amount_lakh") => {
                // ₹12,34,567.89 — Indian lakh/crore grouping
                // Pattern requires amounts >= 1000 for proper XX,XX,XXX grouping
                let int_part = if self.rng.gen_bool(0.4) {
                    self.rng.gen_range(1_000i64..10_000_000)
                } else {
                    self.rng.gen_range(1_000i64..100_000)
                };
                let cents = self.rng.gen_range(0..100u32);
                let formatted = Self::format_indian_grouping(int_part);
                // Pattern supports ₹ or Rs. prefix (no negative)
                if self.rng.gen_bool(0.8) {
                    Ok(format!("₹{}.{:02}", formatted, cents))
                } else {
                    Ok(format!("Rs. {}.{:02}", formatted, cents))
                }
            }
            ("currency", "amount_apostrophe") => {
                // CHF 1'234.56 — Swiss apostrophe thousands
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, '\'');
                let is_negative = self.rng.gen_bool(0.1);
                // Pattern: ^(CHF\s?)?-?[0-9]...(\.[0-9]{1,2})?(\s?CHF)?$
                if is_negative {
                    Ok(format!("CHF -{}.{:02}", formatted, cents))
                } else {
                    Ok(format!("CHF {}.{:02}", formatted, cents))
                }
            }
            ("currency", "amount_nodecimal") => {
                // ¥1,234 — zero-decimal currencies (JPY, KRW, VND)
                let int_part = if self.rng.gen_bool(0.3) {
                    self.rng.gen_range(1_000i64..10_000_000)
                } else {
                    self.rng.gen_range(100i64..100_000)
                };
                let formatted = Self::format_int_with_separator(int_part, ',');
                let symbols = ["¥", "₩"];
                let sym = symbols[self.rng.gen_range(0..symbols.len())];
                // Pattern: ^[¥₩\p{Sc}]?-? — symbol then minus
                let is_negative = self.rng.gen_bool(0.1);
                if is_negative {
                    Ok(format!("{}-{}", sym, formatted))
                } else {
                    Ok(format!("{}{}", sym, formatted))
                }
            }
            ("currency", "amount_code_prefix") => {
                // USD 1,234.56 — ISO code prefix
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, ',');
                let codes = ["USD", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD", "CNY"];
                let code = codes[self.rng.gen_range(0..codes.len())];
                // Pattern: ^[A-Z]{3}\s?-? — code then space then minus
                let is_negative = self.rng.gen_bool(0.1);
                if is_negative {
                    Ok(format!("{} -{}.{:02}", code, formatted, cents))
                } else {
                    Ok(format!("{} {}.{:02}", code, formatted, cents))
                }
            }
            ("currency", "amount_crypto") => {
                // 0.00123456 BTC — high decimal precision
                let tickers = ["BTC", "ETH", "SOL", "DOGE", "XRP", "ADA"];
                let ticker = tickers[self.rng.gen_range(0..tickers.len())];
                let whole = self.rng.gen_range(0i64..100);
                let frac = self.rng.gen_range(0u64..100_000_000);
                Ok(format!("{}.{:08} {}", whole, frac, ticker))
            }
            ("currency", "amount_multisym") => {
                // R$ 1.234,56 — multi-character symbol prefix, EU-style separators
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, '.');
                let symbols = ["R$", "HK$", "S$", "A$", "NZ$", "C$"];
                let sym = symbols[self.rng.gen_range(0..symbols.len())];
                // Pattern: ^(symbol)\s?-? — symbol then minus
                let is_negative = self.rng.gen_bool(0.1);
                if is_negative {
                    Ok(format!("{} -{},{:02}", sym, formatted, cents))
                } else {
                    Ok(format!("{} {},{:02}", sym, formatted, cents))
                }
            }
            ("currency", "amount_neg_trailing") => {
                // $1,234.56- or 1,234.56 CR — trailing negative notation
                // Pattern ALWAYS requires trailing indicator (-|CR|DR)
                let (int_part, cents) = self.random_amount();
                let formatted = Self::format_int_with_separator(int_part, ',');
                let r = self.rng.gen::<f64>();
                if r < 0.4 {
                    Ok(format!("${}.{:02}-", formatted, cents))
                } else if r < 0.7 {
                    Ok(format!("{}.{:02} CR", formatted, cents))
                } else {
                    Ok(format!("{}.{:02} DR", formatted, cents))
                }
            }

            // ── rate (2 types) ───────────────────────────────────────────
            ("rate", "basis_points") => {
                // 125 bps or 125bps
                let bps = self.rng.gen_range(1i32..500);
                let is_negative = self.rng.gen_bool(0.2);
                let val = if is_negative { -bps } else { bps };
                if self.rng.gen_bool(0.6) {
                    Ok(format!("{} bps", val))
                } else {
                    Ok(format!("{}bps", val))
                }
            }
            ("rate", "yield") => {
                // +2.5% or -1.2%
                let whole = self.rng.gen_range(0i32..20);
                let frac = self.rng.gen_range(0..100u32);
                let is_positive = self.rng.gen_bool(0.7);
                if is_positive {
                    Ok(format!("+{}.{:02}%", whole, frac))
                } else {
                    Ok(format!("-{}.{:02}%", whole, frac))
                }
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "finance.{}.{}",
                category, type_name
            ))),
        }
    }
}
