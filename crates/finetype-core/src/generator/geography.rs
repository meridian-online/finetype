//! Generators for the `geography` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: geography (26 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_geography(
        &mut self,
        category: &str,
        type_name: &str,
    ) -> Result<String, GeneratorError> {
        match (category, type_name) {
            // ── location (6 types) ───────────────────────────────────────
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
            ("location", "state_code") => {
                // 2-letter state/province abbreviation
                let locale = self.current_locale();
                let codes: &[&str] = match locale {
                    "EN_CA" => &[
                        "AB", "BC", "MB", "NB", "NL", "NS", "NT", "NU", "ON", "PE", "QC", "SK",
                        "YT",
                    ],
                    "EN_AU" => &["NSW", "VIC", "QLD", "SA", "WA", "TAS", "NT", "ACT"],
                    _ => &[
                        // US states + DC + territories (default)
                        "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID",
                        "IL", "IN", "IA", "KS", "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS",
                        "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY", "NC", "ND", "OH", "OK",
                        "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
                        "WI", "WY", "DC", "AS", "GU", "MP", "PR", "VI",
                    ],
                };
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

            // ── address (4 types) ────────────────────────────────────────
            ("address", "full_address") => self.gen_full_address(),
            ("address", "street_name") => {
                let names = locale_data::street_names(self.current_locale());
                Ok(names[self.rng.gen_range(0..names.len())].to_string())
            }
            ("address", "street_suffix") => {
                let suffixes = locale_data::street_suffixes(self.current_locale());
                Ok(suffixes[self.rng.gen_range(0..suffixes.len())].to_string())
            }
            ("address", "postal_code") => self.gen_postal_code(),

            // ── format (2 types) ─────────────────────────────────────────
            ("format", "wkt") => {
                let r = self.rng.gen::<f64>();
                if r < 0.4 {
                    let x = self.rng.gen_range(-180.0f64..180.0);
                    let y = self.rng.gen_range(-90.0f64..90.0);
                    Ok(format!("POINT ({:.4} {:.4})", x, y))
                } else if r < 0.6 {
                    let pts: Vec<String> = (0..self.rng.gen_range(2..5))
                        .map(|_| {
                            let x = self.rng.gen_range(-180.0f64..180.0);
                            let y = self.rng.gen_range(-90.0f64..90.0);
                            format!("{:.2} {:.2}", x, y)
                        })
                        .collect();
                    Ok(format!("LINESTRING ({})", pts.join(", ")))
                } else if r < 0.85 {
                    let n = self.rng.gen_range(4..7);
                    let mut pts: Vec<String> = (0..n)
                        .map(|_| {
                            let x = self.rng.gen_range(-180.0f64..180.0);
                            let y = self.rng.gen_range(-90.0f64..90.0);
                            format!("{:.2} {:.2}", x, y)
                        })
                        .collect();
                    // Close the ring
                    pts.push(pts[0].clone());
                    Ok(format!("POLYGON (({}))", pts.join(", ")))
                } else if r < 0.95 {
                    let pts: Vec<String> = (0..self.rng.gen_range(2..5))
                        .map(|_| {
                            let x = self.rng.gen_range(-180.0f64..180.0);
                            let y = self.rng.gen_range(-90.0f64..90.0);
                            format!("({:.2} {:.2})", x, y)
                        })
                        .collect();
                    Ok(format!("MULTIPOINT ({})", pts.join(", ")))
                } else {
                    Ok("POINT EMPTY".to_string())
                }
            }
            // ── index (1 type) ───────────────────────────────────────────
            ("index", "h3") => {
                // H3 index: 15 hex characters. Use realistic prefixes.
                let prefixes = ["89283082", "891f1d4c", "8a2a1072", "87283082", "8e283082"];
                let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
                let suffix_len = 15 - prefix.len();
                let suffix: String = (0..suffix_len)
                    .map(|_| {
                        let hex_chars = b"0123456789abcdef";
                        hex_chars[self.rng.gen_range(0..16)] as char
                    })
                    .collect();
                Ok(format!("{}{}", prefix, suffix))
            }

            // ── coordinate (7 types) ─────────────────────────────────────
            ("coordinate", "latitude") => {
                // Vary precision 2–8 decimal places to distinguish from fixed-precision decimals
                let precision = self.rng.gen_range(2..=8usize);
                let lat = if self.rng.gen_bool(0.3) {
                    // Cluster around major city latitudes for realistic distribution
                    let city_lats: [f64; 12] = [
                        -33.87, -23.55, -34.60,
                        -41.29, // Sydney, São Paulo, Buenos Aires, Wellington
                        51.51, 48.85, 52.52, 59.91, // London, Paris, Berlin, Oslo
                        35.69, 1.35, 37.77,
                        19.43, // Tokyo, Singapore, San Francisco, Mexico City
                    ];
                    let base = city_lats[self.rng.gen_range(0..city_lats.len())];
                    let jitter: f64 = (self.rng.gen::<f64>() - 0.5) * 4.0;
                    (base + jitter).clamp(-90.0, 90.0)
                } else {
                    (self.rng.gen::<f64>() - 0.5) * 180.0
                };
                Ok(format!("{:.prec$}", lat, prec = precision))
            }
            ("coordinate", "longitude") => {
                // Vary precision 2–8 decimal places; range -180 to 180
                let precision = self.rng.gen_range(2..=8usize);
                let lon = if self.rng.gen_bool(0.3) {
                    // Cluster around major city longitudes for realistic distribution
                    let city_lons: [f64; 12] = [
                        151.21, -46.63, -58.38,
                        174.76, // Sydney, São Paulo, Buenos Aires, Wellington
                        -0.13, 2.35, 13.40, 10.75, // London, Paris, Berlin, Oslo
                        139.69, 103.82, -122.42,
                        -99.13, // Tokyo, Singapore, San Francisco, Mexico City
                    ];
                    let base = city_lons[self.rng.gen_range(0..city_lons.len())];
                    let jitter: f64 = (self.rng.gen::<f64>() - 0.5) * 4.0;
                    (base + jitter).clamp(-180.0, 180.0)
                } else {
                    (self.rng.gen::<f64>() - 0.5) * 360.0
                };
                Ok(format!("{:.prec$}", lon, prec = precision))
            }
            ("coordinate", "coordinates") => {
                let lat = (self.rng.gen::<f64>() - 0.5) * 180.0;
                let lon = (self.rng.gen::<f64>() - 0.5) * 360.0;
                Ok(format!("{:.4},{:.4}", lat, lon))
            }
            ("coordinate", "geohash") => {
                let alphabet = b"0123456789bcdefghjkmnpqrstuvwxyz";
                let len = self.rng.gen_range(6..=12);
                let hash: String = (0..len)
                    .map(|_| alphabet[self.rng.gen_range(0..alphabet.len())] as char)
                    .collect();
                Ok(hash)
            }
            ("coordinate", "plus_code") => {
                let alphabet = b"23456789CFGHJMPQRVWX";
                let first8: String = (0..8)
                    .map(|_| alphabet[self.rng.gen_range(0..alphabet.len())] as char)
                    .collect();
                let refine_len = self.rng.gen_range(2..=4);
                let refine: String = (0..refine_len)
                    .map(|_| alphabet[self.rng.gen_range(0..alphabet.len())] as char)
                    .collect();
                Ok(format!("{}+{}", first8, refine))
            }
            ("coordinate", "dms") => {
                let lat_d = self.rng.gen_range(0..90);
                let lat_m = self.rng.gen_range(0..60);
                let lat_s = self.rng.gen_range(0..60);
                let lat_dir = if self.rng.gen_bool(0.5) { 'N' } else { 'S' };
                let lon_d = self.rng.gen_range(0..180);
                let lon_m = self.rng.gen_range(0..60);
                let lon_s = self.rng.gen_range(0..60);
                let lon_dir = if self.rng.gen_bool(0.5) { 'E' } else { 'W' };
                Ok(format!(
                    "{}°{}'{}\"{} {}°{}'{}\"{}",
                    lat_d, lat_m, lat_s, lat_dir, lon_d, lon_m, lon_s, lon_dir
                ))
            }
            ("coordinate", "mgrs") => {
                let zone = self.rng.gen_range(1..=60);
                let bands = b"CDEFGHJKLMNPQRSTUVWX";
                let band = bands[self.rng.gen_range(0..bands.len())] as char;
                let sq_chars = b"ABCDEFGHJKLMNPQRSTUVWXYZ"; // excludes I and O
                let sq1 = sq_chars[self.rng.gen_range(0..sq_chars.len())] as char;
                let sq2 = sq_chars[self.rng.gen_range(0..sq_chars.len())] as char;
                // Even number of digits (2, 4, 6, 8, or 10)
                let precision = self.rng.gen_range(1..=5) * 2;
                let digits: String = (0..precision)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                Ok(format!("{}{}{}{}{}", zone, band, sq1, sq2, digits))
            }

            // ── transportation (5 types) ─────────────────────────────────
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
            ("transportation", "iso6346") => {
                // Owner code: 3 uppercase letters
                let owner: String = (0..3)
                    .map(|_| (b'A' + self.rng.gen_range(0..26)) as char)
                    .collect();
                // Equipment category: U, J, or Z
                let cats = ['U', 'J', 'Z'];
                let cat = cats[self.rng.gen_range(0..cats.len())];
                // Serial + check digit: 7 digits
                let serial: String = (0..7)
                    .map(|_| (b'0' + self.rng.gen_range(0..10)) as char)
                    .collect();
                Ok(format!("{}{}{}", owner, cat, serial))
            }
            ("transportation", "hs_code") => {
                // HS chapter: 01–97 (valid WCO chapters; 98-99 are country-specific)
                // Heading = chapter (2 digits) * 100 + sub-heading (2 digits)
                let chapter = self.rng.gen_range(1u32..=97);
                let sub_heading = self.rng.gen_range(0u32..100);
                let heading = chapter * 100 + sub_heading;
                let b = self.rng.gen_range(0u32..100); // HS 6-digit subheading suffix
                let r = self.rng.gen::<f64>();
                if r < 0.10 {
                    // ≤10%: 2-level XXXX.XX — minimised; still present for format variety
                    Ok(format!("{:04}.{:02}", heading, b))
                } else if r < 0.80 {
                    // 70%: 3-level XXXX.XX.XX (standard HS 8-digit national tariff)
                    let c = self.rng.gen_range(0u32..100);
                    Ok(format!("{:04}.{:02}.{:02}", heading, b, c))
                } else {
                    // 20%: 4-level XXXX.XX.XX.XX (10-digit statistical suffix)
                    let c = self.rng.gen_range(0u32..100);
                    let d = self.rng.gen_range(0u32..100);
                    Ok(format!("{:04}.{:02}.{:02}.{:02}", heading, b, c, d))
                }
            }
            ("transportation", "unlocode") => {
                // 2-letter country code + 3-char location
                let country_codes = [
                    "US", "GB", "DE", "FR", "NL", "SG", "CN", "JP", "AU", "BR", "IN", "AE", "KR",
                    "IT", "ES", "CA", "MX", "NO", "SE", "DK",
                ];
                let country = country_codes[self.rng.gen_range(0..country_codes.len())];
                let loc_chars = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
                let loc: String = (0..3)
                    .map(|_| loc_chars[self.rng.gen_range(0..loc_chars.len())] as char)
                    .collect();
                Ok(format!("{}{}", country, loc))
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
}
