//! The two implementations of sibling-context attention must agree.
//!
//! `FrozenSiblingContext` (this crate) enriches header features while the
//! multi-branch header branch is trained. `SiblingContextAttention`
//! (finetype-model) is the same module, implemented separately, and is what any
//! attempt to reproduce training conditions at inference would run.
//!
//! Inference does NOT currently run it — serving the branch raw header
//! embeddings measured neutral-to-better than reproducing its training
//! conditions (docs/sibling-context-serving-measurement.md). This test exists
//! because that measurement is only worth the paper it is on if the two forward
//! passes were the same forward pass, and because the question will come back
//! at the next retrain: the trainer still enriches.
//!
//! So this compares them on the REAL shipped artifact, not on random weights:
//! random weights would prove the arithmetic matches while leaving a wrong
//! tensor-name mapping, a transposed projection, or a stale file entirely
//! invisible.
//!
//! Skips when `models/sibling-context/` is absent (fresh checkout, CI without
//! the model cache).

use candle_core::{Device, Tensor};
use finetype_model::sibling_context::SiblingContextAttention;
use finetype_train::multi_branch::FrozenSiblingContext;
use std::path::PathBuf;

fn artifact_dir() -> Option<PathBuf> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .parent()?
        .join("models/sibling-context");
    dir.join("model.safetensors").exists().then_some(dir)
}

#[test]
fn training_and_serving_run_the_same_attention_on_the_shipped_artifact() {
    let Some(dir) = artifact_dir() else {
        eprintln!("models/sibling-context absent — skipping");
        return;
    };
    let device = Device::Cpu;
    let frozen = FrozenSiblingContext::load(&dir, &device).expect("load frozen (training) module");
    let serving = SiblingContextAttention::load(&dir).expect("load serving module");
    let d = serving.embed_dim();
    assert_eq!(
        frozen.embed_dim(),
        d,
        "the two loaders disagree on embed_dim"
    );

    // Deterministic non-degenerate input: a fixed pseudo-random ramp, so a
    // failure reproduces exactly. Several column counts, because attention
    // mixes across rows and a 1-row case exercises none of that mixing.
    for n in [1usize, 2, 5, 17] {
        let data: Vec<f32> = (0..n * d)
            .map(|i| ((i as f32 * 0.618034).sin() * 0.5) + ((i % 7) as f32 * 0.01))
            .collect();
        let x = Tensor::from_vec(data, (n, d), &device).unwrap();

        let a: Vec<f32> = frozen
            .forward(&x)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();
        let b: Vec<f32> = serving
            .forward(&x)
            .unwrap()
            .flatten_all()
            .unwrap()
            .to_vec1()
            .unwrap();

        assert_eq!(a.len(), b.len(), "output shape mismatch at n={n}");
        let worst = a
            .iter()
            .zip(b.iter())
            .map(|(p, q)| (p - q).abs())
            .fold(0.0f32, f32::max);
        assert!(
            worst < 1e-5,
            "training and serving attention diverge at n={n}: max abs diff {worst}"
        );
        // Guard the guard: an all-zero output would satisfy any tolerance.
        let magnitude = a.iter().map(|v| v.abs()).fold(0.0f32, f32::max);
        assert!(
            magnitude > 1e-3,
            "attention output is degenerate at n={n} (max |value| {magnitude}); \
             the comparison above would hold for a module that does nothing"
        );
    }
}
