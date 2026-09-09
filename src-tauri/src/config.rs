//! Persisted configuration: the source folder, the destination set, and the knobs
//! behind the §7 safety rails.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// §7: hard cap on batch size in v1.
pub const MAX_BATCH: usize = 500;

/// A destination the user has added. `key` is the only thing the LLM ever sees or
/// returns (§4.2); the app owns the key → path mapping, which is what makes path
/// injection structurally impossible.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destination {
    pub key: String,
    pub label: String,
    pub path: PathBuf,
    /// Natural-language rule. Empty until the user writes one.
    #[serde(default)]
    pub brief: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    /// §7: installers stay put unless the user opts in.
    #[serde(default)]
    pub allow_installers: bool,
    /// §7: refuse to move a file another process holds open.
    #[serde(default = "default_true")]
    pub check_open_files: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            allow_installers: false,
            check_open_files: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub source: Option<PathBuf>,
    #[serde(default)]
    pub destinations: Vec<Destination>,
    #[serde(default)]
    pub settings: Settings,
}

impl Config {
    pub fn destination(&self, key: &str) -> Option<&Destination> {
        self.destinations.iter().find(|d| d.key == key)
    }

    pub fn destination_keys(&self) -> Vec<String> {
        self.destinations.iter().map(|d| d.key.clone()).collect()
    }

    /// Rejects a config the rest of the app would have to defend against anyway:
    /// duplicate or malformed keys, and destinations nested inside the source.
    pub fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for d in &self.destinations {
            if d.key.is_empty() {
                return Err(Error::config("a destination has an empty key"));
            }
            if !d
                .key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(Error::config(format!(
                    "destination key `{}` must be alphanumeric, `_` or `-`",
                    d.key
                )));
            }
            if !seen.insert(&d.key) {
                return Err(Error::config(format!(
                    "duplicate destination key `{}`",
                    d.key
                )));
            }
        }
        if let Some(src) = &self.source {
            for d in &self.destinations {
                if d.path == *src {
                    return Err(Error::config(format!(
                        "destination `{}` is the source folder",
                        d.label
                    )));
                }
            }
        }
        Ok(())
    }
}

/// `~/Library/Application Support/menlo`. macOS-only by §1.5, so this is a direct
/// construction rather than a cross-platform lookup.
pub fn app_dir() -> Result<PathBuf> {
    // Testing seam: integration tests must not write into the real Application
    // Support directory. Also lets a user point Menlo at a different profile.
    if let Ok(over) = std::env::var("MENLO_APP_DIR") {
        if !over.is_empty() {
            return Ok(PathBuf::from(over));
        }
    }
    let home = std::env::var("HOME")
        .map_err(|_| Error::config("HOME is not set; cannot locate Application Support"))?;
    Ok(PathBuf::from(home).join("Library/Application Support/menlo"))
}

pub fn journal_dir() -> Result<PathBuf> {
    Ok(app_dir()?.join("journal"))
}

fn config_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("config.json"))
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => Ok(serde_json::from_str(&s)?),
        // First launch is not an error.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(Error::io(&path, e)),
    }
}

pub fn save(config: &Config) -> Result<()> {
    config.validate()?;
    let path = config_path()?;
    write_atomic(&path, serde_json::to_string_pretty(config)?.as_bytes())
}

/// Write to a sibling temp file then rename, so a crash mid-write cannot leave a
/// truncated config or journal behind.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::config(format!("{} has no parent directory", path.display())))?;
    std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    let tmp = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&tmp, bytes).map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        Error::io(path, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dest(key: &str, path: &str) -> Destination {
        Destination {
            key: key.into(),
            label: key.into(),
            path: PathBuf::from(path),
            brief: String::new(),
        }
    }

    #[test]
    fn rejects_duplicate_keys() {
        let c = Config {
            source: None,
            destinations: vec![dest("finance", "/a"), dest("finance", "/b")],
            settings: Settings::default(),
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_path_shaped_keys() {
        // A key with a slash would let a crafted plan escape the key→path mapping.
        let c = Config {
            source: None,
            destinations: vec![dest("../etc", "/a")],
            settings: Settings::default(),
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_destination_equal_to_source() {
        let c = Config {
            source: Some(PathBuf::from("/tmp/dl")),
            destinations: vec![dest("dl", "/tmp/dl")],
            settings: Settings::default(),
        };
        assert!(c.validate().is_err());
    }

    #[test]
    fn accepts_a_sane_config() {
        let c = Config {
            source: Some(PathBuf::from("/tmp/dl")),
            destinations: vec![
                dest("finance", "/tmp/finance"),
                dest("media-2", "/tmp/media"),
            ],
            settings: Settings::default(),
        };
        assert!(c.validate().is_ok());
    }
}
