import { useLayoutEffect, useRef, useState } from "react";
import { actionable, pathLabel, useApp } from "../store/app";
import { selectedSources, useUi } from "../store/ui";
import { bucketOf, FileGlyph } from "../ui/glyphs";
import { Eyebrow } from "../ui/primitives";

/** Vertical distance between lanes on either side of the track. */
const LANE = 58;
/** How long one file's flight lasts — the `menlo-fly` keyframe. */
const FLIGHT_MS = 1050;
/** The lane key for duplicates sent to the Trash. */
const TRASH = "__trash__";

/**
 * Stage 3 — MOVE.
 *
 * The pass, made watchable. Sources on the left, destinations on the right with their
 * own counters, and one card per file crossing between them as Rust reports it. The
 * counters are the honest part — each tick is a file that has actually landed and
 * been journalled; the flight is what makes it feel like something happened to a
 * thing rather than a progress bar filling.
 */
export function Move() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const plan = useApp((s) => s.plan);
  const progress = useApp((s) => s.progress);
  const outcome = useApp((s) => s.outcome);

  const track = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  useLayoutEffect(() => {
    const el = track.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const sources = selectedSources(view, ui);
  const entries = actionable(plan);
  const byId = new Map(entries.map((e) => [e.id, e]));

  // One lane per destination this plan actually uses, in plan order.
  const lanes: string[] = [];
  for (const e of entries) {
    if (e.destination_key && !lanes.includes(e.destination_key)) lanes.push(e.destination_key);
  }
  const trashed = progress.recent.some((p) => p.outcome === "trashed") || (outcome?.trashed ?? 0) > 0;
  if (trashed) lanes.push(TRASH);

  const planned = (key: string) => entries.filter((e) => e.destination_key === key).length;
  const total = progress.total || entries.length;
  const done = progress.done;
  const stopped = !ui.moving && !!outcome?.stopped;

  // Cards still in the air: the most recent ticks that filed or trashed something.
  const flying = ui.moving
    ? progress.recent.filter((p) => p.outcome === "moved" || p.outcome === "trashed").slice(-8)
    : [];

  return (
    <div className="flex h-full flex-col px-10 pt-8 pb-4">
      <Eyebrow>Move</Eyebrow>
      <p className="mt-2.5 mb-0 text-[12.5px] text-ink-66">
        {stopped
          ? "Stopped. What landed is filed and can be sent back; everything else is where it was."
          : "Files are travelling to their folders in one pass."}
      </p>

      <div className="mt-5 flex items-end gap-4">
        <p className="m-0 leading-none tracking-[-0.04em] text-ink" style={{ fontSize: 56, fontWeight: 400 }}>
          {done}
          <span className="text-ink-60"> / {total}</span>
        </p>
        <div className="mb-2 h-px w-[220px] overflow-hidden bg-hair">
          <div
            className="h-full bg-ink transition-[width] duration-200"
            style={{
              width: total ? `${(done / total) * 100}%` : 0,
              transitionTimingFunction: "var(--ease-menlo)",
            }}
          />
        </div>
      </div>

      <div ref={track} className="relative mt-8 min-h-0 flex-1 overflow-hidden">
        <div className="absolute top-0 left-0">
          {sources.map((s, i) => (
            <p key={s.path} className="absolute m-0 truncate text-[12px] text-ink-70" style={{ top: i * LANE }}>
              {s.label}
            </p>
          ))}
        </div>

        <div className="absolute top-0 right-0 text-right">
          {lanes.map((key, i) => (
            <div key={key} className="absolute right-0 whitespace-nowrap" style={{ top: i * LANE }}>
              <p className="m-0 text-[12px] text-ink-86">{key === TRASH ? "Trash" : pathLabel(view, key)}</p>
              <p className="m-0 text-[11px] text-ink-60">
                {key === TRASH
                  ? `${outcome?.trashed ?? progress.recent.filter((p) => p.outcome === "trashed").length} duplicates`
                  : `${progress.byKey[key] ?? 0} of ${planned(key)}`}
              </p>
            </div>
          ))}
        </div>

        {width > 0 &&
          flying.map((p) => {
            const entry = byId.get(p.entry_id);
            if (!entry) return null;
            // Entries carry no path, so a card's source lane is derived from its id:
            // stable, and spread evenly across the lanes on the left.
            const from = Number(p.entry_id.replace(/\D/g, "")) % Math.max(1, sources.length);
            const to = lanes.indexOf(p.outcome === "trashed" ? TRASH : (p.destination_key ?? ""));
            return (
              <div
                key={p.entry_id}
                className="glass absolute flex items-center gap-2 rounded-[11px] px-2.5 py-1.5"
                style={{
                  top: from * LANE,
                  left: 0,
                  ["--dx" as string]: `${Math.max(0, width - 210)}px`,
                  ["--dy" as string]: `${(Math.max(0, to) - from) * LANE}px`,
                  animation: `menlo-fly ${FLIGHT_MS}ms var(--ease-menlo) both`,
                }}
              >
                <span className="text-ink-60">
                  <FileGlyph bucket={bucketOf(entry.ext)} size={11} />
                </span>
                <span className="max-w-[150px] truncate text-[11px] text-ink-86">
                  {entry.rename_to ?? entry.name}
                </span>
              </div>
            );
          })}
      </div>
    </div>
  );
}
