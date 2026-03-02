//! Data loading, preprocessing, and batching for training.
//!
//! Supports two modes:
//! 1. **JSONL consumption** — Load pre-prepared training data with embeddings
//! 2. **Data preparation** — Load SOTAB parquet + profile CSV via DuckDB,
//!    encode with Model2Vec, and write JSONL

use anyhow::{Context, Result};
use candle_core::Device;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::sense::{EMBED_DIM, MAX_VALUES};
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
/// Returns `None` for unmappable labels.
pub fn sotab_to_broad_category(gt_label: &str) -> Option<usize> {
    // Mapping from prepare_sense_data.py SOTAB_TO_BROAD
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
        | "product"
        | "event" => Some(0),

        // FORMAT (1)
        "url" | "email" | "telephone" | "isbn" => Some(1),

        // GEOGRAPHIC (2)
        "country" | "city" | "state" | "administrativearea" | "place" | "address"
        | "postalcode" | "geocoordinates" | "continent" => Some(2),

        // NUMERIC (3)
        "integer" | "float" | "number" | "quantitativevalue" | "monetaryamount" | "mass"
        | "distance" | "duration_numeric" | "percentage" | "rating" | "unitcode" => Some(3),

        // TEMPORAL (4)
        "date" | "datetime" | "time" | "duration" | "dayofweek" | "month" | "year" => Some(4),

        // TEXT (5)
        "text" | "description" | "name" | "language" | "boolean" | "color" | "category"
        | "enumeration" | "propertyvalue" => Some(5),

        _ => None,
    }
}

/// Map SOTAB Schema.org entity labels to entity subtypes (0–3).
///
/// Returns `None` for non-entity labels.
pub fn sotab_to_entity_subtype(gt_label: &str) -> Option<usize> {
    match gt_label.to_lowercase().as_str() {
        // Person (0)
        "person" => Some(0),

        // Place (1)
        "country" | "city" | "state" | "administrativearea" | "place" | "continent" => Some(1),

        // Organization (2)
        "organization"
        | "musicgroup"
        | "sportsclub"
        | "sportsteam"
        | "localbus"
        | "corporation"
        | "educationalorganization" => Some(2),

        // Creative work (3)
        "creativework" | "movie" | "musicalbum" | "musicrecording" | "tvseries" | "book"
        | "product" => Some(3),

        _ => None,
    }
}
