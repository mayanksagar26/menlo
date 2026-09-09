import { useEffect } from "react";
import { Chrome } from "../components/Chrome";
import { plural, relativeTime, tildePath } from "../lib/format";
import { useStore } from "../store";

/**
 * Every batch, newest first, each with one-click revert (task 7).
 *
 * Revert verifies each file's SHA-256 before restoring it, so a file edited since it
 * was filed is reported and left alone rather than clobbered.
 */
export function History() {
  const batches = useStore((s) => s.batches);
  const busy = useStore((s) => s.busy);
  const refresh = useStore((s) => s.refreshBatches);
  const revert = useStore((s) => s.revert);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="flex h-full flex-col">
      <Chrome />
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-4">
        {batches.length === 0 ? (
          <p className="mt-16 text-center text-mute">
            Nothing filed yet. Every batch you run shows up here, and every one can be undone.
          </p>
        ) : (
          <ul className="mx-auto flex w-full max-w-2xl flex-col gap-2">
            {batches.map((b) => (
              <li key={b.batch_id} className="card flex items-center gap-4 px-3 py-2.5">
                <div className="min-w-0 flex-1">
                  <div className="flex items-baseline gap-2">
                    <span className="font-medium">{plural(b.moved, "file")}</span>
                    <span className="text-mute">{relativeTime(b.started_at)}</span>
                    {b.failed > 0 && (
                      <span className="text-warn">{b.failed} failed</span>
                    )}
                  </div>
                  <div className="mono mt-0.5 truncate text-mute">
                    from {tildePath(b.source)}
                  </div>
                </div>

                {b.fully_reverted ? (
                  <span className="text-ok">Reverted</span>
                ) : (
                  <button
                    type="button"
                    className="btn"
                    disabled={!!busy || b.moved === 0}
                    onClick={() => revert(b.batch_id)}
                  >
                    {b.reverted > 0 ? `Revert remaining ${b.moved - b.reverted}` : "Revert"}
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
