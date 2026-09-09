import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderTile } from "../components/FolderTile";
import { slugify, tildePath } from "../lib/format";
import type { Destination } from "../lib/types";
import { useStore } from "../store";

/**
 * First run. Pick one source folder, then add destinations — each with a key, a label,
 * a path, and the natural-language brief the Phase 2 classifier will read.
 */
export function Setup() {
  const config = useStore((s) => s.config);
  const suggestedKeys = useStore((s) => s.suggestedKeys);
  const busy = useStore((s) => s.busy);
  const setConfig = useStore((s) => s.setConfig);
  const persistConfig = useStore((s) => s.persistConfig);
  const addDestination = useStore((s) => s.addDestination);
  const removeDestination = useStore((s) => s.removeDestination);
  const scan = useStore((s) => s.scan);

  const [draft, setDraft] = useState<Destination | null>(null);

  async function pickSource() {
    const picked = await open({ directory: true, multiple: false, title: "Choose a folder to tidy" });
    if (typeof picked !== "string") return;
    setConfig({ ...config, source: picked });
    await persistConfig();
  }

  async function pickDestinationFolder() {
    const picked = await open({ directory: true, multiple: false, title: "Choose a destination" });
    if (typeof picked !== "string") return;
    const label = picked.split("/").pop() ?? "Folder";
    setDraft({ key: slugify(label), label, path: picked, brief: "" });
  }

  const keyTaken = draft ? config.destinations.some((d) => d.key === draft.key) : false;
  const canSave = draft !== null && draft.key.length > 0 && !keyTaken;

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col overflow-y-auto px-8 py-10">
      <h1 className="text-[24px] font-semibold tracking-tight">Menlo</h1>
      <p className="mt-1 max-w-md text-ink-soft">
        Point Menlo at a cluttered folder and tell it where things belong. It proposes; you
        approve. Nothing moves on its own.
      </p>

      {/* ── 1. Source ───────────────────────────────────────────────────── */}
      <section className="mt-9">
        <Heading step={1} title="The folder to tidy" />
        <div className="card mt-3 flex items-center gap-3 px-3 py-2.5">
          <span className="mono selectable flex-1 truncate text-ink-soft">
            {config.source ? tildePath(config.source) : "No folder chosen"}
          </span>
          <button type="button" className="btn" onClick={pickSource}>
            {config.source ? "Change" : "Choose folder"}
          </button>
        </div>
      </section>

      {/* ── 2. Destinations ─────────────────────────────────────────────── */}
      <section className="mt-9">
        <Heading step={2} title="Where things belong" />
        <p className="mt-1 text-mute">
          The brief is a plain-English rule. Menlo reads it when a file is not obvious.
        </p>

        {config.destinations.length > 0 && (
          <div className="mt-4 grid grid-cols-3 gap-3">
            {config.destinations.map((d) => (
              <div key={d.key} className="relative">
                <FolderTile label={d.label} destKey={d.key} count={0} />
                <button
                  type="button"
                  onClick={() => removeDestination(d.key)}
                  aria-label={`Remove ${d.label}`}
                  className="absolute right-1.5 top-1.5 text-mute hover:text-oxblood"
                >
                  ✕
                </button>
              </div>
            ))}
          </div>
        )}

        {draft ? (
          <div className="card mt-4 flex flex-col gap-3 px-3 py-3">
            <div className="mono selectable truncate text-mute">{tildePath(draft.path)}</div>
            <div className="grid grid-cols-2 gap-3">
              <label className="flex flex-col gap-1">
                <span className="text-xs text-mute">Name</span>
                <input
                  className="field"
                  value={draft.label}
                  onChange={(e) => setDraft({ ...draft, label: e.target.value })}
                />
              </label>
              <label className="flex flex-col gap-1">
                <span className="text-xs text-mute">Key</span>
                <input
                  className="field mono"
                  value={draft.key}
                  onChange={(e) => setDraft({ ...draft, key: slugify(e.target.value) })}
                />
              </label>
            </div>

            {/* Naming a folder with a built-in key gets tier-1 routing for free. */}
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="text-xs text-mute">Known keys:</span>
              {suggestedKeys.map((k) => (
                <button
                  key={k}
                  type="button"
                  onClick={() => setDraft({ ...draft, key: k })}
                  className={`mono rounded-[3px] border px-1.5 py-px text-[11px] transition-colors ${
                    draft.key === k
                      ? "border-oxblood text-oxblood"
                      : "border-rule text-mute hover:text-ink"
                  }`}
                >
                  {k}
                </button>
              ))}
            </div>
            {keyTaken && <p className="text-oxblood">That key is already in use.</p>}

            <label className="flex flex-col gap-1">
              <span className="text-xs text-mute">
                Brief — what belongs here, in your own words
              </span>
              <textarea
                className="field min-h-[68px] resize-y"
                placeholder="Invoices, receipts, bank statements, anything with a rupee or dollar amount on it."
                value={draft.brief}
                onChange={(e) => setDraft({ ...draft, brief: e.target.value })}
              />
            </label>

            <div className="flex justify-end gap-2">
              <button type="button" className="btn" onClick={() => setDraft(null)}>
                Cancel
              </button>
              <button
                type="button"
                className="btn btn-primary"
                disabled={!canSave}
                onClick={async () => {
                  await addDestination(draft);
                  setDraft(null);
                }}
              >
                Add folder
              </button>
            </div>
          </div>
        ) : (
          <button type="button" className="btn mt-4" onClick={pickDestinationFolder}>
            Add a destination
          </button>
        )}
      </section>

      <div className="mt-10 flex items-center gap-3 border-t border-rule pt-5">
        <p className="text-mute">
          {config.source && config.destinations.length > 0
            ? "Menlo will show you a plan before touching anything."
            : "Choose a source folder and at least one destination."}
        </p>
        <button
          type="button"
          className="btn btn-primary ml-auto"
          disabled={!config.source || config.destinations.length === 0 || !!busy}
          onClick={scan}
        >
          Scan folder
        </button>
      </div>
    </div>
  );
}

function Heading({ step, title }: { step: number; title: string }) {
  return (
    <h2 className="flex items-baseline gap-2 text-[15px] font-medium">
      <span className="mono text-mute">{step}</span>
      {title}
    </h2>
  );
}
