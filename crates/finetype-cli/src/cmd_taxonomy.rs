//! `cmd_taxonomy` — extracted from main.rs (mechanical split, no behaviour change).

use super::*;

// ═══════════════════════════════════════════════════════════════════════════════
// GENERATE — Create synthetic training data
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) fn cmd_generate(
    samples: usize,
    priority: u8,
    output: PathBuf,
    taxonomy_path: PathBuf,
    seed: u64,
    localized: bool,
) -> Result<()> {
    eprintln!("Loading taxonomy from {:?}", taxonomy_path);

    let taxonomy = load_taxonomy(&taxonomy_path)?;

    eprintln!(
        "Loaded {} label definitions across {} domains",
        taxonomy.len(),
        taxonomy.domains().len()
    );

    let mode = if localized {
        "localized (4-level)"
    } else {
        "flat (3-level)"
    };
    eprintln!(
        "Generating {} samples per label (priority >= {}, mode: {})",
        samples, priority, mode
    );

    let mut generator = Generator::with_seed(taxonomy, seed);
    let all_samples = if localized {
        generator.generate_all_localized(priority, samples)
    } else {
        generator.generate_all(priority, samples)
    };

    eprintln!("Generated {} total samples", all_samples.len());

    // Write to file
    let mut file = std::fs::File::create(&output)?;
    for sample in all_samples {
        let record = json!({
            "text": sample.text,
            "classification": sample.label,
        });
        writeln!(file, "{}", record)?;
    }

    eprintln!("Saved to {:?}", output);
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// TAXONOMY — Display taxonomy information
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) fn cmd_taxonomy(
    type_key: Option<String>,
    file: PathBuf,
    domain: Option<String>,
    category: Option<String>,
    priority: Option<u8>,
    output: OutputFormat,
    full: bool,
) -> Result<()> {
    let taxonomy = load_taxonomy(&file)?;

    // Collect matching definitions. A positional KEY takes precedence
    // over --domain / --category / --priority filters and uses the same
    // exact-match-or-glob predicate previously implemented in
    // `cmd_schema` (card 0006 absorbs that path).
    let mut defs: Vec<(&String, &finetype_core::Definition)> = if let Some(key) = &type_key {
        if key.contains('*') {
            // Glob: support "domain.*", "domain.category.*", "*", etc.
            let prefix = key.trim_end_matches(".*").trim_end_matches('*');
            taxonomy
                .definitions()
                .filter(|(k, _)| {
                    if prefix.is_empty() {
                        true
                    } else {
                        k.starts_with(prefix)
                            && (k.len() == prefix.len()
                                || k.as_bytes().get(prefix.len()) == Some(&b'.'))
                    }
                })
                .collect()
        } else {
            // Exact match — exit 1 with edit-distance suggestions on miss.
            match taxonomy.get(key) {
                Some(_) => taxonomy
                    .definitions()
                    .filter(|(k, _)| k.as_str() == key.as_str())
                    .collect(),
                None => {
                    let mut suggestions: Vec<(&String, usize)> = taxonomy
                        .definitions()
                        .map(|(k, _)| (k, levenshtein_distance(key, k)))
                        .collect();
                    suggestions.sort_by_key(|(_, d)| *d);
                    suggestions.truncate(5);

                    eprintln!("Error: unknown type '{}'", key);
                    if !suggestions.is_empty() {
                        eprintln!("\nDid you mean:");
                        for (s, _) in &suggestions {
                            eprintln!("  {}", s);
                        }
                    }
                    std::process::exit(1);
                }
            }
        }
    } else if let (Some(dom), Some(cat)) = (&domain, &category) {
        taxonomy.by_category(dom, cat)
    } else if let Some(dom) = &domain {
        taxonomy.by_domain(dom)
    } else if let Some(prio) = priority {
        taxonomy.at_priority(prio)
    } else {
        taxonomy.definitions().collect()
    };

    // Apply priority filter on top of domain/category. Skipped when a
    // positional KEY is supplied (the KEY is authoritative — it pins to
    // a single type or a glob and ignores priority).
    if type_key.is_none() {
        if let Some(prio) = priority {
            defs.retain(|(_, d)| d.release_priority >= prio);
        }
    }

    defs.sort_by_key(|(k, _)| (*k).clone());

    // Glob-with-zero-matches under positional KEY gets the same exit-1
    // contract as exact-key-with-zero-matches (already handled above).
    if type_key.is_some() && defs.is_empty() {
        eprintln!(
            "Error: no types matching '{}'",
            type_key.as_deref().unwrap_or("")
        );
        std::process::exit(1);
    }

    match output {
        OutputFormat::Plain
        | OutputFormat::Markdown
        | OutputFormat::Arrow
        | OutputFormat::Datapackage => {
            println!("Domains: {:?}", taxonomy.domains());
            println!("Total labels: {}", taxonomy.len());
            if let Some(dom) = &domain {
                println!("Categories in {}: {:?}", dom, taxonomy.categories(dom));
            }
            println!();

            for (key, def) in &defs {
                let broad = def.broad_type.as_deref().unwrap_or("?");
                println!("{} \u{2192} {}", key, broad);
                if let Some(title) = &def.title {
                    println!("  {}", title);
                }
            }

            println!("\n{} definitions shown", defs.len());
        }
        OutputFormat::Json => {
            let labels: Vec<_> = defs
                .iter()
                .map(|(key, d)| {
                    if full {
                        definition_to_full_json(key, d)
                    } else {
                        json!({
                            "key": key,
                            "title": d.title,
                            "broad_type": d.broad_type,
                            "designation": format!("{:?}", d.designation),
                            "priority": d.release_priority,
                            "transform": d.transform,
                            "locales": d.locales,
                        })
                    }
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&labels)?);
        }
        OutputFormat::Csv => {
            println!("key,broad_type,priority,designation,title");
            for (key, def) in &defs {
                println!(
                    "\"{}\",\"{}\",{},\"{:?}\",\"{}\"",
                    key,
                    def.broad_type.as_deref().unwrap_or(""),
                    def.release_priority,
                    def.designation,
                    def.title.as_deref().unwrap_or("")
                );
            }
        }
        OutputFormat::JsonSchema => {
            // ac-03: per-type JSON Schema export, always-array shape
            // (even single matches) — matches `taxonomy`'s other output
            // formats. Pretty-print is unconditional, as with `Json`.
            let schemas: Vec<serde_json::Value> = defs
                .iter()
                .map(|(key, def)| json_schema::emit_type_schema(key, def))
                .collect();
            println!("{}", serde_json::to_string_pretty(&schemas)?);
        }
    }

    Ok(())
}

/// Convert a Serialize value to serde_json::Value.
/// Used for serde_yaml::Value fields (samples, references, decompose) that need JSON output.
pub(crate) fn to_json_value<T: serde::Serialize>(value: &T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}

/// Serialize a Definition with all fields for --full export.
pub(crate) fn definition_to_full_json(
    key: &str,
    d: &finetype_core::Definition,
) -> serde_json::Value {
    let label = Label::parse(key);

    let samples: serde_json::Value = to_json_value(&d.samples);

    let validation = d.validation.as_ref().map(|v| v.to_json_schema());

    let validation_by_locale: Option<serde_json::Map<String, serde_json::Value>> =
        d.validation_by_locale.as_ref().map(|locales| {
            locales
                .iter()
                .map(|(locale, v)| (locale.clone(), v.to_json_schema()))
                .collect()
        });

    let decompose = d.decompose.as_ref().map(to_json_value);

    let references = d.references.as_ref().map(to_json_value);

    // Serialize designation as snake_case string via serde
    let designation = serde_json::to_value(&d.designation).unwrap_or(json!("universal"));

    let mut obj = serde_json::Map::new();
    obj.insert("key".into(), json!(key));
    if let Some(ref l) = label {
        obj.insert("domain".into(), json!(l.domain));
        obj.insert("category".into(), json!(l.category));
        obj.insert("type".into(), json!(l.type_name));
    }
    obj.insert("title".into(), json!(d.title));
    obj.insert("description".into(), json!(d.description));
    obj.insert("designation".into(), designation);
    obj.insert("broad_type".into(), json!(d.broad_type));
    obj.insert("format_string".into(), json!(d.format_string));
    obj.insert("format_string_alt".into(), json!(d.format_string_alt));
    obj.insert("transform".into(), json!(d.transform));
    obj.insert("transform_ext".into(), json!(d.transform_ext));
    obj.insert("locales".into(), json!(d.locales));
    obj.insert("tier".into(), json!(d.tier));
    obj.insert("release_priority".into(), json!(d.release_priority));
    obj.insert("aliases".into(), json!(d.aliases));
    obj.insert("pii".into(), json!(d.pii));
    obj.insert("notes".into(), json!(d.notes));
    obj.insert("samples".into(), json!(samples));
    obj.insert(
        "validation".into(),
        validation.unwrap_or(serde_json::Value::Null),
    );
    if let Some(locales) = validation_by_locale {
        obj.insert(
            "validation_by_locale".into(),
            serde_json::Value::Object(locales),
        );
    }
    if let Some(dec) = decompose {
        obj.insert("decompose".into(), dec);
    }
    if let Some(refs) = references {
        obj.insert("references".into(), refs);
    }
    if let Some(ref canonical) = d.canonical {
        obj.insert("canonical".into(), json!(canonical));
    }
    if let Some(ref glyph) = d.glyph {
        obj.insert("glyph".into(), json!(glyph));
    }

    serde_json::Value::Object(obj)
}

/// Simple Levenshtein distance for type name suggestions.
pub(crate) fn levenshtein_distance(a: &str, b: &str) -> usize {
    let b_len = b.len();
    let mut prev = (0..=b_len).collect::<Vec<_>>();
    let mut curr = vec![0; b_len + 1];
    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b_len]
}

/// Map DuckDB SQL type to Arrow DataType JSON representation.
///
/// Uses the Arrow IPC JSON schema format compatible with arrow-rs and pyarrow.
pub(crate) fn duckdb_to_arrow_type(duckdb_type: &str) -> serde_json::Value {
    match duckdb_type {
        "VARCHAR" => json!({"name": "utf8"}),
        "DOUBLE" => json!({"name": "floatingpoint", "precision": "DOUBLE"}),
        "BIGINT" => json!({"name": "int", "bitWidth": 64, "isSigned": true}),
        "DECIMAL" => json!({"name": "decimal", "precision": 38, "scale": 10, "bitWidth": 128}),
        "DATE" => json!({"name": "date", "unit": "DAY"}),
        "TIMESTAMP" => json!({"name": "timestamp", "unit": "MICROSECOND", "timezone": null}),
        "TIME" => json!({"name": "time", "unit": "MICROSECOND", "bitWidth": 64}),
        "BOOLEAN" => json!({"name": "bool"}),
        "JSON" => json!({"name": "utf8"}),
        "STRUCT" => json!({"name": "struct"}),
        "LIST" => json!({"name": "list"}),
        _ => json!({"name": "utf8"}),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// CHECK — Validate generator ↔ taxonomy alignment
// ═══════════════════════════════════════════════════════════════════════════════

pub(crate) fn cmd_check(
    taxonomy_path: PathBuf,
    samples: usize,
    seed: u64,
    priority: Option<u8>,
    verbose: bool,
    output: OutputFormat,
) -> Result<()> {
    eprintln!("Loading taxonomy from {:?}", taxonomy_path);
    let taxonomy = load_taxonomy(&taxonomy_path)?;
    eprintln!("Loaded {} definitions", taxonomy.len());

    let checker = Checker::new(samples).with_seed(seed);
    eprintln!(
        "Checking {} samples per definition (seed={})...",
        samples, seed
    );

    let report = checker.run(&taxonomy);

    match output {
        OutputFormat::Plain
        | OutputFormat::Markdown
        | OutputFormat::Arrow
        | OutputFormat::JsonSchema
        | OutputFormat::Datapackage => {
            print!("{}", format_report(&report, verbose));
        }
        OutputFormat::Json => {
            let results: Vec<serde_json::Value> = report
                .results
                .iter()
                .filter(|r| priority.map(|p| r.release_priority >= p).unwrap_or(true))
                .map(|r| {
                    let mut obj = serde_json::Map::new();
                    obj.insert("key".to_string(), json!(r.key));
                    obj.insert("domain".to_string(), json!(r.domain));
                    obj.insert("generator_exists".to_string(), json!(r.generator_exists));
                    obj.insert("samples_generated".to_string(), json!(r.samples_generated));
                    obj.insert("samples_passed".to_string(), json!(r.samples_passed));
                    obj.insert("samples_failed".to_string(), json!(r.samples_failed));
                    obj.insert("pass_rate".to_string(), json!(r.pass_rate()));
                    obj.insert("has_pattern".to_string(), json!(r.has_pattern));
                    obj.insert("release_priority".to_string(), json!(r.release_priority));
                    obj.insert("passed".to_string(), json!(r.passed()));
                    if !r.failures.is_empty() {
                        let failures: Vec<serde_json::Value> = r
                            .failures
                            .iter()
                            .map(|f| {
                                json!({
                                    "sample": f.sample,
                                    "reason": format!("{}", f.reason),
                                })
                            })
                            .collect();
                        obj.insert("failures".to_string(), json!(failures));
                    }
                    serde_json::Value::Object(obj)
                })
                .collect();

            let summary = json!({
                "total_definitions": report.total_definitions,
                "generators_found": report.generators_found,
                "generators_missing": report.generators_missing,
                "fully_passing": report.fully_passing,
                "has_failures": report.has_failures,
                "no_pattern": report.no_pattern,
                "total_samples": report.total_samples,
                "total_passed": report.total_passed,
                "total_failed": report.total_failed,
                "pass_rate": report.pass_rate(),
                "all_passed": report.all_passed(),
                "results": results,
            });

            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        OutputFormat::Csv => {
            println!("key,domain,generator_exists,samples_generated,samples_passed,samples_failed,pass_rate,has_pattern,priority,passed");
            for r in &report.results {
                if priority.map(|p| r.release_priority >= p).unwrap_or(true) {
                    println!(
                        "\"{}\",\"{}\",{},{},{},{},{:.4},{},{},{}",
                        r.key,
                        r.domain,
                        r.generator_exists,
                        r.samples_generated,
                        r.samples_passed,
                        r.samples_failed,
                        r.pass_rate(),
                        r.has_pattern,
                        r.release_priority,
                        r.passed(),
                    );
                }
            }
        }
    }

    // Frictionless mapping gate (choice 0105, spec 2026-06-24 ac-01): every
    // definition must carry a valid {type, format} block, else `check` fails.
    let mut fx_failures: Vec<String> = Vec::new();
    for (key, def) in taxonomy.definitions() {
        match &def.frictionless {
            None => fx_failures.push(format!("{key}: missing `frictionless` block")),
            Some(fr) => {
                if let Err(e) = fr.validate() {
                    fx_failures.push(format!("{key}: {e}"));
                }
            }
        }
    }
    if !fx_failures.is_empty() {
        eprintln!(
            "\nFrictionless mapping check FAILED ({} definition(s)):",
            fx_failures.len()
        );
        for f in &fx_failures {
            eprintln!("  - {f}");
        }
        std::process::exit(1);
    }

    // Exit non-zero if checks failed
    if !report.all_passed() {
        std::process::exit(1);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATE — Schema-driven CSV quality gate
// ═══════════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════════
// VALIDATE — DuckDB-native reject pipeline (spec v1.2 ac-06, ac-08, ac-09, ac-10, ac-11)
// ═══════════════════════════════════════════════════════════════════════════════

/// Exit codes.
///
/// - 0: no rejects, no error
/// - 1: one or more rejects (default CI-gate)
/// - 2: error (bad schema, file unreadable, DuckDB error, staging collision
///   without `--append`). Not suppressed by `--lenient`.
///
/// `--lenient` forces 0 whenever the exit would otherwise be 1.
pub(crate) fn exit_with(code: i32) -> ! {
    std::process::exit(code);
}

/// Load + parse a JSON Schema file with structured error messages (ac-08).
///
/// Emits one-line `error:` messages to stderr, then exits 2. Fail-fast ordering:
/// (1) missing-file, (2) permission-denied, (3) invalid-JSON, (4) missing
/// `properties` object.
pub(crate) fn load_schema_or_exit(schema_path: &PathBuf) -> serde_json::Value {
    // (1) + (2): open and read. std::fs::read_to_string produces distinct
    //     io::ErrorKind values we can discriminate on.
    let schema_content = match std::fs::read_to_string(schema_path) {
        Ok(s) => s,
        Err(e) => {
            match e.kind() {
                std::io::ErrorKind::NotFound => {
                    eprintln!("error: schema file not found: {}", schema_path.display());
                }
                std::io::ErrorKind::PermissionDenied => {
                    eprintln!(
                        "error: permission denied reading schema file: {}",
                        schema_path.display()
                    );
                }
                _ => {
                    eprintln!(
                        "error: could not read schema file {}: {}",
                        schema_path.display(),
                        e
                    );
                }
            }
            exit_with(2);
        }
    };

    // (3): parse JSON. serde_json errors include a byte/line position.
    let schema: serde_json::Value = match serde_json::from_str(&schema_content) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error: invalid JSON in schema file {}: {} (at line {} col {})",
                schema_path.display(),
                e,
                e.line(),
                e.column()
            );
            exit_with(2);
        }
    };

    // (4): structural check — must have an object `properties`.
    if !schema.is_object()
        || schema
            .get("properties")
            .and_then(|p| p.as_object())
            .is_none()
    {
        eprintln!(
            "error: schema file {} is missing required `properties` object",
            schema_path.display()
        );
        exit_with(2);
    }

    schema
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Validate a single value against a label's live `CompiledValidator`.
///
/// Hidden subcommand backing the runtime/eval parity test (ac-04). It
/// exercises the SAME validator the profile veto (ac-06) uses —
/// `validate_value_for_label`, which carries the ac-02 scoped enum
/// case-fold and the ac-01 widened patterns/bounds. Prints `PASS`/`FAIL`
/// so a shell-out test can cross-check it against the Python eval gate.
pub(crate) fn cmd_validate_value(
    label: String,
    value: String,
    taxonomy_path: PathBuf,
) -> Result<()> {
    let mut taxonomy = load_taxonomy(&taxonomy_path)?;
    taxonomy.compile_validators();
    let result = finetype_core::validate_value_for_label(&value, &label, &taxonomy)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("{}", if result.is_valid { "PASS" } else { "FAIL" });
    Ok(())
}
