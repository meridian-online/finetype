//! Enum-emission gate for the JSON Schema profile output.
//!
//! Two pieces of policy live here:
//!
//!   1. **Label-family gate** ([`label_is_enum_eligible`]) — only labels
//!      whose taxonomic semantics are finite-domain (categorical or
//!      boolean) are eligible to receive an emitted `enum` array.
//!      Numeric, datetime, identifier and other open-domain labels are
//!      excluded — emitting an enum for them is the classic
//!      `enum_overfit` failure mode (see spec
//!      `2026-04-28-validate-precision-corpus`, mechanism table).
//!
//!   2. **Cardinality cap** ([`collect_unique_values_if_categorical`]) —
//!      even for an enum-eligible label, the emitter only writes an
//!      `enum` array when the column's distinct-value count is
//!      `≤ enum_threshold`. The flag default is 32 (lowered from 50 in
//!      v0.6.20 under ac-09 sub-fix (a)).
//!
//! Both pieces are re-exported through `lib.rs` so the binary and the
//! integration-test suite see exactly the same gate.

/// Labels that are enum-eligible — the JSON Schema emitter attaches an
/// `enum` array (when cardinality ≤ enum_threshold) only for these labels.
///
/// `representation.discrete.categorical` — discrete finite-domain enum
/// (the original gate, retained byte-for-byte from v0.6.19).
/// `representation.boolean.{binary,initials,terms}` — boolean variants
/// receive the same gate treatment as categorical ones.
pub fn label_is_enum_eligible(label: &str) -> bool {
    // Delegates to the shared policy in finetype-core so the CLI and MCP emit the
    // same conservative `enum` keyword (spec 2026-06-17-enum-domain-emission).
    finetype_core::enum_domain::label_is_enum_keyword_eligible(label)
}

/// Collect sorted unique values for enum-eligible columns when under the
/// enum threshold.
///
/// Returns `Some(sorted_values)` when:
///   * `label` is enum-eligible per [`label_is_enum_eligible`]; AND
///   * `enum_threshold > 0`; AND
///   * the column's distinct-value count is `≤ enum_threshold`.
///
/// Returns `None` otherwise (the emitter then skips the `enum` field
/// and emits only the type/pattern constraints).
pub fn collect_unique_values_if_categorical(
    label: &str,
    values: &[String],
    enum_threshold: usize,
) -> Option<Vec<String>> {
    if !label_is_enum_eligible(label) || enum_threshold == 0 {
        return None;
    }
    let mut unique: Vec<String> = values
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .cloned()
        .collect();
    if unique.len() > enum_threshold {
        return None;
    }
    unique.sort();
    Some(unique)
}
