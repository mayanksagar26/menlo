//! §1.4: every batch is reversible.
//!
//! One append-only JSONL file per batch at
//! `~/Library/Application Support/menlo/journal/<batch_id>.jsonl`. Reverts append too
//! rather than rewriting, so the file is a complete audit trail and a crash mid-revert
//! leaves a readable record instead of a corrupted one.

use crate::config::{journal_dir, write_atomic};
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Strategy {
    /// Same volume: a single atomic `rename(2)`.
    Rename,
    /// Across volumes: copy, verify the hash, then delete the source.
    CopyVerifyDelete,
    /// A duplicate sent to the Trash. Recorded as a move so the hash-verified revert
    /// below brings it back with no special case — the Trash is just somewhere a file
    /// went.
    Trash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchHeader {
    pub batch_id: String,
    pub started_at: DateTime<Utc>,
    /// The first source. Journals from before multi-source support carry only this.
    pub source: PathBuf,
    #[serde(default)]
    pub sources: Vec<PathBuf>,
    /// The folder set this run was made from, for the Runs page ("Everyday · …").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub set_label: Option<String>,
    /// How many moves this batch intended, so a truncated file is detectable.
    pub planned: usize,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRecord {
    /// Manifest id, for correlating back to the plan.
    pub entry_id: String,
    pub from: PathBuf,
    pub to: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
    pub strategy: Strategy,
    /// True when a collision forced a ` (2)` suffix.
    pub renamed_for_collision: bool,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub entry_id: String,
    pub from: PathBuf,
    pub intended_to: PathBuf,
    pub error: String,
    pub at: DateTime<Utc>,
}

/// A file the user chose to leave where it was — a duplicate they kept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeptRecord {
    pub entry_id: String,
    pub from: PathBuf,
    pub reason: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertRecord {
    pub entry_id: String,
    /// Where the file was found (the previous `to`).
    pub from: PathBuf,
    /// Where it was put back. May carry a collision suffix if the original name was
    /// taken in the meantime.
    pub to: PathBuf,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Line {
    Header(BatchHeader),
    Move(MoveRecord),
    Failure(FailureRecord),
    Revert(RevertRecord),
    /// Written when a revert could not restore a file, with why.
    RevertFailure(FailureRecord),
    Kept(KeptRecord),
}

/// A batch as reconstructed from its JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub header: BatchHeader,
    pub moves: Vec<MoveRecord>,
    pub failures: Vec<FailureRecord>,
    pub reverts: Vec<RevertRecord>,
    pub revert_failures: Vec<FailureRecord>,
    #[serde(default)]
    pub kept: Vec<KeptRecord>,
}

impl Batch {
    /// Moves with no corresponding revert — what a revert would still act on.
    pub fn outstanding(&self) -> Vec<&MoveRecord> {
        self.moves
            .iter()
            .filter(|m| !self.reverts.iter().any(|r| r.entry_id == m.entry_id))
            .collect()
    }

    pub fn fully_reverted(&self) -> bool {
        !self.moves.is_empty() && self.outstanding().is_empty()
    }
}

/// Summary row for the Runs page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    pub batch_id: String,
    pub started_at: DateTime<Utc>,
    pub source: PathBuf,
    pub set_label: Option<String>,
    /// Filed into a destination. Excludes duplicates sent to the Trash.
    pub moved: usize,
    pub trashed: usize,
    pub kept: usize,
    pub failed: usize,
    pub reverted: usize,
    pub fully_reverted: bool,
}

impl From<&Batch> for BatchSummary {
    fn from(b: &Batch) -> Self {
        let trashed = b
            .moves
            .iter()
            .filter(|m| m.strategy == Strategy::Trash)
            .count();
        BatchSummary {
            batch_id: b.header.batch_id.clone(),
            started_at: b.header.started_at,
            source: b.header.source.clone(),
            set_label: b.header.set_label.clone(),
            moved: b.moves.len() - trashed,
            trashed,
            kept: b.kept.len(),
            failed: b.failures.len(),
            reverted: b.reverts.len(),
            fully_reverted: b.fully_reverted(),
        }
    }
}

fn batch_path(batch_id: &str) -> Result<PathBuf> {
    // The id is app-generated (a UUID), but it reaches this function from the frontend
    // on revert, so treat it as untrusted: reject anything that is not a bare segment.
    if batch_id.is_empty()
        || !batch_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err(Error::safety(format!(
            "`{batch_id}` is not a valid batch id"
        )));
    }
    Ok(journal_dir()?.join(format!("{batch_id}.jsonl")))
}

/// Handle to one batch's journal file, held open for the duration of a run.
pub struct Journal {
    path: PathBuf,
    file: std::fs::File,
}

impl Journal {
    pub fn create(header: BatchHeader) -> Result<Journal> {
        let dir = journal_dir()?;
        std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        let path = batch_path(&header.batch_id)?;
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)
            .map_err(|e| Error::io(&path, e))?;
        let mut j = Journal { path, file };
        j.append(&Line::Header(header))?;
        Ok(j)
    }

    /// Reopen an existing batch to append revert records.
    pub fn reopen(batch_id: &str) -> Result<Journal> {
        let path = batch_path(batch_id)?;
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .map_err(|e| Error::io(&path, e))?;
        Ok(Journal { path, file })
    }

    /// Append one line and fsync. The fsync is the point: a journal that loses its
    /// last few entries in a crash cannot be trusted to revert.
    pub fn append(&mut self, line: &Line) -> Result<()> {
        let mut buf = serde_json::to_vec(line)?;
        buf.push(b'\n');
        self.file
            .write_all(&buf)
            .map_err(|e| Error::io(&self.path, e))?;
        self.file
            .sync_data()
            .map_err(|e| Error::io(&self.path, e))?;
        Ok(())
    }
}

pub fn read_batch(batch_id: &str) -> Result<Batch> {
    let path = batch_path(batch_id)?;
    let text = std::fs::read_to_string(&path).map_err(|e| Error::io(&path, e))?;
    parse_batch(&text)
}

fn parse_batch(text: &str) -> Result<Batch> {
    let mut header: Option<BatchHeader> = None;
    let mut moves = Vec::new();
    let mut failures = Vec::new();
    let mut reverts = Vec::new();
    let mut revert_failures = Vec::new();
    let mut kept = Vec::new();

    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        // A torn final line (crash mid-write) must not make the batch unreadable.
        let Ok(line) = serde_json::from_str::<Line>(raw) else {
            continue;
        };
        match line {
            Line::Header(h) => header = Some(h),
            Line::Move(m) => moves.push(m),
            Line::Failure(f) => failures.push(f),
            Line::Revert(r) => reverts.push(r),
            Line::RevertFailure(f) => revert_failures.push(f),
            Line::Kept(k) => kept.push(k),
        }
    }

    let header = header.ok_or_else(|| Error::integrity("journal has no header record"))?;
    Ok(Batch {
        header,
        moves,
        failures,
        reverts,
        revert_failures,
        kept,
    })
}

/// Every batch, newest first.
pub fn list_batches() -> Result<Vec<BatchSummary>> {
    let dir = journal_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for de in std::fs::read_dir(&dir)
        .map_err(|e| Error::io(&dir, e))?
        .flatten()
    {
        let path = de.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        // Skip an unreadable batch rather than failing the whole History screen.
        let Ok(batch) = parse_batch(&text) else {
            continue;
        };
        out.push(BatchSummary::from(&batch));
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.started_at));
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevertOutcome {
    pub batch_id: String,
    pub restored: usize,
    pub failed: Vec<FailureRecord>,
}

/// Put a batch back, verifying hashes first (§1.4, task 7).
///
/// A file whose hash no longer matches has been edited since the move; restoring it
/// over whatever now sits at the original path could destroy the newer work. Those are
/// reported and left in place rather than moved.
pub fn revert_batch(batch_id: &str) -> Result<RevertOutcome> {
    revert_batch_where(batch_id, |_| true)
}

/// Put back only the moves `keep` selects — one destination of a run, say, or just
/// the duplicates it sent to the Trash. Same hash checks as a whole-batch revert.
pub fn revert_batch_where(
    batch_id: &str,
    keep: impl Fn(&MoveRecord) -> bool,
) -> Result<RevertOutcome> {
    let batch = read_batch(batch_id)?;
    let mut journal = Journal::reopen(batch_id)?;
    let mut restored = 0usize;
    let mut failed: Vec<FailureRecord> = Vec::new();

    // Reverse order, so a rename-chain within one batch unwinds cleanly.
    for record in batch.outstanding().into_iter().filter(|m| keep(m)).rev() {
        match revert_one(record) {
            Ok(to) => {
                journal.append(&Line::Revert(RevertRecord {
                    entry_id: record.entry_id.clone(),
                    from: record.to.clone(),
                    to,
                    at: Utc::now(),
                }))?;
                restored += 1;
            }
            Err(e) => {
                let f = FailureRecord {
                    entry_id: record.entry_id.clone(),
                    from: record.to.clone(),
                    intended_to: record.from.clone(),
                    error: e.to_string(),
                    at: Utc::now(),
                };
                journal.append(&Line::RevertFailure(f.clone()))?;
                failed.push(f);
            }
        }
    }

    Ok(RevertOutcome {
        batch_id: batch_id.to_string(),
        restored,
        failed,
    })
}

fn revert_one(record: &MoveRecord) -> Result<PathBuf> {
    if !record.to.exists() {
        return Err(Error::integrity(format!(
            "{} is no longer there; it may have been moved or deleted since",
            record.to.display()
        )));
    }

    // Hash-verified: §1.4 is explicit about this.
    let current = crate::scan::hash_file(&record.to)?;
    if current != record.sha256 {
        return Err(Error::integrity(format!(
            "{} has changed since it was filed; leaving it alone",
            record.to.display()
        )));
    }

    crate::safety::assert_operable(&record.from)?;
    crate::safety::assert_operable(&record.to)?;

    let parent = record
        .from
        .parent()
        .ok_or_else(|| Error::integrity("original location has no parent directory"))?;
    std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;

    // Never overwrite on the way back either.
    let target = crate::execute::unique_path(&record.from);
    crate::execute::transfer(&record.to, &target)?;
    Ok(target)
}

/// Convenience for callers that want to persist a plan alongside its batch, so the
/// History screen can show what was proposed as well as what happened.
pub fn write_plan_snapshot(batch_id: &str, plan: &crate::plan::Plan) -> Result<()> {
    let dir = journal_dir()?;
    std::fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let path = dir.join(format!("{batch_id}.plan.json"));
    write_atomic(&path, serde_json::to_vec_pretty(plan)?.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> BatchHeader {
        BatchHeader {
            batch_id: "b1".into(),
            started_at: Utc::now(),
            source: PathBuf::from("/tmp/dl"),
            sources: vec![PathBuf::from("/tmp/dl")],
            set_label: Some("Everyday".into()),
            planned: 2,
            app_version: "test".into(),
        }
    }

    fn mv(entry_id: &str) -> MoveRecord {
        MoveRecord {
            entry_id: entry_id.into(),
            from: PathBuf::from(format!("/tmp/dl/{entry_id}")),
            to: PathBuf::from(format!("/tmp/docs/{entry_id}")),
            sha256: "deadbeef".into(),
            size_bytes: 1,
            strategy: Strategy::Rename,
            renamed_for_collision: false,
            at: Utc::now(),
        }
    }

    fn jsonl(lines: &[Line]) -> String {
        lines
            .iter()
            .map(|l| serde_json::to_string(l).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn round_trips_a_batch() {
        let text = jsonl(&[
            Line::Header(header()),
            Line::Move(mv("f_000")),
            Line::Move(mv("f_001")),
        ]);
        let b = parse_batch(&text).unwrap();
        assert_eq!(b.moves.len(), 2);
        assert_eq!(b.outstanding().len(), 2);
        assert!(!b.fully_reverted());
    }

    #[test]
    fn outstanding_excludes_already_reverted_entries() {
        let text = jsonl(&[
            Line::Header(header()),
            Line::Move(mv("f_000")),
            Line::Move(mv("f_001")),
            Line::Revert(RevertRecord {
                entry_id: "f_000".into(),
                from: PathBuf::from("/tmp/docs/f_000"),
                to: PathBuf::from("/tmp/dl/f_000"),
                at: Utc::now(),
            }),
        ]);
        let b = parse_batch(&text).unwrap();
        assert_eq!(b.outstanding().len(), 1);
        assert_eq!(b.outstanding()[0].entry_id, "f_001");
    }

    #[test]
    fn survives_a_torn_final_line() {
        // Simulates a crash mid-append.
        let mut text = jsonl(&[Line::Header(header()), Line::Move(mv("f_000"))]);
        text.push_str("\n{\"type\":\"move\",\"entry_id\":\"f_00");
        let b = parse_batch(&text).unwrap();
        assert_eq!(b.moves.len(), 1);
    }

    #[test]
    fn a_summary_separates_filed_trashed_and_kept() {
        let mut trashed = mv("f_001");
        trashed.strategy = Strategy::Trash;
        let text = jsonl(&[
            Line::Header(header()),
            Line::Move(mv("f_000")),
            Line::Move(trashed),
            Line::Kept(KeptRecord {
                entry_id: "f_002".into(),
                from: PathBuf::from("/tmp/dl/f_002"),
                reason: "duplicate kept".into(),
                at: Utc::now(),
            }),
        ]);
        let s = BatchSummary::from(&parse_batch(&text).unwrap());
        assert_eq!((s.moved, s.trashed, s.kept), (1, 1, 1));
        assert_eq!(s.set_label.as_deref(), Some("Everyday"));
    }

    #[test]
    fn a_journal_from_before_multi_source_still_reads() {
        let old = r#"{"type":"header","batch_id":"b0","started_at":"2026-01-01T00:00:00Z","source":"/tmp/dl","planned":1,"app_version":"0.1.0"}"#;
        let b = parse_batch(old).unwrap();
        assert!(b.header.sources.is_empty());
        assert!(b.kept.is_empty());
    }

    #[test]
    fn a_headerless_journal_is_an_error() {
        let text = jsonl(&[Line::Move(mv("f_000"))]);
        assert!(parse_batch(&text).is_err());
    }

    #[test]
    fn rejects_batch_ids_that_are_paths() {
        assert!(batch_path("../../etc/passwd").is_err());
        assert!(batch_path("a/b").is_err());
        assert!(batch_path("").is_err());
        assert!(batch_path("2f8a1b3c-0000-4000-8000-000000000000").is_ok());
    }
}
