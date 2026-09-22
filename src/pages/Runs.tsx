import { useEffect } from "react";
import { shortDate } from "../screens/Home";
import { useApp } from "../store/app";
import { useUi } from "../store/ui";
import { Page, RailRow } from "./Page";

/**
 * Runs — the undo history, kept per run and per folder.
 *
 * Restoring is the promise the rest of the app makes, so this page is blunt about what
 * a restore can and cannot do: a file edited, renamed or moved on since the run stays
 * where it is, and the page says so rather than reporting a clean success it did not
 * achieve.
 */
export function Runs() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const run = useApp((s) => s.run);
  const openRun = useApp((s) => s.openRun);
  const revertRun = useApp((s) => s.revertRun);
  const revertGroup = useApp((s) => s.revertGroup);

  const runs = view?.runs ?? [];
  const active = ui.runActive ?? runs[0]?.batch_id ?? null;

  useEffect(() => {
    if (active && run?.summary.batch_id !== active) void openRun(active);
  }, [active, run, openRun]);

  if (runs.length === 0) {
    return (
      <Page title="Runs" rail={null}>
        <p className="m-0 max-w-[420px] text-[12.5px] leading-[1.7] text-ink-60">
          No runs yet. Every run is kept here, and any of it — a whole run, one folder of it, or the
          duplicates it sent to the Trash — can be sent back.
        </p>
      </Page>
    );
  }

  const shown = run?.summary.batch_id === active ? run : null;
  const s = shown?.summary;
  const folders = shown?.groups.filter((g) => !g.trash).length ?? 0;

  return (
    <Page
      title="Runs"
      rail={runs.map((r) => (
        <RailRow key={r.batch_id} active={r.batch_id === active} onClick={() => ui.set({ runActive: r.batch_id })}>
          <span className="min-w-0 flex-1">
            <span className="block truncate text-[12.5px]">{shortDate(r.started_at)}</span>
            <span className="block truncate text-[11px] text-ink-60">
              {r.set_label ? `${r.set_label} · ` : ""}
              {r.moved} {r.moved === 1 ? "file" : "files"}
              {r.fully_reverted ? " · restored" : ""}
            </span>
          </span>
        </RailRow>
      ))}
    >
      {shown && s && (
        <div className="max-w-[620px]">
          <div className="flex items-start gap-4">
            <div className="min-w-0 flex-1">
              <h3 className="m-0 text-[17px] font-medium tracking-[-0.015em] text-ink">
                {s.set_label ?? "Custom"} · {shortDate(s.started_at)}
              </h3>
              <p className="mt-1.5 mb-0 text-[11.5px] text-ink-60">
                {s.moved} {s.moved === 1 ? "file" : "files"} moved into {folders}{" "}
                {folders === 1 ? "folder" : "folders"}
                {s.kept ? ` · ${s.kept} kept where they were` : ""}
                {s.failed ? ` · ${s.failed} could not be moved` : ""}
              </p>
            </div>
            <button
              type="button"
              className="btn-glass shrink-0"
              disabled={s.fully_reverted}
              onClick={() => revertRun(s.batch_id)}
            >
              {s.fully_reverted ? "Restored" : "Restore this run"}
            </button>
          </div>

          {shown.stayed_put > 0 && (
            <p className="mt-4 mb-0 text-[11.5px] leading-[1.6] text-ink-70">
              {shown.stayed_put} {shown.stayed_put === 1 ? "file was" : "files were"} edited, renamed or moved
              on since this run, so {shown.stayed_put === 1 ? "it" : "they"} stayed put rather than being
              overwritten. Everything else went back to where it came from.
            </p>
          )}

          <ul className="mt-6 flex list-none flex-col gap-1.5 p-0">
            {shown.groups.map((g) => {
              const done = g.restored >= g.count;
              const restorable = g.trash || g.key !== null;
              return (
                <li
                  key={g.label}
                  className="glass flex items-center gap-3 rounded-[11px] px-3.5 py-2.5 transition-opacity duration-200"
                  style={{ opacity: done ? 0.5 : 1 }}
                >
                  <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink-86">
                    {g.trash ? "Sent to the Trash" : g.label}
                  </span>
                  <span className="shrink-0 text-[11px] text-ink-60">
                    {g.count} {g.count === 1 ? "file" : "files"}
                  </span>
                  {restorable ? (
                    <button
                      type="button"
                      disabled={done}
                      onClick={() => revertGroup(s.batch_id, g.trash ? null : g.key)}
                      className="shrink-0 rounded-full border border-hair px-2.5 py-1 text-[11px] text-ink-70 transition-colors duration-200 hover:border-hair-hi hover:text-ink"
                    >
                      {done ? "Restored" : g.trash ? "Bring back" : "Restore"}
                    </button>
                  ) : (
                    <span className="shrink-0 text-[11px] text-ink-60" title="Restore the whole run to bring these back">
                      No longer in Menlo
                    </span>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      )}
    </Page>
  );
}
