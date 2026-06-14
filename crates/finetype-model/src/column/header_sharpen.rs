use super::*;

/// True if the header carries a coordinate token, i.e. it corroborates a
/// latitude/longitude prediction. Generous by design — protecting the recall of
/// genuine coordinate columns matters more than catching every false positive,
/// so any plausible coordinate token keeps the prediction. Token-aware (splits
/// on non-alphanumerics) so `lat_dd`/`y_lat` corroborate but `latency`/`translate`
/// do not.
pub(crate) fn header_corroborates_coordinate(header: &str) -> bool {
    let h = header.to_lowercase();
    if header_hint(&h).is_some_and(|t| t.starts_with("geography.coordinate")) {
        return true;
    }
    if h.contains("latitude")
        || h.contains("longitude")
        || h.contains("coord")
        || h.contains("wgs84")
        || h.contains("northing")
        || h.contains("easting")
    {
        return true;
    }
    h.split(|c: char| !c.is_alphanumeric()).any(|tok| {
        matches!(
            tok,
            "lat" | "lon" | "lng" | "long" | "lats" | "lons" | "latdd" | "londd" | "gps"
        )
    })
}
/// Closed-vocabulary state/province codes for the three `state_code` locales the
/// taxonomy defines (EN_US, EN_CA, EN_AU; `geography.location.state_code`
/// `validation_by_locale`). Duplicated here as the value discriminator for the
/// Sharpen promotion — the taxonomy enum remains the validation source of truth;
/// this set is stable (subdivision codes change on the order of decades). US 50 +
/// DC, Canadian provinces/territories, Australian states/territories (2–3 letters).
pub(crate) const STATE_CODES: &[&str] = &[
    // US 50 + DC
    "AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA", "HI", "ID", "IL", "IN", "IA", "KS",
    "KY", "LA", "ME", "MD", "MA", "MI", "MN", "MS", "MO", "MT", "NE", "NV", "NH", "NJ", "NM", "NY",
    "NC", "ND", "OH", "OK", "OR", "PA", "RI", "SC", "SD", "TN", "TX", "UT", "VT", "VA", "WA", "WV",
    "WI", "WY", "DC", // Canadian provinces/territories
    "AB", "BC", "MB", "NB", "NL", "NS", "ON", "PE", "QC", "SK", "NT", "NU", "YT",
    // Australian states/territories
    "NSW", "VIC", "QLD", "WA", "SA", "TAS", "ACT",
];
/// True if the header corroborates a state/province subdivision (the disambiguator
/// the `state_code` promotion REQUIRES, because 2-letter codes overlap ISO country
/// codes — `CA` is California AND Canada, `GA` is Georgia-state AND Georgia-country).
/// Token-aware so `state`/`province` match but false friends do not (`statement`,
/// `status`, `estate` tokenize away from `state`).
pub(crate) fn header_corroborates_state(header: &str) -> bool {
    header
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .any(|tok| matches!(tok, "state" | "states" | "province" | "provinces" | "prov"))
}
/// True if the sampled values are predominantly closed-vocabulary state/province
/// codes — the value discriminator for the `state_code` promotion. Requires a
/// majority (>= 80%) of non-empty values to be members of [`STATE_CODES`]
/// (uppercased). A full state-name column (`California`) fails (not 2-3 letter
/// codes); a country-code column fails (most ISO codes are not state codes), so
/// enum membership plus the header guard cleanly separates state_code from its
/// value-identical neighbours (state, region, country_code).
pub(crate) fn values_look_like_state_codes(sample: &[String]) -> bool {
    let (mut hit, mut total) = (0usize, 0usize);
    for v in sample.iter().take(16) {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        total += 1;
        let up = t.to_ascii_uppercase();
        if STATE_CODES.contains(&up.as_str()) {
            hit += 1;
        }
    }
    total >= 3 && hit * 100 >= total * 80
}
/// True if the sampled values are predominantly BARE numbers — a plain integer or
/// decimal literal (`^-?\d+(\.\d+)?$`) with NO currency symbol, code, grouping, or
/// other money formatting. The value discriminator for the currency-amount veto:
/// on gold a genuine `currency.amount` carries a currency signal (`£45.17`,
/// `EUR 4 459 807`), whereas accounting line items (`netIncome` 795000000,
/// `interestExpense` -68000000) are bare numbers the gold labels `integer_number` /
/// `decimal_number` regardless of their money-ish HEADER — so the header cannot
/// disambiguate (both sides carry money headers) but the value formatting can.
/// Returns (column-is-bare → demote, any-value-decimal → demote to decimal not integer).
pub(crate) fn values_look_like_bare_numbers(sample: &[String]) -> (bool, bool) {
    let (mut bare, mut total, mut any_decimal) = (0usize, 0usize, false);
    for v in sample.iter().take(16) {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        total += 1;
        let body = t.strip_prefix('-').unwrap_or(t);
        let mut dots = 0usize;
        let is_bare = !body.is_empty()
            && body.chars().all(|c| {
                if c == '.' {
                    dots += 1;
                    true
                } else {
                    c.is_ascii_digit()
                }
            })
            && dots <= 1;
        if is_bare {
            bare += 1;
            if dots == 1 {
                any_decimal = true;
            }
        }
    }
    (total >= 3 && bare * 100 >= total * 80, any_decimal)
}
/// True if the sampled values are predominantly numeric — the value-validation
/// guard for the coordinate veto, so it only fires on genuine numeric columns.
pub(crate) fn values_look_like_generic_decimals(sample: &[String]) -> bool {
    let (mut numeric, mut total) = (0usize, 0usize);
    for v in sample.iter().take(8) {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        total += 1;
        if t.parse::<f64>().is_ok() {
            numeric += 1;
        }
    }
    total > 0 && numeric * 100 >= total * 80
}
/// True if the header carries a postal token, i.e. it corroborates a
/// postal_code prediction. Generous by design (recall of genuine postal
/// columns beats catching every false positive). Token-aware for the short
/// terms so `zip_code`/`business_zip` corroborate but `zipper`/`unzip` do not;
/// locale terms included (PLZ, CEP, pincode).
pub(crate) fn header_corroborates_postal(header: &str) -> bool {
    // NB: deliberately NOT delegating to header_hint() — its substring matcher
    // treats any "zip" substring as postal (zipper, gzip), which is exactly the
    // looseness a corroboration check must not inherit.
    let h = header.to_lowercase();
    if h.contains("postal") || h.contains("postcode") || h.contains("pincode") {
        return true;
    }
    let mut has_post = false;
    let mut has_code = false;
    for tok in h.split(|c: char| !c.is_alphanumeric()) {
        match tok {
            "zip" | "zipcode" | "zip4" | "plz" | "cep" => return true,
            "post" => has_post = true,
            "code" => has_code = true,
            _ => {}
        }
    }
    has_post && has_code
}
/// True if the sampled values are predominantly bare integers WITHOUT leading
/// zeros — the value-validation guard for the postal veto. A leading zero
/// (`01219`) is positive postal evidence, so any such value blocks the veto.
pub(crate) fn values_look_like_generic_integers(sample: &[String]) -> bool {
    let (mut bare_int, mut total) = (0usize, 0usize);
    for v in sample.iter().take(8) {
        let t = v.trim();
        if t.is_empty() {
            continue;
        }
        total += 1;
        let digits = t.strip_prefix('-').unwrap_or(t);
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            if digits.len() > 1 && digits.starts_with('0') {
                return false; // leading zero — looks like a real code/zip
            }
            bare_int += 1;
        }
    }
    total > 0 && bare_int * 100 >= total * 80
}
/// Map a column header name to a hinted type label.
///
/// Uses case-insensitive substring/keyword matching. Returns the most likely
/// type label for the column based on its name, or None if no match.
pub(crate) fn header_hint(header: &str) -> Option<&'static str> {
    let h = header.to_lowercase();
    // Remove common prefixes/suffixes and separators for matching
    let h = h.replace(['_', '-'], " ");
    let h = h.trim();

    // RHH instrumentation hooks — see crate::rhh and
    // .orbit/specs/2026-04-24-remove-header-hints/spec.yaml ac-02. When the
    // `rhh-instrumentation` feature is off, all `disable_*` are constant
    // `false` and the optimiser eliminates the conjunctions; zero overhead.
    let disable_table = rhh::is_disabled("header_hint_table");
    let disable_identity = rhh::is_disabled("substring_matcher_identity");
    let disable_technology = rhh::is_disabled("substring_matcher_technology");
    let disable_geography = rhh::is_disabled("substring_matcher_geography");
    let disable_datetime = rhh::is_disabled("substring_matcher_datetime");
    let disable_finance = rhh::is_disabled("substring_matcher_finance");
    let disable_representation = rhh::is_disabled("substring_matcher_representation");

    // Exact or near-exact matches first (most specific)
    if !disable_table {
        match h {
            "email" | "e mail" | "email address" | "emailaddress" => {
                return Some("identity.person.email");
            }
            "url" | "uri" | "link" | "href" | "website" | "homepage" => {
                return Some("technology.internet.url");
            }
            "ip" | "ip address" | "ipaddress" | "ip addr" | "source ip" | "destination ip"
            | "src ip" | "dst ip" | "server ip" | "client ip" | "remote ip" | "local ip" => {
                return Some("technology.internet.ip_v4");
            }
            "uuid" | "guid" => {
                return Some("technology.identifier.uuid");
            }
            "gender" | "sex" => {
                return Some("identity.person.gender");
            }
            // "age" is not a taxonomy type; redirect to integer_number
            "age" | "patient age" | "customer age" | "user age" => {
                return Some("representation.numeric.integer_number");
            }
            // Epoch / Unix timestamps — 10-digit integers confused with NPI
            "epoch" | "unix epoch" | "unix timestamp" | "epoch time" | "unix time"
            | "epoch seconds" | "unix seconds" | "epoch s" | "posix time"
            | "unix epoch seconds" => {
                return Some("datetime.epoch.unix_seconds");
            }
            "epoch ms" | "unix ms" | "epoch milliseconds" | "unix milliseconds" => {
                return Some("datetime.epoch.unix_milliseconds");
            }
            // Altitude / elevation — numeric measurements confused with numeric_code
            "altitude" | "elevation" | "alt" | "elev" => {
                return Some("representation.numeric.integer_number");
            }
            // Duration in numeric units
            // Note: bare "duration" excluded — could be ISO 8601 (PT1H30M).
            // Only specific numeric-unit variants match.
            "duration minutes" | "duration min" | "duration seconds" | "duration sec"
            | "duration hours" | "duration hrs" | "duration ms" | "elapsed" | "elapsed time" => {
                return Some("representation.numeric.integer_number");
            }
            // Attendance / headcount — large integers
            "attendance" | "headcount" | "participants" | "crowd size" | "capacity" => {
                return Some("representation.numeric.integer_number");
            }
            // Heart rate / vital signs — numeric measurements
            "heart rate" | "heartrate" | "hr" | "bpm" | "pulse" => {
                return Some("representation.numeric.integer_number");
            }
            // Pages — book/document page counts
            "pages" | "page count" | "num pages" | "total pages" => {
                return Some("representation.numeric.integer_number");
            }
            // Language — programming or natural language names
            "language" | "lang" | "programming language" | "spoken language" => {
                return Some("representation.discrete.categorical");
            }
            // Sport / athletic discipline
            "sport" | "discipline" | "event type" | "game type" => {
                return Some("representation.discrete.categorical");
            }
            // Species / taxonomy
            "species" | "genus" | "taxon" | "breed" | "variety" => {
                return Some("representation.discrete.categorical");
            }
            // Exchange — financial exchange names
            "exchange" | "stock exchange" | "market" | "bourse" => {
                return Some("representation.discrete.categorical");
            }
            "latitude" | "lat" => {
                return Some("geography.coordinate.latitude");
            }
            "longitude" | "lng" | "lon" | "long" => {
                return Some("geography.coordinate.longitude");
            }
            "country" | "country name" => {
                return Some("geography.location.country");
            }
            "country code" | "alpha 2" | "alpha 3" | "iso country" | "iso alpha 2"
            | "iso alpha 3" | "country iso" => {
                return Some("geography.location.country_code");
            }
            "city" | "city name" => {
                return Some("geography.location.city");
            }
            "state" | "province" => {
                return Some("geography.location.state");
            }
            // "region" removed — model predicts region correctly; exact-matching it
            // to state was wrong for datasets where subcountry values are regions
            // (v15, Option C keyword audit).
            "currency" | "currency code" => {
                return Some("identity.financial.currency_code");
            }
            // "id" | "identifier" intentionally unhinted — ID columns are genuinely
            // ambiguous (numeric, UUID, alphanumeric). Let the model decide from values.
            // Count / frequency columns — small integers representing quantities
            "sibsp" | "parch" | "siblings" | "parents" | "children" | "dependents" | "qty"
            | "quantity" => {
                return Some("representation.numeric.integer_number");
            }
            // Class / rank / tier columns — ordinal categories
            "class" | "pclass" | "grade" | "rank" | "level" | "tier" | "rating" | "priority"
            | "score" => {
                return Some("representation.discrete.ordinal");
            }
            // Survival / binary outcome columns
            "survived" | "alive" | "deceased" | "dead" | "active" | "enabled" | "disabled"
            | "deleted" | "verified" | "approved" | "flagged" => {
                return Some("representation.boolean.binary");
            }
            // UTC / timezone offset columns
            "utc offset" | "gmt offset" | "timezone offset" | "tz offset" | "utcoffset"
            | "gmtoffset" => {
                return Some("datetime.offset.utc");
            }
            // IANA timezone columns
            "timezone" | "tz" | "time zone" | "iana timezone" | "iana tz" | "olson timezone" => {
                return Some("datetime.offset.iana");
            }
            // Publisher / publishing
            // Company / venue / station — entity names confused with city
            "publisher" | "publishing house" | "published by" | "company" | "employer"
            | "organization" | "organisation" | "venue" | "stadium" | "arena" | "theater"
            | "theatre" | "station" | "station name" | "facility" | "building" | "hotel"
            | "restaurant" | "hospital" | "school" | "university" | "manufacturer" => {
                return Some("representation.text.entity_name");
            }
            // Financial code columns
            "swift" | "swift code" | "bic" | "bic code" | "swiftcode" | "biccode" => {
                return Some("finance.banking.swift_bic");
            }
            "issn" => {
                return Some("identity.commerce.issn");
            }
            // ICAO airport code — unambiguous (stopgap per decision 0042)
            "icao" | "icao code" => {
                return Some("geography.transportation.icao_code");
            }
            // Author — unambiguous person name (stopgap per decision 0042)
            "author" | "authors" | "author name" => {
                return Some("identity.person.full_name");
            }
            // Medical identifiers
            "npi" | "npi number" => {
                return Some("identity.medical.npi");
            }
            "ean" | "barcode" | "gtin" => {
                return Some("identity.commerce.ean");
            }
            "upc" => {
                return Some("identity.commerce.upc");
            }
            // Operating system
            "os" | "operating system" | "platform" => {
                return Some("technology.development.os");
            }
            // Occupation / job title — collapsed to categorical
            "occupation" | "job title" | "jobtitle" | "job" | "profession" | "role"
            | "position" => {
                return Some("representation.discrete.categorical");
            }
            // Subcountry / subregion — removed (v15, Option C keyword audit).
            // Model predicts region correctly for subcountry columns; these exact
            // matches forced state, which was wrong for world_cities/subcountry.
            // Let the model decide from values.
            // Embarked / boarding columns — categorical
            "embarked" | "boarded" | "departed" | "terminal" | "gate" => {
                return Some("representation.discrete.categorical");
            }
            // Ticket / cabin — alphanumeric identifiers
            "ticket" | "ticket number" | "ticketno" => {
                return Some("representation.alphanumeric.alphanumeric_id");
            }
            "cabin" | "room" | "compartment" | "berth" | "seat" => {
                return Some("representation.alphanumeric.alphanumeric_id");
            }
            // Fare / fee columns (→ finance.currency.amount)
            "fare" | "fee" | "toll" | "charge" => {
                return Some("finance.currency.amount");
            }
            // Amount-variant exact matches (decision 0065).
            // These MUST precede the generic `h.contains("amount")` substring
            // matcher below — otherwise every variant header like `amount_comma`
            // collapses to the plain `finance.currency.amount` label. See spec
            // .orbit/specs/2026-04-24-amount-variant-generators ac-01..04 for the
            // diagnostic arc. Header normalisation replaces `_`/`-` with space.
            "amount accounting" | "amount acc" => {
                return Some("finance.currency.amount_accounting");
            }
            "amount apostrophe" => {
                return Some("finance.currency.amount_apostrophe");
            }
            "amount code prefix" | "amount code" | "amount with code" => {
                return Some("finance.currency.amount_code_prefix");
            }
            "amount comma" => {
                return Some("finance.currency.amount_comma");
            }
            "amount comma suffix" => {
                return Some("finance.currency.amount_comma_suffix");
            }
            "amount crypto" | "crypto amount" => {
                return Some("finance.currency.amount_crypto");
            }
            "amount lakh" | "lakh amount" | "amount indian" => {
                return Some("finance.currency.amount_lakh");
            }
            "amount multisym" | "amount multi sym" | "amount multiple symbols" => {
                return Some("finance.currency.amount_multisym");
            }
            "amount neg trailing" | "amount negative trailing" => {
                return Some("finance.currency.amount_neg_trailing");
            }
            "amount nodecimal" | "amount no decimal" => {
                return Some("finance.currency.amount_nodecimal");
            }
            "amount space" => {
                return Some("finance.currency.amount_space");
            }
            _ => {}
        }
    }

    // Keyword/substring matching (less specific)
    // Guard: "email_display" headers should NOT match — the model correctly
    // identifies email_display from value patterns (v15, Option C keyword audit).
    if !disable_identity && (h.contains("email") || h.contains("e mail")) && !h.contains("display")
    {
        return Some("identity.person.email");
    }
    // Guard: "phone_e164" headers should NOT match — the model correctly
    // identifies E.164 from value patterns (v15, Option C keyword audit).
    if !disable_identity
        && (h.contains("phone") || h.contains("tel") || h.contains("mobile") || h.contains("fax"))
        && !h.contains("e164")
    {
        return Some("identity.person.phone_number");
    }
    // IPv6 — check before the generic ip_v4 catch-all so "ip_v6", "server_ipv6",
    // "ipv6_address" all route correctly. Exact "ip" / "ip address" etc. handled
    // above in the exact-match arm (→ ip_v4).
    if !disable_technology && (h.contains("v6") || h.contains("ipv6")) {
        return Some("technology.internet.ip_v6");
    }
    // IP address — match " ip" suffix, "ip " prefix, or " ip " infix
    // (underscores already replaced with spaces, exact "ip" handled above)
    // Guard: "ip_v4_with_port" headers should NOT match — the model correctly
    // identifies ip_v4_with_port from value patterns (v15, Option C keyword audit).
    if !disable_technology
        && (h.ends_with(" ip") || h.starts_with("ip ") || h.contains(" ip "))
        && !h.contains("port")
    {
        return Some("technology.internet.ip_v4");
    }
    if !disable_geography && (h.contains("zip") || h.contains("postal") || h.contains("postcode")) {
        return Some("geography.address.postal_code");
    }
    if !disable_identity && h.contains("name") && (h.contains("first") || h.contains("given")) {
        return Some("identity.person.first_name");
    }
    if !disable_identity
        && h.contains("name")
        && (h.contains("last") || h.contains("family") || h.contains("sur"))
    {
        return Some("identity.person.last_name");
    }
    if !disable_identity && h.contains("name") && (h.contains("full") || h.contains("complete")) {
        return Some("identity.person.full_name");
    }
    if !disable_identity && h.ends_with(" name") && h != "name" {
        // Qualified: "display name", "user name", "account name" → full_name
        // Bare "name" is ambiguous (could be person, city, country, company) — let
        // Sense + CharCNN decide based on value evidence.
        // Exclude datetime component names (month_name, day_name) — those are
        // temporal, not person names.
        if h.contains("month") || h.contains("day") || h.contains("weekday") {
            // Let the model decide — these are datetime components
        } else {
            return Some("identity.person.full_name");
        }
    }
    if !disable_geography
        && h.contains("address")
        && !h.contains("email")
        && !h.contains("ip")
        && !h.contains("mac")
        && !h.contains("bitcoin")
        && !h.contains("btc")
        && !h.contains("crypto")
        && !h.contains("wallet")
    {
        // "street address" or "street_address" → street_address
        // "full address" → full_address
        // Bare "address" → full_address (more commonly means full address)
        if h.contains("street") {
            return Some("geography.address.street_address");
        }
        return Some("geography.address.full_address");
    }
    if !disable_geography && h.contains("street") {
        return Some("geography.address.street_address");
    }
    if !disable_datetime && (h.contains("born") || h.contains("birth") || h.contains("dob")) {
        return Some("datetime.date.iso");
    }
    // Specific timestamp formats — must precede the generic date/timestamp catch-all.
    // Note: underscores are already replaced with spaces by header normalization.
    if !disable_datetime && (h.contains("rfc 2822") || h.contains("rfc2822")) {
        return Some("datetime.timestamp.rfc_2822");
    }
    if !disable_datetime && (h.contains("rfc 3339") || h.contains("rfc3339")) {
        return Some("datetime.timestamp.rfc_3339");
    }
    if !disable_datetime && h.contains("sql") && (h.contains("timestamp") || h.contains("datetime"))
    {
        return Some("datetime.timestamp.sql_standard");
    }
    // Epoch/Unix timestamp substrings — must precede generic date/timestamp catch-all.
    // "timestamp_unix", "unix_timestamp_ms", "epoch_created" etc.
    if !disable_datetime && (h.contains("epoch") || h.contains("unix")) {
        if h.contains("ms") || h.contains("milli") {
            return Some("datetime.epoch.unix_milliseconds");
        }
        return Some("datetime.epoch.unix_seconds");
    }
    if !disable_datetime
        && (h.contains("date") || h.contains("timestamp") || h.contains("datetime"))
        && !h.contains("month")
    {
        // Guard: headers containing "month" (e.g. "abbreviated_month_date",
        // "long_full_month_date") are specific date formats, not generic iso_8601.
        // Let the model decide those from values.
        return Some("datetime.timestamp.iso_8601");
    }
    if !disable_datetime && h.contains("year") {
        return Some("datetime.component.year");
    }
    if !disable_identity && h.contains("weight") {
        return Some("identity.person.weight");
    }
    if !disable_identity && h.contains("height") {
        return Some("identity.person.height");
    }
    if !disable_identity && (h.contains("password") || h.contains("passwd")) {
        return Some("identity.credential.password");
    }
    if !disable_technology && (h.contains("url") || h.contains("link") || h.contains("href")) {
        return Some("technology.internet.url");
    }
    if !disable_finance
        && (h.contains("price")
            || h.contains("cost")
            || h.contains("amount")
            || h.contains("salary")
            || h.contains("revenue")
            || h.contains("income")
            || h.contains("wage")
            || h.contains("budget")
            || h.contains("expense"))
    {
        return Some("finance.currency.amount");
    }
    // "count" must not match "country" — use word boundary check
    if !disable_representation
        && ((h.contains("count") && !h.contains("country") && !h.contains("county"))
            || h.contains("quantity")
            || h.contains("num ")
            || h.starts_with("num")
            || h.ends_with(" num"))
    {
        return Some("representation.numeric.integer_number");
    }
    if !disable_representation
        && (h.contains("class") || h.contains("grade") || h.contains("rank") || h.contains("tier"))
    {
        return Some("representation.discrete.ordinal");
    }
    if !disable_representation
        && (h.contains("ticket") || h.contains("cabin") || h.contains("seat") || h.contains("room"))
    {
        return Some("representation.alphanumeric.alphanumeric_id");
    }
    if !disable_finance
        && (h.contains("fare") || h.contains("fee") || h.contains("charge") || h.contains("toll"))
    {
        return Some("finance.currency.amount");
    }
    // Scientific / engineering measurement keywords → decimal_number.
    // Numeric measurement columns like "pressure_atm" get misclassified as
    // latitude when values fall in [-90, 90]. Header keywords disambiguate.
    if !disable_representation
        && (h.contains("pressure")
            || h.contains("temperature")
            || h.contains("voltage")
            || h.contains("density")
            || h.contains("velocity")
            || h.contains("acceleration")
            || h.contains("frequency")
            || h.contains("resistance")
            || h.contains("capacitance")
            || h.contains("inductance"))
    {
        return Some("representation.numeric.decimal_number");
    }
    // Latency / elapsed / duration — numeric measurements
    // "response time" removed — model correctly handles both integer and decimal
    // response time columns (v15, Option C keyword audit).
    // "duration" excluded when it looks like ISO 8601 (duration_iso, duration_8601)
    if !disable_representation
        && (h.contains("latency")
            || h.contains("elapsed")
            || (h.contains("duration") && !h.contains("iso") && !h.contains("8601")))
    {
        return Some("representation.numeric.integer_number");
    }
    // Payload / byte size — numeric measurements
    if !disable_representation
        && (h.contains("payload") || h.contains("bytes") || h.contains("size"))
    {
        return Some("representation.numeric.integer_number");
    }

    None
}
