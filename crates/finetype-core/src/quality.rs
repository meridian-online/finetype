//! Per-column quality scores and file-level quality grades.

use std::fmt;

/// Quality score for a single column based on validation results.
#[derive(Debug, Clone)]
pub struct ColumnQualityScore {
    /// Fraction of non-null values that pass JSON Schema validation (0.0–1.0).
    pub type_conforming_rate: f64,
    /// Fraction of total values that are null (0.0–1.0).
    pub null_rate: f64,
    /// Fraction of total values that are non-null (1.0 - null_rate).
    pub completeness: f64,
    /// Composite quality score: type_conforming_rate × completeness (0.0–1.0).
    pub quality_score: f64,
}

/// File-level quality grade based on aggregate column quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileQualityGrade {
    A,
    B,
    C,
    D,
    F,
}

impl fmt::Display for FileQualityGrade {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::B => write!(f, "B"),
            Self::C => write!(f, "C"),
            Self::D => write!(f, "D"),
            Self::F => write!(f, "F"),
        }
    }
}

impl FileQualityGrade {
    /// Map a score (0.0–1.0) to a grade.
    pub fn from_score(score: f64) -> Self {
        if score >= 0.95 {
            Self::A
        } else if score >= 0.85 {
            Self::B
        } else if score >= 0.70 {
            Self::C
        } else if score >= 0.50 {
            Self::D
        } else {
            Self::F
        }
    }
}

/// Compute quality score for a column from validation counts.
///
/// - `valid_count`: values passing schema validation
/// - `invalid_count`: values failing schema validation
/// - `null_count`: null/missing values
pub fn compute_column_quality(
    valid_count: usize,
    invalid_count: usize,
    null_count: usize,
) -> ColumnQualityScore {
    let total = valid_count + invalid_count + null_count;
    if total == 0 {
        return ColumnQualityScore {
            type_conforming_rate: 0.0,
            null_rate: 0.0,
            completeness: 0.0,
            quality_score: 0.0,
        };
    }

    let non_null = valid_count + invalid_count;
    let type_conforming_rate = if non_null > 0 {
        valid_count as f64 / non_null as f64
    } else {
        0.0
    };
    let null_rate = null_count as f64 / total as f64;
    let completeness = 1.0 - null_rate;
    let quality_score = type_conforming_rate * completeness;

    ColumnQualityScore {
        type_conforming_rate,
        null_rate,
        completeness,
        quality_score,
    }
}

/// Compute file-level grade from column quality scores.
/// Uses the mean quality_score across all columns.
pub fn compute_file_grade(scores: &[ColumnQualityScore]) -> FileQualityGrade {
    if scores.is_empty() {
        return FileQualityGrade::F;
    }
    let mean = scores.iter().map(|s| s.quality_score).sum::<f64>() / scores.len() as f64;
    FileQualityGrade::from_score(mean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grade_thresholds() {
        assert_eq!(FileQualityGrade::from_score(1.0), FileQualityGrade::A);
        assert_eq!(FileQualityGrade::from_score(0.95), FileQualityGrade::A);
        assert_eq!(FileQualityGrade::from_score(0.94), FileQualityGrade::B);
        assert_eq!(FileQualityGrade::from_score(0.85), FileQualityGrade::B);
        assert_eq!(FileQualityGrade::from_score(0.84), FileQualityGrade::C);
        assert_eq!(FileQualityGrade::from_score(0.70), FileQualityGrade::C);
        assert_eq!(FileQualityGrade::from_score(0.69), FileQualityGrade::D);
        assert_eq!(FileQualityGrade::from_score(0.50), FileQualityGrade::D);
        assert_eq!(FileQualityGrade::from_score(0.49), FileQualityGrade::F);
        assert_eq!(FileQualityGrade::from_score(0.0), FileQualityGrade::F);
    }

    #[test]
    fn test_column_quality_perfect() {
        let q = compute_column_quality(100, 0, 0);
        assert_eq!(q.type_conforming_rate, 1.0);
        assert_eq!(q.null_rate, 0.0);
        assert_eq!(q.completeness, 1.0);
        assert_eq!(q.quality_score, 1.0);
    }

    #[test]
    fn test_column_quality_with_nulls() {
        let q = compute_column_quality(80, 0, 20);
        assert_eq!(q.type_conforming_rate, 1.0);
        assert!((q.null_rate - 0.2).abs() < 1e-10);
        assert!((q.completeness - 0.8).abs() < 1e-10);
        assert!((q.quality_score - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_column_quality_with_invalids() {
        let q = compute_column_quality(75, 25, 0);
        assert!((q.type_conforming_rate - 0.75).abs() < 1e-10);
        assert_eq!(q.null_rate, 0.0);
        assert_eq!(q.completeness, 1.0);
        assert!((q.quality_score - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_column_quality_mixed() {
        // 60 valid, 20 invalid, 20 null → total 100
        // type_conforming = 60/80 = 0.75, completeness = 0.8, score = 0.6
        let q = compute_column_quality(60, 20, 20);
        assert!((q.type_conforming_rate - 0.75).abs() < 1e-10);
        assert!((q.null_rate - 0.2).abs() < 1e-10);
        assert!((q.completeness - 0.8).abs() < 1e-10);
        assert!((q.quality_score - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_column_quality_empty() {
        let q = compute_column_quality(0, 0, 0);
        assert_eq!(q.quality_score, 0.0);
    }

    #[test]
    fn test_column_quality_all_null() {
        let q = compute_column_quality(0, 0, 100);
        assert_eq!(q.type_conforming_rate, 0.0);
        assert_eq!(q.null_rate, 1.0);
        assert_eq!(q.completeness, 0.0);
        assert_eq!(q.quality_score, 0.0);
    }

    #[test]
    fn test_file_grade_perfect() {
        let scores = vec![
            compute_column_quality(100, 0, 0),
            compute_column_quality(100, 0, 0),
        ];
        assert_eq!(compute_file_grade(&scores), FileQualityGrade::A);
    }

    #[test]
    fn test_file_grade_mixed() {
        let scores = vec![
            compute_column_quality(100, 0, 0),  // 1.0
            compute_column_quality(60, 20, 20), // 0.6
        ];
        // mean = 0.8 → C
        assert_eq!(compute_file_grade(&scores), FileQualityGrade::C);
    }

    #[test]
    fn test_file_grade_empty() {
        assert_eq!(compute_file_grade(&[]), FileQualityGrade::F);
    }

    #[test]
    fn test_grade_display() {
        assert_eq!(format!("{}", FileQualityGrade::A), "A");
        assert_eq!(format!("{}", FileQualityGrade::F), "F");
    }
}
