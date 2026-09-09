//! Hand-rolled error type. We deliberately avoid `thiserror`/`anyhow` so the Rust core
//! keeps the dependency surface listed in the build plan's §2.

use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    /// Filesystem call failed. Carries the path so messages are actionable.
    Io {
        path: String,
        source: std::io::Error,
    },
    /// Bare I/O with no meaningful path context (e.g. a pipe).
    RawIo(std::io::Error),
    /// Serde could not (de)serialize.
    Serde(serde_json::Error),
    /// A plan failed validation. `§4.3` — the whole batch is rejected.
    Validation(Vec<String>),
    /// A §7 safety rail refused the operation.
    Safety(String),
    /// Configuration is missing or incoherent.
    Config(String),
    /// A file changed underneath us between planning and execution.
    Integrity(String),
}

impl Error {
    pub fn io(path: impl AsRef<Path>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.as_ref().display().to_string(),
            source,
        }
    }

    pub fn safety(msg: impl Into<String>) -> Self {
        Error::Safety(msg.into())
    }

    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    pub fn integrity(msg: impl Into<String>) -> Self {
        Error::Integrity(msg.into())
    }

    /// Stable machine-readable tag, so the UI can branch without parsing prose.
    pub fn kind(&self) -> &'static str {
        match self {
            Error::Io { .. } | Error::RawIo(_) => "io",
            Error::Serde(_) => "serde",
            Error::Validation(_) => "validation",
            Error::Safety(_) => "safety",
            Error::Config(_) => "config",
            Error::Integrity(_) => "integrity",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io { path, source } => write!(f, "{path}: {source}"),
            Error::RawIo(e) => write!(f, "{e}"),
            Error::Serde(e) => write!(f, "malformed JSON: {e}"),
            Error::Validation(reasons) => {
                write!(
                    f,
                    "plan rejected ({} problem(s)): {}",
                    reasons.len(),
                    reasons.join("; ")
                )
            }
            Error::Safety(m) => write!(f, "refused: {m}"),
            Error::Config(m) => write!(f, "configuration: {m}"),
            Error::Integrity(m) => write!(f, "integrity check failed: {m}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::RawIo(e) => Some(e),
            Error::Serde(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::RawIo(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serde(e)
    }
}

/// Tauri commands must return something `serde`-able; this is the wire shape.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("MenloError", 3)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        let details: Vec<String> = match self {
            Error::Validation(reasons) => reasons.clone(),
            _ => Vec::new(),
        };
        st.serialize_field("details", &details)?;
        st.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
