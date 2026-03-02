//! Data loading and preprocessing for training

use anyhow::{Context, Result};
use candle_core::Tensor;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single training sample: column with embeddings and labels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSample {
    pub header: Option<String>,
    pub values: Vec<String>,

    // Pre-computed embeddings (from Model2Vec)
    pub header_embed: Option<Vec<f32>>,
    pub value_embeds: Vec<Vec<f32>>, // [n_values, 128]

    // Labels
    pub broad_category_idx: usize, // 0-5
    pub entity_subtype_idx: usize, // 0-3
}

/// Training dataset: collection of column samples
pub struct SenseDataset {
    samples: Vec<ColumnSample>,
    device: candle_core::Device,
}

impl SenseDataset {
    /// Load dataset from JSONL file
    pub async fn load(path: &Path) -> Result<Self> {
        let device = candle_core::Device::Cpu;

        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read dataset file")?;

        let mut samples = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let sample: ColumnSample =
                serde_json::from_str(line).context("Failed to parse JSON sample")?;

            samples.push(sample);
        }

        Ok(SenseDataset { samples, device })
    }

    /// Get the number of samples in the dataset
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Check if dataset is empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Get a single sample by index
    pub fn get(&self, idx: usize) -> Option<&ColumnSample> {
        self.samples.get(idx)
    }

    /// Create a batch of samples with padding
    ///
    /// Returns:
    /// - value_embeds: [batch_size, max_values, 128]
    /// - mask: [batch_size, max_values]
    /// - header_embeds: [batch_size, 128]
    /// - has_header: [batch_size]
    /// - broad_labels: [batch_size]
    /// - entity_labels: [batch_size]
    pub fn batch(&self, indices: &[usize]) -> Result<BatchData> {
        let batch_size = indices.len();
        let max_values = 50; // Match PyTorch training config
        let embed_dim = 128;

        let mut value_embeds = vec![vec![vec![0.0; embed_dim]; max_values]; batch_size];
        let mut mask = vec![vec![false; max_values]; batch_size];
        let mut header_embeds = vec![vec![0.0; embed_dim]; batch_size];
        let mut has_header = vec![0.0; batch_size];
        let mut broad_labels = vec![0usize; batch_size];
        let mut entity_labels = vec![0usize; batch_size];

        for (bi, &idx) in indices.iter().enumerate() {
            let sample = &self.samples[idx];

            // Copy header embedding
            if let Some(ref header_embed) = sample.header_embed {
                if header_embed.len() == embed_dim {
                    header_embeds[bi] = header_embed.clone();
                    has_header[bi] = 1.0;
                }
            }

            // Copy value embeddings (up to max_values)
            for (vi, value_embed) in sample.value_embeds.iter().take(max_values).enumerate() {
                if value_embed.len() == embed_dim {
                    value_embeds[bi][vi] = value_embed.clone();
                    mask[bi][vi] = true;
                }
            }

            // Copy labels
            broad_labels[bi] = sample.broad_category_idx;
            entity_labels[bi] = sample.entity_subtype_idx;
        }

        // Convert to tensors
        let value_embeds_tensor = array_to_tensor(&value_embeds)?;
        let mask_tensor = array_to_tensor_bool(&mask)?;
        let header_embeds_tensor = array_to_tensor_2d(&header_embeds)?;
        let has_header_tensor = array_to_tensor_1d(&has_header)?;
        let broad_labels_tensor = array_to_tensor_usize(&broad_labels)?;
        let entity_labels_tensor = array_to_tensor_usize(&entity_labels)?;

        Ok(BatchData {
            value_embeds: value_embeds_tensor,
            mask: mask_tensor,
            header_embeds: header_embeds_tensor,
            has_header: has_header_tensor,
            broad_labels: broad_labels_tensor,
            entity_labels: entity_labels_tensor,
        })
    }
}

/// A batch of training data
pub struct BatchData {
    pub value_embeds: Tensor,  // [B, N, D]
    pub mask: Tensor,          // [B, N]
    pub header_embeds: Tensor, // [B, D]
    pub has_header: Tensor,    // [B]
    pub broad_labels: Tensor,  // [B]
    pub entity_labels: Tensor, // [B]
}

// ── Tensor conversion helpers ────────────────────────────

fn array_to_tensor(data: &[Vec<Vec<f32>>]) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let d2 = data[0][0].len();
    let mut flat = Vec::with_capacity(d0 * d1 * d2);
    for batch in data {
        for row in batch {
            flat.extend_from_slice(row);
        }
    }
    let t = Tensor::new(flat.as_slice(), &candle_core::Device::Cpu)
        .context("Failed to create 3D tensor")?;
    t.reshape((d0, d1, d2))
        .context("Failed to reshape 3D tensor")
}

fn array_to_tensor_2d(data: &[Vec<f32>]) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let mut flat = Vec::with_capacity(d0 * d1);
    for row in data {
        flat.extend_from_slice(row);
    }
    let t = Tensor::new(flat.as_slice(), &candle_core::Device::Cpu)
        .context("Failed to create 2D tensor")?;
    t.reshape((d0, d1)).context("Failed to reshape 2D tensor")
}

fn array_to_tensor_1d(data: &[f32]) -> Result<Tensor> {
    Tensor::new(data, &candle_core::Device::Cpu).context("Failed to create 1D tensor")
}

fn array_to_tensor_usize(data: &[usize]) -> Result<Tensor> {
    let data_u32: Vec<u32> = data.iter().map(|&x| x as u32).collect();
    Tensor::new(data_u32.as_slice(), &candle_core::Device::Cpu)
        .context("Failed to create label tensor")
}

fn array_to_tensor_bool(data: &[Vec<bool>]) -> Result<Tensor> {
    let d0 = data.len();
    let d1 = data[0].len();
    let mut flat = Vec::with_capacity(d0 * d1);
    for row in data {
        for &b in row {
            flat.push(if b { 1.0f32 } else { 0.0 });
        }
    }
    let t = Tensor::new(flat.as_slice(), &candle_core::Device::Cpu)
        .context("Failed to create mask tensor")?;
    t.reshape((d0, d1)).context("Failed to reshape mask tensor")
}
