import { useEffect, useState } from "react";
import logo from "../../design/brand/icon.svg";

/**
 * Where the window is in its first second.
 *
 * - `logo`: the window is frosted glass — the desktop behind it, blurred — with
 *   only the mark in the middle. This holds until the backend has answered, and for at
 *   least `HOLD_MS` so a fast boot does not flash.
 * - `open`: the app fades up over the glass while the mark lifts away, and home
 *   plays its entrance.
 * - `done`: the launch layer is gone and the window behaves as it always does.
 */
export type LaunchPhase = "logo" | "open" | "done";

const HOLD_MS = 1150;
const OPEN_MS = 820;

const reducedMotion = () =>
  typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;

/** Drives the phases. `ready` is whether there is anything to open onto yet. */
export function useLaunch(ready: boolean): LaunchPhase {
  const [phase, setPhase] = useState<LaunchPhase>(() => (reducedMotion() ? "done" : "logo"));
  const [held, setHeld] = useState(false);

  useEffect(() => {
    if (phase !== "logo") return;
    const t = window.setTimeout(() => setHeld(true), HOLD_MS);
    return () => window.clearTimeout(t);
  }, [phase]);

  useEffect(() => {
    if (phase === "logo" && held && ready) setPhase("open");
  }, [phase, held, ready]);

  useEffect(() => {
    if (phase !== "open") return;
    const t = window.setTimeout(() => setPhase("done"), OPEN_MS);
    return () => window.clearTimeout(t);
  }, [phase]);

  return phase;
}

/**
 * The mark, alone in the middle of a translucent window.
 *
 * The glass is the window's own macOS material (`windowEffects` in
 * `tauri.conf.json`), which only shows while the app surface above it is hidden;
 * here it gets a light tint so the mark has something to sit on. The layer never
 * takes a click, so nothing underneath waits on it.
 */
export function LaunchMark({ phase }: { phase: LaunchPhase }) {
  if (phase === "done") return null;
  return (
    <div
      className="pointer-events-none fixed inset-0 z-[100] flex items-center justify-center"
      aria-hidden
    >
      <div className={`launch-tint ${phase === "open" ? "launch-tint-out" : ""}`} />
      <div className={phase === "logo" ? "launch-mark-in" : "launch-mark-out"}>
        <div className="launch-halo" />
        <img src={logo} alt="" draggable={false} className="relative h-[132px] w-[132px]" />
      </div>
    </div>
  );
}
