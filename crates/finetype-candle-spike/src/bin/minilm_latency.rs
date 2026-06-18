//! ac-00 of spec 2026-06-18-minilm-encoder-build: confirm all-MiniLM-L6-v2 runs
//! in Rust/candle at the low-band latency budget (<= ~10 ms/col CPU) BEFORE any
//! training is spent. Loads the off-the-shelf model (candle-transformers BERT +
//! tokenizers + safetensors), mean-pools, and times single-column inference — the
//! low-band escalation regime where the encoder fires only on uncertain columns.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

use anyhow::{Context, Result};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use tokenizers::Tokenizer;

fn main() -> Result<()> {
    let snap = std::env::var("MINILM_DIR").unwrap_or_else(|_| {
        format!(
            "{}/.cache/huggingface/hub/models--sentence-transformers--all-MiniLM-L6-v2/snapshots/1110a243fdf4706b3f48f1d95db1a4f5529b4d41",
            std::env::var("HOME").unwrap()
        )
    });
    let probe = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "output/fine-tuned-encoder-discovery/probe_data.tsv".to_string());

    let device = Device::Cpu;
    let config: Config =
        serde_json::from_reader(File::open(format!("{snap}/config.json")).context("config.json")?)?;
    let mut tokenizer =
        Tokenizer::from_file(format!("{snap}/tokenizer.json")).map_err(|e| anyhow::anyhow!(e))?;
    // pad/truncate to a fixed window — the escalation feeds ~8 values + header
    tokenizer
        .with_truncation(Some(tokenizers::TruncationParams {
            max_length: 128,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!(e))?;
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[format!("{snap}/model.safetensors")], DTYPE, &device)?
    };
    let model = BertModel::load(vb, &config)?;

    // load probe texts (column "text" is the first tab-delimited field)
    let mut texts: Vec<String> = Vec::new();
    for (i, line) in BufReader::new(File::open(&probe).context("probe_data.tsv")?)
        .lines()
        .enumerate()
    {
        let line = line?;
        if i == 0 {
            continue;
        } // header
        if let Some(t) = line.split('\t').next() {
            if !t.is_empty() {
                texts.push(t.to_string());
            }
        }
    }
    anyhow::ensure!(!texts.is_empty(), "no probe texts loaded from {probe}");

    let embed = |text: &str| -> Result<Vec<f32>> {
        let enc = tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!(e))?;
        let ids: Vec<u32> = enc.get_ids().to_vec();
        let mask: Vec<u32> = enc.get_attention_mask().to_vec();
        let n = ids.len();
        let input_ids = Tensor::from_vec(ids, (1, n), &device)?;
        let token_type_ids = input_ids.zeros_like()?;
        let attn = Tensor::from_vec(mask, (1, n), &device)?.to_dtype(DType::F32)?;
        let out = model.forward(&input_ids, &token_type_ids, Some(&attn))?; // [1, T, H]
                                                                            // masked mean pool
        let mask3 = attn.unsqueeze(2)?; // [1, T, 1]
        let summed = out.broadcast_mul(&mask3)?.sum(1)?; // [1, H]
        let cnt = mask3.sum(1)?; // [1, 1]
        let pooled = summed.broadcast_div(&cnt)?;
        Ok(pooled.flatten_all()?.to_vec1::<f32>()?)
    };

    // warmup
    for t in texts.iter().take(5) {
        embed(t)?;
    }
    // determinism check
    let a = embed(&texts[0])?;
    let b = embed(&texts[0])?;
    let deterministic = a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-6);

    // time single-column (low-band escalation, interactive realism)
    let reps = 200usize;
    let start = Instant::now();
    for i in 0..reps {
        embed(&texts[i % texts.len()])?;
    }
    let ms_single = start.elapsed().as_secs_f64() * 1000.0 / reps as f64;

    // batched throughput (corpus-pass regime: low-band columns batched together)
    tokenizer.with_padding(Some(tokenizers::PaddingParams::default()));
    let batch_embed = |batch: &[String]| -> Result<usize> {
        let encs = tokenizer
            .encode_batch(batch.to_vec(), true)
            .map_err(|e| anyhow::anyhow!(e))?;
        let t = encs[0].get_ids().len();
        let b = encs.len();
        let mut ids = Vec::with_capacity(b * t);
        let mut msk = Vec::with_capacity(b * t);
        for e in &encs {
            ids.extend_from_slice(e.get_ids());
            msk.extend_from_slice(e.get_attention_mask());
        }
        let input_ids = Tensor::from_vec(ids, (b, t), &device)?;
        let token_type_ids = input_ids.zeros_like()?;
        let attn = Tensor::from_vec(msk, (b, t), &device)?.to_dtype(DType::F32)?;
        let out = model.forward(&input_ids, &token_type_ids, Some(&attn))?;
        let mask3 = attn.unsqueeze(2)?;
        let _ = out
            .broadcast_mul(&mask3)?
            .sum(1)?
            .broadcast_div(&mask3.sum(1)?)?;
        Ok(b)
    };
    let bs = 32usize;
    let batch: Vec<String> = (0..bs).map(|i| texts[i % texts.len()].clone()).collect();
    batch_embed(&batch)?; // warmup
    let nb = 10usize;
    let start = Instant::now();
    for _ in 0..nb {
        batch_embed(&batch)?;
    }
    let ms_batch = start.elapsed().as_secs_f64() * 1000.0 / (nb * bs) as f64;

    let corpus_low = 6.6e6 * 0.30;
    println!("MiniLM (candle, CPU, fp32) — dim {}", a.len());
    println!("  single-column : {ms_single:.2} ms/col  (interactive low-band; torch was 6.7)");
    println!("  batched(32)   : {ms_batch:.2} ms/col  (corpus-pass low-band regime)");
    println!("  deterministic : {deterministic}");
    println!(
        "  corpus low-band (30% of 6.6M): single {:.1}h | batched {:.1}h  (every-column baseline ~1.6h)",
        corpus_low * ms_single / 1000.0 / 3600.0,
        corpus_low * ms_batch / 1000.0 / 3600.0
    );
    println!(
        "  budget verdict (<=10ms/col): single {} | batched {}",
        if ms_single <= 10.0 { "PASS" } else { "OVER" },
        if ms_batch <= 10.0 { "PASS" } else { "OVER" }
    );
    Ok(())
}
