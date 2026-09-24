/**
 * A stubbed Tauri IPC, injected before the app's own scripts run.
 *
 * Tauri v2 routes every `invoke` through `window.__TAURI_INTERNALS__.invoke`, so
 * replacing that one function is enough to run the real React app against fake data.
 * Calls are recorded on `window.__calls` so a test can assert exactly what the window
 * would have sent to Rust — argument names included, which is the seam a typecheck
 * cannot see: `parentKey` versus `parent_key` compiles either way and fails only at
 * runtime.
 *
 * Events work the way Tauri delivers them: `listen` registers a handler through
 * `plugin:event|listen`, and the stub calls it for each `menlo://progress` tick while
 * `apply_plan` runs.
 */

import { test as base, type Page } from "@playwright/test";
import type { AppView, Plan, PlanEntry, RunView } from "../src/lib/types";

const HOME = "/Users/ada";

export const VIEW: AppView = {
  home: HOME,
  sources: [
    { path: `${HOME}/Downloads`, label: "Downloads", count: 4, exists: true },
    { path: `${HOME}/Desktop`, label: "Desktop", count: 1, exists: true },
    { path: `${HOME}/Documents/Inbox`, label: "Inbox", count: 2, exists: true },
  ],
  destinations: [
    { key: "documents", label: "Documents", path: `${HOME}/Documents`, brief: "" },
    { key: "documents-contracts", label: "Contracts", path: `${HOME}/Documents/Contracts`, brief: "", parent: "documents" },
    { key: "images", label: "Pictures", path: `${HOME}/Pictures`, brief: "" },
    { key: "images-screenshots", label: "Screenshots", path: `${HOME}/Pictures/Screenshots`, brief: "", parent: "images" },
  ],
  folder_sets: [
    {
      id: "everyday",
      label: "Everyday",
      ai: false,
      sources: [`${HOME}/Downloads`, `${HOME}/Desktop`],
      destinations: ["documents", "images"],
      pinned: true,
    },
  ],
  settings: {
    allow_installers: false,
    check_open_files: true,
    profile_name: "Ada Lovelace",
    avatar: "menlo",
    working_model: "fully_local",
    schedule: "manual",
    duplicates: "ask",
    rename_unclear: true,
    ask_before_moving: true,
    include_hidden: false,
    notify_on_finish: true,
    keep_history: true,
  },
  rules: [
    {
      id: "r1",
      destination_key: "documents-contracts",
      text: "Anything with the word lease or agreement goes here",
      understood: true,
      reading: "Takes names with “lease” or “agreement”",
    },
  ],
  runs: [],
  avatar_image: null,
};

function entry(over: Partial<PlanEntry> & { id: string; name: string }): PlanEntry {
  return {
    ext: over.name.split(".").pop() ?? "",
    size_bytes: 184320,
    action: "move",
    destination_key: "documents",
    rename_to: null,
    confidence: 1,
    reason: "A .pdf file → Documents.",
    suggest_rule: null,
    resolved_by: "deterministic",
    included: true,
    overridden: false,
    duplicate: null,
    on_duplicate: null,
    ...over,
  };
}

export const PLAN: Plan = {
  batch_id: "run-1",
  source: `${HOME}/Downloads`,
  sources: [`${HOME}/Downloads`, `${HOME}/Desktop`],
  created_at: "2026-09-22T10:00:00Z",
  entries: [
    entry({
      id: "f_000",
      name: "Lease_Agreement_signed.pdf",
      destination_key: "documents-contracts",
      reason: "Your rule for Contracts: “Anything with the word lease or agreement goes here”",
    }),
    entry({
      id: "f_001",
      name: "Screenshot 2026-09-02 at 10.14.33.png",
      destination_key: "images-screenshots",
      rename_to: "Screenshots 2026-09-02 10.14.33.png",
      reason: "Named like Pictures/Screenshots.",
    }),
    entry({
      id: "f_002",
      name: "Invoice.pdf",
      duplicate: { kind: "identical", existing_name: "Invoice.pdf" },
    }),
    entry({
      id: "f_003",
      name: "report.pdf",
      duplicate: { kind: "same_name", existing_name: "report.pdf" },
    }),
    entry({
      id: "f_004",
      name: "notes.xyz",
      action: "needs_review",
      destination_key: null,
      confidence: 0,
      included: false,
      reason: "No rule matched.",
    }),
  ],
  skipped: [{ name: ".DS_Store", reason: "system_file" }],
  truncated: false,
  prompt_version: null,
};

export const RUN: RunView = {
  summary: {
    batch_id: "run-1",
    started_at: "2026-09-22T10:01:00Z",
    source: `${HOME}/Downloads`,
    set_label: "Everyday",
    moved: 3,
    trashed: 1,
    kept: 0,
    failed: 0,
    reverted: 0,
    fully_reverted: false,
  },
  groups: [
    { key: "documents-contracts", label: "Documents/Contracts", trash: false, count: 1, restored: 0 },
    { key: "images-screenshots", label: "Pictures/Screenshots", trash: false, count: 1, restored: 0 },
    { key: "documents", label: "Documents", trash: false, count: 1, restored: 0 },
    { key: null, label: "Trash", trash: true, count: 1, restored: 0 },
  ],
  stayed_put: 0,
};

export interface RecordedCall {
  cmd: string;
  args: Record<string, unknown>;
}

declare global {
  interface Window {
    __calls: RecordedCall[];
  }
}

export async function installIpc(page: Page, opts: { picked?: string[] } = {}) {
  await page.addInitScript(
    ({ view, plan, run, picked }) => {
      window.__calls = [];
      const w = window as unknown as Record<string, unknown>;
      const listeners: Record<string, (e: unknown) => void> = {};
      let state = structuredClone(view);

      const handlers: Record<string, (a: Record<string, unknown>) => unknown> = {
        get_state: () => state,
        save_settings: (a) => (state = { ...state, settings: a.settings as typeof state.settings }),
        set_avatar: (a) =>
          (state = { ...state, settings: { ...state.settings, avatar: a.id as string } }),
        save_avatar: (a) =>
          (state = {
            ...state,
            settings: { ...state.settings, avatar: "custom" },
            avatar_image: a.dataUrl as string,
          }),
        clear_avatar: () =>
          (state = { ...state, settings: { ...state.settings, avatar: "menlo" }, avatar_image: null }),
        add_source: (a) =>
          (state = {
            ...state,
            sources: [...state.sources, { path: a.path as string, label: "New", count: 0, exists: true }],
          }),
        add_folder_rule: (a) =>
          (state = {
            ...state,
            rules: [
              ...state.rules,
              {
                id: `r${state.rules.length + 1}`,
                destination_key: a.destinationKey as string,
                text: a.text as string,
                understood: true,
                reading: "Takes files",
              },
            ],
          }),
        plan_run: () => plan,
        apply_plan: (a) => {
          const sent = a.plan as typeof plan;
          const moving = sent.entries.filter((e) => e.included && e.action === "move" && e.destination_key);
          moving.forEach((e, i) => {
            const trashed = e.on_duplicate === "trash";
            const kept = e.duplicate?.kind === "identical" && e.on_duplicate !== "trash";
            listeners["menlo://progress"]?.({
              event: "menlo://progress",
              id: 1,
              payload: {
                done: i + 1,
                total: moving.length,
                entry_id: e.id,
                destination_key: e.destination_key,
                outcome: trashed ? "trashed" : kept ? "kept" : "moved",
              },
            });
          });
          // Like the real journal: the run now shows up in Runs.
          state = { ...state, runs: [run.summary, ...state.runs] };
          return {
            batch_id: sent.batch_id,
            stopped: false,
            moved: 3,
            trashed: 1,
            kept: 0,
            failures: [],
          };
        },
        get_run: () => run,
        stop_run: () => null,
        "plugin:event|listen": (a) => {
          listeners[a.event as string] = a.handler as (e: unknown) => void;
          return 1;
        },
        "plugin:event|unlisten": () => null,
        "plugin:dialog|open": () => (picked.length ? picked : null),
      };

      w.__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
          window.__calls.push({ cmd, args });
          const h = handlers[cmd];
          if (h) return h(args);
          throw { kind: "config", message: `no stub for ${cmd}`, details: [] };
        },
        // Tauri hands Rust an id and calls a window function; here the handler is
        // passed straight through so the stub can call it directly.
        transformCallback: (cb: unknown) => cb,
        unregisterCallback: () => {},
        convertFileSrc: (p: string) => p,
      };
      w.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    },
    { view: VIEW, plan: PLAN, run: RUN, picked: opts.picked ?? [] },
  );
}

export const test = base.extend<{ app: Page }>({
  app: async ({ page }, use) => {
    await installIpc(page, { picked: ["/Users/ada/Projects/old-exports"] });
    await page.goto("/");
    await use(page);
  },
});

/** Every call the window made to one command, in order. */
export async function callsTo(page: Page, cmd: string): Promise<Record<string, unknown>[]> {
  return page.evaluate((c) => window.__calls.filter((x) => x.cmd === c).map((x) => x.args), cmd);
}

export { expect } from "@playwright/test";
