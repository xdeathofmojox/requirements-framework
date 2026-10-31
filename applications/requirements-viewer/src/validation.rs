use crate::Error;
use serde_json::Value;
use std::{io::Write, path::PathBuf, process::Command};

/// An explicitly selected validator and schema, supplied by the Nix package.
pub struct Validation {
    pub executable: PathBuf,
    pub schema: Option<PathBuf>,
    pub schema_version: Option<String>,
}

impl Validation {
    /// Describe schema-declared fields for the browser form.
    pub fn describe(&self) -> Result<Value, Error> {
        let result = self.command().arg("--describe").output()?;
        if !result.status.success() {
            return Err(Error::Invalid(
                String::from_utf8_lossy(&result.stderr).into_owned(),
            ));
        }
        Ok(serde_json::from_slice(&result.stdout)?)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        if let Some(schema) = &self.schema {
            command.arg("--schema").arg(schema);
        }
        if let Some(version) = &self.schema_version {
            command.arg("--schema-version").arg(version);
        }
        command
    }

    /// Validate the candidate snapshot before it is displayed or saved.
    pub fn check(&self, value: &Value) -> Result<(), Error> {
        let mut snapshot = tempfile::NamedTempFile::new()?;
        serde_json::to_writer(&mut snapshot, value)?;
        snapshot.flush()?;
        let mut command = self.command();
        let result = command.arg(snapshot.path()).output()?;
        if !result.status.success() {
            return Err(Error::Invalid(
                String::from_utf8_lossy(&result.stderr)
                    .replace(&*snapshot.path().to_string_lossy(), "document"),
            ));
        }
        Ok(())
    }
}
