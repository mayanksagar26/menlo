//! Folder → `Manifest`.
//!
//! The type split here is load-bearing. [`ManifestEntry`] is exactly the §4.1 contract
//! and is the *only* thing that ever reaches a CLI prompt; it carries a basename, never
//! a path. [`ScanItem`] pairs an entry with the real path and stays inside Rust. Making
//! the leak impossible at the type level beats remembering not to do it.

use crate::config::MAX_BATCH;
use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// §4.1. Serialised verbatim into the prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestEntry {
    pub id: String,
    /// Basename only. Never a path — the prompt must not be able to leak directory
    /// structure (§4.1).
    pub name: String,
    pub ext: String,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub mime: String,
    /// `kMDItemWhereFroms`. Populated in Phase 2 by `extract.rs`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Capped at 400 chars. Populated in Phase 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

/// Manifest entry plus the private path. Never serialised as a whole.
#[derive(Debug, Clone)]
pub struct ScanItem {
    pub entry: ManifestEntry,
    pub path: PathBuf,
    /// The source folder this came from. Private for the same reason `path` is.
    pub source: PathBuf,
}

/// Why a scanned path was left out, so the UI can be honest about the difference
/// between "folder has 210 files" and "Menlo is proposing 187 moves".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Dotfile,
    SystemFile,
    PartialDownload,
    Symlink,
    Directory,
    Unreadable,
    BatchCapReached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skipped {
    pub name: String,
    pub reason: SkipReason,
}

#[derive(Debug, Clone)]
pub struct Scan {
    pub items: Vec<ScanItem>,
    pub skipped: Vec<Skipped>,
    /// True when the folders held more eligible files than §7's 500-file cap.
    pub truncated: bool,
    /// The folders that were scanned, in the order they were scanned.
    pub sources: Vec<PathBuf>,
}

impl Scan {
    /// The §4.1 manifest — what a CLI adapter is allowed to see.
    pub fn manifest(&self) -> Vec<&ManifestEntry> {
        self.items.iter().map(|i| &i.entry).collect()
    }

    pub fn get(&self, id: &str) -> Option<&ScanItem> {
        self.items.iter().find(|i| i.entry.id == id)
    }
}

/// In-progress downloads. Moving one of these corrupts it.
const PARTIAL_EXTS: &[&str] = &["crdownload", "part", "partial", "download", "tmp"];

/// Files macOS puts in every folder and nobody wants filed.
const SYSTEM_NAMES: &[&str] = &[".DS_Store", "Icon\r", ".localized"];

fn is_partial(name: &str, ext: &str) -> bool {
    PARTIAL_EXTS.contains(&ext.to_ascii_lowercase().as_str())
        // Safari's in-progress downloads are directories, but be defensive.
        || name.ends_with(".download")
}

/// Scan a single folder, non-recursively. See [`scan_all`].
pub fn scan(dir: &Path) -> Result<Scan> {
    scan_all(&[dir.to_path_buf()], false)
}

/// Scan several folders, each non-recursively, into one manifest.
///
/// v1 deliberately ignores `.DS_Store`, partial downloads, symlinks and directories
/// (§6.2), and dotfiles unless `include_hidden` is set. Subfolders are left entirely
/// alone: a user who has already organised something into a folder does not want it
/// re-organised.
///
/// Ids run across every folder (`f_000` …), so a plan can refer to any file by id
/// alone. §7's batch cap applies to the whole scan, not per folder. A folder listed
/// twice is scanned once.
pub fn scan_all(dirs: &[PathBuf], include_hidden: bool) -> Result<Scan> {
    let mut sources: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if !sources.contains(d) {
            sources.push(d.clone());
        }
    }
    for d in &sources {
        crate::safety::assert_operable(d)?;
    }

    let mut items = Vec::new();
    let mut skipped = Vec::new();
    let mut truncated = false;

    for dir in &sources {
        scan_one(
            dir,
            include_hidden,
            &mut items,
            &mut skipped,
            &mut truncated,
        );
    }

    Ok(Scan {
        items,
        skipped,
        truncated,
        sources,
    })
}

fn scan_one(
    dir: &Path,
    include_hidden: bool,
    items: &mut Vec<ScanItem>,
    skipped: &mut Vec<Skipped>,
    truncated: &mut bool,
) {
    // `walkdir` with depth 1 rather than `read_dir` so the recursive Phase 3 sweep is a
    // one-line change, and so we get consistent symlink handling for free.
    let walker = walkdir::WalkDir::new(dir)
        .min_depth(1)
        .max_depth(1)
        .follow_links(false);

    // Sort for determinism: ids must be stable across runs of the same folder, which
    // matters for the journal and for test assertions.
    let mut entries: Vec<walkdir::DirEntry> = walker.into_iter().filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name().to_os_string());

    for de in entries {
        let name = de.file_name().to_string_lossy().to_string();
        let path = de.path().to_path_buf();

        let file_type = de.file_type();
        if file_type.is_symlink() {
            skipped.push(Skipped {
                name,
                reason: SkipReason::Symlink,
            });
            continue;
        }
        if file_type.is_dir() {
            skipped.push(Skipped {
                name,
                reason: SkipReason::Directory,
            });
            continue;
        }
        if SYSTEM_NAMES.contains(&name.as_str()) {
            skipped.push(Skipped {
                name,
                reason: SkipReason::SystemFile,
            });
            continue;
        }
        if name.starts_with('.') && !include_hidden {
            skipped.push(Skipped {
                name,
                reason: SkipReason::Dotfile,
            });
            continue;
        }

        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();

        if is_partial(&name, &ext) {
            skipped.push(Skipped {
                name,
                reason: SkipReason::PartialDownload,
            });
            continue;
        }

        let Ok(meta) = de.metadata() else {
            skipped.push(Skipped {
                name,
                reason: SkipReason::Unreadable,
            });
            continue;
        };

        if items.len() >= MAX_BATCH {
            *truncated = true;
            skipped.push(Skipped {
                name,
                reason: SkipReason::BatchCapReached,
            });
            continue;
        }

        let entry = ManifestEntry {
            id: format!("f_{:03}", items.len()),
            mime: mime_guess::from_path(&path)
                .first_raw()
                .unwrap_or("application/octet-stream")
                .to_string(),
            name,
            ext,
            size_bytes: meta.len(),
            created_at: system_time_to_utc(meta.created().ok()),
            modified_at: system_time_to_utc(meta.modified().ok()),
            origin: None,  // Phase 2
            excerpt: None, // Phase 2
        };

        items.push(ScanItem {
            entry,
            path,
            source: dir.to_path_buf(),
        });
    }
}

/// How many files a scan of `dir` would consider, without building a manifest. Cheap
/// enough for the home screen to call for every source on every visit.
pub fn count_eligible(dir: &Path, include_hidden: bool) -> usize {
    if crate::safety::assert_operable(dir).is_err() {
        return 0;
    }
    scan_all(&[dir.to_path_buf()], include_hidden)
        .map(|s| s.items.len())
        .unwrap_or(0)
}

/// Falls back to the epoch when a filesystem does not record the timestamp, rather
/// than failing the whole scan over a missing birth time.
fn system_time_to_utc(t: Option<std::time::SystemTime>) -> DateTime<Utc> {
    t.map(DateTime::<Utc>::from)
        .unwrap_or_else(|| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
}

/// Stream a file through SHA-256. Used for cross-volume verification and for the
/// journal's revert integrity check.
pub fn hash_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let mut file = std::fs::File::open(path).map_err(|e| Error::io(path, e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;

    #[test]
    fn ignores_dotfiles_system_files_and_partials() {
        let tmp = TempDir::new();
        tmp.write("report.pdf", b"pdf");
        tmp.write(".DS_Store", b"junk");
        tmp.write(".hidden", b"junk");
        tmp.write("movie.mp4.crdownload", b"partial");
        tmp.write("archive.part", b"partial");

        let scan = scan(tmp.path()).unwrap();
        let names: Vec<_> = scan.items.iter().map(|i| i.entry.name.as_str()).collect();
        assert_eq!(names, vec!["report.pdf"]);
        assert_eq!(scan.skipped.len(), 4);
    }

    #[test]
    fn ignores_directories_and_symlinks() {
        let tmp = TempDir::new();
        tmp.write("keep.txt", b"x");
        std::fs::create_dir(tmp.join("subfolder")).unwrap();
        std::os::unix::fs::symlink(tmp.join("keep.txt"), tmp.join("link.txt")).unwrap();

        let scan = scan(tmp.path()).unwrap();
        let names: Vec<_> = scan.items.iter().map(|i| i.entry.name.as_str()).collect();
        assert_eq!(names, vec!["keep.txt"]);
        assert!(scan
            .skipped
            .iter()
            .any(|s| s.reason == SkipReason::Directory));
        assert!(scan.skipped.iter().any(|s| s.reason == SkipReason::Symlink));
    }

    #[test]
    fn does_not_recurse_into_subfolders() {
        let tmp = TempDir::new();
        tmp.write("top.txt", b"x");
        tmp.write("nested/deep.txt", b"x");

        let scan = scan(tmp.path()).unwrap();
        let names: Vec<_> = scan.items.iter().map(|i| i.entry.name.as_str()).collect();
        assert_eq!(names, vec!["top.txt"]);
    }

    #[test]
    fn populates_the_manifest_contract() {
        let tmp = TempDir::new();
        tmp.write("Invoice_Aug2026.pdf", b"%PDF-1.4 hello");

        let scan = scan(tmp.path()).unwrap();
        let e = &scan.items[0].entry;
        assert_eq!(e.id, "f_000");
        assert_eq!(e.name, "Invoice_Aug2026.pdf");
        assert_eq!(e.ext, "pdf");
        assert_eq!(e.size_bytes, 14);
        assert_eq!(e.mime, "application/pdf");
        assert!(e.origin.is_none(), "origin lands in Phase 2");
    }

    #[test]
    fn manifest_never_contains_a_path() {
        let tmp = TempDir::new();
        tmp.write("secret.pdf", b"x");
        let scan = scan(tmp.path()).unwrap();

        let json = serde_json::to_string(&scan.manifest()).unwrap();
        assert!(!json.contains(&tmp.path().display().to_string()));

        // `mime` legitimately contains a slash, so check the fields that carry
        // user-identifying strings rather than the blob.
        let e = &scan.items[0].entry;
        assert!(!e.name.contains('/'));
        assert_eq!(e.name, "secret.pdf");
        // The one field that could leak a directory is `name`; assert the type has no
        // other string field sourced from the path.
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = value[0].as_object().unwrap();
        for (k, v) in obj {
            let Some(s) = v.as_str() else { continue };
            if k == "mime" {
                continue;
            }
            assert!(!s.contains('/'), "field `{k}` leaked a path component: {s}");
        }
    }

    #[test]
    fn ids_are_stable_across_runs() {
        let tmp = TempDir::new();
        for n in ["c.txt", "a.txt", "b.txt"] {
            tmp.write(n, b"x");
        }
        let a = scan(tmp.path()).unwrap();
        let b = scan(tmp.path()).unwrap();
        let ids_a: Vec<_> = a
            .items
            .iter()
            .map(|i| (&i.entry.id, &i.entry.name))
            .collect();
        let ids_b: Vec<_> = b
            .items
            .iter()
            .map(|i| (&i.entry.id, &i.entry.name))
            .collect();
        assert_eq!(ids_a, ids_b);
        // ...and sorted, so f_000 is always the alphabetically first file.
        assert_eq!(a.items[0].entry.name, "a.txt");
    }

    #[test]
    fn enforces_the_batch_cap() {
        let tmp = TempDir::new();
        for n in 0..MAX_BATCH + 10 {
            tmp.write(format!("file_{n:04}.txt"), b"x");
        }
        let scan = scan(tmp.path()).unwrap();
        assert_eq!(scan.items.len(), MAX_BATCH);
        assert!(scan.truncated);
    }

    #[test]
    fn hashes_are_stable_and_content_sensitive() {
        let tmp = TempDir::new();
        let a = tmp.write("a", b"hello");
        let b = tmp.write("b", b"hello");
        let c = tmp.write("c", b"goodbye");
        assert_eq!(hash_file(&a).unwrap(), hash_file(&b).unwrap());
        assert_ne!(hash_file(&a).unwrap(), hash_file(&c).unwrap());
        // Known vector for "hello".
        assert_eq!(
            hash_file(&a).unwrap(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn refuses_to_scan_a_protected_folder() {
        assert!(scan(Path::new("/System/Library")).is_err());
    }

    #[test]
    fn scans_many_sources_with_ids_that_run_across_all_of_them() {
        let a = TempDir::new();
        let b = TempDir::new();
        a.write("one.pdf", b"x");
        a.write("two.pdf", b"x");
        b.write("three.pdf", b"x");

        let scan = scan_all(&[a.path().to_path_buf(), b.path().to_path_buf()], false).unwrap();
        let got: Vec<_> = scan
            .items
            .iter()
            .map(|i| (i.entry.id.as_str(), i.entry.name.as_str()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("f_000", "one.pdf"),
                ("f_001", "two.pdf"),
                ("f_002", "three.pdf")
            ]
        );
        // Each item knows its own folder, which is what the Move stage's lanes use.
        assert_eq!(scan.items[2].source, b.path());
    }

    #[test]
    fn a_source_listed_twice_is_scanned_once() {
        let a = TempDir::new();
        a.write("one.pdf", b"x");
        let dirs = [a.path().to_path_buf(), a.path().to_path_buf()];
        let scan = scan_all(&dirs, false).unwrap();
        assert_eq!(scan.items.len(), 1);
        assert_eq!(scan.sources.len(), 1);
    }

    #[test]
    fn the_batch_cap_spans_every_source() {
        let a = TempDir::new();
        let b = TempDir::new();
        for n in 0..MAX_BATCH - 5 {
            a.write(format!("a_{n:04}.txt"), b"x");
        }
        for n in 0..20 {
            b.write(format!("b_{n:04}.txt"), b"x");
        }
        let scan = scan_all(&[a.path().to_path_buf(), b.path().to_path_buf()], false).unwrap();
        assert_eq!(scan.items.len(), MAX_BATCH);
        assert!(scan.truncated);
    }

    #[test]
    fn hidden_files_are_opt_in_but_system_files_never_are() {
        let a = TempDir::new();
        a.write(".notes.txt", b"x");
        a.write(".DS_Store", b"x");
        a.write("seen.txt", b"x");

        let off = scan_all(&[a.path().to_path_buf()], false).unwrap();
        assert_eq!(off.items.len(), 1);

        let on = scan_all(&[a.path().to_path_buf()], true).unwrap();
        let names: Vec<_> = on.items.iter().map(|i| i.entry.name.as_str()).collect();
        assert_eq!(names, vec![".notes.txt", "seen.txt"]);
    }

    #[test]
    fn one_protected_source_refuses_the_whole_scan() {
        // Better to say so than to quietly scan the other folders and look complete.
        let a = TempDir::new();
        let dirs = [a.path().to_path_buf(), PathBuf::from("/System/Library")];
        assert!(scan_all(&dirs, false).is_err());
    }
}
