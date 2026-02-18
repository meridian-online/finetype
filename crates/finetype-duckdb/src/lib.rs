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

mod type_mapping;

#[cfg(feature = "embed-models")]
mod column_fn;
#[cfg(feature = "embed-models")]
mod normalize;
#[cfg(feature = "embed-models")]
mod unpack;

// ═══════════════════════════════════════════════════════════════════════════════
// EMBEDDED MODELS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "embed-models")]
mod embedded {
    include!(concat!(env!("OUT_DIR"), "/embedded_models.rs"));
}

/// Extension name and version.
const EXTENSION_VERSION: &str = env!("CARGO_PKG_VERSION");

// ═══════════════════════════════════════════════════════════════════════════════
// GLOBAL CLASSIFIER (lazy-initialized on first use)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "embed-models")]
use std::sync::OnceLock;

/// Global flat classifier, initialized on first finetype() call.
/// Uses the single-pass CharCNN model (91.97% accuracy, ~100x faster than tiered).
#[cfg(feature = "embed-models")]
static CLASSIFIER: OnceLock<finetype_model::CharClassifier> = OnceLock::new();

/// Initialize or get the global classifier from embedded flat model.
#[cfg(feature = "embed-models")]
fn get_classifier() -> &'static finetype_model::CharClassifier {
    CLASSIFIER.get_or_init(|| {
        finetype_model::CharClassifier::from_bytes(
            embedded::FLAT_WEIGHTS,
            embedded::FLAT_LABELS,
            embedded::FLAT_CONFIG,
        )
        .expect("Failed to load embedded flat model")
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// VARCHAR HELPERS
// ═══════════════════════════════════════════════════════════════════════════════

/// Read a VARCHAR value from a DuckDB data chunk at a specific column and row.
///
/// Returns None if the value is NULL.
#[cfg(feature = "embed-models")]
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
#[cfg(feature = "embed-models")]
struct FineType;

#[cfg(feature = "embed-models")]
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
#[cfg(feature = "embed-models")]
struct FineTypeDetail;

#[cfg(feature = "embed-models")]
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
#[cfg(feature = "embed-models")]
struct FineTypeCast;

#[cfg(feature = "embed-models")]
impl VScalar for FineTypeCast {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let classifier = get_classifier();
        let len = input.len();
        let mut output_vec = output.flat_vector();

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if text.is_empty() {
                    output_vec.set_null(i);
                    continue;
                }
                match classifier.classify(&text) {
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
#[cfg(feature = "embed-models")]
struct FineTypeUnpack;

#[cfg(feature = "embed-models")]
impl VScalar for FineTypeUnpack {
    type State = ();

    unsafe fn invoke(
        _state: &Self::State,
        input: &mut DataChunkHandle,
        output: &mut dyn WritableVector,
    ) -> Result<(), Box<dyn Error>> {
        let classifier = get_classifier();
        let len = input.len();
        let mut output_vec = output.flat_vector();

        for i in 0..len {
            if let Some(text) = read_varchar(input, 0, i) {
                if text.is_empty() {
                    output_vec.set_null(i);
                    continue;
                }
                match unpack::unpack_json(&text, classifier) {
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

    #[cfg(feature = "embed-models")]
    {
        con.register_scalar_function::<FineType>("finetype")
            .expect("Failed to register finetype");

        con.register_scalar_function::<FineTypeDetail>("finetype_detail")
            .expect("Failed to register finetype_detail");

        con.register_scalar_function::<FineTypeCast>("finetype_cast")
            .expect("Failed to register finetype_cast");

        con.register_scalar_function::<FineTypeUnpack>("finetype_unpack")
            .expect("Failed to register finetype_unpack");
    }

    Ok(())
}
