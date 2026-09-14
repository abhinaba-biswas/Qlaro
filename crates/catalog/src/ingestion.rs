use crate::format_reader::inspect_dataset_file;
use crate::identity::compute_file_fingerprint;
use crate::metadata_model::CatalogStore;
use core::error::{DiagnosticDetail, QlaroError, QlaroErrorCode};
use core::types::{
    CancellationToken, DatasetFingerprint, DatasetId, IngestionLineage, IngestionOptions,
    IngestionSource, StorageFormat,
};
use arrow_schema::Schema;
use chrono::Utc;
use std::path::Path;
use std::sync::Arc;

pub const INGESTION_PIPELINE_VERSION: &str = "qlaro-ingestion-v1";

#[derive(Debug, Clone)]
pub struct IngestionResult {
    pub dataset_id: DatasetId,
    pub version_id: String,
    pub name: String,
    pub format: StorageFormat,
    pub schema: Arc<Schema>,
    pub row_count: u64,
    pub byte_size: u64,
    pub source_fingerprint: DatasetFingerprint,
    pub lineage: IngestionLineage,
    pub diagnostics: Vec<DiagnosticDetail>,
}

pub trait IngestionPipeline: Send + Sync {
    fn ingest(
        &self,
        source: &IngestionSource,
        options: &IngestionOptions,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<IngestionResult, QlaroError>;
}

pub struct DatasetIngestionService {
    catalog: Arc<dyn CatalogStore>,
}

impl DatasetIngestionService {
    pub fn new(catalog: Arc<dyn CatalogStore>) -> Self {
        Self { catalog }
    }
}

impl IngestionPipeline for DatasetIngestionService {
    fn ingest(
        &self,
        source: &IngestionSource,
        options: &IngestionOptions,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<IngestionResult, QlaroError> {
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                    "Ingestion cancelled before starting pipeline".to_string(),
                )));
            }
        }

        let path = Path::new(&source.source_path);
        if !path.exists() {
            return Err(QlaroError::Io(format!(
                "Source file not found: {}",
                path.display()
            )));
        }

        let metadata = std::fs::metadata(path).map_err(|e| QlaroError::Io(e.to_string()))?;
        let byte_size = metadata.len();

        if byte_size == 0 {
            return Err(QlaroError::Core(QlaroErrorCode::InvalidFormat(
                "Cannot ingest empty (0-byte) source file".to_string(),
            )));
        }

        if let Some(max_bytes) = options.max_bytes {
            if byte_size > max_bytes {
                return Err(QlaroError::Core(QlaroErrorCode::ResourceLimitExceeded(
                    format!(
                        "File size {} bytes exceeds configured limit of {} bytes",
                        byte_size, max_bytes
                    ),
                )));
            }
        }

        let fingerprint_str = compute_file_fingerprint(path)?;
        let fingerprint = DatasetFingerprint(fingerprint_str.clone());

        let (format, schema, _batches, row_count) = inspect_dataset_file(
            path,
            source.format_hint,
            options.sample_limit,
            cancel_token,
        )?;

        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                    "Ingestion cancelled before catalog registration".to_string(),
                )));
            }
        }

        let schema_json = serde_json::to_string(&schema)
            .map_err(|e| QlaroError::Serialization(e.to_string()))?;

        let record = self.catalog.register_dataset(
            &source.name,
            path,
            format,
            &schema_json,
            &fingerprint_str,
            row_count,
            byte_size,
        )?;

        let lineage = IngestionLineage {
            source_path: path.to_string_lossy().to_string(),
            source_fingerprint: fingerprint.clone(),
            ingested_at: Utc::now(),
            pipeline_version: INGESTION_PIPELINE_VERSION.to_string(),
        };

        let mut diagnostics = Vec::new();
        if row_count == 0 {
            diagnostics.push(DiagnosticDetail {
                code: QlaroErrorCode::CorruptedBatch("Zero rows ingested".to_string()),
                message: "Dataset schema registered successfully, but source contains 0 records"
                    .to_string(),
                suggested_action: Some("Verify that source file has valid records".to_string()),
            });
        }

        Ok(IngestionResult {
            dataset_id: record.metadata.id,
            version_id: record.current_version_id,
            name: source.name.clone(),
            format,
            schema,
            row_count,
            byte_size,
            source_fingerprint: fingerprint,
            lineage,
            diagnostics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_model::InMemoryCatalogStore;
    use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, StringArray};
    use arrow_ipc::writer::FileWriter as IpcFileWriter;
    use arrow_schema::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use std::fs::File;

    fn create_synthetic_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("txn_id", DataType::Int64, false),
            Field::new("amount", DataType::Float64, false),
            Field::new("category", DataType::Utf8, true),
        ]))
    }

    fn create_synthetic_batch(schema: Arc<Schema>) -> RecordBatch {
        let txn_id = Arc::new(Int64Array::from(vec![1, 2, 3, 4])) as ArrayRef;
        let amount = Arc::new(Float64Array::from(vec![19.99, 45.50, 100.0, 5.25])) as ArrayRef;
        let category = Arc::new(StringArray::from(vec![
            Some("groceries"),
            Some("dining"),
            Some("travel"),
            None,
        ])) as ArrayRef;

        RecordBatch::try_new(schema, vec![txn_id, amount, category]).unwrap()
    }

    #[test]
    fn test_ingest_parquet_success() {
        let catalog = Arc::new(InMemoryCatalogStore::new());
        let service = DatasetIngestionService::new(catalog.clone());

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("qlaro_ingest_test_{}.parquet", uuid::Uuid::new_v4()));

        let schema = create_synthetic_schema();
        let batch = create_synthetic_batch(schema.clone());

        {
            let file = File::create(&file_path).unwrap();
            let mut writer = ArrowWriter::try_new(file, schema.clone(), None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let source = IngestionSource {
            name: "transactions".to_string(),
            source_path: file_path.clone(),
            format_hint: Some(StorageFormat::Parquet),
        };

        let result = service.ingest(&source, &IngestionOptions::default(), None).unwrap();

        assert_eq!(result.name, "transactions");
        assert_eq!(result.format, StorageFormat::Parquet);
        assert_eq!(result.row_count, 4);
        assert!(result.byte_size > 0);
        assert!(!result.version_id.is_empty());
        assert_eq!(result.lineage.pipeline_version, INGESTION_PIPELINE_VERSION);

        let catalog_rec = catalog.get_dataset(&result.dataset_id).unwrap();
        assert_eq!(catalog_rec.metadata.name, "transactions");
        assert_eq!(catalog_rec.versions.len(), 1);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_ingest_arrow_ipc_success() {
        let catalog = Arc::new(InMemoryCatalogStore::new());
        let service = DatasetIngestionService::new(catalog.clone());

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("qlaro_ingest_test_{}.arrow", uuid::Uuid::new_v4()));

        let schema = create_synthetic_schema();
        let batch = create_synthetic_batch(schema.clone());

        {
            let file = File::create(&file_path).unwrap();
            let mut writer = IpcFileWriter::try_new(file, &schema).unwrap();
            writer.write(&batch).unwrap();
            writer.finish().unwrap();
        }

        let source = IngestionSource {
            name: "ipc_transactions".to_string(),
            source_path: file_path.clone(),
            format_hint: Some(StorageFormat::ArrowIpc),
        };

        let result = service.ingest(&source, &IngestionOptions::default(), None).unwrap();

        assert_eq!(result.name, "ipc_transactions");
        assert_eq!(result.format, StorageFormat::ArrowIpc);
        assert_eq!(result.row_count, 4);
        assert_eq!(result.schema.fields().len(), 3);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_ingest_resource_limit_exceeded() {
        let catalog = Arc::new(InMemoryCatalogStore::new());
        let service = DatasetIngestionService::new(catalog);

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("qlaro_limit_{}.parquet", uuid::Uuid::new_v4()));

        let schema = create_synthetic_schema();
        let batch = create_synthetic_batch(schema.clone());

        {
            let file = File::create(&file_path).unwrap();
            let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let source = IngestionSource {
            name: "oversized".to_string(),
            source_path: file_path.clone(),
            format_hint: None,
        };

        let options = IngestionOptions {
            strict_schema: true,
            max_bytes: Some(10), // unrealistic small limit
            sample_limit: None,
        };

        let res = service.ingest(&source, &options, None);
        assert!(res.is_err());
        match res.err().unwrap() {
            QlaroError::Core(QlaroErrorCode::ResourceLimitExceeded(msg)) => {
                assert!(msg.contains("exceeds configured limit"));
            }
            other => panic!("Expected ResourceLimitExceeded, got {:?}", other),
        }

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_ingest_cancellation() {
        let catalog = Arc::new(InMemoryCatalogStore::new());
        let service = DatasetIngestionService::new(catalog);

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join(format!("qlaro_cancel_{}.parquet", uuid::Uuid::new_v4()));

        let schema = create_synthetic_schema();
        let batch = create_synthetic_batch(schema.clone());

        {
            let file = File::create(&file_path).unwrap();
            let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
            writer.write(&batch).unwrap();
            writer.close().unwrap();
        }

        let source = IngestionSource {
            name: "cancelled_dataset".to_string(),
            source_path: file_path.clone(),
            format_hint: None,
        };

        let token = CancellationToken::new();
        token.cancel();

        let res = service.ingest(&source, &IngestionOptions::default(), Some(&token));
        assert!(res.is_err());
        match res.err().unwrap() {
            QlaroError::Core(QlaroErrorCode::Cancelled(msg)) => {
                assert!(msg.contains("cancelled"));
            }
            other => panic!("Expected Cancelled, got {:?}", other),
        }

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_ingest_empty_and_corrupted_file() {
        let catalog = Arc::new(InMemoryCatalogStore::new());
        let service = DatasetIngestionService::new(catalog);

        let temp_dir = std::env::temp_dir();
        let empty_path = temp_dir.join(format!("qlaro_empty_{}.parquet", uuid::Uuid::new_v4()));
        File::create(&empty_path).unwrap();

        let source_empty = IngestionSource {
            name: "empty_dataset".to_string(),
            source_path: empty_path.clone(),
            format_hint: None,
        };

        let res_empty = service.ingest(&source_empty, &IngestionOptions::default(), None);
        assert!(res_empty.is_err());

        let corrupted_path = temp_dir.join(format!("qlaro_corrupt_{}.parquet", uuid::Uuid::new_v4()));
        std::fs::write(&corrupted_path, b"PAR1garbage_corrupted_payload_without_footer").unwrap();

        let source_corrupted = IngestionSource {
            name: "corrupted_dataset".to_string(),
            source_path: corrupted_path.clone(),
            format_hint: Some(StorageFormat::Parquet),
        };

        let res_corrupted = service.ingest(&source_corrupted, &IngestionOptions::default(), None);
        assert!(res_corrupted.is_err());

        let _ = std::fs::remove_file(empty_path);
        let _ = std::fs::remove_file(corrupted_path);
    }
}
