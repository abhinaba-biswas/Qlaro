use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QlaroErrorCode {
    SchemaMismatch(String),
    UnsupportedDataType(String),
    InvalidFormat(String),
    CorruptedBatch(String),
    DatasetNotFound(String),
    UnentitledCapability(String),
    Internal(String),
}

impl std::fmt::Display for QlaroErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaMismatch(s) => write!(f, "Schema mismatch: {}", s),
            Self::UnsupportedDataType(s) => write!(f, "Unsupported Arrow data type: {}", s),
            Self::InvalidFormat(s) => write!(f, "Invalid dataset format: {}", s),
            Self::CorruptedBatch(s) => write!(f, "Corrupted record batch: {}", s),
            Self::DatasetNotFound(s) => write!(f, "Dataset not found: {}", s),
            Self::UnentitledCapability(s) => write!(f, "Unentitled capability: {}", s),
            Self::Internal(s) => write!(f, "Internal engine error: {}", s),
        }
    }
}

impl std::error::Error for QlaroErrorCode {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticDetail {
    pub code: QlaroErrorCode,
    pub message: String,
    pub suggested_action: Option<String>,
}

#[derive(Debug)]
pub enum QlaroError {
    Core(QlaroErrorCode),
    Arrow(String),
    Io(String),
    Serialization(String),
}

impl std::fmt::Display for QlaroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Core(code) => write!(f, "Core Error: {}", code),
            Self::Arrow(err) => write!(f, "Arrow Error: {}", err),
            Self::Io(err) => write!(f, "IO Error: {}", err),
            Self::Serialization(err) => write!(f, "Serialization Error: {}", err),
        }
    }
}

impl std::error::Error for QlaroError {}

impl From<QlaroErrorCode> for QlaroError {
    fn from(code: QlaroErrorCode) -> Self {
        Self::Core(code)
    }
}

impl From<arrow_schema::ArrowError> for QlaroError {
    fn from(err: arrow_schema::ArrowError) -> Self {
        Self::Arrow(err.to_string())
    }
}

