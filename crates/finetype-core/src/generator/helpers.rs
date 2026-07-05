//! Shared helper methods (check digits, formatting, names, phone, address, postal).

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // SHARED HELPERS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Compute Luhn check digit for a string of digits.
    /// Returns the single digit (0-9) that, when appended, makes the number Luhn-valid.
    pub(crate) fn luhn_check_digit(&self, digits: &str) -> u8 {
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

    /// Compute GS1 mod-10 check digit (alternating weights 1 and 3).
    /// Works for any payload length — EAN-13 (12 input digits) and EAN-8 (7).
    /// GS1 defines the weights from the RIGHT (the digit adjacent to the check
    /// has weight 3), so the alternation is anchored at the right end. For
    /// even-length payloads this coincides with a left-anchored alternation, so
    /// EAN-13/ISBN-13 output is unchanged; for EAN-8's odd 7 digits the
    /// right-anchoring is what makes it match the `gs1` validator.
    pub(crate) fn ean_check_digit(&self, digits: &str) -> u8 {
        let sum: u32 = digits
            .bytes()
            .rev()
            .enumerate()
            .map(|(i, b)| {
                let d = (b - b'0') as u32;
                if i % 2 == 0 {
                    d * 3
                } else {
                    d
                }
            })
            .sum();
        ((10 - (sum % 10)) % 10) as u8
    }

    /// Compute ISBN-13 check digit (same algorithm as EAN-13).
    /// Input: 12-digit string. Returns single check digit 0-9.
    pub(crate) fn isbn13_check_digit(&self, digits: &str) -> u8 {
        self.ean_check_digit(digits)
    }

    /// Compute ISBN-10 check digit.
    /// Input: 9-digit string. Returns check character ('0'-'9' or 'X').
    pub(crate) fn isbn10_check_digit(&self, digits: &str) -> char {
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
    pub(crate) fn alpha_to_num(c: char) -> u32 {
        if c.is_ascii_digit() {
            (c as u32) - ('0' as u32)
        } else {
            (c.to_ascii_uppercase() as u32) - ('A' as u32) + 10
        }
    }

    /// Compute ISIN check digit using Luhn algorithm on alpha-to-numeric expanded string.
    /// Each letter is expanded to two digits (A=10, B=11, ..., Z=35), then standard Luhn.
    pub(crate) fn isin_check_digit(&self, body: &str) -> u8 {
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
    pub(crate) fn cusip_check_digit(&self, body: &str) -> u8 {
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
    pub(crate) fn sedol_check_digit(&self, body: &str) -> u8 {
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
    pub(crate) fn lei_check_digits(&self, body: &str) -> String {
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

    /// Compute IBAN check digits using ISO 7064 Mod 97-10.
    /// Input: country code (2 letters) + BBAN. Returns 2-character check digit string.
    /// Algorithm: rearrange to BBAN + country_numeric + "00", compute 98 - (mod 97).
    pub(crate) fn iban_check_digits(&self, country: &str, bban: &str) -> String {
        // Convert country letters to numbers (A=10..Z=35) and append "00"
        let country_numeric: String = country
            .chars()
            .map(|c| format!("{}", Self::alpha_to_num(c)))
            .collect();
        // Rearranged: BBAN + country_numeric + "00"
        // But BBAN may contain letters too (e.g., GB IBANs), so expand those
        let bban_expanded: String = bban
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
        let full = format!("{}{}00", bban_expanded, country_numeric);
        // Compute mod 97 on the large number (process in chunks to avoid overflow)
        let mut remainder: u64 = 0;
        for chunk in full.as_bytes().chunks(9) {
            let s: String = chunk.iter().map(|&b| b as char).collect();
            let combined = format!("{}{}", remainder, s);
            remainder = combined.parse::<u64>().unwrap_or(0) % 97;
        }
        let check = 98 - remainder;
        format!("{:02}", check)
    }

    pub(crate) fn random_datetime(&mut self) -> NaiveDateTime {
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

    /// Generate a random datetime avoiding May (month 5) to meet minLength
    /// constraints in validation patterns for full month name formats.
    pub(crate) fn random_datetime_avoiding_may(&mut self) -> NaiveDateTime {
        let year = self.rng.gen_range(2015..2030);
        // Months 1-12 excluding 5 (May has only 3 chars)
        let months_no_may = [1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12];
        let month = months_no_may[self.rng.gen_range(0..months_no_may.len())];
        let day = self.rng.gen_range(1..=28);
        let hour = self.rng.gen_range(0..24);
        let minute = self.rng.gen_range(0..60);
        let second = self.rng.gen_range(0..60);
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    /// Generate a random datetime using only months with ≥6-char names
    /// (January, February, August, September, October, November, December)
    /// to meet minLength: 22 for weekday+full month formats.
    pub(crate) fn random_datetime_long_month(&mut self) -> NaiveDateTime {
        let year = self.rng.gen_range(2015..2030);
        // Months with name length >= 6: Jan(7), Feb(8), Aug(6), Sep(9), Oct(7), Nov(8), Dec(8)
        let long_months = [1, 2, 8, 9, 10, 11, 12];
        let month = long_months[self.rng.gen_range(0..long_months.len())];
        let day = self.rng.gen_range(1..=28);
        let hour = self.rng.gen_range(0..24);
        let minute = self.rng.gen_range(0..60);
        let second = self.rng.gen_range(0..60);
        NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, minute, second)
            .unwrap()
    }

    /// Convert Gregorian year to Japanese era letter and era year.
    /// Era offset table: era_year + offset = gregorian_year.
    /// R→2018, H→1988, S→1925, T→1911, M→1867.
    pub(crate) fn gregorian_to_jp_era(&self, year: i32) -> (&'static str, i32) {
        if year >= 2019 {
            ("R", year - 2018)
        } else if year >= 1989 {
            ("H", year - 1988)
        } else if year >= 1926 {
            ("S", year - 1925)
        } else if year >= 1912 {
            ("T", year - 1911)
        } else {
            ("M", year - 1867)
        }
    }

    /// Get full Japanese era name for a Gregorian year.
    pub(crate) fn jp_era_name(&self, year: i32) -> &'static str {
        if year >= 2019 {
            "令和"
        } else if year >= 1989 {
            "平成"
        } else if year >= 1926 {
            "昭和"
        } else if year >= 1912 {
            "大正"
        } else {
            "明治"
        }
    }

    /// Format an integer with a configurable thousands separator.
    pub(crate) fn format_int_with_separator(n: i64, sep: char) -> String {
        let s = n.abs().to_string();
        let mut result = String::new();
        for (i, ch) in s.chars().rev().enumerate() {
            if i > 0 && i % 3 == 0 {
                result.push(sep);
            }
            result.push(ch);
        }
        result.chars().rev().collect()
    }

    /// Format an integer with Indian lakh/crore grouping (XX,XX,XXX).
    pub(crate) fn format_indian_grouping(n: i64) -> String {
        let s = n.abs().to_string();
        if s.len() <= 3 {
            return s;
        }
        // Last 3 digits are grouped normally, then groups of 2
        let (prefix, last3) = s.split_at(s.len() - 3);
        let mut result = String::new();
        for (i, ch) in prefix.chars().rev().enumerate() {
            if i > 0 && i % 2 == 0 {
                result.push(',');
            }
            result.push(ch);
        }
        let prefix_formatted: String = result.chars().rev().collect();
        format!("{},{}", prefix_formatted, last3)
    }

    /// Generate a random monetary amount as (integer_part, cents).
    pub(crate) fn random_amount(&mut self) -> (i64, u32) {
        let r = self.rng.gen::<f64>();
        if r < 0.3 {
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
        }
    }

    pub(crate) fn gen_hex_string(&mut self, char_count: usize) -> String {
        (0..char_count / 2)
            .map(|_| format!("{:02x}", self.rng.gen::<u8>()))
            .collect()
    }

    pub(crate) fn random_word(&mut self) -> String {
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

    /// Generate a non-person entity name (company, product, venue, title).
    ///
    /// Produces diverse formats to help CharCNN distinguish entity names from
    /// personal names: business suffixes, numbers, ampersands, "The" prefix,
    /// multi-word titles, etc. 10 format variants with weighted probabilities.
    pub(crate) fn gen_entity_name(&mut self) -> Result<String, GeneratorError> {
        let company_suffixes = [
            "Inc",
            "Corp",
            "Ltd",
            "LLC",
            "Co",
            "Group",
            "Holdings",
            "Partners",
            "Associates",
            "Foundation",
            "Institute",
            "Solutions",
            "Systems",
            "Technologies",
            "Enterprises",
            "Industries",
            "Services",
        ];
        let entity_words = [
            "Global", "Pacific", "Atlantic", "Summit", "Apex", "Crown", "Prime", "Pinnacle",
            "Horizon", "Pioneer", "Quantum", "Stellar", "Nexus", "Titan", "Phoenix", "Eagle",
            "Crystal", "Diamond", "Royal", "Metro", "Grand", "Capital", "Liberty", "Heritage",
            "Emerald", "Silver", "Golden", "Dragon", "Legacy", "Harmony",
        ];
        let product_words = [
            "Pro", "Max", "Ultra", "Plus", "Elite", "Mini", "Air", "Edge", "Prime", "Neo", "Flex",
            "Core", "Studio", "Duo", "One",
        ];
        let venue_types = [
            "Restaurant",
            "Cafe",
            "Bar",
            "Grill",
            "Bistro",
            "Tavern",
            "Kitchen",
            "House",
            "Inn",
            "Hotel",
            "Lounge",
            "Club",
            "Theater",
            "Cinema",
            "Gallery",
            "Museum",
            "Park",
            "Arena",
            "Stadium",
            "Center",
        ];

        let choice = self.rng.gen_range(0..20);
        let name = if choice < 4 {
            // 20%: Company with suffix — "Apex Technologies Inc"
            let w1 = entity_words[self.rng.gen_range(0..entity_words.len())];
            let w2 = entity_words[self.rng.gen_range(0..entity_words.len())];
            let suffix = company_suffixes[self.rng.gen_range(0..company_suffixes.len())];
            if self.rng.gen_bool(0.5) {
                format!("{} {}", w1, suffix)
            } else {
                format!("{} {} {}", w1, w2, suffix)
            }
        } else if choice < 7 {
            // 15%: Product with version/number — "iPhone 15 Pro", "Model X3"
            let w = entity_words[self.rng.gen_range(0..entity_words.len())];
            let prod = product_words[self.rng.gen_range(0..product_words.len())];
            let num = self.rng.gen_range(1..99);
            if self.rng.gen_bool(0.5) {
                format!("{} {} {}", w, num, prod)
            } else {
                format!("{}{}", w, num)
            }
        } else if choice < 10 {
            // 15%: "The" prefix — "The Olive Garden", "The Grand Hotel"
            let w = entity_words[self.rng.gen_range(0..entity_words.len())];
            let vt = venue_types[self.rng.gen_range(0..venue_types.len())];
            format!("The {} {}", w, vt)
        } else if choice < 13 {
            // 15%: Venue-style — "Mario's Kitchen", "Summit Grill"
            let first = self.random_first_name();
            let vt = venue_types[self.rng.gen_range(0..venue_types.len())];
            if self.rng.gen_bool(0.4) {
                format!("{}'s {}", first, vt)
            } else {
                let w = entity_words[self.rng.gen_range(0..entity_words.len())];
                format!("{} {}", w, vt)
            }
        } else if choice < 15 {
            // 10%: Ampersand style — "Johnson & Johnson", "Barnes & Noble"
            let w1 = entity_words[self.rng.gen_range(0..entity_words.len())];
            let w2 = entity_words[self.rng.gen_range(0..entity_words.len())];
            format!("{} & {}", w1, w2)
        } else if choice < 17 {
            // 10%: University/School — "Harvard University", "MIT"
            let w = entity_words[self.rng.gen_range(0..entity_words.len())];
            let inst = ["University", "College", "Academy", "School"];
            format!("{} {}", w, inst[self.rng.gen_range(0..inst.len())])
        } else if choice < 19 {
            // 10%: Single brand name (capitalised word) — "Spotify", "Tesla"
            let brands = [
                "Spotify",
                "Tesla",
                "Netflix",
                "Amazon",
                "Microsoft",
                "Samsung",
                "Oracle",
                "Walmart",
                "Starbucks",
                "Boeing",
                "Airbus",
                "Siemens",
                "Toyota",
                "Honda",
                "Pepsi",
                "Nestle",
            ];
            brands[self.rng.gen_range(0..brands.len())].to_string()
        } else {
            // 5%: All-caps acronym/abbreviation — "NASA", "BMW", "HSBC"
            let acronyms = [
                "NASA", "BMW", "HSBC", "IBM", "AT&T", "UPS", "BBC", "CNN", "NHL", "UFC", "FIFA",
                "IKEA", "AMD", "HP", "SAP", "DHL",
            ];
            acronyms[self.rng.gen_range(0..acronyms.len())].to_string()
        };
        Ok(name)
    }

    pub(crate) fn random_first_name(&mut self) -> String {
        let names = locale_data::first_names(self.current_locale());
        names[self.rng.gen_range(0..names.len())].to_string()
    }

    pub(crate) fn random_last_name(&mut self) -> String {
        let names = locale_data::last_names(self.current_locale());
        names[self.rng.gen_range(0..names.len())].to_string()
    }

    /// Get the current locale, defaulting to "EN" if not set.
    pub(crate) fn current_locale(&self) -> &str {
        self.locale.as_deref().unwrap_or("EN")
    }

    /// Format a date using a CLDR-derived locale pattern.
    pub(crate) fn format_date_parts(
        pat: &locale_data::DateFormatPattern,
        day: u32,
        month: &str,
        year: i32,
    ) -> String {
        use locale_data::DateFieldOrder;
        match pat.order {
            DateFieldOrder::MonthDayYear => format!(
                "{}{}{}{}{}{}{}",
                month,
                pat.day_month_sep,
                day,
                pat.day_suffix,
                pat.month_year_sep,
                year,
                pat.year_suffix
            ),
            DateFieldOrder::DayMonthYear => format!(
                "{}{}{}{}{}{}{}",
                day,
                pat.day_suffix,
                pat.day_month_sep,
                month,
                pat.month_year_sep,
                year,
                pat.year_suffix
            ),
            DateFieldOrder::YearMonthDay => format!(
                "{}{}{}{}{}{}{}",
                year,
                pat.year_suffix,
                pat.month_year_sep,
                month,
                pat.day_month_sep,
                day,
                pat.day_suffix
            ),
            DateFieldOrder::YearDayMonth => format!(
                "{}{}{}{}{}{}{}",
                year,
                pat.year_suffix,
                pat.month_year_sep,
                day,
                pat.day_suffix,
                pat.day_month_sep,
                month
            ),
        }
    }

    /// Generate a phone number for the current locale.
    /// Produces NATIONAL, INTERNATIONAL, and E164 formats with realistic
    /// spacing and punctuation derived from Google's libphonenumber data.
    /// Covers 40+ country/region formats across all locales.
    pub(crate) fn gen_phone_number(&mut self) -> Result<String, GeneratorError> {
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
            "PT_BR" => {
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
            "ES_MX" => {
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
            "HI" => {
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
            "TH" => {
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
            "MY" => {
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
            "SG" => {
                // Singapore: AAAA AAAA
                let a = self.rng.gen_range(6000..9999);
                let b = self.rng.gen_range(1000..9999);
                Ok(match fmt {
                    PhoneFmt::National => format!("{:04} {:04}", a, b),
                    PhoneFmt::International => format!("+65 {:04} {:04}", a, b),
                    PhoneFmt::E164 => format!("+65{:04}{:04}", a, b),
                })
            }
            "PH" => {
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
            "ID" => {
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
            "TW" => {
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
            "NZ" => {
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
            "IE" => {
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
            "SE" => {
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
            "NO" => {
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
            "DK" => {
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
            "CH" => {
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
            "AT" => {
                // Austria: 0664 AAAAAA
                let p = [664, 676, 680, 699][self.rng.gen_range(0..4)];
                let a = self.rng.gen_range(100000..999999);
                Ok(match fmt {
                    PhoneFmt::National => format!("0{:03} {:06}", p, a),
                    PhoneFmt::International => format!("+43 {:03} {:06}", p, a),
                    PhoneFmt::E164 => format!("+43{:03}{:06}", p, a),
                })
            }
            "BE" => {
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
            "PT" => {
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
            "TR" => {
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
            "IL" => {
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
            "GR" => {
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
            "ZA" => {
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
            "NG" => {
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
            "ES_CL" => {
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
            "ES_CO" => {
                // Colombia: (AAA) AAAAAAA
                let a = self.rng.gen_range(300..399);
                let b = self.rng.gen_range(1000000..9999999);
                Ok(match fmt {
                    PhoneFmt::National => format!("({:03}) {:07}", a, b),
                    PhoneFmt::International => format!("+57 {:03} {:07}", a, b),
                    PhoneFmt::E164 => format!("+57{:03}{:07}", a, b),
                })
            }
            "ES_AR" => {
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
            "FI" => {
                // Finland: 041 AAAAAAA
                let p = [41, 44, 45, 50][self.rng.gen_range(0..4)];
                let a = self.rng.gen_range(1000000..9999999);
                Ok(match fmt {
                    PhoneFmt::National => format!("0{:02} {:07}", p, a),
                    PhoneFmt::International => format!("+358 {:02} {:07}", p, a),
                    PhoneFmt::E164 => format!("+358{:02}{:07}", p, a),
                })
            }
            "ES_PE" => {
                // Peru: 9XX XXX XXX
                let a = self.rng.gen_range(900..999);
                let b = self.rng.gen_range(100..999);
                let c = self.rng.gen_range(100..999);
                Ok(match fmt {
                    PhoneFmt::National => format!("{:03} {:03} {:03}", a, b, c),
                    PhoneFmt::International => {
                        format!("+51 {:03} {:03} {:03}", a, b, c)
                    }
                    PhoneFmt::E164 => format!("+51{:03}{:03}{:03}", a, b, c),
                })
            }
            "HU" => {
                // Hungary: 06 30 XXX XXXX
                let p = [20, 30, 31, 50, 70][self.rng.gen_range(0..5)];
                let a = self.rng.gen_range(100..999);
                let b = self.rng.gen_range(1000..9999);
                Ok(match fmt {
                    PhoneFmt::National => format!("06 {:02} {:03} {:04}", p, a, b),
                    PhoneFmt::International => {
                        format!("+36 {:02} {:03} {:04}", p, a, b)
                    }
                    PhoneFmt::E164 => format!("+36{:02}{:03}{:04}", p, a, b),
                })
            }
            "RO" => {
                // Romania: 07XX XXX XXX
                let a = self.rng.gen_range(20..89);
                let b = self.rng.gen_range(100..999);
                let c = self.rng.gen_range(100..999);
                Ok(match fmt {
                    PhoneFmt::National => format!("07{:02} {:03} {:03}", a, b, c),
                    PhoneFmt::International => {
                        format!("+40 7{:02} {:03} {:03}", a, b, c)
                    }
                    PhoneFmt::E164 => format!("+407{:02}{:03}{:03}", a, b, c),
                })
            }
            "CZ" => {
                // Czech Republic: 6XX XXX XXX
                let a = self.rng.gen_range(601..799);
                let b = self.rng.gen_range(100..999);
                let c = self.rng.gen_range(100..999);
                Ok(match fmt {
                    PhoneFmt::National => format!("{:03} {:03} {:03}", a, b, c),
                    PhoneFmt::International => {
                        format!("+420 {:03} {:03} {:03}", a, b, c)
                    }
                    PhoneFmt::E164 => format!("+420{:03}{:03}{:03}", a, b, c),
                })
            }
            _ => {
                // Generic international format for unlisted locales
                let cc = locale_data::phone_country_code(&locale);
                let a = self.rng.gen_range(1000..9999);
                let b = self.rng.gen_range(1000..9999);
                Ok(match fmt {
                    PhoneFmt::National => format!("{:04} {:04}", a, b),
                    PhoneFmt::International => format!("{} {:04} {:04}", cc, a, b),
                    PhoneFmt::E164 => {
                        format!("{}{:04}{:04}", cc.replace(' ', ""), a, b)
                    }
                })
            }
        }
    }

    /// Generate a full address with locale-specific formatting.
    /// Address format order varies by locale:
    /// - EN: {num} {street}, {city}, {state} {zip}
    /// - DE/NL/PL: {street} {num}, {plz} {city}
    /// - FR/ES/IT: {num} {street}, {code} {city}
    /// - JA/ZH/KO: large→small (prefecture/province → district → street → number)
    /// - RU: {city}, {street}, д. {num}
    pub(crate) fn gen_full_address(&mut self) -> Result<String, GeneratorError> {
        let locale = self.current_locale().to_string();
        let streets = locale_data::street_names(&locale);
        let cities = locale_data::cities(&locale);
        let regions = locale_data::states_or_regions(&locale);
        let street = streets[self.rng.gen_range(0..streets.len())];
        let city = cities[self.rng.gen_range(0..cities.len())];
        let num = self.rng.gen_range(1..999);

        match locale.as_str() {
            "EN_US" | "EN" => {
                let state = regions[self.rng.gen_range(0..regions.len())];
                let zip = format!("{:05}", self.rng.gen_range(10000..99999));
                if self.rng.gen_bool(0.3) {
                    // With apt/suite
                    let apt = self.rng.gen_range(1..999);
                    Ok(format!(
                        "{} {}, Apt {}, {}, {} {}",
                        num, street, apt, city, state, zip
                    ))
                } else {
                    Ok(format!("{} {}, {}, {} {}", num, street, city, state, zip))
                }
            }
            "EN_CA" => {
                let prov = regions[self.rng.gen_range(0..regions.len())];
                let pc = format!(
                    "{}{}{} {}{}{}",
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    self.rng.gen_range(1..9),
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    self.rng.gen_range(1..9),
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    self.rng.gen_range(1..9),
                );
                Ok(format!("{} {}, {}, {} {}", num, street, city, prov, pc))
            }
            "EN_GB" => {
                let pc = format!(
                    "{}{}{} {}{}{}",
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    self.rng.gen_range(1..9),
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    self.rng.gen_range(1..9),
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    (b'A' + self.rng.gen_range(0..26)) as char,
                );
                Ok(format!("{} {}, {}, {}", num, street, city, pc))
            }
            "EN_AU" => {
                let state = regions[self.rng.gen_range(0..regions.len())];
                let pc = format!("{:04}", self.rng.gen_range(2000..9999));
                Ok(format!("{} {}, {} {} {}", num, street, city, state, pc))
            }
            "DE" => {
                // German: Straße Hausnummer, PLZ Stadt
                let plz = format!("{:05}", self.rng.gen_range(10000..99999));
                Ok(format!("{} {}, {} {}", street, num, plz, city))
            }
            "FR" => {
                // French: numéro rue, code postal ville
                let cp = format!("{:05}", self.rng.gen_range(10000..99999));
                Ok(format!("{} {}, {} {}", num, street, cp, city))
            }
            "ES" => {
                // Spanish: Calle nombre número, CP ciudad
                let cp = format!("{:05}", self.rng.gen_range(10000..52999));
                Ok(format!("{} {}, {} {}", street, num, cp, city))
            }
            "IT" => {
                // Italian: Via nome numero, CAP città
                let cap = format!("{:05}", self.rng.gen_range(10000..99999));
                Ok(format!("{} {}, {} {}", street, num, cap, city))
            }
            "NL" => {
                // Dutch: Straat huisnummer, postcode stad
                let pc = format!(
                    "{:04} {}{}",
                    self.rng.gen_range(1000..9999),
                    (b'A' + self.rng.gen_range(0..26)) as char,
                    (b'A' + self.rng.gen_range(0..26)) as char,
                );
                Ok(format!("{} {}, {} {}", street, num, pc, city))
            }
            "PL" => {
                // Polish: ul. nazwa numer, kod miasto
                let code = format!(
                    "{:02}-{:03}",
                    self.rng.gen_range(10..99),
                    self.rng.gen_range(100..999)
                );
                Ok(format!("{} {}, {} {}", street, num, code, city))
            }
            "RU" => {
                // Russian: город, улица, д. номер
                let idx = format!("{:06}", self.rng.gen_range(100000..999999));
                Ok(format!("{}, {}, д. {}, {}", city, street, num, idx))
            }
            "JA" => {
                // Japanese: large→small (prefecture → city → district → chome-ban-go)
                let prefectures = locale_data::states_or_regions("JA");
                let districts = locale_data::districts("JA");
                let pref = prefectures[self.rng.gen_range(0..prefectures.len())];
                let dist = districts[self.rng.gen_range(0..districts.len())];
                let chome = self.rng.gen_range(1..9);
                let ban = self.rng.gen_range(1..30);
                let go = self.rng.gen_range(1..20);
                let pc = format!(
                    "{:03}-{:04}",
                    self.rng.gen_range(100..999),
                    self.rng.gen_range(1000..9999)
                );
                Ok(format!("〒{} {}{}{}-{}-{}", pc, pref, dist, chome, ban, go))
            }
            "ZH" => {
                // Chinese: large→small (province → city → district → street → number)
                let provinces = locale_data::states_or_regions("ZH");
                let districts = locale_data::districts("ZH");
                let prov = provinces[self.rng.gen_range(0..provinces.len())];
                let dist = districts[self.rng.gen_range(0..districts.len())];
                Ok(format!("{}{}{}{}{}号", prov, city, dist, street, num))
            }
            "KO" => {
                // Korean: city → district → street → number
                let districts = locale_data::districts("KO");
                let dist = districts[self.rng.gen_range(0..districts.len())];
                let pc = format!("{:05}", self.rng.gen_range(10000..99999));
                Ok(format!("({}) {} {} {} {}", pc, city, dist, street, num))
            }
            "AR" => {
                // Arabic: street number, city
                Ok(format!("{} {}، {}", street, num, city))
            }
            _ => {
                // Fallback US format
                let state = locale_data::states_or_regions("EN");
                let st = state[self.rng.gen_range(0..state.len())];
                let zip = format!("{:05}", self.rng.gen_range(10000..99999));
                Ok(format!("{} {}, {}, {} {}", num, street, city, st, zip))
            }
        }
    }

    /// Generate a postal code for the current locale.
    pub(crate) fn gen_postal_code(&mut self) -> Result<String, GeneratorError> {
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

            // 4-digit postal codes
            "4D" => Ok(format!("{:04}", self.rng.gen_range(1000..9999))),

            // 5-digit postal codes
            "5D" => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),

            // 6-digit postal codes
            "6D" => Ok(format!("{:06}", self.rng.gen_range(100000..999999))),

            // Czech/Slovak/Greek/Swedish: 3+2 digits with optional space
            "CS" => {
                let d3 = self.rng.gen_range(100..999);
                let d2 = self.rng.gen_range(0..100u32);
                if self.rng.gen_bool(0.5) {
                    Ok(format!("{:03} {:02}", d3, d2))
                } else {
                    Ok(format!("{:03}{:02}", d3, d2))
                }
            }

            // Portugal: XXXX-XXX
            "PT" => Ok(format!(
                "{:04}-{:03}",
                self.rng.gen_range(1000..9999),
                self.rng.gen_range(1..999)
            )),

            // Brazil: XXXXX-XXX (dash optional)
            "BR" => {
                let d5 = self.rng.gen_range(10000..99999);
                let d3 = self.rng.gen_range(1..999);
                if self.rng.gen_bool(0.7) {
                    Ok(format!("{:05}-{:03}", d5, d3))
                } else {
                    Ok(format!("{:05}{:03}", d5, d3))
                }
            }

            // Lithuania: optional LT- prefix + 5 digits
            "LT" => {
                let code = self.rng.gen_range(10000..99999);
                if self.rng.gen_bool(0.5) {
                    Ok(format!("LT-{:05}", code))
                } else {
                    Ok(format!("{:05}", code))
                }
            }

            // Latvia: LV-XXXX
            "LV" => Ok(format!("LV-{:04}", self.rng.gen_range(1000..9999))),

            // Argentina: optional [A-HJ-NP-Z] + 4 digits + optional [A-Z]{3}
            "ES_AR" => {
                const AR_FIRST: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
                let d4 = self.rng.gen_range(1000..9999);
                let variant = self.rng.gen_range(0u32..3);
                match variant {
                    0 => Ok(format!("{:04}", d4)),
                    1 => {
                        let l = AR_FIRST[self.rng.gen_range(0..AR_FIRST.len())] as char;
                        Ok(format!("{}{:04}", l, d4))
                    }
                    _ => {
                        let l1 = AR_FIRST[self.rng.gen_range(0..AR_FIRST.len())] as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l3 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l4 = (b'A' + self.rng.gen_range(0..26)) as char;
                        Ok(format!("{}{:04}{}{}{}", l1, d4, l2, l3, l4))
                    }
                }
            }

            // Chile: 7 digits
            "CL" => Ok(format!("{:07}", self.rng.gen_range(1000000..9999999))),

            // Peru: LIMA dd, CALLAO d, or [0-2]dddd
            "PE" => {
                let variant = self.rng.gen_range(0u32..3);
                match variant {
                    0 => Ok(format!("LIMA {}", self.rng.gen_range(1..100u32))),
                    1 => {
                        let d = self.rng.gen_range(0..10u32);
                        if self.rng.gen_bool(0.5) {
                            Ok(format!("CALLAO {}", d))
                        } else {
                            Ok(format!("CALLAO {:02}", d))
                        }
                    }
                    _ => {
                        let first = self.rng.gen_range(0..3u32);
                        let rest = self.rng.gen_range(0..10000u32);
                        Ok(format!("{}{:04}", first, rest))
                    }
                }
            }

            // Malta: [A-Z]{3} + optional space + 2-4 digits
            "MT" => {
                let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                let l3 = (b'A' + self.rng.gen_range(0..26)) as char;
                let num = match self.rng.gen_range(2u32..5) {
                    2 => format!("{:02}", self.rng.gen_range(10..99)),
                    3 => format!("{:03}", self.rng.gen_range(100..999)),
                    _ => format!("{:04}", self.rng.gen_range(1000..9999)),
                };
                if self.rng.gen_bool(0.7) {
                    Ok(format!("{}{}{} {}", l1, l2, l3, num))
                } else {
                    Ok(format!("{}{}{}{}", l1, l2, l3, num))
                }
            }

            // Ireland: Eircode [\dA-Z]{3} [\dA-Z]{4}
            "IE" => {
                const AN: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
                let len = AN.len();
                Ok(format!(
                    "{}{}{} {}{}{}{}",
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                    AN[self.rng.gen_range(0..len)] as char,
                ))
            }

            // Taiwan: 3, 5, or 6 digits
            "TW" => {
                let variant = self.rng.gen_range(0u32..3);
                match variant {
                    0 => Ok(format!("{:03}", self.rng.gen_range(100..999))),
                    1 => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),
                    _ => Ok(format!("{:06}", self.rng.gen_range(100000..999999))),
                }
            }

            // Israel: 5 or 7 digits
            "HE" => {
                if self.rng.gen_bool(0.5) {
                    Ok(format!("{:05}", self.rng.gen_range(10000..99999)))
                } else {
                    Ok(format!("{:07}", self.rng.gen_range(1000000..9999999)))
                }
            }

            // Iceland: 3 digits
            "IS" => Ok(format!("{:03}", self.rng.gen_range(100..999))),

            // Serbia: 5 or 6 digits
            "SR" => {
                if self.rng.gen_bool(0.5) {
                    Ok(format!("{:05}", self.rng.gen_range(10000..99999)))
                } else {
                    Ok(format!("{:06}", self.rng.gen_range(100000..999999)))
                }
            }

            // Vietnam: 5 or 6 digits
            "VI" => {
                if self.rng.gen_bool(0.5) {
                    Ok(format!("{:05}", self.rng.gen_range(10000..99999)))
                } else {
                    Ok(format!("{:06}", self.rng.gen_range(100000..999999)))
                }
            }

            _ => Ok(format!("{:05}", self.rng.gen_range(10000..99999))),
        }
    }
}
