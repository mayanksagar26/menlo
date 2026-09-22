//! The only place in Menlo that mutates the filesystem (§1.1).
//!
//! Same volume → `rename(2)`. Across volumes → copy, verify SHA-256, then delete the
//! source. Never overwrite: a collision gets a ` (2)` suffix before the extension.
//! Every completed move is journalled before the next one starts, so an interrupted
//! run is still fully revertible.

use crate::config::{Config, DuplicatePolicy};
use crate::error::{Error, Result};
use crate::journal::{BatchHeader, FailureRecord, Journal, KeptRecord, Line, MoveRecord, Strategy};
use crate::plan::{DuplicateChoice, DuplicateKind, Plan, PlanEntry};
use crate::safety;
use crate::scan::{hash_file, Scan};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteOutcome {
    pub batch_id: String,
    /// True when the user pressed Stop. Everything after that point was left alone.
    pub stopped: bool,
    pub moved: usize,
    /// Duplicates sent to the Trash.
    pub trashed: usize,
    /// Duplicates the user chose to leave where they were.
    pub kept: usize,
    pub failures: Vec<FailureRecord>,
    /// Rules crystallised from this batch (§1.2 tier 2).
    pub learned: Vec<crate::rules::Rule>,
}

/// What happened to one file, for the progress stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepOutcome {
    Moved,
    Trashed,
    Kept,
    Failed,
}

/// One tick of the Move stage's counter. Carries ids and keys only — never a path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
    pub entry_id: String,
    pub destination_key: Option<String>,
    pub outcome: StepOutcome,
}

/// Apply the approved entries of a plan. See [`execute_with`].
pub fn execute(plan: &Plan, scan: &Scan, config: &Config) -> Result<ExecuteOutcome> {
    execute_with(plan, scan, config, None, || false, |_| {})
}

/// What to do with an entry's duplicate. An explicit choice wins; otherwise the
/// Duplicates setting decides, and "Ask" — which should have been asked already — falls
/// back to keeping the file, the one choice that can never lose anything.
fn choice_for(entry: &PlanEntry, policy: DuplicatePolicy) -> DuplicateChoice {
    match (&entry.duplicate, entry.on_duplicate) {
        (None, _) => DuplicateChoice::MoveAnyway,
        (Some(_), Some(c)) => c,
        (Some(d), None) => match d.kind {
            DuplicateKind::SameName => DuplicateChoice::MoveAnyway,
            DuplicateKind::Identical => match policy {
                DuplicatePolicy::Trash => DuplicateChoice::Trash,
                DuplicatePolicy::Keep | DuplicatePolicy::Ask => DuplicateChoice::Keep,
            },
        },
    }
}

/// Apply the approved entries of a plan, reporting each file as it lands.
///
/// One file failing does not abort the batch — the other 199 files should still get
/// filed, and the failure is recorded and surfaced. This is distinct from §4.3, where
/// a *malformed plan* is rejected wholesale before we get here.
///
/// `plan.entries[*].duplicate` must be fresh — recomputed by `dupes::detect` against
/// the stored scan — before this is called. The Trash branch re-verifies the bytes
/// regardless, but it is the caller's job not to hand in a stale finding.
///
/// `should_stop` is asked before each file. Stopping never interrupts a file mid-move:
/// the one in flight finishes and is journalled, and the rest are not touched.
pub fn execute_with(
    plan: &Plan,
    scan: &Scan,
    config: &Config,
    set_label: Option<String>,
    should_stop: impl Fn() -> bool,
    mut on_progress: impl FnMut(Progress),
) -> Result<ExecuteOutcome> {
    config.validate()?;

    let actionable: Vec<_> = plan.actionable().collect();
    if actionable.len() > crate::config::MAX_BATCH {
        return Err(Error::safety(format!(
            "batch of {} exceeds the {}-file cap",
            actionable.len(),
            crate::config::MAX_BATCH
        )));
    }

    let mut journal = Journal::create(BatchHeader {
        batch_id: plan.batch_id.clone(),
        started_at: Utc::now(),
        source: plan.source.clone(),
        sources: plan.sources.clone(),
        set_label,
        planned: actionable.len(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    })?;

    let mut moved = 0usize;
    let mut trashed = 0usize;
    let mut kept = 0usize;
    let mut failures = Vec::new();
    let total = actionable.len();
    // Where each file in this batch ended up, so a later duplicate can be checked
    // against the copy that was actually kept.
    let mut landed: HashMap<String, PathBuf> = HashMap::new();
    let mut stopped = false;

    for (n, entry) in actionable.into_iter().enumerate() {
        if should_stop() {
            stopped = true;
            break;
        }
        let mut report = |outcome: StepOutcome| {
            on_progress(Progress {
                done: n + 1,
                total,
                entry_id: entry.id.clone(),
                destination_key: entry.destination_key.clone(),
                outcome,
            })
        };

        let Some(item) = scan.get(&entry.id) else {
            // The plan and the scan disagree; §4.3 should have caught this.
            failures.push(record_failure(
                &mut journal,
                &entry.id,
                Path::new(""),
                Path::new(""),
                "no scanned file for this plan entry",
            )?);
            report(StepOutcome::Failed);
            continue;
        };

        let key = entry.destination_key.as_deref().unwrap_or_default();
        let Some(dest) = config.destination(key) else {
            failures.push(record_failure(
                &mut journal,
                &entry.id,
                &item.path,
                Path::new(""),
                &format!("destination `{key}` no longer exists"),
            )?);
            report(StepOutcome::Failed);
            continue;
        };

        match choice_for(entry, config.settings.duplicates) {
            DuplicateChoice::MoveAnyway => {}
            DuplicateChoice::Keep => {
                journal.append(&Line::Kept(KeptRecord {
                    entry_id: entry.id.clone(),
                    from: item.path.clone(),
                    reason: "already in the destination; kept where it was".into(),
                    at: Utc::now(),
                }))?;
                kept += 1;
                report(StepOutcome::Kept);
                continue;
            }
            DuplicateChoice::Trash => {
                match trash_duplicate(entry, &item.path, &dest.path, &landed, scan, config) {
                    Ok(done) => {
                        journal.append(&Line::Move(MoveRecord {
                            entry_id: entry.id.clone(),
                            from: item.path.clone(),
                            to: done.to,
                            sha256: done.sha256,
                            size_bytes: done.size_bytes,
                            strategy: Strategy::Trash,
                            renamed_for_collision: done.renamed_for_collision,
                            at: Utc::now(),
                        }))?;
                        trashed += 1;
                        report(StepOutcome::Trashed);
                    }
                    Err(e) => {
                        failures.push(record_failure(
                            &mut journal,
                            &entry.id,
                            &item.path,
                            Path::new(""),
                            &e.to_string(),
                        )?);
                        report(StepOutcome::Failed);
                    }
                }
                continue;
            }
        }

        let filename = entry
            .rename_to
            .clone()
            .unwrap_or_else(|| entry.name.clone());
        // Belt and braces: §4.3 already validated this, but this is the last gate
        // before a path is constructed from it.
        if let Err(why) = crate::plan::validate_rename(&filename) {
            failures.push(record_failure(
                &mut journal,
                &entry.id,
                &item.path,
                Path::new(""),
                &why,
            )?);
            report(StepOutcome::Failed);
            continue;
        }

        match move_one(
            &item.path,
            dest.path.as_path(),
            &filename,
            config,
            &entry.ext,
        ) {
            Ok(done) => {
                journal.append(&Line::Move(MoveRecord {
                    entry_id: entry.id.clone(),
                    from: item.path.clone(),
                    to: done.to.clone(),
                    sha256: done.sha256,
                    size_bytes: done.size_bytes,
                    strategy: done.strategy,
                    renamed_for_collision: done.renamed_for_collision,
                    at: Utc::now(),
                }))?;
                landed.insert(entry.id.clone(), done.to);
                moved += 1;
                report(StepOutcome::Moved);
            }
            Err(e) => {
                let intended = dest.path.join(&filename);
                failures.push(record_failure(
                    &mut journal,
                    &entry.id,
                    &item.path,
                    &intended,
                    &e.to_string(),
                )?);
                report(StepOutcome::Failed);
            }
        }
    }

    // §1.2 tier 2: crystallise what the user approved.
    let mut ruleset = crate::rules::load_rules()?;
    let learned = crate::rules::learn_from_approved(plan, &mut ruleset);
    if !learned.is_empty() {
        crate::rules::save_rules(&ruleset)?;
    }

    let _ = crate::journal::write_plan_snapshot(&plan.batch_id, plan);

    Ok(ExecuteOutcome {
        batch_id: plan.batch_id.clone(),
        stopped,
        moved,
        trashed,
        kept,
        failures,
        learned,
    })
}

/// Send a duplicate to the Trash — only if it is, right now, byte-identical to a copy
/// that will survive.
///
/// "The copy that will survive" is the file already in the destination, or for a
/// duplicate within the batch, wherever its twin is now: the destination if it moved,
/// its source if it was kept. Both are re-hashed here, immediately before the move,
/// not trusted from when the plan was built.
fn trash_duplicate(
    entry: &PlanEntry,
    source: &Path,
    dest_dir: &Path,
    landed: &HashMap<String, PathBuf>,
    scan: &Scan,
    config: &Config,
) -> Result<Moved> {
    let dup = entry
        .duplicate
        .as_ref()
        .ok_or_else(|| Error::safety("nothing to compare against; left where it was"))?;
    if dup.kind != DuplicateKind::Identical {
        return Err(Error::safety(
            "only a byte-identical copy can be sent to the Trash; left where it was",
        ));
    }

    let survivor = match &dup.same_as_entry {
        Some(twin) => landed
            .get(twin)
            .cloned()
            .or_else(|| scan.get(twin).map(|i| i.path.clone()))
            .ok_or_else(|| Error::safety("its twin in this batch has gone; left where it was"))?,
        None => {
            crate::plan::validate_rename(&dup.existing_name).map_err(Error::safety)?;
            dest_dir.join(&dup.existing_name)
        }
    };

    if survivor == source || !crate::dupes::still_identical(source, &survivor) {
        return Err(Error::integrity(
            "no longer identical to the copy already filed; left where it was",
        ));
    }

    trash_one(source, config, &entry.ext)
}

/// Move a file into the Trash with a name that does not collide, and report where it
/// went so the journal — and therefore Runs — can bring it back.
///
/// The `trash` crate is in the dependency set, but it discards the resulting location
/// and its default method drives Finder over AppleScript, which asks the user for an
/// Automation permission. Moving into the Trash folder directly needs neither.
fn trash_one(source: &Path, config: &Config, ext: &str) -> Result<Moved> {
    safety::assert_trash_allowed(source, config, ext)?;

    let dir = crate::config::trash_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let name = source
        .file_name()
        .ok_or_else(|| Error::safety(format!("{} has no file name", source.display())))?;
    let naive = dir.join(name);
    let target = unique_path(&naive);

    let size_bytes = std::fs::metadata(source)
        .map_err(|e| Error::io(source, e))?
        .len();
    transfer(source, &target)?;
    let sha256 = hash_file(&target)?;

    Ok(Moved {
        renamed_for_collision: target != naive,
        to: target,
        sha256,
        size_bytes,
        strategy: Strategy::Trash,
    })
}

fn record_failure(
    journal: &mut Journal,
    entry_id: &str,
    from: &Path,
    intended_to: &Path,
    error: &str,
) -> Result<FailureRecord> {
    let f = FailureRecord {
        entry_id: entry_id.to_string(),
        from: from.to_path_buf(),
        intended_to: intended_to.to_path_buf(),
        error: error.to_string(),
        at: Utc::now(),
    };
    journal.append(&Line::Failure(f.clone()))?;
    Ok(f)
}

struct Moved {
    to: PathBuf,
    sha256: String,
    size_bytes: u64,
    strategy: Strategy,
    renamed_for_collision: bool,
}

fn move_one(
    source: &Path,
    dest_dir: &Path,
    filename: &str,
    config: &Config,
    ext: &str,
) -> Result<Moved> {
    std::fs::create_dir_all(dest_dir).map_err(|e| Error::io(dest_dir, e))?;

    let naive_target = dest_dir.join(filename);

    // §7 gate: runs here, immediately before the syscall, so a symlink swapped after
    // approval is still caught.
    safety::assert_move_allowed(source, &naive_target, config, ext)?;

    let target = unique_path(&naive_target);
    let renamed_for_collision = target != naive_target;

    let size_bytes = std::fs::metadata(source)
        .map_err(|e| Error::io(source, e))?
        .len();
    let strategy = transfer(source, &target)?;
    let sha256 = hash_file(&target)?;

    Ok(Moved {
        to: target,
        sha256,
        size_bytes,
        strategy,
        renamed_for_collision,
    })
}

/// Move `source` to `target`, choosing the strategy by volume.
///
/// `target` must already be collision-free — callers get that from [`unique_path`].
pub fn transfer(source: &Path, target: &Path) -> Result<Strategy> {
    if same_volume(source, target) {
        std::fs::rename(source, target).map_err(|e| Error::io(source, e))?;
        return Ok(Strategy::Rename);
    }
    copy_verify_delete(source, target)?;
    Ok(Strategy::CopyVerifyDelete)
}

/// Compare device ids. Falls back to "assume different volume" when either stat fails,
/// which costs a copy but never loses data to a failed cross-device `rename`.
fn same_volume(source: &Path, target: &Path) -> bool {
    let Ok(src) = std::fs::metadata(source) else {
        return false;
    };
    // The target does not exist yet; its directory is on the volume that matters.
    let Some(dir) = target.parent() else {
        return false;
    };
    let Ok(dst) = std::fs::metadata(dir) else {
        return false;
    };
    src.dev() == dst.dev()
}

/// Copy → verify SHA-256 → delete. The source is removed only after the copy is proven
/// byte-identical, so an interrupted or corrupted copy costs disk space, never data.
///
/// §7 asks for a free-space check before cross-volume copies. `fs4` was not in the
/// approved dependency set and `std` exposes no free-space API, so this instead handles
/// exhaustion where it actually occurs: an out-of-space copy is caught, the partial
/// file removed, and the source left untouched. That is strictly safer than a
/// pre-check, which races against other writers anyway.
fn copy_verify_delete(source: &Path, target: &Path) -> Result<()> {
    let source_hash = hash_file(source)?;

    if let Err(e) = std::fs::copy(source, target) {
        let _ = std::fs::remove_file(target);
        let msg = if e.kind() == std::io::ErrorKind::StorageFull {
            format!(
                "not enough free space on the destination volume for {}",
                source.display()
            )
        } else {
            format!("copy failed: {e}")
        };
        return Err(Error::safety(msg));
    }

    let target_hash = hash_file(target)?;
    if target_hash != source_hash {
        let _ = std::fs::remove_file(target);
        return Err(Error::integrity(format!(
            "copy of {} did not verify; the original has not been touched",
            source.display()
        )));
    }

    std::fs::remove_file(source).map_err(|e| Error::io(source, e))?;
    Ok(())
}

/// Never overwrite (task 6). Appends ` (2)`, ` (3)`, … before the extension.
///
/// The loop is bounded: past 999 collisions something is wrong and a unique suffix is
/// better than spinning.
pub fn unique_path(target: &Path) -> PathBuf {
    if !target.exists() {
        return target.to_path_buf();
    }
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let stem = target
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = target.extension().map(|e| e.to_string_lossy().to_string());

    for n in 2..=999 {
        let name = match &ext {
            Some(e) => format!("{stem} ({n}).{e}"),
            None => format!("{stem} ({n})"),
        };
        let candidate = dir.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }

    let name = match &ext {
        Some(e) => format!("{stem} ({}).{e}", uuid::Uuid::new_v4()),
        None => format!("{stem} ({})", uuid::Uuid::new_v4()),
    };
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn unique_path_leaves_a_free_name_alone() {
        let tmp = TempDir::new();
        let p = tmp.join("a.pdf");
        assert_eq!(unique_path(&p), p);
    }

    #[test]
    fn unique_path_suffixes_before_the_extension() {
        let tmp = TempDir::new();
        tmp.write("a.pdf", b"x");
        assert_eq!(unique_path(&tmp.join("a.pdf")), tmp.join("a (2).pdf"));

        tmp.write("a (2).pdf", b"x");
        assert_eq!(unique_path(&tmp.join("a.pdf")), tmp.join("a (3).pdf"));
    }

    #[test]
    fn unique_path_handles_extensionless_and_dotted_names() {
        let tmp = TempDir::new();
        tmp.write("README", b"x");
        assert_eq!(unique_path(&tmp.join("README")), tmp.join("README (2)"));

        tmp.write("archive.tar.gz", b"x");
        assert_eq!(
            unique_path(&tmp.join("archive.tar.gz")),
            tmp.join("archive.tar (2).gz")
        );
    }

    #[test]
    fn transfer_moves_within_a_volume() {
        let tmp = TempDir::new();
        let src = tmp.write("a.txt", b"hello");
        std::fs::create_dir_all(tmp.join("dest")).unwrap();
        let dst = tmp.join("dest/a.txt");

        assert_eq!(transfer(&src, &dst).unwrap(), Strategy::Rename);
        assert!(!src.exists());
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello");
    }

    #[test]
    fn copy_verify_delete_preserves_contents_and_removes_the_source() {
        let tmp = TempDir::new();
        let src = tmp.write("a.txt", b"hello");
        let dst = tmp.join("copied.txt");

        copy_verify_delete(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(std::fs::read(&dst).unwrap(), b"hello");
    }
}
