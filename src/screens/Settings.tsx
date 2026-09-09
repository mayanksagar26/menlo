import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Chrome } from "../components/Chrome";
import { slugify, tildePath } from "../lib/format";
import type { Destination } from "../lib/types";
import { useStore } from "../store";

/**
 * Destinations, their briefs, the rule list, and the §7 knobs.
 *
 * The rule list is where the learned rules from §1.2 tier 2 become inspectable — the
 * user can see exactly what Menlo has concluded and delete anything it got wrong.
 */
export function Settings() {
  const config = useStore((s) => s.config);
  const rules = useStore((s) => s.rules);
  const busy = useStore((s) => s.busy);
  const setConfig = useStore((s) => s.setConfig);
  const persistConfig = useStore((s) => s.persistConfig);
  const removeDestination = useStore((s) => s.removeDestination);
  const addDestination = useStore((s) => s.addDestination);
  const refreshRules = useStore((s) => s.refreshRules);
  const deleteRule = useStore((s) => s.deleteRule);
  const toggleRule = useStore((s) => s.toggleRule);
  const addRule = useStore((s) => s.addRule);

  const [newRule, setNewRule] = useState({ match: "", key: "" });

  useEffect(() => {
    void refreshRules();
  }, [refreshRules]);

  function patchDestination(key: string, patch: Partial<Destination>) {
    setConfig({
      ...config,
      destinations: config.destinations.map((d) => (d.key === key ? { ...d, ...patch } : d)),
    });
  }

  async function pickSource() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    setConfig({ ...config, source: picked });
    await persistConfig();
  }

  async function addFolder() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked !== "string") return;
    const label = picked.split("/").pop() ?? "Folder";
    let key = slugify(label);
    // Keys must be unique; nudge rather than reject.
    let n = 2;
    while (config.destinations.some((d) => d.key === key)) key = `${slugify(label)}-${n++}`;
    await addDestination({ key, label, path: picked, brief: "" });
  }

  return (
    <div className="flex h-full flex-col">
      <Chrome />
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-6 py-6">
          <section>
            <SectionTitle>Source folder</SectionTitle>
            <div className="card flex items-center gap-3 px-3 py-2.5">
              <span className="mono selectable flex-1 truncate text-ink-soft">
                {config.source ? tildePath(config.source) : "None"}
              </span>
              <button type="button" className="btn" onClick={pickSource}>
                Change
              </button>
            </div>
          </section>

          <section>
            <SectionTitle>Destinations</SectionTitle>
            <div className="flex flex-col gap-3">
              {config.destinations.map((d) => (
                <div key={d.key} className="card flex flex-col gap-2 px-3 py-3">
                  <div className="flex items-center gap-2">
                    <input
                      className="field h-7 flex-1 py-0 font-medium"
                      value={d.label}
                      onChange={(e) => patchDestination(d.key, { label: e.target.value })}
                      onBlur={persistConfig}
                      aria-label="Folder name"
                    />
                    <span className="mono text-mute">{d.key}</span>
                    <button
                      type="button"
                      className="text-mute hover:text-oxblood"
                      onClick={() => removeDestination(d.key)}
                      aria-label={`Remove ${d.label}`}
                    >
                      ✕
                    </button>
                  </div>
                  <div className="mono selectable truncate text-mute">{tildePath(d.path)}</div>
                  <textarea
                    className="field min-h-[54px] resize-y"
                    placeholder="What belongs here, in your own words."
                    value={d.brief}
                    onChange={(e) => patchDestination(d.key, { brief: e.target.value })}
                    onBlur={persistConfig}
                    aria-label={`Brief for ${d.label}`}
                  />
                </div>
              ))}
              <button type="button" className="btn self-start" onClick={addFolder}>
                Add a destination
              </button>
            </div>
          </section>

          <section>
            <SectionTitle>Rules</SectionTitle>
            <p className="mb-3 text-mute">
              Rules resolve files with no AI at all. Menlo writes a learned rule each time you
              approve a decision it had to think about.
            </p>

            <div className="card divide-y divide-rule">
              {rules.rules.length === 0 && (
                <p className="px-3 py-3 text-mute">
                  No rules yet. They accumulate as you approve batches.
                </p>
              )}
              {rules.rules.map((r) => (
                <div key={r.id} className="flex items-center gap-3 px-3 py-2">
                  <input
                    type="checkbox"
                    checked={r.enabled}
                    onChange={(e) => toggleRule(r.id, e.target.checked)}
                    aria-label={`Enable ${r.match}`}
                    className="h-3.5 w-3.5 accent-oxblood"
                  />
                  <span className={`mono flex-1 truncate ${r.enabled ? "" : "text-mute line-through"}`}>
                    {r.match}
                  </span>
                  <span className="text-mute">→</span>
                  <span className="mono w-28 truncate">{r.destination_key}</span>
                  <span
                    className="w-16 text-right text-[11px] uppercase tracking-wide text-mute"
                    title={r.source === "learned" ? "Learned from an approval you made" : undefined}
                  >
                    {r.source}
                  </span>
                  <button
                    type="button"
                    className="text-mute hover:text-oxblood"
                    onClick={() => deleteRule(r.id)}
                    aria-label={`Delete rule ${r.match}`}
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>

            <div className="mt-3 flex items-center gap-2">
              <input
                className="field mono flex-1"
                placeholder="glob:Invoice_*.pdf"
                value={newRule.match}
                onChange={(e) => setNewRule({ ...newRule, match: e.target.value })}
              />
              <select
                className="field h-[30px] w-40 py-0"
                value={newRule.key}
                onChange={(e) => setNewRule({ ...newRule, key: e.target.value })}
                aria-label="Rule destination"
              >
                <option value="">Destination…</option>
                {config.destinations.map((d) => (
                  <option key={d.key} value={d.key}>
                    {d.label}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className="btn"
                disabled={!newRule.match || !newRule.key || !!busy}
                onClick={async () => {
                  await addRule(newRule.match, newRule.key);
                  setNewRule({ match: "", key: "" });
                }}
              >
                Add
              </button>
            </div>
            <p className="mt-1.5 text-mute">
              Prefixes: <code className="mono">ext:</code> <code className="mono">glob:</code>{" "}
              <code className="mono">origin:</code>
            </p>
          </section>

          <section>
            <SectionTitle>Safety</SectionTitle>
            <div className="card flex flex-col divide-y divide-rule">
              <Toggle
                checked={config.settings.check_open_files}
                onChange={(v) => {
                  setConfig({
                    ...config,
                    settings: { ...config.settings, check_open_files: v },
                  });
                  void persistConfig();
                }}
                title="Skip files open in another app"
                detail="Menlo asks lsof before each move. Leave this on."
              />
              <Toggle
                checked={config.settings.allow_installers}
                onChange={(v) => {
                  setConfig({
                    ...config,
                    settings: { ...config.settings, allow_installers: v },
                  });
                  void persistConfig();
                }}
                title="Include installers"
                detail=".dmg, .pkg, .app and .iso are left alone unless you turn this on."
              />
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return <h2 className="mb-2 text-xs uppercase tracking-wide text-mute">{children}</h2>;
}

function Toggle({
  checked,
  onChange,
  title,
  detail,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  title: string;
  detail: string;
}) {
  return (
    <label className="flex cursor-pointer items-start gap-3 px-3 py-2.5">
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="mt-0.5 h-3.5 w-3.5 accent-oxblood"
      />
      <span>
        <span className="block">{title}</span>
        <span className="block text-mute">{detail}</span>
      </span>
    </label>
  );
}
