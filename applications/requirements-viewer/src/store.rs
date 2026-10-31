use crate::{Document, Error, Validation, build_graph, load_documents};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Files explicitly opened in this session and the validation policy for writes.
pub struct Store {
    inputs: Vec<PathBuf>,
    folders: Vec<PathBuf>,
    documents: Vec<Document>,
    validation: Validation,
    form_schema: Value,
    editable: bool,
    revision: u64,
}

impl Store {
    /// Validate all files before creating a session.
    pub fn new(
        documents: Vec<Document>,
        validation: Validation,
        editable: bool,
    ) -> Result<Self, Error> {
        for document in &documents {
            validation.check(&document.value)?;
        }
        build_graph(&documents)?;
        let form_schema = validation.describe()?;
        let inputs = documents
            .iter()
            .map(|document| document.path.clone())
            .collect();
        Ok(Self {
            inputs,
            folders: Vec::new(),
            documents,
            validation,
            form_schema,
            editable,
            revision: 0,
        })
    }

    /// Open files or directory roots, retaining the inputs for explicit reloads.
    pub fn open(
        inputs: Vec<PathBuf>,
        validation: Validation,
        editable: bool,
    ) -> Result<Self, Error> {
        let inputs = inputs
            .iter()
            .map(|path| path.canonicalize())
            .collect::<Result<Vec<_>, _>>()?;
        let mut store = Self::new(load_documents(&inputs)?, validation, editable)?;
        store.folders = inputs
            .iter()
            .filter(|path| path.is_dir())
            .cloned()
            .collect();
        store.inputs = inputs;
        Ok(store)
    }

    /// Rescan and validate before replacing the snapshot. Failed reloads leave it intact.
    pub fn reload(&mut self) -> Result<Value, Error> {
        let documents = load_documents(&self.inputs)?;
        for document in &documents {
            self.validation
                .check(&document.value)
                .map_err(|error| Error::Invalid(format!("{}: {error}", document.path.display())))?;
        }
        build_graph(&documents)?;
        let form_schema = self.validation.describe()?;
        let changed = form_schema != self.form_schema
            || documents.len() != self.documents.len()
            || documents
                .iter()
                .zip(&self.documents)
                .any(|(new, old)| new.path != old.path || new.original != old.original);
        if changed {
            self.form_schema = form_schema;
            self.documents = documents;
            self.revision += 1;
        }
        self.snapshot()
    }

    /// Return the graph and the file targets available to the editor.
    pub fn snapshot(&self) -> Result<Value, Error> {
        let mut graph = build_graph(&self.documents)?;
        graph["documents"] = json!(
            self.documents
                .iter()
                .enumerate()
                .map(|(index, doc)| json!({"index":index,"path":doc.path}))
                .collect::<Vec<_>>()
        );
        graph["folders"] = json!(
            self.folders
                .iter()
                .enumerate()
                .map(|(index, path)| json!({"index": index, "path": path}))
                .collect::<Vec<_>>()
        );
        graph["formSchema"] = self.form_schema.clone();
        graph["editable"] = json!(self.editable);
        graph["revision"] = json!(self.revision);
        Ok(graph)
    }

    /// Append to an opened file after schema, uniqueness, and conflict checks.
    pub fn add(
        &mut self,
        target: usize,
        revision: u64,
        requirement: Value,
    ) -> Result<Value, Error> {
        if !self.editable {
            return Err(Error::Invalid(
                "Restart with --edit to enable additions".into(),
            ));
        }
        if revision != self.revision {
            return Err(Error::Conflict("Session revision is stale".into()));
        }
        let id = requirement
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Invalid("Requirement needs an ID".into()))?;
        for document in &self.documents {
            if document.value["requirements"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["id"].as_str() == Some(id)))
            {
                return Err(Error::Invalid(format!(
                    "ID {id} already exists in this session"
                )));
            }
        }
        let document = self
            .documents
            .get(target)
            .ok_or_else(|| Error::Invalid("Unknown target document".into()))?;
        self.check_files_unchanged()?;
        let mut candidate = document.value.clone();
        candidate["requirements"]
            .as_array_mut()
            .ok_or_else(|| Error::Invalid("Missing requirements array".into()))?
            .push(requirement);
        self.commit(target, candidate)
    }

    /// Replace the selected record while retaining its stable ID and document position.
    pub fn update(&mut self, key: &str, revision: u64, requirement: Value) -> Result<Value, Error> {
        if !self.editable {
            return Err(Error::Invalid(
                "Restart with --edit to enable editing".into(),
            ));
        }
        if revision != self.revision {
            return Err(Error::Conflict("Session revision is stale".into()));
        }
        let (target, record) = self.locate_record(key)?;
        let mut candidate = self.documents[target].value.clone();
        if requirement.get("id") != candidate["requirements"][record].get("id") {
            return Err(Error::Invalid(
                "Requirement IDs are stable; editing cannot rename an ID".into(),
            ));
        }
        candidate["requirements"][record] = requirement;
        self.commit(target, candidate)
    }

    /// Delete a confirmed record without leaving incoming parent references dangling.
    pub fn delete(&mut self, key: &str, revision: u64, confirm_id: &str) -> Result<Value, Error> {
        if !self.editable {
            return Err(Error::Invalid(
                "Restart with --edit to enable deletion".into(),
            ));
        }
        if revision != self.revision {
            return Err(Error::Conflict("Session revision is stale".into()));
        }
        let (target, record) = self.locate_record(key)?;
        let id = self.documents[target].value["requirements"][record]["id"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Requirement has no ID".into()))?;
        if confirm_id != id {
            return Err(Error::Invalid(
                "Deletion confirmation does not match this requirement".into(),
            ));
        }
        let mut dependents = Vec::new();
        for (document_index, document) in self.documents.iter().enumerate() {
            if let Some(items) = document.value["requirements"].as_array() {
                for (record_index, item) in items.iter().enumerate() {
                    if (document_index, record_index) != (target, record)
                        && item["parents"].as_array().is_some_and(|parents| {
                            parents.iter().any(|parent| parent.as_str() == Some(id))
                        })
                    {
                        dependents.push(item["id"].as_str().unwrap_or("?").to_owned());
                    }
                }
            }
        }
        if !dependents.is_empty() {
            return Err(Error::Invalid(format!(
                "Remove parent references from these requirements before deleting {id}: {}",
                dependents.join(", ")
            )));
        }
        let mut candidate = self.documents[target].value.clone();
        candidate["requirements"]
            .as_array_mut()
            .ok_or_else(|| Error::Invalid("Missing requirements array".into()))?
            .remove(record);
        if candidate["requirements"]
            .as_array()
            .is_some_and(Vec::is_empty)
        {
            self.check_files_unchanged()?;
            let path = self.documents[target].path.clone();
            std::fs::remove_file(&path)?;
            self.documents.remove(target);
            // An explicitly opened file must no longer be required by future reloads.
            self.inputs.retain(|input| input != &path);
            self.revision += 1;
            return self.snapshot();
        }
        self.commit(target, candidate)
    }

    fn locate_record(&self, key: &str) -> Result<(usize, usize), Error> {
        let index = key
            .parse::<usize>()
            .map_err(|_| Error::Invalid("Unknown requirement".into()))?;
        self.documents
            .iter()
            .enumerate()
            .flat_map(|(target, document)| {
                (0..document.value["requirements"]
                    .as_array()
                    .map_or(0, Vec::len))
                    .map(move |record| (target, record))
            })
            .nth(index)
            .ok_or_else(|| Error::Invalid("Unknown requirement".into()))
    }

    fn commit(&mut self, target: usize, candidate: Value) -> Result<Value, Error> {
        self.check_files_unchanged()?;
        let document = &self.documents[target];
        self.validation.check(&candidate)?;
        let mut bytes = serde_json::to_vec_pretty(&candidate)?;
        bytes.push(b'\n');
        let parent = document
            .path
            .parent()
            .ok_or_else(|| Error::Invalid("File has no parent directory".into()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
        temporary
            .as_file()
            .set_permissions(std::fs::metadata(&document.path)?.permissions())?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        self.check_files_unchanged()?;
        temporary
            .persist(&document.path)
            .map_err(|error| Error::Io(error.error))?;
        self.documents[target].value = candidate;
        self.documents[target].original = bytes;
        self.revision += 1;
        self.snapshot()
    }

    /// Create a new document in a supplied folder without overwriting an existing file.
    pub fn create(
        &mut self,
        folder: usize,
        filename: &str,
        revision: u64,
        mut document: Value,
        requirement: Value,
    ) -> Result<Value, Error> {
        if !self.editable {
            return Err(Error::Invalid(
                "Restart with --edit to enable additions".into(),
            ));
        }
        if revision != self.revision {
            return Err(Error::Conflict("Session revision is stale".into()));
        }
        validate_document_path(filename)?;
        let root = self
            .folders
            .get(folder)
            .ok_or_else(|| Error::Invalid("Unknown target folder".into()))?;
        let path = root.join(filename);
        if std::fs::symlink_metadata(&path).is_ok() {
            return Err(Error::Invalid(
                "File already exists; reload and select it instead".into(),
            ));
        }
        let id = requirement
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Invalid("Requirement needs an ID".into()))?;
        if self.documents.iter().any(|doc| {
            doc.value["requirements"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item["id"].as_str() == Some(id)))
        }) {
            return Err(Error::Invalid(format!(
                "ID {id} already exists in this session"
            )));
        }
        let object = document
            .as_object_mut()
            .ok_or_else(|| Error::Invalid("Document fields must be a JSON object".into()))?;
        if object.contains_key("requirements") {
            return Err(Error::Invalid(
                "Document fields cannot include requirements".into(),
            ));
        }
        object.insert("requirements".into(), json!([requirement]));
        self.validation.check(&document)?;
        let mut bytes = serde_json::to_vec_pretty(&document)?;
        bytes.push(b'\n');
        self.check_files_unchanged()?;
        let parent = prepare_document_parent(root, filename)?;
        let mut temporary = tempfile::NamedTempFile::new_in(&parent)?;
        temporary.write_all(&bytes)?;
        temporary.as_file().sync_all()?;
        self.check_files_unchanged()?;
        temporary
            .persist_noclobber(&path)
            .map_err(|error| Error::Io(error.error))?;
        self.documents.push(Document {
            path,
            value: document,
            original: bytes,
        });
        self.revision += 1;
        self.snapshot()
    }

    fn check_files_unchanged(&self) -> Result<(), Error> {
        for document in &self.documents {
            if std::fs::read(&document.path)? != document.original {
                return Err(Error::Conflict(document.path.display().to_string()));
            }
        }
        Ok(())
    }
}

/// Keep paths portable and consistent with recursive discovery (no hidden entries).
fn validate_document_path(filename: &str) -> Result<(), Error> {
    if filename.contains(['\\', ':', '\0'])
        || filename
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
        || !filename.ends_with(".json")
        || filename.ends_with(".schema.json")
    {
        return Err(Error::Invalid(
            "Choose a relative .json path inside the selected root (no hidden folders, .., or absolute paths)".into(),
        ));
    }
    Ok(())
}

/// Check every directory rather than following symlinks through create_dir_all.
fn prepare_document_parent(root: &Path, filename: &str) -> Result<PathBuf, Error> {
    let mut parent = root.to_path_buf();
    require_real_directory(&parent)?;
    for part in filename.split('/').take(filename.split('/').count() - 1) {
        parent.push(part);
        match std::fs::create_dir(&parent) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        require_real_directory(&parent)?;
    }
    Ok(parent)
}

fn require_real_directory(path: &Path) -> Result<(), Error> {
    if !std::fs::symlink_metadata(path)?.file_type().is_dir() || path.canonicalize()? != path {
        return Err(Error::Invalid(format!(
            "Target folder must be a directory, not a symlink: {}",
            path.display()
        )));
    }
    Ok(())
}
