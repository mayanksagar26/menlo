/**
 * Application state. Kept small on purpose (§2).
 *
 * The plan lives here as the single source of truth for the review screen; every edit
 * the user makes on that screen is a mutation of `plan.entries`, and `applyPlan` sends
 * exactly what is on screen back to Rust for re-validation.
 */

import { create } from "zustand";
import * as ipc from "../lib/ipc";
import { errorMessage } from "../lib/types";
import type {
  BatchSummary,
  Config,
  Destination,
  ExecuteOutcome,
  Plan,
  PlanEntry,
  RuleSet,
} from "../lib/types";

export type Screen = "setup" | "review" | "history" | "settings";

interface State {
  screen: Screen;
  ready: boolean;
  busy: string | null;
  error: string | null;

  config: Config;
  suggestedKeys: string[];
  rules: RuleSet;

  plan: Plan | null;
  outcome: ExecuteOutcome | null;
  batches: BatchSummary[];

  go: (screen: Screen) => void;
  dismissError: () => void;

  boot: () => Promise<void>;
  setConfig: (config: Config) => void;
  persistConfig: () => Promise<void>;
  addDestination: (destination: Destination) => Promise<void>;
  removeDestination: (key: string) => Promise<void>;

  scan: () => Promise<void>;
  updateEntry: (id: string, patch: Partial<PlanEntry>) => void;
  setAllIncluded: (included: boolean, ids?: string[]) => void;
  apply: () => Promise<void>;
  clearOutcome: () => void;

  refreshBatches: () => Promise<void>;
  revert: (batchId: string) => Promise<void>;

  refreshRules: () => Promise<void>;
  addRule: (matchExpr: string, destinationKey: string) => Promise<void>;
  deleteRule: (ruleId: string) => Promise<void>;
  toggleRule: (ruleId: string, enabled: boolean) => Promise<void>;
}

const emptyConfig: Config = {
  source: null,
  destinations: [],
  settings: { allow_installers: false, check_open_files: true },
};

export const useStore = create<State>((set, get) => {
  /** Run an async action with busy/error bookkeeping so screens never duplicate it. */
  async function guard<T>(label: string, fn: () => Promise<T>): Promise<T | undefined> {
    set({ busy: label, error: null });
    try {
      return await fn();
    } catch (e) {
      set({ error: errorMessage(e) });
      return undefined;
    } finally {
      set({ busy: null });
    }
  }

  return {
    screen: "setup",
    ready: false,
    busy: null,
    error: null,

    config: emptyConfig,
    suggestedKeys: [],
    rules: { rules: [] },

    plan: null,
    outcome: null,
    batches: [],

    go: (screen) => set({ screen, error: null }),
    dismissError: () => set({ error: null }),

    boot: async () => {
      await guard("Loading", async () => {
        const state = await ipc.getSetupState();
        const configured =
          state.config.source !== null && state.config.destinations.length > 0;
        set({
          config: state.config,
          suggestedKeys: state.suggested_keys,
          rules: state.rules,
          // A configured user lands on the hero screen, not on setup.
          screen: configured ? "review" : "setup",
          ready: true,
        });
      });
      set({ ready: true });
    },

    setConfig: (config) => set({ config }),

    persistConfig: async () => {
      await guard("Saving", async () => {
        const saved = await ipc.saveConfig(get().config);
        set({ config: saved });
      });
    },

    addDestination: async (destination) => {
      await guard("Adding folder", async () => {
        await ipc.ensureDestination(destination);
        const config = {
          ...get().config,
          destinations: [...get().config.destinations, destination],
        };
        const saved = await ipc.saveConfig(config);
        set({ config: saved });
      });
    },

    removeDestination: async (key) => {
      await guard("Removing folder", async () => {
        const config = {
          ...get().config,
          destinations: get().config.destinations.filter((d) => d.key !== key),
        };
        const saved = await ipc.saveConfig(config);
        set({ config: saved });
      });
    },

    scan: async () => {
      await guard("Scanning", async () => {
        const plan = await ipc.scanAndPlan();
        set({ plan, outcome: null, screen: "review" });
      });
    },

    updateEntry: (id, patch) => {
      const plan = get().plan;
      if (!plan) return;
      set({
        plan: {
          ...plan,
          entries: plan.entries.map((e) => (e.id === id ? { ...e, ...patch } : e)),
        },
      });
    },

    setAllIncluded: (included, ids) => {
      const plan = get().plan;
      if (!plan) return;
      const scope = ids ? new Set(ids) : null;
      set({
        plan: {
          ...plan,
          entries: plan.entries.map((e) => {
            if (scope && !scope.has(e.id)) return e;
            // Nothing without a destination can be included; the toggle would be a lie.
            if (included && !e.destination_key) return e;
            return { ...e, included };
          }),
        },
      });
    },

    apply: async () => {
      const plan = get().plan;
      if (!plan) return;
      await guard("Filing", async () => {
        const outcome = await ipc.applyPlan(plan);
        const rules = await ipc.getRules();
        set({ outcome, rules, plan: null });
        await get().refreshBatches();
      });
    },

    clearOutcome: () => set({ outcome: null }),

    refreshBatches: async () => {
      await guard("Loading history", async () => {
        set({ batches: await ipc.listBatches() });
      });
    },

    revert: async (batchId) => {
      await guard("Reverting", async () => {
        const result = await ipc.revertBatch(batchId);
        await get().refreshBatches();
        if (result.failed.length > 0) {
          set({
            error: `Restored ${result.restored}. ${result.failed.length} could not be put back: ${result.failed[0].error}`,
          });
        }
      });
    },

    refreshRules: async () => {
      await guard("Loading rules", async () => {
        set({ rules: await ipc.getRules() });
      });
    },

    addRule: async (matchExpr, destinationKey) => {
      await guard("Adding rule", async () => {
        set({ rules: await ipc.addRule(matchExpr, destinationKey) });
      });
    },

    deleteRule: async (ruleId) => {
      await guard("Deleting rule", async () => {
        set({ rules: await ipc.deleteRule(ruleId) });
      });
    },

    toggleRule: async (ruleId, enabled) => {
      await guard("Updating rule", async () => {
        set({ rules: await ipc.setRuleEnabled(ruleId, enabled) });
      });
    },
  };
});

/** Derived counts the review screen leans on. */
export function planStats(plan: Plan | null) {
  if (!plan) return { total: 0, willMove: 0, withoutAi: 0, needsReview: 0, excluded: 0 };
  const willMove = plan.entries.filter(
    (e) => e.included && e.action === "move" && e.destination_key,
  ).length;
  const withoutAi = plan.entries.filter(
    (e) => e.action === "move" && (e.resolved_by === "deterministic" || e.resolved_by === "learned"),
  ).length;
  const needsReview = plan.entries.filter((e) => e.action === "needs_review").length;
  const excluded = plan.entries.filter((e) => !e.included).length;
  return { total: plan.entries.length, willMove, withoutAi, needsReview, excluded };
}
