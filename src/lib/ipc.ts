/**
 * Typed wrappers over the Tauri command layer in `src-tauri/src/lib.rs`.
 *
 * Command *arguments* are camelCase here — Tauri renames them from Rust's snake_case —
 * while the fields of any struct passed or returned stay snake_case, because serde
 * owns those. `parentKey` the argument; `parent_key` the field.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  AppView,
  ExecuteOutcome,
  FolderSet,
  FolderSuggestion,
  Plan,
  Progress,
  RunView,
  Settings,
} from "./types";

export const getState = () => invoke<AppView>("get_state");
export const saveSettings = (settings: Settings) => invoke<AppView>("save_settings", { settings });

export const addSource = (path: string) => invoke<AppView>("add_source", { path });
export const removeSource = (path: string) => invoke<AppView>("remove_source", { path });

export const addDestination = (path: string, parentKey?: string | null, key?: string | null) =>
  invoke<AppView>("add_destination", { path, parentKey: parentKey ?? null, key: key ?? null });
export const createSubfolder = (parentKey: string, name: string) =>
  invoke<AppView>("create_subfolder", { parentKey, name });
export const removeDestination = (key: string) => invoke<AppView>("remove_destination", { key });
export const suggestFolders = (parentKey?: string | null) =>
  invoke<FolderSuggestion[]>("suggest_folders", { parentKey: parentKey ?? null });

export const saveFolderSet = (set: FolderSet) => invoke<AppView>("save_folder_set", { set });
export const deleteFolderSet = (id: string) => invoke<AppView>("delete_folder_set", { id });

export const addFolderRule = (destinationKey: string, text: string) =>
  invoke<AppView>("add_folder_rule", { destinationKey, text });
export const removeFolderRule = (id: string) => invoke<AppView>("remove_folder_rule", { id });

export const planRun = (sources: string[], destinations: string[], setLabel: string | null) =>
  invoke<Plan>("plan_run", { sources, destinations, setLabel });
export const applyPlan = (plan: Plan) => invoke<ExecuteOutcome>("apply_plan", { plan });
export const stopRun = () => invoke<void>("stop_run");

export const getRun = (batchId: string) => invoke<RunView>("get_run", { batchId });
export const revertRun = (batchId: string) => invoke<RunView>("revert_run", { batchId });
export const revertRunGroup = (batchId: string, key: string | null) =>
  invoke<RunView>("revert_run_group", { batchId, key });

/** One event per file while a plan is applied. */
export const onProgress = (cb: (p: Progress) => void): Promise<UnlistenFn> =>
  listen<Progress>("menlo://progress", (e) => cb(e.payload));

/**
 * The system folder picker. Resolves to the chosen folders, or none if cancelled.
 * `defaultPath` opens it somewhere useful — inside a folder, to pick a subfolder; its
 * own New Folder button covers making one.
 */
export async function pickFolders(title: string, multiple = true, defaultPath?: string): Promise<string[]> {
  const picked = await open({ directory: true, multiple, title, defaultPath });
  if (picked === null) return [];
  return Array.isArray(picked) ? picked : [picked];
}
