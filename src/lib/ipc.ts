/** Typed wrappers over the Tauri command layer in `src-tauri/src/lib.rs`. */

import { invoke } from "@tauri-apps/api/core";
import type {
  Batch,
  BatchSummary,
  Config,
  Destination,
  ExecuteOutcome,
  Plan,
  RevertOutcome,
  RuleSet,
  SetupState,
} from "./types";

export const getSetupState = () => invoke<SetupState>("get_setup_state");
export const saveConfig = (config: Config) => invoke<Config>("save_config", { config });
export const ensureDestination = (destination: Destination) =>
  invoke<void>("ensure_destination", { destination });

export const scanAndPlan = () => invoke<Plan>("scan_and_plan");
export const applyPlan = (plan: Plan) => invoke<ExecuteOutcome>("apply_plan", { plan });

export const listBatches = () => invoke<BatchSummary[]>("list_batches");
export const getBatch = (batchId: string) => invoke<Batch>("get_batch", { batchId });
export const revertBatch = (batchId: string) => invoke<RevertOutcome>("revert_batch", { batchId });

export const getRules = () => invoke<RuleSet>("get_rules");
export const addRule = (matchExpr: string, destinationKey: string) =>
  invoke<RuleSet>("add_rule", { matchExpr, destinationKey });
export const deleteRule = (ruleId: string) => invoke<RuleSet>("delete_rule", { ruleId });
export const setRuleEnabled = (ruleId: string, enabled: boolean) =>
  invoke<RuleSet>("set_rule_enabled", { ruleId, enabled });
