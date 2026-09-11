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
