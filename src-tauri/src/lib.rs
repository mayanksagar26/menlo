//! Menlo — local-first file organiser.
//!
//! The command layer. Note what is *not* here: no command accepts a filesystem path
//! for a destination. The frontend works in destination *keys* and the app maps them
//! (§4.2), which is what keeps path injection structurally impossible.

pub mod config;
pub mod error;
pub mod execute;
pub mod journal;
pub mod plan;
pub mod rules;
pub mod safety;
pub mod scan;
pub mod testutil;

use config::{Config, Destination};
use error::{Error, Result};
use plan::Plan;
use scan::Scan;
use serde::Serialize;
use std::sync::Mutex;

/// The scan behind the currently-displayed plan.
///
/// It lives in Rust because it holds real paths, which never cross to the frontend.
/// The frontend refers to files by manifest id alone.
#[derive(Default)]
pub struct AppState {
    scan: Option<Scan>,
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

#[derive(Serialize)]
pub struct SetupState {
    config: Config,
    /// Destination keys the built-in extension map understands, for the Setup screen's
    /// suggestions. A destination named with one of these gets tier-1 routing for free.
    suggested_keys: Vec<String>,
    rules: rules::RuleSet,
}

#[tauri::command]
fn get_setup_state() -> Result<SetupState> {
    Ok(SetupState {
        config: config::load()?,
        suggested_keys: rules::BUILTIN_CATEGORIES
            .iter()
            .map(|(k, _)| k.to_string())
            .collect(),
        rules: rules::load_rules()?,
    })
}

#[tauri::command]
fn save_config(config: Config) -> Result<Config> {
    // Destinations are created here, so this is where their paths are vetted.
    for d in &config.destinations {
        safety::assert_operable(&d.path)?;
    }
    if let Some(src) = &config.source {
        safety::assert_operable(src)?;
    }
    config::save(&config)?;
    Ok(config)
}

/// Create a destination folder the user picked but that does not exist yet.
#[tauri::command]
fn ensure_destination(destination: Destination) -> Result<()> {
    safety::assert_operable(&destination.path)?;
    std::fs::create_dir_all(&destination.path).map_err(|e| Error::io(&destination.path, e))?;
    Ok(())
}

/// Scan the source folder and run tiers 1 and 2. Never touches a file.
#[tauri::command]
fn scan_and_plan(menlo: tauri::State<'_, Menlo>) -> Result<Plan> {
    let config = config::load()?;
    let source = config
        .source
        .clone()
        .ok_or_else(|| Error::config("no source folder has been chosen yet"))?;

    let scanned = scan::scan(&source)?;
    let ruleset = rules::load_rules()?;
    let plan = rules::build_plan(&scanned, &config, &ruleset);

    state(&menlo).scan = Some(scanned);
    Ok(plan)
}

/// Apply an approved plan.
///
/// The plan arrives from the frontend carrying the user's edits, so it is untrusted
/// input and gets the full §4.3 treatment again before anything moves.
#[tauri::command]
fn apply_plan(plan: Plan, menlo: tauri::State<'_, Menlo>) -> Result<execute::ExecuteOutcome> {
    let config = config::load()?;
    let guard = state(&menlo);
    let scanned = guard
        .scan
        .as_ref()
        .ok_or_else(|| Error::config("no scan is loaded; re-scan before applying"))?;

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
    plan::validate(&raw, scanned, &config.destination_keys())?;

    execute::execute(&plan, scanned, &config)
}

#[tauri::command]
fn list_batches() -> Result<Vec<journal::BatchSummary>> {
    journal::list_batches()
}

#[tauri::command]
fn get_batch(batch_id: String) -> Result<journal::Batch> {
    journal::read_batch(&batch_id)
}

#[tauri::command]
fn revert_batch(batch_id: String) -> Result<journal::RevertOutcome> {
    journal::revert_batch(&batch_id)
}

#[tauri::command]
fn get_rules() -> Result<rules::RuleSet> {
    rules::load_rules()
}

#[tauri::command]
fn add_rule(match_expr: String, destination_key: String) -> Result<rules::RuleSet> {
    let config = config::load()?;
    if config.destination(&destination_key).is_none() {
        return Err(Error::config(format!(
            "no destination named `{destination_key}`"
        )));
    }
    // Parse eagerly so a malformed rule is rejected at the point of entry, where the
    // user can see the error, rather than silently never matching.
    rules::Matcher::parse(&match_expr)?;

    let mut set = rules::load_rules()?;
    set.insert(rules::Rule::new(
        match_expr,
        destination_key,
        rules::RuleSource::User,
    ));
    rules::save_rules(&set)?;
    Ok(set)
}

#[tauri::command]
fn delete_rule(rule_id: String) -> Result<rules::RuleSet> {
    let mut set = rules::load_rules()?;
    set.remove(&rule_id);
    rules::save_rules(&set)?;
    Ok(set)
}

#[tauri::command]
fn set_rule_enabled(rule_id: String, enabled: bool) -> Result<rules::RuleSet> {
    let mut set = rules::load_rules()?;
    if let Some(r) = set.rules.iter_mut().find(|r| r.id == rule_id) {
        r.enabled = enabled;
    }
    rules::save_rules(&set)?;
    Ok(set)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Menlo::new())
        .invoke_handler(tauri::generate_handler![
            get_setup_state,
            save_config,
            ensure_destination,
            scan_and_plan,
            apply_plan,
            list_batches,
            get_batch,
            revert_batch,
            get_rules,
            add_rule,
            delete_rule,
            set_rule_enabled,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Menlo");
}
