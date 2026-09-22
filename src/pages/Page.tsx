import type { ReactNode } from "react";
import { useUi } from "../store/ui";

/**
 * The shell the five overlay pages share.
 *
 * They open *over* the flow rather than replacing it, so the flow keeps its place
 * and closing the page puts the user back exactly where they were. Left rail to
 * navigate, right panel that scrolls, one ✕.
 */
export function Page({
  title,
  rail,
  children,
}: {
  title: string;
  rail: ReactNode;
  children: ReactNode;
}) {
  const closePage = useUi((s) => s.closePage);

  return (
    <div
      className="anim-fade-in absolute inset-0 z-40 flex flex-col"
      style={{ background: "var(--color-scrim)", backdropFilter: "blur(8px)" }}
    >
      <div className="flex h-14 shrink-0 items-center gap-4 border-b border-hair px-6">
        <h2 className="m-0 text-[20px] font-medium tracking-[-0.02em] text-ink">{title}</h2>
        <button
          type="button"
          onClick={closePage}
          aria-label="Close"
          className="ml-auto text-[13px] text-ink-60 transition-colors duration-200 hover:text-ink"
        >
          ✕
        </button>
      </div>

      <div className="flex min-h-0 flex-1">
        <nav className="w-[220px] shrink-0 overflow-y-auto border-r border-hair p-3">{rail}</nav>
        <div className="min-w-0 flex-1 overflow-y-auto p-6">{children}</div>
      </div>
    </div>
  );
}

/** A row in a page's left rail. */
export function RailRow({
  active,
  onClick,
  children,
  indent = 0,
}: {
  active: boolean;
  onClick: () => void;
  children: ReactNode;
  indent?: number;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-current={active ? "true" : undefined}
      className="flex w-full items-center gap-2 rounded-[10px] py-2 pr-2.5 text-left transition-colors duration-200"
      style={{
        paddingLeft: 10 + indent,
        background: active ? "var(--color-glass-sel)" : "transparent",
        color: active ? "var(--color-ink)" : "var(--color-ink-70)",
        transitionTimingFunction: "var(--ease-menlo)",
      }}
    >
      {children}
    </button>
  );
}
