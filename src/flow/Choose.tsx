import { useState } from "react";
import { pickFolders } from "../lib/ipc";
import { subfoldersOf, tilde, topLevel, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { PlusIcon } from "../ui/glyphs";
import { DashedChip, Eyebrow, PickRow, SearchField } from "../ui/primitives";

/**
 * Stage 1 — CHOOSE.
 *
 * Two lists, deliberately symmetrical: the places files are leaving and the places
 * they can land. Either can take any folder on the Mac — Add folder opens the system
 * picker, and what is picked is selected for this run straight away.
 */
export function Choose() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const addSources = useApp((s) => s.addSources);
  const addDestinations = useApp((s) => s.addDestinations);
  const [fromQuery, setFromQuery] = useState("");
  const [toQuery, setToQuery] = useState("");

  if (!view) return null;

  const allDests = topLevel(view);
  const sources = view.sources.filter((s) => match(tilde(view, s.path), fromQuery));
  const dests = allDests.filter((d) => match(d.label, toQuery) || match(tilde(view, d.path), toQuery));

  const srcCount = view.sources.filter((s) => ui.src[s.path]).length;
  const dstCount = allDests.filter((d) => ui.dst[d.key]).length;

  async function addFrom() {
    const paths = await pickFolders("Choose folders to clear");
    if (!paths.length) return;
    // The backend stores canonical paths, which need not equal what the picker
    // returned — so select whatever is new, not the raw picks.
    const before = new Set(useApp.getState().view?.sources.map((s) => s.path));
    await addSources(paths);
    const added = (useApp.getState().view?.sources ?? [])
      .map((s) => s.path)
      .filter((p) => !before.has(p));
    ui.pickSrc(added);
  }

  async function addTo() {
    const paths = await pickFolders("Choose folders files can land in");
    if (!paths.length) return;
    const before = new Set(useApp.getState().view?.destinations.map((d) => d.key));
    await addDestinations(paths);
    const added = (useApp.getState().view?.destinations ?? [])
      .filter((d) => !d.parent && !before.has(d.key))
      .map((d) => d.key);
    ui.pickDst(added);
  }

  return (
    <div className="flex h-full flex-col px-10 pt-8 pb-4">
      <Eyebrow>Choose</Eyebrow>
      <h2 className="mt-2.5 mb-0 text-[28px] leading-[1.15] font-medium tracking-[-0.022em] text-ink">
        Clear what's piled up.
      </h2>

      <div className="mt-7 grid min-h-0 flex-1 grid-cols-2 gap-[26px]">
        <Column
          title="From"
          summary={`${srcCount} of ${view.sources.length}`}
          query={fromQuery}
          onQuery={setFromQuery}
          placeholder="Search source folders"
          empty={sources.length === 0}
          onAdd={addFrom}
        >
          {sources.map((s) => (
            <PickRow
              key={s.path}
              on={!!ui.src[s.path]}
              name={tilde(view, s.path)}
              meta={!s.exists ? "Missing — it has moved or been deleted" : `${s.count} ${s.count === 1 ? "file" : "files"}`}
              onClick={() => ui.toggleSrc(s.path)}
            />
          ))}
        </Column>

        <Column
          title="To"
          summary={`${dstCount} of ${allDests.length}`}
          query={toQuery}
          onQuery={setToQuery}
          placeholder="Search destination folders"
          empty={dests.length === 0}
          onAdd={addTo}
        >
          {dests.map((d) => {
            const subs = subfoldersOf(view, d.key).map((s) => s.label);
            return (
              <PickRow
                key={d.key}
                on={!!ui.dst[d.key]}
                name={d.label}
                // The subfolders it already has — never a file count. Which file goes
                // where is not revealed until the move preview.
                meta={subs.length ? subs.join(" · ") : tilde(view, d.path)}
                onClick={() => ui.toggleDst(d.key)}
              />
            );
          })}
        </Column>
      </div>
    </div>
  );
}

function Column({
  title,
  summary,
  query,
  onQuery,
  placeholder,
  empty,
  onAdd,
  children,
}: {
  title: string;
  summary: string;
  query: string;
  onQuery: (q: string) => void;
  placeholder: string;
  empty: boolean;
  onAdd: () => void;
  children: React.ReactNode;
}) {
  return (
    <section className="flex min-h-0 flex-col">
      <div className="mb-2.5 flex items-baseline gap-2">
        <Eyebrow>{title}</Eyebrow>
        <span className="ml-auto text-[11px] text-ink-60">{summary}</span>
      </div>
      <SearchField value={query} onChange={onQuery} placeholder={placeholder} />
      <div className="mt-2.5 flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto pr-1">
        {empty && query ? (
          <p className="mt-6 text-center text-[11.5px] text-ink-60">No folder by that name</p>
        ) : (
          children
        )}
        <DashedChip className="!justify-center !py-2.5 !text-[12px]" onClick={onAdd}>
          <PlusIcon size={12} /> Add folder
        </DashedChip>
      </div>
    </section>
  );
}

function match(value: string, query: string) {
  return value.toLowerCase().includes(query.trim().toLowerCase());
}
