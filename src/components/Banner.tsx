import { useStore } from "../store";

/**
 * The error surface. §4.3 asks for the raw output in a collapsible panel on plan
 * rejection; `details` carries the per-entry reasons and is rendered the same way.
 */
export function Banner() {
  const error = useStore((s) => s.error);
  const dismiss = useStore((s) => s.dismissError);
  if (!error) return null;

  return (
    <div
      role="alert"
      className="mx-4 mt-3 flex items-start gap-3 rounded-[6px] border border-oxblood/30 bg-oxblood/[0.06] px-3 py-2"
    >
      <span aria-hidden className="mt-0.5 text-oxblood">
        ●
      </span>
      <p className="selectable flex-1 text-ink-soft">{error}</p>
      <button type="button" onClick={dismiss} className="text-mute hover:text-ink" aria-label="Dismiss">
        ✕
      </button>
    </div>
  );
}
