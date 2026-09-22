/**
 * The run, as a sequence: plan → duplicates → preview → move → receipt.
 *
 * Each step is a real call. Planning scans and routes in Rust; the duplicate sheet
 * appears only when there is a byte-identical copy to decide about and the setting
 * says to ask; the preview appears only when the setting says to confirm. The move
 * streams real progress and can be stopped between files.
 *
 * Duplicates are asked about the moment Move is pressed, before the first file goes,
 * rather than by pausing half way through. It is the same decision, and it means a
 * run is never left open in the backend waiting on the window.
 */

import { identicalDuplicates, useApp } from "../store/app";
import { presetLabel, selectedDests, selectedSources, useUi } from "../store/ui";

/** Long enough that the sorting sheet reads as a step, not a flicker. */
const MIN_PLANNING_MS = 900;

export async function beginRun() {
  const ui = useUi.getState();
  const app = useApp.getState();
  const view = app.view;
  if (!view) return;

  const sources = selectedSources(view, ui).map((s) => s.path);
  const dests = selectedDests(view, ui).map((d) => d.key);

  ui.set({ planning: true });
  const started = Date.now();
  const plan = await app.makePlan(sources, dests, presetLabel(view, ui.preset));
  const wait = MIN_PLANNING_MS - (Date.now() - started);
  if (wait > 0) await new Promise((r) => setTimeout(r, wait));
  useUi.getState().set({ planning: false });
  if (!plan) return;

  if (view.settings.duplicates === "ask" && identicalDuplicates(plan).length > 0) {
    useUi.getState().set({ dupesOpen: true });
    return;
  }
  afterDuplicates();
}

export function afterDuplicates() {
  const ui = useUi.getState();
  ui.set({ dupesOpen: false });
  if (useApp.getState().view?.settings.ask_before_moving) {
    ui.set({ confirmOpen: true });
  } else {
    void startMove();
  }
}

export async function startMove() {
  const ui = useUi.getState();
  ui.set({ confirmOpen: false, step: 2, moving: true });
  const outcome = await useApp.getState().apply();
  useUi.getState().set({ moving: false });
  // A stopped run stays on the Move stage, where Restore is offered.
  if (outcome && !outcome.stopped) {
    // A beat, so the last card can finish landing before the receipt slides up.
    setTimeout(() => useUi.getState().set({ step: 3 }), 700);
  }
}

export function cancelRun() {
  useUi.getState().set({ planning: false, dupesOpen: false, confirmOpen: false });
  useApp.getState().clearRun();
}

export async function stopMove() {
  await useApp.getState().stop();
}

/** After a Stop: send back what did land, and return to Shape. */
export async function restoreStopped() {
  const outcome = useApp.getState().outcome;
  if (outcome) await useApp.getState().revertRun(outcome.batch_id);
  useApp.getState().clearRun();
  useUi.getState().set({ step: 1 });
}

export function finishRun() {
  useApp.getState().clearRun();
  useUi.getState().goHome();
}

// ── Suggest folders ─────────────────────────────────────────────────────────

export async function openSuggest() {
  const ui = useUi.getState();
  ui.set({ suggest: { open: true, loading: true, items: [] } });
  const items = await useApp.getState().suggest(ui.shapeFolder);
  useUi.getState().set({ suggest: { open: true, loading: false, items } });
}

export function closeSuggest() {
  useUi.getState().set({ suggest: { open: false, loading: false, items: [] } });
}
