//! What the window reads: the shapes the IPC layer hands the frontend.
//!
//! Everything here is assembled from the persisted state on request, so the frontend
//! holds a projection it can throw away, never a second copy of the truth.

use crate::config::{self, Config, Destination, FolderSet, Settings};
use crate::error::Result;
use crate::journal::{self, Batch, BatchSummary, MoveRecord, Strategy};
use crate::prose;
use crate::rules::{self, RuleSet};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct SourceView {
    pub path: PathBuf,
    pub label: String,
    /// Files a run would consider right now. Capped at the batch limit.
    pub count: usize,
    /// False when the folder has been deleted or its drive unplugged since it was added.
    pub exists: bool,
}

#[derive(Debug, Serialize)]
pub struct FolderRuleView {
    pub id: String,
    pub destination_key: String,
    pub text: String,
    /// Whether Menlo can act on this without a model.
    pub understood: bool,
    /// What Menlo took from the sentence, in words.
    pub reading: String,
}

#[derive(Debug, Serialize)]
pub struct AppView {
    pub home: PathBuf,
    pub sources: Vec<SourceView>,
    pub destinations: Vec<Destination>,
    pub folder_sets: Vec<FolderSet>,
    pub settings: Settings,
    pub rules: Vec<FolderRuleView>,
    pub runs: Vec<BatchSummary>,
    /// The uploaded profile picture as a `data:` URL, when that is the one chosen.
    /// Read only then, so an unused file is not encoded on every call.
    pub avatar_image: Option<String>,
}

pub fn app_view(config: &Config, ruleset: &RuleSet) -> Result<AppView> {
    let sources = config
        .sources
        .iter()
        .map(|p| SourceView {
            path: p.clone(),
            label: label_of(p),
            count: crate::scan::count_eligible(p, config.settings.include_hidden),
            exists: p.is_dir(),
        })
        .collect();

    let rules = ruleset
        .folder_rules
        .iter()
        .filter(|r| config.destination(&r.destination_key).is_some())
        .map(|r| {
            let reading = prose::read(&r.text);
            FolderRuleView {
                id: r.id.clone(),
                destination_key: r.destination_key.clone(),
                text: r.text.clone(),
                understood: reading.understood(),
                reading: reading.describe(),
            }
        })
        .collect();

    Ok(AppView {
        home: config::home()?,
        sources,
        destinations: config.destinations.clone(),
        folder_sets: config.folder_sets.clone(),
        settings: config.settings.clone(),
        rules,
        runs: journal::list_batches()?,
        avatar_image: if config.settings.avatar == crate::avatar::CUSTOM {
            crate::avatar::load()?
        } else {
            None
        },
    })
}

pub fn label_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

// ── Runs ─────────────────────────────────────────────────────────────────────

/// One destination's share of a run.
#[derive(Debug, Serialize)]
pub struct RunGroup {
    /// The destination key, or `None` for the Trash group and for a folder that has
    /// since been removed from Menlo.
    pub key: Option<String>,
    /// `Documents/Invoices 2026`, or `Trash`.
    pub label: String,
    pub trash: bool,
    pub count: usize,
    pub restored: usize,
}

#[derive(Debug, Serialize)]
pub struct RunView {
    pub summary: BatchSummary,
    pub groups: Vec<RunGroup>,
    /// Files a restore could not put back because they were edited, renamed or moved
    /// on since the run — reported, never forced.
    pub stayed_put: usize,
}

/// The destination a filed path belongs to: the most specific one whose folder
/// contains it, so a file in Documents/Contracts is Contracts', not Documents'.
pub fn owner_of<'a>(to: &Path, config: &'a Config) -> Option<&'a Destination> {
    config
        .destinations
        .iter()
        .filter(|d| to.starts_with(&d.path))
        .max_by_key(|d| d.path.components().count())
}

/// Which group a move is shown in, and restored with. Shared by the view and by the
/// per-destination restore so the two can never disagree about a file.
pub fn group_key(m: &MoveRecord, config: &Config) -> GroupKey {
    if m.strategy == Strategy::Trash {
        return GroupKey::Trash;
    }
    match owner_of(&m.to, config) {
        Some(d) => GroupKey::Destination(d.key.clone()),
        None => GroupKey::Folder(m.to.parent().map(Path::to_path_buf).unwrap_or_default()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupKey {
    Destination(String),
    /// A folder that is no longer one of Menlo's destinations.
    Folder(PathBuf),
    Trash,
}

fn group_label(key: &GroupKey, config: &Config) -> String {
    match key {
        GroupKey::Trash => "Trash".into(),
        GroupKey::Folder(p) => label_of(p),
        GroupKey::Destination(k) => {
            let Some(d) = config.destination(k) else {
                return k.clone();
            };
            match d.parent.as_deref().and_then(|p| config.destination(p)) {
                Some(parent) => format!("{}/{}", parent.label, d.label),
                None => d.label.clone(),
            }
        }
    }
}

pub fn run_view(batch: &Batch, config: &Config) -> RunView {
    let reverted = |m: &MoveRecord| batch.reverts.iter().any(|r| r.entry_id == m.entry_id);

    let mut order: Vec<GroupKey> = Vec::new();
    let mut counts: Vec<(usize, usize)> = Vec::new();
    for m in &batch.moves {
        let key = group_key(m, config);
        let i = match order.iter().position(|k| *k == key) {
            Some(i) => i,
            None => {
                order.push(key);
                counts.push((0, 0));
                order.len() - 1
            }
        };
        counts[i].0 += 1;
        if reverted(m) {
            counts[i].1 += 1;
        }
    }

    let groups = order
        .iter()
        .zip(counts)
        .map(|(key, (count, restored))| RunGroup {
            key: match key {
                GroupKey::Destination(k) => Some(k.clone()),
                _ => None,
            },
            label: group_label(key, config),
            trash: *key == GroupKey::Trash,
            count,
            restored,
        })
        .collect();

    // A file a restore could not bring back, and has not since.
    let mut stuck: Vec<&str> = batch
        .revert_failures
        .iter()
        .map(|f| f.entry_id.as_str())
        .filter(|id| !batch.reverts.iter().any(|r| r.entry_id == *id))
        .collect();
    stuck.sort_unstable();
    stuck.dedup();

    RunView {
        summary: BatchSummary::from(batch),
        groups,
        stayed_put: stuck.len(),
    }
}

// ── Suggestions ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FolderSuggestion {
    pub path: PathBuf,
    pub label: String,
    pub reason: String,
    /// The key to add it under, so a suggested Movies is `video` and the built-in
    /// extension map files into it with no rule.
    pub key: Option<String>,
    pub parent_key: Option<String>,
    /// False when adding it will create the folder.
    pub exists: bool,
}

/// The standard home folder for each built-in category that has one.
const HOME_FOLDERS: &[(&str, &str, &str)] = &[
    ("documents", "Documents", "documents"),
    ("images", "Pictures", "images"),
    ("video", "Movies", "video files"),
    ("audio", "Music", "audio files"),
    ("code", "Developer", "source files"),
];

/// Folders worth adding, drafted from what is actually sitting in the sources — no
/// model involved, so every reason given is a count Menlo made, not a guess.
///
/// At the top level: a standard home folder for each kind of file the sources hold
/// that has nowhere to land. Inside a destination: the subfolders already on disk
/// that Menlo has not been told about.
pub fn suggest_folders(config: &Config, parent_key: Option<&str>) -> Result<Vec<FolderSuggestion>> {
    let home = config::home()?;

    if let Some(pk) = parent_key {
        let Some(parent) = config.destination(pk) else {
            return Ok(Vec::new());
        };
        return Ok(config::existing_subfolders(&parent.path, 50)
            .into_iter()
            .filter(|p| !config.destinations.iter().any(|d| d.path == *p))
            .map(|p| FolderSuggestion {
                label: label_of(&p),
                reason: format!("Already in {} on disk", parent.label),
                key: None,
                parent_key: Some(pk.to_string()),
                exists: true,
                path: p,
            })
            .collect());
    }

    let scan = crate::scan::scan_all(&config.sources, config.settings.include_hidden)?;
    let mut out = Vec::new();
    for (key, folder, noun) in HOME_FOLDERS {
        if config.destination(key).is_some() {
            continue;
        }
        let path = home.join(folder);
        if config.destinations.iter().any(|d| d.path == path) {
            continue;
        }
        let exts = rules::BUILTIN_CATEGORIES
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, e)| *e)
            .unwrap_or(&[]);
        let n = scan
            .items
            .iter()
            .filter(|i| exts.contains(&i.entry.ext.as_str()))
            .count();
        if n == 0 {
            continue;
        }
        out.push(FolderSuggestion {
            label: folder.to_string(),
            reason: format!(
                "Your sources hold {n} {} with nowhere to land",
                if n == 1 {
                    noun.trim_end_matches('s')
                } else {
                    noun
                }
            ),
            key: Some(key.to_string()),
            parent_key: None,
            exists: path.is_dir(),
            path,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{BatchHeader, RevertRecord};
    use chrono::Utc;

    fn dest(key: &str, path: &str, parent: Option<&str>) -> Destination {
        Destination {
            key: key.into(),
            label: path.rsplit('/').next().unwrap().into(),
            path: PathBuf::from(path),
            brief: String::new(),
            parent: parent.map(str::to_string),
        }
    }

    fn mv(id: &str, to: &str, strategy: Strategy) -> MoveRecord {
        MoveRecord {
            entry_id: id.into(),
            from: PathBuf::from(format!("/src/{id}")),
            to: PathBuf::from(to),
            sha256: "x".into(),
            size_bytes: 1,
            strategy,
            renamed_for_collision: false,
            at: Utc::now(),
        }
    }

    #[test]
    fn a_file_belongs_to_its_most_specific_folder() {
        let config = Config::new(
            vec![],
            vec![
                dest("docs", "/h/Documents", None),
                dest("docs-tax", "/h/Documents/Tax", Some("docs")),
            ],
        );
        assert_eq!(
            owner_of(Path::new("/h/Documents/Tax/a.pdf"), &config)
                .unwrap()
                .key,
            "docs-tax"
        );
        assert_eq!(
            owner_of(Path::new("/h/Documents/b.pdf"), &config)
                .unwrap()
                .key,
            "docs"
        );
        assert!(owner_of(Path::new("/h/Elsewhere/c.pdf"), &config).is_none());
    }

    #[test]
    fn a_run_groups_by_folder_with_the_trash_apart() {
        let config = Config::new(
            vec![],
            vec![
                dest("docs", "/h/Documents", None),
                dest("docs-tax", "/h/Documents/Tax", Some("docs")),
            ],
        );
        let batch = Batch {
            header: BatchHeader {
                batch_id: "b".into(),
                started_at: Utc::now(),
                source: PathBuf::from("/src"),
                sources: vec![],
                set_label: None,
                planned: 3,
                app_version: "t".into(),
            },
            moves: vec![
                mv("f_000", "/h/Documents/Tax/a.pdf", Strategy::Rename),
                mv("f_001", "/h/Documents/b.pdf", Strategy::Rename),
                mv("f_002", "/Users/x/.Trash/c.pdf", Strategy::Trash),
            ],
            failures: vec![],
            reverts: vec![RevertRecord {
                entry_id: "f_001".into(),
                from: PathBuf::from("/h/Documents/b.pdf"),
                to: PathBuf::from("/src/f_001"),
                at: Utc::now(),
            }],
            revert_failures: vec![],
            kept: vec![],
        };

        let view = run_view(&batch, &config);
        let labels: Vec<_> = view.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["Documents/Tax", "Documents", "Trash"]);
        assert_eq!(view.groups[1].restored, 1);
        assert!(view.groups[2].trash);
        assert_eq!(view.summary.moved, 2);
        assert_eq!(view.summary.trashed, 1);
    }
}
