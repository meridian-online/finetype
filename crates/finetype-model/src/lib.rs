//! FineType Model
//!
//! Candle-based models for semantic type classification.
//!
//! Inference runs a single multi-branch model per column (the Sense stage).

pub mod char_cnn;
pub mod char_distribution;
pub mod char_training;
pub mod column;
pub mod column_stats;
mod device;
pub mod embedding_aggregation;
pub mod entity;
pub mod features;
pub mod inference;
pub mod label_category_map;
pub mod model;
pub mod model2vec_shared;
pub mod multi_branch;
pub mod rhh;
pub mod semantic;
pub mod sense;
pub mod sibling_context;
pub mod training;
pub mod validation_features;
pub mod value_attention;

pub use char_cnn::{CharCnn, CharCnnConfig, CharVocab, HeadType, HierarchicalHead, HierarchyMap};
pub use char_distribution::{extract_char_distribution, CHAR_DIST_DIM, CHAR_DIST_NAMES};
pub use char_training::{CharTrainer, CharTrainingConfig};
pub use column::{
    aggregate_features, ColumnClassifier, ColumnConfig, ColumnFeatures, ColumnResult,
};
pub use column_stats::{extract_column_stats, COLUMN_STATS_DIM, COLUMN_STATS_NAMES};
pub use embedding_aggregation::{
    extract_embedding_aggregation, extract_embedding_aggregation_dyn, EMBED_AGG_DIM, EMBED_DIM,
};
pub use entity::EntityClassifier;
pub use features::{extract_features, FEATURE_DIM, FEATURE_NAMES};
pub use inference::{
    extract_validation_patterns, CharClassifier, ClassificationResult, Classifier, ValueClassifier,
};
pub use label_category_map::LabelCategoryMap;
pub use model::{TextClassifier, TextClassifierConfig};
pub use model2vec_shared::Model2VecResources;
pub use multi_branch::MultiBranchClassifier;
pub use semantic::SemanticHintClassifier;
pub use sense::{BroadCategory, EntitySubtype, SenseClassifier, SenseResult};
pub use sibling_context::SiblingContextAttention;
pub use training::{Trainer, TrainingConfig, TrainingError};
pub use validation_features::ValidationFeatureExtractor;
pub use value_attention::{ValueAttentionConfig, ValueAttentionPool};
