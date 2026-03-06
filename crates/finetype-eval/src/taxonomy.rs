use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

/// Taxonomy coverage stats computed from YAML definition files.
pub struct TaxonomyStats {
    pub total_types: usize,
    pub with_format_string: usize,
    pub with_validation: usize,
    pub with_locale_validation: usize,
    pub with_transform: usize,
    pub domains: BTreeMap<String, usize>,
}

/// Load format_string (and format_string_alt) for all types from taxonomy YAML files.
///
/// Returns a list of format strings per type. The primary format_string is first,
/// followed by format_string_alt if present. This allows the eval to try multiple
/// format variants (e.g., ISO 8601 with and without fractional seconds).
pub fn load_format_strings(labels_dir: &Path) -> Result<BTreeMap<String, Vec<String>>> {
    let mut format_strings = BTreeMap::new();
    let mut paths: Vec<_> = std::fs::read_dir(labels_dir)
        .with_context(|| format!("Failed to read labels dir: {}", labels_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("definitions_") && n.ends_with(".yaml"))
        })
        .collect();
    paths.sort();

    let fs_key = serde_yaml::Value::String("format_string".to_string());
    let fs_alt_key = serde_yaml::Value::String("format_string_alt".to_string());

    for yaml_file in &paths {
        let text = std::fs::read_to_string(yaml_file)
            .with_context(|| format!("Failed to read {}", yaml_file.display()))?;
        let data: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&text)
            .with_context(|| format!("Failed to parse {}", yaml_file.display()))?;

        for (key, val) in &data {
            if let serde_yaml::Value::Mapping(map) = val {
                let mut fmts = Vec::new();
                if let Some(fs) = map.get(&fs_key) {
                    if let Some(s) = fs.as_str() {
                        if s != "null" {
                            fmts.push(s.to_string());
                        }
                    }
                }
                if let Some(fs_alt) = map.get(&fs_alt_key) {
                    if let Some(s) = fs_alt.as_str() {
                        if s != "null" {
                            fmts.push(s.to_string());
                        }
                    }
                }
                if !fmts.is_empty() {
                    format_strings.insert(key.clone(), fmts);
                }
            }
        }
    }
    Ok(format_strings)
}

/// Load transform SQL for types that have a transform but NO format_string.
///
/// These are "Tier B" types (epochs, currency, numeric, JSON, etc.) whose
/// actionability is tested by executing the transform SQL rather than TRY_STRPTIME.
pub fn load_transforms(labels_dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut transforms = BTreeMap::new();
    let mut paths: Vec<_> = std::fs::read_dir(labels_dir)
        .with_context(|| format!("Failed to read labels dir: {}", labels_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("definitions_") && n.ends_with(".yaml"))
        })
        .collect();
    paths.sort();

    let fs_key = serde_yaml::Value::String("format_string".to_string());
    let tr_key = serde_yaml::Value::String("transform".to_string());

    for yaml_file in &paths {
        let text = std::fs::read_to_string(yaml_file)
            .with_context(|| format!("Failed to read {}", yaml_file.display()))?;
        let data: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&text)
            .with_context(|| format!("Failed to parse {}", yaml_file.display()))?;

        for (key, val) in &data {
            if let serde_yaml::Value::Mapping(map) = val {
                // Skip types that have a format_string (those are Tier A)
                let has_format_string = map
                    .get(&fs_key)
                    .and_then(|v| v.as_str())
                    .is_some_and(|s| s != "null");
                if has_format_string {
                    continue;
                }

                // Include types with a transform
                if let Some(tr) = map.get(&tr_key) {
                    if let Some(s) = tr.as_str() {
                        if s != "null" {
                            transforms.insert(key.clone(), s.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(transforms)
}

/// Compute taxonomy coverage stats from YAML definitions.
pub fn load_taxonomy_stats(labels_dir: &Path) -> Result<TaxonomyStats> {
    let mut stats = TaxonomyStats {
        total_types: 0,
        with_format_string: 0,
        with_validation: 0,
        with_locale_validation: 0,
        with_transform: 0,
        domains: BTreeMap::new(),
    };

    let mut paths: Vec<_> = std::fs::read_dir(labels_dir)
        .with_context(|| format!("Failed to read labels dir: {}", labels_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("definitions_") && n.ends_with(".yaml"))
        })
        .collect();
    paths.sort();

    for yaml_file in &paths {
        let text = std::fs::read_to_string(yaml_file)
            .with_context(|| format!("Failed to read {}", yaml_file.display()))?;
        let data: BTreeMap<String, serde_yaml::Value> = serde_yaml::from_str(&text)
            .with_context(|| format!("Failed to parse {}", yaml_file.display()))?;

        for (key, val) in &data {
            let map = match val {
                serde_yaml::Value::Mapping(m) => m,
                _ => continue,
            };

            stats.total_types += 1;

            let domain = key.split('.').next().unwrap_or("unknown").to_string();
            *stats.domains.entry(domain).or_insert(0) += 1;

            let get_str = |field: &str| -> Option<&str> {
                map.get(&serde_yaml::Value::String(field.to_string()))
                    .and_then(|v| v.as_str())
            };

            if let Some(fs) = get_str("format_string") {
                if fs != "null" {
                    stats.with_format_string += 1;
                }
            }

            if map
                .get(&serde_yaml::Value::String("validation".to_string()))
                .is_some()
            {
                stats.with_validation += 1;
            }

            if map
                .get(&serde_yaml::Value::String(
                    "validation_by_locale".to_string(),
                ))
                .is_some()
            {
                stats.with_locale_validation += 1;
            }

            if let Some(t) = get_str("transform") {
                if t != "null" {
                    stats.with_transform += 1;
                }
            }
        }
    }

    Ok(stats)
}
