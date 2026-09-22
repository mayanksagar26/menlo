//! Duplicates: what is already sitting where a file is about to go.
//!
//! Three ways a file can be a duplicate, cheapest check first:
//!
//!   1. **By name.** A file with the same name is already in the destination. Same
//!      size and same SHA-256 makes it [`DuplicateKind::Identical`]; anything else is
//!      [`DuplicateKind::SameName`] — a different file that happens to share a name.
//!   2. **By contents.** No name clash, but a file in the destination has the same
//!      bytes under another name. Only files of exactly the same size are hashed, so
//!      a folder of large videos is not read end to end to check one PDF.
//!   3. **Within the batch.** The same screenshot in both Downloads and Desktop. The
//!      first copy is kept; later ones point back at it.
//!
//! Only `Identical` may ever be sent to the Trash. A name match alone is not evidence
//! two files are the same, and neither is a size match.
//!
//! This reads files and never changes them. It runs when a plan is built and again,
//! from scratch, when one is applied — the copy of these findings that comes back from
//! the frontend is never trusted.

use crate::config::Config;
use crate::error::Result;
use crate::plan::{Duplicate, DuplicateKind, Plan};
use crate::scan::{hash_file, Scan};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Hashes are the expensive part and the same file is asked about more than once.
#[derive(Default)]
struct Hashes(HashMap<PathBuf, Option<String>>);

impl Hashes {
    fn of(&mut self, path: &Path) -> Option<String> {
        self.0
            .entry(path.to_path_buf())
            .or_insert_with(|| hash_file(path).ok())
            .clone()
    }
}

/// A destination folder's visible files and their sizes, listed once per plan.
fn listing(dir: &Path) -> Vec<(String, u64)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, u64)> = read
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            let name = e.file_name().to_string_lossy().to_string();
            (!name.starts_with('.')).then_some((name, meta.len()))
        })
        .collect();
    out.sort();
    out
}

/// Annotate every entry that has a destination with what it collides with. Clears
/// any previous finding first, so this is safe to run twice.
pub fn detect(plan: &mut Plan, scan: &Scan, config: &Config) -> Result<()> {
    let mut hashes = Hashes::default();
    let mut listings: HashMap<String, Vec<(String, u64)>> = HashMap::new();

    for entry in plan.entries.iter_mut() {
        entry.duplicate = None;
    }

    // Sizes shared by more than one scanned file. Only those are worth hashing for
    // the within-batch check.
    let mut size_count: HashMap<u64, usize> = HashMap::new();
    for item in &scan.items {
        *size_count.entry(item.entry.size_bytes).or_default() += 1;
    }
    // hash → id of the first file in the batch with that content.
    let mut first_seen: HashMap<String, (String, String)> = HashMap::new();

    for entry in plan.entries.iter_mut() {
        let Some(key) = entry.destination_key.as_deref() else {
            continue;
        };
        let Some(dest) = config.destination(key) else {
            continue;
        };
        let Some(item) = scan.get(&entry.id) else {
            continue;
        };
        let filename = entry
            .rename_to
            .clone()
            .unwrap_or_else(|| entry.name.clone());
        let files = listings
            .entry(key.to_string())
            .or_insert_with(|| listing(&dest.path));

        // 1. By name.
        if let Some((existing, size)) = files.iter().find(|(n, _)| *n == filename) {
            let identical = *size == item.entry.size_bytes
                && hashes.of(&item.path).is_some()
                && hashes.of(&item.path) == hashes.of(&dest.path.join(existing));
            entry.duplicate = Some(Duplicate {
                kind: if identical {
                    DuplicateKind::Identical
                } else {
                    DuplicateKind::SameName
                },
                existing_name: existing.clone(),
                same_as_entry: None,
            });
            continue;
        }

        // 2. By contents, under another name.
        let same_size: Vec<String> = files
            .iter()
            .filter(|(_, s)| *s == item.entry.size_bytes)
            .map(|(n, _)| n.clone())
            .collect();
        if !same_size.is_empty() {
            if let Some(src_hash) = hashes.of(&item.path) {
                if let Some(existing) = same_size
                    .into_iter()
                    .find(|n| hashes.of(&dest.path.join(n)).as_deref() == Some(src_hash.as_str()))
                {
                    entry.duplicate = Some(Duplicate {
                        kind: DuplicateKind::Identical,
                        existing_name: existing,
                        same_as_entry: None,
                    });
                    continue;
                }
            }
        }

        // 3. Within the batch.
        if size_count.get(&item.entry.size_bytes).copied().unwrap_or(0) > 1 {
            if let Some(h) = hashes.of(&item.path) {
                match first_seen.get(&h) {
                    Some((first_id, first_name)) => {
                        entry.duplicate = Some(Duplicate {
                            kind: DuplicateKind::Identical,
                            existing_name: first_name.clone(),
                            same_as_entry: Some(first_id.clone()),
                        });
                    }
                    None => {
                        first_seen.insert(h, (entry.id.clone(), entry.name.clone()));
                    }
                }
            }
        }
    }

    Ok(())
}

/// Whether a file about to be trashed is still byte-identical to the copy it
/// duplicates. Called immediately before the Trash move, not when the plan was
/// built: either file may have changed in between, and a stale "identical" must
/// never be what deletes something.
pub fn still_identical(source: &Path, existing: &Path) -> bool {
    match (hash_file(source), hash_file(existing)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Destination;
    use crate::rules::{build_plan, RuleSet};
    use crate::testutil::TempDir;

    struct Fx {
        _tmp: TempDir,
        scan: Scan,
        config: Config,
        docs: PathBuf,
    }

    /// Sources a/ and b/, destination docs/ (key `documents`).
    fn fx(a: &[(&str, &[u8])], b: &[(&str, &[u8])], docs: &[(&str, &[u8])]) -> Fx {
        let tmp = TempDir::new();
        for (n, bytes) in a {
            tmp.write(format!("a/{n}"), bytes);
        }
        for (n, bytes) in b {
            tmp.write(format!("b/{n}"), bytes);
        }
        std::fs::create_dir_all(tmp.join("a")).unwrap();
        std::fs::create_dir_all(tmp.join("b")).unwrap();
        std::fs::create_dir_all(tmp.join("docs")).unwrap();
        for (n, bytes) in docs {
            tmp.write(format!("docs/{n}"), bytes);
        }
        let docs_path = tmp.join("docs");
        let config = Config::new(
            vec![tmp.join("a"), tmp.join("b")],
            vec![Destination {
                key: "documents".into(),
                label: "Documents".into(),
                path: docs_path.clone(),
                brief: String::new(),
                parent: None,
            }],
        );
        let scan = crate::scan::scan_all(&config.sources, false).unwrap();
        Fx {
            _tmp: tmp,
            scan,
            config,
            docs: docs_path,
        }
    }

    fn planned(f: &Fx) -> Plan {
        let mut plan = build_plan(&f.scan, &f.config, &RuleSet::default());
        detect(&mut plan, &f.scan, &f.config).unwrap();
        plan
    }

    fn dup<'a>(plan: &'a Plan, name: &str) -> Option<&'a Duplicate> {
        plan.entries
            .iter()
            .filter(|e| e.name == name)
            .find_map(|e| e.duplicate.as_ref())
    }

    #[test]
    fn same_name_same_bytes_is_identical() {
        let f = fx(&[("report.pdf", b"same")], &[], &[("report.pdf", b"same")]);
        let plan = planned(&f);
        let d = dup(&plan, "report.pdf").unwrap();
        assert_eq!(d.kind, DuplicateKind::Identical);
        assert_eq!(d.existing_name, "report.pdf");
    }

    #[test]
    fn same_name_different_bytes_is_only_a_name_clash() {
        let f = fx(
            &[("report.pdf", b"new draft")],
            &[],
            &[("report.pdf", b"old")],
        );
        assert_eq!(
            dup(&planned(&f), "report.pdf").unwrap().kind,
            DuplicateKind::SameName
        );
    }

    #[test]
    fn same_name_same_size_different_bytes_is_still_only_a_name_clash() {
        // Size alone must never be taken as proof.
        let f = fx(&[("report.pdf", b"aaaa")], &[], &[("report.pdf", b"bbbb")]);
        assert_eq!(
            dup(&planned(&f), "report.pdf").unwrap().kind,
            DuplicateKind::SameName
        );
    }

    #[test]
    fn same_bytes_under_another_name_is_identical() {
        let f = fx(
            &[("invoice (1).pdf", b"%PDF-1")],
            &[],
            &[("Invoice.pdf", b"%PDF-1")],
        );
        let plan = planned(&f);
        let d = dup(&plan, "invoice (1).pdf").unwrap();
        assert_eq!(d.kind, DuplicateKind::Identical);
        assert_eq!(d.existing_name, "Invoice.pdf");
    }

    #[test]
    fn the_same_file_in_two_sources_keeps_the_first() {
        let f = fx(
            &[("shot.pdf", b"pixels")],
            &[("shot copy.pdf", b"pixels")],
            &[],
        );
        let plan = planned(&f);
        assert!(
            dup(&plan, "shot.pdf").is_none(),
            "the first copy is the one kept"
        );
        let second = dup(&plan, "shot copy.pdf").unwrap();
        assert_eq!(second.kind, DuplicateKind::Identical);
        assert_eq!(second.same_as_entry.as_deref(), Some("f_000"));
    }

    #[test]
    fn an_unrelated_file_is_not_a_duplicate() {
        let f = fx(&[("a.pdf", b"one")], &[], &[("b.pdf", b"two")]);
        assert!(dup(&planned(&f), "a.pdf").is_none());
    }

    #[test]
    fn detection_can_run_twice() {
        let f = fx(&[("report.pdf", b"same")], &[], &[("report.pdf", b"same")]);
        let mut plan = planned(&f);
        detect(&mut plan, &f.scan, &f.config).unwrap();
        assert_eq!(
            dup(&plan, "report.pdf").unwrap().kind,
            DuplicateKind::Identical
        );
    }

    #[test]
    fn still_identical_rechecks_the_bytes() {
        let f = fx(&[("report.pdf", b"same")], &[], &[("report.pdf", b"same")]);
        let src = f.scan.items[0].path.clone();
        let existing = f.docs.join("report.pdf");
        assert!(still_identical(&src, &existing));
        std::fs::write(&existing, b"edited since").unwrap();
        assert!(!still_identical(&src, &existing));
    }
}
