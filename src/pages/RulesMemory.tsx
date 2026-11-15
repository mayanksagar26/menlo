import { useEffect, useMemo, useState } from "react";
import { BUILTIN_KINDS, ideasFor } from "../lib/copy";
import { listDeviceFolders } from "../lib/ipc";
import type { AppView, DeviceFolder } from "../lib/types";
import { rulesFor, tilde, useApp } from "../store/app";
import { useUi } from "../store/ui";
import { Chevron } from "../ui/glyphs";
import { DashedChip, RuleTag } from "../ui/primitives";
import { Page, RailRow } from "./Page";

/**
 * Rules memory — every folder on this Mac, and what Menlo has been told or has learned
 * about each.
 *
 * The folder list is read from disk when this page opens and only then; reading it
 * changes nothing. A folder joins Menlo only when a rule is written for it. Main
 * folders collapse; their subfolders sit inside with their own rules.
 */
export function RulesMemory() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const addRule = useApp((s) => s.addRule);
  const addRuleAt = useApp((s) => s.addRuleAt);
  const removeRule = useApp((s) => s.removeRule);
  const forgetRule = useApp((s) => s.forgetRule);

  const [folders, setFolders] = useState<DeviceFolder[] | null>(null);
  const [open, setOpen] = useState<Set<string>>(new Set());
  const [draft, setDraft] = useState("");

  useEffect(() => {
    listDeviceFolders()
      .then((list) => {
        setFolders(list);
        // Folders Menlo already files into start open; the rest start folded.
        setOpen(new Set(list.filter((f) => f.key && f.children.length).map((f) => f.path)));
      })
      .catch(() => setFolders([]));
  }, []);

  // Which folder is shown: the one picked, else the first Menlo already uses.
  const all = useMemo(() => (folders ?? []).flatMap((f) => [f, ...f.children]), [folders]);
  const selected = all.find((f) => f.path === ui.rulesActive) ?? folders?.[0] ?? null;
  const parentOf = (f: DeviceFolder) => folders?.find((t) => t.children.some((c) => c.path === f.path));

  // Keep the selected folder's parent open.
  useEffect(() => {
    const p = selected && parentOf(selected);
    if (p && !open.has(p.path)) setOpen(new Set(open).add(p.path));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected?.path, folders]);

  const keyOf = (f: DeviceFolder) => view?.destinations.find((d) => d.path === f.path)?.key ?? null;

  function toggle(path: string) {
    const next = new Set(open);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setOpen(next);
  }

  async function add(text: string) {
    if (!selected || !text.trim()) return;
    const key = keyOf(selected);
    if (key) await addRule(key, text);
    else await addRuleAt(selected.path, text);
    setDraft("");
  }

  const rail =
    folders === null ? (
      <p className="m-0 px-2.5 py-2 text-[11.5px] text-ink-60">Reading your folders…</p>
    ) : (
      folders.map((f) => {
        const expanded = open.has(f.path);
        return (
          <div key={f.path}>
            <RailRow active={selected?.path === f.path} onClick={() => ui.set({ rulesActive: f.path })}>
              <span
                role="button"
                tabIndex={-1}
                aria-label={expanded ? `Collapse ${f.label}` : `Expand ${f.label}`}
                aria-expanded={expanded}
                className="flex h-4 w-4 shrink-0 items-center justify-center text-ink-60"
                style={{ visibility: f.children.length ? "visible" : "hidden" }}
                onClick={(e) => {
                  e.stopPropagation();
                  toggle(f.path);
                }}
              >
                <Chevron open={expanded} />
              </span>
              <span className="flex-1 truncate text-[12.5px]">{f.label}</span>
              <Count n={countFor(view, keyOf(f))} />
            </RailRow>
            {expanded &&
              f.children.map((c) => (
                <RailRow
                  key={c.path}
                  indent={24}
                  active={selected?.path === c.path}
                  onClick={() => ui.set({ rulesActive: c.path })}
                >
                  <span className="flex-1 truncate text-[11.5px]">{c.label}</span>
                  <Count n={countFor(view, keyOf(c))} />
                </RailRow>
              ))}
          </div>
        );
      })
    );

  if (!selected) {
    return (
      <Page title="Rules memory" rail={rail}>
        <p className="m-0 text-[12.5px] text-ink-60">{folders === null ? "" : "No folders found in your home folder."}</p>
      </Page>
    );
  }

  const key = keyOf(selected);
  const isSub = !!parentOf(selected);
  const sentences = key ? rulesFor(view, key) : [];
  const learned = key ? (view?.matchers ?? []).filter((m) => m.destination_key === key) : [];
  const total = sentences.length + learned.length;

  return (
    <Page title="Rules memory" rail={rail}>
      <div className="max-w-[620px]">
        <h3 className="m-0 text-[17px] font-medium tracking-[-0.015em] text-ink">{tilde(view, selected.path)}</h3>
        <p className="mt-1.5 mb-0 text-[11.5px] text-ink-60">
          {key
            ? `${total} ${total === 1 ? "rule" : "rules"} saved for this folder`
            : "Menlo doesn't file into this folder yet. Writing a rule here adds it."}
        </p>

        <ul className="mt-5 flex list-none flex-col gap-2.5 p-0">
          {key && total === 0 && (
            <li className="text-[11.5px] leading-[1.6] text-ink-60">
              {isSub
                ? `No rules yet, so this folder takes files named like “${selected.label}”. Anything you write here replaces that.`
                : BUILTIN_KINDS[key]
                  ? `No rules yet. ${BUILTIN_KINDS[key]} still land here by type until you say otherwise.`
                  : "No rules yet, so nothing lands here. Write one, or tap an idea below."}
            </li>
          )}
          {sentences.map((r) => (
            <li key={r.id} className="flex flex-col gap-1">
              <span>
                <RuleTag text={r.text} understood={r.understood} reading={r.reading} onRemove={() => removeRule(r.id)} />
              </span>
              <span className="pl-1 text-[11px] text-ink-60">{r.reading}</span>
            </li>
          ))}
        </ul>

        {learned.length > 0 && (
          <div className="mt-6">
            <p className="eyebrow m-0 mb-2.5">Learned from what you approved</p>
            <ul className="m-0 flex list-none flex-col gap-1.5 p-0">
              {learned.map((m) => (
                <li key={m.id} className="flex items-center gap-3">
                  <span className="min-w-0 flex-1 truncate text-[12px] text-ink-86">
                    {capitalise(m.human)}
                    {!m.learned && <span className="text-ink-60"> · your rule</span>}
                  </span>
                  <span className="shrink-0 text-[10.5px] text-ink-60">
                    {m.hits ? `${m.hits} filed` : ""}
                  </span>
                  <button
                    type="button"
                    onClick={() => forgetRule(m.id)}
                    aria-label={`Forget ${m.match_expr}`}
                    className="shrink-0 text-[11px] text-ink-60 transition-colors duration-200 hover:text-ink"
                  >
                    Forget
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}

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
          {ideasFor(selected.label, isSub).map((idea) => (
            <DashedChip key={idea} onClick={() => add(idea)}>
              {idea}
            </DashedChip>
          ))}
        </div>
      </div>
    </Page>
  );
}

function countFor(view: AppView | null, key: string | null) {
  if (!key) return 0;
  return rulesFor(view, key).length + (view?.matchers ?? []).filter((m) => m.destination_key === key).length;
}

function capitalise(s: string) {
  return s ? s[0].toUpperCase() + s.slice(1) : s;
}

function Count({ n }: { n: number }) {
  return (
    <span className="shrink-0 text-[10.5px] text-ink-60" aria-label={`${n} rules`}>
      {n || ""}
    </span>
  );
}
