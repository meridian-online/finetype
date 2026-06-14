//! Synthetic data generation for all type definitions.
//!
//! Generates synthetic training data using taxonomy keys:
//! `domain.category.type` (e.g., `datetime.timestamp.iso_8601`).
//!
//! Each generator produces strings that match the transformation contract
//! defined in the YAML specification.

use crate::locale_data;
use crate::taxonomy::{Designation, Taxonomy};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use rand::prelude::*;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum GeneratorError {
    #[error("Unknown label: {0}")]
    UnknownLabel(String),
    #[error("Generator not implemented for: {0}")]
    NotImplemented(String),
}

/// Phone number format selection for training data diversity.
enum PhoneFmt {
    National,
    International,
    E164,
}

/// A generated sample with its label.
#[derive(Debug, Clone)]
pub struct Sample {
    pub text: String,
    pub label: String,
}

/// Data generator for creating synthetic training samples.
pub struct Generator {
    taxonomy: Taxonomy,
    rng: StdRng,
    /// Current locale for locale-aware generation (set during generate_all_localized).
    locale: Option<String>,
}

impl Generator {
    /// Create a new generator with the given taxonomy.
    pub fn new(taxonomy: Taxonomy) -> Self {
        Self {
            taxonomy,
            rng: StdRng::from_entropy(),
            locale: None,
        }
    }

    /// Create a generator with a fixed seed for reproducibility.
    pub fn with_seed(taxonomy: Taxonomy, seed: u64) -> Self {
        Self {
            taxonomy,
            rng: StdRng::seed_from_u64(seed),
            locale: None,
        }
    }

    /// Generate samples for all labels at a given priority level.
    ///
    /// For locale-specific types, randomly cycles through available locales
    /// to produce diverse training samples with 3-level labels. This ensures
    /// the model sees month names, phone formats, etc. from many locales
    /// without expanding the label space.
    pub fn generate_all(&mut self, min_priority: u8, samples_per_label: usize) -> Vec<Sample> {
        let entries: Vec<(String, Designation, Vec<String>)> = self
            .taxonomy
            .at_priority(min_priority)
            .into_iter()
            .map(|(k, d)| (k.clone(), d.designation.clone(), d.locales.clone()))
            .collect();

        let mut all_samples = Vec::new();

        for (key, designation, locales) in &entries {
            let has_locales =
                matches!(designation, Designation::LocaleSpecific) && !locales.is_empty();

            for i in 0..samples_per_label {
                // Cycle through locales for locale-specific types
                if has_locales {
                    self.locale = Some(locales[i % locales.len()].clone());
                }

                if let Ok(text) = self.generate_value(key) {
                    all_samples.push(Sample {
                        text,
                        label: key.clone(),
                    });
                }

                if has_locales {
                    self.locale = None;
                }
            }
        }

        all_samples
    }

    /// Generate samples with 4-level labels (domain.category.type.LOCALE).
    ///
    /// For locale_specific types, generates `samples_per_label` samples for EACH locale.
    /// For universal/broad types, generates samples with `.UNIVERSAL` suffix.
    pub fn generate_all_localized(
        &mut self,
        min_priority: u8,
        samples_per_label: usize,
    ) -> Vec<Sample> {
        let entries: Vec<(String, Designation, Vec<String>)> = self
            .taxonomy
            .at_priority(min_priority)
            .into_iter()
            .map(|(k, d)| (k.clone(), d.designation.clone(), d.locales.clone()))
            .collect();

        let mut all_samples = Vec::new();

        for (key, designation, locales) in &entries {
            match designation {
                Designation::LocaleSpecific => {
                    // Generate per-locale samples with 4-level labels
                    for locale in locales {
                        let label = format!("{}.{}", key, locale);
                        self.locale = Some(locale.clone());
                        for _ in 0..samples_per_label {
                            if let Ok(text) = self.generate_value(key) {
                                all_samples.push(Sample {
                                    text,
                                    label: label.clone(),
                                });
                            }
                        }
                    }
                    self.locale = None;
                }
                _ => {
                    // Universal and broad types get .UNIVERSAL suffix
                    let label = format!("{}.UNIVERSAL", key);
                    for _ in 0..samples_per_label {
                        if let Ok(text) = self.generate_value(key) {
                            all_samples.push(Sample {
                                text,
                                label: label.clone(),
                            });
                        }
                    }
                }
            }
        }

        all_samples
    }

    /// Generate a single value for a key (domain.category.type).
    pub fn generate_value(&mut self, key: &str) -> Result<String, GeneratorError> {
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() != 3 {
            return Err(GeneratorError::UnknownLabel(key.to_string()));
        }

        let (domain, category, type_name) = (parts[0], parts[1], parts[2]);

        match domain {
            "datetime" => self.gen_datetime(category, type_name),
            "technology" => self.gen_technology(category, type_name),
            "identity" => self.gen_identity(category, type_name),
            "geography" => self.gen_geography(category, type_name),
            "representation" => self.gen_representation(category, type_name),
            "container" => self.gen_container(category, type_name),
            "finance" => self.gen_finance(category, type_name),
            _ => Err(GeneratorError::UnknownLabel(key.to_string())),
        }
    }
}

mod container;
mod datetime;
mod finance;
mod geography;
mod helpers;
mod identity;
mod representation;
mod technology;

#[cfg(test)]
mod tests;
