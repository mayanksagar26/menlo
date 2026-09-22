import { useState } from "react";
import type { FolderSet } from "../lib/types";
import { tilde, useApp } from "../store/app";
import { CUSTOM, fileCount, selectedSources, useUi } from "../store/ui";
import { DriftField, StackedFolders } from "../ui/glyphs";
import { DashedChip, Eyebrow } from "../ui/primitives";

/**
 * Home.
 *
 * One column, and the only decision on it is which folder set to tidy. Picking a set
 * re-points the greeting line and the three figures at that set rather than starting
 * anything — the move is always the button at the bottom, never a click up here.
 */
export function Home() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const [hovered, setHovered] = useState<string | null>(null);

  if (!view) return null;

  const pinned = view.folder_sets.filter((s) => s.pinned);
  const more = view.folder_sets.length - pinned.length;

  const sources = selectedSources(view, ui);
  const count = fileCount(view, ui);
  const first = view.settings.profile_name.trim().split(" ")[0];
  const lastRun = view.runs[0]?.started_at;

  return (
    <div className="relative flex h-full flex-col items-center justify-center overflow-y-auto px-10 py-8">
      <DriftField />

      <div className="anim-rise relative flex w-full max-w-[720px] flex-col items-center">
        <h1
          className="m-0 text-center text-[52px] leading-[1.1] tracking-[-0.04em] text-ink"
          style={{ fontWeight: 400 }}
        >
          {greeting()}
          {first ? `, ${first}` : ""}.
        </h1>
        <p className="mt-3.5 max-w-[460px] text-center text-[12.5px] leading-[1.6] text-ink-66">
          {sources.length === 0
            ? "Pick a folder set below, or start from nothing with Custom."
            : count === 0
              ? "Nothing waiting in these folders. Menlo will be here when there is."
              : `${count} ${count === 1 ? "file is" : "files are"} waiting in ${
                  sources.length === 1 ? sources[0].label : `${sources.length} folders`
                }. Menlo can put them away in about a minute.`}
        </p>

        <div className="glass mt-8 flex rounded-[14px] p-1">
          <Stat value={String(count)} label={count === 1 ? "File" : "Files"} />
          <Divider />
          <Stat
            value={String(sources.length)}
            label={sources.length === 1 ? "Source folder" : "Source folders"}
          />
          <Divider />
          <Stat value={lastRun ? shortDate(lastRun) : "Never"} label="Last run" size={19} />
        </div>

        <div className="mt-10 w-full text-center">
          <Eyebrow>Source folders</Eyebrow>
          <div className="mt-3.5 flex flex-wrap items-start justify-center gap-2.5">
            {pinned.map((set) => (
              <SetPill
                key={set.id}
                set={set}
                selected={ui.preset === set.id}
                open={hovered === set.id}
                onHover={setHovered}
              />
            ))}

            <CustomPill />

            {more > 0 && (
              <DashedChip
                className="!h-11 !rounded-[14px] !px-4 !text-[12.5px]"
                onClick={() => ui.openSets(true)}
              >
                More folder sets ({more} more)
              </DashedChip>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

function greeting(now: Date = new Date()) {
  const h = now.getHours();
  if (h < 12) return "Good morning";
  if (h < 17) return "Good afternoon";
  return "Good evening";
}

/** "18 Sep · 9:00 pm" */
export function shortDate(iso: string): string {
  const d = new Date(iso);
  const date = d.toLocaleDateString(undefined, { day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" }).toLowerCase();
  return `${date} · ${time}`;
}

function Stat({ value, label, size = 26 }: { value: string; label: string; size?: number }) {
  return (
    <div className="w-[160px] px-3 py-3 text-center">
      <p className="m-0 leading-none tracking-[-0.02em] text-ink" style={{ fontSize: size, fontWeight: 400 }}>
        {value}
      </p>
      <p className="eyebrow mt-2">{label}</p>
    </div>
  );
}

function Divider() {
  return <div className="my-3 w-px bg-hair" aria-hidden />;
}

function pillStyle(selected: boolean, lifted: boolean) {
  return {
    "--glass-fill": selected ? "var(--color-glass-sel)" : "var(--color-glass)",
    "--glass-hair": selected ? "rgba(255,255,255,0.4)" : "rgba(255,255,255,0.14)",
    "--glass-rim": selected ? "rgba(255,255,255,0.24)" : "rgba(255,255,255,0.1)",
    transform: lifted ? "translateY(-2px)" : "none",
    transitionTimingFunction: "var(--ease-menlo)",
  } as React.CSSProperties;
}

/**
 * A folder set. Hovering a set with more than one source fans its folder glyphs out
 * and drops the folder names underneath, so the pill answers "which folders?" without
 * having to be opened.
 */
function SetPill({
  set,
  selected,
  open,
  onHover,
}: {
  set: FolderSet;
  selected: boolean;
  open: boolean;
  onHover: (id: string | null) => void;
}) {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const expandable = set.sources.length > 1;
  const showList = open && expandable;

  return (
    <div
      className="relative"
      style={{ zIndex: showList ? 40 : 1 }}
      onMouseEnter={() => onHover(set.id)}
      onMouseLeave={() => onHover(null)}
    >
      <button
        type="button"
        onClick={() => ui.chooseSet(set)}
        aria-pressed={selected}
        className="glass flex h-11 items-center gap-2.5 rounded-[14px] px-3.5 transition-all duration-200"
        style={pillStyle(selected, showList)}
      >
        <span className="text-ink-86">
          <StackedFolders count={set.sources.length} open={open} />
        </span>
        <span className="text-[12.5px]" style={{ color: selected ? "#FFFFFF" : "var(--color-ink-86)" }}>
          {set.label}
        </span>
        {set.ai && (
          <span className="text-[11px] text-ink-60" title="Drafted by Menlo">
            ✦
          </span>
        )}
        <span className="text-[11px] text-ink-60">
          {set.sources.length} {set.sources.length === 1 ? "folder" : "folders"}
        </span>
        <span
          role="button"
          tabIndex={0}
          aria-label={`Edit ${set.label}`}
          onClick={(e) => {
            e.stopPropagation();
            ui.openEditor(set.id);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.stopPropagation();
              ui.openEditor(set.id);
            }
          }}
          className="ml-0.5 text-[11px] text-ink-60 transition-colors duration-200 hover:text-ink"
        >
          ✎
        </span>
      </button>

      {/* Absolutely positioned, so it owes the layout no height and animates only
          transform and opacity — both on the compositor, which is what lets the
          open curve be springy without tearing. */}
      {expandable && (
        <div
          className="glass-pop absolute top-[calc(100%+6px)] left-0 w-full min-w-[170px] origin-top rounded-[12px] px-2.5 py-[7px]"
          style={{
            opacity: showList ? 1 : 0,
            transform: showList ? "translateY(0) scaleY(1)" : "translateY(-6px) scaleY(0.92)",
            transition: showList
              ? "opacity 160ms var(--ease-menlo), transform 340ms var(--ease-spring)"
              : "opacity 140ms var(--ease-menlo), transform 200ms var(--ease-menlo)",
            pointerEvents: showList ? "auto" : "none",
            willChange: "transform, opacity",
          }}
          aria-hidden={!showList}
        >
          {set.sources.map((path) => (
            <p key={path} className="m-0 truncate text-[11px] leading-[26px] text-ink-70">
              {tilde(view, path)}
            </p>
          ))}
        </div>
      )}
    </div>
  );
}

function CustomPill() {
  const ui = useUi();
  return (
    <button
      type="button"
      onClick={() => ui.chooseSet(CUSTOM)}
      className="glass flex h-11 items-center gap-2.5 rounded-[14px] px-3.5 transition-all duration-200"
      style={pillStyle(false, false)}
    >
      <span className="text-ink-86">
        <StackedFolders count={1} open={false} />
      </span>
      <span className="text-[12.5px] text-ink-86">Custom</span>
      <span className="text-[11px] text-ink-60">Start empty</span>
    </button>
  );
}
