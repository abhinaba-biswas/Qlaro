use core::arrow_contract::validate_canonical_schema;
use core::error::{QlaroError, QlaroErrorCode};
use core::types::StorageFormat;
use arrow_array::RecordBatch;
use arrow_schema::Schema;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Storage format from file magic bytes and extension
pub fn detect_storage_format<P: AsRef<Path>>(path: P) -> Result<StorageFormat, QlaroError> {
    let p = path.as_ref();
    if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
        match ext.to_lowercase().as_str() {
            "parquet" | "pq" => return Ok(StorageFormat::Parquet),
            "arrow" | "feather" | "ipc" => return Ok(StorageFormat::ArrowIpc),
            "csv" | "tsv" => return Ok(StorageFormat::Csv),
            _ => {}
        }
    }

    //  magic bytes inspection if extension is ambiguous
    let mut file = File::open(p).map_err(|e| QlaroError::Io(e.to_string()))?;
    let mut magic = [0u8; 4];
    use std::io::Read;
    if file.read_exact(&mut magic).is_ok() {
        if &magic == b"PAR1" {
            return Ok(StorageFormat::Parquet);
        }
        if &magic == b"ARR1" {
            return Ok(StorageFormat::ArrowIpc);
        }
    }

    Err(QlaroError::Core(QlaroErrorCode::InvalidFormat(format!(
        "Unable to determine storage format for {}",
        p.display()
    ))))
}

/// schema and sample reading from a Parquet file
pub fn inspect_parquet_file<P: AsRef<Path>>(
    path: P,
    sample_limit: Option<usize>,
) -> Result<(Arc<Schema>, Vec<RecordBatch>, u64), QlaroError> {
    let file = File::open(path.as_ref()).map_err(|e| QlaroError::Io(e.to_string()))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;

    let schema = builder.schema().clone();
    validate_canonical_schema(&schema)?;

    let mut reader = builder
        .with_batch_size(sample_limit.unwrap_or(1024))
        .build()
        .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;

    let mut batches = Vec::new();
    let mut total_rows = 0u64;

    while let Some(batch_res) = reader.next() {
        let batch = batch_res.map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
        total_rows += batch.num_rows() as u64;
        batches.push(batch);
        if let Some(limit) = sample_limit {
            if total_rows >= limit as u64 {
                break;
            }
        }
    }

    Ok((schema, batches, total_rows))
}
