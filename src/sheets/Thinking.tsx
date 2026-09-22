import { Check, FileGlyph, type Bucket } from "../ui/glyphs";

/**
 * The two "Menlo is working" popups share this body.
 *
 * Both stand in for a wait the user cannot see into, so both show the same two
 * things: something moving that looks like files being handled, and a list of the
 * actual steps ticking off in order, so the wait is accounted for rather than spun.
 */

const CARDS: { bucket: Bucket; sx: number; sy: number; rot: number; dur: number; left: number; top: number }[] = [
  { bucket: "doc", sx: 46, sy: -18, rot: -9, dur: 2.4, left: 6, top: 14 },
  { bucket: "img", sx: -38, sy: 26, rot: 7, dur: 2.9, left: 34, top: 52 },
  { bucket: "code", sx: 30, sy: 34, rot: 11, dur: 3.3, left: 62, top: 12 },
  { bucket: "vid", sx: -44, sy: -22, rot: -6, dur: 2.6, left: 48, top: 74 },
  { bucket: "arc", sx: 24, sy: -30, rot: 8, dur: 3.1, left: 14, top: 72 },
  { bucket: "aud", sx: -28, sy: 18, rot: -10, dur: 2.8, left: 76, top: 46 },
];

/** A framed field of file cards shuffling themselves into order. */
export function ShuffleField({ height = 128 }: { height?: number }) {
  return (
    <div
      className="glass relative overflow-hidden rounded-[14px]"
      style={{ height }}
      aria-hidden
    >
      {CARDS.map((c, i) => (
        <span
          key={i}
          className="glass absolute flex h-7 w-7 items-center justify-center rounded-[9px] text-ink-70"
          style={{
            left: `${c.left}%`,
            top: `${c.top}%`,
            ["--sx" as string]: `${c.sx}px`,
            ["--sy" as string]: `${c.sy}px`,
            ["--srot" as string]: `${c.rot}deg`,
            animation: `menlo-sort ${c.dur}s ease-in-out ${i * 0.18}s infinite`,
          }}
        >
          <FileGlyph bucket={c.bucket} size={12} />
        </span>
      ))}
    </div>
  );
}

/** The steps, ticking off in sequence. Anything past `active` is still to come. */
export function StepList({ steps, active }: { steps: string[]; active: number }) {
  return (
    <ul className="m-0 flex list-none flex-col gap-2.5 p-0">
      {steps.map((step, i) => {
        const done = active > i;
        const now = active === i;
        return (
          <li key={step} className="flex items-center gap-2.5">
            <span
              className="flex h-[15px] w-[15px] shrink-0 items-center justify-center rounded-full border transition-all duration-[280ms]"
              style={{
                background: done ? "var(--color-ink)" : "transparent",
                borderColor: done || now ? "var(--color-ink)" : "rgba(255,255,255,0.22)",
                color: "#0B0D11",
                transform: done ? "scale(1)" : "scale(0.86)",
                transitionTimingFunction: "var(--ease-spring)",
              }}
            >
              {done && <Check size={8} />}
            </span>
            <span
              className="text-[11.5px] transition-colors duration-200"
              style={{ color: done || now ? "var(--color-ink-86)" : "var(--color-ink-60)" }}
            >
              {step}
            </span>
          </li>
        );
      })}
    </ul>
  );
}
