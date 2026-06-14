//! Tests for the multi-branch model.

use super::*;

#[test]
fn test_config_deserialization() {
    // Old-style config without header/activation/layer_norm fields (backward compat)
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.char_dim, 960);
    assert_eq!(config.embed_dim, 512);
    assert_eq!(config.stats_dim, 27);
    assert_eq!(config.header_dim, 0); // default
    assert_eq!(config.header_hidden, [0, 0]); // default
    assert!(!config.has_header_branch());
    assert_eq!(config.n_classes, 250);
    assert_eq!(config.merged_dim(), 564); // 3-branch only
                                          // New fields default to backward-compatible values
    assert_eq!(config.activation, Activation::ReLU);
    assert!(!config.use_layer_norm);
}

#[test]
fn test_config_deserialization_with_header() {
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "header_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.header_dim, 128);
    assert_eq!(config.header_hidden, [128, 64]);
    assert!(config.has_header_branch());
    assert_eq!(config.merged_dim(), 628); // 300+200+64+64
}

#[test]
fn test_config_deserialization_gelu_layer_norm() {
    // New-style config with GELU + LayerNorm (autoresearch winner)
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "char_hidden": [450, 450],
            "embed_hidden": [300, 300],
            "stats_hidden": [192, 96],
            "header_hidden": [750, 750],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat",
            "activation": "GELU",
            "use_layer_norm": true
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.activation, Activation::GELU);
    assert!(config.use_layer_norm);
    assert!(config.has_header_branch());
}

#[test]
fn test_config_deserialization_hierarchical() {
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Hierarchical"
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.head_type, HeadType::Hierarchical);
    assert_eq!(config.n_classes, 250);
}

#[test]
fn ac05_config_deserialization_with_validation_branch() {
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "valid_dim": 239,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "header_hidden": [128, 64],
            "valid_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat",
            "type_index_keys": ["container.data.csv", "datetime.date.iso_8601"]
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.valid_dim, 239);
    assert_eq!(config.valid_hidden, [128, 64]);
    assert!(config.has_validation_branch());
    assert_eq!(config.type_index_keys.len(), 2);
    // 5-branch merged_dim: 300 + 200 + 64 + 64 + 64 = 692
    assert_eq!(config.merged_dim(), 692);
}

#[test]
fn ac06_config_backward_compat_no_validation() {
    // Old config without valid_dim/valid_hidden — should default to no validation
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "char_hidden": [300, 300],
            "embed_hidden": [200, 200],
            "stats_hidden": [128, 64],
            "header_hidden": [128, 64],
            "merge_hidden": [500, 500],
            "n_classes": 250,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.valid_dim, 0);
    assert_eq!(config.valid_hidden, [0, 0]);
    assert!(!config.has_validation_branch());
    assert!(config.type_index_keys.is_empty());
    // 4-branch merged_dim: 300 + 200 + 64 + 64 = 628
    assert_eq!(config.merged_dim(), 628);
}

#[test]
fn test_is_multi_branch_dir_missing_files() {
    // Use a path that definitely doesn't contain model files
    assert!(!MultiBranchClassifier::is_multi_branch_dir(
        "/tmp/nonexistent-finetype-test"
    ));
}

#[test]
fn ac06_config_deserialization_v13_validation_branch() {
    // v13 config with valid_hidden=[192, 128] (wider validation branch).
    let json = r#"{
            "char_dim": 960,
            "embed_dim": 512,
            "stats_dim": 27,
            "header_dim": 128,
            "valid_dim": 240,
            "char_hidden": [450, 450],
            "embed_hidden": [300, 300],
            "stats_hidden": [192, 96],
            "header_hidden": [192, 96],
            "valid_hidden": [192, 128],
            "merge_hidden": [750, 750],
            "n_classes": 240,
            "dropout": 0.35,
            "head_type": "Flat"
        }"#;
    let config: MultiBranchConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.valid_dim, 240);
    assert_eq!(config.valid_hidden, [192, 128]);
    assert!(config.has_validation_branch());
    // v13 merged_dim: 450 + 300 + 96 + 96 + 128 = 1070
    assert_eq!(config.merged_dim(), 1070);
}
