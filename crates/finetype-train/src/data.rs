//! Data loading, preprocessing, and batching for training.
//!
//! Supports two modes:
//! 1. **JSONL consumption** — Load pre-prepared training data with embeddings
//! 2. **Data preparation** — Load SOTAB parquet + profile CSV via DuckDB,
//!    encode with Model2Vec, and write JSONL

use anyhow::{bail, Context, Result};
use candle_core::Device;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::sense::{BROAD_CATEGORIES, EMBED_DIM, ENTITY_SUBTYPES, MAX_VALUES};
use crate::training::{bool2d_to_tensor, usize_to_tensor, vec2_to_tensor, vec3_to_tensor};

// ── Training Sample ──────────────────────────────────────────────────────────

/// A single training sample: one column with embeddings and labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSample {
    /// Column header text (None if headerless).
    pub header: Option<String>,

    /// Raw column values (for reference / feature computation).
    pub values: Vec<String>,

    /// Pre-computed Model2Vec embedding of header [128]. None if no header.
    pub header_embed: Option<Vec<f32>>,

    /// Pre-computed Model2Vec embeddings of values [n_values, 128].
    pub value_embeds: Vec<Vec<f32>>,

    /// Broad category index (0–5): entity, format, geographic, numeric, temporal, text.
    pub broad_category_idx: usize,

    /// Entity subtype index (0–3): person, place, organization, creative_work.
    pub entity_subtype_idx: usize,

    /// Broad category label (for debugging).
    #[serde(default)]
    pub broad_category: String,

    /// Entity subtype label (for debugging).
    #[serde(default)]
    pub entity_subtype: String,
}

// ── Dataset ──────────────────────────────────────────────────────────────────

/// Training dataset: indexed collection of column samples.
pub struct SenseDataset {
    pub samples: Vec<ColumnSample>,
    device: Device,
}

impl SenseDataset {
    /// Load dataset from JSONL file (one ColumnSample per line).
    pub fn load(path: &Path) -> Result<Self> {
        let device = Device::Cpu;
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read dataset: {}", path.display()))?;

        let mut samples = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let sample: ColumnSample = serde_json::from_str(line)
                .with_context(|| format!("Failed to parse sample on line {}", i + 1))?;
            samples.push(sample);
        }

        tracing::info!("Loaded {} samples from {}", samples.len(), path.display());
        Ok(Self { samples, device })
    }

    /// Create dataset from pre-built samples.
    pub fn from_samples(samples: Vec<ColumnSample>) -> Self {
        Self {
            samples,
            device: Device::Cpu,
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Create a padded batch from sample indices.
    ///
    /// Returns tensors ready for `SenseModelA::forward()`.
    pub fn batch(&self, indices: &[usize]) -> Result<BatchData> {
        let batch_size = indices.len();

        let mut value_embeds = vec![vec![vec![0.0f32; EMBED_DIM]; MAX_VALUES]; batch_size];
        let mut mask = vec![vec![false; MAX_VALUES]; batch_size];
        let mut header_embeds = vec![vec![0.0f32; EMBED_DIM]; batch_size];
        let mut has_header = vec![0.0f32; batch_size];
        let mut broad_labels = vec![0usize; batch_size];
        let mut entity_labels = vec![0usize; batch_size];

        for (bi, &idx) in indices.iter().enumerate() {
            let sample = &self.samples[idx];

            // Header embedding
            if let Some(ref h_emb) = sample.header_embed {
                if h_emb.len() == EMBED_DIM {
                    header_embeds[bi] = h_emb.clone();
                    has_header[bi] = 1.0;
                }
            }

            // Value embeddings (padded to MAX_VALUES)
            for (vi, v_emb) in sample.value_embeds.iter().take(MAX_VALUES).enumerate() {
                if v_emb.len() == EMBED_DIM {
                    value_embeds[bi][vi] = v_emb.clone();
                    mask[bi][vi] = true;
                }
            }

            broad_labels[bi] = sample.broad_category_idx;
            entity_labels[bi] = sample.entity_subtype_idx;
        }

        Ok(BatchData {
            value_embeds: vec3_to_tensor(&value_embeds, &self.device)?,
            mask: bool2d_to_tensor(&mask, &self.device)?,
            header_embeds: vec2_to_tensor(&header_embeds, &self.device)?,
            has_header: candle_core::Tensor::new(has_header.as_slice(), &self.device)?,
            broad_labels: usize_to_tensor(&broad_labels, &self.device)?,
            entity_labels: usize_to_tensor(&entity_labels, &self.device)?,
        })
    }
}

/// A batch of training data with all tensors for model forward pass.
pub struct BatchData {
    /// [B, MAX_VALUES, EMBED_DIM] — padded value embeddings.
    pub value_embeds: candle_core::Tensor,
    /// [B, MAX_VALUES] — 1.0 for real values, 0.0 for padding.
    pub mask: candle_core::Tensor,
    /// [B, EMBED_DIM] — header embeddings (zeros if no header).
    pub header_embeds: candle_core::Tensor,
    /// [B] — 1.0 if header present, 0.0 otherwise.
    pub has_header: candle_core::Tensor,
    /// [B] — broad category target indices (u32).
    pub broad_labels: candle_core::Tensor,
    /// [B] — entity subtype target indices (u32).
    pub entity_labels: candle_core::Tensor,
}

// ── SOTAB Label Mapping ──────────────────────────────────────────────────────

/// Map SOTAB Schema.org ground-truth labels to broad categories (0–5).
///
/// Handles both simple lowercase labels and SOTAB slash-suffixed labels
/// (e.g. "Person/name", "LocalBusiness/name") from the Python mapping.
///
/// Returns `None` for unmappable labels.
pub fn sotab_to_broad_category(gt_label: &str) -> Option<usize> {
    // First try exact match (case-sensitive) for slash-style SOTAB labels
    match gt_label {
        // ENTITY (0) — slash-style labels
        "Person"
        | "Person/name"
        | "Organization"
        | "MusicArtistAT"
        | "LocalBusiness/name"
        | "Hotel/name"
        | "Restaurant/name"
        | "Brand"
        | "SportsTeam"
        | "EducationalOrganization"
        | "MusicGroup"
        | "Museum/name"
        | "MusicAlbum"
        | "MusicRecording/name"
        | "Event/name"
        | "Book/name"
        | "Recipe/name"
        | "Movie/name"
        | "CreativeWork/name"
        | "CreativeWork"
        | "CreativeWorkSeries"
        | "SportsEvent/name"
        | "TVEpisode/name"
        | "MusicAlbum/name"
        | "JobPosting/name"
        | "Product/name"
        | "ProductModel" => return Some(0),
        // FORMAT (1)
        "URL" | "faxNumber" | "postalCode" | "IdentifierAT" | "identifierNameAP" | "unitCode"
        | "CategoryCode" => return Some(1),
        // TEMPORAL (4)
        "Date" | "DateTime" | "Duration" | "Time" | "DayOfWeek" | "openingHours" | "workHours" => {
            return Some(4)
        }
        // NUMERIC (3)
        "Number" | "Integer" | "Mass" | "Distance" | "Energy" | "QuantitativeValue"
        | "MonetaryAmount" | "CoordinateAT" | "Rating" | "typicalAgeRange" => return Some(3),
        // GEOGRAPHIC (2)
        "addressLocality" | "addressRegion" | "Country" | "streetAddress" | "PostalAddress" => {
            return Some(2)
        }
        // TEXT (5) — compound labels
        "ItemAvailability"
        | "ItemList"
        | "Review"
        | "EventStatusType"
        | "BookFormatType"
        | "Language"
        | "Thing"
        | "GenderType"
        | "EventAttendanceModeEnumeration"
        | "OccupationalExperienceRequirements"
        | "unitText"
        | "OfferItemCondition"
        | "Boolean"
        | "paymentAccepted"
        | "Photograph"
        | "Offer"
        | "Action"
        | "DeliveryMethod"
        | "RestrictedDiet"
        | "Product"
        | "LocationFeatureSpecification"
        | "MusicRecording"
        | "WarrantyPromise"
        | "EducationalOccupationalCredential" => return Some(5),
        _ => {}
    }

    // Fallback: case-insensitive match for simple labels
    match gt_label.to_lowercase().as_str() {
        // ENTITY (0)
        "person"
        | "organization"
        | "musicgroup"
        | "sportsclub"
        | "sportsteam"
        | "localbus"
        | "corporation"
        | "educationalorganization"
        | "creativework"
        | "movie"
        | "musicalbum"
        | "musicrecording"
        | "tvseries"
        | "book"
        | "event" => Some(0),

        // FORMAT (1)
        "url" | "email" | "telephone" | "isbn" => Some(1),

        // GEOGRAPHIC (2)
        "country" | "city" | "state" | "administrativearea" | "place" | "address"
        | "postalcode" | "geocoordinates" | "continent" => Some(2),

        // NUMERIC (3)
        "integer" | "float" | "number" | "quantitativevalue" | "monetaryamount" | "mass"
        | "distance" | "duration_numeric" | "percentage" | "rating" | "unitcode" | "weight"
        | "price" | "pricerange" | "currency" => Some(3),

        // TEMPORAL (4)
        "date" | "datetime" | "time" | "duration" | "dayofweek" | "month" | "year" => Some(4),

        // TEXT (5)
        "text" | "description" | "name" | "language" | "boolean" | "color" | "category"
        | "enumeration" | "propertyvalue" | "audience" => Some(5),

        _ => None,
    }
}

/// Map SOTAB Schema.org entity labels to entity subtypes (0–3).
///
/// Returns `None` for non-entity labels.
pub fn sotab_to_entity_subtype(gt_label: &str) -> Option<usize> {
    // Exact match first for slash-style labels
    match gt_label {
        "Person" | "Person/name" | "MusicArtistAT" => return Some(0),
        "Place" | "Place/name" | "Hotel/name" | "Restaurant/name" | "Museum/name" => {
            return Some(1)
        }
        "Organization"
        | "LocalBusiness/name"
        | "Brand"
        | "SportsTeam"
        | "EducationalOrganization"
        | "MusicGroup" => return Some(2),
        "MusicAlbum"
        | "MusicRecording/name"
        | "Event/name"
        | "Book/name"
        | "Recipe/name"
        | "Movie/name"
        | "CreativeWork/name"
        | "CreativeWork"
        | "CreativeWorkSeries"
        | "SportsEvent/name"
        | "TVEpisode/name"
        | "MusicAlbum/name"
        | "JobPosting/name"
        | "Product/name"
        | "ProductModel" => return Some(3),
        _ => {}
    }

    // Fallback: case-insensitive
    match gt_label.to_lowercase().as_str() {
        "person" => Some(0),
        "country" | "city" | "state" | "administrativearea" | "place" | "continent" => Some(1),
        "organization"
        | "musicgroup"
        | "sportsclub"
        | "sportsteam"
        | "localbus"
        | "corporation"
        | "educationalorganization" => Some(2),
        "creativework" | "movie" | "musicalbum" | "musicrecording" | "tvseries" | "book"
        | "product" => Some(3),
        _ => None,
    }
}

// ── FineType Label → Broad Category (mirrors LabelCategoryMap) ──────────────

/// Map FineType type label to broad category index (0–5).
///
/// This mirrors the authoritative mapping in `label_category_map.rs`.
/// Used for profile eval columns where gt_label maps to a FineType label.
pub fn finetype_to_broad_category(ft_label: &str) -> Option<usize> {
    // Use domain prefix for efficient routing
    if ft_label.starts_with("datetime.") {
        return Some(4); // temporal
    }
    if ft_label.starts_with("geography.") {
        return Some(2); // geographic
    }

    // Entity labels (identity.person.* that are names/attributes, not formats)
    match ft_label {
        "identity.person.blood_type"
        | "identity.person.first_name"
        | "identity.person.full_name"
        | "identity.person.gender"
        | "identity.person.gender_code"
        | "identity.person.gender_symbol"
        | "identity.person.last_name"
        | "identity.person.username"
        | "representation.text.entity_name" => return Some(0), // entity
        _ => {}
    }

    // Numeric labels
    match ft_label {
        "identity.person.age"
        | "identity.person.height"
        | "identity.person.weight"
        | "representation.file.file_size"
        | "representation.numeric.decimal_number"
        | "representation.numeric.integer_number"
        | "representation.numeric.percentage"
        | "representation.numeric.scientific_notation"
        | "representation.numeric.si_number"
        | "technology.hardware.ram_size"
        | "technology.hardware.screen_size"
        | "technology.internet.http_status_code"
        | "technology.internet.port" => return Some(3), // numeric
        _ => {}
    }

    // Format labels (structured identifiers, codes)
    if ft_label.starts_with("container.") {
        return Some(1); // format
    }
    match ft_label {
        "finance.banking.swift_bic"
        | "finance.crypto.bitcoin_address"
        | "finance.crypto.ethereum_address"
        | "finance.payment.credit_card_expiration_date"
        | "finance.payment.credit_card_number"
        | "finance.payment.paypal_email"
        | "finance.securities.cusip"
        | "finance.securities.isin"
        | "finance.securities.lei"
        | "finance.securities.sedol"
        | "identity.commerce.ean"
        | "identity.commerce.isbn"
        | "identity.commerce.issn"
        | "identity.medical.dea_number"
        | "identity.medical.ndc"
        | "identity.medical.npi"
        | "identity.person.email"
        | "identity.person.password"
        | "identity.person.phone_number"
        | "representation.identifier.alphanumeric_id"
        | "representation.identifier.increment"
        | "representation.identifier.uuid"
        | "representation.scientific.dna_sequence"
        | "representation.scientific.protein_sequence"
        | "representation.scientific.rna_sequence"
        | "representation.text.color_hex"
        | "representation.text.color_rgb"
        | "technology.code.doi"
        | "technology.code.imei"
        | "technology.code.locale_code"
        | "technology.code.pin"
        | "technology.internet.hostname"
        | "technology.internet.ip_v4"
        | "technology.internet.ip_v4_with_port"
        | "technology.internet.ip_v6"
        | "technology.internet.mac_address"
        | "technology.internet.url"
        | "technology.internet.user_agent" => return Some(1), // format
        _ => {}
    }

    // Text labels (everything else)
    match ft_label {
        "finance.currency.currency_code"
        | "finance.currency.currency_symbol"
        | "finance.payment.credit_card_network"
        | "representation.boolean.binary"
        | "representation.boolean.initials"
        | "representation.boolean.terms"
        | "representation.discrete.categorical"
        | "representation.discrete.ordinal"
        | "representation.file.excel_format"
        | "representation.file.extension"
        | "representation.file.mime_type"
        | "representation.scientific.measurement_unit"
        | "representation.scientific.metric_prefix"
        | "representation.text.emoji"
        | "representation.text.paragraph"
        | "representation.text.plain_text"
        | "representation.text.sentence"
        | "representation.text.word"
        | "technology.cryptographic.hash"
        | "technology.cryptographic.token_hex"
        | "technology.cryptographic.token_urlsafe"
        | "technology.development.calver"
        | "technology.development.os"
        | "technology.development.programming_language"
        | "technology.development.software_license"
        | "technology.development.stage"
        | "technology.development.version"
        | "technology.internet.http_method"
        | "technology.internet.top_level_domain" => return Some(5), // text
        _ => {}
    }

    // Fallback: use domain-based heuristic
    let domain = ft_label.split('.').next().unwrap_or("");
    match domain {
        "datetime" => Some(4),
        "geography" => Some(2),
        "container" => Some(1),
        _ => None,
    }
}

/// Map FineType entity labels to entity subtypes (0–3).
///
/// Returns `None` for non-entity labels.
pub fn finetype_to_entity_subtype(ft_label: &str) -> Option<usize> {
    match ft_label {
        "identity.person.full_name"
        | "identity.person.first_name"
        | "identity.person.last_name"
        | "identity.person.username"
        | "identity.person.gender"
        | "identity.person.gender_code"
        | "identity.person.gender_symbol"
        | "identity.person.blood_type" => Some(0), // person
        "representation.text.entity_name" => Some(2), // organization (conservative default)
        _ => None,
    }
}

// ── SOTAB Column Loading ────────────────────────────────────────────────────

/// A raw column from SOTAB parquet data.
#[derive(Debug, Clone)]
pub struct SotabColumn {
    pub table_name: String,
    pub col_index: i32,
    pub gt_label: String,
    pub values: Vec<String>,
}

/// Load SOTAB columns from parquet via DuckDB.
///
/// Reads `{sotab_dir}/{split}/column_values.parquet` and groups values by
/// (table_name, col_index).
pub fn load_sotab_columns(sotab_dir: &Path, split: &str) -> Result<Vec<SotabColumn>> {
    let parquet_path = sotab_dir.join(split).join("column_values.parquet");
    if !parquet_path.exists() {
        bail!("SOTAB parquet not found: {}", parquet_path.display());
    }

    let conn = duckdb::Connection::open_in_memory()?;
    let query = format!(
        "SELECT table_name, col_index, gt_label, col_value FROM read_parquet('{}')",
        parquet_path.display()
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i32>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    // Group by (table_name, col_index)
    let mut columns: HashMap<(String, i32), Vec<String>> = HashMap::new();
    let mut gt_labels: HashMap<(String, i32), String> = HashMap::new();

    for row in rows {
        let (table_name, col_index, gt_label, col_value) = row?;
        let key = (table_name, col_index);
        if let Some(val) = col_value {
            columns.entry(key.clone()).or_default().push(val);
        }
        gt_labels.entry(key).or_insert(gt_label);
    }

    let mut result = Vec::new();
    for (key, values) in columns {
        let gt_label = gt_labels.get(&key).cloned().unwrap_or_default();
        result.push(SotabColumn {
            table_name: key.0,
            col_index: key.1,
            gt_label,
            values,
        });
    }

    tracing::info!(
        "Loaded {} SOTAB columns from {}/{}",
        result.len(),
        sotab_dir.display(),
        split,
    );
    Ok(result)
}

// ── Value Sampling ──────────────────────────────────────────────────────────

/// Sample up to `max_values` from a column using frequency-weighted strategy.
///
/// Takes top-K most frequent values first (preserves distribution signal),
/// then fills remaining slots with random diverse values.
pub fn sample_values(values: &[String], max_values: usize, rng: &mut impl Rng) -> Vec<String> {
    if values.len() <= max_values {
        return values.to_vec();
    }

    // Count frequencies
    let mut freq: HashMap<&str, usize> = HashMap::new();
    for v in values {
        *freq.entry(v.as_str()).or_insert(0) += 1;
    }

    let mut unique_vals: Vec<&str> = freq.keys().copied().collect();

    // Take top half by frequency
    let top_k = max_values / 2;
    unique_vals.sort_by(|a, b| freq[b].cmp(&freq[a]));
    let mut selected: Vec<String> = unique_vals
        .iter()
        .take(top_k)
        .map(|s| s.to_string())
        .collect();

    // Build a set of selected values (owned strings to avoid borrow conflict)
    let selected_set: std::collections::HashSet<String> = selected.iter().cloned().collect();

    // Fill remaining with random diverse values (not already selected)
    let mut remaining: Vec<&str> = unique_vals
        .iter()
        .copied()
        .filter(|v| !selected_set.contains(*v))
        .collect();
    let fill_n = max_values.saturating_sub(selected.len());
    if fill_n > 0 && !remaining.is_empty() {
        remaining.shuffle(rng);
        for v in remaining.iter().take(fill_n) {
            selected.push(v.to_string());
        }
    }

    // If still short (fewer unique values than max_values), add duplicates
    if selected.len() < max_values {
        let mut extras: Vec<String> = values
            .iter()
            .filter(|v| !selected_set.contains(v.as_str()))
            .cloned()
            .collect();
        extras.shuffle(rng);
        let need = max_values - selected.len();
        selected.extend(extras.into_iter().take(need));
    }

    selected.truncate(max_values);
    selected
}

// ── Synthetic Header Generation ─────────────────────────────────────────────

/// Generate a synthetic column header for a SOTAB GT label.
///
/// Returns `None` if the label has no header templates.
pub fn generate_synthetic_header(gt_label: &str, rng: &mut impl Rng) -> Option<String> {
    let templates: &[&str] = match gt_label {
        // ENTITY
        "Person" | "Person/name" => &[
            "name",
            "full_name",
            "person_name",
            "person",
            "contact",
            "contact_name",
        ],
        "MusicArtistAT" => &["artist", "artist_name", "performer", "musician", "singer"],
        "Organization" => &["organization", "org_name", "company", "company_name", "org"],
        "LocalBusiness/name" => &["business_name", "company", "store", "shop_name", "business"],
        "Hotel/name" => &[
            "hotel",
            "hotel_name",
            "accommodation",
            "lodging",
            "property",
        ],
        "Restaurant/name" => &["restaurant", "restaurant_name", "dining", "eatery"],
        "Brand" => &["brand", "brand_name", "manufacturer", "make"],
        "SportsTeam" => &["team", "team_name", "club", "squad"],
        "EducationalOrganization" => &["school", "university", "institution", "college"],
        "MusicGroup" => &["band", "group", "band_name", "ensemble"],
        "Museum/name" => &["museum", "museum_name", "gallery", "exhibit"],
        "MusicAlbum" | "MusicAlbum/name" => &["album", "album_name", "album_title", "release"],
        "MusicRecording/name" => &["song", "track", "track_name", "song_name", "recording"],
        "Event/name" => &["event", "event_name", "activity", "occasion"],
        "Book/name" => &["book", "book_title", "title", "publication"],
        "Recipe/name" => &["recipe", "recipe_name", "dish", "meal"],
        "Movie/name" => &["movie", "movie_title", "film", "film_name"],
        "CreativeWork/name" | "CreativeWork" => &["work", "title", "creative_work", "work_name"],
        "CreativeWorkSeries" => &["series", "series_name", "show", "franchise"],
        "SportsEvent/name" => &["match", "game", "event", "fixture", "competition"],
        "TVEpisode/name" => &["episode", "episode_name", "show", "program"],
        "JobPosting/name" => &["job", "job_title", "position", "role", "vacancy"],
        "Product/name" => &["product", "product_name", "item", "article"],
        "ProductModel" => &["model", "model_name", "product_model", "variant"],
        "Place" | "Place/name" => &["place", "place_name", "venue", "location_name"],
        // FORMAT
        "URL" => &["url", "website", "link", "web_address", "homepage"],
        "email" => &["email", "email_address", "contact_email", "e_mail"],
        "telephone" => &[
            "phone",
            "telephone",
            "phone_number",
            "contact_phone",
            "tel",
            "mobile",
        ],
        "faxNumber" => &["fax", "fax_number", "fax_no"],
        "postalCode" => &["postal_code", "zip", "zip_code", "postcode"],
        "IdentifierAT" => &["id", "identifier", "code", "ref"],
        "identifierNameAP" => &["identifier_name", "id_name", "ref_name"],
        "unitCode" => &["unit_code", "unit", "units", "uom"],
        "CategoryCode" => &["category_code", "cat_code", "code", "classification"],
        // TEMPORAL
        "Date" => &[
            "date",
            "created_date",
            "event_date",
            "start_date",
            "end_date",
            "birth_date",
            "due_date",
        ],
        "DateTime" => &[
            "datetime",
            "timestamp",
            "created_at",
            "updated_at",
            "date_time",
            "modified_at",
        ],
        "Duration" => &["duration", "length", "time_span", "runtime", "elapsed"],
        "Time" => &["time", "start_time", "end_time", "clock_time"],
        "DayOfWeek" => &["day", "day_of_week", "weekday", "day_name"],
        "openingHours" => &["hours", "opening_hours", "business_hours", "schedule"],
        "workHours" => &["work_hours", "shift", "working_hours"],
        // NUMERIC
        "Number" => &[
            "number", "count", "quantity", "amount", "total", "value", "num",
        ],
        "Integer" => &["integer", "count", "number", "qty", "int"],
        "Mass" | "weight" => &["mass", "weight", "weight_kg", "mass_kg", "grams"],
        "Distance" => &["distance", "length", "distance_km", "range", "miles"],
        "Energy" => &["energy", "calories", "energy_kj", "power", "watts"],
        "QuantitativeValue" => &["value", "quantity", "amount", "measurement", "metric"],
        "price" => &["price", "cost", "amount", "unit_price", "total_price"],
        "priceRange" => &["price_range", "cost_range", "pricing", "price_bracket"],
        "currency" => &["currency", "currency_code", "curr", "monetary_unit"],
        "MonetaryAmount" => &["amount", "monetary_amount", "payment", "total", "sum"],
        "CoordinateAT" => &["coordinate", "lat", "lng", "coord", "latitude", "longitude"],
        "Rating" => &["rating", "score", "stars", "review_score", "rank"],
        "typicalAgeRange" => &["age_range", "age", "target_age", "age_group"],
        // GEOGRAPHIC
        "addressLocality" => &["city", "locality", "town", "municipality", "place"],
        "addressRegion" => &["region", "state", "province", "area", "district"],
        "Country" => &["country", "nation", "country_name", "state"],
        "streetAddress" => &["address", "street_address", "street", "location"],
        "PostalAddress" => &[
            "postal_address",
            "mailing_address",
            "full_address",
            "address",
        ],
        // TEXT
        "Text" => &["text", "description", "content", "notes", "details", "info"],
        "category" => &["category", "type", "class", "group", "kind"],
        "ItemAvailability" => &["availability", "in_stock", "status", "stock_status"],
        "ItemList" => &["items", "list", "item_list", "entries"],
        "Review" => &["review", "feedback", "comment", "testimonial"],
        "EventStatusType" => &["status", "event_status", "state"],
        "BookFormatType" => &["format", "book_format", "edition_type"],
        "Language" => &["language", "lang", "locale"],
        "Thing" => &["thing", "item", "object", "entity"],
        "GenderType" => &["gender", "sex"],
        "Boolean" => &["is_active", "flag", "enabled", "active", "boolean"],
        "Product" => &["product", "item", "goods"],
        _ => return None,
    };

    if templates.is_empty() {
        return None;
    }

    let idx = rng.gen_range(0..templates.len());
    Some(templates[idx].to_string())
}

// ── Profile Eval Column Loading ─────────────────────────────────────────────

/// A column from profile eval with real headers.
#[derive(Debug, Clone)]
pub struct ProfileColumn {
    pub dataset: String,
    pub column_name: String,
    pub gt_label: String,
    pub finetype_label: String,
    pub broad_category: String,
    pub broad_category_idx: usize,
    pub entity_subtype: Option<String>,
    pub entity_subtype_idx: usize,
    pub values: Vec<String>,
}

/// Load profile eval columns from manifest CSV and schema mapping YAML.
///
/// Reads manifest.csv to find (dataset, file_path, column_name, gt_label),
/// then loads each CSV to extract column values. Maps gt_label to FineType
/// label via schema_mapping.yaml, then to broad category and entity subtype.
pub fn load_profile_columns(
    manifest_path: &Path,
    schema_mapping_path: &Path,
) -> Result<Vec<ProfileColumn>> {
    // Load schema mapping: gt_label → finetype_label
    let mapping_content = std::fs::read_to_string(schema_mapping_path).with_context(|| {
        format!(
            "Failed to read schema mapping: {}",
            schema_mapping_path.display()
        )
    })?;
    let mapping: serde_yaml::Value = serde_yaml::from_str(&mapping_content)?;

    let mut label_to_ft: HashMap<String, String> = HashMap::new();
    if let Some(mappings) = mapping.get("mappings").and_then(|m| m.as_sequence()) {
        for entry in mappings {
            if let (Some(gt), Some(ft)) = (
                entry.get("gt_label").and_then(|v| v.as_str()),
                entry.get("finetype_label").and_then(|v| v.as_str()),
            ) {
                label_to_ft.insert(gt.to_string(), ft.to_string());
            }
        }
    }

    // Load manifest
    let mut reader = csv::Reader::from_path(manifest_path)
        .with_context(|| format!("Failed to read manifest: {}", manifest_path.display()))?;

    let mut result = Vec::new();
    let mut skipped = Vec::new();

    for record in reader.records() {
        let record = record?;
        let dataset = record.get(0).unwrap_or("").to_string();
        let file_path_str = record.get(1).unwrap_or("");
        let col_name = record.get(2).unwrap_or("").to_string();
        let gt_label = record.get(3).unwrap_or("").to_string();

        let file_path = std::path::PathBuf::from(file_path_str);
        if !file_path.exists() {
            skipped.push(format!(
                "{}.{}: file not found ({})",
                dataset,
                col_name,
                file_path.display()
            ));
            continue;
        }

        // Read column values via DuckDB
        let values = match load_csv_column(&file_path, &col_name) {
            Ok(v) => v,
            Err(e) => {
                skipped.push(format!("{}.{}: read error ({})", dataset, col_name, e));
                continue;
            }
        };

        if values.is_empty() {
            skipped.push(format!("{}.{}: no values", dataset, col_name));
            continue;
        }

        // Map gt_label → finetype_label → broad category
        let ft_label = label_to_ft.get(&gt_label).cloned().unwrap_or_default();
        let broad_cat_idx = finetype_to_broad_category(&ft_label).unwrap_or_else(|| {
            // Fallback: domain-based guess
            if !ft_label.is_empty() {
                let domain = ft_label.split('.').next().unwrap_or("");
                match domain {
                    "datetime" => 4,
                    "geography" => 2,
                    "container" => 1,
                    _ => 5, // text
                }
            } else {
                5 // text
            }
        });
        let broad_cat = BROAD_CATEGORIES[broad_cat_idx].to_string();

        let entity_sub = finetype_to_entity_subtype(&ft_label);
        let entity_subtype = entity_sub.map(|idx| ENTITY_SUBTYPES[idx].to_string());
        let entity_subtype_idx = entity_sub.unwrap_or(0);

        result.push(ProfileColumn {
            dataset,
            column_name: col_name,
            gt_label,
            finetype_label: ft_label,
            broad_category: broad_cat,
            broad_category_idx: broad_cat_idx,
            entity_subtype,
            entity_subtype_idx,
            values,
        });
    }

    if !skipped.is_empty() {
        tracing::warn!(
            "Skipped {} profile columns (first 5: {:?})",
            skipped.len(),
            &skipped[..skipped.len().min(5)]
        );
    }

    tracing::info!("Loaded {} profile eval columns", result.len());
    Ok(result)
}

/// Load a single column from a CSV file via DuckDB.
fn load_csv_column(file_path: &Path, col_name: &str) -> Result<Vec<String>> {
    let conn = duckdb::Connection::open_in_memory()?;
    let query = format!(
        "SELECT CAST(\"{}\" AS VARCHAR) FROM read_csv_auto('{}') WHERE \"{}\" IS NOT NULL",
        col_name,
        file_path.display(),
        col_name,
    );

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map([], |row| row.get::<_, Option<String>>(0))?;

    let mut values = Vec::new();
    for row in rows {
        if let Some(val) = row? {
            values.push(val);
        }
    }
    Ok(values)
}

// ── Data Preparation Pipeline ───────────────────────────────────────────────

/// Configuration for the data preparation pipeline.
pub struct PrepareConfig {
    pub sotab_dir: std::path::PathBuf,
    pub output_dir: std::path::PathBuf,
    pub max_values: usize,
    pub val_fraction: f64,
    pub seed: u64,
    pub include_profile: bool,
    pub profile_repeat: usize,
    pub synthetic_headers: bool,
    pub header_fraction: f64,
    pub manifest_path: std::path::PathBuf,
    pub schema_mapping_path: std::path::PathBuf,
    pub model2vec_dir: std::path::PathBuf,
}

/// Statistics from the data preparation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrepareStats {
    pub n_train: usize,
    pub n_val: usize,
    pub n_sotab_columns: usize,
    pub n_profile_columns: usize,
    pub n_synthetic_headers: usize,
    pub category_distribution: HashMap<String, usize>,
}

/// Intermediate column representation before encoding.
struct PrepColumn {
    #[allow(dead_code)]
    table_name: String,
    #[allow(dead_code)]
    col_index: i32,
    gt_label: String,
    broad_category: String,
    broad_category_idx: usize,
    entity_subtype: Option<String>,
    entity_subtype_idx: usize,
    header: Option<String>,
    sampled_values: Vec<String>,
}

/// Run the full data preparation pipeline.
///
/// 1. Load SOTAB validation (+ test) columns
/// 2. Map labels to broad category + entity subtype
/// 3. Sample values
/// 4. Optionally load profile columns (repeated N times)
/// 5. Optionally generate synthetic headers
/// 6. Encode values + headers with Model2VecResources
/// 7. Stratified train/val split
/// 8. Write train.jsonl + val.jsonl + meta.json
pub fn prepare_and_write(config: &PrepareConfig) -> Result<PrepareStats> {
    use rand::SeedableRng;
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

    // 1. Load SOTAB columns
    tracing::info!("Loading SOTAB validation columns...");
    let mut sotab_cols = load_sotab_columns(&config.sotab_dir, "validation")?;
    tracing::info!("  Loaded {} validation columns", sotab_cols.len());

    let test_parquet = config.sotab_dir.join("test").join("column_values.parquet");
    if test_parquet.exists() {
        tracing::info!("Loading SOTAB test columns...");
        let test_cols = load_sotab_columns(&config.sotab_dir, "test")?;
        tracing::info!("  Loaded {} test columns", test_cols.len());
        sotab_cols.extend(test_cols);
    }

    // 2. Map SOTAB labels and sample values
    let mut columns: Vec<PrepColumn> = Vec::new();
    for col in &sotab_cols {
        let broad_cat_idx = match sotab_to_broad_category(&col.gt_label) {
            Some(idx) => idx,
            None => continue, // skip unmappable
        };
        let entity_sub_idx = sotab_to_entity_subtype(&col.gt_label);

        let sampled = sample_values(&col.values, config.max_values, &mut rng);

        columns.push(PrepColumn {
            table_name: col.table_name.clone(),
            col_index: col.col_index,
            gt_label: col.gt_label.clone(),
            broad_category: BROAD_CATEGORIES[broad_cat_idx].to_string(),
            broad_category_idx: broad_cat_idx,
            entity_subtype: entity_sub_idx.map(|i| ENTITY_SUBTYPES[i].to_string()),
            entity_subtype_idx: entity_sub_idx.unwrap_or(0),
            header: None,
            sampled_values: sampled,
        });
    }

    let n_sotab = columns.len();
    tracing::info!("Mapped {} SOTAB columns to categories", n_sotab);

    // 3. Optionally assign synthetic headers
    let mut n_synthetic = 0;
    if config.synthetic_headers {
        for col in &mut columns {
            if col.header.is_some() {
                continue;
            }
            if rng.gen::<f64>() >= config.header_fraction {
                continue;
            }
            if let Some(header) = generate_synthetic_header(&col.gt_label, &mut rng) {
                col.header = Some(header);
                n_synthetic += 1;
            }
        }
        tracing::info!("Assigned {} synthetic headers", n_synthetic);
    }

    // 4. Optionally load profile columns
    let mut profile_cols_raw = Vec::new();
    if config.include_profile {
        tracing::info!("Loading profile eval columns...");
        profile_cols_raw =
            load_profile_columns(&config.manifest_path, &config.schema_mapping_path)?;
        tracing::info!(
            "  Loaded {} profile eval columns with real headers",
            profile_cols_raw.len()
        );
    }

    let n_profile = profile_cols_raw.len();

    // Convert profile columns to PrepColumn
    let profile_prep: Vec<PrepColumn> = profile_cols_raw
        .iter()
        .map(|pc| {
            let sampled = sample_values(&pc.values, config.max_values, &mut rng);
            PrepColumn {
                table_name: format!("profile_{}", pc.dataset),
                col_index: 0,
                gt_label: pc.gt_label.clone(),
                broad_category: pc.broad_category.clone(),
                broad_category_idx: pc.broad_category_idx,
                entity_subtype: pc.entity_subtype.clone(),
                entity_subtype_idx: pc.entity_subtype_idx,
                header: Some(pc.column_name.clone()),
                sampled_values: sampled,
            }
        })
        .collect();

    // 5. Stratified train/val split of SOTAB
    let mut by_category: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, col) in columns.iter().enumerate() {
        by_category
            .entry(col.broad_category_idx)
            .or_default()
            .push(i);
    }

    let mut train_indices = Vec::new();
    let mut val_indices = Vec::new();
    for (_cat, mut indices) in by_category {
        indices.shuffle(&mut rng);
        let split_idx = ((indices.len() as f64) * (1.0 - config.val_fraction)) as usize;
        train_indices.extend_from_slice(&indices[..split_idx]);
        val_indices.extend_from_slice(&indices[split_idx..]);
    }

    // Add profile columns to training (repeated N times)
    let mut train_columns: Vec<&PrepColumn> = train_indices.iter().map(|&i| &columns[i]).collect();
    if !profile_prep.is_empty() && config.profile_repeat > 0 {
        for _ in 0..config.profile_repeat {
            for pc in &profile_prep {
                train_columns.push(pc);
            }
        }
        tracing::info!(
            "Added {} profile eval rows ({} columns x {} repeats)",
            profile_prep.len() * config.profile_repeat,
            profile_prep.len(),
            config.profile_repeat
        );
    }

    let val_columns: Vec<&PrepColumn> = val_indices.iter().map(|&i| &columns[i]).collect();

    // Shuffle train set
    let mut train_order: Vec<usize> = (0..train_columns.len()).collect();
    train_order.shuffle(&mut rng);
    let train_columns: Vec<&PrepColumn> = train_order.iter().map(|&i| train_columns[i]).collect();

    tracing::info!(
        "Split: {} train, {} val",
        train_columns.len(),
        val_columns.len()
    );

    // 6. Encode with Model2Vec and write JSONL
    tracing::info!("Loading Model2Vec resources...");
    let model2vec = finetype_model::Model2VecResources::load(&config.model2vec_dir)
        .context("Failed to load Model2Vec")?;

    std::fs::create_dir_all(&config.output_dir)?;

    tracing::info!("Encoding and writing train.jsonl...");
    let train_path = config.output_dir.join("train.jsonl");
    encode_and_write_jsonl(&train_columns, &model2vec, &train_path)?;

    tracing::info!("Encoding and writing val.jsonl...");
    let val_path = config.output_dir.join("val.jsonl");
    encode_and_write_jsonl(&val_columns, &model2vec, &val_path)?;

    // 7. Compute stats
    let mut category_dist: HashMap<String, usize> = HashMap::new();
    for col in &train_columns {
        *category_dist.entry(col.broad_category.clone()).or_insert(0) += 1;
    }
    for col in &val_columns {
        *category_dist.entry(col.broad_category.clone()).or_insert(0) += 1;
    }

    let stats = PrepareStats {
        n_train: train_columns.len(),
        n_val: val_columns.len(),
        n_sotab_columns: n_sotab,
        n_profile_columns: n_profile,
        n_synthetic_headers: n_synthetic,
        category_distribution: category_dist,
    };

    // 8. Write meta.json
    let meta = serde_json::json!({
        "broad_categories": BROAD_CATEGORIES,
        "entity_subtypes": ENTITY_SUBTYPES,
        "max_values": config.max_values,
        "seed": config.seed,
        "n_train": stats.n_train,
        "n_val": stats.n_val,
        "include_profile": config.include_profile,
        "profile_repeat": if config.include_profile { config.profile_repeat } else { 0 },
        "n_profile_columns": n_profile,
        "synthetic_headers": config.synthetic_headers,
        "header_fraction": if config.synthetic_headers { config.header_fraction } else { 0.0 },
    });
    let meta_path = config.output_dir.join("meta.json");
    std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;

    tracing::info!(
        "Wrote train.jsonl ({} samples), val.jsonl ({} samples), meta.json",
        stats.n_train,
        stats.n_val,
    );

    Ok(stats)
}

/// Encode columns with Model2Vec and write to JSONL.
fn encode_and_write_jsonl(
    columns: &[&PrepColumn],
    model2vec: &finetype_model::Model2VecResources,
    output_path: &Path,
) -> Result<()> {
    use std::io::Write;

    let file = std::fs::File::create(output_path)?;
    let mut writer = std::io::BufWriter::new(file);

    for col in columns {
        // Encode values
        let value_refs: Vec<&str> = col.sampled_values.iter().map(|s| s.as_str()).collect();
        let value_tensor = model2vec.encode_batch(&value_refs)?;
        let value_vecs: Vec<Vec<f32>> = value_tensor.to_vec2()?;

        // Encode header
        let header_embed = if let Some(ref header) = col.header {
            let h_tensor = model2vec.encode_batch(&[header.as_str()])?;
            let h_row: Vec<f32> = h_tensor.get(0)?.to_vec1()?;
            // Check if it's a zero vector (encoding failed)
            if h_row.iter().all(|&v| v.abs() < 1e-8) {
                None
            } else {
                Some(h_row)
            }
        } else {
            None
        };

        let sample = ColumnSample {
            header: col.header.clone(),
            values: col.sampled_values.clone(),
            header_embed,
            value_embeds: value_vecs,
            broad_category_idx: col.broad_category_idx,
            entity_subtype_idx: col.entity_subtype_idx,
            broad_category: col.broad_category.clone(),
            entity_subtype: col.entity_subtype.clone().unwrap_or_default(),
        };

        let json_line = serde_json::to_string(&sample)?;
        writeln!(writer, "{}", json_line)?;
    }

    writer.flush()?;
    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn test_sotab_to_broad_category_basic() {
        // Simple lowercase labels
        assert_eq!(sotab_to_broad_category("person"), Some(0));
        assert_eq!(sotab_to_broad_category("url"), Some(1));
        assert_eq!(sotab_to_broad_category("country"), Some(2));
        assert_eq!(sotab_to_broad_category("integer"), Some(3));
        assert_eq!(sotab_to_broad_category("date"), Some(4));
        assert_eq!(sotab_to_broad_category("text"), Some(5));
        assert_eq!(sotab_to_broad_category("unknown_label_xyz"), None);
    }

    #[test]
    fn test_sotab_to_broad_category_slash_labels() {
        // SOTAB slash-style labels (case-sensitive)
        assert_eq!(sotab_to_broad_category("Person/name"), Some(0));
        assert_eq!(sotab_to_broad_category("LocalBusiness/name"), Some(0));
        assert_eq!(sotab_to_broad_category("MusicAlbum/name"), Some(0));
        assert_eq!(sotab_to_broad_category("URL"), Some(1));
        assert_eq!(sotab_to_broad_category("PostalAddress"), Some(2));
        assert_eq!(sotab_to_broad_category("CoordinateAT"), Some(3));
        assert_eq!(sotab_to_broad_category("DateTime"), Some(4));
        assert_eq!(sotab_to_broad_category("Boolean"), Some(5));
    }

    #[test]
    fn test_sotab_to_entity_subtype() {
        assert_eq!(sotab_to_entity_subtype("Person"), Some(0));
        assert_eq!(sotab_to_entity_subtype("Person/name"), Some(0));
        assert_eq!(sotab_to_entity_subtype("country"), Some(1));
        assert_eq!(sotab_to_entity_subtype("Organization"), Some(2));
        assert_eq!(sotab_to_entity_subtype("MusicGroup"), Some(2));
        assert_eq!(sotab_to_entity_subtype("CreativeWork"), Some(3));
        assert_eq!(sotab_to_entity_subtype("Movie/name"), Some(3));
        assert_eq!(sotab_to_entity_subtype("URL"), None);
        assert_eq!(sotab_to_entity_subtype("date"), None);
    }

    #[test]
    fn test_finetype_to_broad_category() {
        // Temporal
        assert_eq!(
            finetype_to_broad_category("datetime.timestamp.iso_8601"),
            Some(4)
        );
        assert_eq!(finetype_to_broad_category("datetime.date.iso"), Some(4));
        // Geographic
        assert_eq!(
            finetype_to_broad_category("geography.location.city"),
            Some(2)
        );
        assert_eq!(
            finetype_to_broad_category("geography.coordinate.latitude"),
            Some(2)
        );
        // Entity
        assert_eq!(
            finetype_to_broad_category("identity.person.full_name"),
            Some(0)
        );
        assert_eq!(
            finetype_to_broad_category("representation.text.entity_name"),
            Some(0)
        );
        // Numeric
        assert_eq!(
            finetype_to_broad_category("representation.numeric.decimal_number"),
            Some(3)
        );
        assert_eq!(finetype_to_broad_category("identity.person.age"), Some(3));
        // Format
        assert_eq!(finetype_to_broad_category("identity.person.email"), Some(1));
        assert_eq!(
            finetype_to_broad_category("technology.internet.url"),
            Some(1)
        );
        assert_eq!(finetype_to_broad_category("container.object.json"), Some(1));
        // Text
        assert_eq!(
            finetype_to_broad_category("representation.text.sentence"),
            Some(5)
        );
        assert_eq!(
            finetype_to_broad_category("technology.development.os"),
            Some(5)
        );
    }

    #[test]
    fn test_finetype_to_entity_subtype() {
        assert_eq!(
            finetype_to_entity_subtype("identity.person.full_name"),
            Some(0)
        );
        assert_eq!(
            finetype_to_entity_subtype("identity.person.first_name"),
            Some(0)
        );
        assert_eq!(
            finetype_to_entity_subtype("representation.text.entity_name"),
            Some(2)
        );
        assert_eq!(finetype_to_entity_subtype("identity.person.email"), None);
        assert_eq!(finetype_to_entity_subtype("datetime.date.iso"), None);
    }

    #[test]
    fn test_sample_values_short_input() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let values = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let sampled = sample_values(&values, 5, &mut rng);
        assert_eq!(sampled.len(), 3); // fewer than max, return all
        assert!(sampled.contains(&"a".to_string()));
        assert!(sampled.contains(&"b".to_string()));
        assert!(sampled.contains(&"c".to_string()));
    }

    #[test]
    fn test_sample_values_frequency_weighted() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        // "a" appears 10 times, "b" 5, "c" 1, "d" 1, "e" 1
        let mut values = Vec::new();
        for _ in 0..10 {
            values.push("a".to_string());
        }
        for _ in 0..5 {
            values.push("b".to_string());
        }
        values.push("c".to_string());
        values.push("d".to_string());
        values.push("e".to_string());

        let sampled = sample_values(&values, 4, &mut rng);
        assert_eq!(sampled.len(), 4);
        // Most frequent ("a" and "b") should be in the top-K selection
        assert!(
            sampled.contains(&"a".to_string()),
            "highest freq 'a' should be selected"
        );
        assert!(
            sampled.contains(&"b".to_string()),
            "second highest freq 'b' should be selected"
        );
    }

    #[test]
    fn test_sample_values_respects_max() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let values: Vec<String> = (0..100).map(|i| format!("val_{}", i)).collect();
        let sampled = sample_values(&values, 10, &mut rng);
        assert_eq!(sampled.len(), 10);
    }

    #[test]
    fn test_generate_synthetic_header_known_labels() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        // Known labels should produce headers
        let header = generate_synthetic_header("Person", &mut rng);
        assert!(header.is_some());
        let h = header.unwrap();
        assert!(
            [
                "name",
                "full_name",
                "person_name",
                "person",
                "contact",
                "contact_name"
            ]
            .contains(&h.as_str()),
            "unexpected header: {}",
            h
        );

        let header = generate_synthetic_header("URL", &mut rng);
        assert!(header.is_some());

        let header = generate_synthetic_header("Date", &mut rng);
        assert!(header.is_some());

        let header = generate_synthetic_header("Number", &mut rng);
        assert!(header.is_some());

        let header = generate_synthetic_header("Country", &mut rng);
        assert!(header.is_some());
    }

    #[test]
    fn test_generate_synthetic_header_unknown_label() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        assert_eq!(
            generate_synthetic_header("SomeUnknownLabel", &mut rng),
            None
        );
    }

    #[test]
    fn test_generate_synthetic_header_deterministic() {
        let mut rng1 = rand::rngs::StdRng::seed_from_u64(99);
        let mut rng2 = rand::rngs::StdRng::seed_from_u64(99);
        let h1 = generate_synthetic_header("Person", &mut rng1);
        let h2 = generate_synthetic_header("Person", &mut rng2);
        assert_eq!(h1, h2, "same seed should produce same header");
    }
}
