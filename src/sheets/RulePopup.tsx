import { useState } from "react";
import { ideasFor } from "../lib/copy";
import { destination, pathLabel, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { DashedChip, Modal, SheetFooter } from "../ui/primitives";

/**
 * Adding a rule.
 *
 * Rules are ordinary sentences of any length — that is the whole premise — so the
 * input is a textarea rather than a field, and the suggestions underneath are full
 * sentences too. Every suggestion is one Menlo can apply without a model.
 */
export function RulePopup({ destKey }: { destKey: string }) {
  const view = useApp((s) => s.view);
  const addRule = useApp((s) => s.addRule);
  const ui = useUi();
  const [draft, setDraft] = useState("");
  const close = () => ui.openRuleFor(null);

  const dest = destination(view, destKey);
  const ideas = ideasFor(dest?.label ?? "", !!dest?.parent);

  async function commit(text: string) {
    if (!text.trim()) return;
    await addRule(destKey, text);
    close();
  }

  return (
    <Modal onClose={close} width={460} labelledBy="rule-title">
      <div className="flex flex-col gap-4 p-6">
        <div>
          <h2 id="rule-title" className="m-0 text-[17px] leading-[1.3] font-medium tracking-[-0.015em] text-ink">
            Add a rule
          </h2>
          <p className="mt-1.5 mb-0 truncate text-[11px] text-ink-60">For {pathLabel(view, destKey)}</p>
        </div>

        <textarea
          autoFocus
          rows={3}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            // Enter commits; Shift+Enter is a newline, since rules can be long.
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void commit(draft);
            }
          }}
          placeholder="Add a rule — plain words, as long as you like"
          className="field h-auto resize-none py-2.5 leading-[1.5]"
        />

        <div className="flex flex-wrap gap-1.5">
          {ideas.map((idea) => (
            <DashedChip key={idea} onClick={() => commit(idea)}>
              {idea}
            </DashedChip>
          ))}
        </div>
      </div>

      <SheetFooter>
        <button type="button" className="btn-glass" onClick={close}>
          Cancel
        </button>
        <button type="button" className="btn-solid" disabled={!draft.trim()} onClick={() => commit(draft)}>
          Add rule
        </button>
      </SheetFooter>
    </Modal>
  );
}
