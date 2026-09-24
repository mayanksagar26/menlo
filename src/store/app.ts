/**
 * What Menlo knows: the backend's view of folders, sets, rules, settings and runs,
 * plus the plan and the move in progress.
 *
 * Every mutation goes to Rust and comes back as a fresh `AppView`, which replaces the
 * old one wholesale. The window never edits its own copy of the truth, so it cannot
 * drift from what is on disk. Navigation and what the user has picked for this run
 * live separately, in `ui.ts`.
 */

import { create } from "zustand";
import * as ipc from "../lib/ipc";
import {
  errorMessage,
  type AppView,
  type Destination,
  type DuplicateChoice,
  type ExecuteOutcome,
  type FolderSet,
  type FolderSuggestion,
  type Plan,
  type Progress,
  type RunView,
  type Settings,
} from "../lib/types";

export interface MoveProgress {
  done: number;
  total: number;
  /** Files landed per destination key. */
  byKey: Record<string, number>;
  /** The last few files through, newest last — the cards in flight. */
  recent: Progress[];
}

const emptyProgress: MoveProgress = { done: 0, total: 0, byKey: {}, recent: [] };

interface AppStore {
  view: AppView | null;
  error: string | null;

  plan: Plan | null;
  outcome: ExecuteOutcome | null;
  progress: MoveProgress;
  run: RunView | null;

  boot: () => Promise<void>;
  dismissError: () => void;

  addSources: (paths: string[]) => Promise<void>;
  removeSource: (path: string) => Promise<void>;
  addDestinations: (paths: string[], parentKey?: string | null) => Promise<void>;
  addSuggestion: (s: FolderSuggestion) => Promise<void>;
  createSubfolder: (parentKey: string, name: string) => Promise<void>;
  removeDestination: (key: string) => Promise<void>;
  suggest: (parentKey: string | null) => Promise<FolderSuggestion[]>;

  saveSet: (set: FolderSet) => Promise<void>;
  deleteSet: (id: string) => Promise<void>;

  addRule: (key: string, text: string) => Promise<void>;
  removeRule: (id: string) => Promise<void>;

  saveSettings: (patch: Partial<Settings>) => Promise<void>;
  setAvatar: (id: string) => Promise<void>;
  uploadAvatar: (dataUrl: string) => Promise<void>;
  clearAvatar: () => Promise<void>;

  makePlan: (sources: string[], dests: string[], label: string | null) => Promise<Plan | null>;
  choose: (entryId: string, choice: DuplicateChoice) => void;
  chooseAll: (choice: DuplicateChoice) => void;
  apply: () => Promise<ExecuteOutcome | null>;
  stop: () => Promise<void>;
  clearRun: () => void;

  openRun: (batchId: string) => Promise<void>;
  revertRun: (batchId: string) => Promise<void>;
  revertGroup: (batchId: string, key: string | null) => Promise<void>;
}

export const useApp = create<AppStore>((set, get) => {
  /** Run an IPC call, surfacing a failure as the window's one error line. */
  async function call<T>(fn: () => Promise<T>): Promise<T | undefined> {
    set({ error: null });
    try {
      return await fn();
    } catch (e) {
      set({ error: errorMessage(e) });
      return undefined;
    }
  }

  async function mutate(fn: () => Promise<AppView>) {
    const view = await call(fn);
    if (view) set({ view });
  }

  return {
    view: null,
    error: null,
    plan: null,
    outcome: null,
    progress: emptyProgress,
    run: null,

    boot: () => mutate(ipc.getState),
    dismissError: () => set({ error: null }),

    addSources: async (paths) => {
      for (const p of paths) await mutate(() => ipc.addSource(p));
    },
    removeSource: (path) => mutate(() => ipc.removeSource(path)),
    addDestinations: async (paths, parentKey) => {
      for (const p of paths) await mutate(() => ipc.addDestination(p, parentKey));
    },
    addSuggestion: (s) => mutate(() => ipc.addDestination(s.path, s.parent_key, s.key)),
    createSubfolder: (parentKey, name) => mutate(() => ipc.createSubfolder(parentKey, name)),
    removeDestination: (key) => mutate(() => ipc.removeDestination(key)),
    suggest: async (parentKey) => (await call(() => ipc.suggestFolders(parentKey))) ?? [],

    saveSet: (s) => mutate(() => ipc.saveFolderSet(s)),
    deleteSet: (id) => mutate(() => ipc.deleteFolderSet(id)),

    addRule: (key, text) => mutate(() => ipc.addFolderRule(key, text)),
    removeRule: (id) => mutate(() => ipc.removeFolderRule(id)),

    saveSettings: async (patch) => {
      const view = get().view;
      if (!view) return;
      // Optimistic, because a toggle that waits on a round trip feels broken; the
      // view that comes back replaces this either way.
      const settings = { ...view.settings, ...patch };
      set({ view: { ...view, settings } });
      await mutate(() => ipc.saveSettings(settings));
    },

    setAvatar: (id) => mutate(() => ipc.setAvatar(id)),
    uploadAvatar: (dataUrl) => mutate(() => ipc.saveAvatar(dataUrl)),
    clearAvatar: () => mutate(ipc.clearAvatar),

    makePlan: async (sources, dests, label) => {
      set({ plan: null, outcome: null, progress: emptyProgress });
      const plan = await call(() => ipc.planRun(sources, dests, label));
      if (!plan) return null;
      set({ plan });
      return plan;
    },

    choose: (entryId, choice) => {
      const plan = get().plan;
      if (!plan) return;
      set({
        plan: {
          ...plan,
          entries: plan.entries.map((e) =>
            e.id === entryId ? { ...e, on_duplicate: choice } : e,
          ),
        },
      });
    },

    chooseAll: (choice) => {
      const plan = get().plan;
      if (!plan) return;
      set({
        plan: {
          ...plan,
          entries: plan.entries.map((e) =>
            // Trash is only ever offered for byte-identical copies.
            e.duplicate?.kind === "identical" ? { ...e, on_duplicate: choice } : e,
          ),
        },
      });
    },

    apply: async () => {
      const plan = get().plan;
      if (!plan) return null;
      const total = plan.entries.filter(
        (e) => e.included && e.action === "move" && e.destination_key,
      ).length;
      set({ outcome: null, progress: { ...emptyProgress, total } });

      const unlisten = await ipc.onProgress((p) => {
        const prev = get().progress;
        const byKey = { ...prev.byKey };
        if (p.outcome === "moved" && p.destination_key) {
          byKey[p.destination_key] = (byKey[p.destination_key] ?? 0) + 1;
        }
        set({
          progress: {
            done: p.done,
            total: p.total,
            byKey,
            recent: [...prev.recent, p].slice(-10),
          },
        });
      });
      try {
        const outcome = await call(() => ipc.applyPlan(plan));
        if (outcome) set({ outcome });
        // The Runs list and the source counts both changed.
        await mutate(ipc.getState);
        return outcome ?? null;
      } finally {
        unlisten();
      }
    },

    stop: async () => {
      await call(ipc.stopRun);
    },

    clearRun: () => set({ plan: null, outcome: null, progress: emptyProgress }),

    openRun: async (batchId) => {
      const run = await call(() => ipc.getRun(batchId));
      if (run) set({ run });
    },
    revertRun: async (batchId) => {
      const run = await call(() => ipc.revertRun(batchId));
      if (run) set({ run });
      await mutate(ipc.getState);
    },
    revertGroup: async (batchId, key) => {
      const run = await call(() => ipc.revertRunGroup(batchId, key));
      if (run) set({ run });
      await mutate(ipc.getState);
    },
  };
});

// ── Derived ─────────────────────────────────────────────────────────────────

export function topLevel(view: AppView | null): Destination[] {
  return view?.destinations.filter((d) => !d.parent) ?? [];
}

export function subfoldersOf(view: AppView | null, key: string): Destination[] {
  return view?.destinations.filter((d) => d.parent === key) ?? [];
}

export function destination(view: AppView | null, key: string | null | undefined) {
  return key ? view?.destinations.find((d) => d.key === key) : undefined;
}

export function rulesFor(view: AppView | null, key: string) {
  return view?.rules.filter((r) => r.destination_key === key) ?? [];
}

/** "Documents/Invoices 2026" for a subfolder, "Documents" for a top-level folder. */
export function pathLabel(view: AppView | null, key: string | null | undefined): string {
  const d = destination(view, key);
  if (!d) return key ?? "";
  const parent = destination(view, d.parent);
  return parent ? `${parent.label}/${d.label}` : d.label;
}

/** `/Users/x/Downloads` → `~/Downloads`. */
export function tilde(view: AppView | null, path: string): string {
  const home = view?.home;
  return home && path.startsWith(home) ? `~${path.slice(home.length)}` : path;
}

/** Entries that will be touched when the plan is applied. */
export function actionable(plan: Plan | null) {
  return plan?.entries.filter((e) => e.included && e.action === "move" && e.destination_key) ?? [];
}

/** Duplicates the user needs to decide about: byte-identical copies. */
export function identicalDuplicates(plan: Plan | null) {
  return actionable(plan).filter((e) => e.duplicate?.kind === "identical");
}
