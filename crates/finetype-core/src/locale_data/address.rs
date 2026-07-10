/// Get street suffixes for a locale.
pub fn street_suffixes(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => &[
            "Street",
            "Avenue",
            "Boulevard",
            "Road",
            "Lane",
            "Drive",
            "Court",
            "Way",
            "Circle",
            "Place",
            "St",
            "Ave",
            "Blvd",
            "Rd",
            "Ln",
            "Dr",
        ],
        "DE" => &[
            "Straße", "Weg", "Gasse", "Allee", "Platz", "Ring", "Damm", "Steig", "Pfad", "Str.",
        ],
        "FR" => &[
            "Rue",
            "Avenue",
            "Boulevard",
            "Place",
            "Chemin",
            "Allée",
            "Impasse",
            "Passage",
            "Quai",
            "Cours",
        ],
        "ES" => &[
            "Calle",
            "Avenida",
            "Plaza",
            "Paseo",
            "Camino",
            "Carrera",
            "Vía",
            "Ronda",
            "Travesía",
            "Glorieta",
        ],
        "IT" => &[
            "Via",
            "Viale",
            "Piazza",
            "Corso",
            "Vicolo",
            "Largo",
            "Strada",
            "Piazzale",
            "Lungomare",
        ],
        "NL" => &[
            "Straat", "Weg", "Laan", "Plein", "Gracht", "Singel", "Kade", "Steeg", "Pad",
        ],
        "PL" => &[
            "ulica", "aleja", "plac", "skwer", "rondo", "bulwar", "droga", "szosa",
        ],
        "RU" => &[
            "улица",
            "проспект",
            "переулок",
            "бульвар",
            "площадь",
            "набережная",
            "шоссе",
            "тупик",
        ],
        _ => street_suffixes("EN"),
    }
}

/// Get street names for a locale.
pub fn street_names(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => &[
            "Main Street",
            "Oak Avenue",
            "Elm Street",
            "Park Road",
            "Broadway",
            "5th Avenue",
            "High Street",
            "King Street",
            "Queen Street",
            "Church Street",
            "Maple Drive",
            "Cedar Lane",
        ],
        "DE" => &[
            "Hauptstraße",
            "Bahnhofstraße",
            "Berliner Straße",
            "Gartenstraße",
            "Kirchstraße",
            "Schulstraße",
            "Bergstraße",
            "Waldstraße",
            "Lindenstraße",
            "Friedhofstraße",
        ],
        "FR" => &[
            "Rue de la Paix",
            "Avenue des Champs-Élysées",
            "Boulevard Saint-Germain",
            "Rue du Faubourg Saint-Honoré",
            "Place de la République",
            "Rue de Rivoli",
            "Avenue Victor Hugo",
            "Boulevard Haussmann",
        ],
        "ES" => &[
            "Calle Mayor",
            "Gran Vía",
            "Paseo de la Castellana",
            "Avenida de la Constitución",
            "Calle de Alcalá",
            "Rambla de Cataluña",
            "Calle Real",
            "Paseo del Prado",
        ],
        "IT" => &[
            "Via Roma",
            "Via Garibaldi",
            "Via Dante",
            "Corso Italia",
            "Via Mazzini",
            "Via Verdi",
            "Piazza della Repubblica",
            "Via Nazionale",
            "Via dei Condotti",
        ],
        "NL" => &[
            "Keizersgracht",
            "Herengracht",
            "Prinsengracht",
            "Damstraat",
            "Kalverstraat",
            "Leidsestraat",
            "Westerstraat",
            "Utrechtsestraat",
        ],
        "PL" => &[
            "ulica Marszałkowska",
            "aleja Solidarności",
            "ulica Nowy Świat",
            "ulica Krakowskie Przedmieście",
            "ulica Piotrkowska",
            "ulica Floriańska",
            "aleja Mickiewicza",
        ],
        "RU" => &[
            "Невский проспект",
            "Тверская улица",
            "Арбат",
            "Кутузовский проспект",
            "Ленинградский проспект",
            "улица Пушкина",
            "проспект Мира",
            "Садовая улица",
        ],
        "JA" => &[
            "中央通り",
            "表参道",
            "銀座通り",
            "明治通り",
            "青山通り",
            "靖国通り",
            "外堀通り",
        ],
        "ZH" => &[
            "南京路",
            "长安街",
            "中山路",
            "解放路",
            "人民路",
            "建设路",
            "和平路",
        ],
        "KO" => &[
            "종로",
            "세종대로",
            "강남대로",
            "테헤란로",
            "을지로",
            "명동길",
            "삼성로",
        ],
        "AR" => &[
            "شارع الملك فهد",
            "شارع التحلية",
            "شارع العليا",
            "شارع الأمير سلطان",
            "طريق الملك عبدالعزيز",
        ],
        _ => street_names("EN"),
    }
}

/// Map locale to phone number country code for locale-specific phone generation.
pub fn phone_country_code(locale: &str) -> &'static str {
    match locale {
        "EN_US" | "EN_CA" => "+1",
        "EN_GB" => "+44",
        "EN_AU" => "+61",
        "EN" => "+1", // default EN = US
        "DE" => "+49",
        "FR" => "+33",
        "ES" => "+34",
        "IT" => "+39",
        "NL" => "+31",
        "PL" => "+48",
        "RU" => "+7",
        "JA" => "+81",
        "ZH" => "+86",
        "KO" => "+82",
        "AR" => "+966",
        "PT_BR" => "+55",
        "ES_MX" => "+52",
        "HI" => "+91",
        "TH" => "+66",
        "MY" => "+60",
        "SG" => "+65",
        "PH" => "+63",
        "ID" => "+62",
        "TW" => "+886",
        "NZ" => "+64",
        "IE" => "+353",
        "SE" => "+46",
        "NO" => "+47",
        "DK" => "+45",
        "CH" => "+41",
        "AT" => "+43",
        "BE" => "+32",
        "PT" => "+351",
        "TR" => "+90",
        "IL" => "+972",
        "GR" => "+30",
        "ZA" => "+27",
        "NG" => "+234",
        "ES_CL" => "+56",
        "ES_CO" => "+57",
        "ES_AR" => "+54",
        "FI" => "+358",
        "ES_PE" => "+51",
        "HU" => "+36",
        "RO" => "+40",
        "CZ" => "+420",
        _ => "+1",
    }
}

/// Map locale to postal code format description.
/// Returns (pattern_type, example) for generation.
pub fn postal_format(locale: &str) -> &'static str {
    match locale {
        // Existing formats
        "EN_US" | "EN" => "US", // 5 digits or ZIP+4
        "EN_GB" => "UK",        // A9 9AA or A9A 9AA
        "EN_AU" => "AU",        // 4 digits
        "EN_CA" => "CA",        // A1A 1A1
        "DE" => "DE",           // 5 digits
        "FR" => "FR",           // 5 digits
        "ES" => "ES",           // 5 digits
        "IT" => "IT",           // 5 digits
        "NL" => "NL",           // 4 digits + 2 letters
        "PL" => "PL",           // XX-XXX
        "RU" => "RU",           // 6 digits
        "JA" => "JP",           // XXX-XXXX
        "ZH" => "CN",           // 6 digits
        "KO" => "KR",           // 5 digits

        // 4-digit formats
        "BG" | "BN" | "DA" | "DE_AT" | "DE_CH" | "EN_NZ" | "EN_PH" | "HU" | "LU" | "NB"
        | "NL_BE" | "SL" | "ZA" => "4D",

        // 5-digit formats
        "AR_EG" | "AR_MA" | "AR_SA" | "EN_KE" | "ES_MX" | "ET" | "FI" | "HR" | "ID" | "MS"
        | "TH" | "TR" | "UA" | "UR" => "5D",

        // 6-digit formats
        "EN_NG" | "EN_SG" | "ES_CO" | "HI" | "NG" | "RO" => "6D",

        // 3+2 digits with optional space (Czech, Slovak, Greek, Swedish)
        "CS" | "SK" | "EL" | "SV" => "CS",

        // Portugal: XXXX-XXX
        "PT" => "PT",

        // Brazil: XXXXX-XXX
        "PT_BR" => "BR",

        // Lithuania: optional LT- prefix + 5 digits
        "LT" => "LT",

        // Latvia: LV-XXXX
        "LV" => "LV",

        // Argentina: optional letter + 4 digits + optional 3 letters
        "ES_AR" => "ES_AR",

        // Chile: 7 digits
        "ES_CL" => "CL",

        // Peru: LIMA/CALLAO/numeric
        "ES_PE" => "PE",

        // Malta: AAA + 2-4 digits
        "MT" => "MT",

        // Ireland: Eircode
        "EN_IE" => "IE",

        // Taiwan: 3, 5, or 6 digits
        "ZH_TW" => "TW",

        // Israel: 5 or 7 digits
        "HE" => "HE",

        // Iceland: 3 digits
        "IS" => "IS",

        // Serbia: 5-6 digits
        "SR" => "SR",

        // Vietnam: 5-6 digits
        "VI" => "VI",

        _ => "US",
    }
}

/// Get country calling codes for a locale (the code used in the locale's region).
pub fn calling_codes(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN_US" | "EN_CA" | "EN" => &["+1"],
        "EN_GB" => &["+44"],
        "EN_AU" => &["+61"],
        "DE" => &["+49"],
        "FR" => &["+33"],
        "ES" => &["+34"],
        "IT" => &["+39"],
        "NL" => &["+31"],
        "PL" => &["+48"],
        "RU" => &["+7"],
        "JA" => &["+81"],
        "ZH" => &["+86"],
        "KO" => &["+82"],
        "AR" => &[
            "+966", "+971", "+973", "+974", "+968", "+965", "+962", "+961", "+20", "+213",
        ],
        "PT_BR" => &["+55"],
        "ES_MX" => &["+52"],
        "HI" => &["+91"],
        "TH" => &["+66"],
        "MY" => &["+60"],
        "SG" => &["+65"],
        "PH" => &["+63"],
        "ID" => &["+62"],
        "TW" => &["+886"],
        "NZ" => &["+64"],
        "IE" => &["+353"],
        "SE" => &["+46"],
        "NO" => &["+47"],
        "DK" => &["+45"],
        "CH" => &["+41"],
        "AT" => &["+43"],
        "BE" => &["+32"],
        "PT" => &["+351"],
        "TR" => &["+90"],
        "IL" => &["+972"],
        "GR" => &["+30"],
        "ZA" => &["+27"],
        "NG" => &["+234"],
        "ES_CL" => &["+56"],
        "ES_CO" => &["+57"],
        "ES_AR" => &["+54"],
        "FI" => &["+358"],
        "ES_PE" => &["+51"],
        "HU" => &["+36"],
        "RO" => &["+40"],
        "CZ" => &["+420"],
        _ => &[
            "+1", "+44", "+33", "+49", "+81", "+86", "+91", "+61", "+55", "+82",
        ],
    }
}

/// Get the base locale for regional variants (e.g., EN_AU -> EN for name data).
pub fn base_locale(locale: &str) -> &str {
    if locale.starts_with("EN_") {
        "EN"
    } else {
        locale
    }
}
