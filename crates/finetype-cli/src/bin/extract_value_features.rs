//! Per-value feature exporter for the value-level YDF labelling pipeline
//! (spec 2026-06-04-value-level-ydf-labelling, ac-00).
//!
//! Reads NDJSON (one `{"text": "...", "classification": "..."}` per line;
//! `classification` optional) and writes a CSV whose columns are, in fixed order:
//!
//!   classification, text, <37 deterministic feature names>, schema_pass:<type>...
//!
//! The feature block is produced by `finetype_model::value_feature_row`, the one
//! contract shared by the synthetic-prior path (ac-01) and the gittables-scoring
//! path (ac-02). `--emit-manifest <path>` writes the ordered feature names (the
//! block after `classification,text`) so downstream readers pin the same order.
//!
//! Usage:
//!   extract_value_features --input values.ndjson --output features.csv \
//!       [--emit-manifest manifest.txt]
//!   extract_value_features --emit-manifest manifest.txt   # manifest only

use anyhow::{Context, Result};
use finetype_core::Taxonomy;
use finetype_model::{value_feature_names, value_feature_row, ValidationFeatureExtractor};
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;

struct Args {
    input: Option<String>,
    output: Option<String>,
    manifest: Option<String>,
}

fn parse_args() -> Result<Args> {
    let mut input = None;
    let mut output = None;
    let mut manifest = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--input" => input = it.next(),
            "--output" => output = it.next(),
            "--emit-manifest" => manifest = it.next(),
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }
    Ok(Args {
        input,
        output,
        manifest,
    })
}

fn load_taxonomy() -> Result<Taxonomy> {
    let mut taxonomy =
        Taxonomy::from_directory(&PathBuf::from("labels")).context("loading taxonomy from labels/")?;
    taxonomy.compile_validators();
    taxonomy.compile_locale_validators();
    Ok(taxonomy)
}

/// Minimal NDJSON field reader — pulls a single string field without a JSON dep
/// already in this bin's tree. Returns None if the key is absent.
fn json_str_field(line: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let start = line.find(&pat)? + pat.len();
    let rest = &line[start..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let mut chars = after.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for c in chars {
        if escaped {
            match c {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                other => out.push(other),
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
}

fn main() -> Result<()> {
    let args = parse_args()?;

    let taxonomy = load_taxonomy()?;
    let extractor = ValidationFeatureExtractor::new(&taxonomy);
    let feature_names = value_feature_names(&extractor);
    eprintln!(
        "Per-value contract: {} dims (37 deterministic + {} schema)",
        feature_names.len(),
        extractor.dim()
    );

    if let Some(path) = &args.manifest {
        let mut w = BufWriter::new(std::fs::File::create(path).context("creating manifest")?);
        for name in &feature_names {
            writeln!(w, "{name}")?;
        }
        w.flush()?;
        eprintln!("Wrote feature manifest ({} names) to {path}", feature_names.len());
    }

    let input = match args.input {
        Some(p) => p,
        None => {
            if args.manifest.is_some() {
                return Ok(()); // manifest-only run
            }
            anyhow::bail!("--input is required unless only --emit-manifest is given");
        }
    };
    let output = args
        .output
        .context("--output is required when --input is given")?;

    let reader: Box<dyn BufRead> = if input == "-" {
        Box::new(std::io::stdin().lock())
    } else {
        Box::new(std::io::BufReader::new(
            std::fs::File::open(&input).with_context(|| format!("opening {input}"))?,
        ))
    };
    let mut wtr = csv::WriterBuilder::new().from_path(&output)?;

    let mut header: Vec<String> = vec!["classification".into(), "text".into()];
    header.extend(feature_names.iter().cloned());
    wtr.write_record(&header)?;

    let mut n = 0usize;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let text = match json_str_field(&line, "text") {
            Some(t) => t,
            None => {
                eprintln!("  skipping line without a \"text\" field");
                continue;
            }
        };
        let label = json_str_field(&line, "classification").unwrap_or_default();

        let row_feats = value_feature_row(&text, &extractor, &taxonomy);
        let mut row: Vec<String> = vec![label, text];
        for v in row_feats {
            row.push(format!("{v:.6}"));
        }
        wtr.write_record(&row)?;
        n += 1;
        if n % 50_000 == 0 {
            eprintln!("  {n} values...");
        }
    }
    wtr.flush()?;
    eprintln!("Wrote {n} value rows to {output}");
    Ok(())
}
