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
}
