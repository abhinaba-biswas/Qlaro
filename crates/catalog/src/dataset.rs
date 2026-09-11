use crate::format_reader::detect_storage_format;
use crate::identity::compute_file_fingerprint;
use core::error::QlaroError;
use core::types::{DatasetFingerprint, DatasetId, DatasetMetadata, StorageFormat};
use chrono::Utc;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DatasetRegistration {
    pub metadata: DatasetMetadata,
    pub source_path: PathBuf,
}

impl DatasetRegistration {
    pub fn from_file<P: AsRef<Path>>(name: String, path: P) -> Result<Self, QlaroError> {
        let p = path.as_ref().to_path_buf();
        let format = detect_storage_format(&p)?;
        let hash = compute_file_fingerprint(&p)?;
        let file_meta = std::fs::metadata(&p).map_err(|e| QlaroError::Io(e.to_string()))?;

        let metadata = DatasetMetadata {
            id: DatasetId::new_v4(),
            name,
            fingerprint: DatasetFingerprint(hash),
            format,
            row_count: 0, // Inferred asynchronously during profiling
            byte_size: file_meta.len(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        Ok(Self {
            metadata,
            source_path: p,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, Int32Array, RecordBatch, StringArray};
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use std::fs::File;
    use std::sync::Arc;

    #[test]
    fn test_parquet_file_registration_and_fingerprint() {
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("qlaro_test_{}.parquet", uuid::Uuid::new_v4()));

        let schema = Arc::new(Schema::new(vec![
            Field::new("user_id", DataType::Int32, false),
            Field::new("city", DataType::Utf8, true),
        ]));

        let user_id = Arc::new(Int32Array::from(vec![101, 102, 103])) as ArrayRef;
        let city = Arc::new(StringArray::from(vec![Some("Berlin"), Some("Tokyo"), None])) as ArrayRef;
        let batch = RecordBatch::try_new(schema.clone(), vec![user_id, city]).unwrap();

        {
            let file = File::create(&file_path).unwrap();
            let mut writer = ArrowWriter::try_new(file, schema.clone(), None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let reg = DatasetRegistration::from_file("test_users".to_string(), &file_path).unwrap();
        assert_eq!(reg.metadata.name, "test_users");
        assert_eq!(reg.metadata.format, StorageFormat::Parquet);
        assert!(!reg.metadata.fingerprint.0.is_empty());
        assert!(reg.metadata.byte_size > 0);

        let _ = std::fs::remove_file(file_path);
    }
}
