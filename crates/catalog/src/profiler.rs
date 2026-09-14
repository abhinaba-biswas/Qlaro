use core::error::QlaroError;
use core::types::CanonicalScalar;
use arrow_array::{
    Array, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array,
    Int8Array, RecordBatch, StringArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use arrow_schema::DataType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary statistics for a single column
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnProfile {
    pub name: String,
    pub data_type: String,
    pub total_count: u64,
    pub null_count: u64,
    pub null_percentage: f64,
    pub distinct_count_approx: Option<u64>,
    pub min_value: Option<CanonicalScalar>,
    pub max_value: Option<CanonicalScalar>,
}

/// Comprehensive summary profile for a dataset
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetProfile {
    pub row_count: u64,
    pub column_count: usize,
    pub columns: Vec<ColumnProfile>,
    pub quality_violations: Vec<QualityViolation>,
}

/// Quality assertion rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QualityRule {
    MaxNullPercentage { column: String, max_allowed: f64 },
    MinRowCount(u64),
    NonEmptyColumn(String),
}

/// Detected quality rule violation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityViolation {
    pub rule_description: String,
    pub column: Option<String>,
    pub details: String,
}

/// Profiler engine for analyzing RecordBatches
pub struct DatasetProfiler;

impl DatasetProfiler {
    /// Profiles a slice of RecordBatches and verifies quality rules
    pub fn profile_batches(
        batches: &[RecordBatch],
        rules: &[QualityRule],
    ) -> Result<DatasetProfile, QlaroError> {
        if batches.is_empty() {
            let profile = DatasetProfile {
                row_count: 0,
                column_count: 0,
                columns: Vec::new(),
                quality_violations: Self::evaluate_rules(0, &[], rules),
            };
            return Ok(profile);
        }

        let schema = batches[0].schema();
        let total_rows: u64 = batches.iter().map(|b| b.num_rows() as u64).sum();
        let num_cols = schema.fields().len();

        let mut col_profiles = Vec::with_capacity(num_cols);

        for (col_idx, field) in schema.fields().iter().enumerate() {
            let mut null_count = 0u64;
            let mut min_val: Option<CanonicalScalar> = None;
            let mut max_val: Option<CanonicalScalar> = None;

            for batch in batches {
                let col = batch.column(col_idx);
                null_count += col.null_count() as u64;
                let (b_min, b_max) = extract_min_max(col);

                min_val = update_min(min_val, b_min);
                max_val = update_max(max_val, b_max);
            }

            let null_pct = if total_rows > 0 {
                (null_count as f64 / total_rows as f64) * 100.0
            } else {
                0.0
            };

            col_profiles.push(ColumnProfile {
                name: field.name().clone(),
                data_type: format!("{:?}", field.data_type()),
                total_count: total_rows,
                null_count,
                null_percentage: null_pct,
                distinct_count_approx: None,
                min_value: min_val,
                max_value: max_val,
            });
        }

        let violations = Self::evaluate_rules(total_rows, &col_profiles, rules);

        Ok(DatasetProfile {
            row_count: total_rows,
            column_count: num_cols,
            columns: col_profiles,
            quality_violations: violations,
        })
    }

    fn evaluate_rules(
        row_count: u64,
        cols: &[ColumnProfile],
        rules: &[QualityRule],
    ) -> Vec<QualityViolation> {
        let mut violations = Vec::new();
        let col_map: HashMap<&str, &ColumnProfile> =
            cols.iter().map(|c| (c.name.as_str(), c)).collect();

        for rule in rules {
            match rule {
                QualityRule::MinRowCount(min_rows) => {
                    if row_count < *min_rows {
                        violations.push(QualityViolation {
                            rule_description: format!("Minimum row count requirement of {}", min_rows),
                            column: None,
                            details: format!("Dataset contains {} rows, expected at least {}", row_count, min_rows),
                        });
                    }
                }
                QualityRule::MaxNullPercentage { column, max_allowed } => {
                    if let Some(col_prof) = col_map.get(column.as_str()) {
                        if col_prof.null_percentage > *max_allowed {
                            violations.push(QualityViolation {
                                rule_description: format!("Max null threshold of {:.1}% for column '{}'", max_allowed, column),
                                column: Some(column.clone()),
                                details: format!("Column '{}' has {:.2}% nulls, exceeding limit of {:.1}%", column, col_prof.null_percentage, max_allowed),
                            });
                        }
                    }
                }
                QualityRule::NonEmptyColumn(column) => {
                    if let Some(col_prof) = col_map.get(column.as_str()) {
                        if col_prof.null_count == col_prof.total_count && col_prof.total_count > 0 {
                            violations.push(QualityViolation {
                                rule_description: format!("Non-empty column requirement for '{}'", column),
                                column: Some(column.clone()),
                                details: format!("Column '{}' is 100% null", column),
                            });
                        }
                    }
                }
            }
        }

        violations
    }
}

fn extract_min_max(array: &std::sync::Arc<dyn Array>) -> (Option<CanonicalScalar>, Option<CanonicalScalar>) {
    if array.len() == 0 || array.null_count() == array.len() {
        return (None, None);
    }

    match array.data_type() {
        DataType::Int8 => {
            let arr = array.as_any().downcast_ref::<Int8Array>().unwrap();
            let mut min = i8::MAX;
            let mut max = i8::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Int8(min)), Some(CanonicalScalar::Int8(max)))
        }
        DataType::Int16 => {
            let arr = array.as_any().downcast_ref::<Int16Array>().unwrap();
            let mut min = i16::MAX;
            let mut max = i16::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Int16(min)), Some(CanonicalScalar::Int16(max)))
        }
        DataType::Int32 => {
            let arr = array.as_any().downcast_ref::<Int32Array>().unwrap();
            let mut min = i32::MAX;
            let mut max = i32::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Int32(min)), Some(CanonicalScalar::Int32(max)))
        }
        DataType::Int64 => {
            let arr = array.as_any().downcast_ref::<Int64Array>().unwrap();
            let mut min = i64::MAX;
            let mut max = i64::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Int64(min)), Some(CanonicalScalar::Int64(max)))
        }
        DataType::UInt8 => {
            let arr = array.as_any().downcast_ref::<UInt8Array>().unwrap();
            let mut min = u8::MAX;
            let mut max = u8::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::UInt8(min)), Some(CanonicalScalar::UInt8(max)))
        }
        DataType::UInt16 => {
            let arr = array.as_any().downcast_ref::<UInt16Array>().unwrap();
            let mut min = u16::MAX;
            let mut max = u16::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::UInt16(min)), Some(CanonicalScalar::UInt16(max)))
        }
        DataType::UInt32 => {
            let arr = array.as_any().downcast_ref::<UInt32Array>().unwrap();
            let mut min = u32::MAX;
            let mut max = u32::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::UInt32(min)), Some(CanonicalScalar::UInt32(max)))
        }
        DataType::UInt64 => {
            let arr = array.as_any().downcast_ref::<UInt64Array>().unwrap();
            let mut min = u64::MAX;
            let mut max = u64::MIN;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::UInt64(min)), Some(CanonicalScalar::UInt64(max)))
        }
        DataType::Float32 => {
            let arr = array.as_any().downcast_ref::<Float32Array>().unwrap();
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Float32(min)), Some(CanonicalScalar::Float32(max)))
        }
        DataType::Float64 => {
            let arr = array.as_any().downcast_ref::<Float64Array>().unwrap();
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i);
                    if v < min { min = v; }
                    if v > max { max = v; }
                }
            }
            (Some(CanonicalScalar::Float64(min)), Some(CanonicalScalar::Float64(max)))
        }
        DataType::Utf8 => {
            let arr = array.as_any().downcast_ref::<StringArray>().unwrap();
            let mut min: Option<String> = None;
            let mut max: Option<String> = None;
            for i in 0..arr.len() {
                if !arr.is_null(i) {
                    let v = arr.value(i).to_string();
                    if min.as_ref().map_or(true, |m| &v < m) { min = Some(v.clone()); }
                    if max.as_ref().map_or(true, |m| &v > m) { max = Some(v); }
                }
            }
            (min.map(CanonicalScalar::Utf8), max.map(CanonicalScalar::Utf8))
        }
        _ => (None, None),
    }
}

fn update_min(current: Option<CanonicalScalar>, new_val: Option<CanonicalScalar>) -> Option<CanonicalScalar> {
    match (current, new_val) {
        (None, n) => n,
        (c, None) => c,
        (Some(CanonicalScalar::Int64(c)), Some(CanonicalScalar::Int64(n))) => Some(CanonicalScalar::Int64(c.min(n))),
        (Some(CanonicalScalar::Int32(c)), Some(CanonicalScalar::Int32(n))) => Some(CanonicalScalar::Int32(c.min(n))),
        (Some(CanonicalScalar::Float64(c)), Some(CanonicalScalar::Float64(n))) => Some(CanonicalScalar::Float64(c.min(n))),
        (Some(CanonicalScalar::Utf8(c)), Some(CanonicalScalar::Utf8(n))) => Some(CanonicalScalar::Utf8(if n < c { n } else { c })),
        (c, _) => c,
    }
}

fn update_max(current: Option<CanonicalScalar>, new_val: Option<CanonicalScalar>) -> Option<CanonicalScalar> {
    match (current, new_val) {
        (None, n) => n,
        (c, None) => c,
        (Some(CanonicalScalar::Int64(c)), Some(CanonicalScalar::Int64(n))) => Some(CanonicalScalar::Int64(c.max(n))),
        (Some(CanonicalScalar::Int32(c)), Some(CanonicalScalar::Int32(n))) => Some(CanonicalScalar::Int32(c.max(n))),
        (Some(CanonicalScalar::Float64(c)), Some(CanonicalScalar::Float64(n))) => Some(CanonicalScalar::Float64(c.max(n))),
        (Some(CanonicalScalar::Utf8(c)), Some(CanonicalScalar::Utf8(n))) => Some(CanonicalScalar::Utf8(if n > c { n } else { c })),
        (c, _) => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, Float64Array, Int64Array, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;

    #[test]
    fn test_profiling_and_quality_rule_evaluation() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("user_id", DataType::Int64, false),
            Field::new("score", DataType::Float64, true),
            Field::new("country", DataType::Utf8, true),
        ]));

        let user_ids = Arc::new(Int64Array::from(vec![10, 20, 30, 40])) as ArrayRef;
        let scores = Arc::new(Float64Array::from(vec![Some(85.5), None, Some(92.0), Some(71.2)])) as ArrayRef;
        let countries = Arc::new(StringArray::from(vec![Some("US"), Some("DE"), None, Some("CA")])) as ArrayRef;

        let batch = RecordBatch::try_new(schema, vec![user_ids, scores, countries]).unwrap();

        let rules = vec![
            QualityRule::MinRowCount(5), // Should fail (only 4 rows)
            QualityRule::MaxNullPercentage {
                column: "score".to_string(),
                max_allowed: 15.0, // Should fail (25% nulls)
            },
            QualityRule::NonEmptyColumn("country".to_string()), // Should pass
        ];

        let profile = DatasetProfiler::profile_batches(&[batch], &rules).unwrap();

        assert_eq!(profile.row_count, 4);
        assert_eq!(profile.column_count, 3);

        // Check user_id profile
        let user_prof = &profile.columns[0];
        assert_eq!(user_prof.name, "user_id");
        assert_eq!(user_prof.null_count, 0);
        assert_eq!(user_prof.min_value, Some(CanonicalScalar::Int64(10)));
        assert_eq!(user_prof.max_value, Some(CanonicalScalar::Int64(40)));

        // Check score profile
        let score_prof = &profile.columns[1];
        assert_eq!(score_prof.null_count, 1);
        assert_eq!(score_prof.null_percentage, 25.0);

        // Quality rule violations: 2 failures
        assert_eq!(profile.quality_violations.len(), 2);
        assert!(profile.quality_violations[0].details.contains("expected at least 5"));
        assert!(profile.quality_violations[1].details.contains("exceeding limit of 15.0%"));
    }
}
