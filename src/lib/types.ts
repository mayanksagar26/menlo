/**
 * TypeScript mirror of the Rust types that cross the IPC boundary.
 *
 * Hand-maintained rather than generated. If you change a `serde` struct in
 * `src-tauri/src`, change it here in the same commit. `docs/PROMPT_CONTRACT.md` holds
 * the canonical JSON Schema for the two shapes that cross to a model.
 */

// ── Config ───────────────────────────────────────────────────────────────────

/** A folder files are filed into. A subfolder is a destination with a `parent`. */
export interface Destination {
  key: string;
  label: string;
  path: string;
  brief: string;
  parent?: string | null;
}

export interface FolderSet {
  id: string;
  label: string;
  ai: boolean;
  sources: string[];
  /** Top-level destination keys; subfolders come with their parent. */
  destinations: string[];
  pinned: boolean;
}

export type WorkingModel = "fully_local" | "gemma" | "cli_agent";
export type Schedule = "manual" | "daily" | "on_login";
export type DuplicatePolicy = "ask" | "keep" | "trash";

export interface Settings {
  allow_installers: boolean;
  check_open_files: boolean;
  profile_name: string;
  /** A preset's id, or "custom" for an uploaded picture. */
  avatar: string;
  working_model: WorkingModel;
  schedule: Schedule;
  duplicates: DuplicatePolicy;
  rename_unclear: boolean;
  ask_before_moving: boolean;
  include_hidden: boolean;
  notify_on_finish: boolean;
  keep_history: boolean;
}

// ── The view the window reads ────────────────────────────────────────────────

export interface SourceView {
  path: string;
  label: string;
  count: number;
  /** False when the folder has gone since it was added. */
  exists: boolean;
}

export interface FolderRuleView {
  id: string;
  destination_key: string;
  text: string;
  /** Whether Menlo can act on this without a model. */
  understood: boolean;
  /** What Menlo took from the sentence, in words. */
  reading: string;
}

export interface BatchSummary {
  batch_id: string;
  started_at: string;
  source: string;
  set_label: string | null;
  moved: number;
  trashed: number;
  kept: number;
  failed: number;
  reverted: number;
  fully_reverted: boolean;
}

export interface AppView {
  home: string;
  sources: SourceView[];
  destinations: Destination[];
  folder_sets: FolderSet[];
  settings: Settings;
  rules: FolderRuleView[];
  runs: BatchSummary[];
  /** The uploaded picture as a data URL, when that is the one chosen. */
  avatar_image: string | null;
}

export interface FolderSuggestion {
  path: string;
  label: string;
  reason: string;
  key: string | null;
  parent_key: string | null;
  exists: boolean;
}

// ── A plan ───────────────────────────────────────────────────────────────────

export type Action = "move" | "skip" | "needs_review";
export type ResolvedBy = "deterministic" | "learned" | "llm";

export type DuplicateKind = "identical" | "same_name";
export type DuplicateChoice = "keep" | "trash" | "move_anyway";

export interface Duplicate {
  kind: DuplicateKind;
  /** Basename of the file it matched. Never a path. */
  existing_name: string;
  /** The id of this file's twin elsewhere in the same batch. */
  same_as_entry?: string;
}

export interface SuggestRule {
  match: string;
  destination_key: string;
}

export interface PlanEntry {
  id: string;
  name: string;
  ext: string;
  size_bytes: number;
  action: Action;
  destination_key: string | null;
  rename_to: string | null;
  confidence: number;
  reason: string;
  suggest_rule: SuggestRule | null;
  resolved_by: ResolvedBy;
  included: boolean;
  overridden: boolean;
  duplicate: Duplicate | null;
  on_duplicate: DuplicateChoice | null;
}

export type SkipReason =
  | "dotfile"
  | "system_file"
  | "partial_download"
  | "symlink"
  | "directory"
  | "unreadable"
  | "batch_cap_reached";

export interface Plan {
  batch_id: string;
  source: string;
  sources: string[];
  created_at: string;
  entries: PlanEntry[];
  skipped: { name: string; reason: SkipReason }[];
  truncated: boolean;
  prompt_version: string | null;
}

// ── A run ────────────────────────────────────────────────────────────────────

export interface FailureRecord {
  entry_id: string;
  from: string;
  intended_to: string;
  error: string;
  at: string;
}

export interface ExecuteOutcome {
  batch_id: string;
  stopped: boolean;
  moved: number;
  trashed: number;
  kept: number;
  failures: FailureRecord[];
}

export type StepOutcome = "moved" | "trashed" | "kept" | "failed";

/** One tick of the Move stage. Ids and keys only — never a path. */
export interface Progress {
  done: number;
  total: number;
  entry_id: string;
  destination_key: string | null;
  outcome: StepOutcome;
}

export interface RunGroup {
  /** A destination key; null for the Trash row and for a folder since removed. */
  key: string | null;
  label: string;
  trash: boolean;
  count: number;
  restored: number;
}

export interface RunView {
  summary: BatchSummary;
  groups: RunGroup[];
  /** Files a restore could not bring back because they changed after the run. */
  stayed_put: number;
}

// ── Errors ───────────────────────────────────────────────────────────────────

/** The wire shape of `error::Error`. */
export interface MenloError {
  kind: "io" | "serde" | "validation" | "safety" | "config" | "integrity";
  message: string;
  details: string[];
}

export function isMenloError(e: unknown): e is MenloError {
  return (
    typeof e === "object" &&
    e !== null &&
    "kind" in e &&
    "message" in e &&
    typeof (e as MenloError).message === "string"
  );
}

export function errorMessage(e: unknown): string {
  if (isMenloError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}
