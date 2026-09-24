import { useEffect, useRef, useState, type ReactNode } from "react";
import { SELECTABLE_MODELS } from "../lib/copy";
import type { DuplicatePolicy, Schedule } from "../lib/types";
import { useApp } from "../store/app";
import { AboutIcon, ModelIcon, ProfileIcon, RunningIcon } from "../ui/glyphs";
import { Segmented, Toggle } from "../ui/primitives";
import { Page, RailRow } from "./Page";

const SECTIONS: { title: string; icon: ReactNode }[] = [
  { title: "Profile", icon: <ProfileIcon /> },
  { title: "Working model", icon: <ModelIcon /> },
  { title: "Running", icon: <RunningIcon /> },
  { title: "About", icon: <AboutIcon /> },
];

/**
 * Settings.
 *
 * Every control here is saved to disk and does what it says. Where something is
 * designed but not built yet — a model other than plain matching, scheduled runs — it
 * is shown, marked, and switched off, rather than offered as a choice that would
 * silently do nothing.
 */
export function SettingsPage() {
  const view = useApp((s) => s.view);
  const save = useApp((s) => s.saveSettings);
  const [active, setActive] = useState(0);
  const sections = useRef<(HTMLElement | null)[]>([]);
  if (!view) return null;
  const s = view.settings;

  function go(i: number) {
    setActive(i);
    sections.current[i]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  return (
    <Page
      title="Settings"
      rail={SECTIONS.map((sec, i) => (
        <RailRow key={sec.title} active={active === i} onClick={() => go(i)}>
          <span className="text-ink-60">{sec.icon}</span>
          <span className="text-[12.5px]">{sec.title}</span>
        </RailRow>
      ))}
    >
      <div className="flex max-w-[620px] flex-col gap-9">
        <Section index={0} refs={sections} title="Profile" onEnter={setActive}>
          <label className="block">
            <span className="mb-2 block text-[11.5px] text-ink-60">Your name — Menlo greets you with it.</span>
            <NameField value={s.profile_name} onSave={(profile_name) => save({ profile_name })} />
          </label>
        </Section>

        <Section index={1} refs={sections} title="Working model" onEnter={setActive}>
          <div className="flex flex-col gap-2">
            {SELECTABLE_MODELS.map((m) => {
              const on = s.working_model === m.value;
              return (
                <button
                  key={m.value}
                  type="button"
                  disabled={!m.ready}
                  onClick={() => save({ working_model: m.value })}
                  aria-pressed={on}
                  className="glass rounded-[14px] p-3.5 text-left transition-colors duration-200 disabled:cursor-default"
                  style={
                    {
                      "--glass-fill": on ? "var(--color-glass-sel)" : "var(--color-glass)",
                      "--glass-hair": on ? "rgba(255,255,255,0.4)" : "var(--color-hair)",
                      opacity: m.ready ? 1 : 0.62,
                      transitionTimingFunction: "var(--ease-menlo)",
                    } as React.CSSProperties
                  }
                >
                  <p
                    className="m-0 flex items-center gap-2 text-[13.5px] font-medium tracking-[-0.01em]"
                    style={{ color: on ? "#FFFFFF" : "var(--color-ink-86)" }}
                  >
                    {m.title}
                    {!m.ready && (
                      <span className="rounded-full border border-hair px-2 py-[1px] text-[10px] font-normal text-ink-60">
                        {m.when}
                      </span>
                    )}
                  </p>
                  <p className="mt-1.5 mb-0 text-[11.5px] leading-[1.6] text-ink-66">{m.body}</p>
                </button>
              );
            })}
          </div>
        </Section>

        <Section index={2} refs={sections} title="Running" onEnter={setActive}>
          <div className="flex flex-col gap-4">
            <Choice label="Duplicates" note="What to do with a file already filed, byte for byte.">
              <Segmented<DuplicatePolicy>
                value={s.duplicates}
                onChange={(duplicates) => save({ duplicates })}
                options={[
                  { value: "ask", label: "Ask each time" },
                  { value: "keep", label: "Keep" },
                  { value: "trash", label: "Trash" },
                ]}
              />
            </Choice>

            <Flag
              label="Rename unclear filenames"
              note="Screenshots and downloads get their folder's name and the time they were made."
              on={s.rename_unclear}
              onChange={(rename_unclear) => save({ rename_unclear })}
            />
            <Flag
              label="Show the preview before moving"
              on={s.ask_before_moving}
              onChange={(ask_before_moving) => save({ ask_before_moving })}
            />
            <Flag
              label="Include hidden files"
              on={s.include_hidden}
              onChange={(include_hidden) => save({ include_hidden })}
            />
            <Flag
              label="Skip files open in another app"
              on={s.check_open_files}
              onChange={(check_open_files) => save({ check_open_files })}
            />
            <Flag
              label="Move installers"
              note=".dmg and .pkg files stay put unless this is on."
              on={s.allow_installers}
              onChange={(allow_installers) => save({ allow_installers })}
            />

            <Choice label="Schedule" note="Scheduled runs arrive with the menu-bar agent.">
              <Segmented<Schedule>
                value={s.schedule}
                onChange={() => {}}
                disabled
                options={[
                  { value: "manual", label: "Manually" },
                  { value: "daily", label: "Daily, 9pm" },
                  { value: "on_login", label: "On login" },
                ]}
              />
            </Choice>
            <Flag
              label="Notify when a run finishes"
              note="Arrives with the menu-bar agent."
              on={false}
              disabled
              onChange={() => {}}
            />
            <Flag
              label="Keep run history"
              note="Always on — the history is how Restore works."
              on
              disabled
              onChange={() => {}}
            />
          </div>
        </Section>

        <Section index={3} refs={sections} title="About" onEnter={setActive}>
          <p className="m-0 text-[12.5px] leading-[1.7] text-ink-70">
            Menlo 0.1 — created by Mayank Sagar. Everything happens on this Mac: folders are read here, plans are
            made here, and no file or filename leaves the device. Every move is journalled and can be sent back.
          </p>
        </Section>
      </div>
    </Page>
  );
}

/** Saves on blur or Enter, not per keystroke. */
function NameField({ value, onSave }: { value: string; onSave: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => draft !== value && onSave(draft.trim());
  return (
    <input
      className="field max-w-[280px]"
      value={draft}
      placeholder="Your name"
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
    />
  );
}

function Section({
  index,
  refs,
  title,
  onEnter,
  children,
}: {
  index: number;
  refs: React.RefObject<(HTMLElement | null)[]>;
  title: string;
  onEnter: (i: number) => void;
  children: ReactNode;
}) {
  return (
    <section
      ref={(el) => {
        refs.current[index] = el;
      }}
      onMouseEnter={() => onEnter(index)}
    >
      <p className="eyebrow m-0 mb-3.5">{title}</p>
      {children}
    </section>
  );
}

function Choice({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return (
    <div className="flex items-center gap-4">
      <span className="flex-1">
        <span className="block text-[12.5px] text-ink-86">{label}</span>
        {note && <span className="block text-[11px] text-ink-60">{note}</span>}
      </span>
      {children}
    </div>
  );
}

function Flag({
  label,
  note,
  on,
  onChange,
  disabled = false,
}: {
  label: string;
  note?: string;
  on: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex items-center gap-4" style={{ opacity: disabled ? 0.55 : 1 }}>
      <span className="flex-1">
        <span className="block text-[12.5px] text-ink-86">{label}</span>
        {note && <span className="block text-[11px] text-ink-60">{note}</span>}
      </span>
      <Toggle on={on} onChange={disabled ? () => {} : onChange} label={label} />
    </div>
  );
}

