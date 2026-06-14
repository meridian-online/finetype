//! Generators for the `technology` domain.

use super::*;

impl Generator {
    // ═══════════════════════════════════════════════════════════════════════════
    // DOMAIN: technology (34 types)
    // ═══════════════════════════════════════════════════════════════════════════

    pub(crate) fn gen_technology(
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
            // ("internet", "port") => REMOVED
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
                // v14 AC-03(b): Diverse UAs — browser, tool, bot, and short UAs.
                // Weighted to produce realistic distribution: ~50% browser, ~30% tool, ~20% bot.
                let r = self.rng.gen_range(0..100);
                let agent = if r < 50 {
                    // Browser UAs (diverse platforms and versions)
                    let browser_uas = [
                        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_2) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
                        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
                        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
                        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
                        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.43 Mobile Safari/537.36",
                        "Mozilla/5.0 (iPad; CPU OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
                        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
                    ];
                    browser_uas[self.rng.gen_range(0..browser_uas.len())]
                } else if r < 80 {
                    // Tool/library UAs (short format — key for breaking JWT false pattern)
                    let tool_uas = [
                        "curl/8.4.0",
                        "python-requests/2.31.0",
                        "Go-http-client/2.0",
                        "kube-probe/1.28",
                        "Wget/1.21.4",
                        "HTTPie/3.2.2",
                        "axios/1.6.2",
                        "node-fetch/3.3.2",
                        "okhttp/4.12.0",
                        "Apache-HttpClient/4.5.14",
                        "libcurl/8.4.0 OpenSSL/3.0.12",
                        "Ruby/3.2.2",
                        "Dart/3.2 (dart:io)",
                        "grpc-go/1.60.0",
                    ];
                    tool_uas[self.rng.gen_range(0..tool_uas.len())]
                } else {
                    // Bot/crawler UAs
                    let bot_uas = [
                        "Googlebot/2.1 (+http://www.google.com/bot.html)",
                        "Bingbot/2.0 (+http://www.bing.com/bingbot.htm)",
                        "Slackbot-LinkExpanding 1.0 (+https://api.slack.com/robots)",
                        "Twitterbot/1.0",
                        "facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)",
                        "LinkedInBot/1.0 (compatible; Mozilla/5.0)",
                        "Prometheus/2.48.0",
                        "Datadog/Agent/7.50.0",
                    ];
                    bot_uas[self.rng.gen_range(0..bot_uas.len())]
                };
                Ok(agent.to_string())
            }
            ("internet", "http_method") => {
                let methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];
                Ok(methods[self.rng.gen_range(0..methods.len())].to_string())
            }
            // ("internet", "http_status_code") => REMOVED
            ("internet", "cidr") => {
                let prefix_len = self.rng.gen_range(0..33u8);
                // Generate network-aligned IP for common prefixes
                let (a, b, c, d) = match prefix_len {
                    0 => (0, 0, 0, 0),
                    1..=8 => (self.rng.gen_range(1..255), 0, 0, 0),
                    9..=16 => (self.rng.gen_range(1..255), self.rng.gen_range(0..255), 0, 0),
                    17..=24 => (
                        self.rng.gen_range(1..255),
                        self.rng.gen_range(0..255),
                        self.rng.gen_range(0..255),
                        0,
                    ),
                    _ => (
                        self.rng.gen_range(1..255),
                        self.rng.gen_range(0..255),
                        self.rng.gen_range(0..255),
                        self.rng.gen_range(0..255),
                    ),
                };
                Ok(format!("{}.{}.{}.{}/{}", a, b, c, d, prefix_len))
            }
            ("internet", "urn") => {
                let nids = [
                    "isbn", "ietf", "uuid", "oid", "lex", "example", "xmlorg", "publicid",
                ];
                let nid = nids[self.rng.gen_range(0..nids.len())];
                let nss = match nid {
                    "isbn" => format!("{}", self.rng.gen_range(1000000000u64..9999999999)),
                    "ietf" => format!("rfc:{}", self.rng.gen_range(1000..9999)),
                    "uuid" => {
                        let hex = self.gen_hex_string(32);
                        format!(
                            "{}-{}-{}-{}-{}",
                            &hex[0..8],
                            &hex[8..12],
                            &hex[12..16],
                            &hex[16..20],
                            &hex[20..32]
                        )
                    }
                    "oid" => format!(
                        "2.16.{}.{}",
                        self.rng.gen_range(100..999),
                        self.rng.gen_range(1..99999)
                    ),
                    _ => format!("{}-{}", self.random_word(), self.rng.gen_range(1..999)),
                };
                Ok(format!("urn:{}:{}", nid, nss))
            }
            ("internet", "data_uri") => {
                let media_types = [
                    "text/plain",
                    "text/html",
                    "image/png",
                    "image/jpeg",
                    "image/svg+xml",
                    "application/json",
                    "application/pdf",
                ];
                let media = media_types[self.rng.gen_range(0..media_types.len())];
                if self.rng.gen_bool(0.7) {
                    // base64 variant
                    let base64url =
                        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
                    let len = self.rng.gen_range(20..60);
                    let data: String = (0..len)
                        .map(|_| base64url.as_bytes()[self.rng.gen_range(0..64)] as char)
                        .collect();
                    Ok(format!("data:{};base64,{}=", media, data))
                } else {
                    // plain text variant
                    let word = self.random_word();
                    Ok(format!("data:{},{}", media, word))
                }
            }

            // ── cryptographic (4 types) ──────────────────────────────────
            // uuid moved to representation.identifier
            ("cryptographic", "hash") => {
                // Generate SHA-1 (40) or SHA-256 (64) length hashes.
                // v14 AC-02(d): Bias toward 40/64-char to separate from tsid (32-char).
                // MD5 (32-char) overlaps with tsid — known limitation.
                let len = if self.rng.gen_bool(0.5) { 64 } else { 40 };
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
            ("cryptographic", "jwt") => {
                // Generate realistic JWT: header.payload.signature
                use std::fmt::Write as FmtWrite;
                // Header: {"alg":"HS256","typ":"JWT"} or RS256 variant
                let headers = [
                    r#"{"alg":"HS256","typ":"JWT"}"#,
                    r#"{"alg":"RS256","typ":"JWT"}"#,
                    r#"{"alg":"ES256","typ":"JWT"}"#,
                ];
                let header = headers[self.rng.gen_range(0..headers.len())];
                // Payload: {"sub":"...","iat":...,"exp":...}
                let sub_id = self.rng.gen_range(1000..999999);
                let iat = self.rng.gen_range(1_600_000_000u64..1_900_000_000);
                let exp = iat + self.rng.gen_range(3600..86400);
                let payload = format!(
                    r#"{{"sub":"{}","name":"{}","iat":{},"exp":{}}}"#,
                    sub_id,
                    self.random_word(),
                    iat,
                    exp
                );
                // Signature: random bytes, base64url-encoded
                let sig_bytes: Vec<u8> = (0..32).map(|_| self.rng.gen::<u8>()).collect();

                fn base64url_encode(input: &[u8]) -> String {
                    let mut out = String::new();
                    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
                    for chunk in input.chunks(3) {
                        let b0 = chunk[0] as usize;
                        let b1 = if chunk.len() > 1 {
                            chunk[1] as usize
                        } else {
                            0
                        };
                        let b2 = if chunk.len() > 2 {
                            chunk[2] as usize
                        } else {
                            0
                        };
                        let _ = write!(out, "{}", table[b0 >> 2] as char);
                        let _ = write!(out, "{}", table[((b0 & 3) << 4) | (b1 >> 4)] as char);
                        if chunk.len() > 1 {
                            let _ = write!(out, "{}", table[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
                        }
                        if chunk.len() > 2 {
                            let _ = write!(out, "{}", table[b2 & 0x3f] as char);
                        }
                    }
                    out
                }

                let h = base64url_encode(header.as_bytes());
                let p = base64url_encode(payload.as_bytes());
                let s = base64url_encode(&sig_bytes);
                Ok(format!("{}.{}.{}", h, p, s))
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
            // ── development ────────────────────────────────────
            ("development", "version") => {
                // Always ≥3-part (M.N.P) to avoid collision with decimal_number (2-part)
                let major = self.rng.gen_range(0..20u32);
                let minor = self.rng.gen_range(0..50u32);
                let patch = self.rng.gen_range(0..100u32);
                // v-prefix at ≥40% rate (was 30%)
                let prefix = if self.rng.gen_bool(0.45) { "v" } else { "" };
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
            // technology.development.boolean — REMOVED
            // Relocated to representation.boolean.{binary,initials,terms}
            ("development", "docker_ref") => {
                let registries = [
                    "",
                    "docker.io/",
                    "ghcr.io/",
                    "gcr.io/",
                    "quay.io/",
                    "registry.example.com:5000/",
                ];
                let registry = registries[self.rng.gen_range(0..registries.len())];
                let orgs = ["library", "myorg", "acme", "hashicorp", "grafana"];
                let images = [
                    "nginx", "redis", "postgres", "alpine", "ubuntu", "node", "python", "consul",
                    "vault",
                ];
                let org = orgs[self.rng.gen_range(0..orgs.len())];
                let image = images[self.rng.gen_range(0..images.len())];
                let tag = match self.rng.gen_range(0..4) {
                    0 => "latest".to_string(),
                    1 => format!(
                        "{}.{}.{}",
                        self.rng.gen_range(1..20),
                        self.rng.gen_range(0..10),
                        self.rng.gen_range(0..10)
                    ),
                    2 => format!("{}-alpine", self.rng.gen_range(1..20)),
                    _ => format!("sha-{}", &self.gen_hex_string(8)[..7]),
                };
                if registry.is_empty() {
                    Ok(format!("{}/{}:{}", org, image, tag))
                } else {
                    Ok(format!("{}{}/{}:{}", registry, org, image, tag))
                }
            }

            // ── identifier (3 types) ──────────────────────────────────
            ("identifier", "ulid") => {
                // Crockford Base32: 0-9, A-H, J-K, M-N, P-T, V-X, Y-Z (no I, L, O, U)
                let crockford = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
                // First 10 chars: encode a timestamp in 2020-2030 range
                let base_ms: u64 = 1_577_836_800_000; // 2020-01-01
                let range_ms: u64 = 315_360_000_000; // ~10 years
                let ts = base_ms + self.rng.gen_range(0..range_ms);
                let mut ts_chars = [0u8; 10];
                let mut t = ts;
                for i in (0..10).rev() {
                    ts_chars[i] = crockford[(t % 32) as usize];
                    t /= 32;
                }
                let rand_chars: String = (0..16)
                    .map(|_| crockford[self.rng.gen_range(0..32)] as char)
                    .collect();
                let ts_str: String = ts_chars.iter().map(|&b| b as char).collect();
                Ok(format!("{}{}", ts_str, rand_chars))
            }
            ("identifier", "tsid") => {
                // 32 hex chars with realistic timestamp in leading bytes
                let base_ms: u64 = 1_577_836_800_000; // 2020-01-01
                let range_ms: u64 = 315_360_000_000; // ~10 years
                let ts = base_ms + self.rng.gen_range(0..range_ms);
                let ts_hex = format!("{:012x}", ts);
                let random_hex = self.gen_hex_string(20);
                Ok(format!("{}{}", ts_hex, random_hex))
            }
            ("identifier", "snowflake_id") => {
                // Twitter snowflake: (timestamp_ms - epoch) << 22 | worker << 12 | sequence
                let twitter_epoch: u64 = 1_288_834_974_657;
                let base_ms: u64 = 1_577_836_800_000; // 2020-01-01
                let range_ms: u64 = 315_360_000_000;
                let ts = base_ms + self.rng.gen_range(0..range_ms) - twitter_epoch;
                let worker: u64 = self.rng.gen_range(0..1024);
                let seq: u64 = self.rng.gen_range(0..4096);
                let id = (ts << 22) | (worker << 12) | seq;
                Ok(id.to_string())
            }

            // ── cloud (2 types) ───────────────────────────────────────
            ("cloud", "aws_arn") => {
                let services = [
                    ("s3", "", "", "my-bucket"),
                    ("iam", "", "123456789012", "user/johndoe"),
                    ("iam", "", "123456789012", "role/AdminRole"),
                    (
                        "ec2",
                        "us-east-1",
                        "123456789012",
                        "instance/i-0abcdef1234567890",
                    ),
                    (
                        "ec2",
                        "us-west-2",
                        "987654321098",
                        "vpc/vpc-0a1b2c3d4e5f67890",
                    ),
                    (
                        "lambda",
                        "eu-west-1",
                        "123456789012",
                        "function:my-function",
                    ),
                    (
                        "lambda",
                        "ap-southeast-1",
                        "555555555555",
                        "function:processor",
                    ),
                    ("sqs", "us-east-1", "123456789012", "my-queue"),
                    ("sns", "us-west-2", "123456789012", "my-topic"),
                    ("dynamodb", "eu-central-1", "123456789012", "table/users"),
                    ("rds", "us-east-1", "123456789012", "db:mydb"),
                    (
                        "logs",
                        "us-east-1",
                        "123456789012",
                        "log-group:/aws/lambda/my-function",
                    ),
                ];
                let (service, region, account, resource) =
                    services[self.rng.gen_range(0..services.len())];
                Ok(format!(
                    "arn:aws:{}:{}:{}:{}",
                    service, region, account, resource
                ))
            }
            ("cloud", "s3_uri") => {
                let buckets = [
                    "my-bucket",
                    "data-lake-raw",
                    "production-logs",
                    "ml-models",
                    "analytics-output",
                    "backup-daily",
                    "etl-staging",
                ];
                let bucket = buckets[self.rng.gen_range(0..buckets.len())];
                let path = match self.rng.gen_range(0..4) {
                    0 => format!("data/{}.csv", self.random_word()),
                    1 => format!(
                        "{}/{:02}/events.parquet",
                        self.rng.gen_range(2020..2026),
                        self.rng.gen_range(1..13)
                    ),
                    2 => format!(
                        "ingestion/batch-{:03}/part-{:05}.json",
                        self.rng.gen_range(1..100),
                        self.rng.gen_range(0..10)
                    ),
                    _ => format!("models/{}/model.safetensors", self.random_word()),
                };
                Ok(format!("s3://{}/{}", bucket, path))
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
}
