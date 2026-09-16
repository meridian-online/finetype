//! Column-level helpers shared by the `ft_profile` and `ft_detail` aggregates.
//!
//! Both aggregates live in `profile_agg.rs` and share one state: a seeded
//! reservoir of up to `PROFILE_SAMPLE_CAP` values per group, plus the first
//! header hint the group saw. `ft_profile` finalizes that state into a
//! three-field STRUCT; `ft_detail` finalizes the same state into the full
//! classification as a JSON string, formatted here. Because the sample is
//! shared, `ft_detail(col)` explains the verdict `ft_profile(col)` gives over the
//! same rows.
//!
//! Usage:
//! ```sql
//! -- Why did the column type as it did? One row, the same sample as ft_profile.
//! SELECT ft_detail(col_value) FROM values_table;
//!
//! -- Per column, with the column's name as the header hint
//! SELECT col_name, ft_detail(col_value, col_name)
//! FROM values_table GROUP BY col_name;
//! ```
//!
//! Neither aggregate supports an aggregate-level `ORDER BY`:
//! `ft_profile(col ORDER BY col)` and `ft_detail(col ORDER BY col)` read out of
//! bounds inside DuckDB (see `profile_agg.rs`).

use crate::type_mapping;

use finetype_model::ColumnResult;

/// Sample cap for the `ft_profile` and `ft_detail` aggregates. The classifier pools
/// `sample_size` (100) values, so keeping more is wasted work: each group's
/// state holds at most this many values however many rows it sees, which is
/// what lets the aggregate run over a whole column without materialising it.
pub const PROFILE_SAMPLE_CAP: usize = 100;

/// Format a ColumnResult as a JSON string.
pub fn format_column_result_json(result: &ColumnResult) -> String {
    let duckdb_type = type_mapping::to_duckdb_type(&result.label);

    // Build top-N votes as a JSON object
    let votes: Vec<String> = result
        .vote_distribution
        .iter()
        .take(5) // Top 5 candidates
        .map(|(label, frac)| format!(r#""{}": {:.3}"#, label, frac))
        .collect();
    let votes_json = format!("{{{}}}", votes.join(", "));

    let disambiguation = if let Some(ref rule) = result.disambiguation_rule {
        format!(r#", "disambiguation": "{}""#, rule)
    } else {
        String::new()
    };

    format!(
        r#"{{"type": "{}", "confidence": {:.3}, "duckdb_type": "{}", "samples": {}{}, "votes": {}}}"#,
        result.label,
        result.confidence,
        duckdb_type,
        result.samples_used,
        disambiguation,
        votes_json,
    )
}
