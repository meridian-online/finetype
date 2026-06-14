//! Generators for the `representation` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: representation (19 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_representation(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── numeric (5 types) ────────────────────────────────────────
            ("numeric", "integer_number") => {
                // Wider range with varied magnitudes to distinguish from
                // postal codes, etc.
                let r = self.rng.gen::<f64>();
                let val = if r < 0.3 {
                    // Large numbers (thousands to millions)
                    self.rng.gen_range(1000i64..10_000_000)
                } else if r < 0.5 {
                    // Negative numbers (distinctive vs street/postal)
                    self.rng.gen_range(-100000i64..-1)
                } else if r < 0.7 {
                    // Medium range
                    self.rng.gen_range(-10000i64..10000)
                } else if r < 0.85 {
                    // Small positive (overlaps with others, but still needed)
                    self.rng.gen_range(0i64..1000)
                } else {
                    // Very large
                    self.rng.gen_range(100000i64..1_000_000_000)
                };
                Ok(val.to_string())
            }
            ("numeric", "decimal_number") => {
                let val = (self.rng.gen::<f64>() - 0.5) * 2000.0;
                let precision = self.rng.gen_range(1..8);
                Ok(format!("{:.prec$}", val, prec = precision))
            }
            ("numeric", "decimal_number_comma") => {
                // European format: period thousands separator, comma decimal separator
                let r = self.rng.gen::<f64>();
                let (integer_part, decimal_digits) = if r < 0.3 {
                    // Large with thousands: 1.234 to 9.999.999
                    (
                        self.rng.gen_range(1_000i64..10_000_000),
                        self.rng.gen_range(1..3),
                    )
                } else if r < 0.5 {
                    // Negative
                    (
                        -(self.rng.gen_range(1i64..100_000)),
                        self.rng.gen_range(1..3),
                    )
                } else if r < 0.7 {
                    // Small with decimals only
                    (self.rng.gen_range(0i64..999), self.rng.gen_range(1..4))
                } else {
                    // Medium
                    (
                        self.rng.gen_range(1i64..1_000_000),
                        self.rng.gen_range(1..3),
                    )
                };
                let is_negative = integer_part < 0;
                let abs_int = integer_part.unsigned_abs();
                // Format integer part with period thousands separators
                let int_str = abs_int.to_string();
                let mut with_sep = String::new();
                for (i, ch) in int_str.chars().rev().enumerate() {
                    if i > 0 && i % 3 == 0 {
                        with_sep.push('.');
                    }
                    with_sep.push(ch);
                }
                let formatted_int: String = with_sep.chars().rev().collect();
                // Generate decimal part
                let decimal_val = self.rng.gen_range(0..10u32.pow(decimal_digits as u32));
                let decimal_str = format!("{:0>width$}", decimal_val, width = decimal_digits);
                let result = if is_negative {
                    format!("-{},{}", formatted_int, decimal_str)
                } else {
                    format!("{},{}", formatted_int, decimal_str)
                };
                Ok(result)
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
            // increment moved to representation.identifier
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
                    Ok(format!(
                        "{}{:.prec$}{}",
                        prefix,
                        value,
                        suffix,
                        prec = precision
                    ))
                }
            }

            // ── text (8 types) ───────────────────────────────────────────
            ("text", "plain_text") => {
                let words: Vec<String> = (0..self.rng.gen_range(5..25))
                    .map(|_| self.random_word())
                    .collect();
                Ok(words.join(" "))
            }
            ("text", "word") => Ok(self.random_word()),
            ("text", "entity_name") => self.gen_entity_name(),
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

            // ── format (2 types) ─────────────────────────────────────────
            ("format", "color_hex") => {
                let r = self.rng.gen::<u8>();
                let g = self.rng.gen::<u8>();
                let b = self.rng.gen::<u8>();
                if self.rng.gen_bool(0.8) {
                    Ok(format!("#{:02X}{:02X}{:02X}", r, g, b))
                } else {
                    Ok(format!("{:02x}{:02x}{:02x}", r, g, b))
                }
            }
            ("format", "color_rgb") => {
                let r = self.rng.gen_range(0..256);
                let g = self.rng.gen_range(0..256);
                let b = self.rng.gen_range(0..256);
                if self.rng.gen_bool(0.6) {
                    Ok(format!("rgb({}, {}, {})", r, g, b))
                } else {
                    Ok(format!("{}, {}, {}", r, g, b))
                }
            }
            ("format", "color_hsl") => {
                let h = self.rng.gen_range(0..361);
                let s = self.rng.gen_range(0..101);
                let l = self.rng.gen_range(0..101);
                if self.rng.gen_bool(0.3) {
                    let a = self.rng.gen_range(1..10) as f32 / 10.0;
                    Ok(format!("hsla({}, {}%, {}%, {})", h, s, l, a))
                } else {
                    Ok(format!("hsl({}, {}%, {}%)", h, s, l))
                }
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
                // Excel custom number format strings.
                //
                // v17 ac-02 improvements:
                //   - Expanded token grammar (# 0 , . ; [ ] $ € £ ¥ % E e
                //     A/a P/p M/m / d D y Y h H s S). Locale codes
                //     [$-409] / [$-40C] / [$-C09] emitted with realistic
                //     bracket syntax.
                //   - Multi-section syntax: positive;negative;zero;text.
                //   - Date/time variants (ISO, US, EU, long-form) and
                //     AM/PM / 24-hour variants.
                //   - Colour conditionals ([Red], [Green], [Blue],
                //     [Magenta], [Cyan], [Yellow]) and threshold
                //     conditionals ([>N], [<N], [=N]).
                //   - Text / literal suffixes (e.g. `0"kg"`, `@`).
                //
                // Character set stays within the YAML validation pattern.
                let decimals_s = |n: usize| {
                    if n == 0 {
                        String::new()
                    } else {
                        format!(".{}", "0".repeat(n))
                    }
                };
                // Weighted branch selection — favour the high-cardinality
                // branches (currency/dates/multi-section/thresholds) so the
                // generator saturates >500 unique values quickly.
                let weights = [3u32, 10, 3, 10, 8, 10, 4, 3, 12, 12, 8, 6];
                let total: u32 = weights.iter().sum();
                let mut pick = self.rng.gen_range(0..total);
                let mut choice = 0usize;
                for (i, w) in weights.iter().enumerate() {
                    if pick < *w {
                        choice = i;
                        break;
                    }
                    pick -= *w;
                }
                let fmt = match choice {
                    // ── Plain number formats (grouped / ungrouped) ─────────
                    0 => {
                        let decimals = self.rng.gen_range(0..6);
                        let body = if self.rng.gen_bool(0.6) { "#,##0" } else { "0" };
                        format!("{}{}", body, decimals_s(decimals))
                    }
                    // ── Currency formats (symbol or locale-prefixed) ───────
                    1 => {
                        let decimals = self.rng.gen_range(0..3);
                        let grouped = if self.rng.gen_bool(0.85) {
                            "#,##0"
                        } else {
                            "0"
                        };
                        if self.rng.gen_bool(0.35) {
                            // Locale-prefixed, e.g. [$-409] for en-US.
                            let locales = [
                                "[$-409]", "[$-809]", "[$-40C]", "[$-407]", "[$-C09]", "[$-410]",
                                "[$-804]", "[$-411]", "[$-419]", "[$-C0A]", "[$-416]", "[$-413]",
                                "[$-40A]", "[$-41D]",
                            ];
                            let loc = locales[self.rng.gen_range(0..locales.len())];
                            let sym = ["$", "€", "£", "¥"][self.rng.gen_range(0..4)];
                            format!("{}{}{}{}", loc, sym, grouped, decimals_s(decimals))
                        } else {
                            let sym = ["$", "€", "£", "¥"][self.rng.gen_range(0..4)];
                            // Trailing vs leading symbol variants.
                            if self.rng.gen_bool(0.8) {
                                format!("{}{}{}", sym, grouped, decimals_s(decimals))
                            } else {
                                format!("{}{} {}", grouped, decimals_s(decimals), sym)
                            }
                        }
                    }
                    // ── Percentage formats ─────────────────────────────────
                    2 => {
                        let decimals = self.rng.gen_range(0..5);
                        let body = if self.rng.gen_bool(0.5) { "0" } else { "#,##0" };
                        format!("{}{}%", body, decimals_s(decimals))
                    }
                    // ── Date formats (ISO, US, EU, mixed) ──────────────────
                    3 => {
                        let fmts = [
                            "m/d/yyyy",
                            "mm/dd/yyyy",
                            "mm/dd/yy",
                            "m/d/yy",
                            "d/m/yyyy",
                            "dd/mm/yyyy",
                            "dd/mm/yy",
                            "yyyy-mm-dd",
                            "yy-mm-dd",
                            "yyyy/mm/dd",
                            "yyyymmdd",
                            "d-mmm",
                            "d-mmm-yy",
                            "d-mmm-yyyy",
                            "dd-mmm-yyyy",
                            "mmm-yy",
                            "mmmm-yy",
                            "mmmm d, yyyy",
                            "mmm d, yyyy",
                            "dddd, mmmm d, yyyy",
                            "ddd, mmm d yyyy",
                            "mm-dd-yyyy",
                            "m.d.yyyy",
                            "d.m.yyyy",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // ── Time formats ───────────────────────────────────────
                    4 => {
                        let fmts = [
                            "h:mm AM/PM",
                            "h:mm:ss AM/PM",
                            "hh:mm AM/PM",
                            "h:mm",
                            "h:mm:ss",
                            "hh:mm",
                            "hh:mm:ss",
                            "mm:ss",
                            "mm:ss.0",
                            "h:mm:ss.00",
                            "[h]:mm",
                            "[h]:mm:ss",
                            "[mm]:ss",
                            "[ss]",
                            "h:mm:ss.000",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // ── Combined date + time ───────────────────────────────
                    5 => {
                        let dates = ["m/d/yyyy", "yyyy-mm-dd", "d-mmm-yy", "dd/mm/yyyy"];
                        let times = ["h:mm", "h:mm:ss", "h:mm AM/PM", "hh:mm:ss"];
                        let d = dates[self.rng.gen_range(0..dates.len())];
                        let t = times[self.rng.gen_range(0..times.len())];
                        format!("{} {}", d, t)
                    }
                    // ── Scientific notation ────────────────────────────────
                    6 => {
                        let decimals = self.rng.gen_range(1..6);
                        let exp_digits = self.rng.gen_range(1..=3);
                        let e_char = if self.rng.gen_bool(0.5) { "E" } else { "e" };
                        let sign = if self.rng.gen_bool(0.7) { "+" } else { "-" };
                        format!(
                            "0.{}{}{}{}",
                            "0".repeat(decimals),
                            e_char,
                            sign,
                            "0".repeat(exp_digits)
                        )
                    }
                    // ── Fractions ──────────────────────────────────────────
                    7 => {
                        let fmts = [
                            "# ?/?",
                            "# ??/??",
                            "# ???/???",
                            "# ?/2",
                            "# ?/4",
                            "# ?/8",
                            "# ?/16",
                            "# ?/10",
                            "# ?/100",
                        ];
                        fmts[self.rng.gen_range(0..fmts.len())].to_string()
                    }
                    // ── Multi-section (positive;negative;zero;text) ────────
                    8 => {
                        let decimals = self.rng.gen_range(0..4);
                        let grouped = if self.rng.gen_bool(0.7) { "#,##0" } else { "0" };
                        let body = format!("{}{}", grouped, decimals_s(decimals));
                        let colours = [
                            "[Red]",
                            "[Green]",
                            "[Blue]",
                            "[Magenta]",
                            "[Cyan]",
                            "[Yellow]",
                        ];
                        let colour = colours[self.rng.gen_range(0..colours.len())];
                        let variant = self.rng.gen_range(0..8);
                        match variant {
                            0 => format!("{};-{}", body, body),
                            1 => format!("{};({})", body, body),
                            2 => format!("{};{}-{}", body, colour, body),
                            3 => format!("{};{}({})", body, colour, body),
                            4 => format!("{};-{};0", body, body),
                            5 => format!("{};-{};0;@", body, body),
                            6 => format!("{};{}-{};0;@", body, colour, body),
                            _ => format!("{};{}({});\"-\";@", body, colour, body),
                        }
                    }
                    // ── Threshold conditionals ─────────────────────────────
                    9 => {
                        // Pick a round threshold (10, 100, 1000, 10000, etc.).
                        let exp = self.rng.gen_range(1..=5u32);
                        let threshold = 10u32.pow(exp);
                        let op = [">", ">=", "<", "<=", "="][self.rng.gen_range(0..5)];
                        let decimals = self.rng.gen_range(0..3);
                        let grouped = if self.rng.gen_bool(0.6) { "#,##0" } else { "0" };
                        let body = format!("{}{}", grouped, decimals_s(decimals));
                        let colours = ["[Red]", "[Green]", "[Blue]", "[Magenta]", "[Cyan]"];
                        let colour = colours[self.rng.gen_range(0..colours.len())];
                        if self.rng.gen_bool(0.5) {
                            format!("[{}{}]{};{}{}", op, threshold, body, colour, body)
                        } else {
                            format!("[{}{}]{}{};{}", op, threshold, colour, body, body)
                        }
                    }
                    // ── Literal-text suffix / prefix units ─────────────────
                    10 => {
                        let units = [
                            "kg", "g", "mg", "m", "cm", "mm", "km", "lb", "oz", "bp", "pts",
                            "units", "pcs", "ea", "hrs", "min", "sec", "days", "items", "%",
                        ];
                        let unit = units[self.rng.gen_range(0..units.len())];
                        let decimals = self.rng.gen_range(0..4);
                        let grouped = if self.rng.gen_bool(0.5) { "#,##0" } else { "0" };
                        let body = format!("{}{}", grouped, decimals_s(decimals));
                        if self.rng.gen_bool(0.7) {
                            format!("{}\" {}\"", body, unit)
                        } else {
                            format!("\"{}: \"{}", unit, body)
                        }
                    }
                    // ── Text placeholder / pass-through / canned fixtures ──
                    _ => {
                        let fmts = [
                            "@",
                            "\"Status: \"@",
                            "\"ID-\"@",
                            "[$USD] #,##0.00",
                            "[$GBP] #,##0.00",
                            "[$EUR] #,##0.00",
                            "[$JPY] #,##0",
                            "0.00;[Red]-0.00",
                            "#,##0;(#,##0);\"--\"",
                            "#,##0,\"K\"",
                            "#,##0,,\"M\"",
                            "#,##0,,,\"B\"",
                            "0.00\"E-03\"",
                            "General",
                            "Text",
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
            ("scientific", "cas_number") => {
                // CAS: 2-7 digits, hyphen, 2 digits, hyphen, 1 check digit
                let part1_len = self.rng.gen_range(2..=7);
                let part1: String = (0..part1_len)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                let part2: String = (0..2)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                let all_digits: Vec<u32> = format!("{}{}", part1, part2)
                    .bytes()
                    .map(|b| (b - b'0') as u32)
                    .collect();
                let check: u32 = all_digits
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(i, &d)| d * (i as u32 + 1))
                    .sum();
                Ok(format!("{}-{}-{}", part1, part2, check % 10))
            }
            ("scientific", "inchi") => {
                let molecules = [
                    "InChI=1S/H2O/h1H2",
                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                    "InChI=1S/CH4/h1H4",
                    "InChI=1S/C6H6/c1-2-4-6-5-3-1/h1-6H",
                    "InChI=1S/CO2/c2-1-3",
                    "InChI=1S/NaCl/c1-2",
                    "InChI=1S/C2H4O2/c1-2(3)4/h1H3,(H,3,4)",
                    "InChI=1S/C6H12O6/c7-1-2-3(8)4(9)5(10)6(11)12-2/h2-11H,1H2",
                    "InChI=1S/C3H8O/c1-2-3-4/h4H,2-3H2,1H3",
                    "InChI=1S/NH3/h1H3",
                    "InChI=1S/C8H10N4O2/c1-10-4-9-6-5(10)7(13)12(2)8(14)11(6)3",
                    "InChI=1S/C9H8O4/c1-6(10)13-8-5-3-2-4-7(8)9(11)12/h2-5H,1H3,(H,11,12)",
                ];
                Ok(molecules[self.rng.gen_range(0..molecules.len())].to_string())
            }
            ("scientific", "smiles") => {
                let molecules = [
                    "O",
                    "CCO",
                    "CC",
                    "c1ccccc1",
                    "CC(=O)O",
                    "CC(=O)Oc1ccccc1C(=O)O",
                    "C(=O)(N)N",
                    "OC(=O)C(O)C(O)C(O)C(O)CO",
                    "CC(C)CC1=CC=C(C=C1)C(C)C(=O)O",
                    "C1CCCCC1",
                    "CC(=O)NC1=CC=C(C=C1)O",
                    "C(C(=O)O)N",
                    "c1cc(ccc1O)O",
                    "CCCCCCCCCCCCCCCC(=O)O",
                    "CC(O)=O",
                    "N#N",
                    "O=C=O",
                    "[Na+].[Cl-]",
                ];
                Ok(molecules[self.rng.gen_range(0..molecules.len())].to_string())
            }
            ("scientific", "measurement_unit") => {
                let units = [
                    "meter", "kilogram", "second", "ampere", "kelvin", "mole", "candela", "hertz",
                    "newton", "joule", "watt", "pascal", "liter", "gram", "m", "kg", "s", "A", "K",
                    "mol", "cd", "Hz", "N", "J", "W", "Pa", "L", "g",
                ];
                Ok(units[self.rng.gen_range(0..units.len())].to_string())
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

            // ── identifier (4 types) ─────────────────────────────────────
            ("identifier", "uuid") => Ok(Uuid::new_v4().to_string()),
            ("identifier", "increment") => Ok(self.rng.gen_range(1..100000).to_string()),
            ("identifier", "numeric_code") => {
                // All-digit codes with consistent length and leading zeros.
                // Mix of real-world code patterns to train the model.
                let r = self.rng.gen::<f64>();
                if r < 0.25 {
                    // ISO 3166-1 numeric country codes (3-digit, leading zeros)
                    let codes = [
                        "004", "008", "012", "016", "020", "024", "028", "031", "032", "036",
                        "040", "044", "048", "050", "051", "052", "056", "060", "064", "068",
                        "070", "072", "076", "084", "090", "096", "100", "104", "108", "112",
                        "116", "120", "124", "132", "140", "144", "148", "152", "156", "170",
                        "174", "178", "180", "188", "191", "192", "196", "203", "204", "208",
                        "214", "218", "222", "226", "231", "232", "233", "242", "246", "250",
                        "258", "262", "266", "268", "270", "276", "288", "296", "300", "308",
                        "320", "324", "328", "332", "340", "344", "348", "352", "356", "360",
                        "364", "368", "372", "376", "380", "384", "388", "392", "398", "400",
                        "404", "408", "410", "414", "417", "418", "422", "426", "428", "430",
                        "434", "440", "442", "450", "454", "458", "462", "466", "470", "478",
                        "480", "484", "492", "496", "498", "500", "504", "508", "512", "516",
                        "520", "524", "528", "540", "548", "554", "558", "562", "566", "578",
                        "580", "583", "584", "585", "586", "591", "598", "600", "604", "608",
                        "616", "620", "624", "626", "630", "634", "642", "643", "646", "659",
                        "660", "662", "670", "674", "678", "682", "686", "688", "690", "694",
                        "702", "703", "704", "706", "710", "716", "724", "728", "729", "732",
                        "740", "748", "752", "756", "760", "762", "764", "768", "776", "780",
                        "784", "788", "792", "795", "798", "800", "804", "807", "818", "826",
                        "831", "832", "833", "834", "840", "854", "858", "860", "862", "882",
                        "887", "894",
                    ];
                    Ok(codes[self.rng.gen_range(0..codes.len())].to_string())
                } else if r < 0.50 {
                    // NAICS codes (2-6 digits, no leading zeros typically but consistent length)
                    let naics = [
                        "11", "21", "22", "23", "31", "32", "33", "42", "44", "45", "48", "49",
                        "51", "52", "53", "54", "55", "56", "61", "62", "71", "72", "81", "92",
                        "111", "112", "113", "114", "115", "211", "212", "213", "221", "236",
                        "237", "238", "311", "312", "313", "314", "315", "316", "321", "322",
                        "323", "324", "325", "326", "327", "331", "332", "333", "334", "335",
                        "336", "337", "339", "1111", "1112", "1113", "1114", "1119", "1121",
                        "1122", "1123", "1124", "1125", "1129", "1131", "1132", "1133", "1141",
                        "1142", "1151", "1152", "1153", "2111", "2121", "2122", "2123", "2131",
                        "2211", "2212", "2213", "51111", "51112", "51113", "51114", "51119",
                        "51121", "51211", "51213", "51219", "51911",
                    ];
                    Ok(naics[self.rng.gen_range(0..naics.len())].to_string())
                } else if r < 0.75 {
                    // FIPS codes (5-digit state+county, leading zeros common)
                    let state = self.rng.gen_range(1u32..56);
                    let county = self.rng.gen_range(1u32..999);
                    Ok(format!("{:02}{:03}", state, county))
                } else {
                    // Product/category codes (4-digit with leading zeros)
                    Ok(format!("{:04}", self.rng.gen_range(1u32..9999)))
                }
            }
            ("identifier", "alphanumeric_id") => {
                // Generate mixed letter+digit identifier patterns.
                // v14 AC-02(e): Added prefix+code patterns (11-12) for earthquake IDs etc.
                let pattern_idx = self.rng.gen_range(0..13);
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
                    9 => {
                        let l1 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l2 = (b'A' + self.rng.gen_range(0..26)) as char;
                        let l3 = (b'a' + self.rng.gen_range(0..26)) as char;
                        let num = self.rng.gen_range(10000..99999);
                        Ok(format!("{}{}{}{}", l1, l2, l3, num))
                    }
                    // v14 AC-02(e): prefix+code patterns (e.g., "us6000pgkh" earthquake IDs)
                    10 => {
                        let prefixes = ["us", "nc", "ci", "ak", "hv", "pr", "nn"];
                        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                        let num = self.rng.gen_range(1000..99999);
                        let suffix: String = (0..4)
                            .map(|_| (b'a' + self.rng.gen_range(0..26)) as char)
                            .collect();
                        Ok(format!("{}{}{}", prefix, num, suffix))
                    }
                    // v14 AC-02(e): alphanumeric reference codes (e.g., "A1B2C3D4")
                    11 => {
                        let len = self.rng.gen_range(6..12);
                        let code: String = (0..len)
                            .map(|i| {
                                if i % 2 == 0 {
                                    (b'A' + self.rng.gen_range(0..26)) as char
                                } else {
                                    (b'0' + self.rng.gen_range(0..10)) as char
                                }
                            })
                            .collect();
                        Ok(code)
                    }
                    // v14 AC-02(e): versioned identifiers (e.g., "v2.3.1", "r1234")
                    _ => {
                        let prefixes = ["v", "r", "b", "p"];
                        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                        let num = self.rng.gen_range(1..9999);
                        if self.rng.gen_bool(0.5) {
                            let minor = self.rng.gen_range(0..20);
                            let patch = self.rng.gen_range(0..50);
                            Ok(format!("{}{}.{}.{}", prefix, num, minor, patch))
                        } else {
                            Ok(format!("{}{}", prefix, num))
                        }
                    }
                }
            }

            // ── boolean (3 types) ─────────────────────────────────────────
            // Split from single boolean into format-specific subtypes
            ("boolean", "binary") => {
                let vals = ["0", "1"];
                Ok(vals[self.rng.gen_range(0..vals.len())].to_string())
            }
            ("boolean", "initials") => {
                let vals = ["T", "F", "t", "f", "Y", "N", "y", "n"];
                Ok(vals[self.rng.gen_range(0..vals.len())].to_string())
            }
            ("boolean", "terms") => {
                let vals = [
                    "true", "false", "True", "False", "TRUE", "FALSE", "yes", "no", "Yes", "No",
                    "YES", "NO", "on", "off", "On", "Off", "ON", "OFF", "enabled", "disabled",
                    "Enabled", "Disabled", "active", "inactive", "Active", "Inactive",
                ];
                Ok(vals[self.rng.gen_range(0..vals.len())].to_string())
            }

            _ => Err(GeneratorError::NotImplemented(format!(
                "representation.{}.{}",
                category, type_name
            ))),
        }
    }
}
