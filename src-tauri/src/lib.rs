//! Menlo — local-first file organiser.
//!
//! The command layer. Note what is *not* here: no command accepts a filesystem path
//! for a plan's destination. The frontend works in destination *keys* and the app
//! maps them (§4.2), which is what keeps path injection structurally impossible. The
//! commands that do take a path — adding a source or a destination — are the user
//! pointing at a folder with the picker, and each is vetted by `safety` before it is
//! saved.

pub mod config;
pub mod dupes;
pub mod error;
pub mod execute;
pub mod journal;
pub mod plan;
pub mod prose;
pub mod rename;
pub mod rules;
pub mod safety;
pub mod scan;
pub mod testutil;
pub mod view;

use config::{Config, Destination, FolderSet, Settings};
use error::{Error, Result};
use plan::Plan;
use scan::Scan;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use view::{AppView, FolderSuggestion, RunView};

/// The scan behind the plan currently on screen.
///
/// It lives in Rust because it holds real paths, which never cross to the frontend.
/// The frontend refers to files by manifest id alone.
#[derive(Default)]
pub struct AppState {
    scan: Option<Scan>,
    /// The folder set the plan was made for, stamped on the run for the Runs page.
    set_label: Option<String>,
    /// Raised by Stop, read by the move between files.
    stop: Arc<AtomicBool>,
}

pub struct Menlo(pub Mutex<AppState>);

impl Menlo {
    fn new() -> Self {
        Menlo(Mutex::new(AppState::default()))
    }
}

/// A poisoned mutex means another command panicked mid-mutation. Recover the guard
/// rather than propagating the panic — the state it protects is rebuilt on the next
/// scan anyway.
fn state(menlo: &Menlo) -> std::sync::MutexGuard<'_, AppState> {
    menlo.0.lock().unwrap_or_else(|e| e.into_inner())
}

/// Load the config, treating a missing or blank one as a first launch and seeding it
/// with the folders that actually exist on this Mac.
fn load_or_seed() -> Result<Config> {
    let config = config::load()?;
    let blank = config.sources.is_empty()
        && config.destinations.is_empty()
        && config.folder_sets.is_empty();
    if config::exists()? && !blank {
        return Ok(config);
    }
    let mut seeded = config::first_run_defaults(&config::home()?);
    // A blank config may still carry settings the user chose; keep them.
    seeded.settings = config.settings;
    config::save(&seeded)?;
    Ok(seeded)
}

/// Validate, save, and hand back the fresh view — the shape every mutating command
/// returns, so the window never has to guess what changed.
fn commit(config: &Config) -> Result<AppView> {
    config::save(config)?;
    view::app_view(config, &rules::load_rules()?)
}

fn canonical_folder(path: PathBuf) -> Result<PathBuf> {
    let path = path.canonicalize().unwrap_or(path);
    safety::assert_operable(&path)?;
    Ok(path)
}

// ── State ────────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_state() -> Result<AppView> {
    view::app_view(&load_or_seed()?, &rules::load_rules()?)
}

#[tauri::command]
fn save_settings(settings: Settings) -> Result<AppView> {
    let mut config = config::load()?;
    config.settings = settings;
    commit(&config)
}

// ── Sources ──────────────────────────────────────────────────────────────────

#[tauri::command]
fn add_source(path: PathBuf) -> Result<AppView> {
    let path = canonical_folder(path)?;
    if !path.is_dir() {
        return Err(Error::config(format!("{} is not a folder", path.display())));
    }
    let mut config = config::load()?;
    if !config.sources.contains(&path) {
        config.sources.push(path);
    }
    commit(&config)
}

#[tauri::command]
fn remove_source(path: PathBuf) -> Result<AppView> {
    let mut config = config::load()?;
    config.sources.retain(|p| *p != path);
    for set in &mut config.folder_sets {
        set.sources.retain(|p| *p != path);
    }
    commit(&config)
}

// ── Destinations ─────────────────────────────────────────────────────────────

/// Add a folder the user picked, or one Menlo suggested. `key` is a preference — a
/// suggested Movies asks for `video` so the extension map files into it — and is
/// used only if free.
#[tauri::command]
fn add_destination(
    path: PathBuf,
    parent_key: Option<String>,
    key: Option<String>,
) -> Result<AppView> {
    let mut config = config::load()?;
    let path = canonical_folder(path)?;
    if let Some(pk) = &parent_key {
        if config.destination(pk).is_none() {
            return Err(Error::config(format!("no destination named `{pk}`")));
        }
    }
    if config.destinations.iter().any(|d| d.path == path) {
        return view::app_view(&config, &rules::load_rules()?);
    }
    std::fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;

    let label = view::label_of(&path);
    let key = key
        .filter(|k| !k.is_empty() && config.destination(k).is_none())
        .unwrap_or_else(|| config.fresh_key(&label, parent_key.as_deref()));
    config.destinations.push(Destination {
        key,
        label,
        path,
        brief: String::new(),
        parent: parent_key,
    });
    commit(&config)
}

/// Make a new subfolder by name inside a destination, and add it.
#[tauri::command]
fn create_subfolder(parent_key: String, name: String) -> Result<AppView> {
    let config = config::load()?;
    let parent = config
        .destination(&parent_key)
        .ok_or_else(|| Error::config(format!("no destination named `{parent_key}`")))?;
    let name = name.trim();
    plan::validate_rename(name).map_err(|why| Error::config(format!("folder name: {why}")))?;
    let path = parent.path.join(name);
    add_destination(path, Some(parent_key), None)
}

/// Stop using a folder. Its subfolders and their rules go with it; the folders
/// themselves, and everything in them, stay exactly where they are on disk.
#[tauri::command]
fn remove_destination(key: String) -> Result<AppView> {
    let mut config = config::load()?;
    let gone: Vec<String> = config
        .destinations
        .iter()
        .filter(|d| d.key == key || d.parent.as_deref() == Some(key.as_str()))
        .map(|d| d.key.clone())
        .collect();
    config.destinations.retain(|d| !gone.contains(&d.key));
    for set in &mut config.folder_sets {
        set.destinations.retain(|k| !gone.contains(k));
    }

    let mut ruleset = rules::load_rules()?;
    ruleset
        .folder_rules
        .retain(|r| !gone.contains(&r.destination_key));
    rules::save_rules(&ruleset)?;
    commit(&config)
}

#[tauri::command]
fn suggest_folders(parent_key: Option<String>) -> Result<Vec<FolderSuggestion>> {
    view::suggest_folders(&config::load()?, parent_key.as_deref())
}

// ── Folder sets ──────────────────────────────────────────────────────────────

/// Create a set (empty `id`) or replace one.
#[tauri::command]
fn save_folder_set(mut set: FolderSet) -> Result<AppView> {
    let mut config = config::load()?;
    for s in &set.sources {
        if !config.sources.contains(s) {
            return Err(Error::config(format!(
                "{} is not one of your source folders",
                s.display()
            )));
        }
    }
    if set.label.trim().is_empty() {
        set.label = "New set".into();
    }
    if set.id.is_empty() {
        set.id = uuid::Uuid::new_v4().to_string();
        config.folder_sets.push(set);
    } else if let Some(existing) = config.folder_sets.iter_mut().find(|s| s.id == set.id) {
        *existing = set;
    } else {
        config.folder_sets.push(set);
    }
    commit(&config)
}

#[tauri::command]
fn delete_folder_set(id: String) -> Result<AppView> {
    let mut config = config::load()?;
    config.folder_sets.retain(|s| s.id != id);
    commit(&config)
}

// ── Rules ────────────────────────────────────────────────────────────────────

#[tauri::command]
fn add_folder_rule(destination_key: String, text: String) -> Result<AppView> {
    let config = config::load()?;
    if config.destination(&destination_key).is_none() {
        return Err(Error::config(format!(
            "no destination named `{destination_key}`"
        )));
    }
    let mut ruleset = rules::load_rules()?;
    ruleset.add_folder_rule(&destination_key, &text);
    rules::save_rules(&ruleset)?;
    view::app_view(&config, &ruleset)
}

#[tauri::command]
fn remove_folder_rule(id: String) -> Result<AppView> {
    let mut ruleset = rules::load_rules()?;
    ruleset.remove_folder_rule(&id);
    rules::save_rules(&ruleset)?;
    view::app_view(&config::load()?, &ruleset)
}

// ── A run ────────────────────────────────────────────────────────────────────

/// Scan the chosen sources and work out where everything goes. Never touches a file.
///
/// Off the main thread: hashing for the duplicate check can take a while on a folder
/// of large videos, and a synchronous command would freeze the window meanwhile.
#[tauri::command]
async fn plan_run(
    sources: Vec<PathBuf>,
    destinations: Vec<String>,
    set_label: Option<String>,
    menlo: tauri::State<'_, Menlo>,
) -> Result<Plan> {
    let config = config::load()?;
    // Only folders the user has added. The window cannot make Menlo scan somewhere
    // new; that goes through the picker and `add_source` first.
    if let Some(stranger) = sources.iter().find(|s| !config.sources.contains(s)) {
        return Err(Error::safety(format!(
            "{} is not one of your source folders",
            stranger.display()
        )));
    }
    if sources.is_empty() {
        return Err(Error::config("choose at least one folder to clear"));
    }

    let (scanned, plan) = tauri::async_runtime::spawn_blocking(move || -> Result<(Scan, Plan)> {
        let scanned = scan::scan_all(&sources, config.settings.include_hidden)?;
        let narrowed = config.restricted_to(&destinations);
        let mut plan = rules::build_plan(&scanned, &narrowed, &rules::load_rules()?);
        if config.settings.rename_unclear {
            rename::propose(&mut plan, &scanned, &narrowed);
        }
        dupes::detect(&mut plan, &scanned, &narrowed)?;
        Ok((scanned, plan))
    })
    .await
    .map_err(|e| Error::config(format!("planning stopped unexpectedly: {e}")))??;

    let mut guard = state(&menlo);
    guard.scan = Some(scanned);
    guard.set_label = set_label;
    Ok(plan)
}

/// Apply an approved plan, streaming a `menlo://progress` event per file.
///
/// The plan arrives from the frontend carrying the user's edits, so it is untrusted
/// input: it gets the full §4.3 treatment again, and its duplicate findings are thrown
/// away and recomputed — only the user's *choices* about them are kept.
///
/// Generic over the runtime only so the IPC contract tests can drive it on Tauri's
/// mock runtime; the app itself always runs it on the real one.
#[tauri::command]
async fn apply_plan<R: tauri::Runtime>(
    plan: Plan,
    menlo: tauri::State<'_, Menlo>,
    app: tauri::AppHandle<R>,
) -> Result<execute::ExecuteOutcome> {
    let config = config::load()?;
    let (scanned, set_label, stop) = {
        let guard = state(&menlo);
        let scanned = guard
            .scan
            .clone()
            .ok_or_else(|| Error::config("no scan is loaded; plan the run again"))?;
        guard.stop.store(false, Ordering::SeqCst);
        (scanned, guard.set_label.clone(), guard.stop.clone())
    };

    tauri::async_runtime::spawn_blocking(move || {
        let raw: Vec<plan::RawPlanEntry> = plan
            .entries
            .iter()
            .filter(|e| e.included)
            .map(|e| plan::RawPlanEntry {
                id: e.id.clone(),
                action: e.action,
                destination_key: e.destination_key.clone(),
                rename_to: e.rename_to.clone(),
                confidence: e.confidence,
                reason: e.reason.clone(),
                suggest_rule: e.suggest_rule.clone(),
            })
            .collect();
        plan::validate(&raw, &scanned, &config.destination_keys())?;

        let mut plan = plan;
        dupes::detect(&mut plan, &scanned, &config)?;
        execute::execute_with(
            &plan,
            &scanned,
            &config,
            set_label,
            || stop.load(Ordering::SeqCst),
            |p| {
                let _ = app.emit("menlo://progress", &p);
            },
        )
    })
    .await
    .map_err(|e| Error::config(format!("the move stopped unexpectedly: {e}")))?
}

/// Stop the move in progress after the file currently in flight.
#[tauri::command]
fn stop_run(menlo: tauri::State<'_, Menlo>) {
    state(&menlo).stop.store(true, Ordering::SeqCst);
}

// ── Runs ─────────────────────────────────────────────────────────────────────

#[tauri::command]
fn get_run(batch_id: String) -> Result<RunView> {
    Ok(view::run_view(
        &journal::read_batch(&batch_id)?,
        &config::load()?,
    ))
}

#[tauri::command]
fn revert_run(batch_id: String) -> Result<RunView> {
    journal::revert_batch(&batch_id)?;
    get_run(batch_id)
}

/// Restore one row of a run. `key` names a destination; `None` restores what the run
/// sent to the Trash.
#[tauri::command]
fn revert_run_group(batch_id: String, key: Option<String>) -> Result<RunView> {
    let config = config::load()?;
    let want = match key {
        Some(k) => view::GroupKey::Destination(k),
        None => view::GroupKey::Trash,
    };
    journal::revert_batch_where(&batch_id, |m| view::group_key(m, &config) == want)?;
    get_run(batch_id)
}

/// The commands and the state they need. Shared by the app and by the IPC contract
/// tests (tests/ipc.rs), so the tests exercise exactly this registration — the one
/// place a mismatch between what the window sends and what a command expects would
/// otherwise only show up at runtime.
pub fn with_commands<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(Menlo::new())
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_settings,
            add_source,
            remove_source,
            add_destination,
            create_subfolder,
            remove_destination,
            suggest_folders,
            save_folder_set,
            delete_folder_set,
            add_folder_rule,
            remove_folder_rule,
            plan_run,
            apply_plan,
            stop_run,
            get_run,
            revert_run,
            revert_run_group,
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    with_commands(
        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_dialog::init()),
    )
    .run(tauri::generate_context!())
    .expect("error while running Menlo");
}
