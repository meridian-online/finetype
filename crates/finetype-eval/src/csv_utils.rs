use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Load a CSV file as a vector of rows, where each row is a HashMap of column name → value.
pub fn load_csv(path: &Path) -> Result<Vec<HashMap<String, String>>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut rdr = csv::Reader::from_path(path)
        .with_context(|| format!("Failed to open CSV: {}", path.display()))?;
    let headers: Vec<String> = rdr.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let mut row = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            row.insert(header.clone(), record.get(i).unwrap_or("").to_string());
        }
        rows.push(row);
    }
    Ok(rows)
}
