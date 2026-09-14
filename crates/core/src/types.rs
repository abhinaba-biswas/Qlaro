use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetId(pub Uuid);

impl DatasetId {
    pub fn new_v4() -> Self {
        Self(Uuid::new_v4())
    }
}

impl std::fmt::Display for DatasetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatasetFingerprint(pub String); // Hex SHA-256

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub id: DatasetId,
    pub name: String,
    pub fingerprint: DatasetFingerprint,
    pub format: StorageFormat,
    pub row_count: u64,
    pub byte_size: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageFormat {
    Parquet,
    ArrowIpc,
    Csv,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CanonicalScalar {
    Null,
    Boolean(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Utf8(String),
    Date32(i32),
    TimestampMicros(i64, Option<String>),
}

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionSource {
    pub name: String,
    pub source_path: PathBuf,
    pub format_hint: Option<StorageFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionOptions {
    pub strict_schema: bool,
    pub max_bytes: Option<u64>,
    pub sample_limit: Option<usize>,
}

impl Default for IngestionOptions {
    fn default() -> Self {
        Self {
            strict_schema: true,
            max_bytes: None,
            sample_limit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionLineage {
    pub source_path: String,
    pub source_fingerprint: DatasetFingerprint,
    pub ingested_at: DateTime<Utc>,
    pub pipeline_version: String,
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

