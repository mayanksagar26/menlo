import { useState } from "react";
import type { FolderSet } from "../lib/types";
import { tilde, topLevel, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { FolderGlyph, StackedFolders } from "../ui/glyphs";
import { DashedChip, Eyebrow, Modal, SearchField, SelectDot, SheetFooter } from "../ui/primitives";

/** The home screen has room for this many pinned sets (`config::PIN_LIMIT`). */
const PIN_LIMIT = 9;

/**
 * Folder sets — the list, and the editor behind it.
 *
 * Home has room for nine sets, so pinning is a real decision and the row says which
 * state it is in rather than making the user count. Deleting is in the editor, one
 * level down, where the set being deleted is the thing you are looking at.
 */
export function FolderSets() {
  const editSet = useUi((s) => s.editSet);
  return editSet ? <SetEditor id={editSet} /> : <SetList />;
}

function SetList() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const saveSet = useApp((s) => s.saveSet);
  const close = () => ui.openSets(false);
  if (!view) return null;

  const sets = view.folder_sets;
  const pinnedCount = sets.filter((s) => s.pinned).length;
  const full = pinnedCount >= PIN_LIMIT;

  return (
    <Modal onClose={close} width={480} labelledBy="sets-title">
      <div className="p-6 pb-3">
        <h2 id="sets-title" className="m-0 text-[21px] leading-[1.25] font-medium tracking-[-0.02em] text-ink">
          Folder sets
        </h2>
      </div>

      <div className="flex max-h-[360px] flex-col gap-1.5 overflow-y-auto px-4 pb-2">
        {sets.map((s) => (
          <div
            key={s.id}
            className="flex items-center gap-3 rounded-[11px] px-3 py-2.5 transition-colors duration-200 hover:bg-glass-hi"
            style={{ transitionTimingFunction: "var(--ease-menlo)" }}
          >
            <span className="text-ink-70">
              <StackedFolders count={Math.max(1, s.sources.length)} open={false} />
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-center gap-1.5">
                <span className="truncate text-[12.5px] text-ink-86">{s.label}</span>
                {s.ai && (
                  <span className="text-[10.5px] text-ink-60" title="Drafted by Menlo">
                    ✦
                  </span>
                )}
              </span>
              <span className="block truncate text-[11px] text-ink-60">
                {s.sources.map((p) => tilde(view, p)).join(" · ") || "No folders yet"}
              </span>
            </span>

            <button
              type="button"
              onClick={() => ui.openEditor(s.id)}
              aria-label={`Edit ${s.label}`}
              className="shrink-0 text-[11px] text-ink-60 transition-colors duration-200 hover:text-ink"
            >
              ✎
            </button>
            <button
              type="button"
              onClick={() => saveSet({ ...s, pinned: !s.pinned })}
              disabled={!s.pinned && full}
              className="shrink-0 rounded-full px-2.5 py-1 text-[11px] transition-colors duration-200"
              style={{
                background: s.pinned ? "var(--color-glass-sel)" : "transparent",
                border: `1px solid ${s.pinned ? "var(--color-hair-rim)" : "var(--color-hair)"}`,
                color: s.pinned ? "var(--color-ink)" : "var(--color-ink-60)",
                opacity: !s.pinned && full ? 0.45 : 1,
              }}
            >
              {s.pinned ? "On home" : full ? "Home full" : "Add"}
            </button>
          </div>
        ))}

        <DashedChip className="!justify-center !py-3" onClick={() => ui.openEditor("new")}>
          + Create folder set
        </DashedChip>
      </div>

      <SheetFooter note={`${sets.length} ${sets.length === 1 ? "set" : "sets"} · ${pinnedCount} of ${PIN_LIMIT} on home`}>
        <button type="button" className="btn-solid" onClick={close}>
          Done
        </button>
      </SheetFooter>
    </Modal>
  );
}

function SetEditor({ id }: { id: string }) {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const saveSet = useApp((s) => s.saveSet);
  const deleteSet = useApp((s) => s.deleteSet);

  const existing = view?.folder_sets.find((s) => s.id === id);
  const [draft, setDraft] = useState<FolderSet>(
    () =>
      existing ?? {
        id: "",
        label: "New set",
        ai: false,
        sources: [],
        destinations: topLevel(view).map((d) => d.key),
        pinned: (view?.folder_sets.filter((s) => s.pinned).length ?? 0) < PIN_LIMIT,
      },
  );
  const [query, setQuery] = useState("");
  if (!view) return null;

  const sources = view.sources.filter((s) =>
    tilde(view, s.path).toLowerCase().includes(query.trim().toLowerCase()),
  );
  const toggle = (field: "sources" | "destinations", value: string) =>
    setDraft((d) => ({
      ...d,
      [field]: d[field].includes(value) ? d[field].filter((v) => v !== value) : [...d[field], value],
    }));

  async function save() {
    await saveSet(draft);
    ui.openEditor(null);
  }

  return (
    <Modal onClose={() => ui.openSets(false)} width={460} labelledBy="editor-title">
      <div className="flex flex-col gap-3 p-6 pb-3">
        <h2 id="editor-title" className="sr-only">
          {existing ? "Edit folder set" : "New folder set"}
        </h2>
        <input
          className="field !h-[38px] !text-[15px]"
          value={draft.label}
          aria-label="Set name"
          autoFocus={!existing}
          onChange={(e) => setDraft((d) => ({ ...d, label: e.target.value }))}
        />
        <Eyebrow>Clear from</Eyebrow>
        <SearchField value={query} onChange={setQuery} placeholder="Search source folders" />
      </div>

      <div className="flex max-h-[300px] flex-col gap-1 overflow-y-auto px-4 pb-2">
        {sources.map((s) => {
          const on = draft.sources.includes(s.path);
          return (
            <button
              key={s.path}
              type="button"
              onClick={() => toggle("sources", s.path)}
              aria-pressed={on}
              className="flex items-center gap-3 rounded-[11px] px-3 py-2 text-left transition-colors duration-200 hover:bg-glass-hi"
            >
              <span className="text-ink-60">
                <FolderGlyph />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[12.5px] text-ink-86">{tilde(view, s.path)}</span>
                <span className="block text-[11px] text-ink-60">{s.count} files</span>
              </span>
              <SelectDot on={on} />
            </button>
          );
        })}

        <div className="px-2 pt-3 pb-1">
          <Eyebrow>File into</Eyebrow>
        </div>
        {topLevel(view).map((d) => {
          const on = draft.destinations.includes(d.key);
          return (
            <button
              key={d.key}
              type="button"
              onClick={() => toggle("destinations", d.key)}
              aria-pressed={on}
              className="flex items-center gap-3 rounded-[11px] px-3 py-2 text-left transition-colors duration-200 hover:bg-glass-hi"
            >
              <span className="text-ink-60">
                <FolderGlyph />
              </span>
              <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink-86">{d.label}</span>
              <SelectDot on={on} />
            </button>
          );
        })}
      </div>

      <SheetFooter
        note={`${draft.sources.length} ${draft.sources.length === 1 ? "folder" : "folders"} → ${draft.destinations.length}`}
      >
        {existing && (
          <button
            type="button"
            className="btn-glass !text-ink-60"
            onClick={async () => {
              await deleteSet(existing.id);
              ui.openEditor(null);
            }}
          >
            Delete set
          </button>
        )}
        <button type="button" className="btn-glass" onClick={() => ui.openEditor(null)}>
          ← All sets
        </button>
        <button type="button" className="btn-solid" disabled={!draft.sources.length} onClick={save}>
          Save
        </button>
      </SheetFooter>
    </Modal>
  );
}
