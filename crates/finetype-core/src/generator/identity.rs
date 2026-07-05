//! Generators for the `identity` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: identity (32 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_identity(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
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
                    0..=2 => Ok(format!("{} {}", first, last)),
                    // "LastName, FirstName" (CSV/database style)
                    3..=4 => Ok(format!("{}, {}", last, first)),
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
            ("person", "email_display") => {
                let first = self.random_first_name();
                let last = self.random_last_name();
                let domains = [
                    "example.com",
                    "corp.com",
                    "company.org",
                    "mail.com",
                    "work.net",
                ];
                let domain = domains[self.rng.gen_range(0..domains.len())];
                let email = format!(
                    "{}.{}@{}",
                    first.to_lowercase(),
                    last.to_lowercase(),
                    domain
                );
                if self.rng.gen_bool(0.5) {
                    Ok(format!("\"{} {}\" <{}>", first, last, email))
                } else {
                    Ok(format!("{} {} <{}>", first, last, email))
                }
            }
            ("person", "phone_e164") => {
                let prefixes: &[(&str, u32)] = &[
                    ("+1", 10),  // US/CA
                    ("+44", 10), // UK
                    ("+61", 9),  // AU
                    ("+33", 9),  // FR
                    ("+49", 10), // DE
                    ("+81", 10), // JP
                    ("+86", 11), // CN
                    ("+91", 10), // IN
                    ("+55", 11), // BR
                    ("+82", 10), // KR
                ];
                let &(prefix, digits) = &prefixes[self.rng.gen_range(0..prefixes.len())];
                let subscriber: String = (0..digits)
                    .map(|i| {
                        if i == 0 {
                            (b'1' + self.rng.gen_range(0..9)) as char
                        } else {
                            (b'0' + self.rng.gen_range(0..10)) as char
                        }
                    })
                    .collect();
                Ok(format!("{}{}", prefix, subscriber))
            }
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
                // FHIR AdministrativeGender (male|female|other|unknown), mixed
                // case to exercise the case-insensitive enum match (ac-02).
                let genders = [
                    "Male", "Female", "Other", "Unknown", "male", "female", "MALE", "FEMALE",
                ];
                Ok(genders[self.rng.gen_range(0..genders.len())].to_string())
            }
            ("person", "gender_code") => {
                // ICAO 9303 alpha (M/F/X) + ISO/IEC 5218 numeric (0/1/2/9).
                let codes = ["M", "F", "X", "0", "1", "2", "9"];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
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
            // credit_card_expiration_date collapsed into datetime.date.month_year_slash
            // cvv removed in taxonomy revision v0.5.1
            // credit_card_network removed (low precision, enum-only)
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
            // paypal_email collapsed into identity.person.email

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
                                                             // Luhn check digit over the 80840-prefixed identifier (ISO issuer
                                                             // prefix for US healthcare), via the shared primitive so the
                                                             // generator and the `npi` validator can never drift.
                let check = self.luhn_check_digit(&format!("80840{}", partial));
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

            // ── medical: new types (4 types) ─────────────────────────────
            ("medical", "icd10") => {
                let letters = b"ABCDEFGHJKLMNPRSTV";
                let first = letters[self.rng.gen_range(0..letters.len())] as char;
                let d1 = self.rng.gen_range(0..10u8);
                let d2 = self.rng.gen_range(0..10u8);
                if self.rng.gen_bool(0.7) {
                    let sub_len = self.rng.gen_range(1..=4);
                    let sub: String = (0..sub_len)
                        .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                        .collect();
                    Ok(format!("{}{}{}.{}", first, d1, d2, sub))
                } else {
                    Ok(format!("{}{}{}", first, d1, d2))
                }
            }
            ("medical", "loinc") => {
                let len = self.rng.gen_range(1..=5usize);
                let num: u32 = match len {
                    1 => self.rng.gen_range(1..10),
                    2 => self.rng.gen_range(10..100),
                    3 => self.rng.gen_range(100..1000),
                    4 => self.rng.gen_range(1000..10000),
                    _ => self.rng.gen_range(10000..100000),
                };
                let check = self.rng.gen_range(0..10u8);
                Ok(format!("{}-{}", num, check))
            }
            ("medical", "cpt") => {
                // CPT (Current Procedural Terminology) — codes only, no
                // AMA-copyrighted descriptors. v17 ac-02 improvements:
                //   - Category I: 5-digit 00100..=99999 (the current range
                //     covers the entire published space; earlier versions
                //     skipped 00100..09999).
                //   - Category II (performance measurement): 4 digits + 'F'.
                //   - Category III (emerging technology): 4 digits + 'T'.
                //   - PLA/temporary "U" suffix retained at low frequency for
                //     compatibility with the existing YAML pattern.
                // Distribution: Category I dominates real-world usage.
                let dice = self.rng.gen_range(0..100u32);
                if dice < 85 {
                    // Category I: zero-padded 5-digit codes, 00100..=99999.
                    let code = self.rng.gen_range(100..=99_999u32);
                    Ok(format!("{:05}", code))
                } else if dice < 93 {
                    // Category II: performance measurement (F suffix).
                    let code = self.rng.gen_range(1..=9999u32);
                    Ok(format!("{:04}F", code))
                } else if dice < 99 {
                    // Category III: emerging technology (T suffix).
                    let code = self.rng.gen_range(1..=9999u32);
                    Ok(format!("{:04}T", code))
                } else {
                    // PLA / temporary (U suffix) — rare.
                    let code = self.rng.gen_range(1..=9999u32);
                    Ok(format!("{:04}U", code))
                }
            }
            ("medical", "hcpcs") => {
                let letter = (b'A' + self.rng.gen_range(0..22u8)) as char; // A-V
                let code = self.rng.gen_range(0..10000u32);
                Ok(format!("{}{:04}", letter, code))
            }

            // ── industry (1 type) ────────────────────────────────────────
            ("industry", "naics") => {
                // Real NAICS 2022 codes across levels (sector → national
                // industry); the closed set lives in labels/sets/naics_codes.txt
                // (membership directive) — the generator samples genuine codes
                // so generated columns pass their own membership guard.
                let codes = [
                    "11", "21", "23", "31", "42", "44", "48", "52", "54", "62", "72", "92", "111",
                    "236", "541", "722", "1111", "2362", "5415", "7225", "11111", "23622", "54151",
                    "72251", "111110", "236220", "445110", "541511", "541512", "621111", "722511",
                    "928120",
                ];
                Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
            }

            // ── government (6 types) ─────────────────────────────────────
            ("government", "vin") => {
                let vin_chars = b"ABCDEFGHJKLMNPRSTUVWXYZ0123456789";
                let vin: String = (0..17)
                    .map(|_| vin_chars[self.rng.gen_range(0..vin_chars.len())] as char)
                    .collect();
                Ok(vin)
            }
            ("government", "eu_vat") => match self.rng.gen_range(0..6u8) {
                0 => Ok(format!("DE{:09}", self.rng.gen_range(0..1_000_000_000u32))),
                1 => {
                    let n: u64 = self.rng.gen_range(0..100_000_000_000u64);
                    Ok(format!("FR{:011}", n))
                }
                2 => Ok(format!("ATU{:08}", self.rng.gen_range(0..100_000_000u32))),
                3 => Ok(format!(
                    "NL{:09}B{:02}",
                    self.rng.gen_range(0..1_000_000_000u32),
                    self.rng.gen_range(1..99u32)
                )),
                4 => {
                    let n: u64 = self.rng.gen_range(0..100_000_000_000u64);
                    Ok(format!("IT{:011}", n))
                }
                _ => {
                    let letter = (b'A' + self.rng.gen_range(0..26)) as char;
                    Ok(format!(
                        "ES{}{:08}",
                        letter,
                        self.rng.gen_range(0..100_000_000u32)
                    ))
                }
            },
            ("government", "ssn") => {
                // US Social Security Number — SYNTHETIC ONLY.
                //
                // PRIVACY: This generator never uses any real-world SSN.
                // Values are uniformly sampled within SSA-valid ranges.
                //
                // v17 ac-02 improvements:
                //   - Area number 001..=899 excluding 666 (SSA-reserved).
                //     Area 900..=999 (tax-ID range) is excluded.
                //   - Group number 01..=99 (00 never issued).
                //   - Serial number 0001..=9999 (0000 never issued).
                //   - Emits both dashed (XXX-XX-XXXX) and no-dash
                //     (XXXXXXXXX) variants to match real-world
                //     column-data diversity. Dashed form is primary.
                let area = loop {
                    let a = self.rng.gen_range(1..=899u32);
                    if a != 666 {
                        break a;
                    }
                };
                let group = self.rng.gen_range(1..=99u32);
                let serial = self.rng.gen_range(1..=9999u32);
                // 80% dashed (canonical), 20% no-dash.
                if self.rng.gen_bool(0.8) {
                    Ok(format!("{:03}-{:02}-{:04}", area, group, serial))
                } else {
                    Ok(format!("{:03}{:02}{:04}", area, group, serial))
                }
            }
            ("government", "ein") => {
                let prefix = self.rng.gen_range(10..99u32);
                let suffix = self.rng.gen_range(0..10_000_000u32);
                Ok(format!("{:02}-{:07}", prefix, suffix))
            }
            ("government", "pan_india") => {
                let holder_types = b"PCHABGJLTF";
                let first3: String = (0..3)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                let fourth = holder_types[self.rng.gen_range(0..holder_types.len())] as char;
                let fifth = (b'A' + self.rng.gen_range(0..26)) as char;
                let digits = self.rng.gen_range(0..10000u32);
                let last = (b'A' + self.rng.gen_range(0..26)) as char;
                Ok(format!(
                    "{}{}{}{:04}{}",
                    first3, fourth, fifth, digits, last
                ))
            }
            ("government", "abn") => {
                // Valid ABN: 11 digits passing the ATO modulus-89 check
                // (weights 10,1,3,5,7,9,11,13,15,17,19 with 1 subtracted from
                // the first digit). Fix the last ten digits, then solve for the
                // first (weight 10) so the weighted sum is ≡ 0 mod 89; the
                // modular inverse of 10 mod 89 is 9. Only ~9/89 of tails admit a
                // single-digit first digit (1–9), so regenerate the tail until
                // one does.
                const W: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
                let mut digits = [0u8; 11];
                loop {
                    for d in digits.iter_mut().skip(1) {
                        *d = self.rng.gen_range(0..10u8);
                    }
                    let rest: u32 = (1..11).map(|i| W[i] * digits[i] as u32).sum();
                    // 10·(d0−1) + rest ≡ 0 (mod 89) → d0−1 ≡ −rest·9 (mod 89).
                    let first_minus_one = (((89 - (rest % 89)) % 89) * 9) % 89;
                    if first_minus_one <= 8 {
                        digits[0] = (first_minus_one + 1) as u8; // d0 ∈ 1..=9
                        break;
                    }
                }
                if self.rng.gen_bool(0.6) {
                    Ok(format!(
                        "{}{} {}{}{} {}{}{} {}{}{}",
                        digits[0],
                        digits[1],
                        digits[2],
                        digits[3],
                        digits[4],
                        digits[5],
                        digits[6],
                        digits[7],
                        digits[8],
                        digits[9],
                        digits[10]
                    ))
                } else {
                    let s: String = digits.iter().map(|d| (b'0' + d) as char).collect();
                    Ok(s)
                }
            }

            // ── academic (1 type) ────────────────────────────────────────
            ("academic", "orcid") => {
                let groups: Vec<String> = (0..3)
                    .map(|_| {
                        let g = self.rng.gen_range(0..10000u16);
                        format!("{:04}", g)
                    })
                    .collect();
                let last3 = self.rng.gen_range(0..1000u16);
                let last_char = if self.rng.gen_bool(0.1) {
                    'X'
                } else {
                    (b'0' + self.rng.gen_range(0..10)) as char
                };
                Ok(format!(
                    "{}-{}-{}-{:03}{}",
                    groups[0], groups[1], groups[2], last3, last_char
                ))
            }

            // ── commerce (5 types) ───────────────────────────────────────
            ("commerce", "isbn") => self.gen_technology("code", "isbn"),
            ("commerce", "ean") => self.gen_technology("code", "ean"),
            ("commerce", "issn") => self.gen_technology("code", "issn"),
            ("commerce", "upc") => {
                // UPC-A: 11 digits + Mod 10 check digit
                let body: Vec<u8> = (0..11).map(|_| self.rng.gen_range(0..10u8)).collect();
                let sum: u32 = body
                    .iter()
                    .enumerate()
                    .map(|(i, &d)| {
                        let d = d as u32;
                        if i % 2 == 0 {
                            d * 3
                        } else {
                            d
                        }
                    })
                    .sum();
                let check = ((10 - (sum % 10)) % 10) as u8;
                let s: String = body.iter().map(|d| (b'0' + d) as char).collect();
                Ok(format!("{}{}", s, check))
            }
            ("commerce", "isrc") => {
                let countries = ["US", "GB", "FR", "DE", "JP", "AU", "CA", "SE", "NL", "IT"];
                let cc = countries[self.rng.gen_range(0..countries.len())];
                let registrant: String = (0..3)
                    .map(|_| {
                        if self.rng.gen_bool(0.7) {
                            (b'A' + self.rng.gen_range(0..26)) as char
                        } else {
                            (b'0' + self.rng.gen_range(0..10)) as char
                        }
                    })
                    .collect();
                let year = self.rng.gen_range(0..100u32);
                let designation = self.rng.gen_range(0..100000u32);
                Ok(format!("{}{}{:02}{:05}", cc, registrant, year, designation))
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "identity.{}.{}",
                category, type_name
            ))),
        }
    }
}
