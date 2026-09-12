use core::error::{QlaroError, QlaroErrorCode};
use core::types::{DatasetFingerprint, DatasetId, DatasetMetadata, StorageFormat};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSchemaVersion {
    pub version_id: String,
    pub dataset_id: DatasetId,
    pub schema_json: String,
    pub created_at: DateTime<Utc>,
    pub parent_version_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub metadata: DatasetMetadata,
    pub source_path: PathBuf,
    pub current_version_id: String,
    pub versions: Vec<DatasetSchemaVersion>,
}

pub trait CatalogStore: Send + Sync {
    fn register_dataset(
        &self,
        name: &str,
        path: &Path,
        format: StorageFormat,
        schema_json: &str,
        fingerprint: &str,
        row_count: u64,
        byte_size: u64,
    ) -> Result<DatasetRecord, QlaroError>;

    fn get_dataset(&self, id: &DatasetId) -> Result<DatasetRecord, QlaroError>;
    fn get_dataset_by_name(&self, name: &str) -> Result<DatasetRecord, QlaroError>;
    fn list_datasets(&self) -> Result<Vec<DatasetRecord>, QlaroError>;
    fn remove_dataset(&self, id: &DatasetId) -> Result<DatasetRecord, QlaroError>;
}

pub struct InMemoryCatalogStore {
    datasets: Arc<RwLock<HashMap<DatasetId, DatasetRecord>>>,
    name_index: Arc<RwLock<HashMap<String, DatasetId>>>,
}

impl InMemoryCatalogStore {
    pub fn new() -> Self {
        Self {
            datasets: Arc::new(RwLock::new(HashMap::new())),
            name_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCatalogStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CatalogStore for InMemoryCatalogStore {
    fn register_dataset(
        &self,
        name: &str,
        path: &Path,
        format: StorageFormat,
        schema_json: &str,
        fingerprint: &str,
        row_count: u64,
        byte_size: u64,
    ) -> Result<DatasetRecord, QlaroError> {
        let mut names = self.name_index.write().unwrap();
        let mut datasets = self.datasets.write().unwrap();

        let dataset_id = if let Some(existing_id) = names.get(name) {
            *existing_id
        } else {
            let id = DatasetId::new_v4();
            names.insert(name.to_string(), id);
            id
        };

        let now = Utc::now();
        let version_id = crate::identity::compute_dataset_version_id(fingerprint, schema_json.as_bytes());

        let new_version = DatasetSchemaVersion {
            version_id: version_id.clone(),
            dataset_id,
            schema_json: schema_json.to_string(),
            created_at: now,
            parent_version_id: None,
        };

        let metadata = DatasetMetadata {
            id: dataset_id,
            name: name.to_string(),
            fingerprint: DatasetFingerprint(fingerprint.to_string()),
            format,
            row_count,
            byte_size,
            created_at: now,
            updated_at: now,
        };

        let record = if let Some(existing_record) = datasets.get_mut(&dataset_id) {
            existing_record.metadata = metadata;
            existing_record.source_path = path.to_path_buf();
            existing_record.current_version_id = version_id;
            existing_record.versions.push(new_version);
            existing_record.clone()
        } else {
            let rec = DatasetRecord {
                metadata,
                source_path: path.to_path_buf(),
                current_version_id: version_id,
                versions: vec![new_version],
            };
            datasets.insert(dataset_id, rec.clone());
            rec
        };

        Ok(record)
    }

    fn get_dataset(&self, id: &DatasetId) -> Result<DatasetRecord, QlaroError> {
        let datasets = self.datasets.read().unwrap();
        datasets
            .get(id)
            .cloned()
            .ok_or_else(|| QlaroError::Core(QlaroErrorCode::DatasetNotFound(id.to_string())))
    }

    fn get_dataset_by_name(&self, name: &str) -> Result<DatasetRecord, QlaroError> {
        let names = self.name_index.read().unwrap();
        let id = names
            .get(name)
            .ok_or_else(|| QlaroError::Core(QlaroErrorCode::DatasetNotFound(name.to_string())))?;
        self.get_dataset(id)
    }

    fn list_datasets(&self) -> Result<Vec<DatasetRecord>, QlaroError> {
        let datasets = self.datasets.read().unwrap();
        Ok(datasets.values().cloned().collect())
    }

    fn remove_dataset(&self, id: &DatasetId) -> Result<DatasetRecord, QlaroError> {
        let mut datasets = self.datasets.write().unwrap();
        let mut names = self.name_index.write().unwrap();

        let record = datasets
            .remove(id)
            .ok_or_else(|| QlaroError::Core(QlaroErrorCode::DatasetNotFound(id.to_string())))?;
        names.remove(&record.metadata.name);
        Ok(record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_store_lifecycle() {
        let store = InMemoryCatalogStore::new();
        let path = PathBuf::from("/tmp/synthetic_sales.parquet");

        let rec = store
            .register_dataset(
                "sales",
                &path,
                StorageFormat::Parquet,
                r#"{"fields":[]}"#,
                "abc123sha256",
                1000,
                50000,
            )
            .unwrap();

        assert_eq!(rec.metadata.name, "sales");
        assert_eq!(rec.versions.len(), 1);

        let retrieved = store.get_dataset_by_name("sales").unwrap();
        assert_eq!(retrieved.metadata.id, rec.metadata.id);

        let list = store.list_datasets().unwrap();
        assert_eq!(list.len(), 1);

        let removed = store.remove_dataset(&rec.metadata.id).unwrap();
        assert_eq!(removed.metadata.name, "sales");
        assert!(store.get_dataset(&rec.metadata.id).is_err());
    }
}
