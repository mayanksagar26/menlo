//! Persisted configuration: the source folders, the destinations and their subfolders,
//! the saved folder sets, and the settings behind the §7 safety rails.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// §7: hard cap on batch size in v1.
pub const MAX_BATCH: usize = 500;

/// The home screen has room for this many pinned sets.
pub const PIN_LIMIT: usize = 9;

/// A destination the user has added. `key` is the only thing a classifier ever sees or
/// returns (§4.2); the app owns the key → path mapping, which is what makes path
/// injection structurally impossible.
///
/// A subfolder is a destination with a `parent`. Modelling it that way rather than as a
/// path suffix means every check that guards a destination — the allowlist, plan
/// validation, the key→path mapping — guards a subfolder too, with no new code path
/// for a plan to escape through.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Destination {
    pub key: String,
    pub label: String,
    pub path: PathBuf,
    /// Legacy single rule from the first scaffold. Superseded by `FolderRule`s in
    /// rules.json; kept so an old config still loads.
    #[serde(default)]
    pub brief: String,
    /// Key of the destination this is a subfolder of. One level only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

impl Destination {
    pub fn is_subfolder(&self) -> bool {
        self.parent.is_some()
    }
}

/// A saved pairing of places to clear with places things belong.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderSet {
    pub id: String,
    pub label: String,
    /// Drafted by Menlo rather than built by hand; the UI marks these ✦.
    #[serde(default)]
    pub ai: bool,
    pub sources: Vec<PathBuf>,
    /// Top-level destination keys. Subfolders come with their parent.
    #[serde(default)]
    pub destinations: Vec<String>,
    #[serde(default)]
    pub pinned: bool,
}

/// Who decides where a file goes once the deterministic tiers have had their turn.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkingModel {
    /// Rules only. No model, no network, no process spawned.
    #[default]
    FullyLocal,
    /// A local Gemma served by Ollama on localhost.
    Gemma,
    /// An agentic CLI the user already has — `claude`, `codex` — used as a pure
    /// classifier (docs/PROMPT_CONTRACT.md). Optional, never required.
    CliAgent,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Schedule {
    #[default]
    Manual,
    Daily,
    OnLogin,
}

/// What to do with a file that is already sitting in its destination, byte for byte.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicatePolicy {
    /// Stop and ask before the move.
    #[default]
    Ask,
    /// Leave the source copy where it is.
    Keep,
    /// Send the source copy to the Trash. Journalled, so Runs can bring it back.
    Trash,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    /// §7: installers stay put unless the user opts in.
    #[serde(default)]
    pub allow_installers: bool,
    /// §7: refuse to move a file another process holds open.
    #[serde(default = "default_true")]
    pub check_open_files: bool,

    #[serde(default)]
    pub profile_name: String,
    /// Which profile picture: a preset's id, or `custom` for an uploaded one
    /// (`avatar::CUSTOM`). The picture itself is never stored here.
    #[serde(default = "default_avatar")]
    pub avatar: String,
    #[serde(default)]
    pub working_model: WorkingModel,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub duplicates: DuplicatePolicy,
    /// Propose clearer names for files called things like `Screenshot 2026-…` or
    /// `IMG_4821`. Every proposal is shown in the move preview before anything happens.
    #[serde(default = "default_true")]
    pub rename_unclear: bool,
    #[serde(default = "default_true")]
    pub ask_before_moving: bool,
    #[serde(default)]
    pub include_hidden: bool,
    #[serde(default = "default_true")]
    pub notify_on_finish: bool,
    #[serde(default = "default_true")]
    pub keep_history: bool,
}

fn default_true() -> bool {
    true
}

fn default_avatar() -> String {
    crate::avatar::DEFAULT.to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            allow_installers: false,
            check_open_files: true,
            profile_name: String::new(),
            avatar: default_avatar(),
            working_model: WorkingModel::default(),
            schedule: Schedule::default(),
            duplicates: DuplicatePolicy::default(),
            rename_unclear: true,
            ask_before_moving: true,
            include_hidden: false,
            notify_on_finish: true,
            keep_history: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub sources: Vec<PathBuf>,
    #[serde(default)]
    pub destinations: Vec<Destination>,
    #[serde(default)]
    pub folder_sets: Vec<FolderSet>,
    #[serde(default)]
    pub settings: Settings,
    /// The first scaffold had exactly one source. Read so an old config migrates on
    /// load; never written back.
    #[serde(default, skip_serializing, rename = "source")]
    legacy_source: Option<PathBuf>,
}

impl Config {
    pub fn new(sources: Vec<PathBuf>, destinations: Vec<Destination>) -> Self {
        Config {
            sources,
            destinations,
            ..Config::default()
        }
    }

    pub fn destination(&self, key: &str) -> Option<&Destination> {
        self.destinations.iter().find(|d| d.key == key)
    }

    pub fn destination_keys(&self) -> Vec<String> {
        self.destinations.iter().map(|d| d.key.clone()).collect()
    }

    pub fn top_level(&self) -> impl Iterator<Item = &Destination> {
        self.destinations.iter().filter(|d| !d.is_subfolder())
    }

    pub fn subfolders_of<'a>(&'a self, key: &'a str) -> impl Iterator<Item = &'a Destination> {
        self.destinations
            .iter()
            .filter(move |d| d.parent.as_deref() == Some(key))
    }

    pub fn folder_set(&self, id: &str) -> Option<&FolderSet> {
        self.folder_sets.iter().find(|s| s.id == id)
    }

    /// This config with only the named top-level destinations and their subfolders —
    /// what a plan for one folder set routes into. Sources and settings are kept.
    ///
    /// Unknown keys are ignored rather than rejected: the caller is choosing what to
    /// plan for, and a key that no longer exists simply contributes nothing.
    pub fn restricted_to(&self, top_level_keys: &[String]) -> Config {
        let keep = |d: &Destination| {
            let root = d.parent.as_deref().unwrap_or(&d.key);
            top_level_keys.iter().any(|k| k == root)
        };
        Config {
            destinations: self
                .destinations
                .iter()
                .filter(|d| keep(d))
                .cloned()
                .collect(),
            ..self.clone()
        }
    }

    /// Fold the one-source shape of the first scaffold into `sources`.
    fn migrate(&mut self) {
        if let Some(src) = self.legacy_source.take() {
            if !self.sources.contains(&src) {
                self.sources.insert(0, src);
            }
        }
    }

    /// A destination key for a new folder: the label slugged, with its parent's key in
    /// front for a subfolder, and a numeric suffix if that is taken.
    pub fn fresh_key(&self, label: &str, parent: Option<&str>) -> String {
        let slug = slug(label);
        let slug = if slug.is_empty() {
            "folder".to_string()
        } else {
            slug
        };
        let base = match parent {
            Some(p) => format!("{p}-{slug}"),
            None => slug,
        };
        if self.destination(&base).is_none() {
            return base;
        }
        (2..)
            .map(|n| format!("{base}-{n}"))
            .find(|k| self.destination(k).is_none())
            .expect("an unbounded range always yields a free key")
    }

    /// Rejects a config the rest of the app would have to defend against anyway.
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

        for d in &self.destinations {
            let Some(parent_key) = &d.parent else {
                continue;
            };
            let Some(parent) = self.destination(parent_key) else {
                return Err(Error::config(format!(
                    "`{}` is a subfolder of `{parent_key}`, which does not exist",
                    d.label
                )));
            };
            // One level. The UI drills one folder deep, and so does routing.
            if parent.is_subfolder() {
                return Err(Error::config(format!(
                    "`{}` is nested two levels deep; subfolders go one level only",
                    d.label
                )));
            }
            if !d.path.starts_with(&parent.path) || d.path == parent.path {
                return Err(Error::config(format!(
                    "`{}` is not inside {}",
                    d.label,
                    parent.path.display()
                )));
            }
        }

        for src in &self.sources {
            if let Some(d) = self.destinations.iter().find(|d| d.path == *src) {
                return Err(Error::config(format!(
                    "`{}` is both a source and a destination",
                    d.label
                )));
            }
        }

        for set in &self.folder_sets {
            for key in &set.destinations {
                if self.destination(key).is_none() {
                    return Err(Error::config(format!(
                        "folder set `{}` refers to a destination that no longer exists",
                        set.label
                    )));
                }
            }
        }
        if self.folder_sets.iter().filter(|s| s.pinned).count() > PIN_LIMIT {
            return Err(Error::config(format!(
                "at most {PIN_LIMIT} folder sets can be pinned to home"
            )));
        }

        Ok(())
    }
}

/// `Tax Documents` → `tax-documents`. Keys must survive `validate`, which accepts
/// `[A-Za-z0-9_-]` only, so everything else collapses to a hyphen.
pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in input.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').chars().take(40).collect()
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
    Ok(home()?.join("Library/Application Support/menlo"))
}

pub fn home() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| Error::config("HOME is not set"))
}

pub fn journal_dir() -> Result<PathBuf> {
    Ok(app_dir()?.join("journal"))
}

/// The user's Trash. `MENLO_TRASH_DIR` overrides it, for the same reason
/// `MENLO_APP_DIR` exists: tests must never put anything in the real one.
pub fn trash_dir() -> Result<PathBuf> {
    if let Ok(over) = std::env::var("MENLO_TRASH_DIR") {
        if !over.is_empty() {
            return Ok(PathBuf::from(over));
        }
    }
    Ok(home()?.join(".Trash"))
}

fn config_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("config.json"))
}

/// Whether Menlo has ever saved a config. First launch is when defaults get seeded.
pub fn exists() -> Result<bool> {
    Ok(config_path()?.exists())
}

pub fn load() -> Result<Config> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(s) => {
            let mut config: Config = serde_json::from_str(&s)?;
            config.migrate();
            Ok(config)
        }
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

/// What a fresh Menlo starts with: the usual places things pile up as sources, and
/// the standard home folders as destinations — only those that actually exist.
///
/// Destination keys deliberately equal the built-in extension categories in rules.rs,
/// so a fresh install files PDFs into Documents and photos into Pictures with no
/// setup and no model. Each destination's existing subfolders come along too; they
/// stay inert until something names them, so nothing is routed into a subfolder the
/// user has not been shown.
pub fn first_run_defaults(home: &Path) -> Config {
    let sources: Vec<PathBuf> = ["Downloads", "Desktop"]
        .iter()
        .map(|n| home.join(n))
        .filter(|p| p.is_dir())
        .collect();

    let mut config = Config::new(sources.clone(), Vec::new());

    for (parent_key, label) in [
        ("documents", "Documents"),
        ("images", "Pictures"),
        ("video", "Movies"),
        ("audio", "Music"),
        ("code", "Developer"),
    ] {
        let path = home.join(label);
        if !path.is_dir() {
            continue;
        }
        config.destinations.push(Destination {
            key: parent_key.to_string(),
            label: label.to_string(),
            path: path.clone(),
            brief: String::new(),
            parent: None,
        });
        for sub in existing_subfolders(&path, 12) {
            let name = sub
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let child_key = config.fresh_key(&name, Some(parent_key));
            config.destinations.push(Destination {
                key: child_key,
                label: name,
                path: sub,
                brief: String::new(),
                parent: Some(parent_key.to_string()),
            });
        }
    }

    let top: Vec<String> = config.top_level().map(|d| d.key.clone()).collect();
    if !sources.is_empty() && !top.is_empty() {
        config.folder_sets.push(FolderSet {
            id: "everyday".into(),
            label: "Everyday".into(),
            ai: false,
            sources: sources.clone(),
            destinations: top.clone(),
            pinned: true,
        });
        if let Some(downloads) = sources.iter().find(|p| p.ends_with("Downloads")) {
            config.folder_sets.push(FolderSet {
                id: "downloads".into(),
                label: "Downloads only".into(),
                ai: false,
                sources: vec![downloads.clone()],
                destinations: top,
                pinned: true,
            });
        }
    }

    config
}

/// Visible, real subfolders of `dir`, alphabetically, at most `limit`.
///
/// Skips hidden folders and packages. A package — `Photos Library.photoslibrary`,
/// `Notes.rtfd`, `App.xcodeproj` — is a directory on disk but a single document to
/// the user, and filing loose files into one would corrupt it. `Photos Library` lives
/// in `~/Pictures`, so this is not hypothetical.
pub fn existing_subfolders(dir: &Path, limit: usize) -> Vec<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = read
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            !name.starts_with('.') && !is_package(p) && crate::safety::assert_operable(p).is_ok()
        })
        .collect();
    out.sort();
    out.truncate(limit);
    out
}

/// A directory Finder presents as one document. Real folder names rarely carry a
/// purely alphanumeric extension (`v1.2 notes` has the "extension" `2 notes`), while
/// every package does, so this errs toward leaving an oddly named folder out.
fn is_package(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy();
            !e.is_empty() && e.chars().all(|c| c.is_ascii_alphanumeric())
        })
        .unwrap_or(false)
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
    use crate::testutil::TempDir;

    fn dest(key: &str, path: &str) -> Destination {
        Destination {
            key: key.into(),
            label: key.into(),
            path: PathBuf::from(path),
            brief: String::new(),
            parent: None,
        }
    }

    fn sub(key: &str, path: &str, parent: &str) -> Destination {
        Destination {
            parent: Some(parent.into()),
            ..dest(key, path)
        }
    }

    #[test]
    fn rejects_duplicate_keys() {
        let c = Config::new(vec![], vec![dest("finance", "/a"), dest("finance", "/b")]);
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_path_shaped_keys() {
        // A key with a slash would let a crafted plan escape the key→path mapping.
        let c = Config::new(vec![], vec![dest("../etc", "/a")]);
        assert!(c.validate().is_err());
    }

    #[test]
    fn rejects_a_destination_that_is_also_a_source() {
        let c = Config::new(
            vec![PathBuf::from("/tmp/other"), PathBuf::from("/tmp/dl")],
            vec![dest("dl", "/tmp/dl")],
        );
        assert!(c.validate().is_err());
    }

    #[test]
    fn accepts_a_sane_config() {
        let c = Config::new(
            vec![PathBuf::from("/tmp/dl"), PathBuf::from("/tmp/desk")],
            vec![
                dest("finance", "/tmp/finance"),
                dest("media-2", "/tmp/media"),
                sub("finance-tax", "/tmp/finance/Tax", "finance"),
            ],
        );
        assert!(c.validate().is_ok());
    }

    #[test]
    fn a_subfolder_must_sit_inside_its_parent() {
        let c = Config::new(
            vec![],
            vec![
                dest("docs", "/tmp/docs"),
                sub("docs-x", "/tmp/elsewhere/x", "docs"),
            ],
        );
        assert!(c.validate().is_err());
    }

    #[test]
    fn a_subfolder_needs_a_parent_that_exists() {
        let c = Config::new(vec![], vec![sub("docs-x", "/tmp/docs/x", "docs")]);
        assert!(c.validate().is_err());
    }

    #[test]
    fn subfolders_go_one_level_deep() {
        let c = Config::new(
            vec![],
            vec![
                dest("docs", "/tmp/docs"),
                sub("docs-a", "/tmp/docs/a", "docs"),
                sub("docs-a-b", "/tmp/docs/a/b", "docs-a"),
            ],
        );
        assert!(c.validate().is_err());
    }

    #[test]
    fn migrates_the_single_source_shape() {
        let old = r#"{"source":"/tmp/dl","destinations":[],"settings":{}}"#;
        let mut c: Config = serde_json::from_str(old).unwrap();
        c.migrate();
        assert_eq!(c.sources, vec![PathBuf::from("/tmp/dl")]);
        // ...and the legacy field is not written back out.
        assert!(!serde_json::to_string(&c).unwrap().contains("\"source\""));
    }

    #[test]
    fn settings_default_to_the_safe_choice_on_an_old_config() {
        let c: Config = serde_json::from_str(r#"{"settings":{}}"#).unwrap();
        assert!(c.settings.check_open_files);
        assert!(!c.settings.allow_installers);
        assert_eq!(c.settings.duplicates, DuplicatePolicy::Ask);
        assert_eq!(c.settings.working_model, WorkingModel::FullyLocal);
    }

    #[test]
    fn fresh_keys_are_valid_unique_and_carry_the_parent() {
        let mut c = Config::new(vec![], vec![dest("documents", "/tmp/d")]);
        assert_eq!(
            c.fresh_key("Tax 2026", Some("documents")),
            "documents-tax-2026"
        );
        c.destinations
            .push(sub("documents-tax-2026", "/tmp/d/t", "documents"));
        assert_eq!(
            c.fresh_key("Tax 2026", Some("documents")),
            "documents-tax-2026-2"
        );
        assert_eq!(c.fresh_key("!!!", None), "folder");
        assert_eq!(slug("Fotografías & Co."), "fotograf-as-co");
    }

    #[test]
    fn first_run_seeds_only_folders_that_exist() {
        let home = TempDir::new();
        std::fs::create_dir_all(home.join("Downloads")).unwrap();
        std::fs::create_dir_all(home.join("Documents/Finance")).unwrap();
        std::fs::create_dir_all(home.join("Documents/.hidden")).unwrap();
        std::fs::create_dir_all(home.join("Documents/v1.2 notes")).unwrap();
        std::fs::create_dir_all(home.join("Pictures/Photos Library.photoslibrary")).unwrap();

        let c = first_run_defaults(home.path());
        assert_eq!(c.sources, vec![home.join("Downloads")]);

        let keys = c.destination_keys();
        assert!(keys.contains(&"documents".to_string()));
        assert!(keys.contains(&"images".to_string()));
        assert!(
            !keys.contains(&"video".to_string()),
            "no ~/Movies, so no video"
        );
        assert!(keys.contains(&"documents-finance".to_string()));
        assert!(!keys.iter().any(|k| k.contains("hidden")));
        // A package is one document, never a place to file into.
        assert!(!keys.iter().any(|k| k.contains("photos")));
        // ...but a folder that merely has a dot in its name is still a folder.
        assert!(keys.contains(&"documents-v1-2-notes".to_string()));

        // Each subfolder names its real parent, not itself.
        let finance = c.destination("documents-finance").unwrap();
        assert_eq!(finance.parent.as_deref(), Some("documents"));

        assert!(c.validate().is_ok());
        assert_eq!(c.folder_sets[0].label, "Everyday");
    }
}
