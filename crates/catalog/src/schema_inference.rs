use core::error::{QlaroError, QlaroErrorCode};
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use std::io::{BufRead, BufReader, Read, Seek};
use std::sync::Arc;

/// Schema Inference Engine for CSV and untyped tabular text formats
#[derive(Debug, Clone)]
pub struct SchemaInferenceEngine {
    pub max_sample_rows: usize,
    pub delimiter: u8,
    pub has_header: bool,
    pub null_values: Vec<String>,
}

impl Default for SchemaInferenceEngine {
    fn default() -> Self {
        Self {
            max_sample_rows: 1000,
            delimiter: b',',
            has_header: true,
            null_values: vec![
                "".to_string(),
                "null".to_string(),
                "NULL".to_string(),
                "None".to_string(),
                "NA".to_string(),
                "N/A".to_string(),
                "nan".to_string(),
                "NaN".to_string(),
            ],
        }
    }
}

impl SchemaInferenceEngine {
    pub fn new(max_sample_rows: usize, delimiter: u8, has_header: bool) -> Self {
        Self {
            max_sample_rows,
            delimiter,
            has_header,
            ..Default::default()
        }
    }

    /// Infers an Apache Arrow Schema from a readable CSV stream
    pub fn infer_schema<R: Read + Seek>(&self, mut reader: R) -> Result<Arc<Schema>, QlaroError> {
        let mut buf_reader = BufReader::new(&mut reader);
        let mut lines = Vec::new();

        let mut line = String::new();
        while lines.len() < self.max_sample_rows && buf_reader.read_line(&mut line).unwrap_or(0) > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                lines.push(trimmed.to_string());
            }
            line.clear();
        }

        if lines.is_empty() {
            return Err(QlaroError::Core(QlaroErrorCode::InvalidFormat(
                "Cannot infer schema from empty input stream".to_string(),
            )));
        }

        let delimiter_char = self.delimiter as char;

        let (header_names, data_lines) = if self.has_header {
            let header_row = parse_csv_line(&lines[0], delimiter_char);
            (header_row, &lines[1..])
        } else {
            let first_row = parse_csv_line(&lines[0], delimiter_char);
            let generated_headers = (0..first_row.len())
                .map(|i| format!("column_{}", i + 1))
                .collect();
            (generated_headers, &lines[..])
        };

        if header_names.is_empty() {
            return Err(QlaroError::Core(QlaroErrorCode::InvalidFormat(
                "Failed to parse column headers from CSV".to_string(),
            )));
        }

        let num_cols = header_names.len();
        let mut col_types = vec![InferredType::Null; num_cols];

        for row_str in data_lines {
            let parsed_cols = parse_csv_line(row_str, delimiter_char);
            for (idx, type_acc) in col_types.iter_mut().enumerate() {
                if let Some(val) = parsed_cols.get(idx) {
                    let cell_type = self.infer_cell_type(val.trim());
                    *type_acc = type_acc.merge(cell_type);
                }
            }
        }

        let fields: Vec<Field> = header_names
            .into_iter()
            .zip(col_types)
            .map(|(name, inf_type)| {
                Field::new(name, inf_type.to_arrow_datatype(), true)
            })
            .collect();

        Ok(Arc::new(Schema::new(fields)))
    }

    fn infer_cell_type(&self, val: &str) -> InferredType {
        if self.null_values.iter().any(|n| n == val) {
            return InferredType::Null;
        }

        if val.eq_ignore_ascii_case("true") || val.eq_ignore_ascii_case("false") {
            return InferredType::Boolean;
        }

        if let Ok(_) = val.parse::<i64>() {
            return InferredType::Int64;
        }

        if let Ok(_) = val.parse::<f64>() {
            return InferredType::Float64;
        }

        // Date (YYYY-MM-DD)
        if chrono::NaiveDate::parse_from_str(val, "%Y-%m-%d").is_ok() {
            return InferredType::Date32;
        }

        // Timestamp (ISO-8601 / RFC-3339)
        if chrono::DateTime::parse_from_rfc3339(val).is_ok()
            || chrono::NaiveDateTime::parse_from_str(val, "%Y-%m-%d %H:%M:%S").is_ok()
            || chrono::NaiveDateTime::parse_from_str(val, "%Y-%m-%dT%H:%M:%S").is_ok()
        {
            return InferredType::TimestampMicros;
        }

        InferredType::Utf8
    }
}

fn parse_csv_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == delimiter && !in_quotes {
            fields.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    fields.push(current.trim().to_string());
    fields
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InferredType {
    Null,
    Boolean,
    Int64,
    Float64,
    Date32,
    TimestampMicros,
    Utf8,
}

impl InferredType {
    fn merge(self, other: InferredType) -> InferredType {
        match (self, other) {
            (InferredType::Null, t) | (t, InferredType::Null) => t,
            (InferredType::Boolean, InferredType::Boolean) => InferredType::Boolean,
            (InferredType::Int64, InferredType::Int64) => InferredType::Int64,
            (InferredType::Int64, InferredType::Float64) | (InferredType::Float64, InferredType::Int64) => {
                InferredType::Float64
            }
            (InferredType::Float64, InferredType::Float64) => InferredType::Float64,
            (InferredType::Date32, InferredType::Date32) => InferredType::Date32,
            (InferredType::Date32, InferredType::TimestampMicros)
            | (InferredType::TimestampMicros, InferredType::Date32) => InferredType::TimestampMicros,
            (InferredType::TimestampMicros, InferredType::TimestampMicros) => {
                InferredType::TimestampMicros
            }
            _ => InferredType::Utf8, // Fallback to Utf8 string on mixed types
        }
    }

    fn to_arrow_datatype(self) -> DataType {
        match self {
            InferredType::Null | InferredType::Utf8 => DataType::Utf8,
            InferredType::Boolean => DataType::Boolean,
            InferredType::Int64 => DataType::Int64,
            InferredType::Float64 => DataType::Float64,
            InferredType::Date32 => DataType::Date32,
            InferredType::TimestampMicros => DataType::Timestamp(TimeUnit::Microsecond, None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_csv_schema_inference_primitives() {
        let csv_data = "id,name,active,balance,registered_at,birth_date\n\
                        1,Alice,true,1250.50,2026-01-15T08:30:00Z,1990-05-12\n\
                        2,Bob,false,300.00,2026-02-20T14:15:00Z,1985-11-23\n\
                        3,Charlie,true,,2026-03-01 10:00:00,1995-07-04";

        let engine = SchemaInferenceEngine::default();
        let schema = engine.infer_schema(Cursor::new(csv_data)).unwrap();

        assert_eq!(schema.fields().len(), 6);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(0).data_type(), &DataType::Int64);

        assert_eq!(schema.field(1).name(), "name");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);

        assert_eq!(schema.field(2).name(), "active");
        assert_eq!(schema.field(2).data_type(), &DataType::Boolean);

        assert_eq!(schema.field(3).name(), "balance");
        assert_eq!(schema.field(3).data_type(), &DataType::Float64);

        assert_eq!(schema.field(4).name(), "registered_at");
        assert_eq!(
            schema.field(4).data_type(),
            &DataType::Timestamp(TimeUnit::Microsecond, None)
        );

        assert_eq!(schema.field(5).name(), "birth_date");
        assert_eq!(schema.field(5).data_type(), &DataType::Date32);
    }

    #[test]
    fn test_csv_schema_type_promotion_and_fallback() {
        // Mixed int and float should promote to Float64, mixed with string falls back to Utf8
        let csv_data = "score,mixed\n\
                        100,true\n\
                        200.75,not_a_bool\n\
                        300,false";

        let engine = SchemaInferenceEngine::default();
        let schema = engine.infer_schema(Cursor::new(csv_data)).unwrap();

        assert_eq!(schema.field(0).name(), "score");
        assert_eq!(schema.field(0).data_type(), &DataType::Float64);

        assert_eq!(schema.field(1).name(), "mixed");
        assert_eq!(schema.field(1).data_type(), &DataType::Utf8);
    }

    #[test]
    fn test_csv_empty_input_fails() {
        let engine = SchemaInferenceEngine::default();
        let res = engine.infer_schema(Cursor::new(""));
        assert!(res.is_err());
    }
}
