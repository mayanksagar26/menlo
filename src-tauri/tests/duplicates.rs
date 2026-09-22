//! Duplicates, end to end, on real files.
//!
//! This is the one path through Menlo that can make a file disappear from where the
//! user left it, so it is tested the way the organise-and-revert test is: by content
//! hash, before and after, asserting that nothing's bytes are lost — through the apply
//! and again through the revert that brings the Trash back.

use menlo_lib::config::{Config, Destination, DuplicatePolicy, Settings};
use menlo_lib::execute::{self, StepOutcome};
use menlo_lib::plan::{DuplicateChoice, DuplicateKind, Plan};
use menlo_lib::rules::{build_plan, RuleSet};
use menlo_lib::{dupes, journal, scan};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// `MENLO_APP_DIR` and `MENLO_TRASH_DIR` are process-global.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("menlo-dup-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p.canonicalize().unwrap())
    }
    fn join(&self, r: impl AsRef<Path>) -> PathBuf {
        self.0.join(r)
    }
    fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let p = self.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, bytes).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct World {
    tmp: TempDir,
    config: Config,
    scan: scan::Scan,
}

impl World {
    /// Downloads and Desktop as sources; Filed/documents and Filed/images as
    /// destinations; a private Trash; a private app directory.
    fn new(policy: DuplicatePolicy) -> Self {
        let tmp = TempDir::new();
        std::env::set_var("MENLO_APP_DIR", tmp.join("app"));
        std::env::set_var("MENLO_TRASH_DIR", tmp.join("Trash"));
        for d in [
            "Downloads",
            "Desktop",
            "Filed/documents",
            "Filed/images",
            "Trash",
        ] {
            std::fs::create_dir_all(tmp.join(d)).unwrap();
        }

        let dest = |key: &str| Destination {
            key: key.into(),
            label: key.into(),
            path: tmp.join(format!("Filed/{key}")),
            brief: String::new(),
            parent: None,
        };
        let mut config = Config::new(
            vec![tmp.join("Downloads"), tmp.join("Desktop")],
            vec![dest("documents"), dest("images")],
        );
        config.settings = Settings {
            check_open_files: false,
            duplicates: policy,
            ..Settings::default()
        };

        World {
            tmp,
            config,
            scan: scan::scan_all(&[], false).unwrap(),
        }
    }

    fn rescan(&mut self) {
        self.scan = scan::scan_all(&self.config.sources, false).unwrap();
    }

    fn plan(&self) -> Plan {
        let mut plan = build_plan(&self.scan, &self.config, &RuleSet::default());
        dupes::detect(&mut plan, &self.scan, &self.config).unwrap();
        plan
    }

    /// The hash of every file under the world's root, as a set. Contents that exist
    /// anywhere — a source, a destination, the Trash — are not lost.
    fn contents(&self) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for root in ["Downloads", "Desktop", "Filed", "Trash"] {
            for e in walk(&self.tmp.join(root)) {
                out.insert(scan::hash_file(&e).unwrap());
            }
        }
        out
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}

fn choose(plan: &mut Plan, name: &str, choice: Option<DuplicateChoice>) {
    let e = plan.entries.iter_mut().find(|e| e.name == name).unwrap();
    e.on_duplicate = choice;
}

fn kind(plan: &Plan, name: &str) -> Option<DuplicateKind> {
    plan.entries
        .iter()
        .find(|e| e.name == name)
        .and_then(|e| e.duplicate.as_ref())
        .map(|d| d.kind)
}

#[test]
fn every_duplicate_choice_applies_and_reverts_with_nothing_lost() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Ask);

    // Already filed, byte for byte, under the same name.
    w.tmp.write("Filed/documents/Invoice.pdf", b"invoice bytes");
    w.tmp.write("Downloads/Invoice.pdf", b"invoice bytes");
    // Already filed under another name.
    w.tmp.write("Downloads/Invoice (1).pdf", b"invoice bytes");
    // Same name, different file.
    w.tmp
        .write("Filed/documents/report.pdf", b"last year's report");
    w.tmp.write("Downloads/report.pdf", b"this year's report!");
    // No relation to anything.
    w.tmp.write("Downloads/notes.pdf", b"notes");
    // The same screenshot in both sources.
    w.tmp.write("Downloads/shot copy.png", b"pixels");
    w.tmp.write("Desktop/shot.png", b"pixels");
    w.rescan();

    let before = w.contents();

    let mut plan = w.plan();
    assert_eq!(kind(&plan, "Invoice.pdf"), Some(DuplicateKind::Identical));
    assert_eq!(
        kind(&plan, "Invoice (1).pdf"),
        Some(DuplicateKind::Identical)
    );
    assert_eq!(kind(&plan, "report.pdf"), Some(DuplicateKind::SameName));
    assert_eq!(kind(&plan, "notes.pdf"), None);
    assert_eq!(kind(&plan, "shot copy.png"), None, "first copy is kept");
    assert_eq!(kind(&plan, "shot.png"), Some(DuplicateKind::Identical));

    choose(&mut plan, "Invoice.pdf", Some(DuplicateChoice::Trash));
    // Left unanswered under "Ask": must fall back to keeping it.
    choose(&mut plan, "Invoice (1).pdf", None);
    choose(&mut plan, "shot.png", Some(DuplicateChoice::Trash));

    let mut ticks = Vec::new();
    let out = execute::execute_with(
        &plan,
        &w.scan,
        &w.config,
        Some("Everyday".into()),
        || false,
        |p| ticks.push(p.outcome),
    )
    .unwrap();

    assert!(out.failures.is_empty(), "{:?}", out.failures);
    assert_eq!((out.moved, out.trashed, out.kept), (3, 2, 1));
    assert_eq!(ticks.len(), 6, "one progress tick per file");
    assert_eq!(
        ticks.iter().filter(|t| **t == StepOutcome::Trashed).count(),
        2
    );

    // Where everything is now.
    assert!(w.tmp.join("Trash/Invoice.pdf").exists());
    assert!(!w.tmp.join("Downloads/Invoice.pdf").exists());
    assert!(
        w.tmp.join("Downloads/Invoice (1).pdf").exists(),
        "kept means kept"
    );
    assert!(w.tmp.join("Filed/documents/report.pdf").exists());
    assert!(
        w.tmp.join("Filed/documents/report (2).pdf").exists(),
        "never overwritten"
    );
    assert!(w.tmp.join("Filed/images/shot copy.png").exists());
    assert!(w.tmp.join("Trash/shot.png").exists());

    // Nothing's contents were lost — only a redundant copy went to the Trash.
    assert_eq!(w.contents(), before);

    // The Runs page sees this run under its set, with the three kinds counted apart.
    let summary = journal::list_batches()
        .unwrap()
        .into_iter()
        .find(|b| b.batch_id == plan.batch_id)
        .unwrap();
    assert_eq!(summary.set_label.as_deref(), Some("Everyday"));
    assert_eq!((summary.moved, summary.trashed, summary.kept), (3, 2, 1));

    // And a full revert brings the Trash back too.
    let back = journal::revert_batch(&plan.batch_id).unwrap();
    assert!(back.failed.is_empty(), "{:?}", back.failed);
    assert_eq!(back.restored, 5);
    for p in [
        "Downloads/Invoice.pdf",
        "Downloads/report.pdf",
        "Downloads/notes.pdf",
        "Downloads/shot copy.png",
        "Desktop/shot.png",
    ] {
        assert!(w.tmp.join(p).exists(), "{p} should be back");
    }
    assert!(
        walk(&w.tmp.join("Trash")).is_empty(),
        "the Trash is emptied back out"
    );
    assert_eq!(w.contents(), before);
}

#[test]
fn a_duplicate_that_changed_since_planning_is_never_trashed() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Ask);
    w.tmp.write("Filed/documents/Invoice.pdf", b"invoice bytes");
    w.tmp.write("Downloads/Invoice.pdf", b"invoice bytes");
    w.rescan();

    let mut plan = w.plan();
    choose(&mut plan, "Invoice.pdf", Some(DuplicateChoice::Trash));

    // Between the preview and the click, someone edits the filed copy.
    std::fs::write(
        w.tmp.join("Filed/documents/Invoice.pdf"),
        b"annotated since",
    )
    .unwrap();

    let out = execute::execute(&plan, &w.scan, &w.config).unwrap();
    assert_eq!(out.trashed, 0);
    assert_eq!(out.failures.len(), 1);
    assert!(w.tmp.join("Downloads/Invoice.pdf").exists());
    assert!(walk(&w.tmp.join("Trash")).is_empty());
}

#[test]
fn a_name_clash_can_never_be_trashed() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Trash);
    w.tmp.write("Filed/documents/report.pdf", b"old");
    w.tmp.write("Downloads/report.pdf", b"new and different");
    w.rescan();

    let mut plan = w.plan();
    // Even asked for explicitly — say, by a tampered plan from the frontend.
    choose(&mut plan, "report.pdf", Some(DuplicateChoice::Trash));

    let out = execute::execute(&plan, &w.scan, &w.config).unwrap();
    assert_eq!(out.trashed, 0);
    assert!(w.tmp.join("Downloads/report.pdf").exists());
    assert!(walk(&w.tmp.join("Trash")).is_empty());
}

#[test]
fn the_trash_setting_applies_when_the_user_was_not_asked() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Trash);
    w.tmp.write("Filed/documents/Invoice.pdf", b"invoice bytes");
    w.tmp.write("Downloads/Invoice.pdf", b"invoice bytes");
    w.rescan();

    let out = execute::execute(&w.plan(), &w.scan, &w.config).unwrap();
    assert_eq!(out.trashed, 1);
    assert!(w.tmp.join("Trash/Invoice.pdf").exists());
}

#[test]
fn stop_leaves_everything_after_it_untouched() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Ask);
    for n in 0..5 {
        w.tmp.write(
            &format!("Downloads/file_{n}.pdf"),
            format!("doc {n}").as_bytes(),
        );
    }
    w.rescan();

    let plan = w.plan();
    let done = std::cell::Cell::new(0);
    let out = execute::execute_with(
        &plan,
        &w.scan,
        &w.config,
        None,
        || done.get() >= 2,
        |_| done.set(done.get() + 1),
    )
    .unwrap();

    assert!(out.stopped);
    assert_eq!(out.moved, 2);
    assert_eq!(
        walk(&w.tmp.join("Downloads")).len(),
        3,
        "the rest stayed put"
    );
    // What did land is a normal, restorable run.
    assert_eq!(journal::revert_batch(&plan.batch_id).unwrap().restored, 2);
    assert_eq!(walk(&w.tmp.join("Downloads")).len(), 5);
}

#[test]
fn one_destination_of_a_run_can_be_restored_alone() {
    let _g = lock();
    let mut w = World::new(DuplicatePolicy::Ask);
    w.tmp.write("Downloads/a.pdf", b"doc");
    w.tmp.write("Downloads/b.png", b"img");
    w.rescan();

    let plan = w.plan();
    execute::execute(&plan, &w.scan, &w.config).unwrap();

    let images = w.tmp.join("Filed/images");
    let back = journal::revert_batch_where(&plan.batch_id, |m| m.to.starts_with(&images)).unwrap();
    assert_eq!(back.restored, 1);
    assert!(w.tmp.join("Downloads/b.png").exists());
    assert!(
        w.tmp.join("Filed/documents/a.pdf").exists(),
        "documents untouched"
    );
}
