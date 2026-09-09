//! Test-only helpers. `tempfile` was not in the approved dependency set, so this is
//! the minimum viable stand-in: a unique directory under the system temp dir that
//! removes itself on drop.

#![cfg(test)]

use std::path::{Path, PathBuf};

pub struct TempDir {
    path: PathBuf,
}

impl Default for TempDir {
    fn default() -> Self {
        Self::new()
    }
}

impl TempDir {
    pub fn new() -> Self {
        let path = std::env::temp_dir().join(format!("menlo-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        // Canonicalise up front: on macOS /var is a symlink to /private/var, and every
        // path comparison in the safety layer runs against canonical paths.
        let path = path.canonicalize().expect("canonicalize temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, rel: impl AsRef<Path>) -> PathBuf {
        self.path.join(rel)
    }

    /// Create a file with the given contents, making parent directories as needed.
    pub fn write(&self, rel: impl AsRef<Path>, contents: &[u8]) -> PathBuf {
        let p = self.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        std::fs::write(&p, contents).expect("write file");
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
