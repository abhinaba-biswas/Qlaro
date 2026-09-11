use crate::error::{QlaroError, QlaroErrorCode};
use arrow_array::RecordBatch;
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;
use arrow_schema::{DataType, Schema, TimeUnit};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::sync::Arc;

/// Validates that an Arrow schema adheres strictly to Qlaro canonical type
pub fn validate_canonical_schema(schema: &Schema) -> Result<(), QlaroError> {
    for field in schema.fields() {
        validate_canonical_datatype(field.data_type()).map_err(|e| {
            QlaroError::Core(QlaroErrorCode::UnsupportedDataType(format!(
                "Field '{}': {}",
                field.name(),
                e
            )))
        })?;
    }
    Ok(())
}

fn validate_canonical_datatype(dtype: &DataType) -> Result<(), String> {
    match dtype {
        DataType::Boolean
        | DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64
        | DataType::Float32
        | DataType::Float64
        | DataType::Utf8
        | DataType::LargeUtf8
        | DataType::Date32
        | DataType::Timestamp(TimeUnit::Microsecond, _)
        | DataType::Timestamp(TimeUnit::Millisecond, _)
        | DataType::Timestamp(TimeUnit::Nanosecond, _) => Ok(()),

        DataType::List(inner) => validate_canonical_datatype(inner.data_type()),
        DataType::Struct(fields) => {
            for f in fields {
                validate_canonical_datatype(f.data_type())?;
            }
            Ok(())
        }
        unsupported => Err(format!(
            "{:?} is not a supported canonical Arrow data type in Qlaro",
            unsupported
        )),
    }
}

/// Serializes RecordBatches into an Arrow IPC Stream buffer
pub fn serialize_record_batches(
    schema: &Arc<Schema>,
    batches: &[RecordBatch],
) -> Result<Vec<u8>, QlaroError> {
    validate_canonical_schema(schema)?;
    let mut buffer = Vec::new();
    {
        let mut writer = StreamWriter::try_new(&mut buffer, schema)?;
        for batch in batches {
            writer.write(batch)?;
        }
        writer.finish()?;
    }
    Ok(buffer)
}

/// Deserializes RecordBatches from an Arrow IPC Stream buffer
pub fn deserialize_record_batches(
    bytes: &[u8],
) -> Result<(Arc<Schema>, Vec<RecordBatch>), QlaroError> {
    let cursor = Cursor::new(bytes);
    let mut reader = StreamReader::try_new(cursor, None)?;
    let schema = reader.schema();
    validate_canonical_schema(&schema)?;

    let mut batches = Vec::new();
    while let Some(batch_res) = reader.next() {
        let batch = batch_res
            .map_err(|e| QlaroError::Core(QlaroErrorCode::CorruptedBatch(e.to_string())))?;
        batches.push(batch);
    }
    Ok((schema, batches))
}

/// Computes a deterministic cryptographic content hash over a set of RecordBatches
pub fn compute_batches_fingerprint(batches: &[RecordBatch]) -> Result<String, QlaroError> {
    let mut hasher = Sha256::new();
    for batch in batches {
        for col in batch.columns() {
            let data = col.to_data();
            for buf in data.buffers() {
                hasher.update(buf.as_slice());
            }
            if let Some(null_buf) = data.nulls() {
                hasher.update(null_buf.buffer().as_slice());
            }
        }
    }
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{ArrayRef, Int32Array, StringArray};
    use arrow_schema::Field;

    #[test]
    fn test_canonical_schema_validation_success() {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("name", DataType::Utf8, true),
            Field::new("ts", DataType::Timestamp(TimeUnit::Microsecond, None), true),
        ]);
        assert!(validate_canonical_schema(&schema).is_ok());
    }

    #[test]
    fn test_ipc_serialization_roundtrip_determinism() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("country", DataType::Utf8, true),
        ]));

        let id_array = Arc::new(Int32Array::from(vec![1, 2, 3, 4])) as ArrayRef;
        let country_array = Arc::new(StringArray::from(vec![
            Some("US"),
            None,
            Some("DE"),
            Some("JP"),
        ])) as ArrayRef;

        let batch = RecordBatch::try_new(schema.clone(), vec![id_array, country_array]).unwrap();
        let serialized = serialize_record_batches(&schema, &[batch.clone()]).unwrap();

        let (out_schema, out_batches) = deserialize_record_batches(&serialized).unwrap();
        assert_eq!(schema, out_schema);
        assert_eq!(out_batches.len(), 1);
        assert_eq!(out_batches[0].num_rows(), 4);
        assert_eq!(out_batches[0].num_columns(), 2);

        // Verify fingerprint determinism
        let hash1 = compute_batches_fingerprint(&[batch]).unwrap();
        let hash2 = compute_batches_fingerprint(&out_batches).unwrap();
        assert_eq!(hash1, hash2);
    }
}
