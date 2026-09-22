import { pickFolders } from "../lib/ipc";
import type { Destination } from "../lib/types";
import { destination, rulesFor, subfoldersOf, tilde, useApp } from "../store/app";
import { fileCount, selectedDests, useUi } from "../store/ui";
import { openSuggest } from "./run";
import { Chevron, FolderGlyph, PlusIcon } from "../ui/glyphs";
import { DashedChip, Eyebrow, RuleTag } from "../ui/primitives";

/**
 * Stage 2 — SHAPE.
 *
 * Folders and the sentences attached to them, and nothing about individual files.
 * Withholding where each file lands is the point: the user shapes where things
 * *should* go, and the move preview is where they find out where things *will* go.
 * Showing both at once turns shaping into reviewing.
 */
export function Shape() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const addDestinations = useApp((s) => s.addDestinations);
  if (!view) return null;

  const dests = selectedDests(view, ui);
  const folder = destination(view, ui.shapeFolder);
  const count = fileCount(view, ui);

  async function addSubfolder(parent: Destination) {
    const paths = await pickFolders(`Choose folders inside ${parent.label}`, true, parent.path);
    // Only folders actually inside this one; the backend refuses anything else, but
    // there is no reason to ask it to.
    const inside = paths.filter((p) => p.startsWith(parent.path + "/"));
    if (inside.length) await addDestinations(inside, parent.key);
  }

  return (
    <div className="flex h-full flex-col px-10 pt-8 pb-4">
      <div className="flex items-start gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            {folder && (
              <button
                type="button"
                onClick={ui.exitFolder}
                aria-label="Back to all folders"
                className="text-[12px] text-ink-60 transition-colors duration-200 hover:text-ink"
              >
                ←
              </button>
            )}
            <Eyebrow>Shape</Eyebrow>
          </div>
          <h2 className="mt-2.5 mb-0 truncate text-[28px] leading-[1.15] font-medium tracking-[-0.022em] text-ink">
            {folder
              ? tilde(view, folder.path)
              : `Pick the folders your ${count} ${count === 1 ? "file lands" : "files land"} in.`}
          </h2>
        </div>

        <button type="button" className="btn-glass mt-4 shrink-0" onClick={openSuggest}>
          ✦ {folder ? "Suggest subfolders" : "Suggest folders"}
        </button>
      </div>

      {folder && (
        <div className="mt-3.5 flex flex-wrap items-start gap-1.5">
          <Rules destKey={folder.key} />
        </div>
      )}

      <div className="mt-6 grid min-h-0 flex-1 grid-cols-2 content-start gap-3 overflow-y-auto pr-1">
        {folder ? (
          <>
            {subfoldersOf(view, folder.key).map((sub) => (
              <SubCard key={sub.key} sub={sub} />
            ))}
            <DashedChip
              className="!min-h-[88px] !justify-center !rounded-[14px] !text-[12px]"
              onClick={() => addSubfolder(folder)}
            >
              <PlusIcon size={12} /> Add subfolder
            </DashedChip>
          </>
        ) : (
          dests.map((d) => <DestCard key={d.key} dest={d} />)
        )}

        {!folder && dests.length === 0 && (
          <p className="col-span-2 mt-8 text-center text-[11.5px] text-ink-60">
            No destinations picked. Step back to Choose, or let Menlo suggest some.
          </p>
        )}
      </div>

      <p className="mt-4 mb-0 text-[11px] leading-[1.6] text-ink-60">
        {folder
          ? "A filled dot means Menlo applies that rule on its own; a ring means it needs a model. A subfolder with no rule takes files named like it."
          : "Menlo decides which file goes where when it moves. You will see every landing before anything happens."}
      </p>
    </div>
  );
}

/** A folder's rules, then the chip to add one. */
function Rules({ destKey }: { destKey: string }) {
  const view = useApp((s) => s.view);
  const removeRule = useApp((s) => s.removeRule);
  const ui = useUi();
  return (
    <>
      {rulesFor(view, destKey).map((r) => (
        <RuleTag
          key={r.id}
          text={r.text}
          understood={r.understood}
          reading={r.reading}
          onRemove={() => removeRule(r.id)}
        />
      ))}
      <DashedChip onClick={() => ui.openRuleFor(destKey)}>+ Add rule</DashedChip>
    </>
  );
}

/** A destination at the top level: its name, how many subfolders, and its rules. */
function DestCard({ dest }: { dest: Destination }) {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const subs = subfoldersOf(view, dest.key);

  return (
    <article className="glass flex flex-col gap-2.5 rounded-[14px] p-3.5">
      <button
        type="button"
        onClick={() => ui.enterFolder(dest.key)}
        className="group flex items-center gap-2 text-left"
      >
        <span className="text-ink-70">
          <FolderGlyph />
        </span>
        <span className="text-[13.5px] font-medium tracking-[-0.01em] text-ink">{dest.label}</span>
        <span className="text-ink-60 transition-colors duration-200 group-hover:text-ink">
          <Chevron />
        </span>
        <span className="ml-auto text-[11px] text-ink-60">
          {subs.length} {subs.length === 1 ? "subfolder" : "subfolders"}
        </span>
      </button>

      <div className="flex flex-wrap items-start gap-1.5">
        <Rules destKey={dest.key} />
      </div>
    </article>
  );
}

/** A subfolder inside a drill-down. Same card, one level down. */
function SubCard({ sub }: { sub: Destination }) {
  const view = useApp((s) => s.view);
  const hasRules = rulesFor(view, sub.key).length > 0;

  return (
    <article className="glass flex flex-col gap-2.5 rounded-[14px] p-3.5">
      <div className="flex items-center gap-2">
        <span className="text-ink-70">
          <FolderGlyph />
        </span>
        <span className="truncate text-[13.5px] font-medium tracking-[-0.01em] text-ink">{sub.label}</span>
      </div>

      <div className="flex flex-wrap items-start gap-1.5">
        {!hasRules && (
          // Mirrors `prose::implicit_subfolder_reading`, so what the card says is
          // exactly what the router does.
          <RuleTag
            implicit
            understood
            text={`Files named like “${sub.label}” land here`}
            reading="Every word of this folder's name must appear in the file's name."
          />
        )}
        <Rules destKey={sub.key} />
      </div>
    </article>
  );
}
