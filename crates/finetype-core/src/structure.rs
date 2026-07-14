//! Structural substance checks — value-shape predicates that recognise a
//! serialization *structure* rather than a set membership or a check digit.
//!
//! These are the guard-owned equivalent of [`crate::membership`] /
//! [`crate::checksum`] for types whose substance is a nesting grammar. The
//! first (and currently only) member is [`is_s_expression`], the balanced
//! nested-parenthesis check behind `container.object.s_expression`.

/// True if `value` is an S-expression: a balanced, recursively-nested
/// parenthetical structure `(head child child ...)`.
///
/// This is the substance check behind the `s_expression_recovery` Sharpen
/// guard. It is deliberately **truncation-tolerant**: a very long parse tree
/// may reach the guard clipped mid-tree, so a value that opens and nests
/// correctly but never closes (final depth > 0) is still accepted — what is
/// rejected is a value that *closes below zero* (an unbalanced `)` with no
/// matching `(`), which no genuine S-expression prefix can do.
///
/// The signature is self-precise on real corpus data (parse trees, code ASTs,
/// Lisp) with zero measured over-recovery, so no header corroboration is
/// needed — unlike the value-ambiguous checksum/membership types.
///
/// Requirements:
/// - after trimming, at least 5 chars and starts with `(`
/// - parentheses never close below zero (balanced, or open if truncated)
/// - maximum nesting depth >= 2 (a flat `(a b c)` list is not enough)
/// - at least 3 opening parens (multiple nodes, not a single `(x (y))`)
pub fn is_s_expression(value: &str) -> bool {
    let t = value.trim();
    if t.len() < 5 || !t.starts_with('(') {
        return false;
    }
    let mut depth: i32 = 0;
    let mut max_depth: i32 = 0;
    let mut opens: u32 = 0;
    for c in t.chars() {
        match c {
            '(' => {
                depth += 1;
                opens += 1;
                if depth > max_depth {
                    max_depth = depth;
                }
            }
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    max_depth >= 2 && opens >= 3
}

/// Decode an unpadded base64url segment (JWT alphabet `A-Za-z0-9-_`, no `=`).
///
/// Returns `None` on any character outside the alphabet. JWT segments are
/// base64url WITHOUT padding, so padding is never required; a trailing partial
/// group (2–3 leftover chars) contributes its high bits, matching how JWT
/// libraries decode. Small and dependency-free — only the JWT header (the first,
/// short segment) is ever decoded.
fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut buf = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(s.len() * 3 / 4 + 1);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// True if `value` is a JSON Web Token: three dot-separated non-empty segments
/// whose first segment (the header) base64url-decodes to a JSON object carrying
/// an `alg` field.
///
/// This is the substance check behind the `jwt_substance_guard`. The taxonomy
/// pattern only checks the three-base64url-segment SHAPE — which any dotted
/// token-ish string of the right length satisfies — so the model over-emits
/// `jwt` on text (file paths, prose, entity names) at corpus scale. A genuine
/// JWT header decodes to `{"alg":...,"typ":...}`; that `alg` key is the
/// certainty. Only the header is decoded (it is small and leads), so a JWT
/// truncated in its payload or signature still validates.
///
/// Requirements:
/// - exactly three dot-separated, non-empty segments
/// - segment 1 is valid unpadded base64url
/// - the decoded header is a JSON object with an `alg` member
pub fn is_jwt(value: &str) -> bool {
    let t = value.trim();
    let mut parts = t.split('.');
    let (Some(h), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if h.is_empty() || p.is_empty() || s.is_empty() {
        return false;
    }
    let Some(bytes) = b64url_decode(h) else {
        return false;
    };
    match serde_json::from_slice::<serde_json::Value>(&bytes) {
        Ok(v) => v.get("alg").is_some(),
        Err(_) => false,
    }
}

/// The ten registered MIME top-level types (RFC 6838 §4.2). This set is
/// closed by the RFC — new subtypes are registered freely, but a new
/// top-level type requires a standards-track RFC, so the head of every genuine
/// media type is one of these.
const MIME_TOPLEVEL: [&str; 10] = [
    "application",
    "audio",
    "example",
    "font",
    "image",
    "message",
    "model",
    "multipart",
    "text",
    "video",
];

/// True if `value` is a syntactically valid MIME media type: a registered
/// RFC 6838 top-level type, `/`, a token subtype, and optional `;`-parameters.
///
/// This is the substance check behind the `mime_type_substance_guard`. The
/// taxonomy pattern `^[a-zA-Z]+/[a-zA-Z0-9.+\-]+(;.*)?$` accepts ANY word as
/// the top-level type, so the model over-emits `mime_type` on every `word/word`
/// string at corpus scale — slugs (`recipes/deep-mediterranean-quiche`), qualified
/// paths (`ccs/stc2010`, `geoId/15`), namespaces. The certainty the shape lacks
/// is the *closed* top-level-type set: `recipes`/`ccs`/`geoId` are not media types.
/// A full IANA-registry list is deliberately NOT used — it cannot enumerate the
/// open `x-`/`vnd.`/`prs.` subtype trees, so it would false-demote genuine MIME
/// (`application/x-www-form-urlencoded`, `application/vnd.mycorp.thing`).
///
/// Requirements:
/// - exactly one `/` separating a non-empty top-level and a non-empty subtype
///   (parameters after the first `;` are ignored)
/// - the top-level (case-folded) is one of [`MIME_TOPLEVEL`]
/// - the subtype begins with an alphanumeric and is otherwise RFC 6838
///   restricted-name token chars (`A-Za-z0-9` and `!#$&-^_.+`)
pub fn is_mime_type(value: &str) -> bool {
    // Strip parameters: `type/subtype;charset=…` → `type/subtype`.
    let essence = value.trim().split(';').next().unwrap_or("").trim();
    let Some((toplevel, subtype)) = essence.split_once('/') else {
        return false;
    };
    if !MIME_TOPLEVEL.contains(&toplevel.to_ascii_lowercase().as_str()) {
        return false;
    }
    let mut chars = subtype.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false, // empty subtype, or a non-alphanumeric lead
    }
    chars.all(|c| c.is_ascii_alphanumeric() || "!#$&-^_.+".contains(c))
}

/// The 184 ISO 639-1 two-letter language codes.
const ISO639_1: &[&str] = &[
    "aa", "ab", "ae", "af", "ak", "am", "an", "ar", "as", "av", "ay", "az", "ba", "be", "bg", "bh",
    "bi", "bm", "bn", "bo", "br", "bs", "ca", "ce", "ch", "co", "cr", "cs", "cu", "cv", "cy", "da",
    "de", "dv", "dz", "ee", "el", "en", "eo", "es", "et", "eu", "fa", "ff", "fi", "fj", "fo", "fr",
    "fy", "ga", "gd", "gl", "gn", "gu", "gv", "ha", "he", "hi", "ho", "hr", "ht", "hu", "hy", "hz",
    "ia", "id", "ie", "ig", "ii", "ik", "io", "is", "it", "iu", "ja", "jv", "ka", "kg", "ki", "kj",
    "kk", "kl", "km", "kn", "ko", "kr", "ks", "ku", "kv", "kw", "ky", "la", "lb", "lg", "li", "ln",
    "lo", "lt", "lu", "lv", "mg", "mh", "mi", "mk", "ml", "mn", "mr", "ms", "mt", "my", "na", "nb",
    "nd", "ne", "ng", "nl", "nn", "no", "nr", "nv", "ny", "oc", "oj", "om", "or", "os", "pa", "pi",
    "pl", "ps", "pt", "qu", "rm", "rn", "ro", "ru", "rw", "sa", "sc", "sd", "se", "sg", "si", "sk",
    "sl", "sm", "sn", "so", "sq", "sr", "ss", "st", "su", "sv", "sw", "ta", "te", "tg", "th", "ti",
    "tk", "tl", "tn", "to", "tr", "ts", "tt", "tw", "ty", "ug", "uk", "ur", "uz", "ve", "vi", "vo",
    "wa", "wo", "xh", "yi", "yo", "za", "zh", "zu",
];

/// ISO 639-2/639-3 three-letter language codes (both /B bibliographic and /T
/// terminological variants) for languages that appear in real locale data.
/// Load-bearing: without this set, any 3-letter English word (`and`/`are`/`the`)
/// would masquerade as a language, and genuine 3-letter tags (`eng`/`fra`/`ara`)
/// could not be told apart from them.
const ISO639_23: &[&str] = &[
    "aar", "abk", "afr", "aka", "amh", "ara", "arg", "asm", "ava", "ave", "aym", "aze", "bak",
    "bam", "bel", "ben", "bih", "bis", "bod", "bos", "bre", "bul", "cat", "ces", "cha", "che",
    "chi", "chu", "chv", "cor", "cos", "cre", "cym", "cze", "dan", "deu", "div", "dut", "dzo",
    "ell", "eng", "epo", "est", "eus", "ewe", "fao", "fas", "fij", "fin", "fra", "fre", "fry",
    "ful", "geo", "ger", "gla", "gle", "glg", "glv", "gre", "grn", "guj", "hat", "hau", "heb",
    "her", "hin", "hmo", "hrv", "hun", "hye", "ibo", "ice", "ido", "iii", "iku", "ile", "ina",
    "ind", "ipk", "isl", "ita", "jav", "jpn", "kal", "kan", "kas", "kat", "kau", "kaz", "khm",
    "kik", "kin", "kir", "kom", "kon", "kor", "kua", "kur", "lao", "lat", "lav", "lim", "lin",
    "lit", "ltz", "lub", "lug", "mac", "mah", "mal", "mao", "mar", "may", "mkd", "mlg", "mlt",
    "mon", "mri", "msa", "mya", "nau", "nav", "nbl", "nde", "ndo", "nep", "nld", "nno", "nob",
    "nor", "nya", "oci", "oji", "ori", "orm", "oss", "pan", "per", "pli", "pol", "por", "prs",
    "pus", "roh", "ron", "rum", "run", "rus", "sag", "san", "sin", "slk", "slo", "slv", "sme",
    "smo", "sna", "snd", "som", "sot", "spa", "sqi", "srd", "srp", "ssw", "sun", "swa", "swe",
    "tah", "tam", "tat", "tel", "tgk", "tgl", "tha", "tib", "tir", "ton", "tsn", "tso", "tuk",
    "tur", "twi", "uig", "ukr", "urd", "uzb", "ven", "vie", "vol", "wel", "wln", "wol", "xho",
    "yid", "yor", "zha", "zho", "zul", "fil", "ceb", "haw",
];

/// ISO 3166-1 alpha-2 region codes (mirrors the `country_code` taxonomy enum).
/// Load-bearing for rejecting translation pairs like `pt-en`/`es-en`, whose
/// second subtag is a language, not a region.
const ISO3166_1: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

/// Common ISO 15924 four-letter script subtags (`zh-Hans`, `sr-Cyrl`).
const ISO15924: &[&str] = &[
    "Latn", "Cyrl", "Grek", "Hans", "Hant", "Arab", "Hebr", "Deva", "Thai", "Jpan", "Kore", "Hang",
    "Hani", "Kana", "Hira", "Cans", "Ethi", "Geor", "Armn", "Beng", "Guru", "Gujr", "Taml", "Telu",
    "Knda", "Mlym", "Sinh", "Mymr", "Khmr", "Laoo", "Tibt", "Mong",
];

fn in_set(set: &[&str], s: &str) -> bool {
    set.iter().any(|k| k.eq_ignore_ascii_case(s))
}

/// True if `tag` is a single well-formed BCP-47 language tag: a real ISO-639
/// language primary subtag, then only valid script / region / variant subtags.
fn is_bcp47_tag(tag: &str) -> bool {
    let mut subs = tag.split(['-', '_']);
    let Some(prim) = subs.next() else {
        return false;
    };
    let lang_ok = match prim.len() {
        2 => in_set(ISO639_1, prim),
        3 => in_set(ISO639_23, prim),
        _ => false,
    };
    if !lang_ok {
        return false;
    }
    subs.all(|sub| {
        (sub.len() == 4 && in_set(ISO15924, sub))                            // script
            || (sub.len() == 2 && in_set(ISO3166_1, sub))                    // region (alpha-2)
            || (sub.len() == 3 && sub.bytes().all(|b| b.is_ascii_digit()))   // region (M.49)
            || (matches!(sub.len(), 5..=8) && sub.bytes().all(|b| b.is_ascii_alphanumeric()))
        // variant
    })
}

/// True if `value` is a well-formed BCP-47 locale tag (or a delimited list of them).
///
/// This is the substance check behind the `locale_code_substance_guard`. The
/// taxonomy pattern `^[a-zA-Z]{2,3}(?:[-_][a-zA-Z]{2,4})*$` accepts ANY 2–3 letter
/// word, so the model over-emits `locale_code` on text/code columns at corpus
/// scale (survey fragments, dialogue-act tags, single words). The certainty the
/// shape lacks is *closed-set* language membership: the primary subtag must be a
/// real ISO 639 language, and any script/region subtag must be a real ISO 15924 /
/// ISO 3166-1 code. Two calibrated refinements (see `output/certainty-locale/`):
/// - **3-letter primary is checked against the real ISO 639-2/3 set** (not
///   loose-accepted), else English words like `and`/`are` pass;
/// - **delimiter-tolerant**: a cell may be a locale list (`en_US:es_ES:es_MX`),
///   which counts only if EVERY part is a well-formed tag (so a genuine list
///   column is not false-demoted).
///
/// The 2-letter ISO-639 space is collision-dense (`is`/`it`/`to`/`be` are both
/// codes and English words), so a bare-2-letter-word column *can* pass — but such
/// columns route to `word` in the model, and the guard is demote-only, so that is
/// a harmless false-keep, never a false-demote (`output/certainty-locale/findings.md`).
pub fn is_locale_code(value: &str) -> bool {
    let t = value.trim();
    if t.is_empty() {
        return false;
    }
    // Fast path: the whole cell is a single tag.
    if is_bcp47_tag(t) {
        return true;
    }
    // Otherwise treat the cell as a delimited list of locales — genuine locale
    // data (`subtitle_locales`), so it counts only if EVERY part is well-formed
    // and there is more than one part. Space is deliberately NOT a delimiter
    // (it would read prose as a locale list).
    let parts: Vec<&str> = t
        .split([',', ';', ':', '|'])
        .filter(|p| !p.trim().is_empty())
        .map(str::trim)
        .collect();
    parts.len() >= 2 && parts.iter().all(|p| is_bcp47_tag(p))
}

/// File extensions that make a 2-segment dotted string a filename (`report.csv`,
/// `deform_conv_v2.py`) rather than a namespaced code symbol. Only consulted for
/// the 2-segment case — a bare-lowercase filename has no code signal and is
/// already rejected, so this list only needs to catch code/data filenames whose
/// STEM carries a code signal (underscore/CamelCase), the real collision.
const FILE_EXTENSIONS: &[&str] = &[
    "py",
    "rb",
    "js",
    "ts",
    "go",
    "rs",
    "java",
    "c",
    "cpp",
    "cc",
    "h",
    "hpp",
    "cs",
    "php",
    "swift",
    "kt",
    "scala",
    "csv",
    "tsv",
    "txt",
    "json",
    "xml",
    "yaml",
    "yml",
    "toml",
    "md",
    "rst",
    "html",
    "htm",
    "css",
    "scss",
    "sql",
    "sh",
    "bat",
    "ps1",
    "png",
    "jpg",
    "jpeg",
    "gif",
    "svg",
    "bmp",
    "webp",
    "pdf",
    "doc",
    "docx",
    "xls",
    "xlsx",
    "ppt",
    "pptx",
    "zip",
    "gz",
    "tar",
    "rar",
    "log",
    "dat",
    "bin",
    "ini",
    "cfg",
    "conf",
    "properties",
    "jar",
    "war",
    "dll",
    "exe",
    "class",
    "so",
    "mp3",
    "mp4",
    "wav",
    "avi",
    "mov",
    "parquet",
    "npy",
    "pkl",
    "ipynb",
    "h5",
    "hdf5",
    "nc",
    "gz",
    "bz2",
    "xz",
    "orc",
    "avro",
    "feather",
    "sav",
    "dta",
];

/// True when the last dot-separated segment is a LOWERCASE file extension — a
/// filename (`report.tar.gz`, `run_bf.h5`), not a namespaced symbol. Restricted to
/// a lowercase extension so a genuine namespace leaf that happens to share the
/// spelling (`MyApp.Data.Xml`, `Foo.Bar.Sql`) is NOT rejected — a filename's
/// extension is lowercase, a namespace leaf is Capitalized.
fn ends_in_file_extension(parts: &[&str]) -> bool {
    match parts.last() {
        Some(last) if !last.chars().any(|c| c.is_ascii_uppercase()) => {
            FILE_EXTENSIONS.contains(&last.to_ascii_lowercase().as_str())
        }
        _ => false,
    }
}

/// A code signal that distinguishes a namespaced symbol from a bare `foo.bar`
/// hostname: an underscore anywhere, or internal CamelCase (a lowercase letter
/// immediately followed by an uppercase one, e.g. `SisoDb`, `HtmlTags`).
fn has_code_signal(s: &str) -> bool {
    if s.contains('_') {
        return true;
    }
    let b = s.as_bytes();
    b.windows(2)
        .any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase())
}

/// True for a fully-qualified dotted code identifier — a namespaced symbol
/// (Java/.NET package or class, module path, config key: `ICSharpCode.NRefactory6`,
/// `org.jfree.chart.plot.XYPlot`, `calendar.attendee_portal`).
///
/// Every dot-separated segment must be a valid identifier (`^[A-Za-z_]\w*$`).
/// Precision hinges on the 2-segment case, which STRUCTURALLY overlaps a
/// hostname (`www.example`) and a filename (`report.csv`):
/// - **3+ segments** are reverse-DNS-shaped and accepted directly (this matches
///   the `technology.code.qualified_name` taxonomy validator's `{2,}` rule).
/// - **2 segments** are accepted ONLY with a code signal (underscore / internal
///   CamelCase) that a bare lowercase hostname lacks, AND only when the last
///   segment is not a known file extension (excludes `my_file.txt`).
///
/// Whitespace-free, ASCII-identifier segments only. The 3+ case does NOT
/// discriminate hostname (`www.example.com` validates) — that overlap is handled
/// upstream by firing recovery on residual labels only, never overriding a
/// confident `hostname`/`url` prediction.
pub fn is_qualified_name(value: &str) -> bool {
    let t = value.trim();
    if t.len() < 3 || t.len() > 128 || !t.contains('.') || t.chars().any(char::is_whitespace) {
        return false;
    }
    let parts: Vec<&str> = t.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    for p in &parts {
        let mut cs = p.chars();
        match cs.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => return false,
        }
        if !cs.all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    // A lowercase file extension anywhere-terminal marks a filename, not a symbol
    // (`run_bf.h5`, `data.tar.gz`) — reject at any depth.
    if ends_in_file_extension(&parts) {
        return false;
    }
    if parts.len() >= 3 {
        return true;
    }
    // 2-segment: require a code signal (a bare `foo.bar` hostname lacks one).
    has_code_signal(t)
}

/// A common public TLD — the last segment of a canonical hostname. Used to spare a
/// genuine lowercase host (`www.breitbart.com`) from the qualified-name override,
/// which otherwise structurally matches a dotted namespace.
const COMMON_TLDS: &[&str] = &[
    "com", "net", "org", "io", "co", "uk", "de", "fr", "edu", "gov", "mil", "info", "biz", "us",
    "cn", "ru", "jp", "au", "ca", "eu", "nl", "es", "it", "br", "in", "me", "app", "dev", "cloud",
    "xyz", "tv", "ai",
];

/// True for a lowercase, TLD-terminated, underscore-free dotted string — a
/// canonical hostname (`www.example.com`, `amp.washingtontimes.com`) that the
/// qualified-name override must NOT reclassify as code.
fn looks_like_hostname(value: &str) -> bool {
    let t = value.trim();
    if t.contains('_') || t.chars().any(|c| c.is_ascii_uppercase()) {
        return false;
    }
    match t.rsplit('.').next() {
        Some(last) => COMMON_TLDS.contains(&last),
        None => false,
    }
}

/// Stricter form of [`is_qualified_name`] for OVERRIDING a confident foreign
/// prediction — a name/place/host label the model reached for on a dotted
/// PascalCase token (`AgileWizard.Domain` read as a hostname, `Abot2.Tests.Integration`
/// read as an entity name). Requires a code signal a hostname / person-name lacks —
/// an underscore, internal CamelCase (`SisoDb`), or an uppercase letter across 3+
/// segments (`Akka.Remote.Tests`) — AND excludes a canonical hostname
/// ([`looks_like_hostname`]). Measured on the corpus: 613 genuine namespace columns
/// recovered from entity_name/hostname/city with ZERO real-host false positives.
pub fn is_qualified_name_strong(value: &str) -> bool {
    if !is_qualified_name(value) || looks_like_hostname(value) {
        return false;
    }
    let t = value.trim();
    let has_upper = t.chars().any(|c| c.is_ascii_uppercase());
    let n_seg = t.split('.').count();
    has_code_signal(t) || (has_upper && n_seg >= 3)
}

/// True for a bare filename — a stem plus a real, lowercase file extension, with no
/// directory separators (`report_final.xlsx`, `IMG_0042.png`, `archive.tar.gz`).
///
/// The shape `word.word` is not precise (Precision Principle), so precision comes from
/// the curated [`FILE_EXTENSIONS`] set plus four vetoes measured on the corpus:
/// - the terminal extension must be lowercase and a known extension (`gov.MD` / a
///   Capitalized namespace leaf `System.Data.Xml` are excluded — see `is_qualified_name`);
/// - the stem must carry a letter (a pure-numeric stem is not a filename);
/// - a single-char extension (`stdio.h`) requires a >=3-char stem, so a physical-unit value
///   (`mW.h`, `kW.h` watt-hours) is not read as a C-header file;
/// - stem dots are allowed ONLY as a secondary known extension (`archive.tar.gz`), never an
///   arbitrary dotted code namespace (`system.data.sql` is qualified_name territory).
///
/// Residual ccTLD ambiguity (`readme.md` the file vs `gov.md` the domain) is shape-identical
/// and irreducible by value alone — handled at the guard's fire-on + spot-check, not here.
pub fn is_filename(value: &str) -> bool {
    let t = value.trim();
    if t.len() < 3 || t.len() > 255 {
        return false;
    }
    if t.contains('/') || t.contains('\\') || t.contains('@') || t.contains("://") {
        return false;
    }
    if t.chars().any(char::is_whitespace) {
        return false;
    }
    let Some(dot) = t.rfind('.') else {
        return false;
    };
    let (stem, ext) = (&t[..dot], &t[dot + 1..]);
    if stem.is_empty() || ext.is_empty() {
        return false;
    }
    if ext.chars().any(|c| c.is_ascii_uppercase()) || !FILE_EXTENSIONS.contains(&ext) {
        return false;
    }
    if !stem.chars().any(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    if ext.len() == 1 && stem.chars().filter(|c| c.is_ascii_alphanumeric()).count() < 3 {
        return false;
    }
    for seg in stem.split('.').skip(1) {
        if !FILE_EXTENSIONS.contains(&seg.to_ascii_lowercase().as_str()) {
            return false;
        }
    }
    true
}

/// The delimiter a homogeneous delimited-list cell uses.
///
/// Only the *self-precise* delimiters are represented. The bare comma is
/// deliberately absent: a comma between two words is structurally identical
/// whether it separates list items (`LTE, NR`) or lives inside one entity
/// (`Winter Park, Florida`, `$928,760,770`, `Dec 30, 2020`, `10,91`), so a
/// bare-comma list cannot be told from a place / money / date / decimal by
/// value alone. A comma is accepted only when *brackets* disambiguate it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListDelim {
    /// Bracket-wrapped comma list — `[a, b, c]` / `('x', 'y')`.
    Comma,
    /// Pipe list — `a|b|c`.
    Pipe,
    /// Semicolon list — `a;b;c`.
    Semicolon,
}

/// True if every part of a candidate list shares a coarse element shape
/// (numeric / single-token / short-phrase) with no prose element — i.e. it
/// reads as a homogeneous *list* rather than a heterogeneous positional
/// *record* (which is `container.object.csv`, a different leaf). A part longer
/// than six words is prose and disqualifies the cell.
fn list_homogeneous(parts: &[&str]) -> bool {
    if parts.len() < 2 {
        return false;
    }
    let mut counts = [0usize; 3]; // [numeric, single-token, short-phrase]
    for p in parts {
        let words = p.split_whitespace().count();
        if words == 0 || words > 6 {
            return false; // empty or prose
        }
        let numeric = p.chars().any(|c| c.is_ascii_digit())
            && p.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | ','));
        if numeric {
            counts[0] += 1;
        } else if words == 1 {
            counts[1] += 1;
        } else {
            counts[2] += 1;
        }
    }
    let top = *counts.iter().max().unwrap();
    // one coarse shape must dominate (>=80% of parts).
    top * 5 >= parts.len() * 5 - parts.len()
}

/// A part is "date/time-ish" — a clock, a 4-digit year, or a month name. Used
/// to veto a pipe/semicolon "list" that is really a split datetime
/// (`Tuesday, 21 Feb 2017 | 7:58 AM ET`).
fn looks_datetimey(part: &str) -> bool {
    let p = part.trim();
    if p.contains(':') && p.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        return true; // clock 7:58
    }
    let lower = p.to_ascii_lowercase();
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    if MONTHS.iter().any(|m| lower.starts_with(m)) {
        return true;
    }
    // a bare 4-digit token that is a plausible year
    p.len() == 4 && p.chars().all(|c| c.is_ascii_digit()) && p.starts_with(['1', '2'])
}

/// Classify a cell as a homogeneous delimited **list** under a self-precise
/// delimiter, returning which delimiter it uses. Returns `None` for anything
/// that is not unambiguously a list.
///
/// This is the substance check behind the `delimited_array_recovery` Sharpen
/// guard (reservoir-mining sweep, 2026-07-14). Recovers `container.array.*`
/// leaves the 244-dim model strands as residual/entity text. Three branches,
/// each self-precise:
///
/// - **Bracket** `[a, b, c]` / `('x', 'y')` → [`ListDelim::Comma`]. The
///   brackets disambiguate the comma, so a two-element numeric list `[-1, 0]`
///   is admitted (a bare `-1, 0` would not be).
/// - **Pipe** `a|b|c` → [`ListDelim::Pipe`]. The pipe never lives inside
///   dates / money / prose.
/// - **Semicolon** `a;b;c` → [`ListDelim::Semicolon`]. Likewise self-precise.
///
/// Vetoes: any `://` (a URL with commas), prose parts, heterogeneous records
/// (`list_homogeneous`), nested brackets, and — for a **bare** two-part
/// pipe/semicolon list only — anything but two alphabetic single tokens (this
/// drops `id|number` positional records and numeric coordinate/decimal pairs,
/// whose two-element ambiguity a bracket would otherwise resolve). Balanced
/// nested parens are left to [`is_s_expression`], which runs first.
pub fn delimited_list_delim(value: &str) -> Option<ListDelim> {
    let raw = value.trim();
    if raw.len() < 3 || raw.contains("://") {
        return None;
    }
    let bytes = raw.as_bytes();
    let (open, close) = (bytes[0], bytes[raw.len() - 1]);
    let bracketed = (open == b'[' && close == b']') || (open == b'(' && close == b')');
    if bracketed {
        let body = raw[1..raw.len() - 1].trim();
        if !body.contains(',') {
            return None;
        }
        let parts: Vec<&str> = body
            .split(',')
            .map(|p| p.trim().trim_matches(|c| c == '\'' || c == '"').trim())
            .collect();
        if parts.len() >= 2
            && parts
                .iter()
                .all(|p| !p.is_empty() && !p.contains(['[', ']', '{', '}', '(', ')']))
            && list_homogeneous(&parts)
        {
            return Some(ListDelim::Comma);
        }
        return None;
    }
    // Bare pipe / semicolon — self-precise delimiters.
    for (delim, kind) in [('|', ListDelim::Pipe), (';', ListDelim::Semicolon)] {
        if !raw.contains(delim) {
            continue;
        }
        let parts: Vec<&str> = raw.split(delim).map(str::trim).collect();
        if parts.len() < 2 || parts.iter().any(|p| p.is_empty()) {
            continue;
        }
        // A flat-list element is a scalar, never a structured value: a bracket /
        // brace in a part means the cell is a positional record or code artifact
        // (JVM signature `(L…;L…`, `path/|hash/|{json}`), not a homogeneous list.
        if parts
            .iter()
            .any(|p| p.contains(['(', ')', '{', '}', '[', ']']))
        {
            continue;
        }
        if !list_homogeneous(&parts) || parts.iter().any(|p| looks_datetimey(p)) {
            continue;
        }
        // Two-element bare lists are ambiguous (id|num records, x;y pairs); admit
        // only two alphabetic single tokens.
        if parts.len() == 2
            && !parts.iter().all(|p| {
                p.split_whitespace().count() == 1 && p.chars().any(|c| c.is_ascii_alphabetic())
            })
        {
            continue;
        }
        return Some(kind);
    }
    None
}

/// True if `value` is a semantic-version string: `v?MAJOR.MINOR.PATCH` (exactly
/// three numeric components) with an optional `-prerelease` / `+build` suffix.
///
/// This is the VALUE half of the header-gated `version_string_recovery` guard.
/// Unlike the self-precise structural checks in this module, the version shape is
/// value-AMBIGUOUS: a `YYYY.MM.DD` date and a `YYYY.MM.PATCH` calendar version
/// share the same three-dotted-number shape, so this predicate is NEVER used
/// without a corroborating `version`/`ver`/`build` header. It carries one
/// discriminator the taxonomy's plain SemVer regex lacks — a **year veto**: any
/// component that is a four-digit 1900–2099 number is a date / calver, not a
/// SemVer release, so the value is rejected (`2021.03.15`, `1.2020.0`). A genuine
/// SemVer never carries a year-shaped component (`10.15.7`, `1.20.0` pass).
///
/// Requirements:
/// - optional leading `v` / `V`
/// - exactly three `.`-separated components, each non-empty ASCII digits
/// - no component is a four-digit 1900–2099 year
/// - an optional `-`/`+` suffix must be a non-empty identifier (`[A-Za-z0-9.+-]`)
pub fn is_version_string(value: &str) -> bool {
    let v = value.trim().trim_start_matches(['v', 'V']);
    // Split the numeric core from an optional -prerelease / +build tail.
    let (core, has_suffix) = match v.find(['-', '+']) {
        Some(i) => (&v[..i], true),
        None => (v, false),
    };
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    for p in &parts {
        if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        // Year veto: a four-digit 1900–2099 component is a date / calver.
        if p.len() == 4 {
            if let Ok(n) = p.parse::<u32>() {
                if (1900..=2099).contains(&n) {
                    return false;
                }
            }
        }
    }
    if has_suffix {
        let suffix = &v[core.len() + 1..];
        if suffix.is_empty()
            || !suffix
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_accepts_genuine_tokens() {
        // The taxonomy `technology.cryptographic.jwt` samples — real JWTs.
        assert!(is_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"));
        assert!(is_jwt("eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJhdXRoLmV4YW1wbGUuY29tIn0.dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"));
        // Truncated signature — header still decodes, still a JWT.
        assert!(is_jwt(
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxIn0.x"
        ));
    }

    #[test]
    fn jwt_rejects_lookalikes() {
        // The corpus over-emission: Windows file paths and prose the model calls jwt.
        assert!(!is_jwt(
            "D:\\research\\architectureSmells\\repos\\flextry_Tel"
        ));
        assert!(!is_jwt("The tool detected the smell in this class because"));
        // Three dotted segments, but the header is not JSON-with-alg.
        assert!(!is_jwt("foo.bar.baz"));
        assert!(!is_jwt("a.b.c"));
        assert!(!is_jwt("1.2.3")); // dotted version string
                                   // Header decodes to JSON but has no `alg` (e.g. only `typ`).
        assert!(!is_jwt("eyJ0eXAiOiJKV1QifQ.eyJzdWIiOiIxIn0.sig"));
        // Wrong segment count.
        assert!(!is_jwt("eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0")); // two segments
        assert!(!is_jwt("")); // empty
    }

    #[test]
    fn mime_accepts_genuine_media_types() {
        // Taxonomy samples + the real MIME the model correctly labels.
        for m in [
            "text/plain",
            "application/json",
            "image/png",
            "text/html; charset=utf-8",
            "application/vnd.ms-excel",
            "video/3gpp",
            "audio/mpeg",
            "application/x-www-form-urlencoded", // x- tree — a registry list would miss it
            "application/vnd.api+json",          // +suffix
            "IMAGE/JPEG",                        // case-insensitive top-level
            "multipart/form-data",
            "font/woff2",
            "model/gltf-binary",
        ] {
            assert!(is_mime_type(m), "{m} should be a MIME type");
        }
    }

    #[test]
    fn mime_rejects_word_slash_word_lookalikes() {
        // The corpus over-emission: `word/word` strings the permissive pattern
        // confirms but whose top-level is not a registered media type.
        for s in [
            "recipes/deep-mediterranean-quiche",
            "ccs/stc2010",
            "geoId/15",
            "compsec/compsec24",
            "PRESERVE/scapes",
            "public/api/conf",                            // also has a second slash
            "com.google.javascript.jscomp.CodeGenerator", // no slash at all
            "Abstraction",                                // single word
            "/samples/aloha/",                            // leading slash → empty top-level
            "text/",                                      // registered top-level but empty subtype
            "",
        ] {
            assert!(!is_mime_type(s), "{s} must not validate as a MIME type");
        }
    }

    #[test]
    fn accepts_parse_trees_asts_and_lisp() {
        assert!(is_s_expression("(ROOT (S (NP (NN cat)) (VP (VBZ sits))))"));
        assert!(is_s_expression("(program (call (id print) (string hi)))"));
        assert!(is_s_expression("(+ (* 2 3) (- 4 1))"));
        assert!(is_s_expression("(a (b c) (d (e f)))"));
        // Penn comma-token — the shape that fools the comma_separated detector.
        assert!(is_s_expression("(S (INTJ (UH Uh)) (, ,) (NP-SBJ (PRP I)))"));
    }

    #[test]
    fn tolerates_truncation_mid_tree() {
        // Clipped before the closing parens — still a valid open prefix.
        assert!(is_s_expression(
            "(ROOT (SINV (ADVP (RB so)) (, ,) (SBARQ (INTJ (UH uh"
        ));
    }

    #[test]
    fn rejects_non_s_expressions() {
        assert!(!is_s_expression("apple,banana,cherry")); // comma list
        assert!(!is_s_expression("(a b c)")); // flat, depth 1
        assert!(!is_s_expression("hello (world)")); // does not start with (
        assert!(!is_s_expression("(a)) more")); // closes below zero
        assert!(!is_s_expression("(x)")); // too short / not nested
        assert!(!is_s_expression("")); // empty
        assert!(!is_s_expression("{\"a\": 1}")); // JSON, not parens
        assert!(!is_s_expression("(a) (b) (c)")); // three flat groups, depth 1
    }

    #[test]
    fn requires_nesting_not_just_many_parens() {
        // depth 1 throughout despite many opens → not an s-expression
        assert!(!is_s_expression("(a) (b) (c) (d)"));
    }

    #[test]
    fn locale_accepts_genuine_tags() {
        for s in [
            "en",
            "fr",
            "de",
            "pt-BR",
            "en-US",
            "zh-Hans",
            "zh-Hant",
            "de-AT",
            "zh-Hans-CN", // language + script + region
            "eng",
            "fra",
            "ara", // 3-letter ISO 639-2/3 primaries (the `lang_code` column)
            "en_US",
            "pt_br", // underscore separator, mixed case — BCP-47 is case-insensitive
            "EN-US",
            "es-419",                  // M.49 numeric region (Latin America)
            "en_US:es_ES:es_MX:pt_BR", // subtitle_locales: a delimited list of locales
            "de-DE;fr-FR;it-IT",       // semicolon list
        ] {
            assert!(is_locale_code(s), "{s} should validate as a locale code");
        }
    }

    #[test]
    fn locale_rejects_over_emission_lookalikes() {
        for s in [
            "and", // 3-letter English word, not an ISO 639-2/3 code
            "the",
            "are",
            "for",
            "sector", // NAICS `level` enum member
            "responseOptions",
            "decided",
            "qy",    // dialogue-act tag, not a language code
            "en-en", // translation pair: second subtag `en` is a language, not a region
            "pt-en",
            "of",             // 2-letter word that is NOT ISO 639-1
            "recipes/quiche", // slash is not a locale separator
            "en-",            // trailing empty subtag
            "e",              // single letter
            "",
        ] {
            assert!(!is_locale_code(s), "{s} must not validate as a locale code");
        }
    }

    #[test]
    fn qualified_name_accepts_code_namespaces() {
        for s in [
            "ICSharpCode.NRefactory6",  // 2-seg, CamelCase
            "SisoDb.Sql2008",           // 2-seg, CamelCase
            "HtmlTags.Testing",         // 2-seg, CamelCase
            "calendar.attendee_portal", // 2-seg, underscore
            "mail.model_mail_message",  // 2-seg, underscore
            "Aspose.NET_OneClick_Word_Document",
            "org.jfree.chart.plot.XYPlot", // reverse-DNS 5-seg
            "com.example.app",             // reverse-DNS 3-seg lowercase
            "EmployeeDirectory.Android",   // 2-seg, CamelCase
        ] {
            assert!(is_qualified_name(s), "{s} should be a qualified name");
        }
    }

    #[test]
    fn qualified_name_rejects_hostnames_filenames_prose() {
        for s in [
            "example.com",       // 2-seg bare lowercase — hostname, no code signal
            "foo.bar",           // 2-seg bare lowercase — no code signal
            "report.csv",        // filename (no code signal anyway)
            "deform_conv_v2.py", // filename WITH underscore stem — ext check load-bearing
            "scaled_mnist_train.py",
            "myFile.txt",      // filename WITH CamelCase stem
            "John.Smith",      // Title.Title — no internal CamelCase, no underscore
            "hello world.foo", // whitespace
            "no_dot_here",     // no dot
            "123.456",         // segments not identifiers
            ".leading",        // empty first segment
            "trailing.",       // empty last segment
            "",
        ] {
            assert!(!is_qualified_name(s), "{s} must not be a qualified name");
        }
    }

    #[test]
    fn qualified_name_strong_accepts_namespaces_over_confident_labels() {
        // The override detector: dotted namespaces the model mislabels as a
        // name/place/host — must carry a code signal.
        for s in [
            "AgileWizard.Domain",       // 2-seg CamelCase (Sense=hostname)
            "Abot2.Tests.Integration",  // 3-seg, uppercase (Sense=entity_name)
            "Akka.Remote.Tests",        // 3-seg, uppercase, no internal camel
            "calendar.attendee_portal", // underscore
            "SisoDb.Sql2008",           // internal CamelCase
        ] {
            assert!(
                is_qualified_name_strong(s),
                "{s} should be a strong qualified name"
            );
        }
    }

    #[test]
    fn qualified_name_strong_spares_real_hosts_and_lowercase() {
        for s in [
            "www.breitbart.com", // canonical host — must be spared
            "amp.washingtontimes.com",
            "api.github.com",
            "www.politico.com",
            "com.google.common", // lowercase package — ambiguous with a host, spared
            "foo.bar.baz",       // lowercase, no code signal
        ] {
            assert!(
                !is_qualified_name_strong(s),
                "{s} must NOT override a confident label"
            );
        }
    }

    #[test]
    fn filename_accepts_real_files() {
        for s in [
            "report_final.xlsx",
            "IMG_0042.png",
            "archive.tar.gz", // double extension
            "atarisy2.cpp",
            "stdio.h", // single-char ext, >=3-char stem
            "banner1.jpg",
            "readme.md",
        ] {
            assert!(is_filename(s), "{s} should be a filename");
        }
    }

    #[test]
    fn filename_rejects_non_files() {
        for s in [
            "mW.h", // watt-hour unit — single-char ext, 2-char stem
            "kW.h",
            "system.data.sql", // code namespace ending in an ext word (not a double-ext)
            "com.example.xml", // namespace, not a file
            "C:\\dir\\file.txt", // has a path separator
            "/usr/bin/run.sh", // has a path separator
            "user@host.png",   // has @
            "http://x.com/a.png", // URL
            "foo.bar",         // bar is not a known extension
            "42.7",            // numeric stem, not an extension
            "plainword",       // no dot
            "",
        ] {
            assert!(!is_filename(s), "{s} must NOT be a filename");
        }
    }

    #[test]
    fn delimited_list_accepts_self_precise_lists() {
        use ListDelim::*;
        // bracket-wrapped comma lists (brackets disambiguate the comma)
        assert_eq!(delimited_list_delim("[20000, 10000, 15000]"), Some(Comma));
        assert_eq!(
            delimited_list_delim("['email', 'phone', 'facebook']"),
            Some(Comma)
        );
        assert_eq!(delimited_list_delim("[-1, 0]"), Some(Comma)); // 2-elem OK under brackets
        assert_eq!(delimited_list_delim("('soxr', 'ompn')"), Some(Comma));
        // pipe lists
        assert_eq!(
            delimited_list_delim("Biography|Comedy|Drama|Music"),
            Some(Pipe)
        );
        assert_eq!(delimited_list_delim("man | arrive | beginning"), Some(Pipe));
        // semicolon lists
        assert_eq!(
            delimited_list_delim("nanoparticles;polymers;raman-spectroscopy"),
            Some(Semicolon)
        );
        assert_eq!(
            delimited_list_delim("Tim Robbins; Morgan Freeman; Bob Gunton"),
            Some(Semicolon)
        );
        assert_eq!(
            delimited_list_delim("Specialty1;Specialty2"),
            Some(Semicolon)
        ); // 2 single tokens
    }

    #[test]
    fn delimited_list_rejects_false_friends() {
        // bare comma is NEVER a list (city / address / money / date / decimal)
        assert_eq!(delimited_list_delim("Winter Park, Florida"), None);
        assert_eq!(
            delimited_list_delim("Carier Site, East Street, Braintree"),
            None
        );
        assert_eq!(delimited_list_delim("$928,760,770"), None);
        assert_eq!(delimited_list_delim("Dec 30, 2020"), None);
        assert_eq!(delimited_list_delim("10,91"), None);
        assert_eq!(delimited_list_delim("LTE, NR"), None); // genuine 2-item, but unbracketed comma -> not safe
                                                           // URL with commas
        assert_eq!(delimited_list_delim("https://x.com/a,b,c"), None);
        // heterogeneous positional records (csv_record, not a homogeneous array)
        assert_eq!(delimited_list_delim("343597384500|5875"), None); // 2-part numeric record
        assert_eq!(
            delimited_list_delim("Tuesday, 21 Feb 2017 | 7:58 AM ET"),
            None
        ); // split datetime
        assert_eq!(delimited_list_delim("9.9312|76.2673"), None); // coordinate pair
                                                                  // prose list element disqualifies (>6 words)
        assert_eq!(
            delimited_list_delim("the quick brown fox jumped over the lazy dog;a;b"),
            None
        );
        // nested / s-expression shapes are not ours
        assert_eq!(delimited_list_delim("[[1,2],[3,4]]"), None);
        assert_eq!(delimited_list_delim("(a (b c) (d e))"), None);
        // code artifacts: a list element carrying a bracket/brace is not a scalar
        assert_eq!(
            delimited_list_delim("(Lcom/spotify/Point;Lcom/fasterxml/JsonGenerator;"),
            None
        ); // JVM sig
        assert_eq!(delimited_list_delim("demirbas/|26Rew/|{\"olculer\""), None);
        // path|hash|json record
    }

    #[test]
    fn version_string_accepts_semver() {
        for s in [
            "1.6.1",
            "v1.2.3",
            "V2.0.0",
            "0.2.53",
            "10.15.7",
            "1.20.0",
            "1.2.3-alpha",
            "1.2.3-beta.1",
            "1.2.3+build.123",
            "1.11.23",
        ] {
            assert!(is_version_string(s), "{s} must be a version");
        }
    }

    #[test]
    fn version_string_rejects_dates_and_non_versions() {
        for s in [
            "2021.03.15", // YYYY.MM.DD date -> year veto
            "1.2020.0",   // calver year component -> year veto
            "2016.10.05", // date -> year veto
            "1.2",        // two components (decimal / partial)
            "1.2.3.4",    // four components (IPv4)
            "1.2.x",      // non-numeric component
            "",
            "foo",
            "1..3", // empty component
            "v",    // bare v
        ] {
            assert!(!is_version_string(s), "{s} must NOT be a version");
        }
    }
}
