/**
 * TypeScript mirror of the Rust contracts in §4.
 *
 * These are hand-maintained rather than generated. If you change a `serde` struct in
 * `src-tauri/src`, change it here in the same commit — `docs/PROMPT_CONTRACT.md` holds
 * the canonical JSON Schema for the two that cross the CLI boundary.
 */

export type Action = "move" | "skip" | "needs_review";
export type ResolvedBy = "deterministic" | "learned" | "llm";
export type RuleSource = "builtin" | "user" | "learned";
export type Strategy = "rename" | "copy_verify_delete";

export type SkipReason =
  | "dotfile"
  | "system_file"
  | "partial_download"
  | "symlink"
  | "directory"
  | "unreadable"
  | "batch_cap_reached";

export interface Destination {
  key: string;
  label: string;
  path: string;
  brief: string;
}

export interface Settings {
  allow_installers: boolean;
  check_open_files: boolean;
}

export interface Config {
  source: string | null;
  destinations: Destination[];
  settings: Settings;
}

export interface SuggestRule {
  match: string;
  destination_key: string;
}

/** §4.2, plus the app-side provenance and review state. */
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
}

export interface Skipped {
  name: string;
  reason: SkipReason;
}

export interface Plan {
  batch_id: string;
  source: string;
  created_at: string;
  entries: PlanEntry[];
  skipped: Skipped[];
  truncated: boolean;
  prompt_version: string | null;
}

export interface Rule {
  id: string;
  match: string;
  destination_key: string;
  source: RuleSource;
  created_at: string;
  hits: number;
  enabled: boolean;
}

export interface RuleSet {
  rules: Rule[];
}

export interface SetupState {
  config: Config;
  suggested_keys: string[];
  rules: RuleSet;
}

export interface FailureRecord {
  entry_id: string;
  from: string;
  intended_to: string;
  error: string;
  at: string;
}

export interface MoveRecord {
  entry_id: string;
  from: string;
  to: string;
  sha256: string;
  size_bytes: number;
  strategy: Strategy;
  renamed_for_collision: boolean;
  at: string;
}

export interface RevertRecord {
  entry_id: string;
  from: string;
  to: string;
  at: string;
}

export interface BatchHeader {
  batch_id: string;
  started_at: string;
  source: string;
  planned: number;
  app_version: string;
}

export interface Batch {
  header: BatchHeader;
  moves: MoveRecord[];
  failures: FailureRecord[];
  reverts: RevertRecord[];
  revert_failures: FailureRecord[];
}

export interface BatchSummary {
  batch_id: string;
  started_at: string;
  source: string;
  moved: number;
  failed: number;
  reverted: number;
  fully_reverted: boolean;
}

export interface ExecuteOutcome {
  batch_id: string;
  moved: number;
  failures: FailureRecord[];
  learned: Rule[];
}

export interface RevertOutcome {
  batch_id: string;
  restored: number;
  failed: FailureRecord[];
}

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
