import type { ReactNode } from "react";
import { MODELS } from "../lib/copy";
import { useApp } from "../store/app";
import { useUi } from "../store/ui";
import { DocsIcon, RulesIcon, RunsIcon, SetsIcon, SettingsIcon } from "./glyphs";

/**
 * The profile popover: the five places that are not part of the flow.
 *
 * Each row carries the figure that makes it worth opening — how many rules are
 * saved, how many runs are kept — so the menu answers as much as it navigates.
 */
export function ProfileMenu() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const model = MODELS.find((m) => m.value === view?.settings.working_model);

  const links: { icon: ReactNode; label: string; value: string; go: () => void }[] = [
    {
      icon: <RulesIcon />,
      label: "Rules memory",
      value: `${view?.rules.length ?? 0} saved`,
      go: () => ui.openPage("rules"),
    },
    {
      icon: <RunsIcon />,
      label: "Runs",
      value: `${view?.runs.length ?? 0} kept`,
      go: () => ui.openPage("runs"),
    },
    {
      icon: <SetsIcon />,
      label: "Folder sets",
      value: `${view?.folder_sets.length ?? 0} saved`,
      go: () => ui.openSets(true),
    },
    { icon: <DocsIcon />, label: "How it works", value: "5 stages", go: () => ui.openPage("docs") },
    {
      icon: <SettingsIcon />,
      label: "Settings",
      value: model?.title ?? "",
      go: () => ui.openPage("settings"),
    },
  ];

  return (
    <>
      {/* Catches the next click anywhere in the window and closes the menu. */}
      <div className="absolute inset-0 z-40" onClick={ui.closeProfile} />
      <div
        className="glass-pop anim-rise absolute z-50 w-[260px] rounded-[16px] p-1.5"
        style={{ right: 20, top: 62 }}
        role="menu"
      >
        {links.map((l) => (
          <button
            key={l.label}
            type="button"
            role="menuitem"
            onClick={l.go}
            className="group flex w-full items-center gap-3 rounded-[11px] px-3 py-2.5 text-left transition-colors duration-200 hover:bg-glass-hi"
            style={{ transitionTimingFunction: "var(--ease-menlo)" }}
          >
            <span className="text-ink-60 transition-colors duration-200 group-hover:text-ink">
              {l.icon}
            </span>
            <span className="flex-1 text-[12.5px] text-ink-86">{l.label}</span>
            <span className="text-[11px] text-ink-60">{l.value}</span>
          </button>
        ))}
      </div>
    </>
  );
}
