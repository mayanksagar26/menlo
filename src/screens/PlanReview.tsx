import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { Chrome } from "../components/Chrome";
import { FileRow } from "../components/FileRow";
import { FolderTile } from "../components/FolderTile";
import { plural } from "../lib/format";
import type { PlanEntry } from "../lib/types";
import { planStats, useStore } from "../store";

type Filter = "all" | "moving" | "review" | "excluded";

function usePrefersReducedMotion() {
  const [reduced, setReduced] = useState(
    () => window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false,
  );
  useEffect(() => {
    const mq = window.matchMedia("(prefers-reduced-motion: reduce)");
    const on = () => setReduced(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);
  return reduced;
}

/**
 * The hero screen (§1.3). Nothing moves until the button in the top right is pressed,
 * and everything the app intends to do is legible before that happens.
 */
export function PlanReview() {
  const plan = useStore((s) => s.plan);
  const config = useStore((s) => s.config);
  const busy = useStore((s) => s.busy);
  const outcome = useStore((s) => s.outcome);
  const scan = useStore((s) => s.scan);
  const apply = useStore((s) => s.apply);
  const updateEntry = useStore((s) => s.updateEntry);
  const setAllIncluded = useStore((s) => s.setAllIncluded);

  const [filter, setFilter] = useState<Filter>("all");
  const [flying, setFlying] = useState(false);
  const reducedMotion = usePrefersReducedMotion();

  const listRef = useRef<HTMLUListElement>(null);
  const tileRefs = useRef<Record<string, HTMLButtonElement | null>>({});

  const stats = planStats(plan);

  const perDestination = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const d of config.destinations) counts[d.key] = 0;
    for (const e of plan?.entries ?? []) {
      if (e.included && e.action === "move" && e.destination_key) {
        counts[e.destination_key] = (counts[e.destination_key] ?? 0) + 1;
      }
    }
    return counts;
  }, [plan, config.destinations]);

  const visible = useMemo(() => {
    const entries = plan?.entries ?? [];
    switch (filter) {
      case "moving":
        return entries.filter((e) => e.included && e.action === "move");
      case "review":
        return entries.filter((e) => e.action === "needs_review");
      case "excluded":
        return entries.filter((e) => !e.included);
      default:
        return entries;
    }
  }, [plan, filter]);

  /**
   * §5: the file card travels from the source column to its destination tile. The
   * offset is measured at apply time rather than stored, so a resized window or a
   * scrolled list still lands in the right place.
   */
  const flightOffset = useCallback(
    (entry: PlanEntry, rowIndex: number): { x: number; y: number } | null => {
      if (!entry.destination_key) return null;
      const tile = tileRefs.current[entry.destination_key];
      const list = listRef.current;
      if (!tile || !list) return null;
      const rows = list.children;
      const row = rows[rowIndex] as HTMLElement | undefined;
      if (!row) return null;
      const from = row.getBoundingClientRect();
      const to = tile.getBoundingClientRect();
      return {
        x: to.left + to.width / 2 - (from.left + from.width / 2),
        y: to.top + to.height / 2 - (from.top + from.height / 2),
      };
    },
    [],
  );

  const onApply = useCallback(async () => {
    if (stats.willMove === 0) return;
    setFlying(true);
    // The animation exists so the user can audit the plan, not to gate it: the IPC
    // starts immediately and the two settle together.
    const minimumWatchTime = reducedMotion ? 150 : 320 + Math.min(stats.willMove, 12) * 40;
    await Promise.all([apply(), new Promise((r) => setTimeout(r, minimumWatchTime))]);
    setFlying(false);
  }, [apply, stats.willMove, reducedMotion]);

  // Interruptible (§5): Escape cuts the flight short.
  useEffect(() => {
    if (!flying) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setFlying(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [flying]);

  if (outcome) return <Outcome />;

  if (!plan) {
    return (
      <div className="flex h-full flex-col">
        <Chrome
          actions={
            <button type="button" className="btn btn-primary" onClick={scan} disabled={!!busy}>
              {busy === "Scanning" ? "Scanning…" : "Scan folder"}
            </button>
          }
        />
        <div className="flex flex-1 items-center justify-center">
          <div className="max-w-sm text-center">
            <p className="text-ink-soft">
              Menlo has not looked at your folder yet. Nothing moves without your say-so.
            </p>
            <button type="button" className="btn btn-primary mt-4" onClick={scan} disabled={!!busy}>
              Scan folder
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <Chrome
        actions={
          <>
            <button type="button" className="btn" onClick={scan} disabled={!!busy || flying}>
              Rescan
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={onApply}
              disabled={stats.willMove === 0 || !!busy || flying}
            >
              {busy === "Filing" || flying ? "Filing…" : `File ${plural(stats.willMove, "file")}`}
            </button>
          </>
        }
      />

      <div className="flex min-h-0 flex-1">
        <main className="flex min-w-0 flex-1 flex-col">
          <div className="flex flex-wrap items-center gap-x-5 gap-y-1 border-b border-rule px-4 py-2.5">
            <Stat value={stats.total} label="in the folder" />
            {/* Task 12: the compounding value of learned rules, made visible. */}
            <Stat value={stats.withoutAi} label="resolved without AI" accent />
            <Stat value={stats.needsReview} label="need a decision" />
            {plan.skipped.length > 0 && (
              <span className="text-mute" title={plan.skipped.map((s) => s.name).join("\n")}>
                {plural(plan.skipped.length, "item")} ignored
              </span>
            )}
            {plan.truncated && (
              <span className="text-warn">Capped at 500 files — run again for the rest.</span>
            )}
          </div>

          <div className="flex items-center gap-1 border-b border-rule px-3 py-1.5">
            {(
              [
                ["all", `All ${stats.total}`],
                ["moving", `Filing ${stats.willMove}`],
                ["review", `Review ${stats.needsReview}`],
                ["excluded", `Excluded ${stats.excluded}`],
              ] as [Filter, string][]
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setFilter(id)}
                aria-pressed={filter === id}
                className={`h-6 rounded-[6px] px-2 text-xs transition-colors ${
                  filter === id ? "bg-manila-deep text-ink" : "text-mute hover:text-ink"
                }`}
              >
                {label}
              </button>
            ))}
            <div className="ml-auto flex items-center gap-2 text-xs">
              <button
                type="button"
                className="text-mute hover:text-ink"
                onClick={() => setAllIncluded(true, visible.map((e) => e.id))}
              >
                Include all
              </button>
              <span className="text-rule">|</span>
              <button
                type="button"
                className="text-mute hover:text-ink"
                onClick={() => setAllIncluded(false, visible.map((e) => e.id))}
              >
                Exclude all
              </button>
            </div>
          </div>

          {visible.length === 0 ? (
            <p className="p-8 text-center text-mute">Nothing in this view.</p>
          ) : (
            <ul ref={listRef} className="min-h-0 flex-1 overflow-y-auto">
              {visible.map((entry, i) => (
                <FileRow
                  key={entry.id}
                  entry={entry}
                  index={i}
                  destinations={config.destinations}
                  flying={flying}
                  flightTo={flying ? flightOffset(entry, i) : null}
                  reducedMotion={reducedMotion}
                  onChange={(patch) => updateEntry(entry.id, patch)}
                />
              ))}
            </ul>
          )}
        </main>

        <aside className="w-56 shrink-0 overflow-y-auto border-l border-rule px-3 pb-4 pt-3">
          <h2 className="mb-3 px-1 text-xs uppercase tracking-wide text-mute">Destinations</h2>
          <div className="flex flex-col gap-3">
            {config.destinations.map((d) => (
              <FolderTile
                key={d.key}
                ref={(el) => {
                  tileRefs.current[d.key] = el;
                }}
                label={d.label}
                destKey={d.key}
                count={perDestination[d.key] ?? 0}
                active={(perDestination[d.key] ?? 0) > 0}
              />
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
}

function Stat({ value, label, accent }: { value: number; label: string; accent?: boolean }) {
  return (
    <span className="flex items-baseline gap-1.5">
      <span className={`mono text-[15px] ${accent ? "text-oxblood" : "text-ink"}`}>{value}</span>
      <span className="text-mute">{label}</span>
    </span>
  );
}

/** The receipt. Shown once, then the user goes back to a clean folder. */
function Outcome() {
  const outcome = useStore((s) => s.outcome)!;
  const revert = useStore((s) => s.revert);
  const clear = useStore((s) => s.clearOutcome);
  const scan = useStore((s) => s.scan);
  const busy = useStore((s) => s.busy);

  return (
    <div className="flex h-full flex-col">
      <Chrome />
      <div className="flex flex-1 items-center justify-center px-6">
        <AnimatePresence>
          <motion.div
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.32, ease: [0.25, 1, 0.5, 1] }}
            className="w-full max-w-md"
          >
            <p className="text-[22px] tracking-tight">
              Filed <span className="mono text-oxblood">{outcome.moved}</span>{" "}
              {outcome.moved === 1 ? "file" : "files"}.
            </p>

            {outcome.learned.length > 0 && (
              <p className="mt-2 text-ink-soft">
                Learned {plural(outcome.learned.length, "new rule")}. Next time these are filed
                without asking anyone.
              </p>
            )}

            {outcome.failures.length > 0 && (
              <details className="card mt-4 px-3 py-2">
                <summary className="cursor-pointer text-warn">
                  {plural(outcome.failures.length, "file")} could not be moved
                </summary>
                <ul className="mt-2 flex flex-col gap-1.5">
                  {outcome.failures.map((f) => (
                    <li key={f.entry_id} className="selectable text-ink-soft">
                      <span className="mono">{f.from.split("/").pop()}</span> — {f.error}
                    </li>
                  ))}
                </ul>
              </details>
            )}

            <div className="mt-6 flex items-center gap-2">
              <button
                type="button"
                className="btn"
                onClick={() => revert(outcome.batch_id)}
                disabled={!!busy}
              >
                Undo this batch
              </button>
              <button type="button" className="btn" onClick={() => { clear(); scan(); }} disabled={!!busy}>
                Scan again
              </button>
              <button type="button" className="btn btn-primary ml-auto" onClick={clear}>
                Done
              </button>
            </div>
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
}
