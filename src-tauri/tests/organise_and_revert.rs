//! Phase 1 acceptance test (§6):
//!
//! > organise a 200-file synthetic Downloads folder by extension, review, apply, and
//! > fully revert with zero file loss.
//!
//! "Zero file loss" is asserted the only way that means anything: every file's SHA-256
//! is recorded before the run and every one of them is accounted for, byte-identical,
//! after the apply and again after the revert.

use menlo_lib::config::{Config, Destination, Settings};
use menlo_lib::journal;
use menlo_lib::plan::{Action, ResolvedBy};
use menlo_lib::rules::{self, Rule, RuleSet, RuleSource};
use menlo_lib::{execute, scan};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// `MENLO_APP_DIR` is process-global, so tests that write config/journal state take
/// this lock rather than racing each other.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("menlo-it-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p.canonicalize().unwrap())
    }
    fn path(&self) -> &Path {
        &self.0
    }
    fn join(&self, r: impl AsRef<Path>) -> PathBuf {
        self.0.join(r)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Extensions spread across the built-in categories, plus a few that map nowhere so
/// the "needs review" path is exercised too.
const SPREAD: &[(&str, &str)] = &[
    ("pdf", "documents"),
    ("docx", "documents"),
    ("txt", "documents"),
    ("png", "images"),
    ("jpg", "images"),
    ("heic", "images"),
    ("mp4", "video"),
    ("mov", "video"),
    ("mp3", "audio"),
    ("zip", "archives"),
    ("tar", "archives"),
    ("rs", "code"),
    ("ts", "code"),
];

const CATEGORIES: &[&str] = &["documents", "images", "video", "audio", "archives", "code"];

struct Fixture {
    _tmp: TempDir,
    _app: TempDir,
    source: PathBuf,
    config: Config,
    /// basename → sha256, for every real file placed in the source folder.
    expected: BTreeMap<String, String>,
}

/// Build a synthetic Downloads folder of `n` files plus the noise a real one always has.
fn make_fixture(n: usize) -> Fixture {
    let tmp = TempDir::new();
    let app = TempDir::new();
    std::env::set_var("MENLO_APP_DIR", app.path());

    let source = tmp.join("Downloads");
    std::fs::create_dir_all(&source).unwrap();

    let mut expected = BTreeMap::new();
    for i in 0..n {
        let (ext, _) = SPREAD[i % SPREAD.len()];
        let name = format!("file_{i:04}.{ext}");
        // Unique contents so a mixed-up move shows as a hash mismatch, not a pass.
        let contents = format!("menlo synthetic payload {i} for {name}\n").repeat(1 + i % 7);
        std::fs::write(source.join(&name), contents.as_bytes()).unwrap();
        expected.insert(name.clone(), sha_of(&source.join(&name)));
    }

    // Noise the scanner is required to ignore (§6.2).
    std::fs::write(source.join(".DS_Store"), b"junk").unwrap();
    std::fs::write(source.join(".hidden_config"), b"junk").unwrap();
    std::fs::write(source.join("in_progress.mp4.crdownload"), b"partial").unwrap();
    std::fs::create_dir_all(source.join("already_sorted")).unwrap();
    std::fs::write(source.join("already_sorted/inner.pdf"), b"nested").unwrap();
    std::os::unix::fs::symlink(source.join("file_0000.pdf"), source.join("alias.pdf")).unwrap();

    let destinations: Vec<Destination> = CATEGORIES
        .iter()
        .map(|k| {
            let path = tmp.join(format!("Filed/{k}"));
            std::fs::create_dir_all(&path).unwrap();
            Destination {
                key: (*k).to_string(),
                label: k.to_string(),
                path,
                brief: String::new(),
                parent: None,
            }
        })
        .collect();

    let mut config = Config::new(vec![source.clone()], destinations);
    // lsof on 200 files makes the test crawl and proves nothing here; it has its own
    // unit coverage in `safety`.
    config.settings = Settings {
        allow_installers: false,
        check_open_files: false,
        ..Settings::default()
    };
    menlo_lib::config::save(&config).unwrap();

    Fixture {
        _tmp: tmp,
        _app: app,
        source,
        config,
        expected,
    }
}

fn sha_of(p: &Path) -> String {
    scan::hash_file(p).unwrap()
}

/// Every file under `root`, recursively, as basename → sha256.
fn collect(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for e in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !e.file_type().is_file() || e.path_is_symlink() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_string();
        out.insert(name, sha_of(e.path()));
    }
    out
}

#[test]
fn organises_two_hundred_files_and_fully_reverts() {
    let _guard = lock();
    let fx = make_fixture(200);

    // ---- scan -------------------------------------------------------------
    let scanned = scan::scan(&fx.source).unwrap();
    assert_eq!(scanned.items.len(), 200, "the noise must not be scanned");
    assert!(!scanned.truncated);

    // ---- plan (tier 1, no AI) --------------------------------------------
    let plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());
    assert_eq!(plan.entries.len(), 200);
    assert_eq!(
        plan.resolved_without_ai(),
        200,
        "every extension in the fixture maps to a category the user created"
    );
    assert!(plan
        .entries
        .iter()
        .all(|e| e.resolved_by == ResolvedBy::Deterministic));
    assert!(plan
        .entries
        .iter()
        .all(|e| e.action == Action::Move && e.included));

    // ---- apply ------------------------------------------------------------
    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();
    assert_eq!(outcome.moved, 200);
    assert!(
        outcome.failures.is_empty(),
        "failures: {:?}",
        outcome.failures
    );

    // Zero file loss: every original file is present somewhere under Filed/, with the
    // byte-identical contents it started with.
    let filed = collect(&fx.source.parent().unwrap().join("Filed"));
    assert_eq!(
        filed, fx.expected,
        "filed set must match the originals exactly"
    );

    // ...and each landed in the right destination.
    for name in fx.expected.keys() {
        let ext = name.rsplit('.').next().unwrap();
        let category = SPREAD.iter().find(|(e, _)| *e == ext).unwrap().1;
        assert!(
            fx.source
                .parent()
                .unwrap()
                .join("Filed")
                .join(category)
                .join(name)
                .exists(),
            "{name} should be in {category}"
        );
    }

    // The noise is untouched.
    assert!(fx.source.join(".DS_Store").exists());
    assert!(fx.source.join("in_progress.mp4.crdownload").exists());
    assert!(fx.source.join("already_sorted/inner.pdf").exists());
    assert_eq!(
        std::fs::read_dir(&fx.source).unwrap().count(),
        5,
        "only the five ignored entries should remain in the source"
    );

    // ---- revert -----------------------------------------------------------
    let reverted = journal::revert_batch(&outcome.batch_id).unwrap();
    assert_eq!(reverted.restored, 200);
    assert!(
        reverted.failed.is_empty(),
        "revert failures: {:?}",
        reverted.failed
    );

    // Everything is back where it started, byte-identical.
    let mut back = collect(&fx.source);
    // Drop the noise we planted; it was never part of the batch.
    for noise in [
        ".DS_Store",
        ".hidden_config",
        "in_progress.mp4.crdownload",
        "inner.pdf",
    ] {
        back.remove(noise);
    }
    assert_eq!(
        back, fx.expected,
        "revert must restore the originals exactly"
    );

    for category in CATEGORIES {
        let dir = fx.source.parent().unwrap().join("Filed").join(category);
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            0,
            "{category} should be empty after revert"
        );
    }

    // The journal agrees.
    let batch = journal::read_batch(&outcome.batch_id).unwrap();
    assert!(batch.fully_reverted());
    assert_eq!(batch.moves.len(), 200);
    assert_eq!(batch.reverts.len(), 200);
}

#[test]
fn collisions_get_a_suffix_and_never_overwrite() {
    let _guard = lock();
    let fx = make_fixture(3);

    // Plant a different file with a name the batch is about to use.
    let docs = fx.config.destination("documents").unwrap().path.clone();
    std::fs::write(docs.join("file_0000.pdf"), b"PRE-EXISTING, MUST SURVIVE").unwrap();

    let scanned = scan::scan(&fx.source).unwrap();
    let plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());
    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();
    assert_eq!(outcome.moved, 3);

    assert_eq!(
        std::fs::read(docs.join("file_0000.pdf")).unwrap(),
        b"PRE-EXISTING, MUST SURVIVE",
        "an existing file must never be overwritten"
    );
    assert!(
        docs.join("file_0000 (2).pdf").exists(),
        "the incoming file gets a suffix"
    );

    let batch = journal::read_batch(&outcome.batch_id).unwrap();
    assert!(batch.moves.iter().any(|m| m.renamed_for_collision));

    // Revert puts the suffixed file back under its original name.
    let r = journal::revert_batch(&outcome.batch_id).unwrap();
    assert_eq!(r.restored, 3);
    assert!(fx.source.join("file_0000.pdf").exists());
    assert_eq!(
        std::fs::read(docs.join("file_0000.pdf")).unwrap(),
        b"PRE-EXISTING, MUST SURVIVE"
    );
}

#[test]
fn revert_refuses_to_clobber_a_file_edited_since_the_move() {
    let _guard = lock();
    let fx = make_fixture(2);

    let scanned = scan::scan(&fx.source).unwrap();
    let plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());
    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();

    // Simulate the user editing a filed document afterwards.
    let batch = journal::read_batch(&outcome.batch_id).unwrap();
    let edited = batch.moves[0].to.clone();
    std::fs::write(&edited, b"the user has since rewritten this").unwrap();

    let r = journal::revert_batch(&outcome.batch_id).unwrap();
    assert_eq!(
        r.failed.len(),
        1,
        "the changed file must be refused, not restored"
    );
    assert!(r.failed[0].error.contains("changed"));
    assert!(edited.exists(), "the newer version stays where it is");
    assert_eq!(r.restored, batch.moves.len() - 1, "the rest still revert");
}

#[test]
fn excluded_entries_are_not_moved() {
    let _guard = lock();
    let fx = make_fixture(4);

    let scanned = scan::scan(&fx.source).unwrap();
    let mut plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());
    plan.entries[0].included = false;
    plan.entries[1].included = false;
    let kept: Vec<String> = plan.entries[..2].iter().map(|e| e.name.clone()).collect();

    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();
    assert_eq!(outcome.moved, 2);
    for name in kept {
        assert!(
            fx.source.join(&name).exists(),
            "{name} was excluded and must stay put"
        );
    }
}

#[test]
fn a_learned_rule_survives_to_the_next_run() {
    let _guard = lock();
    let fx = make_fixture(0);

    // A file whose extension the built-in map does not cover.
    std::fs::write(fx.source.join("statement_jan.qfx"), b"bank data").unwrap();
    std::fs::write(fx.source.join("statement_feb.qfx"), b"more bank data").unwrap();

    let scanned = scan::scan(&fx.source).unwrap();
    let mut plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());
    assert!(plan.entries.iter().all(|e| e.action == Action::NeedsReview));

    // Stand in for the Phase 2 LLM pass: approve one with a suggested rule.
    plan.entries[0].action = Action::Move;
    plan.entries[0].destination_key = Some("documents".into());
    plan.entries[0].included = true;
    plan.entries[0].suggest_rule = Some(menlo_lib::plan::SuggestRule {
        match_expr: "ext:qfx".into(),
        destination_key: "documents".into(),
    });

    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();
    assert_eq!(outcome.moved, 1);
    assert_eq!(
        outcome.learned.len(),
        1,
        "the approval should crystallise into a rule"
    );

    // Second run: the remaining file is now resolved with no AI at all.
    let scanned2 = scan::scan(&fx.source).unwrap();
    let ruleset = rules::load_rules().unwrap();
    let plan2 = rules::build_plan(&scanned2, &fx.config, &ruleset);
    assert_eq!(plan2.entries.len(), 1);
    assert_eq!(plan2.entries[0].action, Action::Move);
    assert_eq!(plan2.entries[0].resolved_by, ResolvedBy::Learned);
    assert_eq!(plan2.resolved_without_ai(), 1);
}

#[test]
fn refuses_to_move_outside_a_configured_destination() {
    let _guard = lock();
    let mut fx = make_fixture(1);

    // Point a destination at somewhere the allowlist will reject after the plan is
    // built, mimicking a config edited between review and apply.
    let outside = fx.source.parent().unwrap().join("Outside");
    std::fs::create_dir_all(&outside).unwrap();

    let scanned = scan::scan(&fx.source).unwrap();
    let plan = rules::build_plan(&scanned, &fx.config, &RuleSet::default());

    fx.config.destinations.retain(|d| d.key != "documents");
    let outcome = execute::execute(&plan, &scanned, &fx.config).unwrap();

    assert_eq!(outcome.moved, 0);
    assert_eq!(outcome.failures.len(), 1);
    assert!(outcome.failures[0].error.contains("no longer exists"));
    assert!(
        fx.source.join("file_0000.pdf").exists(),
        "the file must not have moved"
    );
}

#[test]
fn user_rules_beat_the_builtin_map_end_to_end() {
    let _guard = lock();
    let fx = make_fixture(0);
    std::fs::write(fx.source.join("Invoice_Aug2026.pdf"), b"tax invoice").unwrap();
    std::fs::write(fx.source.join("Notes.pdf"), b"just notes").unwrap();

    let mut set = RuleSet::default();
    set.insert(Rule::new(
        "glob:Invoice_*.pdf",
        "archives",
        RuleSource::User,
    ));
    rules::save_rules(&set).unwrap();

    let scanned = scan::scan(&fx.source).unwrap();
    let plan = rules::build_plan(&scanned, &fx.config, &rules::load_rules().unwrap());
    execute::execute(&plan, &scanned, &fx.config).unwrap();

    let base = fx.source.parent().unwrap().join("Filed");
    assert!(
        base.join("archives/Invoice_Aug2026.pdf").exists(),
        "the user rule wins"
    );
    assert!(
        base.join("documents/Notes.pdf").exists(),
        "the builtin still handles the rest"
    );
}
