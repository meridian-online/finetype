/// Get month names for a locale.
pub fn month_names(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => &[
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ],
        "DE" => &[
            "Januar",
            "Februar",
            "März",
            "April",
            "Mai",
            "Juni",
            "Juli",
            "August",
            "September",
            "Oktober",
            "November",
            "Dezember",
        ],
        "FR" => &[
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ],
        "ES" => &[
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ],
        "IT" => &[
            "gennaio",
            "febbraio",
            "marzo",
            "aprile",
            "maggio",
            "giugno",
            "luglio",
            "agosto",
            "settembre",
            "ottobre",
            "novembre",
            "dicembre",
        ],
        "NL" => &[
            "januari",
            "februari",
            "maart",
            "april",
            "mei",
            "juni",
            "juli",
            "augustus",
            "september",
            "oktober",
            "november",
            "december",
        ],
        "PL" => &[
            "styczeń",
            "luty",
            "marzec",
            "kwiecień",
            "maj",
            "czerwiec",
            "lipiec",
            "sierpień",
            "wrzesień",
            "październik",
            "listopad",
            "grudzień",
        ],
        "RU" => &[
            "январь",
            "февраль",
            "март",
            "апрель",
            "май",
            "июнь",
            "июль",
            "август",
            "сентябрь",
            "октябрь",
            "ноябрь",
            "декабрь",
        ],
        "JA" => &[
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
        "ZH" => &[
            "一月",
            "二月",
            "三月",
            "四月",
            "五月",
            "六月",
            "七月",
            "八月",
            "九月",
            "十月",
            "十一月",
            "十二月",
        ],
        "KO" => &[
            "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
        ],
        "AR" => &[
            "يناير",
            "فبراير",
            "مارس",
            "أبريل",
            "مايو",
            "يونيو",
            "يوليو",
            "أغسطس",
            "سبتمبر",
            "أكتوبر",
            "نوفمبر",
            "ديسمبر",
        ],
        // ── CLDR-sourced locales ──────────────────────────────
        "BG" => &[
            "януари",
            "февруари",
            "март",
            "април",
            "май",
            "юни",
            "юли",
            "август",
            "септември",
            "октомври",
            "ноември",
            "декември",
        ],
        "CS" => &[
            "leden",
            "únor",
            "březen",
            "duben",
            "květen",
            "červen",
            "červenec",
            "srpen",
            "září",
            "říjen",
            "listopad",
            "prosinec",
        ],
        "DA" => &[
            "januar",
            "februar",
            "marts",
            "april",
            "maj",
            "juni",
            "juli",
            "august",
            "september",
            "oktober",
            "november",
            "december",
        ],
        "EL" => &[
            "Ιανουάριος",
            "Φεβρουάριος",
            "Μάρτιος",
            "Απρίλιος",
            "Μάιος",
            "Ιούνιος",
            "Ιούλιος",
            "Αύγουστος",
            "Σεπτέμβριος",
            "Οκτώβριος",
            "Νοέμβριος",
            "Δεκέμβριος",
        ],
        "ET" => &[
            "jaanuar",
            "veebruar",
            "märts",
            "aprill",
            "mai",
            "juuni",
            "juuli",
            "august",
            "september",
            "oktoober",
            "november",
            "detsember",
        ],
        "FI" => &[
            "tammikuu",
            "helmikuu",
            "maaliskuu",
            "huhtikuu",
            "toukokuu",
            "kesäkuu",
            "heinäkuu",
            "elokuu",
            "syyskuu",
            "lokakuu",
            "marraskuu",
            "joulukuu",
        ],
        "HR" => &[
            "siječanj",
            "veljača",
            "ožujak",
            "travanj",
            "svibanj",
            "lipanj",
            "srpanj",
            "kolovoz",
            "rujan",
            "listopad",
            "studeni",
            "prosinac",
        ],
        "HU" => &[
            "január",
            "február",
            "március",
            "április",
            "május",
            "június",
            "július",
            "augusztus",
            "szeptember",
            "október",
            "november",
            "december",
        ],
        "LT" => &[
            "sausis",
            "vasaris",
            "kovas",
            "balandis",
            "gegužė",
            "birželis",
            "liepa",
            "rugpjūtis",
            "rugsėjis",
            "spalis",
            "lapkritis",
            "gruodis",
        ],
        "LV" => &[
            "janvāris",
            "februāris",
            "marts",
            "aprīlis",
            "maijs",
            "jūnijs",
            "jūlijs",
            "augusts",
            "septembris",
            "oktobris",
            "novembris",
            "decembris",
        ],
        "NO" => &[
            "januar",
            "februar",
            "mars",
            "april",
            "mai",
            "juni",
            "juli",
            "august",
            "september",
            "oktober",
            "november",
            "desember",
        ],
        "PT" | "PT_BR" => &[
            "janeiro",
            "fevereiro",
            "março",
            "abril",
            "maio",
            "junho",
            "julho",
            "agosto",
            "setembro",
            "outubro",
            "novembro",
            "dezembro",
        ],
        "RO" => &[
            "ianuarie",
            "februarie",
            "martie",
            "aprilie",
            "mai",
            "iunie",
            "iulie",
            "august",
            "septembrie",
            "octombrie",
            "noiembrie",
            "decembrie",
        ],
        "SK" => &[
            "január",
            "február",
            "marec",
            "apríl",
            "máj",
            "jún",
            "júl",
            "august",
            "september",
            "október",
            "november",
            "december",
        ],
        "SL" => &[
            "januar",
            "februar",
            "marec",
            "april",
            "maj",
            "junij",
            "julij",
            "avgust",
            "september",
            "oktober",
            "november",
            "december",
        ],
        "SV" => &[
            "januari",
            "februari",
            "mars",
            "april",
            "maj",
            "juni",
            "juli",
            "augusti",
            "september",
            "oktober",
            "november",
            "december",
        ],
        "TR" => &[
            "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül",
            "Ekim", "Kasım", "Aralık",
        ],
        "UK" => &[
            "січень",
            "лютий",
            "березень",
            "квітень",
            "травень",
            "червень",
            "липень",
            "серпень",
            "вересень",
            "жовтень",
            "листопад",
            "грудень",
        ],
        _ => month_names("EN"),
    }
}

/// Get abbreviated month names for a locale.
pub fn month_abbreviations(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => &[
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ],
        "DE" => &[
            "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
        ],
        "FR" => &[
            "janv", "févr", "mars", "avr", "mai", "juin", "juil", "août", "sept", "oct", "nov",
            "déc",
        ],
        "ES" => &[
            "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
        ],
        "IT" => &[
            "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
        ],
        "NL" => &[
            "jan", "feb", "mrt", "apr", "mei", "jun", "jul", "aug", "sep", "okt", "nov", "dec",
        ],
        "PL" => &[
            "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
        ],
        "RU" => &[
            "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
        ],
        "JA" => &[
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
        "ZH" => &[
            "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
        ],
        "KO" => &[
            "1월", "2월", "3월", "4월", "5월", "6월", "7월", "8월", "9월", "10월", "11월", "12월",
        ],
        "AR" => &[
            "يناير",
            "فبراير",
            "مارس",
            "أبريل",
            "مايو",
            "يونيو",
            "يوليو",
            "أغسطس",
            "سبتمبر",
            "أكتوبر",
            "نوفمبر",
            "ديسمبر",
        ],
        // ── CLDR-sourced locales ──────────────────────────────
        "BG" => &[
            "яну", "фев", "март", "апр", "май", "юни", "юли", "авг", "сеп", "окт", "ное", "дек",
        ],
        "CS" => &[
            "led", "úno", "bře", "dub", "kvě", "čvn", "čvc", "srp", "zář", "říj", "lis", "pro",
        ],
        "DA" => &[
            "jan.", "feb.", "mar.", "apr.", "maj", "jun.", "jul.", "aug.", "sep.", "okt.", "nov.",
            "dec.",
        ],
        "EL" => &[
            "Ιαν", "Φεβ", "Μάρ", "Απρ", "Μάι", "Ιούν", "Ιούλ", "Αύγ", "Σεπ", "Οκτ", "Νοέ", "Δεκ",
        ],
        "ET" => &[
            "jaan", "veebr", "märts", "apr", "mai", "juuni", "juuli", "aug", "sept", "okt", "nov",
            "dets",
        ],
        "FI" => &[
            "tammi", "helmi", "maalis", "huhti", "touko", "kesä", "heinä", "elo", "syys", "loka",
            "marras", "joulu",
        ],
        "HR" => &[
            "sij", "velj", "ožu", "tra", "svi", "lip", "srp", "kol", "ruj", "lis", "stu", "pro",
        ],
        "HU" => &[
            "jan.", "febr.", "márc.", "ápr.", "máj.", "jún.", "júl.", "aug.", "szept.", "okt.",
            "nov.", "dec.",
        ],
        "LT" => &[
            "saus.", "vas.", "kov.", "bal.", "geg.", "birž.", "liep.", "rugp.", "rugs.", "spal.",
            "lapkr.", "gruod.",
        ],
        "LV" => &[
            "janv.", "febr.", "marts", "apr.", "maijs", "jūn.", "jūl.", "aug.", "sept.", "okt.",
            "nov.", "dec.",
        ],
        "NO" => &[
            "jan", "feb", "mar", "apr", "mai", "jun", "jul", "aug", "sep", "okt", "nov", "des",
        ],
        "PT" | "PT_BR" => &[
            "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.",
            "dez.",
        ],
        "RO" => &[
            "ian.", "feb.", "mar.", "apr.", "mai", "iun.", "iul.", "aug.", "sept.", "oct.", "nov.",
            "dec.",
        ],
        "SK" => &[
            "jan", "feb", "mar", "apr", "máj", "jún", "júl", "aug", "sep", "okt", "nov", "dec",
        ],
        "SL" => &[
            "jan.", "feb.", "mar.", "apr.", "maj", "jun.", "jul.", "avg.", "sep.", "okt.", "nov.",
            "dec.",
        ],
        "SV" => &[
            "jan.", "feb.", "mars", "apr.", "maj", "juni", "juli", "aug.", "sep.", "okt.", "nov.",
            "dec.",
        ],
        "TR" => &[
            "Oca", "Şub", "Mar", "Nis", "May", "Haz", "Tem", "Ağu", "Eyl", "Eki", "Kas", "Ara",
        ],
        "UK" => &[
            "січ.",
            "лют.",
            "бер.",
            "квіт.",
            "трав.",
            "черв.",
            "лип.",
            "серп.",
            "вер.",
            "жовт.",
            "лист.",
            "груд.",
        ],
        _ => month_abbreviations("EN"),
    }
}

/// Get weekday names for a locale.
pub fn weekday_names(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => &[
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ],
        "DE" => &[
            "Montag",
            "Dienstag",
            "Mittwoch",
            "Donnerstag",
            "Freitag",
            "Samstag",
            "Sonntag",
        ],
        "FR" => &[
            "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
        ],
        "ES" => &[
            "lunes",
            "martes",
            "miércoles",
            "jueves",
            "viernes",
            "sábado",
            "domingo",
        ],
        "IT" => &[
            "lunedì",
            "martedì",
            "mercoledì",
            "giovedì",
            "venerdì",
            "sabato",
            "domenica",
        ],
        "NL" => &[
            "maandag",
            "dinsdag",
            "woensdag",
            "donderdag",
            "vrijdag",
            "zaterdag",
            "zondag",
        ],
        "PL" => &[
            "poniedziałek",
            "wtorek",
            "środa",
            "czwartek",
            "piątek",
            "sobota",
            "niedziela",
        ],
        "RU" => &[
            "понедельник",
            "вторник",
            "среда",
            "четверг",
            "пятница",
            "суббота",
            "воскресенье",
        ],
        "JA" => &[
            "月曜日",
            "火曜日",
            "水曜日",
            "木曜日",
            "金曜日",
            "土曜日",
            "日曜日",
        ],
        "ZH" => &[
            "星期一",
            "星期二",
            "星期三",
            "星期四",
            "星期五",
            "星期六",
            "星期日",
        ],
        "KO" => &[
            "월요일",
            "화요일",
            "수요일",
            "목요일",
            "금요일",
            "토요일",
            "일요일",
        ],
        "AR" => &[
            "الاثنين",
            "الثلاثاء",
            "الأربعاء",
            "الخميس",
            "الجمعة",
            "السبت",
            "الأحد",
        ],
        // ── CLDR-sourced locales ──────────────────────────────
        "BG" => &[
            "понеделник",
            "вторник",
            "сряда",
            "четвъртък",
            "петък",
            "събота",
            "неделя",
        ],
        "CS" => &[
            "pondělí",
            "úterý",
            "středa",
            "čtvrtek",
            "pátek",
            "sobota",
            "neděle",
        ],
        "DA" => &[
            "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag", "søndag",
        ],
        "EL" => &[
            "Δευτέρα",
            "Τρίτη",
            "Τετάρτη",
            "Πέμπτη",
            "Παρασκευή",
            "Σάββατο",
            "Κυριακή",
        ],
        "ET" => &[
            "esmaspäev",
            "teisipäev",
            "kolmapäev",
            "neljapäev",
            "reede",
            "laupäev",
            "pühapäev",
        ],
        "FI" => &[
            "maanantai",
            "tiistai",
            "keskiviikko",
            "torstai",
            "perjantai",
            "lauantai",
            "sunnuntai",
        ],
        "HR" => &[
            "ponedjeljak",
            "utorak",
            "srijeda",
            "četvrtak",
            "petak",
            "subota",
            "nedjelja",
        ],
        "HU" => &[
            "hétfő",
            "kedd",
            "szerda",
            "csütörtök",
            "péntek",
            "szombat",
            "vasárnap",
        ],
        "LT" => &[
            "pirmadienis",
            "antradienis",
            "trečiadienis",
            "ketvirtadienis",
            "penktadienis",
            "šeštadienis",
            "sekmadienis",
        ],
        "LV" => &[
            "pirmdiena",
            "otrdiena",
            "trešdiena",
            "ceturtdiena",
            "piektdiena",
            "sestdiena",
            "svētdiena",
        ],
        "NO" => &[
            "mandag", "tirsdag", "onsdag", "torsdag", "fredag", "lørdag", "søndag",
        ],
        "PT" | "PT_BR" => &[
            "segunda-feira",
            "terça-feira",
            "quarta-feira",
            "quinta-feira",
            "sexta-feira",
            "sábado",
            "domingo",
        ],
        "RO" => &[
            "luni",
            "marți",
            "miercuri",
            "joi",
            "vineri",
            "sâmbătă",
            "duminică",
        ],
        "SK" => &[
            "pondelok", "utorok", "streda", "štvrtok", "piatok", "sobota", "nedeľa",
        ],
        "SL" => &[
            "ponedeljek",
            "torek",
            "sreda",
            "četrtek",
            "petek",
            "sobota",
            "nedelja",
        ],
        "SV" => &[
            "måndag", "tisdag", "onsdag", "torsdag", "fredag", "lördag", "söndag",
        ],
        "TR" => &[
            "Pazartesi",
            "Salı",
            "Çarşamba",
            "Perşembe",
            "Cuma",
            "Cumartesi",
            "Pazar",
        ],
        "UK" => &[
            "понеділок",
            "вівторок",
            "середа",
            "четвер",
            "пʼятниця",
            "субота",
            "неділя",
        ],
        _ => weekday_names("EN"),
    }
}

/// Get weekday abbreviations for a locale.
pub fn weekday_abbreviations(locale: &str) -> &'static [&'static str] {
    match locale {
        "EN" | "EN_US" | "EN_AU" | "EN_GB" | "EN_CA" => {
            &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        }
        "DE" => &["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
        "FR" => &["lun", "mar", "mer", "jeu", "ven", "sam", "dim"],
        "ES" => &["lun", "mar", "mié", "jue", "vie", "sáb", "dom"],
        "IT" => &["lun", "mar", "mer", "gio", "ven", "sab", "dom"],
        "NL" => &["ma", "di", "wo", "do", "vr", "za", "zo"],
        "PL" => &["pon", "wt", "śr", "czw", "pt", "sob", "nie"],
        "RU" => &["пн", "вт", "ср", "чт", "пт", "сб", "вс"],
        "JA" => &["月", "火", "水", "木", "金", "土", "日"],
        "ZH" => &["一", "二", "三", "四", "五", "六", "日"],
        "KO" => &["월", "화", "수", "목", "금", "토", "일"],
        "AR" => &["إثن", "ثلث", "أرب", "خمس", "جمع", "سبت", "أحد"],
        // ── CLDR-sourced locales ──────────────────────────────
        "BG" => &["пн", "вт", "ср", "чт", "пт", "сб", "нд"],
        "CS" => &["po", "út", "st", "čt", "pá", "so", "ne"],
        "DA" => &["man.", "tirs.", "ons.", "tors.", "fre.", "lør.", "søn."],
        "EL" => &["Δευ", "Τρί", "Τετ", "Πέμ", "Παρ", "Σάβ", "Κυρ"],
        "ET" => &["E", "T", "K", "N", "R", "L", "P"],
        "FI" => &["ma", "ti", "ke", "to", "pe", "la", "su"],
        "HR" => &["pon", "uto", "sri", "čet", "pet", "sub", "ned"],
        "HU" => &["H", "K", "Sze", "Cs", "P", "Szo", "V"],
        "LT" => &["pr", "an", "tr", "kt", "pn", "št", "sk"],
        "LV" => &[
            "pirmd.", "otrd.", "trešd.", "ceturtd.", "piektd.", "sestd.", "svētd.",
        ],
        "NO" => &["man.", "tir.", "ons.", "tor.", "fre.", "lør.", "søn."],
        "PT" => &["seg.", "ter.", "qua.", "qui.", "sex.", "sáb.", "dom."],
        "PT_BR" => &["seg.", "ter.", "qua.", "qui.", "sex.", "sáb.", "dom."],
        "RO" => &["lun.", "mar.", "mie.", "joi", "vin.", "sâm.", "dum."],
        "SK" => &["po", "ut", "st", "št", "pi", "so", "ne"],
        "SL" => &["pon.", "tor.", "sre.", "čet.", "pet.", "sob.", "ned."],
        "SV" => &["mån", "tis", "ons", "tors", "fre", "lör", "sön"],
        "TR" => &["Pzt", "Sal", "Çar", "Per", "Cum", "Cmt", "Paz"],
        "UK" => &["пн", "вт", "ср", "чт", "пт", "сб", "нд"],
        _ => weekday_abbreviations("EN"),
    }
}

// ---------------------------------------------------------------------------
// CLDR-sourced date format patterns (data/cldr/cldr_date_patterns.tsv)
// ---------------------------------------------------------------------------

/// Date field ordering in locale-specific date formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFieldOrder {
    /// Month Day Year — "January 15, 2024" (en-US, en-CA)
    MonthDayYear,
    /// Day Month Year — "15 January 2024" (most locales)
    DayMonthYear,
    /// Year Month Day — "2024. január 15." (hu) / "2024 sausio 15" (lt)
    YearMonthDay,
    /// Year Day Month — "2024. gada 15. janvāris" (lv)
    YearDayMonth,
}

/// CLDR-sourced date format pattern for locale-specific generation.
///
/// Separators and suffixes are placed according to the field order:
/// - **MDY:** `{month}{day_month_sep}{day}{day_suffix}{month_year_sep}{year}{year_suffix}`
/// - **DMY:** `{day}{day_suffix}{day_month_sep}{month}{month_year_sep}{year}{year_suffix}`
/// - **YMD:** `{year}{year_suffix}{month_year_sep}{month}{day_month_sep}{day}{day_suffix}`
/// - **YDM:** `{year}{year_suffix}{month_year_sep}{day}{day_suffix}{day_month_sep}{month}`
#[derive(Debug, Clone, Copy)]
pub struct DateFormatPattern {
    /// Component ordering
    pub order: DateFieldOrder,
    /// Separator between day and month components
    pub day_month_sep: &'static str,
    /// Separator between month and year components (or year-to-day for YDM)
    pub month_year_sep: &'static str,
    /// Suffix after day number (e.g., "." for Czech/German)
    pub day_suffix: &'static str,
    /// Suffix after year number (e.g., "." for Hungarian)
    pub year_suffix: &'static str,
}

// -- Pattern constants derived from data/cldr/cldr_date_patterns.tsv --

/// DMY with simple spaces: "15 January 2024"
const DMY: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::DayMonthYear,
    day_month_sep: " ",
    month_year_sep: " ",
    day_suffix: "",
    year_suffix: "",
};

/// DMY with period after day: "15. März 2024" (de, cs, da, et, fi, hr, no, sk, sl)
const DMY_DOT: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::DayMonthYear,
    day_month_sep: " ",
    month_year_sep: " ",
    day_suffix: ".",
    year_suffix: "",
};

/// DMY with "de" prepositions: "15 de enero de 2024" (es full month, pt)
const DMY_DE: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::DayMonthYear,
    day_month_sep: " de ",
    month_year_sep: " de ",
    day_suffix: "",
    year_suffix: "",
};

/// MDY with comma before year: "January 15, 2024" (en, en-US, en-CA)
const MDY_COMMA: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::MonthDayYear,
    day_month_sep: " ",
    month_year_sep: ", ",
    day_suffix: "",
    year_suffix: "",
};

/// YMD Hungarian: "2024. január 15." — periods after year and day
const YMD_HU: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::YearMonthDay,
    day_month_sep: " ",
    month_year_sep: " ",
    day_suffix: ".",
    year_suffix: ".",
};

/// YMD Lithuanian: "2024 sausio 15" — no suffixes
const YMD_LT: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::YearMonthDay,
    day_month_sep: " ",
    month_year_sep: " ",
    day_suffix: "",
    year_suffix: "",
};

/// YDM Latvian: "2024. gada 15. janvāris"
const YDM_LV: DateFormatPattern = DateFormatPattern {
    order: DateFieldOrder::YearDayMonth,
    day_month_sep: " ",
    month_year_sep: " gada ",
    day_suffix: ".",
    year_suffix: ".",
};

/// Get the CLDR-sourced date format pattern for a locale.
///
/// `full_month`: true for `long_full_month`/`weekday_full_month`,
///               false for `abbreviated_month`/`weekday_abbreviated_month`.
///
/// The distinction matters for locales like Spanish where full-month dates use
/// "de" prepositions ("15 de enero de 2024") but abbreviated dates don't
/// ("15 ene 2024").
pub fn date_format_pattern(locale: &str, full_month: bool) -> DateFormatPattern {
    match locale {
        // MDY: US-style
        "EN" | "EN_US" | "EN_CA" => MDY_COMMA,
        // DMY with "de" preposition: Spanish (full month only), Portuguese (both)
        "ES" if full_month => DMY_DE,
        "PT" | "PT_BR" => DMY_DE,
        // DMY with period after day: Germanic and Central European
        "DE" | "CS" | "DA" | "ET" | "FI" | "HR" | "NO" | "SK" | "SL" => DMY_DOT,
        // YMD: Hungarian (with periods)
        "HU" => YMD_HU,
        // YMD: Lithuanian (plain)
        "LT" => YMD_LT,
        // YDM: Latvian
        "LV" => YDM_LV,
        // DMY simple: EN_AU, EN_GB, FR, IT, NL, PL, RU, AR, BG, EL, RO, SV, TR, UK, ...
        _ => DMY,
    }
}

/// Get weekday placement and separator for weekday date patterns.
///
/// Returns `(weekday_before_date, separator)` derived from CLDR full-length
/// date patterns.
pub fn weekday_format(locale: &str) -> (bool, &'static str) {
    match locale {
        // Weekday after date
        "HU" | "LT" => (false, ", "),
        "TR" => (false, " "),
        // Arabic comma (U+060C)
        "AR" => (true, "\u{060C} "),
        // Weekday before, space only (no comma)
        "FR" | "IT" | "NL" | "EL" | "SV" | "CS" | "DA" | "FI" | "NO" | "SK" => (true, " "),
        // Weekday before, comma + space (default)
        _ => (true, ", "),
    }
}
