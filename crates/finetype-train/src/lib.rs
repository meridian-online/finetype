//! FineType training infrastructure — pure Rust via Candle.
//!
//! Provides training for:
//! - **Sense model** — Cross-attention classifier for broad category + entity subtype routing
//! - **Entity classifier** — Deep Sets MLP for entity demotion gating
//! - **Data preparation** — SOTAB + profile eval → JSONL training data
//! - **Model2Vec type embeddings** — FPS algorithm for taxonomy label matching

pub mod data;
pub mod device;
pub mod entity;
pub mod model2vec_prep;
pub mod multi_branch;
pub mod sense;
pub mod sense_train;
pub mod sibling_context;
pub mod sibling_data;
pub mod sibling_train;
pub mod training;

pub use device::get_device;
