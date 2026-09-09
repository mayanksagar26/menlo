/**
 * A stubbed Tauri IPC, injected before the app's own scripts run.
 *
 * Tauri v2 routes every `invoke` through `window.__TAURI_INTERNALS__.invoke`, so
 * replacing that one function is enough to run the real React app against fake data.
 * Calls are recorded on `window.__calls` so a test can assert what the frontend would
 * have sent to Rust — which is the part that matters for the apply flow.
 */

import { test as base, type Page } from "@playwright/test";
import type { Plan, PlanEntry, SetupState } from "../src/lib/types";

export const DESTINATIONS = [
  { key: "finance", label: "Finance", path: "/tmp/Filed/finance", brief: "Invoices." },
  { key: "documents", label: "Documents", path: "/tmp/Filed/documents", brief: "Papers." },
  { key: "images", label: "Images", path: "/tmp/Filed/images", brief: "Photos." },
];

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
    ...over,
  };
}

export const PLAN: Plan = {
  batch_id: "test-batch",
  source: "/Users/ada/Downloads",
  created_at: "2026-09-08T10:00:00Z",
  entries: [
    entry({ id: "f_000", name: "Invoice_Aug2026.pdf", destination_key: "finance",
            resolved_by: "learned",
            reason: "Learned from an earlier approval: the name matches Invoice_*.pdf." }),
    entry({ id: "f_001", name: "annual_report.pdf" }),
    entry({ id: "f_002", name: "IMG_4821.heic", destination_key: "images",
            reason: "A .heic file → Images." }),
    entry({ id: "f_003", name: "Q3_forecast.xlsx", action: "needs_review",
            destination_key: null, confidence: 0, included: false,
            reason: "No rule matched." }),
  ],
  skipped: [{ name: ".DS_Store", reason: "system_file" }],
  truncated: false,
  prompt_version: null,
};

const SETUP: SetupState = {
  config: {
    source: "/Users/ada/Downloads",
    destinations: DESTINATIONS,
    settings: { allow_installers: false, check_open_files: true },
  },
  suggested_keys: ["documents", "images", "finance"],
  rules: { rules: [] },
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

export async function installIpc(page: Page) {
  await page.addInitScript(
    ({ setup, plan }) => {
      window.__calls = [];
      const responses: Record<string, unknown> = {
        get_setup_state: setup,
        scan_and_plan: plan,
        get_rules: { rules: [] },
        list_batches: [],
        apply_plan: { batch_id: "test-batch", moved: 3, failures: [], learned: [] },
      };
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {
        invoke: async (cmd: string, args: Record<string, unknown>) => {
          window.__calls.push({ cmd, args });
          if (cmd in responses) return responses[cmd];
          throw { kind: "config", message: `no stub for ${cmd}`, details: [] };
        },
        transformCallback: (cb: unknown) => cb,
        unregisterCallback: () => {},
        convertFileSrc: (p: string) => p,
      };
    },
    { setup: SETUP, plan: PLAN },
  );
}

export const test = base.extend<{ app: Page }>({
  app: async ({ page }, use) => {
    await installIpc(page);
    await page.goto("/");
    await use(page);
  },
});

export { expect } from "@playwright/test";
