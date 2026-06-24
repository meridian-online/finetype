//! Predict with a trained multi-branch model over an FTMB of PRECOMPUTED features.
//!
//! This is the ac-03 offline scoring path for the gte-embed-swap (spec
//! 2026-06-20-gte-tiny-embed-branch-swap): the normal `profile`/`predict` route
//! computes Model2Vec (512-dim) embed features in-Rust and cannot drive a gte model
//! (1536-dim). Here the features are precomputed into the FTMB by build_ftmb_v5_gte.py
//! and we run the EXACT training forward (MultiBranchDataset::batch_groups, which
//! applies frozen sibling-context per multi-column group, then MultiBranchModel::forward).
//!
//! Each record's label field is treated as a join key and echoed verbatim:
//!   - floor FTMB: it's the true type label -> lets this tool self-verify by
//!     reproducing ~train accuracy (proves the forward is correct).
//!   - gold/repr FTMB: a "sha\tcolumn" join id, one singleton group per column
//!     (singletons skip sibling-context, matching the single-column `profile` path
//!     v19 was scored on).
//!
//! Usage:
//!   predict_multibranch --model <dir> --data <ftmb> --out <tsv> [--no-sibling]
//!
//! Output TSV: join_key<TAB>predicted_label<TAB>confidence

use anyhow::{bail, Context, Result};
use candle_core::{DType, Device, Tensor, D};
use candle_nn::ops::softmax;
use candle_nn::VarBuilder;
use finetype_model::model2vec_shared::Model2VecResources;
use finetype_train::multi_branch::{
    read_training_data, FrozenSiblingContext, MultiBranchConfig, MultiBranchDataset,
    MultiBranchModel,
};
use std::collections::HashMap;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const SIBLING_DIR: &str = "models/sibling-context";
const GROUP_CHUNK: usize = 64; // groups per forward batch

fn main() -> Result<()> {
    let mut model_dir: Option<PathBuf> = None;
    let mut data: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut use_sibling = true;
    let mut zero_embed = false;
    let mut logit_adjust = 0.0f64; // tau; subtract tau*log(prior) from logits before argmax
    let mut priors_path: Option<PathBuf> = None;
    let mut value_encoder: Option<PathBuf> = None;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => model_dir = args.next().map(PathBuf::from),
            "--data" => data = args.next().map(PathBuf::from),
            "--out" => out = args.next().map(PathBuf::from),
            // Value-encoder dir for the per-value attention pool (choice 0106) — the
            // model's config has a value_attention block and the FTMB is v6.
            "--value-encoder" => value_encoder = args.next().map(PathBuf::from),
            "--no-sibling" => use_sibling = false,
            // Ablation: replace the embed branch input with zeros before the forward,
            // so the model decides on char/stats/header/validation only. If format
            // types recover vs the un-ablated run, the embed was overriding them.
            "--zero-embed" => zero_embed = true,
            // Post-hoc logit adjustment: logit_c -= tau * log(prior_c). Down-weights
            // frequent classes (the decimal_number / entity_name attractors) at
            // inference only — no retrain. --priors is a "label<TAB>train_count" TSV.
            "--logit-adjust" => {
                logit_adjust = args.next().context("--logit-adjust needs tau")?.parse()?
            }
            "--priors" => priors_path = args.next().map(PathBuf::from),
            other => bail!("unknown arg: {other}"),
        }
    }
    let model_dir = model_dir.context("--model required")?;
    let data = data.context("--data required")?;
    let out = out.context("--out required")?;

    let device = Device::Cpu;

    // ── Config + labels ────────────────────────────────────────────────
    let config: MultiBranchConfig = serde_json::from_slice(
        &std::fs::read(model_dir.join("config.json")).context("read config.json")?,
    )
    .context("parse config.json")?;
    let model_labels: Vec<String> = serde_json::from_slice(
        &std::fs::read(model_dir.join("label_map.json")).context("read label_map.json")?,
    )
    .context("parse label_map.json")?;
    eprintln!(
        "model: {} classes, embed_dim={}, valid_dim={}",
        model_labels.len(),
        config.embed_dim,
        config.valid_dim
    );

    // ── Logit-adjustment vector: tau * log(prior_c), aligned to model_labels ──
    let adjust: Option<Tensor> = if logit_adjust != 0.0 {
        let path = priors_path.context("--logit-adjust requires --priors")?;
        let mut counts: HashMap<String, f64> = HashMap::new();
        for line in std::fs::read_to_string(&path)?.lines() {
            if let Some((lab, cnt)) = line.split_once('\t') {
                counts.insert(lab.to_string(), cnt.trim().parse().unwrap_or(0.0));
            }
        }
        let total: f64 = counts.values().sum::<f64>().max(1.0);
        let floor = 1.0 / total; // unseen classes get a tiny prior, not -inf
        let adj: Vec<f32> = model_labels
            .iter()
            .map(|l| {
                let p = (counts.get(l).copied().unwrap_or(0.0) / total).max(floor);
                (logit_adjust * p.ln()) as f32
            })
            .collect();
        eprintln!(
            "logit-adjust: tau={logit_adjust} over {} priors",
            counts.len()
        );
        Some(Tensor::from_vec(adj, model_labels.len(), &device)?)
    } else {
        None
    };

    // ── Model weights ──────────────────────────────────────────────────
    let weights = model_dir.join("model.safetensors");
    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights], DType::F32, &device)? };
    let model = MultiBranchModel::new(&config, vb)?;

    // ── Frozen sibling-context (skipped for singleton groups anyway) ────
    let sibling = if use_sibling {
        match FrozenSiblingContext::load(Path::new(SIBLING_DIR), &device) {
            Ok(c) => {
                eprintln!("sibling-context: loaded from {SIBLING_DIR}");
                Some(c)
            }
            Err(e) => {
                eprintln!("sibling-context: not loaded ({e}); headers pass through raw");
                None
            }
        }
    } else {
        None
    };

    // ── Data ───────────────────────────────────────────────────────────
    let (header, records, table_groups) = read_training_data(&data)?;
    eprintln!(
        "data: {} records, {} groups, dims char/embed/stats/header/valid={}/{}/{}/{}/{}",
        records.len(),
        table_groups.len(),
        header.char_dim,
        header.embed_dim,
        header.stats_dim,
        header.header_dim,
        header.valid_dim
    );
    if header.embed_dim as usize != config.embed_dim {
        bail!(
            "FTMB embed_dim {} != model embed_dim {} — wrong feature binary for this model",
            header.embed_dim,
            config.embed_dim
        );
    }

    // label_to_idx only needs to cover the record labels so dataset construction
    // does not error; its values are irrelevant (we decode predictions via
    // model_labels and join via the record's label string directly).
    let mut label_to_idx: HashMap<String, u32> = HashMap::new();
    for r in &records {
        let n = label_to_idx.len() as u32;
        label_to_idx.entry(r.label.clone()).or_insert(n);
    }

    let ds = MultiBranchDataset::from_records_with_groups(
        &records,
        &label_to_idx,
        header.char_dim as usize,
        header.embed_dim as usize,
        header.stats_dim as usize,
        header.header_dim as usize,
        header.valid_dim as usize,
        Some(table_groups),
    )?;

    // Value attention (choice 0106): encode the FTMB v6 value strings once with the
    // value encoder so the forward below matches the native classifier exactly.
    let ds = if let Some(va) = config.value_attention.clone() {
        let enc_dir = value_encoder
            .as_ref()
            .context("model config has value_attention but --value-encoder not given")?;
        let enc = Model2VecResources::load(enc_dir)
            .with_context(|| format!("load value encoder {}", enc_dir.display()))?;
        eprintln!(
            "value attention: encoding up to {} values/col with {} ({}d)",
            va.n_values,
            enc_dir.display(),
            va.value_embed_dim
        );
        ds.with_value_attention(&records, &va, &enc)?
    } else {
        ds
    };

    // ── Predict, group-chunk at a time ─────────────────────────────────
    let f = std::fs::File::create(&out).context("create out")?;
    let mut w = BufWriter::new(f);
    writeln!(w, "join_key\tpredicted_label\tconfidence")?;

    let n_groups = ds.table_groups.len();
    let mut correct = 0usize;
    let mut total = 0usize;
    let mut gi = 0usize;
    while gi < n_groups {
        let end = (gi + GROUP_CHUNK).min(n_groups);
        let chunk: Vec<usize> = (gi..end).collect();
        let (c, e, s, h, v, _labels) = ds.batch_groups(&chunk, sibling.as_ref(), &device)?;
        // Widen the embed input to blender ‖ pool when value attention is on, using
        // the same group→record expansion batch_groups produced.
        let e = if ds.has_value_attention() {
            let idxs = ds.expand_group_indices(&chunk);
            let vbt = ds.value_batch(&idxs, &device)?;
            let (ve, vm) = match &vbt {
                Some((a, b)) => (Some(a), Some(b)),
                None => (None, None),
            };
            model.embed_input(&e, ve, vm, false)?
        } else {
            e
        };
        let e = if zero_embed { e.zeros_like()? } else { e };
        let logits = model.forward(&c, &e, &s, h.as_ref(), v.as_ref(), false)?;
        let logits = match &adjust {
            Some(adj) => logits.broadcast_sub(adj)?, // logit_c -= tau*log(prior_c)
            None => logits,
        };
        let probs = softmax(&logits, D::Minus1)?;
        let pred_idx: Vec<u32> = logits.argmax(D::Minus1)?.to_vec1()?;
        let conf: Vec<f32> = probs.max(D::Minus1)?.to_vec1()?;

        // Rows are in the order: for each group in chunk, its record_indices in order.
        let mut row = 0usize;
        for &g in &chunk {
            for &rec_idx in &ds.table_groups[g].record_indices {
                let join = &records[rec_idx].label;
                let pidx = pred_idx[row] as usize;
                let plabel = model_labels.get(pidx).map(|s| s.as_str()).unwrap_or("?");
                writeln!(w, "{join}\t{plabel}\t{:.4}", conf[row])?;
                if join == plabel {
                    correct += 1;
                }
                total += 1;
                row += 1;
            }
        }
        gi = end;
    }
    w.flush()?;
    eprintln!("wrote {total} predictions -> {}", out.display());
    // Self-check accuracy: meaningful only when record labels are true type labels
    // (floor/training FTMB). For gold join-id labels this is ~0 and ignored.
    eprintln!(
        "label-match accuracy (vs record label): {:.4} ({correct}/{total})",
        correct as f64 / total.max(1) as f64
    );
    Ok(())
}
