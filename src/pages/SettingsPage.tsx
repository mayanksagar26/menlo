import { useEffect, useRef, useState, type ReactNode } from "react";
import { AVATARS, avatarFor, CUSTOM_AVATAR, DEFAULT_AVATAR, squareImage } from "../lib/avatars";
import { SELECTABLE_MODELS } from "../lib/copy";
import { detectCliAgents, testCliAgent } from "../lib/ipc";
import type { AgentInfo, DuplicatePolicy, Schedule } from "../lib/types";
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
          <AvatarPicker />
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
          {s.working_model === "cli_agent" && <CliAgentPicker />}
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

/**
 * The profile picture: Menlo's own icon, the Recess gang, or one of your own.
 *
 * An uploaded picture is cropped and downscaled to 256px here before it is sent, so
 * what crosses to Rust is a few tens of kilobytes rather than a phone photo.
 */
function AvatarPicker() {
  const view = useApp((s) => s.view);
  const setAvatar = useApp((s) => s.setAvatar);
  const uploadAvatar = useApp((s) => s.uploadAvatar);
  const clearAvatar = useApp((s) => s.clearAvatar);
  const file = useRef<HTMLInputElement>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const chosen = view?.settings.avatar ?? DEFAULT_AVATAR;
  const uploaded = view?.avatar_image ?? null;
  const current = avatarFor(chosen, uploaded);

  async function pick(e: React.ChangeEvent<HTMLInputElement>) {
    const f = e.target.files?.[0];
    e.target.value = "";
    if (!f) return;
    setProblem(null);
    try {
      await uploadAvatar(await squareImage(f));
    } catch (err) {
      setProblem(err instanceof Error ? err.message : String(err));
    }
  }

  const options = uploaded ? [...AVATARS, { id: CUSTOM_AVATAR, label: "Your picture", src: uploaded }] : AVATARS;

  return (
    <div className="mt-6">
      <span className="mb-2.5 block text-[11.5px] text-ink-60">
        Your picture — it sits in the corner of the window. Menlo's own icon by default.
      </span>
      <div className="flex flex-wrap items-center gap-2">
        {options.map((a) => {
          const on = a.id === current.id;
          return (
            <button
              key={a.id}
              type="button"
              onClick={() => setAvatar(a.id)}
              aria-label={a.label}
              aria-pressed={on}
              title={a.label}
              className="glass h-11 w-11 overflow-hidden rounded-full p-0 transition-all duration-200"
              style={
                {
                  "--glass-hair": on ? "rgba(255,255,255,0.55)" : "var(--color-hair)",
                  transform: on ? "scale(1.04)" : "none",
                  opacity: on ? 1 : 0.72,
                  transitionTimingFunction: "var(--ease-menlo)",
                } as React.CSSProperties
              }
            >
              <img src={a.src} alt="" className="h-full w-full object-cover" />
            </button>
          );
        })}

        <button type="button" className="btn-glass ml-1" onClick={() => file.current?.click()}>
          Upload…
        </button>
        {uploaded && (
          <button type="button" className="btn-glass !text-ink-60" onClick={() => clearAvatar()}>
            Remove
          </button>
        )}
        <input
          ref={file}
          type="file"
          accept="image/png,image/jpeg"
          onChange={pick}
          className="hidden"
        />
      </div>
      {problem && <p className="mt-2 mb-0 text-[11px] text-ink-70">{problem}</p>}
    </div>
  );
}

/**
 * Which CLI agent the CLI Agent model drives. Lists the ones Menlo knows with where
 * each was found, plus "Other" for any command that reads a prompt on stdin.
 */
function CliAgentPicker() {
  const view = useApp((s) => s.view);
  const save = useApp((s) => s.saveSettings);
  const [agents, setAgents] = useState<AgentInfo[] | null>(null);
  const [check, setCheck] = useState<{ busy: boolean; ok?: boolean; text?: string }>({ busy: false });

  useEffect(() => {
    detectCliAgents()
      .then(setAgents)
      .catch(() => setAgents([]));
  }, []);

  if (!view) return null;
  const s = view.settings;
  const installed = agents?.filter((a) => a.path) ?? [];
  // An empty choice means "the first one found", so show that one as picked.
  const chosen = s.cli_agent || installed[0]?.id || "";

  function pick(id: string) {
    setCheck({ busy: false });
    save({ cli_agent: id });
  }

  async function runCheck() {
    setCheck({ busy: true });
    try {
      setCheck({ busy: false, ok: true, text: await testCliAgent() });
    } catch (err) {
      const e = err as { message?: string } | string;
      setCheck({ busy: false, ok: false, text: typeof e === "string" ? e : (e.message ?? String(err)).replace(/^configuration: /, "") });
    }
  }

  return (
    <div className="glass mt-3 rounded-[14px] p-3.5">
      <p className="m-0 mb-2.5 text-[11.5px] text-ink-60">
        Which CLI to use. Only files your rules can't place are sent — their names, types, sizes and dates, never
        their contents or where they live. Claude Code runs with its tools off; the others run sandboxed, unable
        to open anything in your home folder except their own settings.
      </p>
      {agents === null ? (
        <p className="m-0 text-[11.5px] text-ink-60">Looking for CLI agents on this Mac…</p>
      ) : (
        <div className="flex flex-col gap-1">
          {[...agents, { id: "custom", label: "Other command", path: null } as AgentInfo].map((a) => {
            const custom = a.id === "custom";
            const usable = custom || !!a.path;
            const on = chosen === a.id;
            return (
              <button
                key={a.id}
                type="button"
                disabled={!usable}
                onClick={() => pick(a.id)}
                aria-pressed={on}
                className="flex items-center gap-3 rounded-[10px] px-2.5 py-2 text-left transition-colors duration-200 disabled:cursor-default"
                style={{
                  background: on ? "var(--color-glass-sel)" : "transparent",
                  opacity: usable ? 1 : 0.5,
                  transitionTimingFunction: "var(--ease-menlo)",
                }}
              >
                <span
                  className="h-[9px] w-[9px] shrink-0 rounded-full border"
                  style={{
                    background: on ? "var(--color-ink)" : "transparent",
                    borderColor: on ? "var(--color-ink)" : "rgba(255,255,255,0.3)",
                  }}
                />
                <span className="flex-1 text-[12.5px] text-ink-86">{a.label}</span>
                <span className="selectable max-w-[300px] truncate text-[10.5px] text-ink-60">
                  {custom ? "any command that reads stdin" : (a.path ?? "not installed")}
                </span>
              </button>
            );
          })}
        </div>
      )}

      {chosen === "custom" && (
        <label className="mt-3 block">
          <span className="mb-1.5 block text-[11px] text-ink-60">
            The command, e.g. <code>qwen -p</code> or <code>aichat</code>. It runs directly, not in a shell, with the
            prompt on stdin, and must print the JSON reply.
          </span>
          <CommandField value={s.cli_command} onSave={(cli_command) => save({ cli_command })} />
        </label>
      )}

      <div className="mt-3 flex items-center gap-3">
        <button type="button" className="btn-glass" disabled={check.busy || !chosen} onClick={runCheck}>
          {check.busy ? "Checking…" : "Check it works"}
        </button>
        {check.text && (
          <span className="selectable min-w-0 flex-1 text-[11px] leading-[1.5] text-ink-70">
            {check.ok ? "✓ " : ""}
            {check.text}
          </span>
        )}
      </div>
    </div>
  );
}

function CommandField({ value, onSave }: { value: string; onSave: (v: string) => void }) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const commit = () => draft.trim() !== value && onSave(draft.trim());
  return (
    <input
      className="field max-w-[360px] font-mono"
      value={draft}
      placeholder="command --flags"
      spellCheck={false}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
    />
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

