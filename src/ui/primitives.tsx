/**
 * The pieces every screen is assembled from.
 *
 * The glass recipe itself lives in `styles/tokens.css` as `.glass`; these wrap it
 * with the behaviour the handoff specifies — the selection dot's spring, the row
 * hover lift, the scrim that closes a sheet — so no screen re-invents them.
 */

import { useEffect, type ReactNode } from "react";
import { Check, Chevron } from "./glyphs";

export function Eyebrow({ children }: { children: ReactNode }) {
  return <p className="eyebrow m-0">{children}</p>;
}

/**
 * The 16px dot on every pickable row. Empty ring when off; fills with ink and a
 * checkmark when on, arriving on the springy curve.
 */
export function SelectDot({ on }: { on: boolean }) {
  return (
    <span
      aria-hidden
      className="flex h-4 w-4 shrink-0 items-center justify-center rounded-full border transition-all duration-[420ms]"
      style={{
        transitionTimingFunction: "var(--ease-menlo)",
        transform: on ? "scale(1)" : "scale(0.82)",
        background: on ? "var(--color-ink)" : "transparent",
        borderColor: on ? "var(--color-ink)" : "rgba(255,255,255,0.28)",
        color: "#0B0D11",
      }}
    >
      <span
        className="flex transition-opacity duration-200"
        style={{ opacity: on ? 1 : 0, transitionTimingFunction: "var(--ease-spring)" }}
      >
        <Check />
      </span>
    </span>
  );
}

/** A row in one of the two-column lists: glass, lifts on hover, fills when picked. */
export function PickRow({
  on,
  name,
  meta,
  onClick,
}: {
  on: boolean;
  name: string;
  meta: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={on}
      className="glass flex w-full items-center gap-2.5 rounded-[11px] px-3 py-2.5 text-left transition-colors duration-200"
      style={{
        "--glass-fill": on ? "var(--color-glass-sel)" : "var(--color-glass)",
        "--glass-hair": on ? "rgba(255,255,255,0.4)" : "var(--color-hair-hi)",
        "--glass-rim": on ? "rgba(255,255,255,0.24)" : "rgba(255,255,255,0.1)",
        transitionTimingFunction: "var(--ease-menlo)",
      } as React.CSSProperties}
    >
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[12.5px]" style={{ color: on ? "#FFFFFF" : "var(--color-ink-86)" }}>
          {name}
        </span>
        <span className="block truncate text-[11px] text-ink-60">{meta}</span>
      </span>
      <SelectDot on={on} />
    </button>
  );
}

/**
 * A saved rule. Free text of any length, so it wraps rather than truncating.
 *
 * The leading mark says whether Menlo can act on it without a model — filled when it
 * can, a ring when it needs one — and hovering shows what it understood, so a rule
 * is never trusted blind.
 */
export function RuleTag({
  text,
  onRemove,
  understood,
  reading,
  implicit = false,
}: {
  text: string;
  onRemove?: () => void;
  understood?: boolean;
  reading?: string;
  /** The rule a subfolder follows when nobody has written one. Not removable. */
  implicit?: boolean;
}) {
  return (
    <span
      className="glass inline-flex max-w-full items-start gap-1.5 rounded-[11px] px-2.5 py-1 text-[11px] leading-[1.45]"
      style={{
        color: implicit ? "var(--color-ink-60)" : "var(--color-ink-86)",
        borderStyle: implicit ? "dashed" : undefined,
      }}
      title={reading}
    >
      {understood !== undefined && (
        <span
          aria-label={understood ? "Works without a model" : "Needs a model"}
          className="mt-[5px] h-[6px] w-[6px] shrink-0 rounded-full"
          style={{
            background: understood ? "var(--color-ink-70)" : "transparent",
            border: understood ? "none" : "1px solid var(--color-ink-60)",
          }}
        />
      )}
      <span className="min-w-0">{text}</span>
      {onRemove && (
        <button
          type="button"
          onClick={onRemove}
          aria-label={`Remove rule: ${text}`}
          className="mt-[1px] shrink-0 text-ink-60 transition-colors hover:text-ink"
        >
          ✕
        </button>
      )}
    </span>
  );
}

/** The dashed affordance: "Add rule", a suggested folder, a set that does not exist yet. */
export function DashedChip({
  children,
  onClick,
  className = "",
}: {
  children: ReactNode;
  onClick?: () => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`inline-flex items-center gap-1.5 rounded-[11px] border border-dashed px-2.5 py-1 text-[11px] leading-[1.45] text-ink-60 transition-colors duration-200 hover:bg-glass-hi hover:text-ink ${className}`}
      style={{ borderColor: "rgba(255,255,255,0.22)", transitionTimingFunction: "var(--ease-menlo)" }}
    >
      {children}
    </button>
  );
}

export function Toggle({
  on,
  onChange,
  label,
}: {
  on: boolean;
  onChange: (next: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => onChange(!on)}
      className="relative h-6 w-[42px] shrink-0 rounded-full border transition-colors duration-[280ms]"
      style={{
        background: on ? "var(--color-ink)" : "rgba(255,255,255,0.08)",
        borderColor: on ? "var(--color-ink)" : "var(--color-hair-hi)",
        transitionTimingFunction: "var(--ease-menlo)",
      }}
    >
      <span
        className="absolute top-1/2 left-[2px] h-[18px] w-[18px] rounded-full transition-transform duration-[280ms]"
        style={{
          background: on ? "#0B0D11" : "rgba(237,238,241,0.75)",
          transform: `translateY(-50%) translateX(${on ? 18 : 0}px)`,
          transitionTimingFunction: "var(--ease-menlo)",
        }}
      />
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  disabled = false,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (next: T) => void;
  disabled?: boolean;
}) {
  return (
    <div className="glass inline-flex rounded-full p-[3px]" style={{ opacity: disabled ? 0.55 : 1 }}>
      {options.map((o) => {
        const on = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            aria-pressed={on}
            disabled={disabled}
            onClick={() => onChange(o.value)}
            className="rounded-full px-3 py-[5px] text-[11.5px] transition-colors duration-200"
            style={{
              background: on ? "var(--color-glass-sel)" : "transparent",
              color: on ? "var(--color-ink)" : "var(--color-ink-66)",
              transitionTimingFunction: "var(--ease-menlo)",
            }}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/** An expandable row: a chevron that rotates, a label, a count on the right. */
export function DisclosureRow({
  open,
  title,
  meta,
  onClick,
  indent = 0,
}: {
  open: boolean;
  title: ReactNode;
  meta?: ReactNode;
  onClick: () => void;
  indent?: number;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-expanded={open}
      className="flex w-full items-center gap-2 rounded-[10px] px-2.5 py-2 text-left transition-colors duration-200 hover:bg-glass-hi"
      style={{ paddingLeft: 10 + indent, transitionTimingFunction: "var(--ease-menlo)" }}
    >
      <span className="text-ink-60">
        <Chevron open={open} />
      </span>
      <span className="min-w-0 flex-1 truncate text-[12.5px] text-ink-86">{title}</span>
      {meta && <span className="shrink-0 text-[11px] text-ink-60">{meta}</span>}
    </button>
  );
}

/**
 * The layer every modal sits on. Clicking the scrim closes, Escape closes, and the
 * sheet stops the click from reaching the scrim.
 */
export function Modal({
  onClose,
  width,
  children,
  labelledBy,
}: {
  onClose: () => void;
  width: number;
  children: ReactNode;
  labelledBy?: string;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="anim-fade-in absolute inset-0 z-50 flex items-center justify-center"
      style={{ background: "var(--color-scrim)", backdropFilter: "blur(7px)" }}
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        className="sheet flex max-h-[calc(100%-56px)] flex-col overflow-hidden"
        style={{ width }}
        onClick={(e) => e.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}

/** A sheet's footer: a note on the left, actions on the right. */
export function SheetFooter({ note, children }: { note?: ReactNode; children: ReactNode }) {
  return (
    <div className="flex shrink-0 items-center gap-3 border-t border-hair px-5 py-3.5">
      {note && <span className="min-w-0 flex-1 truncate text-[11px] text-ink-60">{note}</span>}
      <div className={`flex items-center gap-2 ${note ? "" : "flex-1 justify-end"}`}>{children}</div>
    </div>
  );
}

export function SearchField({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (next: string) => void;
  placeholder: string;
}) {
  return (
    <input
      className="field"
      type="search"
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
    />
  );
}
