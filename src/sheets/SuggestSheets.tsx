import { useState } from "react";
import { closeSuggest } from "../flow/run";
import { destination, tilde, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { FolderGlyph } from "../ui/glyphs";
import { Eyebrow, Modal, SelectDot, SheetFooter } from "../ui/primitives";
import { ShuffleField } from "./Thinking";

/**
 * Suggest folders.
 *
 * Drafted by counting what is actually sitting in the sources — no model involved, so
 * every reason shown is a number Menlo counted, never a guess. At the top level it
 * offers a home folder for each kind of file with nowhere to land; inside a folder, the
 * subfolders already on disk that Menlo has not been told about.
 */
export function SuggestSheet() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const addSuggestion = useApp((s) => s.addSuggestion);
  const [picked, setPicked] = useState<Record<string, boolean>>({});
  const [adding, setAdding] = useState(false);

  const { loading, items } = ui.suggest;
  const folder = destination(view, ui.shapeFolder);
  const chosen = items.filter((s) => picked[s.path]);

  async function add() {
    setAdding(true);
    const before = new Set(useApp.getState().view?.destinations.map((d) => d.key));
    for (const s of chosen) await addSuggestion(s);
    // A new top-level folder joins this run straight away.
    if (!folder) {
      const added = (useApp.getState().view?.destinations ?? [])
        .filter((d) => !d.parent && !before.has(d.key))
        .map((d) => d.key);
      ui.pickDst(added);
    }
    setAdding(false);
    closeSuggest();
  }

  if (loading) {
    return (
      <Modal onClose={closeSuggest} width={420} labelledBy="scan-title">
        <div className="flex flex-col gap-5 p-6">
          <div>
            <Eyebrow>Scanning</Eyebrow>
            <h2 id="scan-title" className="mt-2.5 mb-0 text-[20px] leading-[1.25] font-medium tracking-[-0.02em] text-ink">
              {folder ? `Looking inside ${folder.label}` : "Reading the files in your sources"}
            </h2>
          </div>
          <ShuffleField height={104} />
        </div>
      </Modal>
    );
  }

  return (
    <Modal onClose={closeSuggest} width={480} labelledBy="suggest-title">
      <div className="p-6 pb-3">
        <h2 id="suggest-title" className="m-0 text-[21px] leading-[1.25] font-medium tracking-[-0.02em] text-ink">
          {folder ? `Subfolders for ${folder.label}` : "Folders your files could use"}
        </h2>
        <p className="mt-2 mb-0 text-[11.5px] leading-[1.6] text-ink-66">
          {folder
            ? `Folders already inside ${folder.label} on disk that Menlo does not know about yet.`
            : "A folder for each kind of file sitting in your sources with nowhere to land."}
        </p>
      </div>

      <div className="flex max-h-[300px] flex-col gap-1.5 overflow-y-auto px-4 pb-2">
        {items.length === 0 && (
          <p className="py-6 text-center text-[11.5px] leading-[1.6] text-ink-60">
            {folder
              ? `Every folder inside ${folder.label} is already here. Use Add subfolder to make a new one.`
              : "Every kind of file in your sources already has somewhere to go."}
          </p>
        )}
        {items.map((s) => {
          const on = !!picked[s.path];
          return (
            <button
              key={s.path}
              type="button"
              onClick={() => setPicked((p) => ({ ...p, [s.path]: !p[s.path] }))}
              aria-pressed={on}
              className="flex items-center gap-3 rounded-[11px] px-3 py-2.5 text-left transition-colors duration-200 hover:bg-glass-hi"
              style={{ transitionTimingFunction: "var(--ease-menlo)" }}
            >
              <span className="text-ink-60">
                <FolderGlyph dashed={!s.exists} />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[12.5px] text-ink-86">
                  {s.label}
                  <span className="text-ink-60"> · {tilde(view, s.path)}</span>
                </span>
                <span className="block truncate text-[11px] text-ink-60">
                  {s.reason}
                  {s.exists ? "" : " — will be created"}
                </span>
              </span>
              <SelectDot on={on} />
            </button>
          );
        })}
      </div>

      <SheetFooter note={chosen.length ? `${chosen.length} selected` : undefined}>
        <button type="button" className="btn-glass" onClick={closeSuggest}>
          Cancel
        </button>
        <button type="button" className="btn-solid" disabled={!chosen.length || adding} onClick={add}>
          Add {chosen.length || ""}
        </button>
      </SheetFooter>
    </Modal>
  );
}
