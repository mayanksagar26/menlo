//! Menlo — local-first file organiser.
//!
//! The command layer. Note what is *not* here: no command accepts a filesystem path
//! for a plan's destination. The frontend works in destination *keys* and the app
//! maps them (§4.2), which is what keeps path injection structurally impossible. The
//! commands that do take a path — adding a source or a destination — are the user
//! pointing at a folder with the picker, and each is vetted by `safety` before it is
//! saved.

pub mod avatar;
pub mod config;
pub mod dupes;
pub mod error;
pub mod execute;
pub mod harness;
pub mod journal;
pub mod memory;
pub mod plan;
pub mod prose;
pub mod rename;
pub mod rules;
pub mod safety;
pub mod scan;
pub mod testutil;
pub mod tidy;
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

/// Store a picture the user picked as their profile picture. The window has already
/// cropped and downscaled it; this checks it really is an image before writing it.
#[tauri::command]
fn save_avatar(data_url: String) -> Result<AppView> {
    avatar::save(&data_url)?;
    let mut config = config::load()?;
    config.settings.avatar = avatar::CUSTOM.to_string();
    commit(&config)
}

/// Choose one of the pictures that ship with Menlo. An uploaded one is kept on disk,
/// so switching away and back does not lose it.
#[tauri::command]
fn set_avatar(id: String) -> Result<AppView> {
    let mut config = config::load()?;
    config.settings.avatar = id;
    commit(&config)
}

/// Forget an uploaded picture and go back to Menlo's own.
#[tauri::command]
fn clear_avatar() -> Result<AppView> {
    avatar::remove()?;
    set_avatar(avatar::DEFAULT.to_string())
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

// ── The CLI agent ──────────────────────────────────────────────────────────

/// Every CLI agent Menlo knows, and where each was found on this Mac.
#[tauri::command]
async fn detect_cli_agents() -> Result<Vec<harness::AgentInfo>> {
    tauri::async_runtime::spawn_blocking(harness::detect)
        .await
        .map_err(|e| Error::config(format!("detection stopped unexpectedly: {e}")))
}

/// A tiny round trip through the chosen CLI agent. Sends nothing about any file.
#[tauri::command]
async fn test_cli_agent() -> Result<String> {
    let settings = config::load()?.settings;
    tauri::async_runtime::spawn_blocking(move || harness::test(&settings))
        .await
        .map_err(|e| Error::config(format!("the test stopped unexpectedly: {e}")))?
}

// ── Rules memory ─────────────────────────────────────────────────────────────

/// Every folder in the home folder and its subfolders, for Rules memory. Read only:
/// listing a folder does not add it to Menlo.
#[tauri::command]
async fn list_device_folders() -> Result<Vec<view::DeviceFolder>> {
    let config = config::load()?;
    tauri::async_runtime::spawn_blocking(move || view::device_folders(&config))
        .await
        .map_err(|e| Error::config(format!("reading folders stopped unexpectedly: {e}")))?
}

/// Write a rule for any folder. A folder Menlo does not file into yet is added first
/// — with its parent, for a subfolder — since a rule needs a destination to belong to.
#[tauri::command]
fn add_rule_at(path: PathBuf, text: String) -> Result<AppView> {
    let path = canonical_folder(path)?;
    if !path.is_dir() {
        return Err(Error::config(format!("{} is not a folder", path.display())));
    }
    let mut config = config::load()?;
    let home = config::home()?;
    let home = home.canonicalize().unwrap_or(home);

    let ensure = |config: &mut Config, p: &std::path::Path, parent: Option<String>| -> String {
        if let Some(d) = config.destinations.iter().find(|d| d.path == p) {
            return d.key.clone();
        }
        let label = view::label_of(p);
        let key = config.fresh_key(&label, parent.as_deref());
        config.destinations.push(Destination {
            key: key.clone(),
            label,
            path: p.to_path_buf(),
            brief: String::new(),
            parent,
        });
        key
    };

    let key = match config.destinations.iter().find(|d| d.path == path) {
        Some(d) => d.key.clone(),
        None => {
            let parent_dir = path.parent().map(std::path::Path::to_path_buf);
            let parent_dest = parent_dir
                .as_ref()
                .and_then(|pd| {
                    config
                        .destinations
                        .iter()
                        .find(|d| &d.path == pd && !d.is_subfolder())
                })
                .map(|d| d.key.clone());
            match (parent_dest, parent_dir) {
                // Inside a folder Menlo uses: a subfolder of it.
                (Some(pk), _) => ensure(&mut config, &path, Some(pk)),
                // Inside a home folder Menlo does not use yet: add both.
                (None, Some(pd)) if pd != home && pd.parent() == Some(home.as_path()) => {
                    let pk = ensure(&mut config, &pd, None);
                    ensure(&mut config, &path, Some(pk))
                }
                _ => ensure(&mut config, &path, None),
            }
        }
    };
    config::save(&config)?;

    let mut ruleset = rules::load_rules()?;
    ruleset.add_folder_rule(&key, &text);
    rules::save_rules(&ruleset)?;
    view::app_view(&config, &ruleset)
}

/// Forget a pattern rule — one Menlo learned, or one the user made.
#[tauri::command]
fn forget_rule(id: String) -> Result<AppView> {
    let mut ruleset = rules::load_rules()?;
    ruleset.remove(&id);
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
    let mut config = config::load()?;
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

    // A folder tidied in place sorts into its own subfolders, so pick up any made in
    // Finder since it was added.
    let in_place: Vec<String> = config
        .destinations
        .iter()
        .filter(|d| !d.is_subfolder() && sources.contains(&d.path))
        .map(|d| d.key.clone())
        .collect();
    let mut synced = false;
    for key in &in_place {
        synced |= config.sync_subfolders(key);
    }
    if synced {
        config::save(&config)?;
    }

    let (scanned, plan) = tauri::async_runtime::spawn_blocking(move || -> Result<(Scan, Plan)> {
        let scanned = scan::scan_all(&sources, config.settings.include_hidden)?;
        let narrowed = config.restricted_to(&destinations);
        let ruleset = rules::load_rules()?;
        let mut plan = rules::build_plan(&scanned, &narrowed, &ruleset);
        tidy::settle_in_place(&mut plan, &scanned, &narrowed);
        // Tier 3: whatever the rules could not place goes to the CLI agent. A CLI that
        // fails never fails the run — the plan stays rules-only and says why.
        if narrowed.settings.working_model == config::WorkingModel::CliAgent {
            plan.model_note = Some(match harness::classify(&mut plan, &scanned, &narrowed, &ruleset) {
                Ok(o) if o.asked == 0 => format!("The rules placed everything; {} wasn't needed.", o.agent),
                Ok(o) if o.unsure > 0 => format!(
                    "{} placed {} of the {} files the rules couldn't; {} more were guesses and stay put.",
                    o.agent, o.placed, o.asked, o.unsure
                ),
                Ok(o) => format!("{} placed {} of the {} files the rules couldn't.", o.agent, o.placed, o.asked),
                Err(Error::Config(why)) => format!("The CLI agent couldn't help this time: {why}. These are the rules' moves only."),
                Err(e) => format!("The CLI agent couldn't help this time: {e}. These are the rules' moves only."),
            });
        }
        // The model may have picked the folder a file is already in, too.
        tidy::settle_in_place(&mut plan, &scanned, &narrowed);
        if config.settings.rename_unclear {
            rename::propose(&mut plan, &scanned, &narrowed);
        }
        dupes::detect(&mut plan, &scanned, &narrowed)?;
        plan.folder_proposals = tidy::propose(&plan, &scanned, &narrowed);
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
                suggest_folder: None,
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
            save_avatar,
            set_avatar,
            clear_avatar,
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
            detect_cli_agents,
            list_device_folders,
            add_rule_at,
            forget_rule,
            test_cli_agent,
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
