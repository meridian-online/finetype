//! B3 late-fusion Sense classifier (spec 2026-06-08-late-fusion-sense-classifier).
//!
//! A learned residual head combines a value-level CharCNN view (View1) with the
//! multi-branch column-logit view (View2) and replaces multi-branch as the Sense
//! stage. Sharpen still runs after — fusion produces only the (label, confidence)
//! that Sharpen post-processes.
//!
//! Feature layout per column (968 = 728 View1 + 240 View2), identical to the
//! `finetype dump-fusion-features` blob the head was trained on:
//!   mean(240) | max(240) | vote(240) | scalars(8) | mb_logits(240)
//!
//! Head: x(968) -> LayerNorm -> Linear(968->h1) -> GELU -> Linear(h1->h2) -> GELU
//!       -> Linear(h2->240) = head_logits;  fused = head_logits + alpha * mb_logits.
//!
//! `compute_fusion_row` is the SINGLE source of the 968-dim row: both the dump
//! (training corpus) and `FusionClassifier` (inference) call it, so the trained
//! features and the served features cannot diverge.

use std::collections::HashMap;
use std::path::Path;

use candle_core::{DType, Device, Module, Tensor, D};
use candle_nn::{layer_norm, linear, Init, LayerNorm, LayerNormConfig, Linear, VarBuilder};
use finetype_core::Taxonomy;

use crate::inference::{CharClassifier, InferenceError};
use crate::multi_branch::MultiBranchClassifier;

pub const V1_DIM: usize = 728; // 240*3 + 8
pub const V2_DIM: usize = 240;
pub const N_SCALARS: usize = 8;
pub const ROW_DIM: usize = V1_DIM + V2_DIM; // 968
pub const MB_OFFSET: usize = V1_DIM; // start of mb_logits within the 968-dim row

fn err(msg: impl Into<String>) -> InferenceError {
    InferenceError::InvalidPath(msg.into())
}

/// Compute the 968-dim fusion feature row for one column's sampled values.
///
/// `name_to_idx` maps the canonical (multi-branch) label names to their index in
/// the 240-wide vectors; `n_classes` is the canonical label count (240). This is
/// the exact computation the dump performs — keep them identical.
pub fn compute_fusion_row(
    value_clf: &CharClassifier,
    mb: &MultiBranchClassifier,
    name_to_idx: &HashMap<&str, usize>,
    n_classes: usize,
    sampled: &[String],
    header: &str,
    sample_n: usize,
    taxonomy: Option<&Taxonomy>,
) -> Result<Vec<f32>, InferenceError> {
    if sampled.is_empty() {
        return Err(err("compute_fusion_row: empty sample"));
    }
    let n_used = sampled.len();

    // ---- View1: value-CharCNN softmax aggregated over the sampled values ----
    let results = value_clf.classify_batch(sampled)?;
    let mut mean = vec![0.0f32; n_classes];
    let mut vmax = vec![0.0f32; n_classes];
    let mut vote = vec![0.0f32; n_classes];
    let mut top1s: Vec<f32> = Vec::with_capacity(n_used);
    let mut entropies: Vec<f32> = Vec::with_capacity(n_used);
    for r in &results {
        // canonical-aligned softmax for this value
        let mut row = vec![0.0f32; n_classes];
        for (name, p) in &r.all_scores {
            if let Some(&i) = name_to_idx.get(name.as_str()) {
                row[i] = *p;
            }
        }
        let mut argmax = 0usize;
        let mut argmax_p = -1.0f32;
        let mut ent = 0.0f32;
        for (i, &p) in row.iter().enumerate() {
            mean[i] += p;
            if p > vmax[i] {
                vmax[i] = p;
            }
            if p > argmax_p {
                argmax_p = p;
                argmax = i;
            }
            if p > 1e-9 {
                ent -= p * p.ln();
            }
        }
        vote[argmax] += 1.0;
        top1s.push(argmax_p.max(0.0));
        entropies.push(ent);
    }
    let inv = 1.0 / n_used as f32;
    for i in 0..n_classes {
        mean[i] *= inv;
        vote[i] *= inv;
    }

    // 8 confidence scalars (order is load-bearing — mirrored in the dump meta).
    let mean_top1 = top1s.iter().sum::<f32>() * inv;
    let max_top1 = top1s.iter().cloned().fold(0.0f32, f32::max);
    let min_top1 = top1s.iter().cloned().fold(1.0f32, f32::min);
    let var_top1 = top1s.iter().map(|x| (x - mean_top1) * (x - mean_top1)).sum::<f32>() * inv;
    let std_top1 = var_top1.max(0.0).sqrt();
    let mean_entropy = entropies.iter().sum::<f32>() * inv;
    let vote_agreement = vote.iter().cloned().fold(0.0f32, f32::max); // modal share
    let mut uniq = std::collections::HashSet::new();
    for v in sampled {
        uniq.insert(v.as_str());
    }
    let frac_unique = uniq.len() as f32 * inv;
    let n_used_norm = n_used as f32 / sample_n.max(1) as f32;
    let scalars = [
        mean_top1,
        max_top1,
        min_top1,
        std_top1,
        mean_entropy,
        vote_agreement,
        frac_unique,
        n_used_norm,
    ];

    // ---- View2: multi-branch raw logits (canonical order) ----
    let v2 = mb.column_logits(sampled, header, taxonomy)?;
    if v2.len() != n_classes {
        return Err(err(format!(
            "column_logits returned {} != {n_classes}",
            v2.len()
        )));
    }

    // ---- Assemble: mean ⊕ max ⊕ vote ⊕ scalars ⊕ v2 ----
    let mut row: Vec<f32> = Vec::with_capacity(ROW_DIM);
    row.extend_from_slice(&mean);
    row.extend_from_slice(&vmax);
    row.extend_from_slice(&vote);
    row.extend_from_slice(&scalars);
    row.extend_from_slice(&v2);
    debug_assert_eq!(row.len(), ROW_DIM);
    Ok(row)
}

/// The frozen residual head: LayerNorm → Linear → GELU → Linear → GELU → Linear,
/// then `+ alpha * mb_logits`. Inference-only (no dropout).
pub struct FusionHead {
    ln: LayerNorm,
    l1: Linear,
    l2: Linear,
    l3: Linear,
    alpha: Tensor,
}

impl FusionHead {
    /// Load a head from `<dir>/model.safetensors` + `<dir>/config.json`.
    pub fn load(dir: &Path) -> Result<(Self, Vec<String>), InferenceError> {
        let cfg_bytes = std::fs::read(dir.join("config.json"))
            .map_err(|e| err(format!("read fusion head config.json: {e}")))?;
        let cfg: serde_json::Value = serde_json::from_slice(&cfg_bytes)
            .map_err(|e| err(format!("parse fusion head config.json: {e}")))?;
        let h1 = cfg["hidden1"].as_u64().ok_or_else(|| err("config.hidden1"))? as usize;
        let h2 = cfg["hidden2"].as_u64().ok_or_else(|| err("config.hidden2"))? as usize;
        let row_dim = cfg["row_dim"].as_u64().unwrap_or(ROW_DIM as u64) as usize;
        if row_dim != ROW_DIM {
            return Err(err(format!("config.row_dim {row_dim} != {ROW_DIM}")));
        }
        let label_order: Vec<String> = cfg["label_order"]
            .as_array()
            .ok_or_else(|| err("config.label_order"))?
            .iter()
            .map(|v| v.as_str().unwrap_or_default().to_string())
            .collect();
        if label_order.len() != V2_DIM {
            return Err(err(format!(
                "config.label_order len {} != {V2_DIM}",
                label_order.len()
            )));
        }

        let device = Device::Cpu;
        let weights = dir.join("model.safetensors");
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights.clone()], DType::F32, &device)
                .map_err(|e| err(format!("load fusion head {weights:?}: {e}")))?
        };
        let head = Self::from_var_builder(vb, h1, h2)?;
        Ok((head, label_order))
    }

    fn from_var_builder(vb: VarBuilder, h1: usize, h2: usize) -> Result<Self, InferenceError> {
        let ln = layer_norm(ROW_DIM, LayerNormConfig::default(), vb.pp("ln"))
            .map_err(|e| err(format!("fusion head ln: {e}")))?;
        let l1 = linear(ROW_DIM, h1, vb.pp("l1")).map_err(|e| err(format!("fusion head l1: {e}")))?;
        let l2 = linear(h1, h2, vb.pp("l2")).map_err(|e| err(format!("fusion head l2: {e}")))?;
        let l3 = linear(h2, V2_DIM, vb.pp("l3")).map_err(|e| err(format!("fusion head l3: {e}")))?;
        let alpha = vb
            .get_with_hints(1, "alpha", Init::Const(1.0))
            .map_err(|e| err(format!("fusion head alpha: {e}")))?;
        Ok(Self { ln, l1, l2, l3, alpha })
    }

    /// Fused logits for a batch of rows `[n_rows, ROW_DIM]`.
    pub fn forward(&self, x: &Tensor) -> Result<Tensor, InferenceError> {
        let mb_logits = x
            .narrow(1, MB_OFFSET, V2_DIM)
            .map_err(|e| err(format!("fusion head narrow: {e}")))?;
        let h = self.ln.forward(x).map_err(|e| err(format!("fusion ln fwd: {e}")))?;
        let h = self
            .l1
            .forward(&h)
            .and_then(|t| t.gelu_erf())
            .map_err(|e| err(format!("fusion l1 fwd: {e}")))?;
        let h = self
            .l2
            .forward(&h)
            .and_then(|t| t.gelu_erf())
            .map_err(|e| err(format!("fusion l2 fwd: {e}")))?;
        let head_logits = self.l3.forward(&h).map_err(|e| err(format!("fusion l3 fwd: {e}")))?;
        let scaled = mb_logits
            .broadcast_mul(&self.alpha)
            .map_err(|e| err(format!("fusion alpha mul: {e}")))?;
        (head_logits + scaled).map_err(|e| err(format!("fusion add: {e}")))
    }

    /// Fused logits for a single 968-dim row.
    pub fn fused_logits(&self, row: &[f32]) -> Result<Vec<f32>, InferenceError> {
        if row.len() != ROW_DIM {
            return Err(err(format!("fused_logits: row len {} != {ROW_DIM}", row.len())));
        }
        let device = Device::Cpu;
        let x = Tensor::from_slice(row, (1, ROW_DIM), &device)
            .map_err(|e| err(format!("fusion row tensor: {e}")))?;
        let logits = self.forward(&x)?;
        logits
            .squeeze(0)
            .and_then(|t| t.to_vec1::<f32>())
            .map_err(|e| err(format!("fusion logits to_vec1: {e}")))
    }
}

/// Late-fusion column classifier: value-CharCNN (View1) + multi-branch (View2) →
/// frozen residual head → (label, confidence). Replaces multi-branch as the Sense
/// stage; Sharpen runs after in `ColumnClassifier`.
pub struct FusionClassifier {
    value_clf: CharClassifier,
    mb: MultiBranchClassifier,
    head: FusionHead,
    labels: Vec<String>,
    sample_n: usize,
}

impl FusionClassifier {
    pub fn new(
        value_clf: CharClassifier,
        mb: MultiBranchClassifier,
        head: FusionHead,
        head_labels: Vec<String>,
        sample_n: usize,
    ) -> Result<Self, InferenceError> {
        let labels = mb.labels().to_vec();
        if labels.len() != V2_DIM {
            return Err(err(format!(
                "fusion: multi-branch has {} classes, expected {V2_DIM}",
                labels.len()
            )));
        }
        // The head's label_order must match the canonical (multi-branch) order,
        // since the dump aligned everything to multi-branch labels.
        if head_labels != labels {
            return Err(err(
                "fusion: head label_order does not match multi-branch labels",
            ));
        }
        Ok(Self { value_clf, mb, head, labels, sample_n })
    }

    pub fn labels(&self) -> &[String] {
        &self.labels
    }

    /// Classify one column → (label, confidence). `confidence` is the softmax
    /// probability of the argmax fused logit.
    pub fn classify_column(
        &self,
        values: &[String],
        header: &str,
        taxonomy: Option<&Taxonomy>,
    ) -> Result<(String, f32), InferenceError> {
        let ranked = self.classify_column_ranked(values, header, taxonomy)?;
        Ok(ranked
            .into_iter()
            .next()
            .unwrap_or_else(|| ("unknown".to_string(), 0.0)))
    }

    /// Classify one column → all labels ranked by softmax probability, highest
    /// first. The first element is what `classify_column` returns; the rest let a
    /// caller (e.g. a column-level cardinality gate) pick the best alternative
    /// when the top label is implausible for the column's distribution.
    pub fn classify_column_ranked(
        &self,
        values: &[String],
        header: &str,
        taxonomy: Option<&Taxonomy>,
    ) -> Result<Vec<(String, f32)>, InferenceError> {
        if values.is_empty() {
            return Ok(vec![("unknown".to_string(), 0.0)]);
        }
        let n_classes = self.labels.len();
        let name_to_idx: HashMap<&str, usize> =
            self.labels.iter().enumerate().map(|(i, l)| (l.as_str(), i)).collect();

        let row = compute_fusion_row(
            &self.value_clf,
            &self.mb,
            &name_to_idx,
            n_classes,
            values,
            header,
            self.sample_n,
            taxonomy,
        )?;
        let logits = self.head.fused_logits(&row)?;

        let amax = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let denom: f32 = logits.iter().map(|&l| (l - amax).exp()).sum();
        let mut ranked: Vec<(String, f32)> = logits
            .iter()
            .enumerate()
            .map(|(i, &l)| {
                let p = if denom > 0.0 { (l - amax).exp() / denom } else { 0.0 };
                let label = self.labels.get(i).cloned().unwrap_or_else(|| "unknown".to_string());
                (label, p)
            })
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        Ok(ranked)
    }
}

// Re-export Tensor argmax helper for parity tooling if needed.
pub fn argmax(logits: &Tensor) -> Result<Vec<u32>, InferenceError> {
    logits
        .argmax(D::Minus1)
        .and_then(|t| t.to_dtype(DType::U32))
        .and_then(|t| t.to_vec1::<u32>())
        .map_err(|e| err(format!("argmax: {e}")))
}
