use arrow_array::RecordBatch;
use arrow_ipc::reader::{FileReader as IpcFileReader, StreamReader as IpcStreamReader};
use arrow_schema::Schema;
use core::arrow_contract::validate_canonical_schema;
use core::error::{QlaroError, QlaroErrorCode};
use core::types::{CancellationToken, StorageFormat};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
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
    cancel_token: Option<&CancellationToken>,
) -> Result<(Arc<Schema>, Vec<RecordBatch>, u64), QlaroError> {
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                "Inspection cancelled before reading Parquet file".to_string(),
            )));
        }
    }

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
        if let Some(token) = cancel_token {
            if token.is_cancelled() {
                return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                    "Parquet inspection cancelled during batch processing".to_string(),
                )));
            }
        }
        let batch = batch_res
            .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
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

/// schema and sample reading from an Arrow IPC file or stream
pub fn inspect_arrow_ipc_file<P: AsRef<Path>>(
    path: P,
    sample_limit: Option<usize>,
    cancel_token: Option<&CancellationToken>,
) -> Result<(Arc<Schema>, Vec<RecordBatch>, u64), QlaroError> {
    if let Some(token) = cancel_token {
        if token.is_cancelled() {
            return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                "Inspection cancelled before reading Arrow IPC file".to_string(),
            )));
        }
    }

    let file = File::open(path.as_ref()).map_err(|e| QlaroError::Io(e.to_string()))?;
    let mut reader = BufReader::new(file);

    let (schema, batches, total_rows) = match IpcFileReader::try_new(&mut reader, None) {
        Ok(ipc_reader) => {
            let schema = ipc_reader.schema();
            validate_canonical_schema(&schema)?;

            let mut batches = Vec::new();
            let mut total_rows = 0u64;
            for batch_res in ipc_reader {
                if let Some(token) = cancel_token {
                    if token.is_cancelled() {
                        return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                            "Arrow IPC inspection cancelled during batch processing".to_string(),
                        )));
                    }
                }
                let batch = batch_res
                    .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
                total_rows += batch.num_rows() as u64;
                batches.push(batch);
                if let Some(limit) = sample_limit {
                    if total_rows >= limit as u64 {
                        break;
                    }
                }
            }
            (schema, batches, total_rows)
        }
        Err(_) => {
            reader
                .seek(SeekFrom::Start(0))
                .map_err(|e| QlaroError::Io(e.to_string()))?;

            let ipc_stream = IpcStreamReader::try_new(&mut reader, None)
                .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
            let schema = ipc_stream.schema();
            validate_canonical_schema(&schema)?;

            let mut batches = Vec::new();
            let mut total_rows = 0u64;
            for batch_res in ipc_stream {
                if let Some(token) = cancel_token {
                    if token.is_cancelled() {
                        return Err(QlaroError::Core(QlaroErrorCode::Cancelled(
                            "Arrow IPC stream inspection cancelled during batch processing"
                                .to_string(),
                        )));
                    }
                }
                let batch = batch_res
                    .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
                total_rows += batch.num_rows() as u64;
                batches.push(batch);
                if let Some(limit) = sample_limit {
                    if total_rows >= limit as u64 {
                        break;
                    }
                }
            }
            (schema, batches, total_rows)
        }
    };

    Ok((schema, batches, total_rows))
}

/// Unified file inspector for supported formats
pub fn inspect_dataset_file<P: AsRef<Path>>(
    path: P,
    format_hint: Option<StorageFormat>,
    sample_limit: Option<usize>,
    cancel_token: Option<&CancellationToken>,
) -> Result<(StorageFormat, Arc<Schema>, Vec<RecordBatch>, u64), QlaroError> {
    let p = path.as_ref();
    let format = match format_hint {
        Some(fmt) => fmt,
        None => detect_storage_format(p)?,
    };

    match format {
        StorageFormat::Parquet => {
            let (schema, batches, rows) = inspect_parquet_file(p, sample_limit, cancel_token)?;
            Ok((format, schema, batches, rows))
        }
        StorageFormat::ArrowIpc => {
            let (schema, batches, rows) = inspect_arrow_ipc_file(p, sample_limit, cancel_token)?;
            Ok((format, schema, batches, rows))
        }
        StorageFormat::Csv => Err(QlaroError::Core(QlaroErrorCode::InvalidFormat(
            "CSV format requires schema inference engine ".to_string(),
        ))),
    }
}
