//! Core validation tests for Candle feasibility spike
//!
//! These tests verify that Candle can handle FineType's ML requirements:
//! 1. Model construction and forward pass (Sense Architecture A)
//! 2. Entity classifier MLP forward pass
//! 3. Cross-attention mechanism with masking
//! 4. Safetensors serialization round-trip
//! 5. Gradient computation (backprop viability)

use anyhow::Result;
use candle_core::{DType, Device, Tensor};
use candle_nn::{Optimizer, VarMap};

use finetype_candle_spike::models::{EntityClassifier, SenseModelA, EMBED_DIM, N_BROAD, N_ENTITY};

const BATCH: usize = 4;
const MAX_VALUES: usize = 50;

/// Helper: create random input tensors for Sense model
fn make_sense_inputs(device: &Device) -> Result<(Tensor, Tensor, Tensor, Tensor)> {
    let value_embeds = Tensor::randn(0.0f32, 1.0, (BATCH, MAX_VALUES, EMBED_DIM), device)?;
    let mask = Tensor::ones((BATCH, MAX_VALUES), DType::F32, device)?;
    let header_embed = Tensor::randn(0.0f32, 1.0, (BATCH, EMBED_DIM), device)?;
    let has_header = Tensor::new(&[1.0f32, 1.0, 0.0, 1.0], device)?;
    Ok((value_embeds, mask, header_embed, has_header))
}

#[test]
fn test_sense_model_construction() -> Result<()> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let _model = SenseModelA::new(&varmap, &device)?;

    // Verify parameters were registered
    let vars = varmap.all_vars();
    assert!(
        vars.len() > 0,
        "VarMap should contain registered parameters"
    );

    // Count expected layers: header_proj(w+b) + norm(w+b) +
    //   broad_fc1/2/3(w+b each) + entity_fc1/2/3(w+b each) + default_query = 17
    // (linear = 2 vars each, layer_norm = 2 vars, default_query = 1)
    // 1 (header_proj) * 2 + 1 (norm) * 2 + 3 (broad) * 2 + 3 (entity) * 2 + 1 = 17
    assert!(
        vars.len() >= 15,
        "Expected at least 15 parameter tensors, got {}",
        vars.len()
    );

    Ok(())
}

#[test]
fn test_sense_forward_pass() -> Result<()> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    let (value_embeds, mask, header_embed, has_header) = make_sense_inputs(&device)?;
    let (broad_logits, entity_logits) =
        model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

    // Verify output shapes
    assert_eq!(broad_logits.dims(), &[BATCH, N_BROAD], "broad logits shape");
    assert_eq!(
        entity_logits.dims(),
        &[BATCH, N_ENTITY],
        "entity logits shape"
    );

    // Verify outputs are finite (no NaN/Inf)
    let broad_data: Vec<f32> = broad_logits.flatten_all()?.to_vec1()?;
    assert!(
        broad_data.iter().all(|v| v.is_finite()),
        "Broad logits contain NaN or Inf"
    );

    let entity_data: Vec<f32> = entity_logits.flatten_all()?.to_vec1()?;
    assert!(
        entity_data.iter().all(|v| v.is_finite()),
        "Entity logits contain NaN or Inf"
    );

    Ok(())
}

#[test]
fn test_sense_no_header_path() -> Result<()> {
    // When has_header is all zeros, model should use default_query
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    let value_embeds = Tensor::randn(0.0f32, 1.0, (BATCH, MAX_VALUES, EMBED_DIM), &device)?;
    let mask = Tensor::ones((BATCH, MAX_VALUES), DType::F32, &device)?;
    let header_embed = Tensor::zeros((BATCH, EMBED_DIM), DType::F32, &device)?;
    let has_header = Tensor::zeros(BATCH, DType::F32, &device)?;

    let (broad_logits, entity_logits) =
        model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

    assert_eq!(broad_logits.dims(), &[BATCH, N_BROAD]);
    assert_eq!(entity_logits.dims(), &[BATCH, N_ENTITY]);

    let broad_data: Vec<f32> = broad_logits.flatten_all()?.to_vec1()?;
    assert!(
        broad_data.iter().all(|v| v.is_finite()),
        "No-header path produces NaN/Inf"
    );

    Ok(())
}

#[test]
fn test_entity_classifier_construction_and_forward() -> Result<()> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = EntityClassifier::new(&varmap, &device)?;

    // Input: 44 statistical features + 2 * 128 embedding dims = 300
    let input_dim = 44 + 2 * EMBED_DIM;
    let features = Tensor::randn(0.0f32, 1.0, (BATCH, input_dim), &device)?;

    let logits = model.forward(&features)?;

    assert_eq!(logits.dims(), &[BATCH, N_ENTITY], "entity logits shape");

    let logits_data: Vec<f32> = logits.flatten_all()?.to_vec1()?;
    assert!(
        logits_data.iter().all(|v| v.is_finite()),
        "Entity logits contain NaN or Inf"
    );

    Ok(())
}

#[test]
fn test_safetensors_round_trip() -> Result<()> {
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let _model = SenseModelA::new(&varmap, &device)?;

    // Save to safetensors
    let tmp_dir = tempfile::tempdir()?;
    let save_path = tmp_dir.path().join("model.safetensors");
    varmap.save(&save_path)?;

    // Verify file exists and has reasonable size
    let metadata = std::fs::metadata(&save_path)?;
    assert!(
        metadata.len() > 1000,
        "Safetensors file too small: {} bytes",
        metadata.len()
    );

    // Load into a fresh VarMap and verify shapes match
    let mut varmap2 = VarMap::new();
    let _model2 = SenseModelA::new(&varmap2, &device)?;
    varmap2.load(&save_path)?;

    // Run forward pass with loaded weights to verify integrity
    let value_embeds = Tensor::randn(0.0f32, 1.0, (2, 10, EMBED_DIM), &device)?;
    let mask = Tensor::ones((2, 10), DType::F32, &device)?;
    let header_embed = Tensor::randn(0.0f32, 1.0, (2, EMBED_DIM), &device)?;
    let has_header = Tensor::new(&[1.0f32, 0.0], &device)?;

    let (broad_logits, entity_logits) =
        _model2.forward(&value_embeds, &mask, &header_embed, &has_header)?;

    assert_eq!(broad_logits.dims(), &[2, N_BROAD]);
    assert_eq!(entity_logits.dims(), &[2, N_ENTITY]);

    Ok(())
}

#[test]
fn test_gradient_computation() -> Result<()> {
    // Verify Candle can compute gradients through the model
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    let (value_embeds, mask, header_embed, has_header) = make_sense_inputs(&device)?;
    let (broad_logits, _entity_logits) =
        model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

    // Compute a simple loss (sum of logits) and verify backprop works
    let loss = broad_logits.sum_all()?;
    let grads = loss.backward()?;

    // Check that at least some gradients are non-zero
    let params = varmap.all_vars();
    let mut has_nonzero_grad = false;
    for var in &params {
        if let Some(grad) = grads.get(var.as_tensor()) {
            let grad_norm: f32 = grad.sqr()?.sum_all()?.to_scalar()?;
            if grad_norm > 0.0 {
                has_nonzero_grad = true;
                break;
            }
        }
    }

    assert!(
        has_nonzero_grad,
        "Expected at least one parameter to have a non-zero gradient"
    );

    Ok(())
}

#[test]
fn test_optimizer_step() -> Result<()> {
    // Verify SGD optimizer can update model parameters
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    // Forward pass
    let (value_embeds, mask, header_embed, has_header) = make_sense_inputs(&device)?;
    let (broad_logits, _) = model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

    // Simple loss — backward_step does backward + update in one call
    let loss = broad_logits.sum_all()?;
    let mut sgd = candle_nn::SGD::new(varmap.all_vars(), 0.01)?;
    sgd.backward_step(&loss)?;

    // Verify model still produces valid output after parameter update
    let (value_embeds2, mask2, header_embed2, has_header2) = make_sense_inputs(&device)?;
    let (broad_logits2, _) = model.forward(&value_embeds2, &mask2, &header_embed2, &has_header2)?;

    let data: Vec<f32> = broad_logits2.flatten_all()?.to_vec1()?;
    assert!(
        data.iter().all(|v| v.is_finite()),
        "Post-update outputs should be finite"
    );

    Ok(())
}

#[test]
fn test_cross_entropy_loss() -> Result<()> {
    // Verify we can compute proper cross-entropy loss in Candle
    let device = Device::Cpu;

    // Logits: [batch=3, classes=6]
    let logits = Tensor::new(
        &[
            1.0f32, 2.0, 0.5, -1.0, 0.0, 0.3, // sample 0
            0.0, 0.0, 5.0, 0.0, 0.0, 0.0, // sample 1 (clear class 2)
            -1.0, -1.0, -1.0, 3.0, -1.0, -1.0, // sample 2 (clear class 3)
        ],
        &device,
    )?
    .reshape((3, 6))?;

    // Targets: class indices [1, 2, 3]
    let targets = Tensor::new(&[1u32, 2, 3], &device)?;

    // Compute log_softmax + nll_loss manually
    let log_probs = candle_nn::ops::log_softmax(&logits, candle_core::D::Minus1)?;

    // Gather log probs at target indices
    let targets_unsqueezed = targets.unsqueeze(1)?.to_dtype(DType::U32)?;
    let target_log_probs = log_probs.gather(&targets_unsqueezed, 1)?.squeeze(1)?;

    // NLL loss = -mean(log_probs[targets])
    let loss = target_log_probs.neg()?.mean_all()?;
    let loss_val: f32 = loss.to_scalar()?;

    assert!(loss_val.is_finite(), "Loss should be finite");
    assert!(loss_val > 0.0, "Cross-entropy loss should be positive");
    assert!(
        loss_val < 10.0,
        "Loss should be reasonable, got {}",
        loss_val
    );

    Ok(())
}

#[test]
fn test_batch_size_flexibility() -> Result<()> {
    // Verify model handles different batch sizes (important for real training)
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    for batch_size in [1, 2, 8, 16, 32] {
        let value_embeds =
            Tensor::randn(0.0f32, 1.0, (batch_size, MAX_VALUES, EMBED_DIM), &device)?;
        let mask = Tensor::ones((batch_size, MAX_VALUES), DType::F32, &device)?;
        let header_embed = Tensor::randn(0.0f32, 1.0, (batch_size, EMBED_DIM), &device)?;
        let has_header = Tensor::ones(batch_size, DType::F32, &device)?;

        let (broad_logits, entity_logits) =
            model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

        assert_eq!(broad_logits.dims(), &[batch_size, N_BROAD]);
        assert_eq!(entity_logits.dims(), &[batch_size, N_ENTITY]);
    }

    Ok(())
}

#[test]
fn test_variable_sequence_length() -> Result<()> {
    // Verify model handles different numbers of values per column
    let device = Device::Cpu;
    let varmap = VarMap::new();
    let model = SenseModelA::new(&varmap, &device)?;

    for n_values in [1, 5, 10, 50, 100] {
        let value_embeds = Tensor::randn(0.0f32, 1.0, (2, n_values, EMBED_DIM), &device)?;
        let mask = Tensor::ones((2, n_values), DType::F32, &device)?;
        let header_embed = Tensor::randn(0.0f32, 1.0, (2, EMBED_DIM), &device)?;
        let has_header = Tensor::new(&[1.0f32, 0.0], &device)?;

        let (broad_logits, entity_logits) =
            model.forward(&value_embeds, &mask, &header_embed, &has_header)?;

        assert_eq!(
            broad_logits.dims(),
            &[2, N_BROAD],
            "Failed for n_values={}",
            n_values
        );
        assert_eq!(entity_logits.dims(), &[2, N_ENTITY]);
    }

    Ok(())
}
