//! Model2Vec type embedding preparation: FPS algorithm for taxonomy labels.
//!
//! For each type in the taxonomy, computes K representative embeddings
//! using Farthest Point Sampling (FPS) over synonym expansions.
//!
//! Output: `type_embeddings.safetensors` + `label_index.json`

use anyhow::{Context, Result};
use std::path::Path;

/// Farthest Point Sampling: select K representative points maximising min-distance.
///
/// Given N embeddings of dimension D, select K ≤ N that are maximally spread out.
///
/// - `embeddings`: [N, D] flattened row-major
/// - `n`: number of points
/// - `dim`: embedding dimension
/// - `k`: number of representatives to select
///
/// Returns indices of selected points.
pub fn farthest_point_sampling(embeddings: &[f32], n: usize, dim: usize, k: usize) -> Vec<usize> {
    if k >= n {
        return (0..n).collect();
    }

    let mut selected = Vec::with_capacity(k);
    let mut min_distances = vec![f32::INFINITY; n];

    // Start with the first point
    selected.push(0);

    for _ in 1..k {
        let last = *selected.last().unwrap();
        let last_start = last * dim;

        // Update min distances from the last selected point
        for i in 0..n {
            let i_start = i * dim;
            let dist: f32 = (0..dim)
                .map(|d| {
                    let diff = embeddings[last_start + d] - embeddings[i_start + d];
                    diff * diff
                })
                .sum();
            min_distances[i] = min_distances[i].min(dist);
        }

        // Select the point with maximum min-distance (farthest from all selected)
        let next = (0..n)
            .filter(|i| !selected.contains(i))
            .max_by(|&a, &b| {
                min_distances[a]
                    .partial_cmp(&min_distances[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap();

        selected.push(next);
    }

    selected
}

/// Write type embeddings and label index to output directory.
///
/// - `embeddings`: [n_types * k, dim] interleaved (rows 0..K for type 0, etc.)
/// - `labels`: ordered type labels
/// - `output_dir`: directory to write to
pub fn write_type_embeddings(
    embeddings: &[f32],
    n_types: usize,
    k: usize,
    dim: usize,
    labels: &[String],
    output_dir: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output_dir)?;

    // Write type_embeddings.safetensors
    let total_rows = n_types * k;
    let tensor_data: Vec<u8> = embeddings.iter().flat_map(|f| f.to_le_bytes()).collect();

    let mut tensors = std::collections::HashMap::new();
    tensors.insert(
        "embeddings".to_string(),
        safetensors::tensor::TensorView::new(
            safetensors::Dtype::F32,
            [total_rows, dim].to_vec(),
            &tensor_data,
        )?,
    );
    safetensors::tensor::serialize_to_file(
        &tensors,
        &None,
        &output_dir.join("type_embeddings.safetensors"),
    )
    .context("Failed to write type_embeddings.safetensors")?;

    // Write label_index.json
    let label_json = serde_json::to_string_pretty(labels)?;
    std::fs::write(output_dir.join("label_index.json"), label_json)?;

    tracing::info!(
        "Wrote type embeddings: {} types × {} reps = {} rows × {} dim",
        n_types,
        k,
        total_rows,
        dim,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fps_selects_k() {
        // 4 points in 2D, select 2
        let embeddings = vec![
            0.0, 0.0, // point 0
            1.0, 0.0, // point 1
            0.0, 1.0, // point 2
            1.0, 1.0, // point 3
        ];
        let selected = farthest_point_sampling(&embeddings, 4, 2, 2);
        assert_eq!(selected.len(), 2);
        // First is always 0, second should be farthest (point 3: diagonal)
        assert_eq!(selected[0], 0);
        assert_eq!(selected[1], 3);
    }

    #[test]
    fn test_fps_all_when_k_ge_n() {
        let embeddings = vec![0.0, 1.0, 2.0];
        let selected = farthest_point_sampling(&embeddings, 3, 1, 5);
        assert_eq!(selected, vec![0, 1, 2]);
    }

    #[test]
    fn test_fps_single() {
        let embeddings = vec![1.0, 2.0, 3.0, 4.0];
        let selected = farthest_point_sampling(&embeddings, 2, 2, 1);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0], 0);
    }
}
