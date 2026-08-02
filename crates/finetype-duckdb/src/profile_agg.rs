//! `ft_profile` as a true DuckDB aggregate, registered through the raw C API.
//!
//! `SELECT ft_profile(email) FROM people` is the call a reader reaches for
//! first. It is an aggregate: DuckDB pools the column's values into one state
//! per group and hands the pooled sample to the classifier once, which is
//! exactly the shape the column-oriented model wants.
//!
//! duckdb-rs exposes `vscalar` and `vtab` only, so the registration goes
//! through `libduckdb_sys` directly — the same raw-FFI route the crate already
//! uses to read vectors. The entrypoint in `crates/finetype-duckdb/src/lib.rs`
//! hand-expands the loadable-extension macro to reach a raw connection, because
//! the wrapper's `Connection` does not expose one.
//!
//! ## Registration shape
//!
//! Both arities go in through ONE function set. An aggregate registered twice
//! at the same name fails on the second call: the C API registers with
//! `ALTER_ON_CONFLICT` and the aggregate's create-info does not implement the
//! alter path, so the conflict throws and the C API returns `DuckDBError` with
//! no message. `_set` + `_add_..._to_set` + `_register_..._set` is the route
//! that overloads a name.
//!
//! An aggregate also cannot take a name a SCALAR already holds — same
//! swallowed conflict — which is why `ft_profile` no longer registers a scalar
//! over `LIST`. A table macro at the same name is fine: DuckDB routes by call
//! position, so `FROM ft_profile('t')` binds the macro and `ft_profile(col)` in
//! a projection binds this aggregate.
//!
//! ## Two things the callbacks must do that are not obvious
//!
//! **`combine` has to carry the header across.** DuckDB's final combine merges
//! each group's accumulated state into a freshly initialised state that never
//! saw a row, so a header captured during `update` is lost unless `combine`
//! propagates it source → target. It copies rather than moves: the same source
//! may be combined again, and is destroyed either way.
//!
//! **The merge has to respect how much each side saw.** Two capped samples are
//! not interchangeable — one may summarise a thousand rows and the other three.
//! Merging by concatenate-and-truncate biases the result toward whichever
//! partial state was merged first. Each side instead contributes a share of the
//! cap proportional to the row count behind it, drawn without replacement, so
//! every value in the group keeps roughly the same chance of reaching the
//! classifier however DuckDB happened to partition the scan.
//!
//! ## Unsupported: an aggregate-level `ORDER BY`
//!
//! `ft_profile(col ORDER BY col)` reads out of bounds inside DuckDB and can
//! crash the process or return a silently wrong answer. The sorted-aggregate
//! path makes the state vector constant and the C API flattens only the inputs,
//! so the callback receives a one-element state buffer with a multi-row count.
//! It is upstream (duckdb/duckdb#21537, deferred), it is not specific to this
//! aggregate, and the callback cannot detect it — the pointer and the count
//! both arrive from DuckDB and look valid. The `ft_profile(tbl)` table macro
//! never emits one.

use crate::column_fn::PROFILE_SAMPLE_CAP;
use crate::{get_column_classifier, type_mapping};

use libduckdb_sys::*;
use std::ffi::CString;
use std::panic::AssertUnwindSafe;

/// Stamped into every state by `init` and checked by every other callback, so a
/// state that was never initialised is visibly different from one that was.
const PROFILE_MAGIC: u64 = 0x4654_5052_4F46_0001;

/// Fixed seed. Sampling decisions are then reproducible for a given sequence of
/// callback calls, which is as deterministic as an aggregate can be — DuckDB
/// decides how a scan is partitioned, and that is not ours to fix.
const RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The answer for a group with nothing classifiable in it.
const UNKNOWN_LABEL: &str = "unknown";
const UNKNOWN_DUCKDB_TYPE: &str = "VARCHAR";

/// Per-group aggregate state.
///
/// DuckDB hands out raw, uninitialised memory of `state_size` bytes, so
/// everything here is a plain machine word and the growable parts live behind
/// owning raw pointers that `init` nulls and the destructor frees.
#[repr(C)]
struct ProfileState {
    /// `PROFILE_MAGIC` once `init` has run.
    inited: u64,
    /// Owning pointer to the reservoir, or null before the first value.
    sample: *mut Vec<String>,
    /// Owning pointer to the header hint, or null if the group has not seen a
    /// non-empty one.
    header: *mut String,
    /// Every non-empty value the group has seen, not just the kept ones. The
    /// merge needs it to weight the two sides.
    seen: u64,
    /// Sampling PRNG.
    rng: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SAMPLING
// ═══════════════════════════════════════════════════════════════════════════════

/// SplitMix64. Small, seedable and dependency-free; the sampling here needs
/// spread, not cryptographic quality.
fn next_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Keep the first `k` elements of a uniformly drawn subset, in place.
///
/// Partial Fisher-Yates: each of the first `k` slots takes a uniform pick from
/// the not-yet-chosen tail, so every element has the same chance of surviving.
fn retain_random(values: &mut Vec<String>, k: usize, rng: &mut u64) {
    let len = values.len();
    if k >= len {
        return;
    }
    for i in 0..k {
        let j = i + (next_u64(rng) % (len - i) as u64) as usize;
        values.swap(i, j);
    }
    values.truncate(k);
}

/// Offer one value to a group's reservoir (Algorithm R).
///
/// # Safety
/// `state` must point at an initialised `ProfileState`.
unsafe fn reservoir_push(state: &mut ProfileState, value: String) {
    if state.sample.is_null() {
        state.sample = Box::into_raw(Box::new(Vec::with_capacity(PROFILE_SAMPLE_CAP)));
    }
    let sample = &mut *state.sample;
    state.seen += 1;
    if sample.len() < PROFILE_SAMPLE_CAP {
        sample.push(value);
        return;
    }
    let slot = (next_u64(&mut state.rng) % state.seen) as usize;
    if slot < PROFILE_SAMPLE_CAP {
        sample[slot] = value;
    }
}

/// Fold `source`'s reservoir into `target`'s, weighting each side by how many
/// values it stands for.
///
/// # Safety
/// Both must point at initialised `ProfileState`s.
unsafe fn merge_reservoirs(target: &mut ProfileState, source: &ProfileState) {
    let source_sample: &[String] = if source.sample.is_null() {
        &[]
    } else {
        let owned: &Vec<String> = &*source.sample;
        &owned[..]
    };
    if source_sample.is_empty() {
        target.seen += source.seen;
        return;
    }
    if target.sample.is_null() {
        target.sample = Box::into_raw(Box::new(Vec::with_capacity(PROFILE_SAMPLE_CAP)));
    }
    let target_sample = &mut *target.sample;

    if target_sample.len() + source_sample.len() <= PROFILE_SAMPLE_CAP {
        target_sample.extend_from_slice(source_sample);
        target.seen += source.seen;
        return;
    }

    let total = target.seen.saturating_add(source.seen).max(1);
    let mut from_source =
        ((PROFILE_SAMPLE_CAP as u128 * source.seen as u128) / total as u128) as usize;
    from_source = from_source.min(source_sample.len());
    let mut from_target = PROFILE_SAMPLE_CAP - from_source;
    if from_target > target_sample.len() {
        let spare = from_target - target_sample.len();
        from_target = target_sample.len();
        from_source = (from_source + spare).min(source_sample.len());
    }

    let mut rng = target.rng;
    retain_random(target_sample, from_target, &mut rng);
    let mut taken = source_sample.to_vec();
    retain_random(&mut taken, from_source, &mut rng);
    target_sample.extend(taken);
    target.rng = rng;
    target.seen += source.seen;
}

// ═══════════════════════════════════════════════════════════════════════════════
// VECTOR READING
// ═══════════════════════════════════════════════════════════════════════════════

/// Read row `row` of a flat VARCHAR vector. `None` is SQL NULL.
///
/// The C API flattens an aggregate's input vectors before calling `update`, so
/// a flat read is the right one here.
///
/// # Safety
/// `vector` must be a live VARCHAR vector with at least `row + 1` rows.
unsafe fn read_varchar_row(vector: duckdb_vector, row: usize) -> Option<String> {
    let validity = duckdb_vector_get_validity(vector);
    if !validity.is_null() && !duckdb_validity_row_is_valid(validity, row as idx_t) {
        return None;
    }
    let data = duckdb_vector_get_data(vector) as *const duckdb_string_t;
    let raw = *data.add(row);
    let (ptr, len) = if duckdb_string_is_inlined(raw) {
        (
            raw.value.inlined.inlined.as_ptr() as *const u8,
            raw.value.inlined.length as usize,
        )
    } else {
        (
            raw.value.pointer.ptr as *const u8,
            raw.value.pointer.length as usize,
        )
    };
    if ptr.is_null() || len == 0 {
        return Some(String::new());
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len))
        .ok()
        .map(|s| s.to_string())
}

// ═══════════════════════════════════════════════════════════════════════════════
// CALLBACKS
// ═══════════════════════════════════════════════════════════════════════════════

unsafe extern "C" fn profile_state_size(_info: duckdb_function_info) -> idx_t {
    std::mem::size_of::<ProfileState>() as idx_t
}

unsafe extern "C" fn profile_init(_info: duckdb_function_info, state: duckdb_aggregate_state) {
    let state = state as *mut ProfileState;
    if state.is_null() {
        return;
    }
    (*state).inited = PROFILE_MAGIC;
    (*state).sample = std::ptr::null_mut();
    (*state).header = std::ptr::null_mut();
    (*state).seen = 0;
    (*state).rng = RNG_SEED;
}

/// One callback serves both arities. Whether a header argument was passed at
/// all is read off the chunk's column count rather than assumed — reading
/// vector 1 in a one-argument call would be a wild read.
unsafe extern "C" fn profile_update(
    _info: duckdb_function_info,
    input: duckdb_data_chunk,
    states: *mut duckdb_aggregate_state,
) {
    let columns = duckdb_data_chunk_get_column_count(input) as usize;
    if columns == 0 {
        return;
    }
    let rows = duckdb_data_chunk_get_size(input) as usize;
    let values = duckdb_data_chunk_get_vector(input, 0);
    let headers = if columns >= 2 {
        Some(duckdb_data_chunk_get_vector(input, 1))
    } else {
        None
    };

    for row in 0..rows {
        let state = *states.add(row) as *mut ProfileState;
        if state.is_null() || (*state).inited != PROFILE_MAGIC {
            continue;
        }
        if let Some(text) = read_varchar_row(values, row) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                reservoir_push(&mut *state, trimmed.to_string());
            }
        }
        // First non-empty header a group sees wins. A NULL or empty hint is not
        // a hint, and a group whose first row carries one is still entitled to
        // the next row's.
        if let Some(headers) = headers {
            if (*state).header.is_null() {
                if let Some(hint) = read_varchar_row(headers, row) {
                    if !hint.is_empty() {
                        (*state).header = Box::into_raw(Box::new(hint));
                    }
                }
            }
        }
    }
}

unsafe extern "C" fn profile_combine(
    _info: duckdb_function_info,
    source: *mut duckdb_aggregate_state,
    target: *mut duckdb_aggregate_state,
    count: idx_t,
) {
    for i in 0..count as usize {
        let source_state = *source.add(i) as *const ProfileState;
        let target_state = *target.add(i) as *mut ProfileState;
        if source_state.is_null() || target_state.is_null() {
            continue;
        }
        if (*source_state).inited != PROFILE_MAGIC || (*target_state).inited != PROFILE_MAGIC {
            continue;
        }
        // Copy, never move: the same source may be combined again, and will be
        // destroyed whether it was or not. A disagreement degrades to the
        // target's hint rather than throwing — GROUP BY ROLLUP legitimately
        // merges groups with different headers into a grand-total row.
        if (*target_state).header.is_null() && !(*source_state).header.is_null() {
            let carried = (*(*source_state).header).clone();
            (*target_state).header = Box::into_raw(Box::new(carried));
        }
        merge_reservoirs(&mut *target_state, &*source_state);
    }
}

/// Classify one group. Returns the STRUCT's three fields.
///
/// # Safety
/// `state` must point at an initialised `ProfileState`, or be null.
unsafe fn classify_state(state: *const ProfileState) -> (String, f64, String) {
    let unknown = || {
        (
            UNKNOWN_LABEL.to_string(),
            0.0,
            UNKNOWN_DUCKDB_TYPE.to_string(),
        )
    };
    if state.is_null() || (*state).inited != PROFILE_MAGIC || (*state).sample.is_null() {
        return unknown();
    }
    let sample: &Vec<String> = &*(*state).sample;
    if sample.is_empty() {
        return unknown();
    }
    let header = if (*state).header.is_null() {
        None
    } else {
        Some((*(*state).header).clone())
    };

    // The classifier loads a model on first use and can fail. This is a C
    // callback, so an unwind past it would abort the process — a degraded
    // answer is strictly better than a dead session.
    let classified = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let classifier = get_column_classifier();
        match header {
            Some(hint) => classifier.classify_column_with_header(sample, &hint),
            None => classifier.classify_column(sample),
        }
    }));

    match classified {
        Ok(Ok(result)) => {
            let duckdb_type = type_mapping::to_duckdb_type(&result.label).to_string();
            (result.label, result.confidence as f64, duckdb_type)
        }
        _ => unknown(),
    }
}

unsafe extern "C" fn profile_finalize(
    _info: duckdb_function_info,
    source: *mut duckdb_aggregate_state,
    result: duckdb_vector,
    count: idx_t,
    offset: idx_t,
) {
    let type_child = duckdb_struct_vector_get_child(result, 0);
    let confidence_child = duckdb_struct_vector_get_child(result, 1);
    let duckdb_type_child = duckdb_struct_vector_get_child(result, 2);
    let confidence_data = duckdb_vector_get_data(confidence_child) as *mut f64;

    for i in 0..count as usize {
        let row = offset as usize + i;
        let (label, confidence, duckdb_type) =
            classify_state(*source.add(i) as *const ProfileState);
        // An interior NUL cannot come out of the taxonomy, but a raw callback
        // is the wrong place to find out: fall back rather than unwrap.
        let label = CString::new(label).unwrap_or_else(|_| c_unknown_label());
        let duckdb_type = CString::new(duckdb_type).unwrap_or_else(|_| c_unknown_type());
        duckdb_vector_assign_string_element(type_child, row as idx_t, label.as_ptr());
        *confidence_data.add(row) = confidence;
        duckdb_vector_assign_string_element(duckdb_type_child, row as idx_t, duckdb_type.as_ptr());
    }
}

fn c_unknown_label() -> CString {
    CString::new(UNKNOWN_LABEL).expect("a literal without an interior NUL")
}

fn c_unknown_type() -> CString {
    CString::new(UNKNOWN_DUCKDB_TYPE).expect("a literal without an interior NUL")
}

unsafe extern "C" fn profile_destroy(states: *mut duckdb_aggregate_state, count: idx_t) {
    for i in 0..count as usize {
        let state = *states.add(i) as *mut ProfileState;
        if state.is_null() || (*state).inited != PROFILE_MAGIC {
            continue;
        }
        if !(*state).sample.is_null() {
            drop(Box::from_raw((*state).sample));
            (*state).sample = std::ptr::null_mut();
        }
        if !(*state).header.is_null() {
            drop(Box::from_raw((*state).header));
            (*state).header = std::ptr::null_mut();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// REGISTRATION
// ═══════════════════════════════════════════════════════════════════════════════

/// `STRUCT("type" VARCHAR, confidence DOUBLE, duckdb_type VARCHAR)` — the shape
/// the finalize callback writes, and the one the table macro projects out of.
unsafe fn profile_return_type() -> duckdb_logical_type {
    let mut label = duckdb_create_logical_type(DUCKDB_TYPE_DUCKDB_TYPE_VARCHAR);
    let mut confidence = duckdb_create_logical_type(DUCKDB_TYPE_DUCKDB_TYPE_DOUBLE);
    let mut duckdb_type = duckdb_create_logical_type(DUCKDB_TYPE_DUCKDB_TYPE_VARCHAR);
    let mut member_types = [label, confidence, duckdb_type];

    let name_label = CString::new("type").expect("a literal without an interior NUL");
    let name_confidence = CString::new("confidence").expect("a literal without an interior NUL");
    let name_duckdb_type = CString::new("duckdb_type").expect("a literal without an interior NUL");
    let mut member_names = [
        name_label.as_ptr(),
        name_confidence.as_ptr(),
        name_duckdb_type.as_ptr(),
    ];

    let struct_type = duckdb_create_struct_type(
        member_types.as_mut_ptr(),
        member_names.as_mut_ptr(),
        member_names.len() as idx_t,
    );

    duckdb_destroy_logical_type(&mut label);
    duckdb_destroy_logical_type(&mut confidence);
    duckdb_destroy_logical_type(&mut duckdb_type);
    struct_type
}

/// Build one arity of the aggregate. Every parameter is VARCHAR: the value
/// column, and for the two-argument form the header hint.
unsafe fn build_profile_aggregate(name: &CString, arity: usize) -> duckdb_aggregate_function {
    let function = duckdb_create_aggregate_function();
    duckdb_aggregate_function_set_name(function, name.as_ptr());
    for _ in 0..arity {
        let mut parameter = duckdb_create_logical_type(DUCKDB_TYPE_DUCKDB_TYPE_VARCHAR);
        duckdb_aggregate_function_add_parameter(function, parameter);
        duckdb_destroy_logical_type(&mut parameter);
    }
    let mut return_type = profile_return_type();
    duckdb_aggregate_function_set_return_type(function, return_type);
    duckdb_destroy_logical_type(&mut return_type);

    duckdb_aggregate_function_set_functions(
        function,
        Some(profile_state_size),
        Some(profile_init),
        Some(profile_update),
        Some(profile_combine),
        Some(profile_finalize),
    );
    duckdb_aggregate_function_set_destructor(function, Some(profile_destroy));

    // Deliver NULLs to `update` instead of filtering the row out. Without this
    // DuckDB drops any row where ANY argument is NULL, so `ft_profile(v, NULL)`
    // would see no rows at all and every value in the column would be thrown
    // away because the hint was absent.
    duckdb_aggregate_function_set_special_handling(function);

    function
}

/// Register `ft_profile(VARCHAR)` and `ft_profile(VARCHAR, VARCHAR)`.
///
/// # Safety
/// `con` must be a live connection, and the extension API table must already be
/// initialised.
pub unsafe fn register(con: duckdb_connection) -> Result<(), String> {
    let name = CString::new("ft_profile").map_err(|e| e.to_string())?;
    let set = duckdb_create_aggregate_function_set(name.as_ptr());
    if set.is_null() {
        return Err("could not create the ft_profile aggregate function set".to_string());
    }

    let mut failure: Option<String> = None;
    for arity in [1usize, 2usize] {
        let mut function = build_profile_aggregate(&name, arity);
        let added = duckdb_add_aggregate_function_to_set(set, function);
        duckdb_destroy_aggregate_function(&mut function);
        if added != 0 {
            failure = Some(format!(
                "could not add the {arity}-argument ft_profile to its function set"
            ));
            break;
        }
    }

    if failure.is_none() {
        let registered = duckdb_register_aggregate_function_set(con, set);
        if registered != 0 {
            // The C API swallows the catalog's message, so there is nothing
            // more specific to report than the state.
            failure = Some("could not register the ft_profile aggregate".to_string());
        }
    }

    let mut owned = set;
    duckdb_destroy_aggregate_function_set(&mut owned);

    match failure {
        Some(message) => Err(message),
        None => Ok(()),
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn state(seen: u64, values: &[&str]) -> ProfileState {
        ProfileState {
            inited: PROFILE_MAGIC,
            sample: Box::into_raw(Box::new(
                values.iter().map(|v| (*v).to_string()).collect::<Vec<_>>(),
            )),
            header: std::ptr::null_mut(),
            seen,
            rng: RNG_SEED,
        }
    }

    unsafe fn free(state: &mut ProfileState) {
        if !state.sample.is_null() {
            drop(Box::from_raw(state.sample));
            state.sample = std::ptr::null_mut();
        }
        if !state.header.is_null() {
            drop(Box::from_raw(state.header));
            state.header = std::ptr::null_mut();
        }
    }

    #[test]
    fn reservoir_keeps_every_value_until_the_cap() {
        let mut s = state(0, &[]);
        unsafe {
            free(&mut s);
            s.sample = std::ptr::null_mut();
            for i in 0..PROFILE_SAMPLE_CAP {
                reservoir_push(&mut s, format!("v{i}"));
            }
            assert_eq!((*s.sample).len(), PROFILE_SAMPLE_CAP);
            assert_eq!(s.seen, PROFILE_SAMPLE_CAP as u64);
            free(&mut s);
        }
    }

    #[test]
    fn reservoir_never_grows_past_the_cap() {
        let mut s = state(0, &[]);
        unsafe {
            free(&mut s);
            s.sample = std::ptr::null_mut();
            for i in 0..(PROFILE_SAMPLE_CAP * 40) {
                reservoir_push(&mut s, format!("v{i}"));
            }
            assert_eq!((*s.sample).len(), PROFILE_SAMPLE_CAP);
            assert_eq!(s.seen, (PROFILE_SAMPLE_CAP * 40) as u64);
            free(&mut s);
        }
    }

    #[test]
    fn reservoir_reaches_past_the_first_cap_values() {
        // Algorithm R must evict. If it never did, the sample would be exactly
        // the first PROFILE_SAMPLE_CAP values and this would find none of the
        // later ones.
        let mut s = state(0, &[]);
        unsafe {
            free(&mut s);
            s.sample = std::ptr::null_mut();
            for i in 0..(PROFILE_SAMPLE_CAP * 10) {
                reservoir_push(&mut s, format!("v{i}"));
            }
            let late = (*s.sample)
                .iter()
                .filter(|v| {
                    v.trim_start_matches('v').parse::<usize>().unwrap() >= PROFILE_SAMPLE_CAP
                })
                .count();
            assert!(late > 0, "reservoir kept only the first values");
            free(&mut s);
        }
    }

    #[test]
    fn merge_of_two_small_samples_keeps_both_whole() {
        let mut target = state(3, &["a", "b", "c"]);
        let source = state(2, &["d", "e"]);
        unsafe {
            merge_reservoirs(&mut target, &source);
            let merged = &*target.sample;
            assert_eq!(merged.len(), 5);
            assert_eq!(target.seen, 5);
            for v in ["a", "b", "c", "d", "e"] {
                assert!(merged.iter().any(|m| m == v), "{v} was dropped");
            }
            let mut source = source;
            free(&mut target);
            free(&mut source);
        }
    }

    #[test]
    fn merge_caps_the_result() {
        let big: Vec<String> = (0..PROFILE_SAMPLE_CAP).map(|i| format!("t{i}")).collect();
        let other: Vec<String> = (0..PROFILE_SAMPLE_CAP).map(|i| format!("s{i}")).collect();
        let mut target = state(1_000, &big.iter().map(|v| v.as_str()).collect::<Vec<_>>());
        let source = state(1_000, &other.iter().map(|v| v.as_str()).collect::<Vec<_>>());
        unsafe {
            merge_reservoirs(&mut target, &source);
            assert_eq!((*target.sample).len(), PROFILE_SAMPLE_CAP);
            assert_eq!(target.seen, 2_000);
            let mut source = source;
            free(&mut target);
            free(&mut source);
        }
    }

    #[test]
    fn merge_weights_each_side_by_what_it_saw() {
        // A partial state standing for a handful of rows must not displace one
        // standing for thousands. Concatenate-and-truncate would keep all 100
        // of whichever side went first; proportional allocation keeps the tiny
        // side's contribution tiny.
        let big: Vec<String> = (0..PROFILE_SAMPLE_CAP).map(|i| format!("t{i}")).collect();
        let small: Vec<String> = (0..PROFILE_SAMPLE_CAP).map(|i| format!("s{i}")).collect();
        let mut target = state(9_900, &big.iter().map(|v| v.as_str()).collect::<Vec<_>>());
        let source = state(100, &small.iter().map(|v| v.as_str()).collect::<Vec<_>>());
        unsafe {
            merge_reservoirs(&mut target, &source);
            let merged = &*target.sample;
            assert_eq!(merged.len(), PROFILE_SAMPLE_CAP);
            let from_source = merged.iter().filter(|v| v.starts_with('s')).count();
            assert_eq!(
                from_source,
                (PROFILE_SAMPLE_CAP * 100) / 10_000,
                "the small side's share should track its row count"
            );
            let mut source = source;
            free(&mut target);
            free(&mut source);
        }
    }

    #[test]
    fn merging_an_empty_source_still_carries_its_row_count() {
        let mut target = state(4, &["a", "b"]);
        let mut source = state(7, &[]);
        unsafe {
            merge_reservoirs(&mut target, &source);
            assert_eq!(target.seen, 11);
            assert_eq!((*target.sample).len(), 2);
            free(&mut target);
            free(&mut source);
        }
    }

    #[test]
    fn retain_random_keeps_the_requested_count() {
        let mut values: Vec<String> = (0..50).map(|i| format!("v{i}")).collect();
        let mut rng = RNG_SEED;
        retain_random(&mut values, 7, &mut rng);
        assert_eq!(values.len(), 7);
        let mut sorted = values.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 7, "retain_random duplicated an element");
    }

    #[test]
    fn retain_random_can_reach_the_tail() {
        // A truncating implementation would only ever return the head.
        let mut seen_tail = false;
        for seed in 0..32u64 {
            let mut values: Vec<String> = (0..50).map(|i| format!("v{i}")).collect();
            let mut rng = RNG_SEED ^ seed;
            retain_random(&mut values, 5, &mut rng);
            if values
                .iter()
                .any(|v| v.trim_start_matches('v').parse::<usize>().unwrap() >= 25)
            {
                seen_tail = true;
                break;
            }
        }
        assert!(seen_tail, "retain_random never selected from the tail");
    }
}
