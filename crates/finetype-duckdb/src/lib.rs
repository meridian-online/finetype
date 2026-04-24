//! FineType DuckDB Extension
//!
//! Provides scalar functions for semantic type classification:
//! - `finetype_version()` — Returns the extension version
//! - `finetype(value)` — Classify a single value, returns the semantic type label
//! - `finetype(list(values))` — Column-level classification with disambiguation
//! - `finetype(list(values), header)` — Column-level classification with header hint
//! - `finetype_detail(value)` — Classify with detail: returns JSON with type, confidence, DuckDB type
//! - `finetype_detail(list(values))` — Column-level classification with full JSON detail
//! - `finetype_cast(value)` — Normalize a value for safe TRY_CAST (dates → ISO, booleans → true/false, etc.)
//! - `finetype_unpack(json)` — Recursively classify JSON fields, returns annotated JSON

use duckdb::core::{DataChunkHandle, Inserter, LogicalTypeHandle, LogicalTypeId};
use duckdb::vscalar::{ScalarFunctionSignature, VScalar};
use duckdb::vtab::arrow::WritableVector;
use duckdb::{duckdb_entrypoint_c_api, Result};
use std::error::Error;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::OnceLock;

use finetype_model::{ColumnClassifier, ColumnConfig, MultiBranchClassifier};

mod column_fn;
mod normalize;
mod spike;
mod type_mapping;
mod unpack;
mod validate;

// ═══════════════════════════════════════════════════════════════════════════════
// RUNTIME MODEL LOADING
// ═══════════════════════════════════════════════════════════════════════════════

/// Extension name and version.
const EXTENSION_VERSION: &str = env!("CARGO_PKG_VERSION");

/// HuggingFace repository for the default FineType multi-branch model.
const HF_REPO: &str = "meridian-online/finetype-model";

/// Files required for the multi-branch model.
const MODEL_FILES: &[&str] = &["model.safetensors", "label_map.json", "config.json"];

/// Global column classifier backed by multi-branch model.
/// Initialized on first finetype() call, loaded at runtime from
/// HuggingFace Hub or a local directory.
static COLUMN_CLASSIFIER: OnceLock<ColumnClassifier> = OnceLock::new();

/// Resolve the model directory: check `FINETYPE_MODEL_DIR` env var first,
/// then fall back to downloading from HuggingFace Hub.
fn resolve_model_dir() -> std::result::Result<PathBuf, Box<dyn Error>> {
    // 1. Check env var for local path override
    if let Ok(dir) = std::env::var("FINETYPE_MODEL_DIR") {
        let path = PathBuf::from(&dir);
        if path.join("model.safetensors").exists() {
            tracing::info!("Loading FineType model from local path: {}", dir);
            return Ok(path);
        }
        return Err(format!(
            "FINETYPE_MODEL_DIR={} does not contain model.safetensors",
            dir
        )
        .into());
    }

    // 2. Download from HuggingFace Hub (cached after first download)
    tracing::info!(
        "Downloading FineType model from HuggingFace Hub: {}",
        HF_REPO
    );
    let repo = hf_hub::Repo::new(HF_REPO.to_string(), hf_hub::RepoType::Model);
    let api = hf_hub::api::sync::Api::new()?;
    let api = api.repo(repo);

    // Download all required files — hf_hub caches them automatically
    let mut model_dir = None;
    for filename in MODEL_FILES {
        let path = api.get(filename)?;
        if model_dir.is_none() {
            // All files from the same repo land in the same cache directory
            model_dir = path.parent().map(|p| p.to_path_buf());
        }
    }

    model_dir.ok_or_else(|| "Failed to determine model cache directory".into())
}

/// Initialize or get the global column classifier backed by multi-branch model.
fn get_column_classifier() -> &'static ColumnClassifier {
    COLUMN_CLASSIFIER.get_or_init(|| {
        let model_dir = resolve_model_dir().expect("Failed to resolve FineType model directory");
        let mb = MultiBranchClassifier::load(&model_dir)
            .expect("Failed to load multi-branch model from resolved directory");
        let config = ColumnConfig {
            sample_size: 100,
            ..Default::default()
        };
        ColumnClassifier::with_multi_branch(mb, config)
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// VARCHAR HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Read a VARCHAR value from a DuckDB data chunk at a specific column and row.
///
/// Returns None if the value is NULL.
unsafe fn read_varchar(
    input: &mut DataChunkHandle,
    col_idx: usize,
    row_idx: usize,
) -> Option<String> {
    use libduckdb_sys::*;

    let raw_chunk = input.get_ptr();
    let vector = duckdb_data_chunk_get_vector(raw_chunk, col_idx as idx_t);

    // Check validity (NULL check)
    let validity = duckdb_vector_get_validity(vector);
    if !validity.is_null() {
        let entry = row_idx / 64;
        let bit = row_idx % 64;
        let mask = *validity.add(entry);
        if (mask >> bit) & 1 == 0 {
            return None;
        }
    }

    // Read string data
    let data = duckdb_vector_get_data(vector) as *const duckdb_string_t;
    let str_val = *data.add(row_idx);

    let (ptr, len) = if duckdb_string_is_inlined(str_val) {
        (
            str_val.value.inlined.inlined.as_ptr() as *const u8,
            str_val.value.inlined.length as usize,
        )
    } else {
        (
            str_val.value.pointer.ptr as *const u8,
            str_val.value.pointer.length as usize,
        )
    };

    if ptr.is_null() || len == 0 {
        return Some(String::new());
    }

    let bytes = std::slice::from_raw_parts(ptr, len);
    std::str::from_utf8(bytes).ok().map(|s| s.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// SCALAR FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// `finetype_version()` — Returns the FineType extension version string.
struct FineTypeVersion;

impl VScalar for FineTypeVersion {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let len = input.len();
        let output_vec = output.flat_vector();
        let version = CString::new(format!("finetype {}", EXTENSION_VERSION))?;
        for i in 0..len {
            output_vec.insert(i, version.clone());
        }
        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

/// `finetype(value VARCHAR) → VARCHAR` — Semantic type classification.
/// `finetype(list(values) LIST<VARCHAR>) → VARCHAR` — Explicit column classification.
/// `finetype(list(values) LIST<VARCHAR>, header VARCHAR) → VARCHAR` — Column with header hint.
///
/// Classifies data as a semantic type (e.g. "datetime.date.iso", "identity.person.email").
///
/// In scalar mode (`finetype(col)`), the function automatically uses the DuckDB
/// processing chunk (~2048 rows) as a sample for column-level disambiguation.
/// This means majority vote + disambiguation rules (date formats, coordinates,
/// boolean subtypes, categorical detection, numeric range, etc.) are applied
/// even without an explicit `list()` wrapper.
///
/// The `list()` overload gives explicit control over the sample — useful with
/// GROUP BY to classify each group independently, or when you want the full
/// column rather than a chunk-sized sample.
struct FineType;

impl VScalar for FineType {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        // Dispatch based on input type: VARCHAR vs LIST<VARCHAR>
        if column_fn::is_list_input(input) {
            return column_fn::invoke_column_label(input, output);
        }

        // Scalar path: use the chunk as a column sample for disambiguation.
        // Collect all non-null, non-empty values, run column classification,
        // and write the consensus label for every row.
        let len = input.len();
        let mut output_vec = output.flat_vector();

        let mut non_null_indices: Vec<usize> = Vec::with_capacity(len);
        let mut texts: Vec<String> = Vec::with_capacity(len);

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if !text.is_empty() {
                    non_null_indices.push(i);
                    texts.push(text);
                } else {
                    let cstr = CString::new("unknown")?;
                    output_vec.insert(i, cstr);
                }
            } else {
                output_vec.set_null(i);
            }
        }

        if !texts.is_empty() {
            let col_result = column_fn::classify_column(&texts)?;
            let label = CString::new(col_result.label.as_str())?;
            for idx in &non_null_indices {
                output_vec.insert(*idx, label.clone());
            }
        }

        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        let varchar = LogicalTypeHandle::from(LogicalTypeId::Varchar);
        let list_varchar = LogicalTypeHandle::list(&varchar);

        vec![
            // finetype(value VARCHAR) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
            // finetype(list(values) LIST<VARCHAR>) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![list_varchar],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
            // finetype(list(values) LIST<VARCHAR>, header VARCHAR) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![
                    LogicalTypeHandle::list(&LogicalTypeHandle::from(LogicalTypeId::Varchar)),
                    LogicalTypeHandle::from(LogicalTypeId::Varchar),
                ],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
        ]
    }
}

/// `finetype_detail(value VARCHAR) → VARCHAR` — Detailed semantic type classification.
/// `finetype_detail(list(values) LIST<VARCHAR>) → VARCHAR` — Explicit column detail.
/// `finetype_detail(list(values) LIST<VARCHAR>, header VARCHAR) → VARCHAR` — Column detail with header.
///
/// Returns a JSON object with classification details. In both scalar and list modes,
/// the output includes:
/// - `type`: semantic type label
/// - `confidence`: classification confidence (0.0 to 1.0)
/// - `duckdb_type`: recommended DuckDB CAST target type
/// - `samples`: number of values in the sample
/// - `disambiguation`: name of disambiguation rule applied (if any)
/// - `votes`: top vote distribution (label → fraction)
///
/// In scalar mode, the DuckDB processing chunk (~2048 rows) is used as the
/// column sample. The `list()` overload gives explicit control over the sample.
struct FineTypeDetail;

impl VScalar for FineTypeDetail {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        // Dispatch based on input type: VARCHAR vs LIST<VARCHAR>
        if column_fn::is_list_input(input) {
            return column_fn::invoke_column_detail(input, output);
        }

        // Scalar path: column classification over the chunk
        let len = input.len();
        let mut output_vec = output.flat_vector();

        let mut non_null_indices: Vec<usize> = Vec::with_capacity(len);
        let mut texts: Vec<String> = Vec::with_capacity(len);

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if !text.is_empty() {
                    non_null_indices.push(i);
                    texts.push(text);
                } else {
                    let cstr = CString::new(
                        r#"{"type":"unknown","confidence":0.0,"duckdb_type":"VARCHAR","samples":0}"#,
                    )?;
                    output_vec.insert(i, cstr);
                }
            } else {
                output_vec.set_null(i);
            }
        }

        if !texts.is_empty() {
            let col_result = column_fn::classify_column(&texts)?;
            let json = column_fn::format_column_result_json(&col_result);
            let cstr = CString::new(json)?;
            for idx in &non_null_indices {
                output_vec.insert(*idx, cstr.clone());
            }
        }

        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        let varchar = LogicalTypeHandle::from(LogicalTypeId::Varchar);
        let list_varchar = LogicalTypeHandle::list(&varchar);

        vec![
            // finetype_detail(value VARCHAR) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
            // finetype_detail(list(values) LIST<VARCHAR>) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![list_varchar],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
            // finetype_detail(list(values) LIST<VARCHAR>, header VARCHAR) → VARCHAR
            ScalarFunctionSignature::exact(
                vec![
                    LogicalTypeHandle::list(&LogicalTypeHandle::from(LogicalTypeId::Varchar)),
                    LogicalTypeHandle::from(LogicalTypeId::Varchar),
                ],
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ),
        ]
    }
}

/// `finetype_cast(value VARCHAR) → VARCHAR` — Normalize a value for safe casting.
///
/// Classifies the value, then normalizes it to a canonical form suitable for
/// DuckDB `TRY_CAST()`. Returns NULL if the value doesn't validate for its
/// detected type.
///
/// Examples:
/// - `finetype_cast('01/15/2024')` → `'2024-01-15'` (US date → ISO)
/// - `finetype_cast('Yes')` → `'true'` (boolean normalization)
/// - `finetype_cast('550E8400-...')` → `'550e8400-...'` (UUID lowercase)
/// - `finetype_cast('1,234')` → `'1234'` (strip formatting)
struct FineTypeCast;

impl VScalar for FineTypeCast {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let col_classifier = get_column_classifier();
        let len = input.len();
        let mut output_vec = output.flat_vector();

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if text.is_empty() {
                    output_vec.set_null(i);
                    continue;
                }
                // Classify single value via column classifier (1-element column)
                match col_classifier.classify_column(&[text.clone()]) {
                    Ok(result) => {
                        if let Some(normalized) = normalize::normalize(&text, &result.label) {
                            let cstr = CString::new(normalized)?;
                            output_vec.insert(i, cstr);
                        } else {
                            // Validation failed → NULL
                            output_vec.set_null(i);
                        }
                    }
                    Err(_) => {
                        // Classification error → pass through
                        let cstr = CString::new(text)?;
                        output_vec.insert(i, cstr);
                    }
                }
            }
            // NULL input → DuckDB handles NULL propagation
        }

        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

/// `finetype_unpack(json_value VARCHAR) → VARCHAR` — Recursively infer types in JSON.
///
/// Parses a JSON string and classifies each scalar value. Returns annotated JSON
/// where each value is replaced with an object containing:
/// - `value`: the original value
/// - `type`: detected finetype label
/// - `confidence`: classification confidence (0.0 to 1.0)
/// - `duckdb_type`: recommended DuckDB type
///
/// Returns NULL for non-JSON input.
struct FineTypeUnpack;

impl VScalar for FineTypeUnpack {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let col_classifier = get_column_classifier();
        let len = input.len();
        let mut output_vec = output.flat_vector();

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if text.is_empty() {
                    output_vec.set_null(i);
                    continue;
                }
                match unpack::unpack_json_column(&text, col_classifier) {
                    Some(annotated) => {
                        let cstr = CString::new(annotated)?;
                        output_vec.insert(i, cstr);
                    }
                    None => {
                        // Not valid JSON → NULL
                        output_vec.set_null(i);
                    }
                }
            }
        }

        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![LogicalTypeHandle::from(LogicalTypeId::Varchar)],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

/// `finetype_validate(value VARCHAR, schema_json VARCHAR) → VARCHAR`
///
/// Validates a value against a JSON Schema fragment. Returns 'valid' if the value
/// passes, or the first validation error message if it fails.
///
/// The schema is parsed and cached for performance — the same schema string
/// is compiled only once across all rows.
///
/// Examples:
/// - `finetype_validate('test@example.com', '{"type":"string","pattern":"^[^@]+@[^@]+$"}')` → `'valid'`
/// - `finetype_validate('not-an-email', '{"type":"string","pattern":"^[^@]+@[^@]+$"}')` → error message
/// - `finetype_validate('abc', '{"type":"string","minLength":5}')` → error message
struct FineTypeValidate;

impl VScalar for FineTypeValidate {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let len = input.len();
        let mut output_vec = output.flat_vector();

        for i in 0..len {
            let value = read_varchar(input, 0, i);
            let schema = read_varchar(input, 1, i);

            match (value, schema) {
                (Some(val), Some(sch)) => {
                    let result = validate::validate_value(&val, &sch);
                    let cstr = CString::new(result)?;
                    output_vec.insert(i, cstr);
                }
                _ => {
                    // NULL input → NULL output
                    output_vec.set_null(i);
                }
            }
        }

        Ok(())
    }

    fn signatures() -> Vec<ScalarFunctionSignature> {
        vec![ScalarFunctionSignature::exact(
            vec![
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
                LogicalTypeHandle::from(LogicalTypeId::Varchar),
            ],
            LogicalTypeHandle::from(LogicalTypeId::Varchar),
        )]
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXTENSION ENTRYPOINT
// ═══════════════════════════════════════════════════════════════════════════════

/// # Safety
///
/// Called by DuckDB when loading the extension. The Connection is valid for the
/// lifetime of the extension.
#[duckdb_entrypoint_c_api()]
pub unsafe fn extension_entrypoint(con: duckdb::Connection) -> Result<(), Box<dyn Error>> {
    con.register_scalar_function::<FineTypeVersion>("finetype_version")
        .expect("Failed to register finetype_version");

    con.register_scalar_function::<FineTypeValidate>("finetype_validate")
        .expect("Failed to register finetype_validate");

    con.register_scalar_function::<FineType>("finetype")
        .expect("Failed to register finetype");

    con.register_scalar_function::<FineTypeDetail>("finetype_detail")
        .expect("Failed to register finetype_detail");

    con.register_scalar_function::<FineTypeCast>("finetype_cast")
        .expect("Failed to register finetype_cast");

    con.register_scalar_function::<FineTypeUnpack>("finetype_unpack")
        .expect("Failed to register finetype_unpack");

    // Spike (ac-04) — NOT a production function. Registers a trivial
    // table function to preserve the compile-time evidence that (a) vtab
    // is active under loadable-extension and (b) scalar + table function
    // coexistence compiles. No production use — the spike ratified
    // rollback_plan Scenario A (see MADR 0064); the CLI calls
    // finetype_core::table_validator::validate_table directly and writes
    // rejects to the output .db via duckdb-rs (spec ac-06 / ac-09).
    con.register_table_function::<spike::FineTypeSpike>("finetype_spike")
        .expect("Failed to register finetype_spike (spike artefact — not production)");

    Ok(())
}
