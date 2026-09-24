import { avatarFor } from "../lib/avatars";
import { useApp } from "../store/app";
import { useUi } from "../store/ui";

/**
 * 56px of glass across the top: identity on the left, where-am-I on the right.
 *
 * The bar is also the window's drag region. macOS keeps its own traffic lights over
 * the left corner (`titleBarStyle: "Overlay"`): three 12px buttons on a 20px pitch
 * starting at x=20, so they end at x=72. The wordmark starts at 104 to leave a clear
 * gutter rather than crowding them — this window is small and sits over other work,
 * so the two sets of controls are read together and must not look like one row.
 */
export function TopBar() {
  const { home, step, page, openPage, toggleProfile } = useUi();
  const avatar = avatarFor(
    useApp((s) => s.view?.settings.avatar),
    useApp((s) => s.view?.avatar_image),
  );
  const counter = home ? "Home" : `0${step + 1} / 04`;

  return (
    <header
      data-tauri-drag-region
      className="glass-bar relative z-30 flex h-14 shrink-0 items-center gap-4 border-b border-hair px-5 pl-[104px]"
    >
      <span
        data-tauri-drag-region
        className="text-[15px] font-medium tracking-[0.34em] text-ink uppercase"
      >
        Menlo
      </span>

      <button
        type="button"
        onClick={() => openPage("docs")}
        className="text-[12px] text-ink-60 transition-colors duration-200 hover:text-ink"
        style={{ opacity: page === "docs" ? 0 : 1 }}
      >
        How it works?
      </button>

      <div data-tauri-drag-region className="ml-auto flex items-center gap-3.5">
        <span className="text-[11.5px] text-ink-60">{counter}</span>
        <button
          type="button"
          onClick={toggleProfile}
          aria-label="Menu"
          className="glass h-6 w-6 overflow-hidden rounded-full p-0 transition-opacity duration-200 hover:opacity-80"
          style={{ "--glass-fill": "var(--color-glass-sel)" } as React.CSSProperties}
        >
          <img src={avatar.src} alt="" className="h-full w-full object-cover" />
        </button>
      </div>
    </header>
  );
}
