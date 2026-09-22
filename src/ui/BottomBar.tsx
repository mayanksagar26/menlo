import { beginRun, finishRun, restoreStopped, stopMove } from "../flow/run";
import { useApp } from "../store/app";
import { fileCount, selectedDests, selectedSources, useUi } from "../store/ui";
import { TidyButton } from "./TidyButton";

const STAGES = ["Choose", "Shape", "Move", "Files processed"];

/**
 * 74px of glass across the bottom: what is in view, where you are, what happens next.
 *
 * Every committing action in the app lives on the right of this bar, so there is one
 * place to look for "and then what". The left note is the running count that makes
 * the action legible before you take it.
 */
export function BottomBar() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const outcome = useApp((s) => s.outcome);
  const progress = useApp((s) => s.progress);
  const { home, step, moving } = ui;

  const inView = fileCount(view, ui);
  const ready = selectedSources(view, ui).length > 0 && selectedDests(view, ui).length > 0;
  const stopped = !moving && !!outcome?.stopped;

  const note = home
    ? ""
    : step === 0
      ? `${inView} ${inView === 1 ? "file" : "files"} in view`
      : step === 1
        ? `${inView} ${inView === 1 ? "file" : "files"} to sort`
        : step === 2
          ? stopped
            ? `Stopped · ${outcome.moved} of ${progress.total} moved`
            : `Moving · ${progress.done} of ${progress.total}`
          : "Reversible any time from Runs";

  // Back is hidden mid-pass: there is nothing to go back to while files are in flight.
  const canBack = !home && step !== 3 && !moving && !(step === 2 && stopped && outcome.moved > 0);

  return (
    <footer className="glass-bar relative z-30 flex h-[74px] shrink-0 items-center gap-4 border-t border-hair px-5">
      <span className="w-[210px] shrink-0 truncate text-[11.5px] text-ink-60">{note}</span>

      <div className="flex flex-1 items-center justify-center">
        {home ? (
          <TidyButton onClick={ui.enterFlow} />
        ) : (
          <div className="flex items-center gap-1.5" role="tablist" aria-label="Stages">
            {STAGES.map((label, i) => {
              // Only Choose and Shape can be jumped to: a progress dot must never be
              // the thing that starts moving files.
              const reachable = i < 2 && step < 2 && (i === 0 || ready);
              const active = i === step;
              return (
                <button
                  key={label}
                  type="button"
                  role="tab"
                  aria-selected={active}
                  aria-label={label}
                  disabled={!reachable}
                  onClick={() => ui.goStep(i as 0 | 1)}
                  className="h-[2px] rounded-full transition-all duration-[340ms]"
                  style={{
                    width: active ? 26 : 12,
                    background: active
                      ? "var(--color-ink)"
                      : reachable
                        ? "rgba(255,255,255,0.28)"
                        : "rgba(255,255,255,0.14)",
                    transitionTimingFunction: "var(--ease-menlo)",
                  }}
                />
              );
            })}
          </div>
        )}
      </div>

      <div className="flex w-[210px] shrink-0 items-center justify-end gap-2">
        {canBack && (
          <button type="button" className="btn-glass" onClick={ui.back}>
            Back
          </button>
        )}

        {!home && step === 0 && (
          <button type="button" className="btn-solid" disabled={!ready} onClick={() => ui.goStep(1)}>
            Continue
          </button>
        )}

        {!home && step === 1 && (
          <button
            type="button"
            className="btn-solid"
            disabled={!ready || inView === 0 || ui.planning}
            onClick={beginRun}
          >
            Move
          </button>
        )}

        {!home && step === 2 && moving && (
          <button type="button" className="btn-glass" onClick={stopMove}>
            Stop
          </button>
        )}

        {!home && step === 2 && stopped && outcome.moved > 0 && (
          <button type="button" className="btn-solid" onClick={restoreStopped}>
            ↩ Restore {outcome.moved} {outcome.moved === 1 ? "file" : "files"}
          </button>
        )}

        {!home && step === 3 && (
          <button type="button" className="btn-solid" onClick={finishRun}>
            Done
          </button>
        )}
      </div>
    </footer>
  );
}
