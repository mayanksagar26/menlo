import { cancelRun, startMove } from "../flow/run";
import type { PlanEntry } from "../lib/types";
import { actionable, pathLabel, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { DisclosureRow, Modal, SheetFooter } from "../ui/primitives";
import { formatBytes } from "../lib/format";

/**
 * The confirmation.
 *
 * This is the only screen that reveals where each file is actually going, and it is
 * the last thing between the user and files moving — so it opens all the way down to
 * filenames, sizes, and the new name wherever Menlo proposes one. Nothing is committed
 * by scrolling it; the button commits.
 */
export function MovePreview() {
  const view = useApp((s) => s.view);
  const plan = useApp((s) => s.plan);
  const ui = useUi();

  const entries = actionable(plan);
  // Kept duplicates do not move, so they are not "landings".
  const moving = entries.filter((e) => fate(e, view?.settings.duplicates) !== "keep");
  const trashed = moving.filter((e) => fate(e, view?.settings.duplicates) === "trash");
  const landing = moving.filter((e) => !trashed.includes(e));
  const renamed = landing.filter((e) => e.rename_to);
  const unmatched = plan?.entries.filter((e) => e.action === "needs_review").length ?? 0;

  const byDest = new Map<string, PlanEntry[]>();
  for (const e of landing) {
    const k = e.destination_key!;
    byDest.set(k, [...(byDest.get(k) ?? []), e]);
  }

  const sub = [
    renamed.length && `${renamed.length} ${renamed.length === 1 ? "gets" : "get"} a clearer name`,
    trashed.length && `${trashed.length} ${trashed.length === 1 ? "duplicate goes" : "duplicates go"} to the Trash`,
    unmatched && `${unmatched} matched no rule and ${unmatched === 1 ? "stays" : "stay"} where ${unmatched === 1 ? "it is" : "they are"}`,
  ].filter(Boolean);

  return (
    <Modal onClose={cancelRun} width={620} labelledBy="preview-title">
      <div className="p-6 pb-4">
        <h2 id="preview-title" className="m-0 text-[21px] leading-[1.25] font-medium tracking-[-0.02em] text-ink">
          {landing.length} {landing.length === 1 ? "file has" : "files have"} a place
        </h2>
        <p className="mt-2 mb-0 text-[11.5px] leading-[1.6] text-ink-66">
          {sub.length ? `${sub.join(" · ")}. ` : ""}Nothing is overwritten, and the whole run can be sent back
          from Runs.
        </p>
      </div>

      {/* A block container, not a flex column: a flex child would compress these rows
          down to fit instead of letting the list scroll. */}
      <div className="max-h-[352px] overflow-y-auto px-4 pb-2">
        {[...byDest.entries()].map(([key, files]) => {
          const open = !!ui.expDest[key];
          return (
            <div key={key}>
              <DisclosureRow
                open={open}
                title={pathLabel(view, key)}
                meta={`${files.length} ${files.length === 1 ? "file" : "files"}`}
                onClick={() => ui.set({ expDest: { ...ui.expDest, [key]: !open } })}
              />
              {open &&
                files.map((f) => (
                  <div key={f.id} className="flex items-center gap-3 py-1.5 pr-3 pl-[34px]">
                    <span className="selectable min-w-0 flex-1 truncate text-[11.5px] text-ink-70">
                      {f.rename_to ? (
                        <>
                          <span className="text-ink-60">{f.name}</span>
                          <span className="px-1.5 text-ink-60">→</span>
                          <span className="text-ink-86">{f.rename_to}</span>
                        </>
                      ) : (
                        f.name
                      )}
                    </span>
                    <span className="shrink-0 text-[11px] text-ink-60">{formatBytes(f.size_bytes)}</span>
                  </div>
                ))}
            </div>
          );
        })}

        {trashed.length > 0 && (
          <DisclosureRow
            open={!!ui.expDest.__trash}
            title="Trash — duplicates already filed"
            meta={`${trashed.length} ${trashed.length === 1 ? "file" : "files"}`}
            onClick={() => ui.set({ expDest: { ...ui.expDest, __trash: !ui.expDest.__trash } })}
          />
        )}
        {ui.expDest.__trash &&
          trashed.map((f) => (
            <div key={f.id} className="flex items-center gap-3 py-1.5 pr-3 pl-[34px]">
              <span className="selectable min-w-0 flex-1 truncate text-[11.5px] text-ink-70">{f.name}</span>
              <span className="shrink-0 text-[11px] text-ink-60">{formatBytes(f.size_bytes)}</span>
            </div>
          ))}
      </div>

      <SheetFooter>
        <button type="button" className="btn-glass" onClick={cancelRun}>
          Cancel
        </button>
        {/* Counts landings, like the headline; the Trash is already named in the line
            under it. A run that only clears duplicates still needs a button. */}
        <button type="button" className="btn-solid" disabled={moving.length === 0} onClick={startMove}>
          {landing.length > 0 ? `Move ${landing.length}` : `Trash ${trashed.length}`}
        </button>
      </SheetFooter>
    </Modal>
  );
}

/**
 * What will happen to an entry, mirroring `execute::choice_for`: an explicit choice
 * wins; an unanswered byte-identical copy follows the setting, with Ask meaning keep;
 * a name clash moves alongside.
 */
function fate(e: PlanEntry, policy: string | undefined): "move" | "keep" | "trash" {
  if (!e.duplicate) return "move";
  if (e.on_duplicate === "keep") return "keep";
  if (e.on_duplicate === "trash") return e.duplicate.kind === "identical" ? "trash" : "keep";
  if (e.on_duplicate === "move_anyway") return "move";
  if (e.duplicate.kind === "same_name") return "move";
  return policy === "trash" ? "trash" : "keep";
}
