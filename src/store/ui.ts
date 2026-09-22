/**
 * Where the window is, and what the user has picked for this run.
 *
 * One window, one store. Home, the four stages, the five full-window pages and the
 * sheets are all positions in here rather than routes, because the design treats them
 * as one surface that moves — the stage track translates, pages slide over, sheets
 * sit on a scrim — and a router would fight that.
 *
 * Nothing here is data about the disk. Folders, sets, rules and runs come from Rust
 * through `app.ts`; this holds only which of them are selected and which screen is up.
 */

import { create } from "zustand";
import type { AppView, FolderSet, FolderSuggestion } from "../lib/types";

/** Full-window pages that open over the flow. */
export type Page = "docs" | "rules" | "runs" | "settings";

/** "Custom" is not a saved set: it starts the flow with nothing picked. */
export const CUSTOM = "custom";

export interface UiState {
  home: boolean;
  /** 0 Choose · 1 Shape · 2 Move · 3 Files processed. */
  step: 0 | 1 | 2 | 3;
  page: Page | null;
  profileOpen: boolean;

  /** The set this run started from, or `CUSTOM`. */
  preset: string;
  /** Source paths picked for this run. */
  src: Record<string, boolean>;
  /** Top-level destination keys picked for this run. */
  dst: Record<string, boolean>;

  setsOpen: boolean;
  /** The set open in the editor: an id, `"new"`, or null for the list. */
  editSet: string | null;

  /** The destination Shape has drilled into. */
  shapeFolder: string | null;
  /** Destination key the rule popup is adding to. */
  ruleFor: string | null;

  planning: boolean;
  dupesOpen: boolean;
  confirmOpen: boolean;
  suggest: { open: boolean; loading: boolean; items: FolderSuggestion[] };
  expDest: Record<string, boolean>;

  moving: boolean;

  rulesActive: string | null;
  runActive: string | null;
}

interface UiActions {
  openPage: (page: Page) => void;
  closePage: () => void;
  toggleProfile: () => void;
  closeProfile: () => void;

  /** Choose a set on home. Custom has nothing to show, so it opens the flow. */
  chooseSet: (set: FolderSet | typeof CUSTOM) => void;
  enterFlow: () => void;
  goStep: (step: 0 | 1 | 2 | 3) => void;
  back: () => void;
  goHome: () => void;

  toggleSrc: (path: string) => void;
  toggleDst: (key: string) => void;
  pickSrc: (paths: string[]) => void;
  pickDst: (keys: string[]) => void;

  enterFolder: (key: string) => void;
  exitFolder: () => void;
  openRuleFor: (key: string | null) => void;

  openSets: (open: boolean) => void;
  openEditor: (id: string | null) => void;

  set: (patch: Partial<UiState>) => void;
}

export type Ui = UiState & UiActions;

export const useUi = create<Ui>((set, get) => ({
  home: true,
  step: 0,
  page: null,
  profileOpen: false,

  preset: CUSTOM,
  src: {},
  dst: {},

  setsOpen: false,
  editSet: null,

  shapeFolder: null,
  ruleFor: null,

  planning: false,
  dupesOpen: false,
  confirmOpen: false,
  suggest: { open: false, loading: false, items: [] },
  expDest: {},

  moving: false,

  rulesActive: null,
  runActive: null,

  openPage: (page) => set({ page, profileOpen: false }),
  closePage: () => set({ page: null }),
  toggleProfile: () => set((s) => ({ profileOpen: !s.profileOpen })),
  closeProfile: () => set({ profileOpen: false }),

  chooseSet: (s) => {
    if (s === CUSTOM) {
      set({ preset: CUSTOM, src: {}, dst: {}, home: false, step: 0, setsOpen: false });
      return;
    }
    set({
      preset: s.id,
      src: Object.fromEntries(s.sources.map((p) => [p, true])),
      dst: Object.fromEntries(s.destinations.map((k) => [k, true])),
      setsOpen: false,
      editSet: null,
    });
  },

  enterFlow: () => set({ home: false, step: 0, shapeFolder: null }),
  goStep: (step) => set({ step }),

  back: () => {
    const s = get();
    // Out of a drill-down first, then back a stage, then out to home.
    if (s.shapeFolder) return set({ shapeFolder: null });
    if (s.step === 1) return set({ step: 0 });
    if (s.step === 2 || s.step === 3) return set({ step: 1 });
    get().goHome();
  },

  goHome: () => set({ home: true, step: 0, shapeFolder: null, moving: false }),

  toggleSrc: (path) => set((s) => ({ src: { ...s.src, [path]: !s.src[path] } })),
  toggleDst: (key) => set((s) => ({ dst: { ...s.dst, [key]: !s.dst[key] } })),
  pickSrc: (paths) =>
    set((s) => ({ src: { ...s.src, ...Object.fromEntries(paths.map((p) => [p, true])) } })),
  pickDst: (keys) =>
    set((s) => ({ dst: { ...s.dst, ...Object.fromEntries(keys.map((k) => [k, true])) } })),

  enterFolder: (key) => set({ shapeFolder: key }),
  exitFolder: () => set({ shapeFolder: null }),
  openRuleFor: (key) => set({ ruleFor: key }),

  openSets: (open) => set({ setsOpen: open, editSet: null, profileOpen: false }),
  openEditor: (id) => set({ setsOpen: true, editSet: id }),

  set: (patch) => set(patch),
}));

// ── Derived ─────────────────────────────────────────────────────────────────
// Selectors rather than stored fields: counts are always a function of the
// selection and the view, and storing them is how the two drift apart.

/** Sources picked for this run that still exist in the config. */
export function selectedSources(view: AppView | null, ui: Pick<UiState, "src">) {
  return view?.sources.filter((s) => ui.src[s.path]) ?? [];
}

/** Top-level destinations picked for this run. */
export function selectedDests(view: AppView | null, ui: Pick<UiState, "dst">) {
  return view?.destinations.filter((d) => !d.parent && ui.dst[d.key]) ?? [];
}

/** Files sitting in the picked sources — the number the flow counts in. */
export function fileCount(view: AppView | null, ui: Pick<UiState, "src">): number {
  return selectedSources(view, ui).reduce((a, s) => a + s.count, 0);
}

/** A set's display name, or "Custom". */
export function presetLabel(view: AppView | null, preset: string): string | null {
  if (preset === CUSTOM) return null;
  return view?.folder_sets.find((s) => s.id === preset)?.label ?? null;
}
