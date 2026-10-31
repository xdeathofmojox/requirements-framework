use crate::{Document, Error};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Load explicit documents and recursively discover requirement JSON in directories.
/// Hidden entries and nested symlinks are skipped; overlapping inputs are deduplicated.
pub fn load_documents(inputs: &[PathBuf]) -> Result<Vec<Document>, Error> {
    let mut paths = BTreeSet::new();
    for input in inputs {
        if input.is_dir() {
            discover(input, &mut paths)?;
        } else {
            paths.insert(input.canonicalize()?);
        }
    }
    paths.iter().map(|path| Document::load(path)).collect()
}

fn discover(directory: &Path, paths: &mut BTreeSet<PathBuf>) -> Result<(), Error> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            discover(&path, paths)?;
        } else if kind.is_file() && is_requirement_file(&path)? {
            paths.insert(path.canonicalize()?);
        }
    }
    Ok(())
}

fn is_requirement_file(path: &Path) -> Result<bool, Error> {
    if path.extension().is_none_or(|extension| extension != "json")
        || path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().ends_with(".schema.json"))
    {
        return Ok(false);
    }
    let bytes = std::fs::read(path)?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| Error::Invalid(format!("{}: {error}", path.display())))?;
    Ok(value.get("requirements").is_some() || value.get("schemaVersion").is_some())
}
