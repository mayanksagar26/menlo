import { afterDuplicates, cancelRun } from "../flow/run";
import type { DuplicateChoice, PlanEntry } from "../lib/types";
import { actionable, identicalDuplicates, pathLabel, useApp } from "../store/app";
import { bucketOf, FileGlyph } from "../ui/glyphs";
import { Eyebrow, Modal, Segmented, SheetFooter } from "../ui/primitives";

const CHOICES: { value: DuplicateChoice; label: string }[] = [
  { value: "keep", label: "Keep" },
  { value: "trash", label: "Move to Trash" },
];

/**
 * Files that are already where they are going, byte for byte.
 *
 * Asked the moment Move is pressed, before the first file goes. Each copy gets its own
 * answer, with Keep as the default because it is the one choice that can never lose
 * anything: the file stays at its source and is not transferred. Move to Trash sends
 * the extra copy to the Trash, where Runs can bring it back from.
 *
 * Only byte-identical copies are listed here. A file that merely shares a *name* with
 * one already filed is a different file; it is moved alongside as `name (2)` and is
 * never offered for deletion.
 */
export function DuplicatesSheet() {
  const plan = useApp((s) => s.plan);
  const view = useApp((s) => s.view);
  const choose = useApp((s) => s.choose);
  const chooseAll = useApp((s) => s.chooseAll);

  const identical = identicalDuplicates(plan);
  const sameName = actionable(plan).filter((e) => e.duplicate?.kind === "same_name");
  const names = new Map(plan?.entries.map((e) => [e.id, e.name]));

  const allChoice = identical.every((e) => (e.on_duplicate ?? "keep") === "trash")
    ? "trash"
    : identical.every((e) => (e.on_duplicate ?? "keep") === "keep")
      ? "keep"
      : null;

  function where(e: PlanEntry): string {
    const d = e.duplicate!;
    if (d.same_as_entry) {
      return `Same file as ${names.get(d.same_as_entry) ?? d.existing_name}, which is being filed`;
    }
    return `Already in ${pathLabel(view, e.destination_key)} as ${d.existing_name}`;
  }

  return (
    <Modal onClose={cancelRun} width={560} labelledBy="dupes-title">
      <div className="flex items-start gap-4 p-6 pb-3">
        <div className="min-w-0 flex-1">
          <Eyebrow>Duplicates</Eyebrow>
          <h2
            id="dupes-title"
            className="mt-2.5 mb-0 text-[21px] leading-[1.25] font-medium tracking-[-0.02em] text-ink"
          >
            {identical.length} {identical.length === 1 ? "file is" : "files are"} already filed
          </h2>
          <p className="mt-2 mb-0 text-[11.5px] leading-[1.6] text-ink-66">
            Byte for byte the same as a copy already there. <b className="font-medium text-ink-86">Keep</b> leaves
            it where it is and does not move it. <b className="font-medium text-ink-86">Move to Trash</b> removes
            this extra copy — Runs can bring it back.
          </p>
        </div>
        <div className="shrink-0 pt-6">
          <Segmented<DuplicateChoice | "mixed">
            value={allChoice ?? "mixed"}
            options={[
              { value: "keep", label: "Keep all" },
              { value: "trash", label: "Trash all" },
            ]}
            onChange={(v) => v !== "mixed" && chooseAll(v)}
          />
        </div>
      </div>

      <div className="max-h-[320px] overflow-y-auto px-4 pb-2">
        {identical.map((e) => (
          <div key={e.id} className="flex items-center gap-3 rounded-[11px] px-3 py-2.5 hover:bg-glass-hi">
            <span className="text-ink-60">
              <FileGlyph bucket={bucketOf(e.ext)} size={13} />
            </span>
            <span className="min-w-0 flex-1">
              <span className="selectable block truncate text-[12.5px] text-ink-86">{e.name}</span>
              <span className="block truncate text-[11px] text-ink-60">{where(e)}</span>
            </span>
            <Segmented<DuplicateChoice>
              value={e.on_duplicate ?? "keep"}
              options={CHOICES}
              onChange={(v) => choose(e.id, v)}
            />
          </div>
        ))}

        {sameName.length > 0 && (
          <p className="mx-3 mt-3 mb-1 text-[11px] leading-[1.6] text-ink-60">
            {sameName.length} {sameName.length === 1 ? "other file shares a name" : "other files share a name"} with
            something already there but {sameName.length === 1 ? "isn't" : "aren't"} the same file.{" "}
            {sameName.length === 1 ? "It will" : "They will"} be saved alongside — as “
            {secondCopy(sameName[0].rename_to ?? sameName[0].name)}”, say — and never deleted.
          </p>
        )}
      </div>

      <SheetFooter>
        <button type="button" className="btn-glass" onClick={cancelRun}>
          Cancel
        </button>
        <button type="button" className="btn-solid" onClick={afterDuplicates}>
          Continue
        </button>
      </SheetFooter>
    </Modal>
  );
}

/** `report.pdf` → `report (2).pdf`, the name `execute::unique_path` gives a clash. */
function secondCopy(name: string) {
  const i = name.lastIndexOf(".");
  return i > 0 ? `${name.slice(0, i)} (2)${name.slice(i)}` : `${name} (2)`;
}
