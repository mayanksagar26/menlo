import { tildePath } from "../lib/format";
import { useStore, type Screen } from "../store";

const TABS: { id: Screen; label: string }[] = [
  { id: "review", label: "Review" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];

/** The window header: identity on the left, navigation centre, source folder right. */
export function Chrome({ actions }: { actions?: React.ReactNode }) {
  const screen = useStore((s) => s.screen);
  const go = useStore((s) => s.go);
  const source = useStore((s) => s.config.source);

  return (
    <header
      // The drag region: this is a desktop window, so the header moves it.
      data-tauri-drag-region
      className="flex h-12 shrink-0 items-center gap-4 border-b border-rule px-4"
    >
      <div data-tauri-drag-region className="flex items-baseline gap-2 pl-16">
        <span className="text-[15px] font-semibold tracking-tight">Menlo</span>
        {source && (
          <span className="mono selectable hidden truncate text-mute sm:inline">
            {tildePath(source)}
          </span>
        )}
      </div>

      <nav className="ml-auto flex items-center gap-1">
        {TABS.map((t) => (
          <button
            key={t.id}
            type="button"
            onClick={() => go(t.id)}
            aria-current={screen === t.id ? "page" : undefined}
            className={`h-7 rounded-[6px] px-2.5 transition-colors ${
              screen === t.id
                ? "bg-manila-deep text-ink"
                : "text-ink-soft hover:bg-manila hover:text-ink"
            }`}
          >
            {t.label}
          </button>
        ))}
      </nav>

      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </header>
  );
}
