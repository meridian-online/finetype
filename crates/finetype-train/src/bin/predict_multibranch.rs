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
//!                       [--zero-char|--zero-embed|--zero-stats|--zero-header|--zero-valid]
//!
//! The five `--zero-*` flags are single-branch ablations: each replaces one branch's
//! input with zeros immediately before the forward, leaving every weight untouched, so
//! the run measures how much the trained trunk was leaning on that branch at inference.
//! Zeroing an input is NOT equivalent to retraining without the branch — the remaining
//! branches never got a chance to re-fit — so a low delta is a lower bound on the
//! branch's contribution, not a deletion warrant.
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

/// Which branch inputs to replace with zeros immediately before the forward.
///
/// One flag per branch of the five-branch trunk. All five inputs are dense f32, so
/// zeroing is well defined for every one of them: the branch still runs, it just sees
/// no signal, and whatever the trunk recovers came from the other four.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ZeroBranches {
    char_: bool,
    embed: bool,
    stats: bool,
    header: bool,
    valid: bool,
}

impl ZeroBranches {
    /// True when no branch is ablated — i.e. this is the un-ablated control run.
    fn none(&self) -> bool {
        *self == Self::default()
    }

    /// Space-separated flag names for the run banner, so a log line can never be
    /// mistaken for a control run (the failure mode a swallowed error produces).
    fn describe(&self) -> String {
        if self.none() {
            return "none (control)".to_string();
        }
        let mut v = Vec::new();
        for (on, name) in [
            (self.char_, "char"),
            (self.embed, "embed"),
            (self.stats, "stats"),
            (self.header, "header"),
            (self.valid, "valid"),
        ] {
            if on {
                v.push(name);
            }
        }
        v.join(" ")
    }
}

/// Parsed command line. Split out of `main` so the flag wiring is unit-testable
/// without a model, an FTMB, or a forward pass.
#[derive(Debug, Default, Clone, PartialEq)]
struct Args {
    model_dir: Option<PathBuf>,
    data: Option<PathBuf>,
    out: Option<PathBuf>,
    use_sibling: bool,
    zero: ZeroBranches,
    /// tau; subtract tau*log(prior) from logits before argmax
    logit_adjust: f64,
    priors_path: Option<PathBuf>,
    value_encoder: Option<PathBuf>,
}

fn parse_args<I: Iterator<Item = String>>(iter: I) -> Result<Args> {
    let mut parsed = Args {
        use_sibling: true,
        ..Default::default()
    };
    let mut args = iter;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--model" => parsed.model_dir = args.next().map(PathBuf::from),
            "--data" => parsed.data = args.next().map(PathBuf::from),
            "--out" => parsed.out = args.next().map(PathBuf::from),
            // Value-encoder dir for the per-value attention pool (choice 0106) — the
            // model's config has a value_attention block and the FTMB is v6.
            "--value-encoder" => parsed.value_encoder = args.next().map(PathBuf::from),
            "--no-sibling" => parsed.use_sibling = false,
            // Ablations: replace one branch's input with zeros before the forward, so
            // the model decides on the other four only. If format types recover vs the
            // un-ablated run, that branch was overriding them.
            "--zero-char" => parsed.zero.char_ = true,
            "--zero-embed" => parsed.zero.embed = true,
            "--zero-stats" => parsed.zero.stats = true,
            "--zero-header" => parsed.zero.header = true,
            "--zero-valid" => parsed.zero.valid = true,
            // Post-hoc logit adjustment: logit_c -= tau * log(prior_c). Down-weights
            // frequent classes (the decimal_number / entity_name attractors) at
            // inference only — no retrain. --priors is a "label<TAB>train_count" TSV.
            "--logit-adjust" => {
                parsed.logit_adjust = args.next().context("--logit-adjust needs tau")?.parse()?
            }
            "--priors" => parsed.priors_path = args.next().map(PathBuf::from),
            other => bail!("unknown arg: {other}"),
        }
    }
    Ok(parsed)
}

/// Replace `t` with zeros of the same shape/dtype/device when `zero` is set.
fn zero_if(t: Tensor, zero: bool) -> candle_core::Result<Tensor> {
    if zero {
        t.zeros_like()
    } else {
        Ok(t)
    }
}

/// Optional-input variant, for the header and validation branches.
///
/// A `None` input is left as `None`: `MultiBranchModel::forward_trunk` already
/// substitutes a zero tensor of the configured branch width for a missing optional
/// input, so `None` and `Some(zeros)` drive the branch identically.
fn zero_if_opt(t: Option<Tensor>, zero: bool) -> candle_core::Result<Option<Tensor>> {
    match (t, zero) {
        (Some(t), true) => Ok(Some(t.zeros_like()?)),
        (other, _) => Ok(other),
    }
}

fn main() -> Result<()> {
    let Args {
        model_dir,
        data,
        out,
        use_sibling,
        zero,
        logit_adjust,
        priors_path,
        value_encoder,
    } = parse_args(std::env::args().skip(1))?;

    let model_dir = model_dir.context("--model required")?;
    let data = data.context("--data required")?;
    let out = out.context("--out required")?;
    eprintln!("ablation: zeroed branches = {}", zero.describe());

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
        let c = zero_if(c, zero.char_)?;
        let s = zero_if(s, zero.stats)?;
        let h = zero_if_opt(h, zero.header)?;
        let v = zero_if_opt(v, zero.valid)?;
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
        // Applied AFTER the value-attention widening so the zeroed tensor is exactly
        // the tensor the embed branch consumes (blender ‖ pool when attention is on).
        let e = zero_if(e, zero.embed)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(argv: &[&str]) -> Args {
        parse_args(argv.iter().map(|s| s.to_string())).expect("parse")
    }

    const BASE: [&str; 6] = ["--model", "m", "--data", "d", "--out", "o"];

    fn parse_with(extra: &[&str]) -> Args {
        let mut argv: Vec<&str> = BASE.to_vec();
        argv.extend_from_slice(extra);
        parse(&argv)
    }

    #[test]
    fn no_zero_flag_is_the_control() {
        let a = parse_with(&[]);
        assert_eq!(a.zero, ZeroBranches::default());
        assert!(a.zero.none());
        assert_eq!(a.zero.describe(), "none (control)");
        assert_eq!(a.model_dir, Some(PathBuf::from("m")));
        assert_eq!(a.data, Some(PathBuf::from("d")));
        assert_eq!(a.out, Some(PathBuf::from("o")));
        assert!(a.use_sibling);
    }

    /// Each flag sets its own branch and no other — the failure that would silently
    /// mislabel a whole ablation table.
    #[test]
    fn each_zero_flag_sets_exactly_its_own_branch() {
        let cases: [(&str, ZeroBranches); 5] = [
            (
                "--zero-char",
                ZeroBranches {
                    char_: true,
                    ..Default::default()
                },
            ),
            (
                "--zero-embed",
                ZeroBranches {
                    embed: true,
                    ..Default::default()
                },
            ),
            (
                "--zero-stats",
                ZeroBranches {
                    stats: true,
                    ..Default::default()
                },
            ),
            (
                "--zero-header",
                ZeroBranches {
                    header: true,
                    ..Default::default()
                },
            ),
            (
                "--zero-valid",
                ZeroBranches {
                    valid: true,
                    ..Default::default()
                },
            ),
        ];
        for (flag, expected) in cases {
            let a = parse_with(&[flag]);
            assert_eq!(a.zero, expected, "{flag} set the wrong branch");
            assert!(!a.zero.none(), "{flag} still reads as the control");
            assert_eq!(a.zero.describe(), flag.trim_start_matches("--zero-"));
        }
    }

    #[test]
    fn zero_flags_compose_and_do_not_disturb_other_args() {
        let a = parse_with(&["--zero-char", "--zero-valid", "--no-sibling"]);
        assert_eq!(
            a.zero,
            ZeroBranches {
                char_: true,
                valid: true,
                ..Default::default()
            }
        );
        assert_eq!(a.zero.describe(), "char valid");
        assert!(!a.use_sibling);
        assert_eq!(a.logit_adjust, 0.0);
    }

    #[test]
    fn unknown_zero_flag_is_rejected() {
        // A typo must fail loudly rather than silently score an un-ablated run.
        assert!(parse_args(
            ["--model", "m", "--zero-headers"]
                .iter()
                .map(|s| s.to_string())
        )
        .is_err());
    }

    #[test]
    fn zero_if_blanks_the_tensor_only_when_asked() -> Result<()> {
        let dev = Device::Cpu;
        let t = Tensor::from_vec(vec![1.0f32, -2.0, 3.5, 4.0], (2, 2), &dev)?;

        let kept = zero_if(t.clone(), false)?;
        assert_eq!(
            kept.to_vec2::<f32>()?,
            vec![vec![1.0, -2.0], vec![3.5, 4.0]]
        );

        let blanked = zero_if(t.clone(), true)?;
        assert_eq!(blanked.dims(), t.dims());
        assert_eq!(blanked.dtype(), t.dtype());
        assert_eq!(
            blanked.to_vec2::<f32>()?,
            vec![vec![0.0, 0.0], vec![0.0, 0.0]]
        );
        Ok(())
    }

    #[test]
    fn zero_if_opt_blanks_some_and_leaves_none_alone() -> Result<()> {
        let dev = Device::Cpu;
        let t = Tensor::from_vec(vec![5.0f32, 6.0, 7.0], (1, 3), &dev)?;

        let blanked = zero_if_opt(Some(t.clone()), true)?.expect("some");
        assert_eq!(blanked.dims(), t.dims());
        assert_eq!(blanked.to_vec2::<f32>()?, vec![vec![0.0, 0.0, 0.0]]);

        let kept = zero_if_opt(Some(t.clone()), false)?.expect("some");
        assert_eq!(kept.to_vec2::<f32>()?, vec![vec![5.0, 6.0, 7.0]]);

        // `None` stays `None`: forward_trunk substitutes a zero tensor of the
        // configured branch width, so it is already the ablated input.
        assert!(zero_if_opt(None, true)?.is_none());
        assert!(zero_if_opt(None, false)?.is_none());
        Ok(())
    }
}
