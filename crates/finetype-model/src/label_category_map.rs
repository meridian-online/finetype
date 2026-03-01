//! Label → Sense BroadCategory mapping for output masking (NNFT-169).
//!
//! Maps all 163 FineType type labels to their primary `BroadCategory`.
//! Used during column classification to mask CharCNN predictions to
//! the Sense-predicted category.
//!
//! Categories:
//!   temporal (46) — all `datetime.*`
//!   numeric (14) — numeric values, measurements, small integers
//!   geographic (16) — all `geography.*`
//!   entity (9) — person names, entity names
//!   format (48) — structured identifiers, codes, sequences
//!   text (30) — free text, low-cardinality enums, categorical

use crate::sense::BroadCategory;

// ═══════════════════════════════════════════════════════════════════════════════
// Static label sets per category
// ═══════════════════════════════════════════════════════════════════════════════

const TEMPORAL_LABELS: &[&str] = &[
    "datetime.component.century",
    "datetime.component.day_of_month",
    "datetime.component.day_of_week",
    "datetime.component.month_name",
    "datetime.component.periodicity",
    "datetime.component.year",
    "datetime.date.abbreviated_month",
    "datetime.date.compact_dmy",
    "datetime.date.compact_mdy",
    "datetime.date.compact_ymd",
    "datetime.date.eu_dot",
    "datetime.date.eu_slash",
    "datetime.date.iso",
    "datetime.date.iso_week",
    "datetime.date.julian",
    "datetime.date.long_full_month",
    "datetime.date.ordinal",
    "datetime.date.short_dmy",
    "datetime.date.short_mdy",
    "datetime.date.short_ymd",
    "datetime.date.us_slash",
    "datetime.date.weekday_abbreviated_month",
    "datetime.date.weekday_full_month",
    "datetime.duration.iso_8601",
    "datetime.epoch.unix_microseconds",
    "datetime.epoch.unix_milliseconds",
    "datetime.epoch.unix_seconds",
    "datetime.offset.iana",
    "datetime.offset.utc",
    "datetime.time.hm_12h",
    "datetime.time.hm_24h",
    "datetime.time.hms_12h",
    "datetime.time.hms_24h",
    "datetime.time.iso",
    "datetime.timestamp.american",
    "datetime.timestamp.american_24h",
    "datetime.timestamp.european",
    "datetime.timestamp.iso_8601",
    "datetime.timestamp.iso_8601_compact",
    "datetime.timestamp.iso_8601_microseconds",
    "datetime.timestamp.iso_8601_offset",
    "datetime.timestamp.iso_microseconds",
    "datetime.timestamp.rfc_2822",
    "datetime.timestamp.rfc_2822_ordinal",
    "datetime.timestamp.rfc_3339",
    "datetime.timestamp.sql_standard",
];

const NUMERIC_LABELS: &[&str] = &[
    "identity.person.age",
    "identity.person.height",
    "identity.person.weight",
    "representation.file.file_size",
    "representation.numeric.decimal_number",
    "representation.numeric.increment",
    "representation.numeric.integer_number",
    "representation.numeric.percentage",
    "representation.numeric.scientific_notation",
    "representation.numeric.si_number",
    "technology.hardware.ram_size",
    "technology.hardware.screen_size",
    "technology.internet.http_status_code",
    "technology.internet.port",
];

const GEOGRAPHIC_LABELS: &[&str] = &[
    "geography.address.full_address",
    "geography.address.postal_code",
    "geography.address.street_name",
    "geography.address.street_number",
    "geography.address.street_suffix",
    "geography.contact.calling_code",
    "geography.coordinate.coordinates",
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
    "geography.location.city",
    "geography.location.continent",
    "geography.location.country",
    "geography.location.country_code",
    "geography.location.region",
    "geography.transportation.iata_code",
    "geography.transportation.icao_code",
];

const ENTITY_LABELS: &[&str] = &[
    "identity.person.blood_type",
    "identity.person.first_name",
    "identity.person.full_name",
    "identity.person.gender",
    "identity.person.gender_code",
    "identity.person.gender_symbol",
    "identity.person.last_name",
    "identity.person.username",
    "representation.text.entity_name",
];

const FORMAT_LABELS: &[&str] = &[
    // container.*
    "container.array.comma_separated",
    "container.array.pipe_separated",
    "container.array.semicolon_separated",
    "container.array.whitespace_separated",
    "container.key_value.form_data",
    "container.key_value.query_string",
    "container.object.csv",
    "container.object.json",
    "container.object.json_array",
    "container.object.xml",
    "container.object.yaml",
    // identity.medical.*
    "identity.medical.dea_number",
    "identity.medical.ndc",
    "identity.medical.npi",
    // identity.payment (11, excluding credit_card_network → text)
    "identity.payment.bitcoin_address",
    "identity.payment.credit_card_expiration_date",
    "identity.payment.credit_card_number",
    "identity.payment.cusip",
    "identity.payment.cvv",
    "identity.payment.ethereum_address",
    "identity.payment.isin",
    "identity.payment.lei",
    "identity.payment.paypal_email",
    "identity.payment.sedol",
    "identity.payment.swift_bic",
    // identity.person (structured formats)
    "identity.person.email",
    "identity.person.password",
    "identity.person.phone_number",
    // representation.code
    "representation.code.alphanumeric_id",
    // representation.scientific (bio sequences — structured alphabet)
    "representation.scientific.dna_sequence",
    "representation.scientific.protein_sequence",
    "representation.scientific.rna_sequence",
    // representation.text (structured format codes)
    "representation.text.color_hex",
    "representation.text.color_rgb",
    // technology.code.*
    "technology.code.doi",
    "technology.code.ean",
    "technology.code.imei",
    "technology.code.isbn",
    "technology.code.issn",
    "technology.code.locale_code",
    "technology.code.pin",
    // technology.internet (structured network formats)
    "technology.internet.hostname",
    "technology.internet.ip_v4",
    "technology.internet.ip_v4_with_port",
    "technology.internet.ip_v6",
    "technology.internet.mac_address",
    "technology.internet.url",
    "technology.internet.user_agent",
];

const TEXT_LABELS: &[&str] = &[
    // identity.payment (low-cardinality enums)
    "identity.payment.credit_card_network",
    "identity.payment.currency_code",
    "identity.payment.currency_symbol",
    // representation.boolean.*
    "representation.boolean.binary",
    "representation.boolean.initials",
    "representation.boolean.terms",
    // representation.discrete.*
    "representation.discrete.categorical",
    "representation.discrete.ordinal",
    // representation.file (categorical — limited value sets)
    "representation.file.excel_format",
    "representation.file.extension",
    "representation.file.mime_type",
    // representation.scientific (categorical — limited value sets)
    "representation.scientific.measurement_unit",
    "representation.scientific.metric_prefix",
    // representation.text.* (free text)
    "representation.text.emoji",
    "representation.text.paragraph",
    "representation.text.plain_text",
    "representation.text.sentence",
    "representation.text.word",
    // technology.cryptographic.* (random/opaque strings)
    "technology.cryptographic.hash",
    "technology.cryptographic.token_hex",
    "technology.cryptographic.token_urlsafe",
    "technology.cryptographic.uuid",
    // technology.development.* (low-cardinality enums + version strings)
    "technology.development.calver",
    "technology.development.os",
    "technology.development.programming_language",
    "technology.development.software_license",
    "technology.development.stage",
    "technology.development.version",
    // technology.internet (low-cardinality enum)
    "technology.internet.http_method",
    "technology.internet.top_level_domain",
];

/// Overlap types: (label, secondary category the label is also eligible in).
///
/// When Sense predicts the secondary category, these types still pass the mask.
/// Keeps masking permissive for types at category boundaries.
///
/// Ref: PHASE2_DESIGN.md Section B "Overlap Resolution"
const ALSO_ELIGIBLE: &[(&str, BroadCategory)] = &[
    // geographic ↔ format (postal codes and calling codes are structured formats)
    ("geography.address.postal_code", BroadCategory::Format),
    ("geography.contact.calling_code", BroadCategory::Format),
    // geographic ↔ numeric (coordinates are numeric values)
    ("geography.coordinate.coordinates", BroadCategory::Numeric),
    ("geography.coordinate.latitude", BroadCategory::Numeric),
    ("geography.coordinate.longitude", BroadCategory::Numeric),
    // format ↔ entity (emails/phones are identity-domain types)
    ("identity.person.email", BroadCategory::Entity),
    ("identity.person.phone_number", BroadCategory::Entity),
    // text ↔ format (credit card network is low-cardinality but CharCNN can detect)
    (
        "identity.payment.credit_card_network",
        BroadCategory::Format,
    ),
];

// ═══════════════════════════════════════════════════════════════════════════════
// LabelCategoryMap
// ═══════════════════════════════════════════════════════════════════════════════

/// Maps FineType type labels to Sense broad categories.
///
/// Constructed once at startup, then queried during column classification
/// to mask CharCNN predictions to the Sense-predicted category.
pub struct LabelCategoryMap {
    /// label → primary category
    primary: std::collections::HashMap<&'static str, BroadCategory>,
    /// label → additional eligible categories (for overlap types)
    secondary: std::collections::HashMap<&'static str, Vec<BroadCategory>>,
}

impl LabelCategoryMap {
    /// Build the map from static label arrays.
    pub fn new() -> Self {
        let mut primary = std::collections::HashMap::new();
        let mut secondary: std::collections::HashMap<&'static str, Vec<BroadCategory>> =
            std::collections::HashMap::new();

        for label in TEMPORAL_LABELS {
            primary.insert(*label, BroadCategory::Temporal);
        }
        for label in NUMERIC_LABELS {
            primary.insert(*label, BroadCategory::Numeric);
        }
        for label in GEOGRAPHIC_LABELS {
            primary.insert(*label, BroadCategory::Geographic);
        }
        for label in ENTITY_LABELS {
            primary.insert(*label, BroadCategory::Entity);
        }
        for label in FORMAT_LABELS {
            primary.insert(*label, BroadCategory::Format);
        }
        for label in TEXT_LABELS {
            primary.insert(*label, BroadCategory::Text);
        }

        for &(label, cat) in ALSO_ELIGIBLE {
            secondary.entry(label).or_default().push(cat);
        }

        Self { primary, secondary }
    }

    /// Primary category for a label.
    pub fn category_for(&self, label: &str) -> Option<BroadCategory> {
        self.primary.get(label).copied()
    }

    /// Whether a label is eligible under the given category
    /// (primary match OR also_eligible secondary match).
    pub fn is_eligible(&self, label: &str, category: BroadCategory) -> bool {
        if let Some(&primary) = self.primary.get(label) {
            if primary == category {
                return true;
            }
        }
        if let Some(secondaries) = self.secondary.get(label) {
            return secondaries.contains(&category);
        }
        false
    }

    /// All labels eligible under a category (primary + also_eligible).
    pub fn eligible_labels(&self, category: BroadCategory) -> Vec<&'static str> {
        let mut result: Vec<&'static str> = Vec::new();

        // Primary labels
        let primary_labels = match category {
            BroadCategory::Temporal => TEMPORAL_LABELS,
            BroadCategory::Numeric => NUMERIC_LABELS,
            BroadCategory::Geographic => GEOGRAPHIC_LABELS,
            BroadCategory::Entity => ENTITY_LABELS,
            BroadCategory::Format => FORMAT_LABELS,
            BroadCategory::Text => TEXT_LABELS,
        };
        result.extend_from_slice(primary_labels);

        // Secondary (overlap) labels
        for &(label, cat) in ALSO_ELIGIBLE {
            if cat == category {
                result.push(label);
            }
        }

        result
    }

    /// Total number of labels in the map.
    pub fn len(&self) -> usize {
        self.primary.len()
    }

    /// Whether the map is empty (always false for valid maps).
    pub fn is_empty(&self) -> bool {
        self.primary.is_empty()
    }
}

impl Default for LabelCategoryMap {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_is_163() {
        let map = LabelCategoryMap::new();
        assert_eq!(map.len(), 163, "Map should contain exactly 163 types");
    }

    #[test]
    fn test_category_counts() {
        assert_eq!(TEMPORAL_LABELS.len(), 46, "temporal should have 46 types");
        assert_eq!(NUMERIC_LABELS.len(), 14, "numeric should have 14 types");
        assert_eq!(
            GEOGRAPHIC_LABELS.len(),
            16,
            "geographic should have 16 types"
        );
        assert_eq!(ENTITY_LABELS.len(), 9, "entity should have 9 types");
        assert_eq!(FORMAT_LABELS.len(), 48, "format should have 48 types");
        assert_eq!(TEXT_LABELS.len(), 30, "text should have 30 types");
    }

    #[test]
    fn test_no_duplicates() {
        let all_labels: Vec<&str> = TEMPORAL_LABELS
            .iter()
            .chain(NUMERIC_LABELS)
            .chain(GEOGRAPHIC_LABELS)
            .chain(ENTITY_LABELS)
            .chain(FORMAT_LABELS)
            .chain(TEXT_LABELS)
            .copied()
            .collect();

        let mut seen = std::collections::HashSet::new();
        for label in &all_labels {
            assert!(seen.insert(label), "Duplicate label found: {}", label);
        }
    }

    #[test]
    fn test_category_for() {
        let map = LabelCategoryMap::new();

        assert_eq!(
            map.category_for("datetime.date.iso"),
            Some(BroadCategory::Temporal)
        );
        assert_eq!(
            map.category_for("representation.numeric.integer_number"),
            Some(BroadCategory::Numeric)
        );
        assert_eq!(
            map.category_for("geography.location.city"),
            Some(BroadCategory::Geographic)
        );
        assert_eq!(
            map.category_for("identity.person.full_name"),
            Some(BroadCategory::Entity)
        );
        assert_eq!(
            map.category_for("identity.person.email"),
            Some(BroadCategory::Format)
        );
        assert_eq!(
            map.category_for("representation.text.sentence"),
            Some(BroadCategory::Text)
        );
        assert_eq!(map.category_for("nonexistent.type"), None);
    }

    #[test]
    fn test_is_eligible_primary() {
        let map = LabelCategoryMap::new();

        assert!(map.is_eligible("datetime.date.iso", BroadCategory::Temporal));
        assert!(!map.is_eligible("datetime.date.iso", BroadCategory::Format));
    }

    #[test]
    fn test_is_eligible_overlap() {
        let map = LabelCategoryMap::new();

        // Email: primary=format, also=entity
        assert!(map.is_eligible("identity.person.email", BroadCategory::Format));
        assert!(map.is_eligible("identity.person.email", BroadCategory::Entity));
        assert!(!map.is_eligible("identity.person.email", BroadCategory::Text));

        // Postal code: primary=geographic, also=format
        assert!(map.is_eligible("geography.address.postal_code", BroadCategory::Geographic));
        assert!(map.is_eligible("geography.address.postal_code", BroadCategory::Format));
        assert!(!map.is_eligible("geography.address.postal_code", BroadCategory::Text));

        // Latitude: primary=geographic, also=numeric
        assert!(map.is_eligible("geography.coordinate.latitude", BroadCategory::Geographic));
        assert!(map.is_eligible("geography.coordinate.latitude", BroadCategory::Numeric));

        // Credit card network: primary=text, also=format
        assert!(map.is_eligible("identity.payment.credit_card_network", BroadCategory::Text));
        assert!(map.is_eligible(
            "identity.payment.credit_card_network",
            BroadCategory::Format
        ));
    }

    #[test]
    fn test_eligible_labels_count() {
        let map = LabelCategoryMap::new();

        // Primary + overlaps
        let temporal = map.eligible_labels(BroadCategory::Temporal);
        assert_eq!(
            temporal.len(),
            46,
            "temporal eligible should be 46 (no overlaps)"
        );

        let geographic = map.eligible_labels(BroadCategory::Geographic);
        assert_eq!(
            geographic.len(),
            16,
            "geographic eligible should be 16 (no incoming overlaps)"
        );

        let numeric = map.eligible_labels(BroadCategory::Numeric);
        // 14 primary + 3 incoming (coordinates, latitude, longitude)
        assert_eq!(numeric.len(), 17, "numeric eligible should be 14+3=17");

        let entity = map.eligible_labels(BroadCategory::Entity);
        // 9 primary + 2 incoming (email, phone_number)
        assert_eq!(entity.len(), 11, "entity eligible should be 9+2=11");

        let format = map.eligible_labels(BroadCategory::Format);
        // 48 primary + 3 incoming (postal_code, calling_code, credit_card_network)
        assert_eq!(format.len(), 51, "format eligible should be 48+3=51");

        let text = map.eligible_labels(BroadCategory::Text);
        assert_eq!(
            text.len(),
            30,
            "text eligible should be 30 (no incoming overlaps)"
        );
    }

    /// Verify that all labels in the map correspond to real taxonomy types.
    /// Run with `cargo test -p finetype-model -- taxonomy_alignment` and check stderr.
    #[test]
    fn test_all_labels_are_sorted() {
        // Verify each array is sorted (makes maintenance easier)
        let arrays: &[(&str, &[&str])] = &[
            ("temporal", TEMPORAL_LABELS),
            ("numeric", NUMERIC_LABELS),
            ("geographic", GEOGRAPHIC_LABELS),
            ("entity", ENTITY_LABELS),
            ("format", FORMAT_LABELS),
            ("text", TEXT_LABELS),
        ];

        for &(name, labels) in arrays {
            let mut sorted = labels.to_vec();
            sorted.sort();
            assert_eq!(
                labels,
                &sorted[..],
                "{} labels are not sorted alphabetically",
                name
            );
        }
    }
}
