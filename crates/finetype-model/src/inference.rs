//! Inference error type and the value-classifier contract.
//!
//! What used to live here — `Classifier` (a transformer text classifier) and
//! `CharClassifier` (the CharCNN value classifier), together with their
//! `post_process` / `pattern_validate` tails — had no construction site
//! anywhere in the workspace, tests included: the shipped pipeline is
//! `MultiBranchClassifier` (see `crate::multi_branch`), which is column-level
//! and never touches `ValueClassifier`. They were removed rather than left as
//! a second, unreachable answer to "how does FineType classify a value".
//!
//! `ValueClassifier` itself stays: `ColumnClassifier` still holds a
//! `Box<dyn ValueClassifier>`, and the column-level test suite drives the
//! Sharpen stack through `MockClassifier`.

use finetype_core::tokenizer::TokenizerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Model error: {0}")]
    ModelError(#[from] candle_core::Error),
    #[error("Tokenizer error: {0}")]
    TokenizerError(#[from] TokenizerError),
    #[error("Taxonomy error: {0}")]
    TaxonomyError(#[from] finetype_core::taxonomy::TaxonomyError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid model path: {0}")]
    InvalidPath(String),
}

/// Classification result.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    pub label: String,
    pub confidence: f32,
    pub all_scores: Vec<(String, f32)>,
}

/// Trait for any classifier that can classify text values.
pub trait ValueClassifier: Send + Sync {
    /// Classify a single text value.
    fn classify(&self, text: &str) -> Result<ClassificationResult, InferenceError>;

    /// Classify a batch of text values.
    fn classify_batch(&self, texts: &[String])
        -> Result<Vec<ClassificationResult>, InferenceError>;
}

/// A mock classifier for testing column-level inference.
///
/// Always returns the same label with 0.8 confidence, regardless of input.
/// Used in integration tests to verify that header hints properly override the
/// base classifier's output.
#[cfg(test)]
pub struct MockClassifier {
    label: String,
}

#[cfg(test)]
impl MockClassifier {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
        }
    }
}

#[cfg(test)]
impl ValueClassifier for MockClassifier {
    fn classify(&self, _text: &str) -> Result<ClassificationResult, InferenceError> {
        Ok(ClassificationResult {
            label: self.label.clone(),
            confidence: 0.8,
            all_scores: vec![(self.label.clone(), 0.8)],
        })
    }

    fn classify_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<ClassificationResult>, InferenceError> {
        Ok(texts
            .iter()
            .map(|_| ClassificationResult {
                label: self.label.clone(),
                confidence: 0.8,
                all_scores: vec![(self.label.clone(), 0.8)],
            })
            .collect())
    }
}
