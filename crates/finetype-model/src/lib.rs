//! FineType Model
//!
//! Candle-based models for semantic type classification.
//!
//! Supports both flat (single model) and tiered (hierarchical model graph) inference.

pub mod char_cnn;
pub mod char_training;
pub mod column;
pub mod entity;
pub mod features;
pub mod inference;
pub mod label_category_map;
pub mod model;
pub mod model2vec_shared;
pub mod semantic;
pub mod sense;
pub mod sibling_context;
pub mod tiered;
pub mod tiered_training;
pub mod training;

pub use char_cnn::{CharCnn, CharCnnConfig, CharVocab, HeadType, HierarchyMap};
pub use char_training::{CharTrainer, CharTrainingConfig};
pub use column::{
    aggregate_features, ColumnClassifier, ColumnConfig, ColumnFeatures, ColumnResult,
};
pub use entity::EntityClassifier;
pub use features::{extract_features, FEATURE_DIM, FEATURE_NAMES};
pub use inference::{
    extract_validation_patterns, CharClassifier, ClassificationResult, Classifier, ValueClassifier,
};
pub use label_category_map::LabelCategoryMap;
pub use model::{TextClassifier, TextClassifierConfig};
pub use model2vec_shared::Model2VecResources;
pub use semantic::SemanticHintClassifier;
pub use sense::{BroadCategory, EntitySubtype, SenseClassifier, SenseResult};
pub use sibling_context::SiblingContextAttention;
pub use tiered::{TierTiming, TieredClassifier};
pub use tiered_training::{TieredTrainer, TieredTrainingConfig, TieredTrainingReport};
pub use training::{Trainer, TrainingConfig, TrainingError};
