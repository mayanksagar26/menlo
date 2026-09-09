import { forwardRef } from "react";
import { plural } from "../lib/format";

interface Props {
  label: string;
  destKey: string;
  count: number;
  active?: boolean;
  onClick?: () => void;
}

/**
 * A manila folder with a small offset tab on the top-left edge (§5).
 *
 * The tab is drawn in CSS (`.folder::before`) rather than as an SVG so it inherits the
 * hover and active colours without a second code path.
 */
export const FolderTile = forwardRef<HTMLButtonElement, Props>(function FolderTile(
  { label, destKey, count, active = false, onClick },
  ref,
) {
  return (
    <button
      ref={ref}
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={`folder ${active ? "folder-active" : ""} w-full px-3 pt-2.5 pb-3 text-left transition-colors hover:bg-manila-deep`}
    >
      <div className="truncate font-medium">{label}</div>
      <div className="mono mt-0.5 truncate text-mute">{destKey}</div>
      <div className={`mt-2 text-xs ${count > 0 ? "text-ink-soft" : "text-mute"}`}>
        {count > 0 ? plural(count, "file") : "nothing yet"}
      </div>
    </button>
  );
});
