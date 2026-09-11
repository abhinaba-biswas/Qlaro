use core::error::QlaroError;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Computes a cryptographic SHA-256 fingerprint from a file on disk
pub fn compute_file_fingerprint<P: AsRef<Path>>(path: P) -> Result<String, QlaroError> {
    let path_ref = path.as_ref();
    let file = File::open(path_ref)
        .map_err(|e| QlaroError::Io(format!("Failed to open {}: {}", path_ref.display(), e)))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536];

    loop {
        let n = reader
            .read(&mut buffer)
            .map_err(|e| QlaroError::Io(format!("Error reading {}: {}", path_ref.display(), e)))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Computes deterministic dataset version identity combining source fingerprint and schema hash
pub fn compute_dataset_version_id(fingerprint: &str, schema_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fingerprint.as_bytes());
    hasher.update(schema_bytes);
    format!("{:x}", hasher.finalize())
}
