import { useEffect } from "react";
import { useApp } from "../store/app";
import { Eyebrow } from "../ui/primitives";

/**
 * Stage 4 — FILES PROCESSED.
 *
 * A receipt, not a celebration. It is read back from the journal of the run that just
 * happened, so it lists what actually landed rather than what was planned; it says
 * plainly what was left alone and what could not be moved; and it points at the one
 * thing worth knowing afterwards — this is reversible.
 */
export function Receipt() {
  const outcome = useApp((s) => s.outcome);
  const run = useApp((s) => s.run);
  const plan = useApp((s) => s.plan);
  const openRun = useApp((s) => s.openRun);

  useEffect(() => {
    if (outcome) void openRun(outcome.batch_id);
  }, [outcome, openRun]);

  if (!outcome) return null;
  const groups = run?.summary.batch_id === outcome.batch_id ? run.groups : [];

  const extras = [
    outcome.kept && `${outcome.kept} kept where ${outcome.kept === 1 ? "it was" : "they were"}`,
    outcome.trashed && `${outcome.trashed} ${outcome.trashed === 1 ? "duplicate" : "duplicates"} in the Trash`,
    outcome.failures.length && `${outcome.failures.length} could not be moved`,
  ].filter(Boolean);

  const nameOf = (id: string) => plan?.entries.find((e) => e.id === id)?.name ?? id;

  return (
    <div className="flex h-full flex-col px-10 pt-8 pb-4">
      <Eyebrow>Files processed</Eyebrow>
      <h2 className="mt-3 mb-0 leading-[1.1] tracking-[-0.035em] text-ink" style={{ fontSize: 44, fontWeight: 400 }}>
        {outcome.moved} {outcome.moved === 1 ? "file" : "files"} filed
      </h2>
      <p className="mt-2.5 mb-0 text-[12.5px] text-ink-66">
        {[...extras, "reversible any time from Runs"].join(" · ")}
      </p>

      <ul className="mt-7 flex min-h-0 flex-1 list-none flex-col gap-1.5 overflow-y-auto p-0 pr-1">
        {groups.map((g) => (
          <li key={g.label} className="glass flex items-center gap-3 rounded-[11px] px-3.5 py-2.5">
            <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink-86">
              {g.trash ? "Sent to the Trash" : g.label}
            </span>
            <span className="shrink-0 text-[11px] text-ink-60">
              {g.count} {g.count === 1 ? "file" : "files"}
            </span>
          </li>
        ))}

        {outcome.failures.map((f) => (
          <li key={f.entry_id} className="flex items-start gap-3 rounded-[11px] px-3.5 py-2">
            <span className="min-w-0 flex-1 truncate text-[12px] text-ink-70">{nameOf(f.entry_id)}</span>
            <span className="max-w-[60%] shrink-0 text-right text-[11px] text-ink-60">{f.error}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
