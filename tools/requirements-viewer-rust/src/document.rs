use crate::Error;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// A loaded file, its JSON data, and the bytes used for conflict detection.
pub struct Document {
    pub(crate) path: PathBuf,
    pub(crate) value: Value,
    pub(crate) original: Vec<u8>,
}

impl Document {
    /// Load an existing JSON file, resolving its destination once at startup.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let path = path.canonicalize()?;
        let original = std::fs::read(&path)?;
        let value = serde_json::from_slice(&original)?;
        Ok(Self {
            path,
            value,
            original,
        })
    }
}
