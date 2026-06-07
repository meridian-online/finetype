//! CLI binary: train-fusion-head
//!
//! B3 late-fusion Sense head (spec 2026-06-08-late-fusion-sense-classifier, ac-03).
//!
//! Reads the dumped fusion features (produced by `finetype dump-fusion-features`):
//!   <stem>.f32         raw little-endian f32 blob, row-major [N, 968]
//!   <stem>.labels.tsv  per-row `row_idx \t label_index \t label_name`
//!   <stem>.meta.json   { row_dim, view2_dim, label_order: [240], layout, ... }
//!
//! Feature layout per row (968 = 728 view1 + 240 view2):
//!   mean(240) | max(240) | vote(240) | scalars(8) | mb_logits(240)
//!
//! Trains the residual fusion head:
//!   x(968) -> LayerNorm -> Linear(968->h1) -> GELU -> Dropout
//!          -> Linear(h1->h2) -> GELU -> Dropout -> Linear(h2->240) = head_logits
//!   fused = head_logits + alpha * mb_logits   (alpha learnable, init 1.0)
//!
//! The head starts equal to multi-branch (alpha=1, head≈0 has little effect early)
//! and learns a correction — the improve-or-hold lever for a general replacement.
//!
//! Writes to <output>/:
//!   model.safetensors  head weights (ln/l1/l2/l3 + alpha)
//!   config.json        dims, dropout, alpha_init, label_order
//!   metrics.json       best val acc, fused/view2-only/view1-only acc, per-class recall

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use candle_core::{DType, Device, Module, Tensor, D};
use candle_nn::{
    layer_norm, linear, AdamW, Init, LayerNorm, LayerNormConfig, Linear, Optimizer, ParamsAdamW,
    VarBuilder, VarMap,
};
use clap::Parser;
use serde_json::json;

use finetype_train::training::{compute_accuracy, cross_entropy_loss, EarlyStopping};

const ROW_DIM: usize = 968;
const N_CLASSES: usize = 240;
const MB_OFFSET: usize = 728; // start of mb_logits within the 968-dim row

#[derive(Parser, Debug)]
#[command(name = "train-fusion-head", about = "Train/predict the B3 late-fusion residual head")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(clap::Subcommand, Debug)]
enum Cmd {
    /// Train the residual head over a dumped feature blob.
    Train(TrainArgs),
    /// Run a frozen head over a feature blob and emit per-row predicted labels.
    Predict(PredictArgs),
}

#[derive(Parser, Debug)]
struct TrainArgs {
    /// Feature stem; reads <stem>.f32 / <stem>.labels.tsv / <stem>.meta.json
    #[arg(long)]
    features: PathBuf,

    /// Output directory for the frozen head
    #[arg(long, default_value = "models/fusion-head-v25")]
    output: PathBuf,

    #[arg(long, default_value = "512")]
    hidden1: usize,

    #[arg(long, default_value = "256")]
    hidden2: usize,

    #[arg(long, default_value = "0.1")]
    dropout: f32,

    #[arg(long, default_value = "1.0")]
    alpha_init: f64,

    #[arg(long, default_value = "60")]
    epochs: usize,

    #[arg(long, default_value = "1e-3")]
    lr: f64,

    #[arg(long, default_value = "0.01")]
    weight_decay: f64,

    #[arg(long, default_value = "512")]
    batch_size: usize,

    #[arg(long, default_value = "10")]
    patience: usize,

    #[arg(long, default_value = "0.15")]
    val_fraction: f64,

    #[arg(long, default_value = "42")]
    seed: u64,
}

#[derive(Parser, Debug)]
struct PredictArgs {
    /// Frozen head dir (config.json + model.safetensors from a prior `train`)
    #[arg(long)]
    head: PathBuf,

    /// Feature stem; reads <stem>.f32 / <stem>.meta.json
    #[arg(long)]
    features: PathBuf,

    /// Output TSV: `row_idx \t pred_label`
    #[arg(long)]
    out: PathBuf,
}

/// Residual fusion head. Dropout is applied only when `train` is true.
struct FusionHead {
    ln: LayerNorm,
    l1: Linear,
    l2: Linear,
    l3: Linear,
    alpha: Tensor,
    dropout: f32,
}

impl FusionHead {
    fn new(vb: VarBuilder, h1: usize, h2: usize, dropout: f32, alpha_init: f64) -> Result<Self> {
        let ln = layer_norm(ROW_DIM, LayerNormConfig::default(), vb.pp("ln"))?;
        let l1 = linear(ROW_DIM, h1, vb.pp("l1"))?;
        let l2 = linear(h1, h2, vb.pp("l2"))?;
        let l3 = linear(h2, N_CLASSES, vb.pp("l3"))?;
        let alpha = vb.get_with_hints(1, "alpha", Init::Const(alpha_init))?;
        Ok(Self { ln, l1, l2, l3, alpha, dropout })
    }

    fn forward(&self, x: &Tensor, train: bool) -> Result<Tensor> {
        let mb_logits = x.narrow(1, MB_OFFSET, N_CLASSES)?;
        let h = self.ln.forward(x)?;
        let h = self.l1.forward(&h)?.gelu_erf()?;
        let h = if train {
            candle_nn::ops::dropout(&h, self.dropout)?
        } else {
            h
        };
        let h = self.l2.forward(&h)?.gelu_erf()?;
        let h = if train {
            candle_nn::ops::dropout(&h, self.dropout)?
        } else {
            h
        };
        let head_logits = self.l3.forward(&h)?;
        // fused = head_logits + alpha * mb_logits
        let scaled = mb_logits.broadcast_mul(&self.alpha)?;
        Ok((head_logits + scaled)?)
    }
}

fn read_features(stem: &Path) -> Result<(Vec<f32>, Vec<u32>, usize, Vec<String>)> {
    let f32_path = with_ext(stem, "f32");
    let labels_path = with_ext(stem, "labels.tsv");
    let meta_path = with_ext(stem, "meta.json");

    let meta: serde_json::Value = serde_json::from_slice(
        &fs::read(&meta_path).with_context(|| format!("read {meta_path:?}"))?,
    )?;
    let row_dim = meta["row_dim"].as_u64().context("meta.row_dim")? as usize;
    if row_dim != ROW_DIM {
        bail!("meta row_dim {row_dim} != expected {ROW_DIM}");
    }
    let label_order: Vec<String> = meta["label_order"]
        .as_array()
        .context("meta.label_order")?
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();
    if label_order.len() != N_CLASSES {
        bail!("label_order len {} != {N_CLASSES}", label_order.len());
    }

    let raw = fs::read(&f32_path).with_context(|| format!("read {f32_path:?}"))?;
    if raw.len() % (ROW_DIM * 4) != 0 {
        bail!("{f32_path:?} size {} not a multiple of row stride", raw.len());
    }
    let n_rows = raw.len() / (ROW_DIM * 4);
    let mut feats = Vec::with_capacity(n_rows * ROW_DIM);
    for chunk in raw.chunks_exact(4) {
        feats.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    let labels_txt = fs::read_to_string(&labels_path).with_context(|| format!("read {labels_path:?}"))?;
    let mut labels = Vec::with_capacity(n_rows);
    for line in labels_txt.lines() {
        if line.starts_with("row_idx\t") {
            continue; // header
        }
        let mut it = line.split('\t');
        let _row = it.next();
        let idx: u32 = it.next().context("labels label_index")?.parse()?;
        labels.push(idx);
    }
    if labels.len() != n_rows {
        bail!("labels {} != feature rows {n_rows}", labels.len());
    }
    Ok((feats, labels, n_rows, label_order))
}

fn with_ext(stem: &Path, ext: &str) -> PathBuf {
    let mut s = stem.as_os_str().to_os_string();
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

/// Deterministic shuffle (xorshift) → train/val index split.
fn split_indices(n: usize, val_fraction: f64, seed: u64) -> (Vec<usize>, Vec<usize>) {
    let mut idx: Vec<usize> = (0..n).collect();
    let mut state = seed.max(1);
    for i in (1..n).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state % (i as u64 + 1)) as usize;
        idx.swap(i, j);
    }
    let n_val = ((n as f64) * val_fraction).round() as usize;
    let val = idx[..n_val].to_vec();
    let train = idx[n_val..].to_vec();
    (train, val)
}

fn gather_rows(feats: &[f32], labels: &[u32], idx: &[usize], device: &Device) -> Result<(Tensor, Tensor)> {
    let mut xb = Vec::with_capacity(idx.len() * ROW_DIM);
    let mut yb = Vec::with_capacity(idx.len());
    for &i in idx {
        xb.extend_from_slice(&feats[i * ROW_DIM..(i + 1) * ROW_DIM]);
        yb.push(labels[i]);
    }
    let x = Tensor::from_vec(xb, (idx.len(), ROW_DIM), device)?;
    let y = Tensor::from_vec(yb, idx.len(), device)?;
    Ok((x, y))
}

/// Per-class recall from argmax predictions over a label vector.
fn per_class_recall(preds: &[u32], targets: &[u32], n_classes: usize) -> Vec<f32> {
    let mut tp = vec![0u32; n_classes];
    let mut tot = vec![0u32; n_classes];
    for (&p, &t) in preds.iter().zip(targets) {
        let t = t as usize;
        tot[t] += 1;
        if p == t as u32 {
            tp[t] += 1;
        }
    }
    (0..n_classes)
        .map(|c| if tot[c] == 0 { f32::NAN } else { tp[c] as f32 / tot[c] as f32 })
        .collect()
}

fn argmax_rows(x: &Tensor) -> Result<Vec<u32>> {
    Ok(x.argmax(D::Minus1)?.to_dtype(DType::U32)?.to_vec1::<u32>()?)
}

fn read_blob(stem: &Path) -> Result<(Vec<f32>, usize)> {
    let f32_path = with_ext(stem, "f32");
    let raw = fs::read(&f32_path).with_context(|| format!("read {f32_path:?}"))?;
    if raw.len() % (ROW_DIM * 4) != 0 {
        bail!("{f32_path:?} size {} not a multiple of row stride", raw.len());
    }
    let n_rows = raw.len() / (ROW_DIM * 4);
    let mut feats = Vec::with_capacity(n_rows * ROW_DIM);
    for chunk in raw.chunks_exact(4) {
        feats.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok((feats, n_rows))
}

fn run_predict(args: PredictArgs) -> Result<()> {
    let device = Device::Cpu;
    let cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(args.head.join("config.json"))?)?;
    let h1 = cfg["hidden1"].as_u64().context("config.hidden1")? as usize;
    let h2 = cfg["hidden2"].as_u64().context("config.hidden2")? as usize;
    let label_order: Vec<String> = cfg["label_order"]
        .as_array()
        .context("config.label_order")?
        .iter()
        .map(|v| v.as_str().unwrap_or_default().to_string())
        .collect();

    let weights = args.head.join("model.safetensors");
    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights.clone()], DType::F32, &device)
            .with_context(|| format!("load head {weights:?}"))?
    };
    let head = FusionHead::new(vb, h1, h2, 0.0, 1.0)?;

    let (feats, n_rows) = read_blob(&args.features)?;
    println!("predict: {n_rows} rows x {ROW_DIM}");

    let mut out = std::io::BufWriter::new(fs::File::create(&args.out)?);
    use std::io::Write;
    let batch = 4096usize;
    let mut i = 0usize;
    while i < n_rows {
        let j = (i + batch).min(n_rows);
        let x = Tensor::from_slice(&feats[i * ROW_DIM..j * ROW_DIM], (j - i, ROW_DIM), &device)?;
        let logits = head.forward(&x, false)?;
        let preds = argmax_rows(&logits)?;
        for (k, &p) in preds.iter().enumerate() {
            let label = label_order.get(p as usize).map(|s| s.as_str()).unwrap_or("?");
            writeln!(out, "{}\t{}", i + k, label)?;
        }
        i = j;
    }
    out.flush()?;
    println!("wrote {} predictions -> {}", n_rows, args.out.display());
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Train(a) => run_train(a),
        Cmd::Predict(a) => run_predict(a),
    }
}

fn run_train(args: TrainArgs) -> Result<()> {
    let device = Device::Cpu;

    let (feats, labels, n_rows, label_order) = read_features(&args.features)?;
    println!("loaded {n_rows} rows x {ROW_DIM} f32; {} classes", label_order.len());

    let (train_idx, val_idx) = split_indices(n_rows, args.val_fraction, args.seed);
    println!("split: {} train, {} val", train_idx.len(), val_idx.len());

    let (xval, yval) = gather_rows(&feats, &labels, &val_idx, &device)?;
    let yval_u32: Vec<u32> = val_idx.iter().map(|&i| labels[i]).collect();

    // Baselines from the features themselves (no head): view2-only and view1-mean-only.
    let mb_val = xval.narrow(1, MB_OFFSET, N_CLASSES)?;
    let v2_preds = argmax_rows(&mb_val)?;
    let v2_acc = accuracy_from(&v2_preds, &yval_u32);
    let mean_val = xval.narrow(1, 0, N_CLASSES)?; // view1 mean-prob block
    let v1_preds = argmax_rows(&mean_val)?;
    let v1_acc = accuracy_from(&v1_preds, &yval_u32);
    println!("baseline val acc — view2(multi-branch)={v2_acc:.4}  view1(value-mean)={v1_acc:.4}");

    let varmap = VarMap::new();
    let vb = VarBuilder::from_varmap(&varmap, DType::F32, &device);
    let head = FusionHead::new(vb, args.hidden1, args.hidden2, args.dropout, args.alpha_init)?;

    let adamw = ParamsAdamW {
        lr: args.lr,
        weight_decay: args.weight_decay,
        ..Default::default()
    };
    let mut opt = AdamW::new(varmap.all_vars(), adamw)?;

    let mut early = EarlyStopping::new(args.patience, true);
    let mut best_val = 0f32;
    let mut best_recall: Vec<f32> = Vec::new();
    let mut best_alpha = args.alpha_init as f32;
    fs::create_dir_all(&args.output)?;
    let best_path = args.output.join("model.safetensors");

    let n_train = train_idx.len();
    let mut order = train_idx.clone();
    let mut rng = args.seed.max(1);

    for epoch in 0..args.epochs {
        // shuffle training order each epoch
        for i in (1..n_train).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let j = (rng % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }

        let mut running = 0f64;
        let mut nb = 0usize;
        for batch in order.chunks(args.batch_size) {
            let (xb, yb) = gather_rows(&feats, &labels, batch, &device)?;
            let logits = head.forward(&xb, true)?;
            let loss = cross_entropy_loss(&logits, &yb)?;
            opt.backward_step(&loss)?;
            running += loss.to_scalar::<f32>()? as f64;
            nb += 1;
        }

        let vlogits = head.forward(&xval, false)?;
        let vacc = compute_accuracy(&vlogits, &yval)?;
        let alpha_now = head.alpha.to_vec1::<f32>()?[0];
        println!(
            "epoch {:>2}/{}  train_loss={:.4}  val_acc={:.4}  alpha={:.3}",
            epoch + 1,
            args.epochs,
            running / nb.max(1) as f64,
            vacc,
            alpha_now
        );

        if vacc > best_val {
            best_val = vacc;
            best_alpha = alpha_now;
            let vpreds = argmax_rows(&vlogits)?;
            best_recall = per_class_recall(&vpreds, &yval_u32, N_CLASSES);
            varmap.save(&best_path)?;
        }
        if early.step(epoch, vacc) {
            println!("early stop at epoch {}", epoch + 1);
            break;
        }
    }

    // config.json + metrics.json
    let config = json!({
        "row_dim": ROW_DIM,
        "n_classes": N_CLASSES,
        "mb_offset": MB_OFFSET,
        "hidden1": args.hidden1,
        "hidden2": args.hidden2,
        "dropout": args.dropout,
        "alpha_init": args.alpha_init,
        "alpha_final": best_alpha,
        "label_order": label_order,
    });
    fs::write(args.output.join("config.json"), serde_json::to_string_pretty(&config)? + "\n")?;

    let family = [
        "geography.coordinate.latitude",
        "geography.coordinate.longitude",
        "representation.numeric.decimal_number",
        "representation.numeric.integer_number",
    ];
    let mut family_recall = BTreeMap::new();
    for name in family {
        if let Some(ci) = label_order.iter().position(|l| l == name) {
            let r = best_recall.get(ci).copied().unwrap_or(f32::NAN);
            family_recall.insert(name.to_string(), r);
        }
    }
    let recall_map: BTreeMap<String, f32> = label_order
        .iter()
        .cloned()
        .zip(best_recall.iter().copied())
        .collect();
    let metrics = json!({
        "best_val_acc_fused": best_val,
        "val_acc_view2_only": v2_acc,
        "val_acc_view1_mean_only": v1_acc,
        "alpha_final": best_alpha,
        "n_train": train_idx.len(),
        "n_val": val_idx.len(),
        "confusion_family_recall": family_recall,
        "per_class_recall": recall_map,
        "config": {
            "hidden1": args.hidden1, "hidden2": args.hidden2,
            "dropout": args.dropout, "alpha_init": args.alpha_init,
            "lr": args.lr, "weight_decay": args.weight_decay,
            "batch_size": args.batch_size, "epochs": args.epochs,
            "seed": args.seed, "val_fraction": args.val_fraction,
        },
    });
    fs::write(args.output.join("metrics.json"), serde_json::to_string_pretty(&metrics)? + "\n")?;

    println!();
    println!("=== fusion head done ===");
    println!("best val acc (fused):   {best_val:.4}");
    println!("  vs view2 (mb) only:   {v2_acc:.4}");
    println!("  vs view1 (val) only:  {v1_acc:.4}");
    println!("alpha final:            {best_alpha:.3}");
    println!("confusion family recall:");
    for (k, v) in &family_recall {
        println!("  {k:<44} {v:.3}");
    }
    println!("saved -> {}", args.output.display());
    Ok(())
}

fn accuracy_from(preds: &[u32], targets: &[u32]) -> f32 {
    let correct = preds.iter().zip(targets).filter(|(p, t)| p == t).count();
    correct as f32 / preds.len().max(1) as f32
}
