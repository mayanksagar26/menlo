import { useState } from "react";
import { BUILTIN_KINDS, ideasFor } from "../lib/copy";
import { destination, rulesFor, subfoldersOf, tilde, topLevel, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { DashedChip, RuleTag } from "../ui/primitives";
import { Page, RailRow } from "./Page";

/**
 * Rules memory — every sentence Menlo has been told, by the folder it was told about.
 *
 * Rules belong to a folder rather than to a set or a run, so the same sentence governs
 * a folder whichever set reaches it. Each rule shows what Menlo understood from it, and
 * whether it can act on it without a model, so no rule is trusted blind.
 */
export function RulesMemory() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const addRule = useApp((s) => s.addRule);
  const removeRule = useApp((s) => s.removeRule);
  const [draft, setDraft] = useState("");

  const tops = topLevel(view);
  const key = ui.rulesActive ?? tops[0]?.key ?? null;
  const dest = destination(view, key);

  async function add(text: string) {
    if (!key || !text.trim()) return;
    await addRule(key, text);
    setDraft("");
  }

  return (
    <Page
      title="Rules memory"
      rail={tops.map((d) => (
        <div key={d.key}>
          <RailRow active={key === d.key} onClick={() => ui.set({ rulesActive: d.key })}>
            <span className="flex-1 truncate text-[12.5px]">{d.label}</span>
            <Count n={rulesFor(view, d.key).length} />
          </RailRow>
          {subfoldersOf(view, d.key).map((sub) => (
            <RailRow key={sub.key} indent={18} active={key === sub.key} onClick={() => ui.set({ rulesActive: sub.key })}>
              <span className="flex-1 truncate text-[11.5px]">{sub.label}</span>
              <Count n={rulesFor(view, sub.key).length} />
            </RailRow>
          ))}
        </div>
      ))}
    >
      {dest && key ? (
        <div className="max-w-[620px]">
          <h3 className="m-0 text-[17px] font-medium tracking-[-0.015em] text-ink">{tilde(view, dest.path)}</h3>
          <p className="mt-1.5 mb-0 text-[11.5px] text-ink-60">
            {rulesFor(view, key).length} {rulesFor(view, key).length === 1 ? "rule" : "rules"} saved for this folder
          </p>

          <ul className="mt-5 flex list-none flex-col gap-2.5 p-0">
            {rulesFor(view, key).length === 0 && (
              <li className="text-[11.5px] leading-[1.6] text-ink-60">
                {dest.parent
                  ? `No rules yet, so this folder takes files named like “${dest.label}”. Anything you write here replaces that.`
                  : BUILTIN_KINDS[dest.key]
                    ? `No rules yet. ${BUILTIN_KINDS[dest.key]} still land here by type until you say otherwise.`
                    : "No rules yet, so nothing lands here. Write one, or tap an idea below."}
              </li>
            )}
            {rulesFor(view, key).map((r) => (
              <li key={r.id} className="flex flex-col gap-1">
                <span>
                  <RuleTag
                    text={r.text}
                    understood={r.understood}
                    reading={r.reading}
                    onRemove={() => removeRule(r.id)}
                  />
                </span>
                <span className="pl-1 text-[11px] text-ink-60">{r.reading}</span>
              </li>
            ))}
          </ul>

          <input
            className="field mt-6"
            value={draft}
            placeholder="Add a rule — plain words, as long as you like"
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void add(draft);
            }}
          />

          <div className="mt-4 flex flex-wrap gap-1.5">
            {ideasFor(dest.label, !!dest.parent).map((idea) => (
              <DashedChip key={idea} onClick={() => add(idea)}>
                {idea}
              </DashedChip>
            ))}
          </div>
        </div>
      ) : (
        <p className="m-0 text-[12.5px] text-ink-60">Add a destination folder first, then give it rules here.</p>
      )}
    </Page>
  );
}

function Count({ n }: { n: number }) {
  return (
    <span className="shrink-0 text-[10.5px] text-ink-60" aria-label={`${n} rules`}>
      {n || ""}
    </span>
  );
}
